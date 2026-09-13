use crate::{
    external::github::GithubClient,
    service::story_usage::StoryUsageImportService,
    worker::file_watcher::{PathResolution, PollingFileWatcher},
};
use std::path::Path;
use tracing::{error, info};

pub struct GameDataReadyWatcher {
    _watcher: PollingFileWatcher,
}

impl GameDataReadyWatcher {
    #[must_use]
    pub fn new(
        service: StoryUsageImportService,
        github: Option<GithubClient>,
        marker_path: &Path,
    ) -> Self {
        // Keep the logical path: `latest` may be re-pointed at a new version,
        // so every scan must resolve the symlink again.
        let watcher = PollingFileWatcher::new(
            marker_path,
            PathResolution::Canonical,
            "gamedata ready marker",
            "story usages",
            move || {
                let service = service.clone();
                let github = github.clone();
                async move {
                    if let Some(github) = github.as_ref() {
                        info!("Attempting to dispatch GitHub workflow for gamedata ready");
                        if let Err(err) = github.dispatch_workflow().await {
                            error!("Failed to dispatch GitHub workflow: {err}");
                        }
                    }
                    service.import().await
                }
            },
        );
        Self { _watcher: watcher }
    }
}
