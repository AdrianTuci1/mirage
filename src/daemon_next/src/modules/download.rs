use anyhow::{anyhow, Context, Result};
use futures::StreamExt;
use reqwest::header::{HeaderMap, HeaderValue, RANGE};
use sha2::{Digest, Sha256};
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use tokio::sync::mpsc;

/// Command sent to the background download worker.
#[derive(Debug)]
pub enum DownloadCommand {
    Download {
        module_id: String,
        url: String,
        expected_sha256: String,
        expected_size: u64,
        part_path: PathBuf,
    },
    Cancel {
        module_id: String,
    },
}

/// Progress update emitted by the download task.
#[derive(Debug, Clone)]
pub struct DownloadProgress {
    pub module_id: String,
    pub bytes_downloaded: u64,
    pub bytes_total: u64,
    pub finished: bool,
    pub error: Option<String>,
}

impl DownloadProgress {
    fn in_progress(module_id: String, bytes_downloaded: u64, bytes_total: u64) -> Self {
        Self {
            module_id,
            bytes_downloaded,
            bytes_total,
            finished: false,
            error: None,
        }
    }

    fn done(module_id: String) -> Self {
        Self {
            module_id,
            bytes_downloaded: 0,
            bytes_total: 0,
            finished: true,
            error: None,
        }
    }

    fn failed(module_id: String, error: String, bytes_total: u64) -> Self {
        Self {
            module_id,
            bytes_downloaded: 0,
            bytes_total,
            finished: true,
            error: Some(error),
        }
    }
}

struct CurrentJob {
    module_id: String,
    cancel: Arc<AtomicBool>,
}

/// Spawn a single background worker that processes download commands.
///
/// The worker owns the reqwest client and can run one download at a time.
/// Commands for other modules while busy are ignored with a logged warning.
#[allow(unused_assignments)]
pub fn spawn_download_worker(
    client: reqwest::Client,
) -> (mpsc::UnboundedSender<DownloadCommand>, mpsc::Receiver<DownloadProgress>) {
    let (cmd_tx, mut cmd_rx) = mpsc::unbounded_channel::<DownloadCommand>();
    let (progress_tx, progress_rx) = mpsc::channel::<DownloadProgress>(256);

    tokio::spawn(async move {
        let mut current_job: Option<CurrentJob> = None;

        while let Some(cmd) = cmd_rx.recv().await {
            match cmd {
                DownloadCommand::Download {
                    module_id,
                    url,
                    expected_sha256,
                    expected_size,
                    part_path,
                } => {
                    if current_job.is_some() {
                        tracing::warn!(
                            "download worker busy with {}; ignoring request for {}",
                            current_job.as_ref().unwrap().module_id,
                            module_id
                        );
                        let _ = progress_tx.send(DownloadProgress::failed(
                            module_id,
                            String::from("download worker busy"),
                            expected_size,
                        ));
                        continue;
                    }

                    let cancel_flag = Arc::new(AtomicBool::new(false));
                    current_job = Some(CurrentJob {
                        module_id: module_id.clone(),
                        cancel: Arc::clone(&cancel_flag),
                    });

                    let client = client.clone();
                    let progress_tx = progress_tx.clone();
                    let handle = tokio::spawn(async move {
                        let result = download_with_resume(
                            &module_id,
                            &client,
                            &url,
                            expected_size,
                            &part_path,
                            &expected_sha256,
                            cancel_flag,
                            progress_tx.clone(),
                        )
                        .await;

                        let progress = match result {
                            Ok(()) => DownloadProgress::done(module_id.clone()),
                            Err(e) => DownloadProgress::failed(module_id.clone(), e.to_string(), expected_size),
                        };
                        let _ = progress_tx.send(progress);
                    });

                    let _ = handle.await;
                    current_job = None;
                }
                DownloadCommand::Cancel { module_id } => {
                    if current_job.as_ref().map(|j| &j.module_id) == Some(&module_id) {
                        if let Some(job) = current_job.take() {
                            job.cancel.store(true, Ordering::Relaxed);
                        }
                    } else {
                        tracing::warn!(
                            "received cancel for {} but worker is idle or busy with another module",
                            module_id
                        );
                    }
                }
            }
        }
    });

    (cmd_tx, progress_rx)
}

async fn download_with_resume(
    module_id: &str,
    client: &reqwest::Client,
    url: &str,
    expected_size: u64,
    part_path: &Path,
    expected_sha256: &str,
    cancel: Arc<AtomicBool>,
    progress_tx: mpsc::Sender<DownloadProgress>,
) -> Result<()> {
    let existing_offset = if part_path.exists() {
        fs::metadata(part_path)
            .map(|m| m.len())
            .unwrap_or(0)
    } else {
        0
    };

    let mut headers = HeaderMap::new();
    if existing_offset > 0 && existing_offset < expected_size {
        let range_value = format!("bytes={}-", existing_offset);
        headers.insert(RANGE, HeaderValue::from_str(&range_value)?);
    }

    let request = client.get(url).headers(headers);
    let response = request
        .send()
        .await
        .with_context(|| format!("failed to start download from {}", url))?;

    let status = response.status();
    if !status.is_success() && status.as_u16() != 206 {
        return Err(anyhow!("download returned HTTP {}", status));
    }

    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(part_path)
        .with_context(|| format!("failed to open part file {}", part_path.display()))?;

    let mut stream = response.bytes_stream();
    let bytes_downloaded = Arc::new(AtomicU64::new(existing_offset));

    let progress_module_id = module_id.to_string();
    let progress_bytes = Arc::clone(&bytes_downloaded);
    let progress_task = tokio::spawn(async move {
        loop {
            tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;
            let current = progress_bytes.load(Ordering::Relaxed);
            let _ = progress_tx.send(DownloadProgress::in_progress(
                progress_module_id.clone(),
                current,
                expected_size,
            ));
        }
    });

    while let Some(chunk) = stream.next().await {
        if cancel.load(Ordering::Relaxed) {
            let _ = progress_task.abort();
            return Err(anyhow!("download cancelled by user"));
        }

        let chunk = chunk.context("failed to read download chunk")?;
        file.write_all(&chunk)
            .with_context(|| format!("failed to write chunk to {}", part_path.display()))?;
        bytes_downloaded.fetch_add(chunk.len() as u64, Ordering::Relaxed);
    }

    let _ = progress_task.abort();
    drop(file);

    let final_size = fs::metadata(part_path)
        .map(|m| m.len())
        .unwrap_or(0);
    if final_size != expected_size {
        tracing::warn!(
            "downloaded size {} does not match expected size {}",
            final_size,
            expected_size
        );
    }

    let actual_hash = hash_file_sha256(part_path)?;
    if actual_hash != expected_sha256 {
        return Err(anyhow!(
            "archive checksum mismatch: expected {}, got {}",
            expected_sha256,
            actual_hash
        ));
    }

    Ok(())
}

fn hash_file_sha256(path: &Path) -> Result<String> {
    use std::io::Read;
    let mut file = fs::File::open(path)
        .with_context(|| format!("failed to open file for hashing {}", path.display()))?;
    let mut hasher = Sha256::new();
    let mut buf = [0u8; 8192];
    loop {
        let n = file.read(&mut buf)?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    Ok(hex::encode(hasher.finalize()))
}
