use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

/// Live view of the indexing engine, shared between the indexing pass and the
/// IPC layer.
///
/// Mirage never starts a pass on its own: `running` only becomes true through an
/// explicit request (`index_files`) and `stale` is how the file watcher tells the
/// UI that the on-disk state no longer matches the index.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct IndexProgress {
    /// A pass is currently in flight.
    pub running: bool,
    /// Items embedded (or otherwise processed) so far in the current pass, or in
    /// the last completed pass when idle.
    pub indexed: u64,
    /// Total items discovered for the pass, unknown until the scan finishes.
    pub total: Option<u64>,
    /// Human readable description of the current step.
    pub phase: String,
    /// The indexed view is out of date with respect to the file system.
    pub stale: bool,
    /// Message from the last failed pass, cleared when a pass starts.
    pub error: Option<String>,
    /// Milliseconds since the last pass finished, used for the "delta 12 min ago"
    /// style labels in the UI.
    pub last_finished_at_ms: Option<u64>,
}

impl IndexProgress {
    /// Percentage complete, or `None` while the total is unknown or zero.
    pub fn percent(&self) -> Option<u64> {
        match self.total {
            Some(total) if total > 0 => {
                Some(((self.indexed.min(total) as f64 / total as f64) * 100.0).round() as u64)
            }
            _ => None,
        }
    }
}

/// Cheaply clonable handle onto a single [`IndexProgress`].
#[derive(Debug, Clone, Default)]
pub struct IndexState {
    inner: Arc<Mutex<IndexProgress>>,
}

impl IndexState {
    pub fn new() -> Self {
        Self::default()
    }

    /// Claim the right to run a pass. Returns `false` when one is already running.
    pub fn try_begin(&self) -> bool {
        let mut guard = self.lock();
        if guard.running {
            return false;
        }
        guard.running = true;
        guard.indexed = 0;
        guard.total = None;
        guard.phase = String::from("Starting");
        guard.error = None;
        true
    }

    pub fn set_phase(&self, phase: &str) {
        self.lock().phase = phase.to_string();
    }

    pub fn set_total(&self, total: u64) {
        self.lock().total = Some(total);
    }

    pub fn add_indexed(&self, count: u64) {
        self.lock().indexed += count;
    }

    /// Release the pass. `error` is recorded when the pass failed.
    pub fn finish(&self, error: Option<String>) {
        let mut guard = self.lock();
        guard.running = false;
        guard.error = error.clone();
        guard.phase = match &error {
            Some(_) => String::from("Failed"),
            None => String::from("Idle"),
        };
        guard.stale = false;
        guard.last_finished_at_ms = Some(now_ms());
    }

    pub fn mark_stale(&self) {
        self.lock().stale = true;
    }

    pub fn is_running(&self) -> bool {
        self.lock().running
    }

    pub fn snapshot(&self) -> IndexProgress {
        self.lock().clone()
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, IndexProgress> {
        self.inner.lock().unwrap_or_else(|e| e.into_inner())
    }
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_one_pass_can_run_at_a_time() {
        let state = IndexState::new();
        assert!(state.try_begin());
        assert!(!state.try_begin());
        state.finish(None);
        assert!(state.try_begin());
    }

    #[test]
    fn progress_reports_percentage_and_clamps() {
        let state = IndexState::new();
        state.try_begin();
        state.set_total(100);
        state.add_indexed(62);
        let snapshot = state.snapshot();
        assert_eq!(snapshot.percent(), Some(62));
        state.add_indexed(1_000);
        assert_eq!(state.snapshot().percent(), Some(100));
    }

    #[test]
    fn percentage_is_unknown_without_a_total() {
        let state = IndexState::new();
        state.try_begin();
        assert_eq!(state.snapshot().percent(), None);
    }

    #[test]
    fn finishing_clears_stale_and_records_failure() {
        let state = IndexState::new();
        state.try_begin();
        state.mark_stale();
        state.finish(Some(String::from("disk full")));
        let snapshot = state.snapshot();
        assert!(!snapshot.running);
        assert!(!snapshot.stale);
        assert_eq!(snapshot.phase, "Failed");
        assert_eq!(snapshot.error.as_deref(), Some("disk full"));
    }

    #[test]
    fn handle_is_shared_across_clones() {
        let state = IndexState::new();
        let clone = state.clone();
        state.try_begin();
        assert!(clone.is_running());
    }
}
