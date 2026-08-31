use crate::config::DaemonConfig;
use crate::modules::catalog::Catalog;
use crate::modules::download::{spawn_download_worker, DownloadCommand, DownloadProgress};
use crate::modules::extract::{extract_archive, verify_extracted_files};
use crate::modules::manifest::{current_platform_key, ModuleKind, ModuleManifest};
use crate::modules::state::{ModuleInstanceState, ModuleState, PersistedState};
use crate::modules::verify::verify_ed25519;
use anyhow::{anyhow, Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::{broadcast, mpsc, RwLock};
use tokio::task::JoinHandle;

const STATE_FILE: &str = "state.json";
const CATALOG_FILE: &str = "catalog.json";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModuleStatus {
    pub module_id: String,
    pub version: Option<String>,
    pub state: ModuleState,
    pub bytes_downloaded: u64,
    pub bytes_total: u64,
    pub error: Option<String>,
    pub dependencies_ready: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModuleEvent {
    pub module_id: String,
    pub version: Option<String>,
    pub state: ModuleState,
    pub bytes_downloaded: u64,
    pub bytes_total: u64,
    pub bytes_per_second: f64,
    pub error: Option<String>,
}

#[derive(Clone)]
struct ActiveDownload {
    manifest: ModuleManifest,
    platform: crate::modules::manifest::PlatformEntry,
    part_path: PathBuf,
    extract_tmp: PathBuf,
    final_dir: PathBuf,
}

pub struct ModuleManager {
    inner: Arc<Inner>,
    _progress_handle: Arc<JoinHandle<()>>,
}

struct Inner {
    downloads_dir: PathBuf,
    models_dir: PathBuf,
    catalog_url: Option<String>,
    public_key: Option<[u8; 32]>,
    client: reqwest::Client,
    state: RwLock<BTreeMap<String, ModuleInstanceState>>,
    catalog: RwLock<Option<Catalog>>,
    active_downloads: RwLock<BTreeMap<String, ActiveDownload>>,
    events: broadcast::Sender<ModuleEvent>,
    download_cmd: mpsc::UnboundedSender<DownloadCommand>,
}

impl ModuleManager {
    pub async fn new(config: &DaemonConfig, public_key: Option<[u8; 32]>) -> Self {
        let downloads_dir = config.downloads_dir.clone();
        let models_dir = config.models_dir.clone();
        let catalog_url = config.catalog_url.clone();

        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(300))
            .build()
            .expect("failed to build HTTP client");

        let state = load_module_state(&downloads_dir);
        let cached_catalog = load_cached_catalog(&downloads_dir);
        let default_catalog = crate::modules::catalog::default_catalog();

        let merged_catalog = if let Some(cached) = cached_catalog {
            let mut merged = default_catalog;
            for module in cached.modules {
                if merged.find_module(&module.id).is_none() {
                    merged.modules.push(module);
                }
            }
            Some(merged)
        } else {
            Some(default_catalog)
        };

        let (download_cmd, progress_rx) = spawn_download_worker(client.clone());

        let (events, _) = broadcast::channel(256);

        let catalog_for_sync = merged_catalog.clone();

        let inner = Arc::new(Inner {
            downloads_dir,
            models_dir,
            catalog_url,
            public_key,
            client,
            state: RwLock::new(state),
            catalog: RwLock::new(merged_catalog),
            active_downloads: RwLock::new(BTreeMap::new()),
            events,
            download_cmd,
        });

        if let Some(ref catalog) = catalog_for_sync {
            Arc::clone(&inner).sync_catalog_to_state(catalog).await;

            #[cfg(feature = "duckdb")]
            if let Some(manifest) = catalog.find_module("duckdb") {
                inner
                    .set_state(
                        "duckdb",
                        ModuleState::Ready,
                        Some(manifest.version.clone()),
                        0,
                        0,
                        None,
                    )
                    .await;
            }

            #[cfg(feature = "onnx")]
            if let Some(manifest) = catalog.find_module("onnx_runtime") {
                inner
                    .set_state(
                        "onnx_runtime",
                        ModuleState::Ready,
                        Some(manifest.version.clone()),
                        0,
                        0,
                        None,
                    )
                    .await;
            }
        }

        let progress_handle = tokio::spawn(process_progress(Arc::clone(&inner), progress_rx));

        Self {
            inner,
            _progress_handle: Arc::new(progress_handle),
        }
    }

    pub fn subscribe_events(&self) -> broadcast::Receiver<ModuleEvent> {
        self.inner.events.subscribe()
    }

    pub async fn refresh_catalog(&self) -> Result<Catalog> {
        let url = self
            .inner
            .catalog_url
            .as_ref()
            .ok_or_else(|| anyhow!("catalog_url is not configured"))?;

        let bytes = self
            .inner
            .client
            .get(url)
            .send()
            .await
            .with_context(|| format!("failed to fetch catalog from {}", url))?
            .bytes()
            .await
            .context("failed to read catalog body")?;

        if let Some(key) = self.inner.public_key {
            let value: serde_json::Value = serde_json::from_slice(&bytes)
                .context("failed to parse catalog JSON for signature verification")?;
            let mut body = value.clone();
            if let serde_json::Value::Object(ref mut map) = body {
                map.remove("signature");
            }
            let canonical = canonical_json(&body);
            let signature_b64 = value
                .get("signature")
                .and_then(|s| s.get("signature"))
                .and_then(|s| s.as_str())
                .ok_or_else(|| anyhow!("catalog is missing signature.signature field"))?;
            verify_ed25519(canonical.as_bytes(), signature_b64, &key)
                .context("catalog signature verification failed")?;
        } else {
            tracing::warn!("no public key configured; skipping catalog signature verification");
        }

        let catalog: Catalog =
            serde_json::from_slice(&bytes).with_context(|| "failed to parse catalog JSON")?;

        save_cached_catalog(&self.inner.downloads_dir, &bytes)?;

        let mut guard = self.inner.catalog.write().await;
        *guard = Some(catalog.clone());
        drop(guard);

        self.inner.sync_catalog_to_state(&catalog).await;

        Ok(catalog)
    }

    pub async fn download_module(&self, module_id: &str, force: bool) -> Result<()> {
        let catalog = self.require_catalog().await?;
        let manifest = catalog
            .find_module(module_id)
            .ok_or_else(|| anyhow!("module {} not found in catalog", module_id))?
            .clone();

        let platform = manifest
            .platform_for_current_target()
            .cloned()
            .ok_or_else(|| {
                anyhow!(
                    "module {} has no platform entry for {}",
                    module_id,
                    current_platform_key()
                )
            })?;

        if !force {
            let state = self.inner.state.read().await;
            if let Some(s) = state.get(module_id) {
                if s.state == ModuleState::Ready {
                    return Ok(());
                }
                if matches!(
                    s.state,
                    ModuleState::Downloading | ModuleState::Queued | ModuleState::Verifying
                ) {
                    return Ok(());
                }
            }
        }

        self.ensure_dependencies(&manifest).await?;

        let final_dir = self.module_install_dir(&manifest);
        let extract_tmp = self.inner.downloads_dir.join(format!(
            ".tmp.{}.{}.{}",
            manifest.id,
            manifest.version,
            uuid::Uuid::new_v4()
        ));
        let part_path = self
            .inner
            .downloads_dir
            .join(format!("{}.{}.part", manifest.id, manifest.version));

        check_disk_space(&self.inner.downloads_dir, platform.size * 3)
            .with_context(|| "insufficient disk space for download")?;

        {
            let mut active = self.inner.active_downloads.write().await;
            active.insert(
                manifest.id.clone(),
                ActiveDownload {
                    manifest: manifest.clone(),
                    platform: platform.clone(),
                    part_path: part_path.clone(),
                    extract_tmp: extract_tmp.clone(),
                    final_dir: final_dir.clone(),
                },
            );
        }

        self.inner
            .set_state(
                module_id,
                ModuleState::Queued,
                Some(manifest.version.clone()),
                0,
                platform.size,
                None,
            )
            .await;

        self.inner
            .set_state(
                module_id,
                ModuleState::Downloading,
                Some(manifest.version.clone()),
                0,
                platform.size,
                None,
            )
            .await;

        self.inner
            .download_cmd
            .send(DownloadCommand::Download {
                module_id: module_id.to_string(),
                url: platform.url.clone(),
                expected_sha256: platform.checksum.clone(),
                expected_size: platform.size,
                part_path,
            })
            .map_err(|e| anyhow!("download worker channel closed: {}", e))?;

        Ok(())
    }

    pub async fn cancel_download(&self, module_id: &str) -> Result<()> {
        self.inner
            .download_cmd
            .send(DownloadCommand::Cancel {
                module_id: module_id.to_string(),
            })
            .map_err(|e| anyhow!("download worker channel closed: {}", e))?;

        let mut active = self.inner.active_downloads.write().await;
        if let Some(dl) = active.remove(module_id) {
            let _ = fs::remove_file(&dl.part_path);
        }
        drop(active);

        self.inner
            .set_state(module_id, ModuleState::Missing, None, 0, 0, None)
            .await;
        Ok(())
    }

    pub async fn remove_module(&self, module_id: &str) -> Result<()> {
        self.inner
            .set_state(module_id, ModuleState::Removing, None, 0, 0, None)
            .await;

        let catalog = self.inner.catalog.read().await.clone();
        if let Some(catalog) = catalog {
            if let Some(manifest) = catalog.find_module(module_id) {
                let dir = self.module_install_dir(manifest);
                if dir.exists() {
                    fs::remove_dir_all(&dir)
                        .with_context(|| format!("failed to remove {}", dir.display()))?;
                }
            }
        }

        self.inner
            .set_state(module_id, ModuleState::Missing, None, 0, 0, None)
            .await;
        Ok(())
    }

    pub async fn module_status(&self, module_id: &str) -> Option<ModuleStatus> {
        self.build_status(module_id).await
    }

    pub async fn list_modules(&self) -> Vec<ModuleStatus> {
        let catalog = self.inner.catalog.read().await.clone();
        let ids: Vec<String> = if let Some(catalog) = catalog {
            catalog.modules.iter().map(|m| m.id.clone()).collect()
        } else {
            self.inner.state.read().await.keys().cloned().collect()
        };

        let mut statuses = Vec::new();
        for id in ids {
            if let Some(status) = self.build_status(&id).await {
                statuses.push(status);
            }
        }
        statuses
    }

    pub async fn is_ready(&self, module_id: &str) -> bool {
        let state = self.inner.state.read().await;
        state
            .get(module_id)
            .map(|s| s.state == ModuleState::Ready)
            .unwrap_or(false)
    }

    async fn require_catalog(&self) -> Result<Catalog> {
        {
            let guard = self.inner.catalog.read().await;
            if let Some(catalog) = guard.clone() {
                return Ok(catalog);
            }
        }
        self.refresh_catalog().await
    }

    async fn ensure_dependencies(&self, manifest: &ModuleManifest) -> Result<()> {
        let mut visited = HashSet::new();
        self.ensure_dependencies_recursive(manifest, &mut visited)
            .await
    }

    async fn ensure_dependencies_recursive(
        &self,
        manifest: &ModuleManifest,
        visited: &mut HashSet<String>,
    ) -> Result<()> {
        if visited.len() > 32 {
            return Err(anyhow!("dependency graph too deep or circular"));
        }
        for dep in &manifest.dependencies {
            if !visited.insert(dep.clone()) {
                continue;
            }
            if !self.is_ready(dep).await {
                Box::pin(self.download_module(dep, false)).await?;
            }
            let catalog = self.require_catalog().await?;
            if let Some(dep_manifest) = catalog.find_module(dep) {
                Box::pin(self.ensure_dependencies_recursive(dep_manifest, visited)).await?;
            }
        }
        Ok(())
    }

    fn module_install_dir(&self, manifest: &ModuleManifest) -> PathBuf {
        let base = match manifest.kind {
            ModuleKind::Model => &self.inner.models_dir,
            ModuleKind::Runtime | ModuleKind::Library => &self.inner.downloads_dir,
        };
        base.join(&manifest.id).join(&manifest.version)
    }

    async fn build_status(&self, module_id: &str) -> Option<ModuleStatus> {
        let state = self.inner.state.read().await;
        let instance = state.get(module_id)?.clone();
        drop(state);

        let dependencies_ready = if let Some(catalog) = self.inner.catalog.read().await.as_ref() {
            if let Some(manifest) = catalog.find_module(module_id) {
                let mut ready = true;
                for dep in &manifest.dependencies {
                    if !self.is_ready(dep).await {
                        ready = false;
                        break;
                    }
                }
                ready
            } else {
                true
            }
        } else {
            true
        };

        Some(ModuleStatus {
            module_id: module_id.to_string(),
            version: instance.version.clone(),
            state: instance.state.clone(),
            bytes_downloaded: instance.bytes_downloaded,
            bytes_total: instance.bytes_total,
            error: instance.error.clone(),
            dependencies_ready,
        })
    }
}

impl Inner {
    async fn sync_catalog_to_state(&self, catalog: &Catalog) {
        let mut state = self.state.write().await;
        for manifest in &catalog.modules {
            if !state.contains_key(&manifest.id) {
                state.insert(manifest.id.clone(), ModuleInstanceState::new(&manifest.id));
            }
        }
        let _ = persist_module_state(&self.downloads_dir, &*state);
    }
    async fn update_progress(&self, module_id: &str, bytes_downloaded: u64, bytes_total: u64) {
        let mut state = self.state.write().await;
        if let Some(instance) = state.get_mut(module_id) {
            instance.bytes_downloaded = bytes_downloaded;
            instance.bytes_total = bytes_total;
            let event = ModuleEvent {
                module_id: module_id.to_string(),
                version: instance.version.clone(),
                state: instance.state.clone(),
                bytes_downloaded,
                bytes_total,
                bytes_per_second: 0.0,
                error: instance.error.clone(),
            };
            let _ = persist_module_state(&self.downloads_dir, &*state);
            drop(state);
            let _ = self.events.send(event);
        }
    }

    async fn set_error(&self, module_id: &str, message: String) {
        let mut state = self.state.write().await;
        if let Some(instance) = state.get_mut(module_id) {
            instance.state = ModuleState::Error;
            instance.error = Some(message.clone());
            let event = ModuleEvent {
                module_id: module_id.to_string(),
                version: instance.version.clone(),
                state: ModuleState::Error,
                bytes_downloaded: instance.bytes_downloaded,
                bytes_total: instance.bytes_total,
                bytes_per_second: 0.0,
                error: Some(message),
            };
            let _ = persist_module_state(&self.downloads_dir, &*state);
            drop(state);
            let _ = self.events.send(event);
        }
        let mut active = self.active_downloads.write().await;
        active.remove(module_id);
    }

    async fn finalize_download(&self, module_id: &str) {
        let download = {
            let active = self.active_downloads.read().await;
            active.get(module_id).cloned()
        };
        let Some(dl) = download else {
            return;
        };

        self.set_state(module_id, ModuleState::Verifying, None, 0, 0, None)
            .await;

        let result = self.extract_and_install(&dl).await;

        let mut active = self.active_downloads.write().await;
        active.remove(module_id);
        drop(active);

        match result {
            Ok(()) => {
                self.set_state(module_id, ModuleState::Ready, None, 0, 0, None)
                    .await;
            }
            Err(e) => {
                let _ = fs::remove_dir_all(&dl.extract_tmp);
                self.set_state(
                    module_id,
                    ModuleState::Error,
                    None,
                    0,
                    0,
                    Some(e.to_string()),
                )
                .await;
            }
        }
    }

    async fn extract_and_install(&self, dl: &ActiveDownload) -> Result<()> {
        let extract_tmp = dl.extract_tmp.clone();
        let part_path = dl.part_path.clone();
        let final_dir = dl.final_dir.clone();
        let manifest_id = dl.manifest.id.clone();
        let archive_format = dl.platform.archive_format.clone();
        let files = dl.platform.files.clone();

        tokio::task::spawn_blocking(move || {
            fs::create_dir_all(&extract_tmp)
                .with_context(|| format!("failed to create tmp dir {}", extract_tmp.display()))?;

            extract_archive(&part_path, &extract_tmp, &archive_format, &files)
                .with_context(|| format!("failed to extract archive for {}", manifest_id))?;

            verify_extracted_files(&extract_tmp, &files)
                .with_context(|| format!("file verification failed for {}", manifest_id))?;

            if final_dir.exists() {
                fs::remove_dir_all(&final_dir)
                    .with_context(|| format!("failed to remove old {}", final_dir.display()))?;
            }
            if let Some(parent) = final_dir.parent() {
                fs::create_dir_all(parent).context("failed to create module parent directory")?;
            }

            #[cfg(unix)]
            {
                std::fs::rename(&extract_tmp, &final_dir).with_context(|| {
                    format!(
                        "failed to rename {} to {}",
                        extract_tmp.display(),
                        final_dir.display()
                    )
                })?;
            }
            #[cfg(windows)]
            {
                move_dir_all(&extract_tmp, &final_dir)?;
            }

            let _ = fs::remove_file(&part_path);
            Ok::<(), anyhow::Error>(())
        })
        .await
        .context("extraction task panicked")?
    }

    async fn set_state(
        &self,
        module_id: &str,
        new_state: ModuleState,
        version: Option<String>,
        bytes_downloaded: u64,
        bytes_total: u64,
        error: Option<String>,
    ) {
        let mut state = self.state.write().await;
        let instance = state
            .entry(module_id.to_string())
            .or_insert_with(|| ModuleInstanceState::new(module_id));
        instance.state = new_state.clone();
        if let Some(v) = version {
            instance.version = Some(v);
        }
        instance.bytes_downloaded = bytes_downloaded;
        instance.bytes_total = bytes_total;
        instance.error = error.clone();

        let event = ModuleEvent {
            module_id: module_id.to_string(),
            version: instance.version.clone(),
            state: new_state,
            bytes_downloaded,
            bytes_total,
            bytes_per_second: 0.0,
            error,
        };
        let _ = persist_module_state(&self.downloads_dir, &*state);
        drop(state);
        let _ = self.events.send(event);
    }
}

async fn process_progress(inner: Arc<Inner>, mut progress_rx: mpsc::Receiver<DownloadProgress>) {
    while let Some(progress) = progress_rx.recv().await {
        if progress.finished {
            if let Some(error) = progress.error {
                inner.set_error(&progress.module_id, error).await;
            } else {
                inner.finalize_download(&progress.module_id).await;
            }
        } else {
            inner
                .update_progress(
                    &progress.module_id,
                    progress.bytes_downloaded,
                    progress.bytes_total,
                )
                .await;
        }
    }
}

fn canonical_json(value: &serde_json::Value) -> String {
    match value {
        serde_json::Value::Null => String::from("null"),
        serde_json::Value::Bool(b) => b.to_string(),
        serde_json::Value::Number(n) => n.to_string(),
        serde_json::Value::String(s) => serde_json::to_string(s).unwrap_or_else(|_| String::new()),
        serde_json::Value::Array(arr) => {
            let items: Vec<String> = arr.iter().map(canonical_json).collect();
            format!("[{}]", items.join(","))
        }
        serde_json::Value::Object(obj) => {
            let mut keys: Vec<&String> = obj.keys().collect();
            keys.sort();
            let items: Vec<String> = keys
                .into_iter()
                .map(|k| {
                    format!(
                        "{}:{}",
                        serde_json::to_string(k).unwrap_or_else(|_| String::new()),
                        canonical_json(&obj[k])
                    )
                })
                .collect();
            format!("{{{}}}", items.join(","))
        }
    }
}

fn check_disk_space(dir: &Path, required: u64) -> Result<()> {
    let available = fs2::available_space(dir)
        .with_context(|| format!("failed to query disk space for {}", dir.display()))?;
    if available < required {
        return Err(anyhow!(
            "insufficient disk space: required {} bytes, available {} bytes",
            required,
            available
        ));
    }
    Ok(())
}

fn load_module_state(downloads_dir: &Path) -> BTreeMap<String, ModuleInstanceState> {
    let path = downloads_dir.join(STATE_FILE);
    if !path.exists() {
        return BTreeMap::new();
    }
    match fs::read_to_string(&path)
        .and_then(|s| serde_json::from_str::<PersistedState>(&s).map_err(Into::into))
    {
        Ok(persisted) => persisted.modules,
        Err(e) => {
            tracing::warn!("failed to load module state: {}; starting fresh", e);
            BTreeMap::new()
        }
    }
}

fn persist_module_state(
    downloads_dir: &Path,
    modules: &BTreeMap<String, ModuleInstanceState>,
) -> Result<()> {
    fs::create_dir_all(downloads_dir)?;
    let path = downloads_dir.join(STATE_FILE);
    let persisted = PersistedState {
        version: String::from("1"),
        modules: modules.clone(),
    };
    let contents = serde_json::to_string_pretty(&persisted)?;
    fs::write(&path, contents)
        .with_context(|| format!("failed to write state to {}", path.display()))?;
    Ok(())
}

fn load_cached_catalog(downloads_dir: &Path) -> Option<Catalog> {
    let path = downloads_dir.join(CATALOG_FILE);
    if !path.exists() {
        return None;
    }
    match fs::read_to_string(&path)
        .and_then(|s| serde_json::from_str::<Catalog>(&s).map_err(Into::into))
    {
        Ok(catalog) => Some(catalog),
        Err(e) => {
            tracing::warn!("failed to load cached catalog: {}", e);
            None
        }
    }
}

fn save_cached_catalog(downloads_dir: &Path, bytes: &[u8]) -> Result<()> {
    fs::create_dir_all(downloads_dir)?;
    let path = downloads_dir.join(CATALOG_FILE);
    fs::write(&path, bytes)
        .with_context(|| format!("failed to write catalog to {}", path.display()))?;
    Ok(())
}

#[cfg(windows)]
fn move_dir_all(src: &Path, dst: &Path) -> Result<()> {
    fs::create_dir_all(dst.parent().unwrap_or(dst))?;
    if dst.exists() {
        fs::remove_dir_all(dst)?;
    }
    std::fs::rename(src, dst)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::DaemonConfig;
    use tempfile::TempDir;

    #[tokio::test]
    async fn manager_loads_cached_catalog() {
        let dir = TempDir::new().unwrap();
        let mut config = DaemonConfig::default();
        config.downloads_dir = dir.path().join("downloads");
        std::fs::create_dir_all(&config.downloads_dir).unwrap();
        let catalog = serde_json::json!({
            "schema_version": "1.0.0",
            "catalog_version": "test-1",
            "minimum_daemon_version": "0.1.0",
            "signature": {
                "algorithm": "ed25519",
                "public_key_fingerprint": "test",
                "signature": "test"
            },
            "modules": [
                {
                    "id": "onnx_runtime",
                    "name": "ONNX Runtime",
                    "version": "1.19.0",
                    "description": "ONNX Runtime.",
                    "kind": "runtime",
                    "license": "MIT",
                    "is_optional": true,
                    "dependencies": [],
                    "platforms": {
                        "universal": {
                            "url": "https://example.com/onnx.tar.gz",
                            "size": 1024,
                            "checksum": "0000000000000000000000000000000000000000000000000000000000000000",
                            "archive_format": "tar.gz",
                            "files": [
                                {
                                    "relative_path": "lib/libonnxruntime.dylib",
                                    "sha256": "0000000000000000000000000000000000000000000000000000000000000000",
                                    "executable": true,
                                    "required": true
                                }
                            ]
                        }
                    }
                }
            ]
        });
        std::fs::write(
            config.downloads_dir.join("catalog.json"),
            catalog.to_string(),
        )
        .unwrap();

        let bytes = std::fs::read_to_string(config.downloads_dir.join("catalog.json")).unwrap();
        let parsed: Catalog = serde_json::from_str(&bytes).expect("catalog should parse");
        assert_eq!(parsed.modules.len(), 1);

        let direct = load_cached_catalog(&config.downloads_dir);
        assert!(direct.is_some(), "load_cached_catalog should return Some");

        let manager = ModuleManager::new(&config, None).await;
        let modules = manager.list_modules().await;
        let ids: Vec<_> = modules.iter().map(|m| m.module_id.as_str()).collect();
        assert!(
            ids.contains(&"onnx_runtime"),
            "the cached catalog should be merged in"
        );
        assert!(
            ids.contains(&"clip_vision_encoder"),
            "built-in CLIP modules should appear"
        );

        let onnx = modules
            .iter()
            .find(|m| m.module_id == "onnx_runtime")
            .unwrap();
        #[cfg(feature = "onnx")]
        assert_eq!(
            onnx.state,
            ModuleState::Ready,
            "onnx_runtime should be ready when feature enabled"
        );
        #[cfg(not(feature = "onnx"))]
        assert_eq!(
            onnx.state,
            ModuleState::Missing,
            "onnx_runtime should be missing when feature disabled"
        );
    }
}
