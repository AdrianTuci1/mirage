use crate::config::ConnectorConfig;
use crate::connectors::{CloudConnector, RemoteEntry};
use anyhow::{Context, Result};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::time::Duration;

const DRIVE_API: &str = "https://www.googleapis.com/drive/v3";
const FIELDS: &str = "nextPageToken,files(id,name,size,modifiedTime,mimeType,webViewLink,thumbnailLink)";
const MAX_LIST: usize = 10_000;

pub struct GDriveConnector {
    id: String,
    name: String,
    token: String,
    roots: Vec<String>,
    client: reqwest::Client,
    source_type: String,
}

impl GDriveConnector {
    pub fn new(config: &ConnectorConfig) -> Result<Self> {
        let token = config
            .credentials
            .oauth_token
            .clone()
            .context("Google Drive connector requires 'oauth_token'")?;
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
            source_type: "gdrive".to_string(),
        })
    }

    fn uri(&self, id: &str, name: &str) -> String {
        format!("gdrive://{}/{}", id, name)
    }

    fn query_for_root(&self, root: &str) -> Option<String> {
        let trimmed = root.trim();
        if trimmed.is_empty() {
            return None;
        }
        // Support root as folder id or folder name.
        if trimmed.starts_with("folder:") {
            let id = trimmed.strip_prefix("folder:").unwrap_or(trimmed);
            return Some(format!("'{}' in parents", id));
        }
        Some(format!("name contains '{}' or '{}' in parents", trimmed, trimmed))
    }
}

#[async_trait]
impl CloudConnector for GDriveConnector {
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
        let roots = if self.roots.is_empty() { vec![String::new()] } else { self.roots.clone() };

        for root in roots {
            let mut page_token: Option<String> = None;
            let base_query = self.query_for_root(&root);

            loop {
                if all.len() >= MAX_LIST {
                    tracing::warn!("GDrive connector {} reached list limit {}", self.id, MAX_LIST);
                    return Ok(all);
                }
                let mut request = self
                    .client
                    .get(format!("{}/files", DRIVE_API))
                    .bearer_auth(&self.token)
                    .query(&[("pageSize", "100"), ("fields", FIELDS)]);
                if let Some(q) = base_query.as_ref() {
                    request = request.query(&[("q", q)]);
                }
                if let Some(token) = page_token.as_ref() {
                    request = request.query(&[("pageToken", token)]);
                }

                let response = request.send().await?;
                if !response.status().is_success() {
                    let text = response.text().await.unwrap_or_default();
                    anyhow::bail!("Google Drive list failed: {}", text);
                }
                let result: DriveListResult = response.json().await?;
                for file in result.files {
                    if file.mime_type == "application/vnd.google-apps.folder" {
                        continue;
                    }
                    let size = file.size.as_ref().and_then(|s| s.parse().ok()).unwrap_or(0);
                    let open_url = file
                        .web_view_link
                        .clone()
                        .or_else(|| Some(format!("https://drive.google.com/file/d/{}/view", file.id)));
                    all.push(RemoteEntry {
                        id: self.uri(&file.id, &file.name),
                        path: file.id.clone(),
                        name: file.name.clone(),
                        size,
                        modified: file.modified_time.clone(),
                        content_type: Some(file.mime_type.clone()),
                        open_url,
                    });
                }
                page_token = result.next_page_token;
                if page_token.is_none() {
                    break;
                }
            }
        }
        Ok(all)
    }

    async fn thumbnail(&self, _entry: &RemoteEntry) -> Result<Option<Vec<u8>>> {
        // Thumbnail URL is fetched at list time; Drive thumbnails are public-ish with the URL.
        // We do not re-fetch here to keep it simple.
        Ok(None)
    }

    async fn download(&self, entry: &RemoteEntry, dest: &Path) -> Result<()> {
        let export_url = format!("{}/files/{}?alt=media", DRIVE_API, entry.path);
        let response = self
            .client
            .get(&export_url)
            .bearer_auth(&self.token)
            .send()
            .await?;
        if !response.status().is_success() {
            anyhow::bail!("Google Drive download failed: {}", response.text().await.unwrap_or_default());
        }
        let bytes = response.bytes().await?;
        tokio::fs::write(dest, bytes).await?;
        Ok(())
    }
}

#[derive(Serialize)]
struct DriveQuery {
    q: String,
    page_size: i32,
    fields: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    page_token: Option<String>,
}

#[derive(Deserialize)]
struct DriveListResult {
    #[serde(rename = "nextPageToken")]
    next_page_token: Option<String>,
    files: Vec<DriveFile>,
}

#[derive(Deserialize)]
struct DriveFile {
    id: String,
    name: String,
    #[serde(rename = "mimeType")]
    mime_type: String,
    #[serde(rename = "modifiedTime")]
    modified_time: Option<String>,
    size: Option<String>,
    #[serde(rename = "webViewLink")]
    web_view_link: Option<String>,
    #[serde(rename = "thumbnailLink")]
    thumbnail_link: Option<String>,
}
