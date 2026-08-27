use std::process::Stdio;
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;

#[tokio::test]
async fn ipc_ping_returns_pong() {
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

    let ready = tokio::time::timeout(Duration::from_secs(30), async {
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

    let response = mirage_daemon::ipc::client::IpcClient::call(&socket_path, "ping", None)
        .await
        .expect("failed to call ping");

    assert_eq!(response.error, None);
    assert_eq!(response.result, Some(serde_json::json!("pong")));

    let _ = child.kill().await;
    let _ = tokio::time::timeout(Duration::from_secs(5), child.wait()).await;
}
