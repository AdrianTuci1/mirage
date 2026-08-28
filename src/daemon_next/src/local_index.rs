use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// A lightweight, in-memory index for fast local file name / path lookup.
///
/// This is intentionally separate from the semantic (LanceDB) index:
/// file names and paths are matched with cheap string scoring, while
/// embeddings are reserved for meaning / content search.
#[derive(Debug, Default, Clone)]
pub struct LocalFileIndex {
    entries: Vec<LocalEntry>,
    name_to_indices: HashMap<String, Vec<usize>>,
}

#[derive(Debug, Clone)]
pub struct LocalEntry {
    pub id: String,
    pub absolute_path: PathBuf,
    pub relative_path: String,
    pub file_name: String,
    pub source_type: String,
    pub open_url: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct LocalSearchResult {
    pub id: String,
    pub absolute_path: String,
    pub relative_path: String,
    pub file_name: String,
    pub source_type: String,
    pub open_url: Option<String>,
    pub score: f64,
}

impl LocalFileIndex {
    pub fn new() -> Self {
        Self::default()
    }

    /// Replace the current index with entries discovered under `roots`.
    pub fn index_roots(&mut self, roots: &[PathBuf], excluded_dirs: &[String]) {
        self.entries.clear();
        self.name_to_indices.clear();

        for root in roots {
            self.walk(root, excluded_dirs);
        }

        self.rebuild_token_index();
    }

    pub fn index_remote_entries(
        &mut self,
        source_type: &str,
        entries: Vec<crate::connectors::RemoteEntry>,
    ) {
        let source_type = source_type.to_string();
        for entry in entries {
            let file_name = entry.name.clone();
            self.entries.push(LocalEntry {
                id: entry.id.clone(),
                absolute_path: PathBuf::from(&entry.path),
                relative_path: entry.path,
                file_name,
                source_type: source_type.clone(),
                open_url: entry.open_url,
            });
        }
        self.rebuild_token_index();
    }

    pub fn clear_remote_entries(&mut self) {
        self.entries.retain(|e| e.source_type == "local");
        self.rebuild_token_index();
    }

    fn rebuild_token_index(&mut self) {
        self.name_to_indices.clear();
        for (idx, entry) in self.entries.iter().enumerate() {
            for token in path_tokens(&entry.relative_path) {
                self.name_to_indices.entry(token).or_default().push(idx);
            }
        }
    }

    fn walk(&mut self, root: &Path, excluded_dirs: &[String]) {
        let excluded: std::collections::HashSet<String> =
            excluded_dirs.iter().map(|s| s.to_lowercase()).collect();

        let mut stack: Vec<PathBuf> = vec![root.to_path_buf()];
        while let Some(dir) = stack.pop() {
            let dir_name = dir
                .file_name()
                .map(|n| n.to_string_lossy().to_lowercase())
                .unwrap_or_default();
            if excluded.contains(&dir_name) {
                continue;
            }

            let entries = match std::fs::read_dir(&dir) {
                Ok(e) => e,
                Err(_) => continue,
            };

            for entry in entries.flatten() {
                let path = entry.path();
                let metadata = match entry.metadata() {
                    Ok(m) => m,
                    Err(_) => continue,
                };

                if metadata.is_dir() {
                    let child_name = entry.file_name().to_string_lossy().to_lowercase();
                    if !excluded.contains(&child_name) {
                        stack.push(path);
                    }
                } else if metadata.is_file() {
                    let file_name = path
                        .file_name()
                        .map(|n| n.to_string_lossy().to_string())
                        .unwrap_or_default();
                    if file_name.is_empty() {
                        continue;
                    }
                    let relative_path = strip_root(root, &path);
                    self.entries.push(LocalEntry {
                        id: format!("file://{}", path.to_string_lossy()),
                        absolute_path: path.clone(),
                        relative_path: relative_path.clone(),
                        file_name: file_name.clone(),
                        source_type: "local".to_string(),
                        open_url: None,
                    });
                }
            }
        }
    }

