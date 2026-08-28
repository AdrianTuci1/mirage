use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Lightweight index of locally installed OS applications.
///
/// Apps are intentionally ranked above files and semantic media in unified
/// search, matching the Mirage design system.
#[derive(Debug, Default, Clone)]
pub struct AppIndex {
    apps: Vec<AppEntry>,
    name_to_indices: HashMap<String, Vec<usize>>,
}

#[derive(Debug, Clone)]
pub struct AppEntry {
    pub name: String,
    pub app_id: String,
    pub executable_path: Option<PathBuf>,
    pub icon_path: Option<PathBuf>,
    pub source: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AppSearchResult {
    pub name: String,
    pub app_id: String,
    pub executable_path: Option<String>,
    pub icon_path: Option<String>,
    pub source: String,
    pub score: f64,
}

impl AppIndex {
    pub fn new() -> Self {
        Self::default()
    }

    /// Rebuild the app index by scanning OS-specific application directories.
    pub fn refresh(&mut self) {
        self.apps.clear();
        self.name_to_indices.clear();

        #[cfg(target_os = "macos")]
        self.scan_macos();

        #[cfg(target_os = "windows")]
        self.scan_windows();

        #[cfg(target_os = "linux")]
        self.scan_linux();

        for (idx, entry) in self.apps.iter().enumerate() {
            for token in path_tokens(&entry.name) {
                self.name_to_indices.entry(token).or_default().push(idx);
            }
        }
    }

