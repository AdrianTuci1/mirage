use anyhow::{Context, Result};
use clap::Parser;
use mirage_daemon::{
    create_embedder, logging, Analytics, DaemonConfig, FileWatcher, HeuristicSlmEngine, IpcServer,
    LanceDbStore, ModuleManager, UnifiedSearch,
};
use std::path::PathBuf;
use std::sync::Arc;

#[derive(Parser, Debug)]
#[command(name = "mirage-daemon")]
#[command(about = "Mirage core daemon")]
struct Args {
    #[arg(long)]
    data_dir: Option<PathBuf>,

    #[arg(long)]
    models_dir: Option<PathBuf>,

    #[arg(long)]
    downloads_dir: Option<PathBuf>,

    #[arg(long)]
    catalog_url: Option<String>,

    #[cfg(unix)]
    #[arg(long)]
    socket_path: Option<PathBuf>,

    #[cfg(windows)]
    #[arg(long)]
    pipe_name: Option<String>,

    #[arg(long)]
    log_level: Option<String>,

    #[arg(long)]
    config: Option<PathBuf>,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    let config_path = args
        .config
        .clone()
        .unwrap_or_else(|| DaemonConfig::base_dir().join("daemon.yaml"));

    let mut config = DaemonConfig::load(&config_path)
        .with_context(|| format!("failed to load config from {}", config_path.display()))?;

    if let Some(data_dir) = args.data_dir {
        config.data_dir = data_dir;
    }
    if let Some(models_dir) = args.models_dir {
        config.models_dir = models_dir;
    }
    if let Some(downloads_dir) = args.downloads_dir {
        config.downloads_dir = downloads_dir;
    }
    if let Some(catalog_url) = args.catalog_url {
        config.catalog_url = Some(catalog_url);
    }
    #[cfg(unix)]
    if let Some(socket_path) = args.socket_path {
        config.socket_path = socket_path;
    }
    #[cfg(windows)]
    if let Some(pipe_name) = args.pipe_name {
        config.pipe_name = pipe_name;
    }
    if let Some(log_level) = &args.log_level {
        config.log_level.clone_from(log_level);
    }

    logging::init(&config.log_level);

    config
        .ensure_dirs()
        .context("failed to create daemon directories")?;

    if let Some(parent) = config_path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("failed to create config directory {}", parent.display()))?;
    }
    config
        .save(&config_path)
        .with_context(|| format!("failed to save config to {}", config_path.display()))?;

    // The embedder decides the vector layout, so the store is opened with its
    // dimension instead of the configured default.
    let embedder = create_embedder(&config.models_dir, &config.downloads_dir)
        .context("failed to initialize embedder")?;
    let store = Arc::new(
        LanceDbStore::open_with_dimension(&config, embedder.dimension())
            .await
            .context("failed to open LanceDB store")?,
    );
    let analytics = Arc::new(Analytics::open(&config).context("failed to open analytics")?);
    let module_manager = Arc::new(ModuleManager::new(&config, None).await);

    cfg_if::cfg_if! {
        if #[cfg(feature = "onnx")] {
            use mirage_daemon::OnnxSlmEngine;
            let slm: Arc<dyn mirage_daemon::SlmEngine> = match OnnxSlmEngine::new(&config.models_dir) {
                Ok(engine) => Arc::new(engine.with_top_k(10)),
                Err(e) => {
                    tracing::warn!("ONNX SLM not available: {}. Using heuristic fallback.", e);
                    Arc::new(HeuristicSlmEngine::new(10))
                }
            };
        } else {
            let slm: Arc<dyn mirage_daemon::SlmEngine> = Arc::new(HeuristicSlmEngine::new(10));
        }
    }
    let connectors = mirage_daemon::connectors::registry_from_config(&config.connectors);
    let unified_search = Arc::new(UnifiedSearch::new(
        Arc::clone(&store),
        Arc::clone(&embedder),
        &config,
        connectors,
    ));

    // File system changes only mark the index as stale. Mirage never starts an
    // indexing pass on its own: the user presses "Start indexing", or a worker is
    // asked to do the work.
    let watcher_roots = config.roots.clone();
    let watcher_state = unified_search.index_state();

    let server = IpcServer::new(
        store,
        embedder,
        analytics,
        module_manager,
        slm,
        unified_search,
        config_path.clone(),
        config.clone(),
    );

    let server_handle = tokio::spawn(async move { server.run().await });

    // Registering a recursive watch walks the tree, which on a home-directory root
    // takes far longer than the client's startup timeout. The socket is already
    // accepting requests by now, so this runs off the main task instead of before it.
    let watcher = tokio::task::spawn_blocking(move || {
        FileWatcher::new(
            watcher_roots,
            Arc::new(move || {
                tracing::debug!("file system changed, marking the index stale");
                watcher_state.mark_stale();
            }),
        )
        .ok()
    })
    .await
    .unwrap_or(None);
    if watcher.is_none() {
        tracing::warn!("failed to start file watcher");
    }

    tokio::select! {
        result = server_handle => {
            result??;
        }
        _ = tokio::signal::ctrl_c() => {
            tracing::info!("received Ctrl+C, shutting down");
        }
    }

    Ok(())
}
