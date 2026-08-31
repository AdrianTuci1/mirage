use std::process::Stdio;
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;

async fn spawn_daemon(dir: &tempfile::TempDir) -> tokio::process::Child {
    spawn_daemon_with(dir, true).await
}

/// Start the daemon against empty temp directories. `with_engine` decides
/// whether `MIRAGE_DUCKDB_BIN` reaches the child, which is what makes the
/// tabular feature available or missing.
async fn spawn_daemon_with(dir: &tempfile::TempDir, with_engine: bool) -> tokio::process::Child {
    let socket_path = dir.path().join("mirage.sock");
    let config_path = dir.path().join("daemon.yaml");
    // An explicit config keeps the test off the developer's own daemon.yaml, and
    // empty roots mean no recursive watch is registered over a real directory tree.
    std::fs::write(&config_path, "roots: []\n").unwrap();

    let mut command = Command::new(env!("CARGO_BIN_EXE_mirage-daemon"));
    command
        .arg("--config")
        .arg(&config_path)
        .arg("--socket-path")
        .arg(&socket_path)
        .arg("--data-dir")
        .arg(dir.path().join("data"))
        .arg("--models-dir")
        .arg(dir.path().join("models"))
        // An empty module directory, so a locally downloaded DuckDB cannot make
        // a "missing engine" test pass by accident.
        .arg("--downloads-dir")
        .arg(dir.path().join("downloads"))
        .arg("--log-level")
        .arg("debug")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    if !with_engine {
        command.env_remove("MIRAGE_DUCKDB_BIN");
    }

    let mut child = command.spawn().expect("failed to spawn daemon");

    let stdout = child.stdout.take().unwrap();
    let mut reader = BufReader::new(stdout);
    let mut line = String::new();

    let ready = tokio::time::timeout(Duration::from_secs(10), async {
        loop {
            line.clear();
            reader.read_line(&mut line).await.unwrap();
            if line.contains("listening on") {
                break;
            }
        }
    })
    .await;

    assert!(ready.is_ok(), "daemon did not start in time");
    child
}

#[tokio::test]
async fn search_rpc_returns_empty_then_indexed_results() {
    let dir = tempfile::tempdir().unwrap();
    let socket_path = dir.path().join("mirage.sock");
    let mut child = spawn_daemon(&dir).await;

    let empty = mirage_daemon::ipc::client::IpcClient::call(
        &socket_path,
        "search",
        Some(serde_json::json!({
            "query_vector": [1.0_f32, 0.0, 0.0],
            "top_k": 10,
        })),
    )
    .await
    .expect("failed to call search");

    assert_eq!(empty.error, None);
    assert_eq!(empty.result, Some(serde_json::json!([])));

    let index_result = mirage_daemon::ipc::client::IpcClient::call(
        &socket_path,
        "index",
        Some(serde_json::json!({
            "records": [
                {
                    "id": "doc-1",
                    "relative_path": "docs/a.txt",
                    "source_type": "local",
                    "vector": [1.0_f32, 0.0, 0.0],
                    "updated_at": "2024-01-01T00:00:00Z",
                    "version": 1,
                },
                {
                    "id": "doc-2",
                    "relative_path": "docs/b.txt",
                    "source_type": "local",
                    "vector": [0.0_f32, 1.0, 0.0],
                    "updated_at": "2024-01-01T00:00:00Z",
                    "version": 1,
                }
            ]
        })),
    )
    .await
    .expect("failed to call index");

    assert_eq!(index_result.error, None);
    assert_eq!(index_result.result, Some(serde_json::json!({ "count": 2 })));

    let search_result = mirage_daemon::ipc::client::IpcClient::call(
        &socket_path,
        "search",
        Some(serde_json::json!({
            "query_vector": [1.0_f32, 0.0, 0.0],
            "top_k": 10,
        })),
    )
    .await
    .expect("failed to call search after indexing");

    assert_eq!(search_result.error, None);
    let results = search_result
        .result
        .expect("missing search result")
        .as_array()
        .expect("result is not array")
        .clone();
    assert_eq!(results.len(), 2, "expected both documents to be returned");
    assert_eq!(results[0]["id"], "doc-1");
    assert!(
        results[0]["score"].as_f64().unwrap_or(0.0) > 0.99,
        "expected doc-1 to have high score"
    );
    assert_eq!(results[1]["id"], "doc-2");
    assert!(
        results[1]["score"].as_f64().unwrap_or(1.0) < 0.01,
        "expected doc-2 to have low score"
    );

    let _ = child.kill().await;
    let _ = tokio::time::timeout(Duration::from_secs(5), child.wait()).await;
}

