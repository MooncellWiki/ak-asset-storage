use crate::service::launcher_archive::LauncherArchiveService;
use std::time::Duration;
use tokio::time::{MissedTickBehavior, interval};
use tracing::error;

pub async fn run(service: &LauncherArchiveService, poll_interval: Duration) {
    let discovery = async {
        let mut timer = interval(poll_interval);
        timer.set_missed_tick_behavior(MissedTickBehavior::Skip);
        loop {
            timer.tick().await;
            if let Err(err) = service.discover().await {
                error!(?err, "PC launcher discovery failed");
            }
        }
    };
    let archival = async {
        let mut timer = interval(Duration::from_secs(5));
        timer.set_missed_tick_behavior(MissedTickBehavior::Skip);
        loop {
            timer.tick().await;
            if let Err(err) = service.archive_pending().await {
                error!(?err, "PC launcher archival failed");
            }
        }
    };
    tokio::join!(discovery, archival);
}
