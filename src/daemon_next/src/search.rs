use crate::apps::AppIndex;
use crate::config::DaemonConfig;
use crate::connectors::{ConnectorRegistry, RemoteEntry};
use crate::content::{self, MediaKind};
use crate::db::{downsample_vector, LanceDbStore};
use crate::embeddings::Embedder;
use crate::index_state::IndexState;
use crate::local_index::{scan_files, LocalFileIndex, LocalSearchResult, ScannedFile};
use crate::models::{Record, RecordWithScore, SearchResult, SearchResultCategory};
use anyhow::{Context, Result};
use chrono::Utc;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

/// Number of files read, embedded and upserted per step of a local pass.
const LOCAL_STEP: usize = 128;

/// How much wider than the visible window the semantic index is scanned.
const SEMANTIC_SCAN_FACTOR: usize = 6;
/// Ceiling on that scan, so a small window still stays cheap.
const SEMANTIC_SCAN_MAX: usize = 120;

/// Spread a run of semantic hits over the modalities it contains.
///
/// In CLIP's space a sentence is always closer to another sentence (~0.75) than
/// to a photograph (~0.28), so a raw top-k over a mixed corpus shows only
/// documents and the photographs never surface, no matter how well they match.
/// The scores are trustworthy *within* a modality, so this keeps the ordering by
/// similarity but guarantees every modality present a share of the window,
/// strongest group first.
pub fn interleave_modalities(records: Vec<RecordWithScore>, top_k: usize) -> Vec<RecordWithScore> {
    if records.len() <= top_k {
        return records;
    }
    let mut groups: Vec<(String, Vec<RecordWithScore>)> = Vec::new();
    for record in records {
        let key = record.record.modality.clone();
        match groups.iter_mut().find(|(name, _)| *name == key) {
            Some((_, bucket)) => bucket.push(record),
            None => groups.push((key, vec![record])),
        }
    }
    groups.sort_by(|a, b| {
        b.1[0]
            .score
            .partial_cmp(&a.1[0].score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    let mut cursors = vec![0usize; groups.len()];
    let mut picked = Vec::with_capacity(top_k);
    loop {
        let mut advanced = false;
        for (index, group) in groups.iter().enumerate() {
            if cursors[index] >= group.1.len() {
                continue;
            }
            picked.push(group.1[cursors[index]].clone());
            cursors[index] += 1;
            advanced = true;
            if picked.len() == top_k {
                return picked;
            }
        }
        if !advanced {
            return picked;
        }
    }
}

/// Mutable indexing inputs, changed from the Settings window.
#[derive(Debug, Clone, Default)]
pub struct IndexSettings {
    pub roots: Vec<PathBuf>,
    pub excluded_dirs: Vec<String>,
}

/// Unified local search that combines OS apps, file name matches and semantic
/// media search.
///
/// Ranking follows the Mirage design system: apps first, then local files, then
/// semantic results. Embeddings are used only for meaning/content search; file
/// names and app names are matched with a lightweight string index.
pub struct UnifiedSearch {
    store: Arc<LanceDbStore>,
    embedder: Arc<dyn Embedder>,
    file_index: Mutex<LocalFileIndex>,
    app_index: Mutex<AppIndex>,
    settings: Mutex<IndexSettings>,
    connectors: Arc<std::sync::Mutex<ConnectorRegistry>>,
    index_state: IndexState,
    memory_budget_mb: usize,
    index_batch_size: usize,
    cloud_index_limit: usize,
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
            settings: Mutex::new(IndexSettings {
                roots: config.roots.clone(),
                excluded_dirs: config.excluded_dirs.clone(),
            }),
            connectors: Arc::new(std::sync::Mutex::new(connectors)),
            index_state: IndexState::new(),
            memory_budget_mb: config.memory_budget_mb,
            index_batch_size: config.index_batch_size.max(1),
            cloud_index_limit: config.cloud_index_limit,
        }
    }

    /// Handle used to read (and mark) indexing progress.
    pub fn index_state(&self) -> IndexState {
        self.index_state.clone()
    }

    pub fn indexing_settings(&self) -> IndexSettings {
        self.settings
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone()
    }

    /// Replace the roots and excluded directories, persist them and mark the
    /// index stale. The next pass is still the user's decision.
    pub fn update_indexing_settings(
        &self,
        roots: Vec<PathBuf>,
        excluded_dirs: Vec<String>,
        config_path: &Path,
    ) -> Result<()> {
        let mut config = crate::config::DaemonConfig::load(config_path)?;
        config.roots = roots.clone();
        config.excluded_dirs = excluded_dirs.clone();
        config.save(config_path)?;

        {
            let mut settings = self.settings.lock().unwrap_or_else(|e| e.into_inner());
            settings.roots = roots;
            settings.excluded_dirs = excluded_dirs;
        }
        self.index_state.mark_stale();
        Ok(())
    }

    /// Scan the configured roots and cloud connectors and rebuild both the file
    /// name index and the semantic index.
    ///
    /// Only one pass runs at a time: a second caller gets the progress of the pass
    /// already in flight instead of starting a competing scan.
    pub async fn index_files(&self) -> Result<usize> {
        if !self.index_state.try_begin() {
            let snapshot = self.index_state.snapshot();
            tracing::info!(
                "index pass already running ({} of {:?} done), joining it",
                snapshot.indexed,
                snapshot.total
            );
            return Ok(snapshot.indexed as usize);
        }
        let outcome = self.index_pass().await;
        match &outcome {
            Ok(_) => self.index_state.finish(None),
            Err(e) => self.index_state.finish(Some(e.to_string())),
        }
        outcome
    }

    async fn index_pass(&self) -> Result<usize> {
        let state = self.index_state.clone();
        state.set_phase("Scanning files");

        let settings = self.indexing_settings();
        let roots = settings.roots.clone();
        let excluded = settings.excluded_dirs.clone();
        let files = tokio::task::spawn_blocking(move || scan_files(&roots, &excluded))
            .await
            .context("file scan task panicked")?;

        let mut total = files.len() as u64;
        state.set_total(total);

        // The lightweight name index is cheap, so it is rebuilt in one go.
        {
            let mut index = self
                .file_index
                .lock()
                .map_err(|e| anyhow::anyhow!("failed to lock file index: {}", e))?;
            index.index_scanned(&files);
        }

        // Local semantic records are rebuilt from scratch, mirroring the name index.
        self.store
            .delete_by_source_type("local")
            .await
            .context("failed to clear local semantic records")?;

        state.set_phase("Embedding local files");
        let step = self.index_batch_size.min(LOCAL_STEP);
        for chunk in files.chunks(step) {
            self.index_local_chunk(chunk).await?;
            state.add_indexed(chunk.len() as u64);
        }

        // Cloud connectors are indexed by metadata: their contents stay remote and
        // credentials never leave this device.
        let connector_handles: Vec<(String, Arc<dyn crate::connectors::CloudConnector>)> = {
            let connectors = self
                .connectors
                .lock()
                .map_err(|e| anyhow::anyhow!("failed to lock connectors: {}", e))?;
            connectors
                .iter()
                .map(|(id, conn)| (id.to_string(), Arc::clone(&conn)))
                .collect()
        };
        for (id, connector) in connector_handles {
            state.set_phase(&format!("Listing {id}"));
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
            total += entries.len() as u64;
            state.set_total(total);

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

            {
                let mut index = self
                    .file_index
                    .lock()
                    .map_err(|e| anyhow::anyhow!("failed to lock file index: {}", e))?;
                index.index_remote_entries(&id, entries.clone());
            }

            state.set_phase(&format!("Embedding {id}"));
            for chunk in entries.chunks(self.index_batch_size) {
                self.index_remote_chunk(chunk, connector.source_type())
                    .await?;
                state.add_indexed(chunk.len() as u64);
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

    /// Read, embed and store one step of local files.
    async fn index_local_chunk(&self, chunk: &[ScannedFile]) -> Result<()> {
        if chunk.is_empty() {
            return Ok(());
        }
        let records = tokio::task::spawn_blocking({
            let embedder = Arc::clone(&self.embedder);
            let budget = self.memory_budget_mb;
            let dimension = self.store.dimension();
            let chunk = chunk.to_vec();
            move || build_local_records(&embedder, &chunk, dimension, budget)
        })
        .await
        .context("local embedding task panicked")??;

        if records.is_empty() {
            return Ok(());
        }
        self.store
            .upsert_batched(records, self.index_batch_size)
            .await
            .context("failed to upsert local chunk")
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
                let dimension = self.store.dimension();
                let records: Vec<Record> = chunk
                    .iter()
                    .zip(vectors.into_iter())
                    .map(|(entry, vector)| Record {
                        id: entry.id.clone(),
                        relative_path: entry.path.clone(),
                        source_type: source_type.to_string(),
                        vector: downsample_vector(&vector, dimension),
                        updated_at: entry
                            .modified
                            .clone()
                            .unwrap_or_else(|| Utc::now().to_rfc3339()),
                        version: 1,
                        modality: MediaKind::Metadata.as_str().to_string(),
                        caption: entry.name.clone(),
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
        let trimmed = query.trim();
        if !trimmed.is_empty() {
            let vector = tokio::task::spawn_blocking({
                let embedder = Arc::clone(&self.embedder);
                let query = trimmed.to_string();
                move || embedder.embed_text(&query)
            })
            .await
            .context("embedding task panicked")?
            .context("failed to embed query")?;

            let scan = top_k
                .saturating_mul(SEMANTIC_SCAN_FACTOR)
                .clamp(top_k, SEMANTIC_SCAN_MAX);
            let semantic = self
                .store
                .search(vector, scan)
                .await
                .context("semantic search failed")?;
            for r in interleave_modalities(semantic, top_k) {
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

        // Sort by tier. Inside the semantic tier the incoming order is kept: hits
        // arrive spread over modalities and `sort_by` is stable, so comparing raw
        // cosine there would push every photograph below every document.
        results.sort_by(|a, b| {
            let tier_a = category_rank(a.category);
            let tier_b = category_rank(b.category);
            let tier_order = tier_a.cmp(&tier_b);
            if tier_order != std::cmp::Ordering::Equal {
                return tier_order;
            }
            if a.category == SearchResultCategory::Semantic {
                return std::cmp::Ordering::Equal;
            }
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        // The name index and the semantic index describe the same file, so without
        // this a hit on both would be listed twice, the second time with a much
        // weaker score.
        let mut seen = std::collections::HashSet::with_capacity(results.len());
        results.retain(|r| seen.insert(r.id.clone()));

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

    /// Replace the configured connectors and persist them.
    ///
    /// Reindexing is not triggered here: the caller decides when a pass runs, so
    /// saving a connector never starts one by itself.
    pub async fn update_connectors(
        &self,
        configs: Vec<crate::config::ConnectorConfig>,
        config_path: &Path,
    ) -> Result<usize> {
        let mut config = crate::config::DaemonConfig::load(config_path)?;
        config.connectors = configs;
        config.save(config_path)?;

        let registry = crate::connectors::registry_from_config(&config.connectors);
        {
            let mut guard = self
                .connectors
                .lock()
                .map_err(|e| anyhow::anyhow!("failed to lock connectors: {}", e))?;
            *guard = registry;
        }
        self.index_state.mark_stale();

        let index = self
            .file_index
            .lock()
            .map_err(|e| anyhow::anyhow!("failed to lock file index: {}", e))?;
        Ok(index.count())
    }

    /// Download a cloud result to a local path. For local files this copies the file.
    pub async fn download_result(&self, result: &SearchResult, dest: &Path) -> Result<()> {
        if result.source_type == "local" || result.source_type == "app" {
            let src = PathBuf::from(&result.relative_path);
            if src.exists() {
                tokio::fs::copy(&src, dest).await?;
                return Ok(());
            }
            anyhow::bail!("local file not found: {}", result.relative_path);
        }

        let connector = {
            let connectors = self
                .connectors
                .lock()
                .map_err(|e| anyhow::anyhow!("failed to lock connectors: {}", e))?;
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

/// Decide what to embed for each local file, embed it, and build the records.
///
/// Images go through the vision encoder when one is installed, text-like files are
/// embedded from an excerpt of their contents, and everything else from its name and
/// parent directory. Runs on a blocking thread because it reads files and runs
/// inference.
fn build_local_records(
    embedder: &Arc<dyn Embedder>,
    files: &[ScannedFile],
    dimension: usize,
    memory_budget_mb: usize,
) -> Result<Vec<Record>> {
    let mut kinds: Vec<MediaKind> = Vec::with_capacity(files.len());
    let mut captions: Vec<String> = Vec::with_capacity(files.len());
    let mut texts: Vec<String> = Vec::new();
    let mut text_positions: Vec<usize> = Vec::new();
    let mut images: Vec<PathBuf> = Vec::new();
    let mut image_positions: Vec<usize> = Vec::new();

    for (position, file) in files.iter().enumerate() {
        let path = &file.absolute_path;
        let label = content::name_and_parent(path);
        let excerpt = match content::media_kind(path) {
            MediaKind::Text => content::text_excerpt(path),
            _ => None,
        };
        let kind = match content::media_kind(path) {
            MediaKind::Image
                if embedder.supports_images() && file.size <= content::MAX_IMAGE_BYTES =>
            {
                MediaKind::Image
            }
            // Without a vision encoder (or for an image too large to decode) the file is
            // indexed by name; its contents are never guessed at.
            MediaKind::Image => MediaKind::Metadata,
            MediaKind::Text if excerpt.as_deref().unwrap_or("").trim().is_empty() => {
                MediaKind::Metadata
            }
            other => other,
        };
        match kind {
            MediaKind::Image => {
                images.push(path.clone());
                image_positions.push(position);
                captions.push(label);
            }
            MediaKind::Text => {
                let body = excerpt.unwrap_or_default();
                texts.push(format!("{label}\n{body}"));
                text_positions.push(position);
                captions.push(content::truncate_chars(&body, 200));
            }
            MediaKind::Metadata => {
                texts.push(label.clone());
                text_positions.push(position);
                captions.push(label);
            }
        }
        kinds.push(kind);
    }

    let mut vectors: Vec<Option<Vec<f32>>> = (0..files.len()).map(|_| None).collect();

    if !texts.is_empty() {
        let embedded = embedder.embed_texts_batched(&texts, memory_budget_mb)?;
        for (position, vector) in text_positions.iter().zip(embedded) {
            vectors[*position] = Some(downsample_vector(&vector, dimension));
        }
    }

    if !images.is_empty() {
        for (position, result) in image_positions
            .iter()
            .zip(embedder.embed_image_files(&images))
        {
            match result {
                Ok(vector) => vectors[*position] = Some(downsample_vector(&vector, dimension)),
                Err(e) => {
                    // An undecodable image is still searchable by name through the
                    // lightweight index, so it is skipped here instead of failing the pass.
                    tracing::debug!(
                        "skipping image {}: {}",
                        files[*position].absolute_path.display(),
                        e
                    );
                }
            }
        }
    }

    let now = Utc::now().to_rfc3339();
    let mut records = Vec::with_capacity(files.len());
    for (position, file) in files.iter().enumerate() {
        let Some(vector) = vectors[position].clone() else {
            continue;
        };
        records.push(Record {
            id: file.id(),
            relative_path: file.relative_path.clone(),
            source_type: "local".to_string(),
            vector,
            updated_at: now.clone(),
            version: 1,
            modality: kinds[position].as_str().to_string(),
            caption: captions[position].clone(),
        });
    }
    Ok(records)
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
    use crate::models::Record;
    use tempfile::TempDir;

    fn create_record(path: &str) -> Record {
        Record {
            id: format!("rec-{}", path),
            relative_path: path.to_string(),
            source_type: "image".to_string(),
            vector: FallbackEmbedder::new(384).embed_text(path).unwrap(),
            updated_at: "2026-01-01T00:00:00Z".to_string(),
            version: 1,
            modality: "name".to_string(),
            caption: path.to_string(),
        }
    }

    fn record_with_modality(path: &str, modality: &str, vector: Vec<f32>) -> Record {
        Record {
            vector,
            modality: modality.to_string(),
            ..create_record(path)
        }
    }

    fn config_with_roots(dir: &Path) -> DaemonConfig {
        DaemonConfig {
            data_dir: dir.join("data"),
            roots: vec![dir.join("root")],
            vector_dim: 384,
            ..Default::default()
        }
    }

    /// Embedder that records what reached the model, so tests can prove that file
    /// contents (and not just names) are embedded.
    struct SpyEmbedder {
        inner: FallbackEmbedder,
        texts: Mutex<Vec<String>>,
        images: Mutex<Vec<String>>,
        images_enabled: bool,
    }

    impl SpyEmbedder {
        fn new(images_enabled: bool) -> Self {
            Self {
                inner: FallbackEmbedder::new(384),
                texts: Mutex::new(Vec::new()),
                images: Mutex::new(Vec::new()),
                images_enabled,
            }
        }

        fn embedded_texts(&self) -> Vec<String> {
            self.texts.lock().unwrap().clone()
        }

        fn embedded_images(&self) -> Vec<String> {
            self.images.lock().unwrap().clone()
        }
    }

    impl Embedder for SpyEmbedder {
        fn embed_text(&self, text: &str) -> Result<Vec<f32>> {
            self.texts.lock().unwrap().push(text.to_string());
            self.inner.embed_text(text)
        }

        fn embed_image_file(&self, path: &Path) -> Result<Vec<f32>> {
            self.images
                .lock()
                .unwrap()
                .push(path.to_string_lossy().to_string());
            self.inner.embed_text(&path.to_string_lossy())
        }

        fn supports_images(&self) -> bool {
            self.images_enabled
        }

        fn dimension(&self) -> usize {
            384
        }
    }

    /// A mixed corpus of documents and photographs, ranked through the real merge.
    ///
    /// The vectors are built so every document scores higher than every photo,
    /// which is what CLIP does in practice; the test pins the guarantee that the
    /// photographs still take half of the visible window.
    #[tokio::test]
    async fn semantic_window_is_shared_between_modalities() {
        let tmp = TempDir::new().unwrap();
        let cfg = config_with_roots(tmp.path());
        let store = Arc::new(LanceDbStore::open(&cfg).await.unwrap());
        let embedder: Arc<dyn Embedder> = Arc::new(FallbackEmbedder::new(384));
        let query = embedder.embed_text("a photo of a beach").unwrap();
        let width = query.len();

        // A fixed direction made orthogonal to the query, so cosine between the
        // query and a record is exactly the first coefficient.
        let mut side = vec![0.0f32; width];
        side[0] = 1.0;
        let dot: f32 = side.iter().zip(&query).map(|(a, b)| a * b).sum();
        let norm: f32 = query.iter().map(|v| v * v).sum::<f32>().sqrt();
        for (slot, value) in side.iter_mut().zip(query.iter()) {
            *slot -= dot / (norm * norm) * value;
        }
        let side_norm: f32 = side.iter().map(|v| v * v).sum::<f32>().sqrt();
        for value in side.iter_mut() {
            *value /= side_norm;
        }
        let along = |cosine: f32| -> Vec<f32> {
            let sine = (1.0 - cosine * cosine).max(0.0).sqrt();
            query
                .iter()
                .zip(&side)
                .map(|(q, s)| q / norm * cosine + s * sine)
                .collect()
        };

        let mut records = Vec::new();
        for (index, cosine) in [0.92, 0.88, 0.84, 0.80].iter().enumerate() {
            records.push(record_with_modality(
                &format!("notes-{index}.md"),
                "text",
                along(*cosine),
            ));
        }
        for (index, cosine) in [0.30, 0.26, 0.22, 0.18].iter().enumerate() {
            records.push(record_with_modality(
                &format!("photo-{index}.jpg"),
                "image",
                along(*cosine),
            ));
        }
        store.upsert(records).await.unwrap();

        let search = UnifiedSearch::new(
            store,
            embedder,
            &cfg,
            crate::connectors::ConnectorRegistry::empty(),
        );
        let results = search.search("a photo of a beach", 6).await.unwrap();
        let semantic: Vec<&str> = results
            .iter()
            .filter(|r| r.category == SearchResultCategory::Semantic)
            .map(|r| r.relative_path.as_str())
            .collect();
        assert_eq!(
            semantic,
            vec![
                "notes-0.md",
                "photo-0.jpg",
                "notes-1.md",
                "photo-1.jpg",
                "notes-2.md",
                "photo-2.jpg",
            ],
            "documents and photographs should alternate by similarity rank"
        );
    }

    #[tokio::test]
    async fn file_matches_rank_above_semantic_matches() {
        let tmp = TempDir::new().unwrap();

        // Create a local file whose name matches the query.
        let root = tmp.path().join("root");
        std::fs::create_dir(&root).unwrap();
        std::fs::File::create(root.join("budget.xlsx")).unwrap();

        let cfg = config_with_roots(tmp.path());
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

    /// The name index and the semantic index both describe a file, which used to
    /// show it twice in the list.
    #[tokio::test]
    async fn a_file_matched_by_name_and_content_is_listed_once() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path().join("root");
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(root.join("annual-report.pdf"), b"quarterly revenue grew").unwrap();

        let cfg = config_with_roots(tmp.path());
        let store = Arc::new(LanceDbStore::open(&cfg).await.unwrap());
        let embedder: Arc<dyn Embedder> = Arc::new(FallbackEmbedder::new(384));
        let search = UnifiedSearch::new(
            store,
            embedder,
            &cfg,
            crate::connectors::ConnectorRegistry::empty(),
        );
        search.index_files().await.unwrap();

        let results = search.search("annual-report", 10).await.unwrap();
        let ids: Vec<&str> = results.iter().map(|r| r.id.as_str()).collect();
        let unique: std::collections::HashSet<&str> = ids.iter().copied().collect();
        assert!(!results.is_empty());
        assert_eq!(
            ids.len(),
            unique.len(),
            "the list has duplicate rows: {ids:?}"
        );
        assert_eq!(results[0].category, SearchResultCategory::File);
    }

    #[tokio::test]
    async fn a_pass_reports_progress_and_returns_to_idle() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path().join("root");
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(root.join("one.txt"), b"first file").unwrap();
        std::fs::write(root.join("two.txt"), b"second file").unwrap();

        let cfg = config_with_roots(tmp.path());
        let store = Arc::new(LanceDbStore::open(&cfg).await.unwrap());
        let search = UnifiedSearch::new(
            store,
            Arc::new(FallbackEmbedder::new(384)),
            &cfg,
            crate::connectors::ConnectorRegistry::empty(),
        );

        let before = search.index_state().snapshot();
        assert!(!before.running);
        assert_eq!(before.indexed, 0);

        let count = search.index_files().await.unwrap();
        assert_eq!(count, 2);

        let after = search.index_state().snapshot();
        assert!(!after.running);
        assert_eq!(after.total, Some(2));
        assert_eq!(after.indexed, 2);
        assert_eq!(after.phase, "Idle");
        assert_eq!(after.percent(), Some(100));
        assert!(after.last_finished_at_ms.is_some());
    }

    #[tokio::test]
    async fn a_second_call_joins_the_pass_already_running() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path().join("root");
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(root.join("one.txt"), b"first").unwrap();

        let cfg = config_with_roots(tmp.path());
        let store = Arc::new(LanceDbStore::open(&cfg).await.unwrap());
        let search = UnifiedSearch::new(
            store,
            Arc::new(FallbackEmbedder::new(384)),
            &cfg,
            crate::connectors::ConnectorRegistry::empty(),
        );

        // Claim the pass the way index_files() would, then call it again.
        assert!(search.index_state().try_begin());
        let joined = search.index_files().await.unwrap();
        assert_eq!(
            joined, 0,
            "the caller reports the running pass instead of starting one"
        );
        assert!(search.index_state().is_running());
    }

    #[tokio::test]
    async fn text_file_contents_reach_the_embedder() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path().join("root");
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(
            root.join("notes.txt"),
            b"the annual report shows twelve percent growth",
        )
        .unwrap();

        let cfg = config_with_roots(tmp.path());
        let store = Arc::new(LanceDbStore::open(&cfg).await.unwrap());
        let spy = Arc::new(SpyEmbedder::new(false));
        let search = UnifiedSearch::new(
            store,
            spy.clone() as Arc<dyn Embedder>,
            &cfg,
            crate::connectors::ConnectorRegistry::empty(),
        );
        search.index_files().await.unwrap();

        let embedded = spy.embedded_texts().join("\n");
        assert!(
            embedded.contains("annual report shows twelve percent growth"),
            "the file contents should be embedded, got: {embedded}"
        );
    }

    #[tokio::test]
    async fn images_use_the_vision_encoder_when_one_is_installed() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path().join("root");
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(root.join("photo.png"), b"not really a png").unwrap();

        let cfg = config_with_roots(tmp.path());
        let store = Arc::new(LanceDbStore::open(&cfg).await.unwrap());
        let dimension = store.dimension();

        // A vision-capable embedder gets the file handed to it as an image.
        let with_vision = Arc::new(SpyEmbedder::new(true));
        let search = UnifiedSearch::new(
            Arc::clone(&store),
            with_vision.clone() as Arc<dyn Embedder>,
            &cfg,
            crate::connectors::ConnectorRegistry::empty(),
        );
        search.index_files().await.unwrap();
        assert_eq!(with_vision.embedded_images().len(), 1);
        assert!(with_vision.embedded_texts().is_empty());

        let records = store.all_records().await.unwrap();
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].modality, "image");
        assert_eq!(records[0].caption, "photo.png root");
        assert_eq!(records[0].vector.len(), dimension);
    }

    #[tokio::test]
    async fn images_fall_back_to_their_name_without_a_vision_model() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path().join("root");
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(root.join("photo.png"), b"not really a png").unwrap();

        let cfg = config_with_roots(tmp.path());
        let store = Arc::new(LanceDbStore::open(&cfg).await.unwrap());
        let text_only = Arc::new(SpyEmbedder::new(false));
        let search = UnifiedSearch::new(
            store,
            text_only.clone() as Arc<dyn Embedder>,
            &cfg,
            crate::connectors::ConnectorRegistry::empty(),
        );
        search.index_files().await.unwrap();

        assert!(text_only.embedded_images().is_empty());
        assert!(text_only
            .embedded_texts()
            .iter()
            .any(|t| t.contains("photo.png")));
    }

    #[tokio::test]
    async fn indexing_settings_are_persisted_and_mark_the_index_stale() {
        let tmp = TempDir::new().unwrap();
        let cfg = config_with_roots(tmp.path());
        let config_path = tmp.path().join("daemon.yaml");
        cfg.save(&config_path).unwrap();

        let store = Arc::new(LanceDbStore::open(&cfg).await.unwrap());
        let search = UnifiedSearch::new(
            store,
            Arc::new(FallbackEmbedder::new(384)),
            &cfg,
            crate::connectors::ConnectorRegistry::empty(),
        );

        let root = tmp.path().join("documents");
        search
            .update_indexing_settings(
                vec![root.clone()],
                vec!["node_modules".to_string()],
                &config_path,
            )
            .unwrap();

        let settings = search.indexing_settings();
        assert_eq!(settings.roots, vec![root.clone()]);
        assert_eq!(settings.excluded_dirs, vec!["node_modules".to_string()]);
        assert!(search.index_state().snapshot().stale);

        let reloaded = crate::config::DaemonConfig::load(&config_path).unwrap();
        assert_eq!(reloaded.roots, vec![root]);
        assert_eq!(reloaded.excluded_dirs, vec!["node_modules".to_string()]);
    }

    fn semantic_hit(id: &str, modality: &str, score: f64) -> crate::models::RecordWithScore {
        crate::models::RecordWithScore {
            record: Record {
                id: id.to_string(),
                relative_path: id.to_string(),
                source_type: "local".to_string(),
                vector: vec![0.0; 4],
                updated_at: String::new(),
                version: 0,
                modality: modality.to_string(),
                caption: String::new(),
            },
            score,
        }
    }

    /// The failure mode this guards: six text records at ~0.75 crowd six photos at
    /// ~0.28 out of a top-6 window entirely.
    #[test]
    fn semantic_hits_spread_across_modalities() {
        let mut pool = Vec::new();
        for index in 0..6 {
            pool.push(semantic_hit(
                &format!("notes-{index}.md"),
                "text",
                0.75 - index as f64 * 0.01,
            ));
        }
        for index in 0..6 {
            pool.push(semantic_hit(
                &format!("photo-{index}.jpg"),
                "image",
                0.28 - index as f64 * 0.01,
            ));
        }
        let picked = interleave_modalities(pool, 6);
        let images = picked
            .iter()
            .filter(|hit| hit.record.modality == "image")
            .count();
        assert_eq!(picked.len(), 6);
        assert_eq!(images, 3, "images should keep half the window: {picked:?}",);
        assert_eq!(picked[0].record.id, "notes-0.md");
        assert_eq!(picked[1].record.id, "photo-0.jpg");
    }

    #[test]
    fn a_single_modality_keeps_the_whole_window() {
        let pool: Vec<_> = (0..6)
            .map(|index| semantic_hit(&format!("notes-{index}.md"), "text", 0.7 - index as f64))
            .collect();
        let picked = interleave_modalities(pool, 4);
        assert_eq!(picked.len(), 4);
        assert!(picked.iter().all(|hit| hit.record.modality == "text"));
    }
}