#[tokio::test]
async fn query_rpc_executes_duckdb_sql() {
    if mirage_daemon::analytics::engine_override().is_none() {
        eprintln!(
            "skipping: the DuckDB engine is a downloaded module; set MIRAGE_DUCKDB_BIN to test it"
        );
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let socket_path = dir.path().join("mirage.sock");
    let mut child = spawn_daemon(&dir).await;

    let result = mirage_daemon::ipc::client::IpcClient::call(
        &socket_path,
        "query",
        Some(serde_json::json!({
            "sql": "SELECT 1 AS one, 'hello' AS msg"
        })),
    )
    .await
    .expect("failed to call query");

    assert_eq!(result.error, None);
    let rows = result
        .result
        .expect("missing query result")
        .as_array()
        .expect("result is not array")
        .clone();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0]["one"], 1);
    assert_eq!(rows[0]["msg"], "hello");

    let _ = child.kill().await;
    let _ = tokio::time::timeout(Duration::from_secs(5), child.wait()).await;
}

#[tokio::test]
async fn query_rpc_asks_to_install_the_engine_when_it_is_missing() {
    let dir = tempfile::tempdir().unwrap();
    let socket_path = dir.path().join("mirage.sock");
    let mut child = spawn_daemon_with(&dir, false).await;

    let result = mirage_daemon::ipc::client::IpcClient::call(
        &socket_path,
        "query",
        Some(serde_json::json!({ "sql": "SELECT 1 AS one" })),
    )
    .await
    .expect("failed to call query");

    let error = result.error.expect("a missing engine must be reported");
    assert!(
        error.message.contains("not installed"),
        "the client needs an actionable message, got {}",
        error.message
    );

    let _ = child.kill().await;
    let _ = tokio::time::timeout(Duration::from_secs(5), child.wait()).await;
}

#[tokio::test]
async fn embed_rpc_returns_vector() {
    let dir = tempfile::tempdir().unwrap();
    let socket_path = dir.path().join("mirage.sock");
    let mut child = spawn_daemon(&dir).await;

    let result = mirage_daemon::ipc::client::IpcClient::call(
        &socket_path,
        "embed",
        Some(serde_json::json!({
            "text": "semantic search"
        })),
    )
    .await
    .expect("failed to call embed");

    assert_eq!(result.error, None);
    let vector = result
        .result
        .expect("missing embed result")
        .as_array()
        .expect("result is not array")
        .clone();
    assert_eq!(vector.len(), 384);

    let _ = child.kill().await;
    let _ = tokio::time::timeout(Duration::from_secs(5), child.wait()).await;
}

