use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use mirage_daemon::{DaemonConfig, ipc::client::IpcClient};
use serde_json::{json, Value};
use std::path::PathBuf;
use std::time::Duration;

#[derive(Parser, Debug)]
#[command(name = "mirage")]
#[command(about = "Mirage CLI — search, query, and manage the local daemon")]
struct Args {
    #[arg(long, global = true)]
    config: Option<PathBuf>,

    #[cfg(unix)]
    #[arg(long, global = true)]
    socket_path: Option<PathBuf>,

    #[cfg(windows)]
    #[arg(long, global = true)]
    pipe_name: Option<String>,

    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand, Debug)]
enum Command {
    /// Search the local vector index by text.
    Search {
        /// Query text.
        query: String,
        /// Number of results to return.
        #[arg(short, long, default_value_t = 10)]
        top_k: usize,
    },
    /// Execute a DuckDB SQL query.
    Query { sql: String },
    /// Show daemon status.
    Status,
    /// Natural language ask — routes to search or SQL.
    Ask { question: String },
    /// Manage downloadable modules.
    #[command(subcommand)]
    Module(ModuleCommand),
}

#[derive(Subcommand, Debug)]
enum ModuleCommand {
    /// List all modules from the cached catalog.
    List,
    /// Install (download) a module.
    Install {
        module_id: String,
        /// Force re-download even if already installed.
        #[arg(long)]
        force: bool,
    },
    /// Show status of a module.
    Status { module_id: String },
    /// Remove an installed module.
    Remove { module_id: String },
}

fn load_config_path(args: &Args) -> PathBuf {
    args.config
        .clone()
        .unwrap_or_else(|| DaemonConfig::base_dir().join("daemon.yaml"))
}

#[cfg(unix)]
fn socket_path(args: &Args) -> Result<PathBuf> {
    if let Some(path) = args.socket_path.clone() {
        return Ok(path);
    }
    let config_path = load_config_path(args);
    let config = DaemonConfig::load(&config_path)
        .with_context(|| format!("failed to load config from {}", config_path.display()))?;
    Ok(config.socket_path)
}

#[cfg(windows)]
fn pipe_name(args: &Args) -> Result<String> {
    if let Some(name) = args.pipe_name.clone() {
        return Ok(name);
    }
    let config_path = load_config_path(args);
    let config = DaemonConfig::load(&config_path)
        .with_context(|| format!("failed to load config from {}", config_path.display()))?;
    Ok(config.pipe_name)
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    match &args.command {
        Command::Search { query, top_k } => search_command(&args, query, *top_k).await,
        Command::Query { sql } => query_command(&args, sql).await,
        Command::Status => status_command(&args).await,
        Command::Ask { question } => ask_command(&args, question).await,
        Command::Module(cmd) => module_command(&args, cmd).await,
    }
}

async fn search_command(args: &Args, query: &str, top_k: usize) -> Result<()> {
    let params = json!({ "query": query, "top_k": top_k });
    let response = ipc_call(args, "search", Some(params)).await?;
    print_response(&response)?;
    Ok(())
}

async fn query_command(args: &Args, sql: &str) -> Result<()> {
    let params = json!({ "sql": sql });
    let response = ipc_call(args, "query", Some(params)).await?;
    print_response(&response)?;
    Ok(())
}

async fn status_command(args: &Args) -> Result<()> {
    let response = ipc_call(args, "status", Some(json!({}))).await?;
    print_response(&response)?;
    Ok(())
}

async fn ask_command(args: &Args, question: &str) -> Result<()> {
    let params = json!({ "question": question });
    let response = ipc_call(args, "ask", Some(params)).await?;
    print_response(&response)?;
    Ok(())
}

