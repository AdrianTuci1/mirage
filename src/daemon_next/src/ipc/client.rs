use crate::ipc::protocol::{JsonRpcRequest, JsonRpcResponse};
use anyhow::{Context, Result};
use serde_json::Value;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::UnixStream;

pub struct IpcClient;

impl IpcClient {
    #[cfg(unix)]
    pub async fn call(
        socket_path: impl AsRef<std::path::Path>,
        method: &str,
        params: Option<Value>,
    ) -> Result<JsonRpcResponse> {
        let socket_path = socket_path.as_ref();
        let stream = UnixStream::connect(socket_path)
            .await
            .with_context(|| format!("failed to connect to {}", socket_path.display()))?;
        Self::call_on_stream(stream, method, params).await
    }

    #[cfg(windows)]
    pub async fn call(
        pipe_name: &str,
        method: &str,
        params: Option<Value>,
    ) -> Result<JsonRpcResponse> {
        let full_name = format!("\\\\.\\pipe\\{}", pipe_name);
        let stream = tokio::net::windows::named_pipe::ClientOptions::new()
            .open(&full_name)
            .with_context(|| format!("failed to connect to {}", full_name))?;
        Self::call_on_stream(stream, method, params).await
    }

    async fn call_on_stream<S>(
        stream: S,
        method: &str,
        params: Option<Value>,
    ) -> Result<JsonRpcResponse>
    where
        S: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin,
    {
        let (reader, mut writer) = tokio::io::split(stream);
        let mut reader = BufReader::new(reader);

        let request = JsonRpcRequest::new(method, params, Some(Value::from(1)));
        let text = serde_json::to_string(&request).context("failed to serialize request")?;
        writer
            .write_all(text.as_bytes())
            .await
            .context("failed to write request")?;
        writer
            .write_all(b"\n")
            .await
            .context("failed to write newline")?;
        writer.flush().await.context("failed to flush request")?;

        let mut line = String::new();
        reader
            .read_line(&mut line)
            .await
            .context("failed to read response")?;
        let response: JsonRpcResponse =
            serde_json::from_str(&line).context("failed to parse response")?;
        Ok(response)
    }
}
