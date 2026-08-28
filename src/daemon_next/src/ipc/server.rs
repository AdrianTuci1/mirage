use crate::analytics::Analytics;
use crate::config::DaemonConfig;
use crate::db::LanceDbStore;
use crate::embeddings::Embedder;
use crate::ipc::protocol::{JsonRpcError, JsonRpcRequest, JsonRpcResponse};
use crate::models::{Record, SearchResult, SearchResultCategory};
use crate::modules::ModuleManager;
use crate::search::UnifiedSearch;
use crate::slm::{AskResponse, SlmEngine};
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
#[serde(default)]
struct SearchRequest {
    query: Option<String>,
    query_vector: Option<Vec<f32>>,
    top_k: usize,
    hybrid: bool,
}

#[derive(Debug, Deserialize)]
struct DownloadRequest {
    id: String,
    relative_path: String,
    source_type: String,
    dest_path: String,
    #[serde(default)]
    open_url: Option<String>,
}

impl Default for SearchRequest {
    fn default() -> Self {
        Self {
            query: None,
            query_vector: None,
            top_k: 10,
            hybrid: false,
        }
    }
}

#[derive(Debug, Deserialize)]
struct IndexRequest {
    records: Vec<Record>,
}

#[derive(Debug, Deserialize)]
struct QueryRequest {
    sql: String,
}

#[derive(Debug, Deserialize)]
struct EmbedRequest {
    text: String,
}

#[derive(Debug, Deserialize)]
struct AskRequest {
    question: String,
}

#[derive(Debug, Deserialize)]
struct DownloadModuleRequest {
    module_id: String,
    #[serde(default)]
    force: bool,
}

#[derive(Debug, Deserialize)]
struct ModuleIdRequest {
    module_id: String,
}

pub struct IpcServer {
    handlers: HashMap<String, MethodHandler>,
}

