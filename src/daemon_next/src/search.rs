use crate::apps::AppIndex;
use crate::config::DaemonConfig;
use crate::connectors::{ConnectorRegistry, RemoteEntry};
use crate::db::{downsample_vector, LanceDbStore};
use crate::embeddings::Embedder;
use crate::local_index::{LocalFileIndex, LocalSearchResult};
use crate::models::{Record, SearchResult, SearchResultCategory};
use anyhow::{Context, Result};
use chrono::Utc;
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
    connectors: Arc<std::sync::Mutex<ConnectorRegistry>>,
    memory_budget_mb: usize,
    index_batch_size: usize,
    cloud_index_limit: usize,
    vector_dim: usize,
}

impl UnifiedSearch {
    pub fn new(
        store: Arc<LanceDbStore>,
        embedder: Arc<dyn Embedder>,
        config: &DaemonConfig,
        connectors: ConnectorRegistry,
    ) -> Self {
        Self {
            store,
            embedder,
            file_index: Mutex::new(LocalFileIndex::new()),
            app_index: Mutex::new(AppIndex::new()),
            roots: config.roots.clone(),
            excluded_dirs: config.excluded_dirs.clone(),
            connectors: Arc::new(std::sync::Mutex::new(connectors)),
            memory_budget_mb: config.memory_budget_mb,
            index_batch_size: config.index_batch_size.max(1),
            cloud_index_limit: config.cloud_index_limit,
            vector_dim: config.vector_dim.max(1),
        }
    }

    /// Scan the configured roots and configured cloud connectors and rebuild the
    /// file name index. Cloud connectors are indexed by metadata and their paths
    /// are embedded and stored in LanceDB in memory-bounded batches.
    pub async fn index_files(&self) -> Result<usize> {
        {
            let mut index = self
                .file_index
                .lock()
                .map_err(|e| anyhow::anyhow!("failed to lock file index: {}", e))?;
            // Reset remote entries before re-indexing.
            index.clear_remote_entries();
            index.index_roots(&self.roots, &self.excluded_dirs);
        }

        // Index configured cloud sources by metadata and by semantic path
        // embeddings. Collect connector handles without holding the lock across
        // await points.
        let connector_handles: Vec<(String, Arc<dyn crate::connectors::CloudConnector>)> = {
            let connectors = self.connectors.lock().unwrap();
            connectors
                .iter()
                .map(|(id, conn)| (id.to_string(), Arc::clone(&conn)))
                .collect()
        };
        for (id, connector) in connector_handles {
            tracing::info!("indexing connector {}", id);
            let entries = match connector.list_entries("").await {
                Ok(entries) => {
                    if entries.len() > self.cloud_index_limit {
                        tracing::info!(
                            "connector {} entries truncated from {} to {}",
                            id,
                            entries.len(),
                            self.cloud_index_limit
                        );
                    }
                    entries
                        .into_iter()
                        .take(self.cloud_index_limit)
                        .collect::<Vec<_>>()
                }
                Err(e) => {
                    tracing::warn!("connector {} list failed: {}", id, e);
                    continue;
                }
            };
            tracing::info!("connector {} listed {} entries", id, entries.len());

            // Remove stale semantic records for this source type before reindexing.
            if let Err(e) = self
                .store
                .delete_by_source_type(connector.source_type())
                .await
            {
                tracing::warn!(
                    "failed to delete old semantic records for {}: {}",
                    connector.source_type(),
                    e
                );
            }

            // Build the lightweight metadata index once per connector.
            {
                let mut index = self
                    .file_index
                    .lock()
                    .map_err(|e| anyhow::anyhow!("failed to lock file index: {}", e))?;
                index.index_remote_entries(&id, entries.clone());
            }

            // Embed and upsert in batches so memory stays within the configured
            // budget regardless of how many entries the connector returns.
            let batch_size = self.index_batch_size;
            for chunk in entries.chunks(batch_size) {
                self.index_remote_chunk(chunk, connector.source_type())
                    .await?;
            }
        }

        {
            let index = self
                .file_index
                .lock()
                .map_err(|e| anyhow::anyhow!("failed to lock file index: {}", e))?;
            Ok(index.count())
        }
    }