    /// Search by query text. Returns results scored by name/path match quality.
    pub fn search(&self, query: &str, top_k: usize) -> Vec<LocalSearchResult> {
        let q = query.trim().to_lowercase();
        if q.is_empty() {
            return Vec::new();
        }

        let mut scored: Vec<LocalSearchResult> = self
            .entries
            .iter()
            .filter_map(|entry| {
                let score = score_match(&q, entry);
                if score > 0.0 {
                    Some(LocalSearchResult {
                        id: entry.id.clone(),
                        absolute_path: entry.absolute_path.to_string_lossy().to_string(),
                        relative_path: entry.relative_path.clone(),
                        file_name: entry.file_name.clone(),
                        source_type: entry.source_type.clone(),
                        open_url: entry.open_url.clone(),
                        score,
                    })
                } else {
                    None
                }
            })
            .collect();

        scored.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        scored.truncate(top_k);
        scored
    }

    pub fn count(&self) -> usize {
        self.entries.len()
    }
}

fn strip_root(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .to_string_lossy()
        .to_string()
}

fn path_tokens(path: &str) -> Vec<String> {
    path.split(|c: char| c == '/' || c == '\\' || c == '_' || c == '-' || c == '.')
        .filter(|s| !s.is_empty())
        .map(|s| s.to_lowercase())
        .collect()
}

fn source_type_from_path(_path: &Path) -> String {
    "local".to_string()
}

fn score_match(query: &str, entry: &LocalEntry) -> f64 {
    let name_lower = entry.file_name.to_lowercase();
    let path_lower = entry.relative_path.to_lowercase();

    // Name without extension: exact stem match is the strongest signal.
    let stem = std::path::Path::new(&name_lower)
        .file_stem()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| name_lower.clone());
    if stem == query {
        return 1.0;
    }

    // Exact full file name match.
    if name_lower == query {
        return 0.95;
    }

    // Any whole token in the file name equals the query.
    let name_tokens: Vec<String> = path_tokens(&entry.file_name);
    if name_tokens.iter().any(|t| t == query) {
        return 0.85;
    }

    // File name starts with query.
    if name_lower.starts_with(query) {
        return 0.8;
    }

    // File name contains query.
    if name_lower.contains(query) {
        return 0.7;
    }

    // Path contains query.
    if path_lower.contains(query) {
        return 0.5;
    }

    // Token prefix match (e.g. query 'inv' matches 'invoice').
    if path_tokens(&entry.relative_path)
        .iter()
        .any(|t| t.starts_with(query))
    {
        return 0.4;
    }

    0.0
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn indexes_and_searches_file_names() {
        let dir = TempDir::new().unwrap();
        let file = dir.path().join("invoice_march.pdf");
        std::fs::File::create(&file).unwrap();

        let mut index = LocalFileIndex::new();
        index.index_roots(&[dir.path().to_path_buf()], &[]);

        assert_eq!(index.count(), 1);

        let results = index.search("invoice", 10);
        assert_eq!(results.len(), 1);
        assert!(results[0].score > 0.6);
    }

    #[test]
    fn respects_excluded_directories() {
        let dir = TempDir::new().unwrap();
        let included = dir.path().join("report.pdf");
        let excluded = dir.path().join("build").join("artifact.bin");
        std::fs::create_dir_all(excluded.parent().unwrap()).unwrap();
        std::fs::File::create(&included).unwrap();
        std::fs::File::create(&excluded).unwrap();

        let mut index = LocalFileIndex::new();
        index.index_roots(&[dir.path().to_path_buf()], &["build".to_string()]);

        assert_eq!(index.count(), 1);
        let results = index.search("artifact", 10);
        assert!(results.is_empty());
    }

    #[test]
    fn exact_match_scores_highest() {
        let dir = TempDir::new().unwrap();
        std::fs::File::create(dir.path().join("budget.xlsx")).unwrap();
        std::fs::File::create(dir.path().join("budget_2024.xlsx")).unwrap();

        let mut index = LocalFileIndex::new();
        index.index_roots(&[dir.path().to_path_buf()], &[]);

        let results = index.search("budget", 10);
        assert_eq!(results.len(), 2);
        assert_eq!(results[0].file_name, "budget.xlsx");
        assert!(results[0].score > results[1].score);
    }
}
