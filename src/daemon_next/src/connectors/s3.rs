use crate::config::ConnectorConfig;
use crate::connectors::{CloudConnector, RemoteEntry};
use anyhow::{Context, Result};
use async_trait::async_trait;
use aws_config::BehaviorVersion;
use aws_sdk_s3::config::{Credentials, Region};
use aws_sdk_s3::presigning::PresigningConfig;
use aws_sdk_s3::Client;
use std::path::Path;
use std::time::Duration;

const PRESIGN_EXPIRY: Duration = Duration::from_secs(3600);
const MAX_LIST_KEYS: usize = 10_000;

pub struct S3Connector {
    id: String,
    name: String,
    client: Client,
    bucket: String,
    roots: Vec<String>,
    source_type: String,
}

impl S3Connector {
    pub async fn new(config: &ConnectorConfig) -> Result<Self> {
        let bucket = config
            .credentials
            .bucket
            .clone()
            .context("S3 connector requires 'bucket'")?;
        let region_name = config.credentials.region.clone().unwrap_or_else(|| "us-east-1".to_string());
        let endpoint = config.credentials.endpoint.clone();
        let access_key = config.credentials.access_key.clone();
        let secret_key = config.credentials.secret_key.clone();

        let mut builder = aws_sdk_s3::Config::builder()
            .behavior_version(BehaviorVersion::latest())
            .region(Region::new(region_name));
        if let Some(endpoint) = endpoint {
            builder = builder.endpoint_url(endpoint);
        }
        if let (Some(ak), Some(sk)) = (access_key, secret_key) {
            let creds = Credentials::new(ak, sk, None, None, "mirage-config");
            builder = builder.credentials_provider(creds);
        }
        // If no explicit credentials, the SDK will attempt to use the default
        // credential chain (env vars, ~/.aws/credentials, IAM role).
        let s3_config = builder.build();
        let client = Client::from_conf(s3_config);

        let roots = if config.roots.is_empty() {
            vec![String::new()]
        } else {
            config.roots.clone()
        };

        Ok(Self {
            id: config.id.clone(),
            name: config.name.clone(),
            client,
            bucket,
            roots,
            source_type: "s3".to_string(),
        })
    }

    fn uri(&self, key: &str) -> String {
        format!("s3://{}/{}", self.bucket, key)
    }

    fn normalize_prefix(prefix: &str) -> String {
        let trimmed = prefix.trim_start_matches('/');
        if trimmed.is_empty() {
            String::new()
        } else if trimmed.ends_with('/') {
            trimmed.to_string()
        } else {
            format!("{}/", trimmed)
        }
    }

    async fn presign_url(&self, key: &str) -> Result<String> {
        let presign = PresigningConfig::builder()
            .expires_in(PRESIGN_EXPIRY)
            .build()?;
        let presigned = self
            .client
            .get_object()
            .bucket(self.bucket.clone())
            .key(key)
            .presigned(presign)
            .await?;
        Ok(presigned.uri().to_string())
    }
}

#[async_trait]
impl CloudConnector for S3Connector {
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
        let mut entries = Vec::new();
        for root in &self.roots {
            let prefix = Self::normalize_prefix(root);
            let mut continuation_token: Option<String> = None;

            loop {
                if entries.len() >= MAX_LIST_KEYS {
                    tracing::warn!("S3 connector {} reached list limit {}", self.id, MAX_LIST_KEYS);
                    return Ok(entries);
                }
                let mut req = self.client.list_objects_v2().bucket(self.bucket.clone()).prefix(prefix.clone());
                if let Some(token) = continuation_token.as_ref() {
                    req = req.continuation_token(token);
                }
                let resp = req.send().await?;

                if let Some(objects) = resp.contents {
                    for object in objects {
                        let key = object.key.clone().unwrap_or_default();
                        if key.is_empty() || key.ends_with('/') {
                            continue;
                        }
                        let name = key.split('/').next_back().unwrap_or(&key).to_string();
                        let open_url = self.presign_url(&key).await.ok();
                        entries.push(RemoteEntry {
                            id: self.uri(&key),
                            path: key,
                            name,
                            size: object.size.unwrap_or(0) as u64,
                            modified: object.last_modified.map(|d| d.to_string()),
                            content_type: None,
                            open_url,
                        });
                    }
                }

                continuation_token = resp.next_continuation_token.clone();
                if continuation_token.is_none() {
                    break;
                }
            }
        }
        Ok(entries)
    }

    async fn download(&self, entry: &RemoteEntry, dest: &Path) -> Result<()> {
        let resp = self
            .client
            .get_object()
            .bucket(self.bucket.clone())
            .key(entry.path.clone())
            .send()
            .await?;
        let body = resp.body.collect().await?;
        tokio::fs::write(dest, body.into_bytes()).await?;
        Ok(())
    }
}