async fn module_command(args: &Args, cmd: &ModuleCommand) -> Result<()> {
    match cmd {
        ModuleCommand::List => {
            let response = ipc_call(args, "list_modules", Some(json!({}))).await?;
            print_response(&response)?;
        }
        ModuleCommand::Install { module_id, force } => {
            let params = json!({ "module_id": module_id, "force": *force });
            let response = ipc_call(args, "download_module", Some(params)).await?;
            if response.error.is_some() {
                print_response(&response)?;
                return Ok(());
            }
            println!("Module '{}' download requested. Tracking progress...", module_id);
            track_module(args, module_id).await?;
        }
        ModuleCommand::Status { module_id } => {
            let params = json!({ "module_id": module_id });
            let response = ipc_call(args, "module_status", Some(params)).await?;
            print_response(&response)?;
        }
        ModuleCommand::Remove { module_id } => {
            let params = json!({ "module_id": module_id });
            let response = ipc_call(args, "remove_module", Some(params)).await?;
            print_response(&response)?;
        }
    }
    Ok(())
}

async fn track_module(args: &Args, module_id: &str) -> Result<()> {
    let params = json!({ "module_id": module_id });
    let start = std::time::Instant::now();
    let timeout = Duration::from_secs(300);

    loop {
        if start.elapsed() > timeout {
            anyhow::bail!("timed out waiting for module '{}'", module_id);
        }

        tokio::time::sleep(Duration::from_millis(500)).await;
        let response = ipc_call(args, "module_status", Some(params.clone())).await?;

        if let Some(error) = &response.error {
            println!("Error: {}", error.message);
            if let Some(data) = &error.data {
                println!("{}", serde_json::to_string_pretty(data).unwrap_or_default());
            }
            return Ok(());
        }

        let state = response
            .result
            .as_ref()
            .and_then(|r| r.get("state"))
            .and_then(|s| s.as_str())
            .unwrap_or("unknown");

        let bytes_downloaded = response
            .result
            .as_ref()
            .and_then(|r| r.get("bytes_downloaded"))
            .and_then(|b| b.as_u64())
            .unwrap_or(0);

        let bytes_total = response
            .result
            .as_ref()
            .and_then(|r| r.get("bytes_total"))
            .and_then(|b| b.as_u64())
            .unwrap_or(0);

        match state {
            "ready" => {
                println!("Module '{}' is ready.", module_id);
                return Ok(());
            }
            "error" => {
                println!(
                    "Module '{}' failed: {:?}",
                    module_id,
                    response
                        .result
                        .as_ref()
                        .and_then(|r| r.get("error"))
                        .and_then(|e| e.as_str())
                        .unwrap_or("unknown error")
                );
                return Ok(());
            }
            "missing" => {
                println!("Module '{}' is missing.", module_id);
                return Ok(());
            }
            _ => {
                if bytes_total > 0 {
                    let pct = (bytes_downloaded as f64 / bytes_total as f64) * 100.0;
                    println!(
                        "{}: {}% ({}/{} bytes)",
                        module_id, pct as u32, bytes_downloaded, bytes_total
                    );
                } else {
                    println!("{}: {}", module_id, state);
                }
            }
        }
    }
}

async fn ipc_call(args: &Args, method: &str, params: Option<Value>) -> Result<mirage_daemon::ipc::protocol::JsonRpcResponse> {
    #[cfg(unix)]
    {
        let path = socket_path(args)?;
        IpcClient::call(&path, method, params)
            .await
            .with_context(|| format!("failed to call {} via {}", method, path.display()))
    }
    #[cfg(windows)]
    {
        let name = pipe_name(args)?;
        IpcClient::call(&name, method, params)
            .await
            .with_context(|| format!("failed to call {} via {}", method, name))
    }
}

fn print_response(response: &mirage_daemon::ipc::protocol::JsonRpcResponse) -> Result<()> {
    if let Some(error) = &response.error {
        eprintln!("Error (code {}): {}", error.code, error.message);
        if let Some(data) = &error.data {
            eprintln!("{}", serde_json::to_string_pretty(data).unwrap_or_default());
        }
    } else if let Some(result) = &response.result {
        println!("{}", serde_json::to_string_pretty(result).unwrap_or_default());
    }
    Ok(())
}
