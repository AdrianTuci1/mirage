pub mod analytics;
pub mod apps;
pub mod config;
pub mod connectors;
pub mod content;
pub mod daemon_runner;
pub mod db;
pub mod embeddings;
pub mod index_state;
pub mod ipc;
pub mod local_index;
pub mod logging;
pub mod models;
pub mod modules;
pub mod search;
pub mod slm;
pub mod watcher;

pub use daemon_runner::DaemonRunner;
pub use search::UnifiedSearch;

pub use analytics::Analytics;
pub use config::DaemonConfig;
pub use db::LanceDbStore;
pub use embeddings::{create_embedder, Embedder};
pub use ipc::IpcServer;
pub use modules::{ModuleEvent, ModuleManager, ModuleStatus};
pub use slm::{AskResponse, HeuristicSlmEngine, SlmEngine};
cfg_if::cfg_if! {
    if #[cfg(feature = "onnx")] {
        pub use slm::OnnxSlmEngine;
    }
}
pub use watcher::FileWatcher;
