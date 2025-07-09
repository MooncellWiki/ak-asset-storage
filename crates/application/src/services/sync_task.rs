use crate::error::AppResult;
use crate::ports::scheduler::ScheduledTask;
use crate::ports::{
    external_services::{AkApiClient, NotificationService},
    repositories::VersionRepository,
};
use crate::services::AssetDownloadService;
use crate::{BundleRepository, FileRepository, StorageService, VersionCheckService};
use async_trait::async_trait;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::spawn;
use tokio::task::JoinHandle;
use tracing::{error, info, instrument};

/// 版本轮询服务 - 组合版本检查和资源下载
pub struct SyncTask<V, F, B, A, N, S>
where
    V: VersionRepository + Send + Sync + 'static,
    F: FileRepository + Send + Sync + 'static,
    B: BundleRepository + Send + Sync + 'static,
    A: AkApiClient + Send + Sync + 'static,
    N: NotificationService + Send + Sync + 'static,
    S: StorageService + Send + Sync + 'static,
{
    version_check_service: Arc<VersionCheckService<V, A, N>>,
    download_service: Arc<AssetDownloadService<V, F, B, A, N, S>>,
    poll_interval: Duration,
    download_task: Arc<Mutex<Option<JoinHandle<()>>>>,
}

impl<V, F, B, A, N, S> SyncTask<V, F, B, A, N, S>
where
    V: VersionRepository + Send + Sync + 'static,
    F: FileRepository + Send + Sync + 'static,
    B: BundleRepository + Send + Sync + 'static,
    A: AkApiClient + Send + Sync + 'static,
    N: NotificationService + Send + Sync + 'static,
    S: StorageService + Send + Sync + 'static,
{
    pub fn new(
        version_check_service: VersionCheckService<V, A, N>,
        download_service: AssetDownloadService<V, F, B, A, N, S>,
        poll_interval: Duration,
    ) -> Self {
        Self {
            version_check_service: Arc::new(version_check_service),
            download_service: Arc::new(download_service),
            poll_interval,
            download_task: Arc::new(Mutex::new(None)),
        }
    }

    /// 执行完整的轮询周期：检查版本 -> 下载资源
    #[instrument(name = "services.version_poll", skip_all)]
    pub async fn perform_poll(&self) -> AppResult<()> {
        // 1. 检查版本更新
        match self.version_check_service.perform_check().await {
            Ok(has_update) => {
                if has_update {
                    info!("New version detected, starting download...");
                    let mut task = self.download_task.lock().unwrap();
                    if task.is_some() {
                        info!("Download task is already running");
                    } else {
                        let download_service = self.download_service.clone();
                        *task = Some(spawn(async move {
                            loop {
                                match download_service.perform_download().await {
                                    Ok(has_more) => {
                                        if !has_more {
                                            info!("No more versions to download, exiting loop");
                                            break;
                                        }
                                        info!("Continuing download for more versions");
                                    }
                                    Err(e) => {
                                        error!("Download failed: {:?}", e);
                                        break;
                                    }
                                }
                            }
                        }));
                    }
                }
            }
            Err(e) => {
                error!("Version check failed: {:?}", e);
                return Err(e);
            }
        }

        Ok(())
    }
}

#[async_trait]
impl<V, F, B, A, N, S> ScheduledTask for SyncTask<V, F, B, A, N, S>
where
    V: VersionRepository + Send + Sync + 'static,
    F: FileRepository + Send + Sync + 'static,
    B: BundleRepository + Send + Sync + 'static,
    A: AkApiClient + Send + Sync + 'static,
    N: NotificationService + Send + Sync + 'static,
    S: StorageService + Send + Sync + 'static,
{
    async fn run(&self) -> AppResult<()> {
        match self.perform_poll().await {
            Ok(()) => {
                info!("Version poll completed successfully");
                Ok(())
            }
            Err(e) => {
                error!("Version poll failed: {:?}", e);
                Err(e)
            }
        }
    }

    fn interval(&self) -> Duration {
        self.poll_interval
    }

    fn on_error(&self, error: &crate::error::AppError) {
        error!("Version poll task failed: {:?}", error);
    }
    fn stop(&self) {
        let value = self.download_task.lock().unwrap().take();
        if let Some(handle) = value {
            info!("Stopping download task");
            handle.abort();
        }
    }
}
