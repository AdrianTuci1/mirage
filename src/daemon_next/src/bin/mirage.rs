use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use mirage_daemon::{DaemonRunner, ipc::client::IpcClient};
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

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();
    let mut runner = DaemonRunner::from_config_path(args.config.clone())?;

    match &args.command {
        Command::Search { query, top_k } => search_command(&mut runner, query, *top_k).await,
        Command::Query { sql } => query_command(&mut runner, sql).await,
        Command::Status => status_command(&mut runner).await,
        Command::Ask { question } => ask_command(&mut runner, question).await,
        Command::Module(cmd) => module_command(&mut runner, cmd).await,
    }
}

async fn search_command(runner: &mut DaemonRunner, query: &str, top_k: usize) -> Result<()> {
    runner.ensure_running().await?;
    let params = json!({ "query": query, "top_k": top_k });
    let response = ipc_call(runner, "search", Some(params)).await?;
    print_response(&response)?;
    Ok(())
}

async fn query_command(runner: &mut DaemonRunner, sql: &str) -> Result<()> {
    runner.ensure_running().await?;
    let params = json!({ "sql": sql });
    let response = ipc_call(runner, "query", Some(params)).await?;
    print_response(&response)?;
    Ok(())
}

async fn status_command(runner: &mut DaemonRunner) -> Result<()> {
    runner.ensure_running().await?;
    let response = ipc_call(runner, "status", Some(json!({}))).await?;
    print_response(&response)?;
    Ok(())
}

async fn ask_command(runner: &mut DaemonRunner, question: &str) -> Result<()> {
    runner.ensure_running().await?;
    let params = json!({ "question": question });
    let response = ipc_call(runner, "ask", Some(params)).await?;
    print_response(&response)?;
    Ok(())
}

async fn module_command(runner: &mut DaemonRunner, cmd: &ModuleCommand) -> Result<()> {
    runner.ensure_running().await?;
    match cmd {
        ModuleCommand::List => {
            let response = ipc_call(runner, "list_modules", Some(json!({}))).await?;
            print_response(&response)?;
        }
        ModuleCommand::Install { module_id, force } => {
            let params = json!({ "module_id": module_id, "force": *force });
            let response = ipc_call(runner, "download_module", Some(params)).await?;
            if response.error.is_some() {
                print_response(&response)?;
                return Ok(());
            }
            println!("Module '{}' download requested. Tracking progress...", module_id);
            track_module(runner, module_id).await?;
        }
        ModuleCommand::Status { module_id } => {
            let params = json!({ "module_id": module_id });
            let response = ipc_call(runner, "module_status", Some(params)).await?;
            print_response(&response)?;
        }
        ModuleCommand::Remove { module_id } => {
            let params = json!({ "module_id": module_id });
            let response = ipc_call(runner, "remove_module", Some(params)).await?;
            print_response(&response)?;
        }
    }
    Ok(())
}

async fn track_module(runner: &mut DaemonRunner, module_id: &str) -> Result<()> {
    let params = json!({ "module_id": module_id });
    let start = std::time::Instant::now();
    let timeout = Duration::from_secs(300);

    loop {
        if start.elapsed() > timeout {
            anyhow::bail!("timed out waiting for module '{}'", module_id);
        }

        tokio::time::sleep(Duration::from_millis(500)).await;
        let response = ipc_call(runner, "module_status", Some(params.clone())).await?;

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

async fn ipc_call(runner: &mut DaemonRunner, method: &str, params: Option<Value>) -> Result<mirage_daemon::ipc::protocol::JsonRpcResponse> {
    #[cfg(unix)]
    {
        let path = runner.endpoint();
        IpcClient::call(path, method, params)
            .await
            .with_context(|| format!("failed to call {} via {}", method, path.display()))
    }
    #[cfg(windows)]
    {
        let name = runner.endpoint();
        IpcClient::call(name, method, params)
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
