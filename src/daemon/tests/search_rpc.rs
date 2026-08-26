use std::process::Stdio;
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;

#[tokio::test]
async fn search_rpc_returns_empty_then_indexed_results() {
    let dir = tempfile::tempdir().unwrap();
    let socket_path = dir.path().join("mirage.sock");

    let mut child = Command::new(env!("CARGO_BIN_EXE_mirage-daemon"))
        .arg("--socket-path")
        .arg(&socket_path)
        .arg("--data-dir")
        .arg(dir.path().join("data"))
        .arg("--models-dir")
        .arg(dir.path().join("models"))
        .arg("--log-level")
        .arg("debug")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("failed to spawn daemon");

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
    assert_eq!(
        index_result.result,
        Some(serde_json::json!({ "count": 2 }))
    );

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
    let results = search_result.result.expect("missing search result").as_array().expect("result is not array").clone();
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
