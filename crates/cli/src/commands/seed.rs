use anyhow::Result;
use application::{AssetDownloadService, ConfigProvider, RemoteVersion, VersionCheckService};
use infrastructure::{
    HttpAkApiClient, PostgresBundleRepository, PostgresFileRepository, PostgresVersionRepository,
    S3StorageClient, SmtpNotificationClient,
};
use sqlx::postgres::PgPoolOptions;
use std::{fs, path::PathBuf};
use tracing::info;

pub async fn execute(config: &impl ConfigProvider, csv_path: &PathBuf) -> Result<()> {
    info!("Starting database seed...");
    let content = fs::read_to_string(csv_path)?;
    let versions = content
        .split('\n')
        .filter(|line| !line.is_empty())
        .map(|line| {
            let parts = line.split(',').collect::<Vec<&str>>();
            RemoteVersion {
                res_version: parts[1].to_string(),
                client_version: parts[2].to_string(),
            }
        })
        .collect::<Vec<RemoteVersion>>();

    info!("Seeding database from CSV file: {:?}", csv_path);
    let pool = PgPoolOptions::new()
        .connect(&config.database_config().uri)
        .await?;
    let version_repo = PostgresVersionRepository::new(pool.clone());
    let file_repo = PostgresFileRepository::new(pool.clone());
    let bundle_repo = PostgresBundleRepository::new(pool.clone());
    let ak_api_client = HttpAkApiClient::new(config.ak_api_config());
    let notification = SmtpNotificationClient::new(config.smtp_config())?;
    let s3 = S3StorageClient::new(config.s3_config())?;
    let version_check_service = VersionCheckService::new(
        version_repo.clone(),
        ak_api_client.clone(),
        notification.clone(),
    );
    let download_service = AssetDownloadService::new(
        version_repo.clone(),
        file_repo,
        bundle_repo,
        ak_api_client,
        notification,
        s3,
    );
    for remote in versions {
        info!(
            "Inserting new version: {}-{}",
            remote.client_version, remote.res_version
        );
        version_check_service.check_and_save(remote).await?;
    }
    loop {
        match download_service.perform_download().await {
            Ok(has_more) => {
                if !has_more {
                    info!("All downloads completed successfully.");
                    break;
                }
            }
            Err(e) => {
                info!("Error during download: {}", e);
                break;
            }
        };
    }

    info!("Database seeding completed.");
    Ok(())
}
