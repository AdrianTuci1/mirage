use crate::config::ConnectorConfig;
use anyhow::Result;
use async_trait::async_trait;
use std::collections::HashMap;
use std::path::Path;

/// Metadata for a single object known to a connector.
#[derive(Debug, Clone)]
pub struct RemoteEntry {
    pub id: String,
    /// Full path/key inside the source (e.g. "s3://bucket/prefix/file.mp4" or "dropbox:/folder/file.mp4").
    pub path: String,
    /// Display file name.
    pub name: String,
    pub size: u64,
    pub modified: Option<String>,
    pub content_type: Option<String>,
    /// A URL the OS can open without downloading the file. Populated by the connector if available.
    pub open_url: Option<String>,
}

/// A cloud or network source that exposes file metadata and (optionally) stream/preview URLs.
#[async_trait]
pub trait CloudConnector: Send + Sync {
    fn id(&self) -> &str;
    fn name(&self) -> &str;
    /// Source type string used for icon/routing (e.g. "s3", "dropbox", "gdrive", "smb").
    fn source_type(&self) -> &str;

    async fn list_entries(&self, prefix: &str) -> Result<Vec<RemoteEntry>>;

    /// Return the stream/preview URL stored on the entry, if any.
    fn open_url(&self, entry: &RemoteEntry) -> Option<String> {
        entry.open_url.clone()
    }

    /// Fetch a small thumbnail/preview image if the source provides one.
    async fn thumbnail(&self, _entry: &RemoteEntry) -> Result<Option<Vec<u8>>> {
        Ok(None)
    }

    /// Download the full file to a local path. Used only for explicit user actions.
    async fn download(&self, _entry: &RemoteEntry, _dest: &Path) -> Result<()> {
        anyhow::bail!("download not implemented for this connector")
    }
}

/// Build a connector instance from its configuration.
pub fn build_connector(config: &ConnectorConfig) -> Result<Box<dyn CloudConnector>> {
    match config.kind {
        #[cfg(feature = "smb")]
        crate::config::ConnectorKind::Smb => Ok(Box::new(crate::connectors::smb::SmbConnector::new(config)?)),
        crate::config::ConnectorKind::S3 => {
            // S3 connector construction is async because the AWS SDK loads credentials.
            // We run it on the current runtime or build synchronously if no runtime is present.
            let connector = if let Ok(handle) = tokio::runtime::Handle::try_current() {
                handle.block_on(crate::connectors::s3::S3Connector::new(config))?
            } else {
                let rt = tokio::runtime::Runtime::new()?;
                rt.block_on(crate::connectors::s3::S3Connector::new(config))?
            };
            Ok(Box::new(connector))
        }
        crate::config::ConnectorKind::Dropbox => Ok(Box::new(crate::connectors::dropbox::DropboxConnector::new(config)?)),
        crate::config::ConnectorKind::GoogleDrive => Ok(Box::new(crate::connectors::gdrive::GDriveConnector::new(config)?)),
        #[cfg(not(feature = "smb"))]
        crate::config::ConnectorKind::Smb => anyhow::bail!("SMB connector requires the 'smb' feature flag"),
    }
}

/// Create a connector registry from the daemon configuration.
pub fn registry_from_config(configs: &[ConnectorConfig]) -> ConnectorRegistry {
    let mut map: HashMap<String, Box<dyn CloudConnector>> = HashMap::new();
    for cfg in configs {
        if !cfg.enabled {
            continue;
        }
        match build_connector(cfg) {
            Ok(conn) => {
                map.insert(cfg.id.clone(), conn);
            }
            Err(e) => {
                tracing::warn!("failed to build connector {}: {}", cfg.id, e);
            }
        }
    }
    ConnectorRegistry(map)
}

pub struct ConnectorRegistry(HashMap<String, Box<dyn CloudConnector>>);

impl ConnectorRegistry {
    pub fn empty() -> Self {
        Self(HashMap::new())
    }

    pub fn get(&self, id: &str) -> Option<&dyn CloudConnector> {
        self.0.get(id).map(|b| b.as_ref())
    }

    pub fn iter(&self) -> impl Iterator<Item = (&String, &dyn CloudConnector)> {
        self.0.iter().map(|(k, v)| (k, v.as_ref()))
    }
}

mod dropbox;
mod gdrive;
mod s3;
#[cfg(feature = "smb")]
mod smb;
