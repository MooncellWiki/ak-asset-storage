mod file_watcher;
pub mod gamedata_ready_watcher;
pub mod item_demand_watcher;
pub mod launcher;
pub mod manifest_watcher;
pub mod sync;

use std::time::Duration;

/// Polling cadence shared by the file watchers.
#[derive(Clone, Copy, Debug)]
pub struct WatchTiming {
    /// How often watched paths are rescanned and pending imports are checked.
    pub interval: Duration,
    /// Quiet period after the last detected change before importing, so a
    /// file that is still being written is not imported half-way. Keep it
    /// longer than `interval` so the file is rescanned at least once first.
    pub debounce: Duration,
}