    #[cfg(target_os = "macos")]
    fn scan_macos(&mut self) {
        let roots = [
            PathBuf::from("/Applications"),
            dirs::home_dir()
                .map(|h| h.join("Applications"))
                .unwrap_or_default(),
            PathBuf::from("/System/Applications"),
        ];
        for root in roots {
            if let Ok(entries) = std::fs::read_dir(&root) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.extension().and_then(|e| e.to_str()) == Some("app") {
                        if let Some(name) = macos_app_name(&path) {
                            self.apps.push(AppEntry {
                                app_id: path
                                    .file_stem()
                                    .map(|s| s.to_string_lossy().to_string())
                                    .unwrap_or_else(|| name.clone()),
                                name,
                                executable_path: Some(path.clone()),
                                icon_path: macos_app_icon(&path),
                                source: "app".to_string(),
                            });
                        }
                    }
                }
            }
        }
    }

    #[cfg(target_os = "windows")]
    fn scan_windows(&mut self) {
        let mut roots = Vec::new();
        if let Some(program_data) = std::env::var_os("ProgramData") {
            roots.push(
                PathBuf::from(program_data)
                    .join("Microsoft")
                    .join("Windows")
                    .join("Start Menu")
                    .join("Programs"),
            );
        }
        if let Some(app_data) = std::env::var_os("APPDATA") {
            roots.push(
                PathBuf::from(app_data)
                    .join("Microsoft")
                    .join("Windows")
                    .join("Start Menu")
                    .join("Programs"),
            );
        }
        for root in roots {
            self.collect_windows_shortcuts(&root);
        }
    }

    #[cfg(target_os = "windows")]
    fn collect_windows_shortcuts(&mut self, dir: &Path) {
        let Ok(entries) = std::fs::read_dir(dir) else {
            return;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                self.collect_windows_shortcuts(&path);
            } else if path
                .extension()
                .and_then(|e| e.to_str())
                .map(|e| e.to_lowercase())
                == Some("lnk".to_string())
            {
                let name = path
                    .file_stem()
                    .map(|s| s.to_string_lossy().to_string())
                    .unwrap_or_default();
                if !name.is_empty() {
                    self.apps.push(AppEntry {
                        app_id: name.clone(),
                        name,
                        executable_path: Some(path.clone()),
                        icon_path: None,
                        source: "app".to_string(),
                    });
                }
            }
        }
    }

    #[cfg(target_os = "linux")]
    fn scan_linux(&mut self) {
        let roots = [
            PathBuf::from("/usr/share/applications"),
            PathBuf::from("/usr/local/share/applications"),
            dirs::home_dir()
                .map(|h| h.join(".local").join("share").join("applications"))
                .unwrap_or_default(),
        ];
        for root in roots {
            if let Ok(entries) = std::fs::read_dir(&root) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.extension().and_then(|e| e.to_str()) == Some("desktop") {
                        if let Some((name, icon)) = linux_desktop_name(&path) {
                            let app_id = path
                                .file_stem()
                                .map(|s| s.to_string_lossy().to_string())
                                .unwrap_or_else(|| name.clone());
                            self.apps.push(AppEntry {
                                app_id,
                                name,
                                executable_path: None,
                                icon_path: icon.map(PathBuf::from),
                                source: "app".to_string(),
                            });
                        }
                    }
                }
            }
        }
    }

    pub fn search(&self, query: &str, top_k: usize) -> Vec<AppSearchResult> {
        let q = query.trim().to_lowercase();
        if q.is_empty() {
            return Vec::new();
        }

        let mut scored: Vec<AppSearchResult> = self
            .apps
            .iter()
            .filter_map(|entry| {
                let score = score_match(&q, entry);
                if score > 0.0 {
                    Some(AppSearchResult {
                        name: entry.name.clone(),
                        app_id: entry.app_id.clone(),
                        executable_path: entry
                            .executable_path
                            .as_ref()
                            .map(|p| p.to_string_lossy().to_string()),
                        icon_path: entry
                            .icon_path
                            .as_ref()
                            .map(|p| p.to_string_lossy().to_string()),
                        source: entry.source.clone(),
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
        self.apps.len()
    }
}

#[cfg(target_os = "macos")]
fn macos_app_name(app_bundle: &Path) -> Option<String> {
    let plist = app_bundle.join("Contents").join("Info.plist");
    let contents = std::fs::read_to_string(&plist).ok()?;
    parse_plist_string(&contents, "CFBundleDisplayName")
        .or_else(|| parse_plist_string(&contents, "CFBundleName"))
        .or_else(|| {
            app_bundle
                .file_stem()
                .map(|s| s.to_string_lossy().to_string())
        })
}

#[cfg(target_os = "macos")]
fn macos_app_icon(app_bundle: &Path) -> Option<PathBuf> {
    let plist = app_bundle.join("Contents").join("Info.plist");
    let contents = std::fs::read_to_string(&plist).ok()?;
    let icon_name = parse_plist_string(&contents, "CFBundleIconFile")?;
    let resources = app_bundle.join("Contents").join("Resources");
    // Common macOS icon extensions.
    for ext in ["icns", "png", "tiff"] {
        let candidate = resources.join(format!("{}.{}", icon_name, ext));
        if candidate.exists() {
            return Some(candidate);
        }
    }
    // The plist sometimes already includes the extension.
    let candidate = resources.join(&icon_name);
    if candidate.exists() {
        Some(candidate)
    } else {
        None
    }
}

#[cfg(target_os = "macos")]
fn parse_plist_string(contents: &str, key: &str) -> Option<String> {
    // Minimal parser for the common `<key>NAME</key><string>VALUE</string>` form.
    let key_tag = format!("<key>{}</key>", key);
    let pos = contents.find(&key_tag)?;
    let after_key = &contents[pos + key_tag.len()..];
    let start = after_key.find("<string>")? + "<string>".len();
    let end = after_key[start..].find("</string>")?;
    Some(after_key[start..start + end].trim().to_string())
}

#[cfg(target_os = "linux")]
fn linux_desktop_name(path: &Path) -> Option<(String, Option<String>)> {
    let contents = std::fs::read_to_string(path).ok()?;
    let mut name = None;
    let mut icon = None;
    for line in contents.lines() {
        if let Some(value) = line.strip_prefix("Name=") {
            name = Some(value.to_string());
        } else if let Some(value) = line.strip_prefix("Icon=") {
            icon = Some(value.to_string());
        }
        if name.is_some() && icon.is_some() {
            break;
        }
    }
    Some((name?, icon))
}

fn path_tokens(name: &str) -> Vec<String> {
    name.split(|c: char| c.is_whitespace() || c == '_' || c == '-')
        .filter(|s| !s.is_empty())
        .map(|s| s.to_lowercase())
        .collect()
}

fn score_match(query: &str, entry: &AppEntry) -> f64 {
    let name_lower = entry.name.to_lowercase();

    if name_lower == query {
        return 1.0;
    }

    let tokens = path_tokens(&entry.name);
    if tokens.iter().any(|t| t == query) {
        return 0.9;
    }

    if name_lower.starts_with(query) {
        return 0.85;
    }

    if name_lower.contains(query) {
        return 0.75;
    }

    if tokens.iter().any(|t| t.starts_with(query)) {
        return 0.6;
    }

    0.0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scores_app_name_matches() {
        let mut index = AppIndex::new();
        index.apps = vec![
            AppEntry {
                name: "Safari".to_string(),
                app_id: "safari".to_string(),
                executable_path: Some(PathBuf::from("/Applications/Safari.app")),
                icon_path: None,
                source: "app".to_string(),
            },
            AppEntry {
                name: "Safari Technology Preview".to_string(),
                app_id: "safari-technology-preview".to_string(),
                executable_path: Some(PathBuf::from("/Applications/Safari Technology Preview.app")),
                icon_path: None,
                source: "app".to_string(),
            },
        ];
        index.name_to_indices = HashMap::new();
        for (idx, entry) in index.apps.iter().enumerate() {
            for token in path_tokens(&entry.name) {
                index.name_to_indices.entry(token).or_default().push(idx);
            }
        }

        let results = index.search("safari", 10);
        assert_eq!(results.len(), 2);
        assert_eq!(results[0].name, "Safari");
        assert!(results[0].score > results[1].score);
    }
}
