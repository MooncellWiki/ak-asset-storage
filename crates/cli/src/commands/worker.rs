use anyhow::Result;
use application::{
    AssetDownloadService, ConfigProvider, SimpleScheduler, SyncTask, VersionCheckService,
};
use infrastructure::{
    shutdown_signal, HttpAkApiClient, PostgresBundleRepository, PostgresFileRepository,
    PostgresVersionRepository, S3StorageClient, SmtpNotificationClient,
};
use sqlx::postgres::PgPoolOptions;
use std::time::Duration;
use tracing::info;

pub async fn execute(config: &impl ConfigProvider) -> Result<()> {
    info!("Starting worker...");
    let pool = PgPoolOptions::new()
        .connect(&config.database_config().uri)
        .await?;
    let version_repo = PostgresVersionRepository::new(pool.clone());
    let file_repo = PostgresFileRepository::new(pool.clone());
    let bundle_repo = PostgresBundleRepository::new(pool.clone());
    let ak_api_client = HttpAkApiClient::new(config.ak_api_config());
    let notification = SmtpNotificationClient::new(config.smtp_config())?;
    let s3 = S3StorageClient::new(config.s3_config())?;
    let mut scheduler = SimpleScheduler::new(SyncTask::new(
        VersionCheckService::new(
            version_repo.clone(),
            ak_api_client.clone(),
            notification.clone(),
        ),
        AssetDownloadService::new(
            version_repo,
            file_repo,
            bundle_repo,
            ak_api_client,
            notification,
            s3,
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
