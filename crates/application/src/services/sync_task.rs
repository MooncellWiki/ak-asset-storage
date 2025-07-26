use crate::{
    AkApiClient, AppResult, AssetDownloadService, BundleRepository, DockerService, FileRepository,
    NotificationService, ScheduledTask, StorageService, VersionCheckService, VersionRepository,
};
use async_trait::async_trait;
use std::{
    sync::{Arc, Mutex},
    time::Duration,
};
use tokio::{spawn, task::JoinHandle, time::sleep};
use tracing::{error, info, instrument};

/// 版本轮询服务 - 组合版本检查和资源下载
pub struct SyncTask<R, A, N, S, D>
where
    R: VersionRepository + FileRepository + BundleRepository,
    A: AkApiClient,
    N: NotificationService,
    S: StorageService,
    D: DockerService,
{
    version_check_service: Arc<VersionCheckService<R, A, N, D>>,
    download_service: Arc<AssetDownloadService<R, A, N, S>>,
    poll_interval: Duration,
    download_task: Arc<Mutex<Option<JoinHandle<()>>>>,
}

impl<R, A, N, S, D> SyncTask<R, A, N, S, D>
where
    R: VersionRepository + FileRepository + BundleRepository,
    A: AkApiClient,
    N: NotificationService,
    S: StorageService,
    D: DockerService,
{
    pub fn new(
        version_check_service: VersionCheckService<R, A, N, D>,
        download_service: AssetDownloadService<R, A, N, S>,
        poll_interval: Duration,
    ) -> Self {
        let download_service = Arc::new(download_service);
        Self {
            version_check_service: Arc::new(version_check_service),
            download_service: download_service.clone(),
            poll_interval,
            download_task: Arc::new(Mutex::new(Some(Self::start_download_task(
                download_service,
            )))),
        }
    }
    fn start_download_task(
        download_service: Arc<AssetDownloadService<R, A, N, S>>,
    ) -> JoinHandle<()> {
        spawn(async move {
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
                        sleep(Duration::from_secs(60)).await; // Wait before retrying
                    }
                }
            }
        })
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
                        *task = Some(Self::start_download_task(download_service));
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
impl<R, A, N, S, D> ScheduledTask for SyncTask<R, A, N, S, D>
where
    R: VersionRepository + FileRepository + BundleRepository,
    A: AkApiClient,
    N: NotificationService,
    S: StorageService,
    D: DockerService,
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
