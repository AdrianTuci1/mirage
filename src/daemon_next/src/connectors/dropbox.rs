use crate::config::ConnectorConfig;
use crate::connectors::{CloudConnector, RemoteEntry};
use anyhow::{Context, Result};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::time::Duration;

const DROPBOX_API: &str = "https://api.dropboxapi.com/2";
const DROPBOX_CONTENT: &str = "https://content.dropboxapi.com/2";
const MAX_LIST: usize = 10_000;

pub struct DropboxConnector {
    id: String,
    name: String,
    token: String,
    roots: Vec<String>,
    client: reqwest::Client,
    source_type: String,
}

impl DropboxConnector {
    pub fn new(config: &ConnectorConfig) -> Result<Self> {
        let token = config
            .credentials
            .oauth_token
            .clone()
            .context("Dropbox connector requires 'oauth_token'")?;
        let roots = if config.roots.is_empty() {
            vec![String::new()]
        } else {
            config.roots.clone()
        };
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(60))
            .build()?;
        Ok(Self {
            id: config.id.clone(),
            name: config.name.clone(),
            token,
            roots,
            client,
            source_type: "dropbox".to_string(),
        })
    }

    fn uri(&self, path: &str) -> String {
        format!("dropbox:{}", path)
    }

    fn normalize_path(&self, path: &str) -> String {
        let trimmed = path.trim_start_matches('/');
        if trimmed.is_empty() {
            String::from("")
        } else {
            format!("/{}", trimmed)
        }
    }

    fn preview_url(&self, path: &str) -> Option<String> {
        let web_path = path.trim_start_matches('/');
        Some(format!(
            "https://www.dropbox.com/preview/{}",
            urlencoding::encode(web_path)
        ))
    }

    async fn list_path(&self, path: &str) -> Result<Vec<RemoteEntry>> {
        let mut entries = Vec::new();
        let mut cursor: Option<String> = None;
        let normalized = self.normalize_path(path);

        loop {
            if entries.len() >= MAX_LIST {
                break;
            }
            let body = match cursor.as_ref() {
                Some(c) => serde_json::to_string(&ListContinue { cursor: c.clone() })?,
                None => serde_json::to_string(&ListFolderRequest {
                    path: normalized.clone(),
                    recursive: true,
                    include_deleted: false,
                })?,
            };

            let url = if cursor.is_some() {
                format!("{}/files/list_folder/continue", DROPBOX_API)
            } else {
                format!("{}/files/list_folder", DROPBOX_API)
            };

            let response = self
                .client
                .post(&url)
                .header("Authorization", format!("Bearer {}", self.token))
                .header("Content-Type", "application/json")
                .body(body)
                .send()
                .await?;

            if !response.status().is_success() {
                let text = response.text().await.unwrap_or_default();
                anyhow::bail!("Dropbox list_folder failed: {}", text);
            }

            let result: ListFolderResult = response.json().await?;
            for entry in result.entries {
                match entry {
                    DropboxEntry::File(file) => {
                        let name = file.name.clone();
                        let open_url = self.preview_url(&file.path_display);
                        entries.push(RemoteEntry {
                            id: self.uri(&file.path_display),
                            path: file.path_display.clone(),
                            name,
                            size: file.size as u64,
                            modified: file.server_modified,
                            content_type: None,
                            open_url,
                        });
                    }
                    DropboxEntry::Folder(_) => {}
                }
            }

            if !result.has_more {
                break;
            }
            cursor = result.cursor;
        }
        Ok(entries)
    }
}

#[async_trait]
impl CloudConnector for DropboxConnector {
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
        let mut all = Vec::new();
        for root in &self.roots {
            let path = self.normalize_path(root);
            let mut entries = self.list_path(&path).await?;
            all.append(&mut entries);
        }
        Ok(all)
    }

    async fn thumbnail(&self, entry: &RemoteEntry) -> Result<Option<Vec<u8>>> {
        // Dropbox supports thumbnails for images and videos.
        let arg = serde_json::json!({ "path": entry.path, "size": "w64h64", "mode": "strict" });
        let response = self
            .client
            .post(format!("{}/files/get_thumbnail", DROPBOX_CONTENT))
            .header("Authorization", format!("Bearer {}", self.token))
            .header("Dropbox-API-Arg", arg.to_string())
            .send()
            .await?;
        if response.status().is_success() {
            Ok(Some(response.bytes().await?.to_vec()))
        } else {
            Ok(None)
        }
    }

    async fn download(&self, entry: &RemoteEntry, dest: &Path) -> Result<()> {
        let arg = serde_json::json!({ "path": entry.path });
        let response = self
            .client
            .post(format!("{}/files/download", DROPBOX_CONTENT))
            .header("Authorization", format!("Bearer {}", self.token))
            .header("Dropbox-API-Arg", arg.to_string())
            .send()
            .await?;
        if !response.status().is_success() {
            anyhow::bail!("Dropbox download failed: {}", response.text().await.unwrap_or_default());
        }
        let bytes = response.bytes().await?;
        tokio::fs::write(dest, bytes).await?;
        Ok(())
    }
}

#[derive(Serialize)]
struct ListFolderRequest {
    path: String,
    recursive: bool,
    include_deleted: bool,
}

#[derive(Serialize)]
struct ListContinue {
    cursor: String,
}

#[derive(Deserialize)]
struct ListFolderResult {
    entries: Vec<DropboxEntry>,
    has_more: bool,
    cursor: Option<String>,
}

#[derive(Deserialize)]
#[serde(tag = ".tag")]
enum DropboxEntry {
    #[serde(rename = "file")]
    File(DropboxFile),
    #[serde(rename = "folder")]
    Folder(DropboxFolder),
}

#[derive(Deserialize)]
struct DropboxFile {
    name: String,
    path_display: String,
    size: usize,
    server_modified: Option<String>,
}

#[derive(Deserialize)]
struct DropboxFolder {
    #[allow(dead_code)]
    name: String,
}