impl IpcServer {
    pub fn new(
        store: Arc<LanceDbStore>,
        embedder: Arc<dyn Embedder>,
        analytics: Arc<Analytics>,
        module_manager: Arc<ModuleManager>,
        slm: Arc<dyn SlmEngine>,
        search: Arc<UnifiedSearch>,
    ) -> Self {
        let mut server = Self {
            handlers: HashMap::new(),
        };

        server.register("ping", |_params| Box::pin(async { Ok(json!("pong")) }));

        let status_store = Arc::clone(&store);
        let status_analytics = Arc::clone(&analytics);
        server.register("status", move |_params| {
            let store = Arc::clone(&status_store);
            let analytics = Arc::clone(&status_analytics);
            Box::pin(async move {
                let vector_count = store.count().await.unwrap_or(0);
                let response = json!({
                    "status": "ok",
                    "version": "0.1.0",
                    "vector_count": vector_count,
                    "modules": {
                        "vector": true,
                        "text": true,
                        "tabular": true,
                    },
                });
                let _ = analytics;
                Ok(response)
            })
        });

        let search_searcher = Arc::clone(&search);
        let search_store = Arc::clone(&store);
        server.register("search", move |params: Value| {
            let search = Arc::clone(&search_searcher);
            let store = Arc::clone(&search_store);
            Box::pin(async move {
                let request: SearchRequest = serde_json::from_value(params).map_err(|e| {
                    JsonRpcError::new(
                        crate::ipc::protocol::ERROR_INVALID_PARAMS,
                        format!("invalid search params: {}", e),
                    )
                })?;

                let results = match (request.query, request.query_vector) {
                    (Some(query), _) => {
                        search
                            .search(&query, request.top_k)
                            .await
                            .map_err(|e| {
                                JsonRpcError::new(
                                    crate::ipc::protocol::ERROR_INTERNAL_ERROR,
                                    format!("search failed: {}", e),
                                )
                            })?
                    }
                    (None, Some(vector)) => {
                        // Direct vector search: semantic-only, reserved for programmatic callers.
                        let records = store.search(vector, request.top_k).await.map_err(|e| {
                            JsonRpcError::new(
                                crate::ipc::protocol::ERROR_INTERNAL_ERROR,
                                format!("search failed: {}", e),
                            )
                        })?;
                        records
                            .into_iter()
                            .map(|r| SearchResult {
                                id: r.record.id,
                                relative_path: r.record.relative_path,
                                source_type: r.record.source_type,
                                score: r.score,
                                category: SearchResultCategory::Semantic,
                                open_url: None,
                            })
                            .collect()
                    }
                    (None, None) => {
                        return Err(JsonRpcError::new(
                            crate::ipc::protocol::ERROR_INVALID_PARAMS,
                            "search requires either 'query' or 'query_vector'",
                        ));
                    }
                };

                serde_json::to_value(results).map_err(|e| {
                    JsonRpcError::new(
                        crate::ipc::protocol::ERROR_INTERNAL_ERROR,
                        format!("failed to serialize search results: {}", e),
                    )
                })
            })
        });

        let download_search = Arc::clone(&search);
        server.register("download_file", move |params: Value| {
            let search = Arc::clone(&download_search);
            Box::pin(async move {
                let request: DownloadRequest = serde_json::from_value(params).map_err(|e| {
                    JsonRpcError::new(
                        crate::ipc::protocol::ERROR_INVALID_PARAMS,
                        format!("invalid download_file params: {}", e),
                    )
                })?;

                let result = SearchResult {
                    id: request.id,
                    relative_path: request.relative_path,
                    score: 0.0,
                    source_type: request.source_type,
                    category: SearchResultCategory::File,
                    open_url: request.open_url,
                };

                let dest = PathBuf::from(request.dest_path);
                search
                    .download_result(&result, &dest)
                    .await
                    .map_err(|e| {
                        JsonRpcError::new(
                            crate::ipc::protocol::ERROR_INTERNAL_ERROR,
                            format!("download_file failed: {}", e),
                        )
                    })?;

                Ok(json!({ "dest_path": dest.to_string_lossy() }))
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

        let index_files_search = Arc::clone(&search);
        server.register("index_files", move |_params: Value| {
            let search = Arc::clone(&index_files_search);
            Box::pin(async move {
                let count = search.index_files().await.map_err(|e| {
                    JsonRpcError::new(
                        crate::ipc::protocol::ERROR_INTERNAL_ERROR,
                        format!("index_files failed: {}", e),
                    )
                })?;
                Ok(json!({ "count": count }))
            })
        });

        let index_apps_search = Arc::clone(&search);
        server.register("index_apps", move |_params: Value| {
            let search = Arc::clone(&index_apps_search);
            Box::pin(async move {
                let count = search.index_apps().map_err(|e| {
                    JsonRpcError::new(
                        crate::ipc::protocol::ERROR_INTERNAL_ERROR,
                        format!("index_apps failed: {}", e),
                    )
                })?;
                Ok(json!({ "count": count }))
            })
        });

        let query_analytics = Arc::clone(&analytics);
        server.register("query", move |params: Value| {
            let analytics = Arc::clone(&query_analytics);
            Box::pin(async move {
                let request: QueryRequest = serde_json::from_value(params).map_err(|e| {
                    JsonRpcError::new(
                        crate::ipc::protocol::ERROR_INVALID_PARAMS,
                        format!("invalid query params: {}", e),
                    )
                })?;

                let rows = tokio::task::spawn_blocking(move || analytics.query(&request.sql))
                    .await
                    .map_err(|e| {
                        JsonRpcError::new(
                            crate::ipc::protocol::ERROR_INTERNAL_ERROR,
                            format!("query task failed: {}", e),
                        )
                    })?
                    .map_err(|e| {
                        JsonRpcError::new(
                            crate::ipc::protocol::ERROR_INTERNAL_ERROR,
                            format!("query failed: {}", e),
                        )
                    })?;

                serde_json::to_value(rows).map_err(|e| {
                    JsonRpcError::new(
                        crate::ipc::protocol::ERROR_INTERNAL_ERROR,
                        format!("failed to serialize query results: {}", e),
                    )
                })
            })
        });

        let embed_embedder = Arc::clone(&embedder);
        server.register("embed", move |params: Value| {
            let embedder = Arc::clone(&embed_embedder);
            Box::pin(async move {
                let request: EmbedRequest = serde_json::from_value(params).map_err(|e| {
                    JsonRpcError::new(
                        crate::ipc::protocol::ERROR_INVALID_PARAMS,
                        format!("invalid embed params: {}", e),
                    )
                })?;

                let vector = tokio::task::spawn_blocking(move || embedder.embed_text(&request.text))
                    .await
                    .map_err(|e| {
                        JsonRpcError::new(
                            crate::ipc::protocol::ERROR_INTERNAL_ERROR,
                            format!("embed task failed: {}", e),
                        )
                    })?
                    .map_err(|e| {
                        JsonRpcError::new(
                            crate::ipc::protocol::ERROR_INTERNAL_ERROR,
                            format!("embed failed: {}", e),
                        )
                    })?;

                serde_json::to_value(vector).map_err(|e| {
                    JsonRpcError::new(
                        crate::ipc::protocol::ERROR_INTERNAL_ERROR,
                        format!("failed to serialize embedding: {}", e),
                    )
                })
            })
        });

        let modules_manager = Arc::clone(&module_manager);
        server.register("download_module", move |params: Value| {
            let manager = Arc::clone(&modules_manager);
            Box::pin(async move {
                let request: DownloadModuleRequest = serde_json::from_value(params).map_err(|e| {
                    JsonRpcError::new(
                        crate::ipc::protocol::ERROR_INVALID_PARAMS,
                        format!("invalid download_module params: {}", e),
                    )
                })?;

                manager
                    .download_module(&request.module_id, request.force)
                    .await
                    .map_err(|e| {
                        JsonRpcError::new(
                            crate::ipc::protocol::ERROR_INTERNAL_ERROR,
                            format!("download_module failed: {}", e),
                        )
                    })?;

                let status = manager
                    .module_status(&request.module_id)
                    .await
                    .ok_or_else(|| {
                        JsonRpcError::new(
                            crate::ipc::protocol::ERROR_INTERNAL_ERROR,
                            "module status unavailable after download request",
                        )
                    })?;

                serde_json::to_value(status).map_err(|e| {
                    JsonRpcError::new(
                        crate::ipc::protocol::ERROR_INTERNAL_ERROR,
                        format!("failed to serialize module status: {}", e),
                    )
                })
            })
        });

        let modules_manager = Arc::clone(&module_manager);
        server.register("module_status", move |params: Value| {
            let manager = Arc::clone(&modules_manager);
            Box::pin(async move {
                let request: ModuleIdRequest = serde_json::from_value(params).map_err(|e| {
                    JsonRpcError::new(
                        crate::ipc::protocol::ERROR_INVALID_PARAMS,
                        format!("invalid module_status params: {}", e),
                    )
                })?;

                let status = manager.module_status(&request.module_id).await.ok_or_else(|| {
                    JsonRpcError::new(
                        crate::ipc::protocol::ERROR_INTERNAL_ERROR,
                        format!("module {} not found", request.module_id),
                    )
                })?;

                serde_json::to_value(status).map_err(|e| {
                    JsonRpcError::new(
                        crate::ipc::protocol::ERROR_INTERNAL_ERROR,
                        format!("failed to serialize module status: {}", e),
                    )
                })
            })
        });

        let modules_manager = Arc::clone(&module_manager);
        server.register("list_modules", move |_params: Value| {
            let manager = Arc::clone(&modules_manager);
            Box::pin(async move {
                let modules = manager.list_modules().await;
                serde_json::to_value(modules).map_err(|e| {
                    JsonRpcError::new(
                        crate::ipc::protocol::ERROR_INTERNAL_ERROR,
                        format!("failed to serialize module list: {}", e),
                    )
                })
            })
        });

        let modules_manager = Arc::clone(&module_manager);
        server.register("cancel_download", move |params: Value| {
            let manager = Arc::clone(&modules_manager);
            Box::pin(async move {
                let request: ModuleIdRequest = serde_json::from_value(params).map_err(|e| {
                    JsonRpcError::new(
                        crate::ipc::protocol::ERROR_INVALID_PARAMS,
                        format!("invalid cancel_download params: {}", e),
                    )
                })?;

                manager
                    .cancel_download(&request.module_id)
                    .await
                    .map_err(|e| {
                        JsonRpcError::new(
                            crate::ipc::protocol::ERROR_INTERNAL_ERROR,
                            format!("cancel_download failed: {}", e),
                        )
                    })?;

                let status = manager.module_status(&request.module_id).await.ok_or_else(|| {
                    JsonRpcError::new(
                        crate::ipc::protocol::ERROR_INTERNAL_ERROR,
                        "module status unavailable after cancel",
                    )
                })?;

                serde_json::to_value(status).map_err(|e| {
                    JsonRpcError::new(
                        crate::ipc::protocol::ERROR_INTERNAL_ERROR,
                        format!("failed to serialize module status: {}", e),
                    )
                })
            })
        });

        let modules_manager = Arc::clone(&module_manager);
        server.register("remove_module", move |params: Value| {
            let manager = Arc::clone(&modules_manager);
            Box::pin(async move {
                let request: ModuleIdRequest = serde_json::from_value(params).map_err(|e| {
                    JsonRpcError::new(
                        crate::ipc::protocol::ERROR_INVALID_PARAMS,
                        format!("invalid remove_module params: {}", e),
                    )
                })?;

                manager
                    .remove_module(&request.module_id)
                    .await
                    .map_err(|e| {
                        JsonRpcError::new(
                            crate::ipc::protocol::ERROR_INTERNAL_ERROR,
                            format!("remove_module failed: {}", e),
                        )
                    })?;

                let status = manager.module_status(&request.module_id).await.ok_or_else(|| {
                    JsonRpcError::new(
                        crate::ipc::protocol::ERROR_INTERNAL_ERROR,
                        "module status unavailable after remove",
                    )
                })?;

                serde_json::to_value(status).map_err(|e| {
                    JsonRpcError::new(
                        crate::ipc::protocol::ERROR_INTERNAL_ERROR,
                        format!("failed to serialize module status: {}", e),
                    )
                })
            })
        });

        let ask_store = Arc::clone(&store);
        let ask_embedder = Arc::clone(&embedder);
        let ask_analytics = Arc::clone(&analytics);
        server.register("ask", move |params: Value| {
            let store = Arc::clone(&ask_store);
            let embedder = Arc::clone(&ask_embedder);
            let analytics = Arc::clone(&ask_analytics);
            let slm = Arc::clone(&slm);
            Box::pin(async move {
                let request: AskRequest = serde_json::from_value(params).map_err(|e| {
                    JsonRpcError::new(
                        crate::ipc::protocol::ERROR_INVALID_PARAMS,
                        format!("invalid ask params: {}", e),
                    )
                })?;

                let response = slm
                    .ask(&request.question, store, embedder, analytics)
                    .await
                    .map_err(AskResponse::to_json_rpc_error)?;

                serde_json::to_value(response).map_err(|e| {
                    JsonRpcError::new(
                        crate::ipc::protocol::ERROR_INTERNAL_ERROR,
                        format!("failed to serialize ask response: {}", e),
                    )
                })
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
