use crate::config::{ConnectorConfig, ConnectorCredentials};
use crate::connectors::{CloudConnector, RemoteEntry};
use anyhow::{Context, Result};
use async_trait::async_trait;
use pavao::{SmbClient, SmbCredentials, SmbDirent, SmbOptions, SmbResourceType};
use std::path::Path;
use std::time::Duration;

const MAX_LIST: usize = 10_000;

pub struct SmbConnector {
    id: String,
    name: String,
    server: String,
    share: String,
    root: String,
    username: Option<String>,
    password: Option<String>,
    source_type: String,
}

impl SmbConnector {
    pub fn new(config: &ConnectorConfig) -> Result<Self> {
        let host = config
            .credentials
            .host
            .clone()
            .context("SMB connector requires 'host'")?;
        let share = config
            .credentials
            .share
            .clone()
            .context("SMB connector requires 'share'")?;
        let root = config.credentials.endpoint.clone().unwrap_or_default();
        let server = format!("smb://{}/{}", host.trim_end_matches('/'), share.trim_start_matches('/'));

        Ok(Self {
            id: config.id.clone(),
            name: config.name.clone(),
            server,
            share,
            root,
            username: config.credentials.username.clone(),
            password: config.credentials.password.clone(),
            source_type: "smb".to_string(),
        })
    }

    fn uri(&self, path: &str) -> String {
        format!("smb://{}/{}/{}", self.server.trim_start_matches("smb://"), self.share.trim_start_matches('/'), path.trim_start_matches('/'))
    }

    fn normalize_root(&self) -> String {
        let trimmed = self.root.trim().trim_matches('/');
        if trimmed.is_empty() {
            String::from("/")
        } else {
            format!("/{}/", trimmed)
        }
    }

    fn make_client(&self) -> Result<SmbClient> {
        let mut creds = SmbCredentials::default().server(&self.server);
        if let Some(user) = self.username.as_ref() {
            creds = creds.username(user);
        }
        if let Some(pass) = self.password.as_ref() {
            creds = creds.password(pass);
        }
        let opts = SmbOptions::default().case_sensitive(false);
        SmbClient::new(creds, opts).context("failed to create SMB client")
    }

    fn walk_sync(&self, root: &str, entries: &mut Vec<RemoteEntry>) -> Result<()> {
        let client = self.make_client()?;
        let mut stack: Vec<String> = vec![self.normalize_root()];

        while let Some(dir) = stack.pop() {
            if entries.len() >= MAX_LIST {
                tracing::warn!("SMB connector {} reached list limit {}", self.id, MAX_LIST);
                break;
            }
            let dirents = client.list(&dir).with_context(|| format!("SMB list failed for {}", dir))?;
            for entry in dirents {
                let name = entry.name().to_string();
                if name == "." || name == ".." {
                    continue;
                }
                match entry.get_type() {
                    SmbResourceType::Directory => {
                        let child = if dir.ends_with('/') {
                            format!("{}{}/", dir, name)
                        } else {
                            format!("{}/{}/", dir, name)
                        };
                        stack.push(child);
                    }
                    SmbResourceType::File => {
                        let path = if dir.ends_with('/') {
                            format!("{}{}", dir, name)
                        } else {
                            format!("{}/{}", dir, name)
                        };
                        let relative = path.strip_prefix(&self.normalize_root()).unwrap_or(&path);
                        entries.push(RemoteEntry {
                            id: self.uri(relative),
                            path: relative.to_string(),
                            name,
                            size: entry.size() as u64,
                            modified: None,
                            content_type: None,
                            open_url: Some(self.uri(relative)),
                        });
                    }
                    _ => {}
                }
            }
        }
        Ok(())
    }
}

#[async_trait]
impl CloudConnector for SmbConnector {
    fn id(&self) -> &str {
        &self.id
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn source_type(&self) -> &str {
        &self.source_type
    }

    async fn list_entries(&self, _prefix: &str) -> Result<Vec<RemoteEntry>> {
        let this = self.clone();
        let mut entries = Vec::new();
        tokio::task::spawn_blocking(move || this.walk_sync("", &mut entries))
            .await
            .context("SMB list task failed")??;
        Ok(entries)
    }

    async fn download(&self, entry: &RemoteEntry, dest: &Path) -> Result<()> {
        let this = self.clone();
        let path = entry.path.clone();
        tokio::task::spawn_blocking(move || {
            let client = this.make_client()?;
            let full_path = this.normalize_root().trim_end_matches('/').to_string() + "/" + &path;
            let data = client.read(&full_path).context("SMB read failed")?;
            std::fs::write(dest, data).context("failed to write SMB download")
        })
        .await
        .context("SMB download task failed")??;
        Ok(())
    }
}

impl Clone for SmbConnector {
    fn clone(&self) -> Self {
        Self {
            id: self.id.clone(),
            name: self.name.clone(),
            server: self.server.clone(),
            share: self.share.clone(),
            root: self.root.clone(),
            username: self.username.clone(),
            password: self.password.clone(),
            source_type: self.source_type.clone(),
        }
    }
}
