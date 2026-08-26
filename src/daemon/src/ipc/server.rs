use crate::config::DaemonConfig;
use crate::db::LanceDbStore;
use crate::ipc::protocol::{JsonRpcError, JsonRpcRequest, JsonRpcResponse};
use crate::models::{Record, SearchResult};
use anyhow::{Context, Result};
use serde::Deserialize;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tracing::{debug, info, warn};

type MethodHandler = Arc<
    dyn Fn(Value) -> Pin<Box<dyn Future<Output = Result<Value, JsonRpcError>> + Send>> + Send + Sync,
>;

#[derive(Debug, Deserialize)]
struct SearchRequest {
    query_vector: Vec<f32>,
    top_k: usize,
}

#[derive(Debug, Deserialize)]
struct IndexRequest {
    records: Vec<Record>,
}

pub struct IpcServer {
    handlers: HashMap<String, MethodHandler>,
}

impl IpcServer {
    pub fn new(store: Arc<LanceDbStore>) -> Self {
        let mut server = Self {
            handlers: HashMap::new(),
        };
        server.register("ping", |_params| Box::pin(async { Ok(json!("pong")) }));
        server.register("status", |_params| {
            Box::pin(async {
                Ok(json!({
                    "status": "ok",
                    "version": "0.1.0",
                }))
            })
        });

        let search_store = Arc::clone(&store);
        server.register("search", move |params: Value| {
            let store = Arc::clone(&search_store);
            Box::pin(async move {
                let request: SearchRequest = serde_json::from_value(params).map_err(|e| {
                    JsonRpcError::new(
                        crate::ipc::protocol::ERROR_INVALID_PARAMS,
                        format!("invalid search params: {}", e),
                    )
                })?;
                let results = store
                    .search(request.query_vector, request.top_k)
                    .await
                    .map_err(|e| {
                        JsonRpcError::new(
                            crate::ipc::protocol::ERROR_INTERNAL_ERROR,
                            format!("search failed: {}", e),
                        )
                    })?;
                let response: Vec<SearchResult> = results
                    .into_iter()
                    .map(|r| SearchResult {
                        id: r.record.id,
                        relative_path: r.record.relative_path,
                        source_type: r.record.source_type,
                        score: r.score,
                    })
                    .collect();
                serde_json::to_value(response).map_err(|e| {
                    JsonRpcError::new(
                        crate::ipc::protocol::ERROR_INTERNAL_ERROR,
                        format!("failed to serialize search results: {}", e),
                    )
                })
            })
        });

        let index_store = Arc::clone(&store);
        server.register("index", move |params: Value| {
            let store = Arc::clone(&index_store);
            Box::pin(async move {
                let request: IndexRequest = serde_json::from_value(params).map_err(|e| {
                    JsonRpcError::new(
                        crate::ipc::protocol::ERROR_INVALID_PARAMS,
                        format!("invalid index params: {}", e),
                    )
                })?;
                store.upsert(request.records).await.map_err(|e| {
                    JsonRpcError::new(
                        crate::ipc::protocol::ERROR_INTERNAL_ERROR,
                        format!("index failed: {}", e),
                    )
                })?;
                let count = store.count().await.map_err(|e| {
                    JsonRpcError::new(
                        crate::ipc::protocol::ERROR_INTERNAL_ERROR,
                        format!("count failed: {}", e),
                    )
                })?;
                Ok(json!({ "count": count }))
            })
        });

        server
    }

    pub fn register<F, Fut>(&mut self, method: impl Into<String>, handler: F)
    where
        F: Fn(Value) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<Value, JsonRpcError>> + Send + 'static,
    {
        let handler = Arc::new(move |params: Value| -> Pin<Box<dyn Future<Output = Result<Value, JsonRpcError>> + Send>> {
            Box::pin(handler(params))
        });
        self.handlers.insert(method.into(), handler);
    }

