use crate::utils::NotificationClient;
use ak_asset_storage_application::{
    AssetDownloadService, ConfigProvider, SyncTask, VersionCheckService,
};
use ak_asset_storage_infrastructure::{
    shutdown_signal, BollardDockerService, HttpAkApiClient, PostgresRepository, S3StorageClient,
    SimpleScheduler,
};
use anyhow::Result;
use sqlx::postgres::PgPoolOptions;
use std::time::Duration;
use tracing::info;

pub async fn execute(config: &impl ConfigProvider, concurrent: usize) -> Result<()> {
    info!("Starting worker...");
    let pool = PgPoolOptions::new()
        .connect(&config.database_config().uri)
        .await?;
    let repository = PostgresRepository { pool };
    let ak_api_client = HttpAkApiClient::new(config.ak_api_config());
    let notification = NotificationClient::new(config.smtp_config())?;
    let s3 = S3StorageClient::new(config.s3_config())?;

    // 创建Docker服务（如果配置存在）
    let docker_service = if let Some(docker_config) = &config.torappu_config().docker {
        info!("Docker configuration found, creating Docker client");
        Some(BollardDockerService::new(docker_config.clone())?)
    } else {
        info!("Docker configuration not found, skipping Docker service");
        None
    };

    let mut scheduler = SimpleScheduler::new(SyncTask::new(
        VersionCheckService::new(
            repository.clone(),
            ak_api_client.clone(),
            notification.clone(),
            docker_service.clone(),
        ),
        AssetDownloadService::new(
            repository.clone(),
            ak_api_client,
            notification,
            s3,
            concurrent,
        ),
        Duration::from_secs(2 * 60),
    ));
    scheduler.start()?;
    // Wait for shutdown signal
    info!("Worker is running. Press Ctrl+C to stop.");
    shutdown_signal().await;
    info!("Shutdown signal received, stopping worker...");
    scheduler.stop();
    info!("Worker has stopped.");
    Ok(())
}