    /// Embed and upsert one chunk of remote entries into LanceDB.
    async fn index_remote_chunk(&self, chunk: &[RemoteEntry], source_type: &str) -> Result<()> {
        let texts: Vec<String> = chunk
            .iter()
            .map(|e| format!("{} {}", e.name, e.path))
            .collect();

        let vectors = tokio::task::spawn_blocking({
            let embedder = Arc::clone(&self.embedder);
            let budget = self.memory_budget_mb;
            move || embedder.embed_texts_batched(&texts, budget)
        })
        .await
        .context("embedding task panicked")?;

        match vectors {
            Ok(vectors) => {
                let records: Vec<Record> = chunk
                    .iter()
                    .zip(vectors.into_iter())
                    .map(|(entry, vector)| Record {
                        id: entry.id.clone(),
                        relative_path: entry.path.clone(),
                        source_type: source_type.to_string(),
                        vector: downsample_vector(&vector, self.vector_dim),
                        updated_at: entry
                            .modified
                            .clone()
                            .unwrap_or_else(|| Utc::now().to_rfc3339()),
                        version: 1,
                    })
                    .collect();

                self.store
                    .upsert_batched(records, self.index_batch_size)
                    .await
                    .context("failed to upsert remote chunk")?;
            }
            Err(e) => {
                tracing::warn!("failed to embed remote chunk for {}: {}", source_type, e);
            }
        }
        Ok(())
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
                    open_url: None,
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
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        results.truncate(top_k);
        Ok(results)
    }

    /// Return the currently configured connector metadata (without credentials).
    pub fn list_connectors(&self) -> Result<Vec<crate::config::ConnectorConfig>> {
        let connectors = self
            .connectors
            .lock()
            .map_err(|e| anyhow::anyhow!("failed to lock connectors: {}", e))?
            .iter()
            .map(|(id, conn)| crate::config::ConnectorConfig {
                id: id.to_string(),
                name: conn.name().to_string(),
                kind: match conn.source_type() {
                    "s3" => crate::config::ConnectorKind::S3,
                    "dropbox" => crate::config::ConnectorKind::Dropbox,
                    "gdrive" => crate::config::ConnectorKind::GoogleDrive,
                    "smb" => crate::config::ConnectorKind::Smb,
                    _ => crate::config::ConnectorKind::S3,
                },
                enabled: true,
                roots: Vec::new(),
                credentials: crate::config::ConnectorCredentials::default(),
            })
            .collect();
        Ok(connectors)
    }

    pub fn file_index_count(&self) -> Result<usize> {
        let index = self
            .file_index
            .lock()
            .map_err(|e| anyhow::anyhow!("failed to lock file index: {}", e))?;
        Ok(index.count())
    }

    /// Replace the configured connectors, persist them to disk and reindex.
    pub async fn update_connectors(
        &self,
        configs: Vec<crate::config::ConnectorConfig>,
        config_path: &std::path::Path,
    ) -> Result<usize> {
        let mut config = crate::config::DaemonConfig::load(config_path)?;
        config.connectors = configs;
        config.save(config_path)?;

        let registry = crate::connectors::registry_from_config(&config.connectors);
        {
            let mut guard = self.connectors.lock().unwrap();
            *guard = registry;
        }

        self.index_files().await
    }

    /// Download a cloud result to a local path. For local files this copies the file.
    pub async fn download_result(
        &self,
        result: &SearchResult,
        dest: &std::path::Path,
    ) -> Result<()> {
        if result.source_type == "local" || result.source_type == "app" {
            let src = std::path::PathBuf::from(&result.relative_path);
            if src.exists() {
                tokio::fs::copy(&src, dest).await?;
                return Ok(());
            }
            anyhow::bail!("local file not found: {}", result.relative_path);
        }

        let connector = {
            let connectors = self.connectors.lock().unwrap();
            connectors
                .get(&result.source_type)
                .with_context(|| format!("connector {} not found", result.source_type))?
        };
        let name = result
            .relative_path
            .split('/')
            .next_back()
            .unwrap_or(&result.relative_path)
            .to_string();
        let entry = crate::connectors::RemoteEntry {
            id: result.id.clone(),
            path: result.relative_path.clone(),
            name,
            size: 0,
            modified: None,
            content_type: None,
            open_url: result.open_url.clone(),
        };
        connector.download(&entry, dest).await
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
        open_url: None,
    }
}

fn file_to_search_result(r: LocalSearchResult) -> SearchResult {
    SearchResult {
        id: r.id,
        relative_path: r.relative_path,
        score: r.score,
        source_type: r.source_type,
        category: SearchResultCategory::File,
        open_url: r.open_url,
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

        // Create a local file whose name matches the query.
        let root = tmp.path().join("root");
        std::fs::create_dir(&root).unwrap();
        std::fs::File::create(root.join("budget.xlsx")).unwrap();

        let cfg = crate::config::DaemonConfig {
            data_dir: tmp.path().to_path_buf(),
            roots: vec![root.clone()],
            ..Default::default()
        };
        let store = Arc::new(LanceDbStore::open(&cfg).await.unwrap());
        let embedder: Arc<dyn Embedder> = Arc::new(FallbackEmbedder::new(384));

        // Seed LanceDB with a semantic record.
        store
            .upsert(vec![create_record("beach.jpg")])
            .await
            .unwrap();

        let search = UnifiedSearch::new(
            store,
            embedder,
            &cfg,
            crate::connectors::ConnectorRegistry::empty(),
        );
        search.index_files().await.unwrap();

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
