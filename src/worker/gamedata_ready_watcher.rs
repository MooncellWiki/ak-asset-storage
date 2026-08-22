use crate::{
    service::story_usage::StoryUsageImportService,
    worker::file_watcher::{PathResolution, PollingFileWatcher},
};
use std::path::Path;

pub struct GameDataReadyWatcher {
    _watcher: PollingFileWatcher,
}

impl GameDataReadyWatcher {
    #[must_use]
    pub fn new(service: StoryUsageImportService, marker_path: &Path) -> Self {
        // Keep the logical path: `latest` may be re-pointed at a new version,
        // so every scan must resolve the symlink again.
        let watcher = PollingFileWatcher::new(
            marker_path,
            PathResolution::Canonical,
            "gamedata ready marker",
            "story usages",
            move || {
                let service = service.clone();
                async move { service.import().await }
            },
        );
        Self { _watcher: watcher }
    }
}
