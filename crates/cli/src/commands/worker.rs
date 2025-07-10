use anyhow::Result;
use application::{AssetDownloadService, ConfigProvider, SyncTask, VersionCheckService};
use infrastructure::{
    shutdown_signal, HttpAkApiClient, PostgresRepository, S3StorageClient, SimpleScheduler,
    SmtpNotificationClient,
};
use sqlx::postgres::PgPoolOptions;
use std::time::Duration;
use tracing::info;

pub async fn execute(config: &impl ConfigProvider) -> Result<()> {
    info!("Starting worker...");
    let pool = PgPoolOptions::new()
        .connect(&config.database_config().uri)
        .await?;
    let repository = PostgresRepository { pool };
    let ak_api_client = HttpAkApiClient::new(config.ak_api_config());
    let notification = SmtpNotificationClient::new(config.smtp_config())?;
    let s3 = S3StorageClient::new(config.s3_config())?;
    let mut scheduler = SimpleScheduler::new(SyncTask::new(
        VersionCheckService::new(
            repository.clone(),
            ak_api_client.clone(),
            notification.clone(),
        ),
        AssetDownloadService::new(repository.clone(), ak_api_client, notification, s3),
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
