use notify_debouncer_full::new_debouncer;
use notify_debouncer_full::notify::RecursiveMode;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

/// Watches the configured local roots and triggers a lightweight reindex when
/// files change. Cloud connectors are not watched; they are reindexed on demand.
pub struct FileWatcher {
    _handle: Option<std::thread::JoinHandle<()>>,
}

impl FileWatcher {
    /// Spawn a background watcher for `roots`. When file-system events settle,
    /// the provided `reindex` callback is invoked.
    pub fn new(
        roots: Vec<PathBuf>,
        reindex: Arc<dyn Fn() + Send + Sync>,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        if roots.is_empty() {
            return Ok(Self { _handle: None });
        }

        let (tx, rx) = std::sync::mpsc::channel::<()>();

        let mut debouncer = new_debouncer(
            Duration::from_secs(2),
            None,
            move |res: Result<Vec<notify_debouncer_full::DebouncedEvent>, Vec<notify_debouncer_full::notify::Error>>| {
                if let Ok(events) = res {
                    if !events.is_empty() {
                        let _ = tx.send(());
                    }
                }
            },
        )?;

        for root in roots {
            if root.exists() {
                debouncer.watch(&root, RecursiveMode::Recursive).ok();
            }
        }

        let handle = std::thread::spawn(move || {
            while rx.recv().is_ok() {
                tracing::info!("file system changed, triggering reindex");
                reindex();
            }
            // Keep the debouncer alive until the thread exits.
            let _ = debouncer;
        });

        Ok(Self {
            _handle: Some(handle),
        })
    }
}
