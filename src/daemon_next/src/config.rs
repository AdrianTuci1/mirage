use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct ModulesConfig {
    pub vector: bool,
    pub text: bool,
    pub tabular: bool,
    pub audio: bool,
    pub vision: bool,
}

impl Default for ModulesConfig {
    fn default() -> Self {
        Self {
            vector: true,
            text: true,
            tabular: true,
            audio: false,
            vision: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct DaemonConfig {
    pub data_dir: PathBuf,
    pub models_dir: PathBuf,
    pub downloads_dir: PathBuf,
    #[cfg(unix)]
    pub socket_path: PathBuf,
    #[cfg(windows)]
    pub pipe_name: String,
    pub log_level: String,
    pub catalog_url: Option<String>,
    pub modules: ModulesConfig,
    /// Local roots to index for file name / path search.
    pub roots: Vec<PathBuf>,
    /// Directory names to skip while scanning local roots.
    pub excluded_dirs: Vec<String>,
}

impl DaemonConfig {
    pub fn base_dir() -> PathBuf {
        std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(Path::to_path_buf))
            .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")))
    }

    #[cfg(unix)]
    fn default_socket_path() -> PathBuf {
        Self::base_dir().join("mirage.sock")
    }

    #[cfg(windows)]
    fn default_pipe_name() -> String {
        String::from("MirageDaemon")
    }
}

impl Default for DaemonConfig {
    fn default() -> Self {
        let base = Self::base_dir();
        let home = std::env::var("HOME")
            .or_else(|_| std::env::var("USERPROFILE"))
            .map(PathBuf::from)
            .unwrap_or_else(|_| base.clone());
        Self {
            data_dir: base.join("data"),
            models_dir: base.join("models"),
            downloads_dir: base.join("downloads"),
            #[cfg(unix)]
            socket_path: Self::default_socket_path(),
            #[cfg(windows)]
            pipe_name: Self::default_pipe_name(),
            log_level: String::from("info"),
            catalog_url: None,
            modules: ModulesConfig::default(),
            roots: vec![home],
            excluded_dirs: vec![
                "node_modules".to_string(),
                ".git".to_string(),
                "target".to_string(),
                "build".to_string(),
                ".cache".to_string(),
            ],
        }
    }
}

impl DaemonConfig {
    pub fn load(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        if !path.exists() {
            return Ok(Self::default());
        }
        let contents = std::fs::read_to_string(path)
            .with_context(|| format!("failed to read config at {}", path.display()))?;
        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("yaml");
        let config: DaemonConfig = if ext.eq_ignore_ascii_case("json") {
            serde_json::from_str(&contents)
                .with_context(|| format!("failed to parse JSON config at {}", path.display()))?
        } else {
            serde_yaml::from_str(&contents)
                .with_context(|| format!("failed to parse YAML config at {}", path.display()))?
        };
        Ok(config)
    }

    pub fn save(&self, path: impl AsRef<Path>) -> Result<()> {
        let path = path.as_ref();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("failed to create config directory {}", parent.display()))?;
        }
        let contents = serde_yaml::to_string(self)
            .context("failed to serialize config to YAML")?;
        std::fs::write(path, contents)
            .with_context(|| format!("failed to write config to {}", path.display()))?;
        Ok(())
    }

    pub fn ensure_dirs(&self) -> Result<()> {
        std::fs::create_dir_all(&self.data_dir)
            .with_context(|| format!("failed to create data directory {}", self.data_dir.display()))?;
        std::fs::create_dir_all(&self.models_dir)
            .with_context(|| format!("failed to create models directory {}", self.models_dir.display()))?;
        std::fs::create_dir_all(&self.downloads_dir)
            .with_context(|| format!("failed to create downloads directory {}", self.downloads_dir.display()))?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::TempDir;

    #[test]
    fn default_config_uses_exe_parent() {
        let cfg = DaemonConfig::default();
        assert!(!cfg.data_dir.to_string_lossy().contains("~/.mirage"));
        assert_eq!(cfg.modules.vector, true);
        assert_eq!(cfg.modules.audio, false);
    }

    #[test]
    fn load_and_save_yaml() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("daemon.yaml");
        let mut cfg = DaemonConfig::default();
        cfg.log_level = String::from("debug");
        cfg.save(&path).unwrap();

        let loaded = DaemonConfig::load(&path).unwrap();
        assert_eq!(loaded.log_level, "debug");
        assert_eq!(loaded, cfg);
    }

    #[test]
    fn load_json_config() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("daemon.json");
        let mut file = std::fs::File::create(&path).unwrap();
        write!(
            file,
            r#"{{"data_dir":"./data","models_dir":"./models","log_level":"warn","modules":{{"vector":false}}}}"#
        )
        .unwrap();

        let loaded = DaemonConfig::load(&path).unwrap();
        assert_eq!(loaded.log_level, "warn");
        assert_eq!(loaded.modules.vector, false);
        assert_eq!(loaded.modules.text, true);
    }
}
