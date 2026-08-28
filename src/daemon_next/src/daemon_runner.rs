use crate::config::DaemonConfig;
use crate::ipc::client::IpcClient;
use anyhow::{anyhow, Context, Result};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

const READY_TIMEOUT: Duration = Duration::from_secs(10);
const PING_INTERVAL: Duration = Duration::from_millis(250);

/// Ensures the Mirage daemon is running before the CLI talks to it.
///
/// The daemon is *not* an OS service; it is spawned on demand by the GUI/CLI,
/// kept alive while needed, and shut down when the parent app quits.
pub struct DaemonRunner {
    config: DaemonConfig,
    config_path: PathBuf,
    started_by_runner: bool,
}

impl DaemonRunner {
    /// Use the provided config (or default) to talk to / start the daemon.
    pub fn new(config: DaemonConfig, config_path: PathBuf) -> Self {
        Self {
            config,
            config_path,
            started_by_runner: false,
        }
    }

    /// Load the daemon config from a path, falling back to defaults.
    pub fn from_config_path(path: Option<PathBuf>) -> Result<Self> {
        let path = path.unwrap_or_else(|| DaemonConfig::base_dir().join("daemon.yaml"));
        let config = DaemonConfig::load(&path)
            .with_context(|| format!("failed to load config from {}", path.display()))?;
        Ok(Self::new(config, path))
    }

    /// Return the IPC endpoint for the current platform.
    #[cfg(unix)]
    pub fn endpoint(&self) -> &Path {
        &self.config.socket_path
    }

    #[cfg(windows)]
    pub fn endpoint(&self) -> &str {
        &self.config.pipe_name
    }

    /// Check whether the daemon is already responding.
    async fn is_running(&self) -> bool {
        #[cfg(unix)]
        let result = IpcClient::call(&self.config.socket_path, "ping", None).await;
        #[cfg(windows)]
        let result = IpcClient::call(&self.config.pipe_name, "ping", None).await;

        matches!(result, Ok(response) if response.error.is_none())
    }

    /// Start the daemon if it is not already running. Returns once it answers a ping.
    pub async fn ensure_running(&mut self) -> Result<()> {
        if self.is_running().await {
            return Ok(());
        }

        let exe = resolve_daemon_exe()
            .ok_or_else(|| anyhow!("mirage-daemon executable not found; ensure it is in the same directory as mirage or on PATH"))?;

        std::fs::create_dir_all(&self.config.data_dir)
            .with_context(|| format!("failed to create data dir {}", self.config.data_dir.display()))?;
        std::fs::create_dir_all(&self.config.models_dir)
            .with_context(|| format!("failed to create models dir {}", self.config.models_dir.display()))?;
        std::fs::create_dir_all(&self.config.downloads_dir)
            .with_context(|| format!("failed to create downloads dir {}", self.config.downloads_dir.display()))?;

        let mut child = Command::new(&exe)
            .arg("--data-dir")
            .arg(&self.config.data_dir)
            .arg("--models-dir")
            .arg(&self.config.models_dir)
            .arg("--downloads-dir")
            .arg(&self.config.downloads_dir)
            .arg("--config")
            .arg(&self.config_path)
            .stdin(Stdio::null())
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .spawn()
            .with_context(|| format!("failed to spawn daemon from {}", exe.display()))?;

        self.started_by_runner = true;

        // Give the child a moment to fail loudly if something is wrong.
        tokio::time::sleep(Duration::from_millis(300)).await;
        if let Some(status) = child.try_wait()? {
            anyhow::bail!("daemon exited immediately with code {}", status.code().unwrap_or(-1));
        }

        let start = Instant::now();
        while start.elapsed() < READY_TIMEOUT {
            if self.is_running().await {
                return Ok(());
            }
            tokio::time::sleep(PING_INTERVAL).await;
        }

        anyhow::bail!("daemon did not become ready within {:?}", READY_TIMEOUT)
    }
}

impl Drop for DaemonRunner {
    fn drop(&mut self) {
        // If the CLI started the daemon, leave it running. The GUI owns
        // shutdown; the CLI is stateless and should not kill a daemon that
        // might also be serving the GUI or another CLI invocation.
        let _ = self.started_by_runner;
    }
}

fn resolve_daemon_exe() -> Option<PathBuf> {
    if let Ok(exe) = std::env::var("MIRAGE_DAEMON_EXE") {
        let path = PathBuf::from(exe);
        if path.exists() && is_executable(&path) {
            return Some(path);
        }
    }

    // Sibling binary: mirage-daemon next to the mirage CLI.
    if let Ok(current_exe) = std::env::current_exe() {
        if let Some(dir) = current_exe.parent() {
            let sibling = dir.join("mirage-daemon");
            #[cfg(windows)]
            let sibling = dir.join("mirage-daemon.exe");
            if sibling.exists() && is_executable(&sibling) {
                return Some(sibling);
            }
        }
    }

    // PATH.
    if let Ok(path_var) = std::env::var("PATH") {
        for dir in path_var.split(std::path::MAIN_SEPARATOR) {
            let candidate = PathBuf::from(dir).join("mirage-daemon");
            #[cfg(windows)]
            let candidate = PathBuf::from(dir).join("mirage-daemon.exe");
            if candidate.exists() && is_executable(&candidate) {
                return Some(candidate);
            }
        }
    }

    None
}

#[cfg(unix)]
fn is_executable(path: &Path) -> bool {
    use std::os::unix::fs::PermissionsExt;
    std::fs::metadata(path)
        .map(|m| m.permissions().mode() & 0o111 != 0)
        .unwrap_or(false)
}

#[cfg(windows)]
fn is_executable(path: &Path) -> bool {
    path.is_file()
}
