use crate::apps::AppIndex;
use crate::db::LanceDbStore;
use crate::embeddings::Embedder;
use crate::local_index::{LocalFileIndex, LocalSearchResult};
use crate::models::{SearchResult, SearchResultCategory};
use anyhow::{Context, Result};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

/// Unified local search that combines OS apps, file name matches and semantic media search.
///
/// Ranking follows the Mirage design system: apps first, then local files, then
/// semantic results. Embeddings are used only for meaning/content search; file
/// names and app names are matched with a lightweight string index.
pub struct UnifiedSearch {
    store: Arc<LanceDbStore>,
    embedder: Arc<dyn Embedder>,
    file_index: Mutex<LocalFileIndex>,
    app_index: Mutex<AppIndex>,
    roots: Vec<PathBuf>,
    excluded_dirs: Vec<String>,
}

impl UnifiedSearch {
    pub fn new(
        store: Arc<LanceDbStore>,
        embedder: Arc<dyn Embedder>,
        roots: Vec<PathBuf>,
        excluded_dirs: Vec<String>,
    ) -> Self {
        Self {
            store,
            embedder,
            file_index: Mutex::new(LocalFileIndex::new()),
            app_index: Mutex::new(AppIndex::new()),
            roots,
            excluded_dirs,
        }
    }

    /// Scan the configured roots and rebuild the file name index.
    pub fn index_files(&self) -> Result<usize> {
        let mut index = self
            .file_index
            .lock()
            .map_err(|e| anyhow::anyhow!("failed to lock file index: {}", e))?;
        index.index_roots(&self.roots, &self.excluded_dirs);
        Ok(index.count())
    }

    /// Refresh the OS application index.
    pub fn index_apps(&self) -> Result<usize> {
        let mut index = self
            .app_index
            .lock()
            .map_err(|e| anyhow::anyhow!("failed to lock app index: {}", e))?;
        index.refresh();
        Ok(index.count())
    }

    /// Search across all local sources and rank by tier.
    pub async fn search(&self, query: &str, top_k: usize) -> Result<Vec<SearchResult>> {
        let mut results = Vec::new();

        // Tier 1: OS apps.
        {
            let index = self
                .app_index
                .lock()
                .map_err(|e| anyhow::anyhow!("failed to lock app index: {}", e))?;
            let app_results = index.search(query, top_k);
            for r in app_results {
                results.push(app_to_search_result(r));
            }
        }

        // Tier 2: file name / path matches.
        {
            let index = self
                .file_index
                .lock()
                .map_err(|e| anyhow::anyhow!("failed to lock file index: {}", e))?;
            let file_results = index.search(query, top_k);
            for r in file_results {
                results.push(file_to_search_result(r));
            }
        }

        // Tier 3: semantic search (embeddings + LanceDB).
        let query = query.trim();
        if !query.is_empty() {
            let vector = tokio::task::spawn_blocking({
                let embedder = Arc::clone(&self.embedder);
                let query = query.to_string();
                move || embedder.embed_text(&query)
            })
            .await
            .context("embedding task panicked")?
            .context("failed to embed query")?;

            let semantic = self
                .store
                .search(vector, top_k)
                .await
                .context("semantic search failed")?;
            for r in semantic {
                results.push(SearchResult {
                    id: r.record.id,
                    relative_path: r.record.relative_path,
                    score: r.score,
                    source_type: r.record.source_type,
                    category: SearchResultCategory::Semantic,
                });
            }
        }

        // Sort by tier, then by score descending.
        results.sort_by(|a, b| {
            let tier_a = category_rank(a.category);
            let tier_b = category_rank(b.category);
            let tier_order = tier_a.cmp(&tier_b);
            if tier_order != std::cmp::Ordering::Equal {
                return tier_order;
            }
            b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal)
        });

        results.truncate(top_k);
        Ok(results)
    }

    pub fn file_index_count(&self) -> Result<usize> {
        let index = self
            .file_index
            .lock()
            .map_err(|e| anyhow::anyhow!("failed to lock file index: {}", e))?;
        Ok(index.count())
    }

    pub fn app_index_count(&self) -> Result<usize> {
        let index = self
            .app_index
            .lock()
            .map_err(|e| anyhow::anyhow!("failed to lock app index: {}", e))?;
        Ok(index.count())
    }
}

fn app_to_search_result(r: crate::apps::AppSearchResult) -> SearchResult {
    SearchResult {
        id: format!("app://{}", r.app_id),
        relative_path: r.name,
        score: r.score,
        source_type: r.source,
        category: SearchResultCategory::App,
    }
}

fn file_to_search_result(r: LocalSearchResult) -> SearchResult {
    SearchResult {
        id: format!("file://{}", r.absolute_path),
        relative_path: r.relative_path,
        score: r.score,
        source_type: r.source_type,
        category: SearchResultCategory::File,
    }
}

fn category_rank(category: SearchResultCategory) -> u8 {
    match category {
        SearchResultCategory::App => 0,
        SearchResultCategory::File => 1,
        SearchResultCategory::Semantic => 2,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::LanceDbStore;
    use crate::embeddings::FallbackEmbedder;
    use crate::models::{Record, SearchResultCategory};
    use tempfile::TempDir;

    fn create_record(path: &str) -> Record {
        Record {
            id: format!("rec-{}", path),
            relative_path: path.to_string(),
            source_type: "image".to_string(),
            vector: FallbackEmbedder::new(384).embed_text(path).unwrap(),
            updated_at: "2026-01-01T00:00:00Z".to_string(),
            version: 1,
        }
    }

    #[tokio::test]
    async fn file_matches_rank_above_semantic_matches() {
        let tmp = TempDir::new().unwrap();
        let cfg = crate::config::DaemonConfig {
            data_dir: tmp.path().to_path_buf(),
            ..Default::default()
        };
        let store = Arc::new(LanceDbStore::open(&cfg).await.unwrap());
        let embedder: Arc<dyn Embedder> = Arc::new(FallbackEmbedder::new(384));

        // Seed LanceDB with a semantic record.
        store
            .upsert(vec![create_record("beach.jpg")])
            .await
            .unwrap();

        // Create a local file whose name matches the query.
        let root = tmp.path().join("root");
        std::fs::create_dir(&root).unwrap();
        std::fs::File::create(root.join("budget.xlsx")).unwrap();

        let search = UnifiedSearch::new(
            store,
            embedder,
            vec![root],
            vec![],
        );
        search.index_files().unwrap();

        let results = search.search("budget", 10).await.unwrap();
        assert!(!results.is_empty());
        assert_eq!(results[0].category, SearchResultCategory::File);
        assert_eq!(
            std::path::Path::new(&results[0].relative_path)
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_else(|| results[0].relative_path.clone()),
            "budget.xlsx"
        );

        // The semantic record should also appear, but after the file match.
        let semantic_positions: Vec<usize> = results
            .iter()
            .enumerate()
            .filter(|(_, r)| r.category == SearchResultCategory::Semantic)
            .map(|(i, _)| i)
            .collect();
        if let Some(&pos) = semantic_positions.first() {
            assert!(pos > 0, "semantic results should rank after file matches");
        }
    }
}
