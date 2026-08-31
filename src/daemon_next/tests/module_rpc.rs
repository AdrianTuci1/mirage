use std::process::Stdio;
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;

async fn spawn_daemon(dir: &tempfile::TempDir) -> tokio::process::Child {
    let socket_path = dir.path().join("mirage.sock");

    let downloads_dir = dir.path().join("downloads");
    let catalog = serde_json::json!({
        "schema_version": "1.0.0",
        "catalog_version": "2026.08.27-1",
        "minimum_daemon_version": "0.1.0",
        "signature": {
            "algorithm": "ed25519",
            "public_key_fingerprint": "test",
            "signature": "test"
        },
        "modules": [
            {
                "id": "text_embedding_model",
                "name": "Text Embedding Model",
                "version": "1.0.0",
                "description": "ONNX text embedding model.",
                "kind": "model",
                "license": "Apache-2.0",
                "is_optional": true,
                "dependencies": ["onnx_runtime"],
                "platforms": {
                    "universal": {
                        "url": "https://example.com/model.tar.gz",
                        "size": 1024,
                        "checksum": "0000000000000000000000000000000000000000000000000000000000000000",
                        "archive_format": "tar.gz",
                        "files": [
                            {
                                "relative_path": "model.onnx",
                                "sha256": "0000000000000000000000000000000000000000000000000000000000000000",
                                "executable": false,
                                "required": true
                            }
                        ]
                    }
                }
            },
            {
                "id": "onnx_runtime",
                "name": "ONNX Runtime",
                "version": "1.19.0",
                "description": "ONNX Runtime.",
                "kind": "runtime",
                "license": "MIT",
                "is_optional": true,
                "dependencies": [],
                "platforms": {
                    "macos_aarch64": {
                        "url": "https://example.com/onnx.tar.gz",
                        "size": 1024,
                        "checksum": "0000000000000000000000000000000000000000000000000000000000000000",
                        "archive_format": "tar.gz",
                        "files": [
                            {
                                "relative_path": "lib/libonnxruntime.dylib",
                                "sha256": "0000000000000000000000000000000000000000000000000000000000000000",
                                "executable": true,
                                "required": true
                            }
                        ]
                    }
                }
            }
        ]
    });
    std::fs::create_dir_all(&downloads_dir).unwrap();
    std::fs::write(downloads_dir.join("catalog.json"), catalog.to_string()).unwrap();

    let config_path = dir.path().join("daemon.yaml");
    // An explicit config keeps the test off the developer's own daemon.yaml, and
    // empty roots mean no recursive watch is registered over a real directory tree.
    std::fs::write(&config_path, "roots: []\n").unwrap();

    let mut child = Command::new(env!("CARGO_BIN_EXE_mirage-daemon"))
        .arg("--config")
        .arg(&config_path)
        .arg("--socket-path")
        .arg(&socket_path)
        .arg("--data-dir")
        .arg(dir.path().join("data"))
        .arg("--models-dir")
        .arg(dir.path().join("models"))
        .arg("--downloads-dir")
        .arg(&downloads_dir)
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
    child
}

#[tokio::test]
async fn list_modules_returns_cached_catalog() {
    let dir = tempfile::tempdir().unwrap();
    let socket_path = dir.path().join("mirage.sock");
    let mut child = spawn_daemon(&dir).await;

    let result = mirage_daemon::ipc::client::IpcClient::call(
        &socket_path,
        "list_modules",
        Some(serde_json::json!({})),
    )
    .await
    .expect("failed to call list_modules");

    assert_eq!(
        result.error, None,
        "list_modules returned error: {:?}",
        result.error
    );
    let modules = result
        .result
        .expect("missing list_modules result")
        .as_array()
        .expect("result is not array")
        .clone();
    let ids: Vec<String> = modules
        .iter()
        .map(|m| m["module_id"].as_str().unwrap().to_string())
        .collect();
    assert!(ids.contains(&String::from("text_embedding_model")));
    assert!(ids.contains(&String::from("onnx_runtime")));
    // The cached catalog is merged with the built-in module set, so the two fixtures
    // written above arrive next to the CLIP modules the daemon ships with.
    assert!(ids.contains(&String::from("clip_text_encoder")));
    assert!(ids.contains(&String::from("clip_vision_encoder")));
    // DuckDB is no longer linked in; it is offered as a download.
    assert!(ids.contains(&String::from("duckdb")));
    assert_eq!(modules.len(), 6, "unexpected module set: {ids:?}");

    let duckdb = modules.iter().find(|m| m["module_id"] == "duckdb").unwrap();
    // The daemon sees the developer's MIRAGE_DUCKDB_BIN if it is set; the empty
    // downloads directory means that override is the only thing that can make
    // the engine look ready.
    let override_present = mirage_daemon::analytics::engine_override().is_some();
    assert_eq!(
        duckdb["state"],
        if override_present { "ready" } else { "missing" },
        "duckdb state should follow the engine on disk (override: {override_present})"
    );

    let text_model = modules
        .iter()
        .find(|m| m["module_id"] == "text_embedding_model")
        .unwrap();
    assert_eq!(text_model["state"], "missing");
    // The cached catalog declares `onnx_runtime`, which a build with the `onnx`
    // feature marks ready without downloading anything, so the model that depends
    // on it has its dependencies satisfied even though the model itself is absent.
    assert_eq!(
        text_model["dependencies_ready"],
        cfg!(feature = "onnx"),
        "module set: {ids:?}"
    );

    let _ = child.kill().await;
    let _ = tokio::time::timeout(Duration::from_secs(5), child.wait()).await;
}

#[tokio::test]
async fn module_status_returns_missing_for_unknown_module() {
    let dir = tempfile::tempdir().unwrap();
    let socket_path = dir.path().join("mirage.sock");
    let mut child = spawn_daemon(&dir).await;

    let result = mirage_daemon::ipc::client::IpcClient::call(
        &socket_path,
        "module_status",
        Some(serde_json::json!({ "module_id": "unknown" })),
    )
    .await
    .expect("failed to call module_status");

    assert!(result.error.is_some(), "expected error for unknown module");

    let _ = child.kill().await;
    let _ = tokio::time::timeout(Duration::from_secs(5), child.wait()).await;
}

#[tokio::test]
async fn ask_uses_heuristic_slm_without_onnx_model() {
    let dir = tempfile::tempdir().unwrap();
    let socket_path = dir.path().join("mirage.sock");
    let mut child = spawn_daemon(&dir).await;

    // Semantic-style question should route to semantic_search even without SLM module.
    let result = mirage_daemon::ipc::client::IpcClient::call(
        &socket_path,
        "ask",
        Some(serde_json::json!({ "question": "show me beach photos" })),
    )
    .await
    .expect("failed to call ask");

    assert_eq!(
        result.error, None,
        "ask should not fail: {:?}",
        result.error
    );
    let response = result.result.expect("missing ask result");
    assert_eq!(response["type"], "semantic_search");
    assert!(response["results"].is_array());

    let _ = child.kill().await;
    let _ = tokio::time::timeout(Duration::from_secs(5), child.wait()).await;
}