#[tokio::test]
async fn search_rpc_embeds_query_text() {
    let dir = tempfile::tempdir().unwrap();
    let socket_path = dir.path().join("mirage.sock");
    let mut child = spawn_daemon(&dir).await;

    let index_result = mirage_daemon::ipc::client::IpcClient::call(
        &socket_path,
        "index",
        Some(serde_json::json!({
            "records": [
                {
                    "id": "doc-1",
                    "relative_path": "docs/a.txt",
                    "source_type": "local",
                    "vector": vec![1.0_f32; 384],
                    "updated_at": "2024-01-01T00:00:00Z",
                    "version": 1,
                }
            ]
        })),
    )
    .await
    .expect("failed to call index");
    assert_eq!(index_result.error, None);

    let search_result = mirage_daemon::ipc::client::IpcClient::call(
        &socket_path,
        "search",
        Some(serde_json::json!({
            "query": "semantic",
            "top_k": 5,
        })),
    )
    .await
    .expect("failed to call search by text");

    assert_eq!(search_result.error, None);
    let results = search_result
        .result
        .expect("missing search result")
        .as_array()
        .expect("result is not array")
        .clone();
    assert!(!results.is_empty());

    let _ = child.kill().await;
    let _ = tokio::time::timeout(Duration::from_secs(5), child.wait()).await;
}

#[tokio::test]
async fn index_files_rpc_scans_configured_root() {
    let dir = tempfile::tempdir().unwrap();
    let socket_path = dir.path().join("mirage.sock");
    let root = dir.path().join("root");
    std::fs::create_dir(&root).unwrap();
    std::fs::File::create(root.join("report.pdf")).unwrap();

    // Write a config that scopes indexing to the temporary root only.
    let config_path = dir.path().join("daemon.yaml");
    let config = format!(
        "data_dir: {}\nmodels_dir: {}\ndownloads_dir: {}\nroots:\n  - {}\nexcluded_dirs: []\n",
        dir.path().join("data").display(),
        dir.path().join("models").display(),
        dir.path().join("downloads").display(),
        root.display(),
    );
    std::fs::write(&config_path, config).unwrap();

    let mut child = Command::new(env!("CARGO_BIN_EXE_mirage-daemon"))
        .arg("--config")
        .arg(&config_path)
        .arg("--socket-path")
        .arg(&socket_path)
        .arg("--log-level")
        .arg("debug")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("failed to spawn daemon");

    // Wait for the daemon to start listening.
    let stdout = child.stdout.take().unwrap();
    let mut reader = BufReader::new(stdout);
    let mut line = String::new();
    let ready = tokio::time::timeout(Duration::from_secs(10), async {
        loop {
            line.clear();
            reader.read_line(&mut line).await.unwrap();
            if line.contains("listening on") {
                break;
            }
        }
    })
    .await;
    assert!(ready.is_ok(), "daemon did not start in time");

    let result = mirage_daemon::ipc::client::IpcClient::call(&socket_path, "index_files", None)
        .await
        .expect("failed to call index_files");

    assert_eq!(result.error, None);
    let started = result.result.expect("missing result");
    // The RPC answers at once so searches stay free while the pass runs; progress is
    // watched through `index_status`, exactly like the Settings window does.
    assert_eq!(started["started"], true, "index_files response: {started}");
    assert_eq!(started["running"], true, "index_files response: {started}");

    let indexed = tokio::time::timeout(Duration::from_secs(60), async {
        loop {
            let progress =
                mirage_daemon::ipc::client::IpcClient::call(&socket_path, "index_status", None)
                    .await
                    .expect("failed to call index_status")
                    .result
                    .expect("missing index_status result");
            if progress["running"].as_bool() == Some(false) {
                break progress["indexed"].as_u64().unwrap_or(0);
            }
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
    })
    .await
    .expect("the indexing pass never finished");
    assert_eq!(indexed, 1, "expected one indexed file");

    let search_result = mirage_daemon::ipc::client::IpcClient::call(
        &socket_path,
        "search",
        Some(serde_json::json!({
            "query": "report",
            "top_k": 10,
        })),
    )
    .await
    .expect("failed to call search");

    assert_eq!(search_result.error, None);
    let results = search_result
        .result
        .expect("missing result")
        .as_array()
        .expect("result not array")
        .clone();
    assert_eq!(results.len(), 1);
    assert_eq!(results[0]["relative_path"], "report.pdf");

    let _ = child.kill().await;
    let _ = tokio::time::timeout(Duration::from_secs(5), child.wait()).await;
}
