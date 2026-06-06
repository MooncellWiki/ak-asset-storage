use crate::{
    AppResult,
    service::{asset_download::AssetDownloadService, version_check::VersionCheckService},
};
use std::{
    sync::{Arc, Mutex},
    time::Duration,
};
use tokio::{spawn, task::JoinHandle, time::sleep};
use tracing::{error, info, instrument};

pub struct SyncWorker {
    version_check: Arc<VersionCheckService>,
    download: Arc<AssetDownloadService>,
    poll_interval: Duration,
    download_task: Arc<Mutex<Option<JoinHandle<()>>>>,
}

impl SyncWorker {
    #[must_use]
    pub fn new(
        version_check: VersionCheckService,
        download: AssetDownloadService,
        poll_interval: Duration,
    ) -> Self {
        let download = Arc::new(download);
        let worker = Self {
            version_check: Arc::new(version_check),
            download,
            poll_interval,
            download_task: Arc::new(Mutex::new(None)),
        };
        worker
            .download_task
            .lock()
            .unwrap()
            .replace(worker.start_download_task());
        worker
    }

    fn start_download_task(&self) -> JoinHandle<()> {
        let download = self.download.clone();
        let download_task = self.download_task.clone();
        spawn(async move {
            loop {
                match download.perform_download().await {
                    Ok(has_more) => {
                        if !has_more {
                            info!("No more versions to download, exiting loop");
                            break;
                        }
                        info!("Continuing download for more versions");
                    }
                    Err(err) => {
                        error!("Download failed: {err:?}");
                        sleep(Duration::from_mins(1)).await;
                    }
                }
            }
            download_task.lock().unwrap().take();
        })
    }

    #[instrument(name = "services.version_poll", skip_all)]
    pub async fn perform_poll(&self) -> AppResult<()> {
        match self.version_check.perform_check().await {
            Ok(has_update) => {
                if has_update {
                    info!("New version detected, starting download...");
                    let mut task = self.download_task.lock().unwrap();
                    if task.as_ref().is_none_or(JoinHandle::is_finished) {
                        *task = Some(self.start_download_task());
                    } else {
                        info!("Download task is already running");
                    }
                    drop(task);
                }
                Ok(())
            }
            Err(err) => {
                error!("Version check failed: {err:?}");
                Err(err)
            }
        }
    }

    pub async fn run(&self) {
        let mut interval = tokio::time::interval(self.poll_interval);
        interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

        loop {
            interval.tick().await;
            if let Err(err) = self.perform_poll().await {
                error!("Version poll task failed: {err:?}");
            }
        }
    }

    pub fn stop(&self) {
        let value = self.download_task.lock().unwrap().take();
        if let Some(handle) = value {
            info!("Stopping download task");
            handle.abort();
        }
    }
}