    pub async fn run(self, config: &DaemonConfig) -> Result<()> {
        let this = Arc::new(self);

        #[cfg(unix)]
        {
            let socket_path = &config.socket_path;
            if socket_path.exists() {
                std::fs::remove_file(socket_path).with_context(|| {
                    format!("failed to remove stale socket {}", socket_path.display())
                })?;
            }
            let parent = socket_path.parent().unwrap_or_else(|| std::path::Path::new("."));
            tokio::fs::create_dir_all(parent).await.with_context(|| {
                format!("failed to create socket directory {}", parent.display())
            })?;

            let listener = tokio::net::UnixListener::bind(socket_path)
                .with_context(|| format!("failed to bind Unix socket {}", socket_path.display()))?;
            info!("Mirage daemon listening on {}", socket_path.display());
            println!("Mirage daemon listening on {}", socket_path.display());

            loop {
                let (stream, _) = listener.accept().await.context("failed to accept connection")?;
                let server = Arc::clone(&this);
                tokio::spawn(async move {
                    if let Err(e) = server.handle_stream(stream).await {
                        warn!("connection handler error: {}", e);
                    }
                });
            }
        }

        #[cfg(windows)]
        {
            let pipe_name = format!("\\\\.\\pipe\\{}", config.pipe_name);
            info!("Mirage daemon listening on {}", pipe_name);
            println!("Mirage daemon listening on {}", pipe_name);

            let mut first_instance = true;
            loop {
                let mut opts = tokio::net::windows::named_pipe::ServerOptions::new();
                if first_instance {
                    opts.first_pipe_instance(true);
                }
                let pipe = opts
                    .create(&pipe_name)
                    .context("failed to create named pipe server")?;
                first_instance = false;

                pipe.connect().await.context("failed to accept pipe connection")?;
                let server = Arc::clone(&this);
                tokio::spawn(async move {
                    if let Err(e) = server.handle_stream(pipe).await {
                        warn!("connection handler error: {}", e);
                    }
                });
            }
        }
    }

    async fn handle_stream<S>(&self, stream: S) -> Result<()>
    where
        S: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin,
    {
        let (reader, mut writer) = tokio::io::split(stream);
        let mut reader = BufReader::new(reader);
        let mut line = String::new();

        loop {
            line.clear();
            let bytes_read = reader
                .read_line(&mut line)
                .await
                .context("failed to read line from stream")?;
            if bytes_read == 0 {
                break;
            }
            let line = line.trim();
            if line.is_empty() {
                continue;
            }

            debug!("received JSON-RPC request: {}", line);
            let response = self.dispatch_line(line).await;
            let response_text = serde_json::to_string(&response).context("failed to serialize response")?;
            writer
                .write_all(response_text.as_bytes())
                .await
                .context("failed to write response")?;
            writer.write_all(b"\n").await.context("failed to write newline")?;
            writer.flush().await.context("failed to flush response")?;
        }

        Ok(())
    }

    async fn dispatch_line(&self, line: &str) -> JsonRpcResponse {
        let request: JsonRpcRequest = match serde_json::from_str(line) {
            Ok(req) => req,
            Err(e) => {
                return JsonRpcResponse::new_error(
                    crate::ipc::protocol::ERROR_PARSE_ERROR,
                    format!("parse error: {}", e),
                    None,
                );
            }
        };

        if request.jsonrpc != "2.0" {
            return JsonRpcResponse::new_error(
                crate::ipc::protocol::ERROR_INVALID_REQUEST,
                "invalid jsonrpc version",
                request.id,
            );
        }

        let handler = match self.handlers.get(&request.method) {
            Some(h) => h,
            None => {
                return JsonRpcResponse::new_error(
                    crate::ipc::protocol::ERROR_METHOD_NOT_FOUND,
                    format!("method not found: {}", request.method),
                    request.id.clone(),
                );
            }
        };

        match handler(request.params.unwrap_or(Value::Null)).await {
            Ok(result) => JsonRpcResponse::new_result(result, request.id),
            Err(err) => JsonRpcResponse {
                jsonrpc: String::from("2.0"),
                result: None,
                error: Some(err),
                id: request.id,
            },
        }
    }
}
