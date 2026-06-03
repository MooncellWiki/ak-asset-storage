use crate::{
    AppResult,
    config::AppSettings,
    database::Database,
    external::{ak_api::AkApi, notification::NotificationClient, s3::S3Storage},
    service::{
        asset_download::AssetDownloadService, types::RemoteVersion,
        version_check::VersionCheckService,
    },
};
use std::{fs, path::PathBuf};
use tracing::info;

pub async fn execute(
    settings: &AppSettings,
    csv_path: &PathBuf,
    concurrent: usize,
) -> AppResult<()> {
    info!("Starting database seed...");
    let content = fs::read_to_string(csv_path)?;
    let versions = content
        .split('\n')
        .filter(|line| !line.is_empty())
        .map(|line| {
            let parts = line.split(',').collect::<Vec<&str>>();
            RemoteVersion {
                res_version: parts[0].to_string(),
                client_version: parts[1].to_string(),
            }
        })
        .collect::<Vec<_>>();

    info!("Seeding database from CSV file: {:?}", csv_path);
    let database = Database::connect(&settings.database).await?;
    let ak_api = AkApi::new(&settings.ak);
    let storage = S3Storage::new(&settings.s3)?;
    let notification = NotificationClient::new(&settings.mailer)?;
    let version_check = VersionCheckService {
        database: database.clone(),
        ak_api: ak_api.clone(),
        notification: notification.clone(),
        docker: None,
        github: None,
    };
    let download = AssetDownloadService {
        database,
        ak_api,
        notification,
        storage,
        concurrent,
    };

    for remote in versions {
        info!(
            "Inserting new version: {}-{}",
            remote.client_version, remote.res_version
        );
        version_check.check_and_save(remote).await?;
    }

    loop {
        match download.perform_download().await {
            Ok(executed) => {
                if !executed {
                    info!("All downloads completed successfully.");
                    break;
                }
            }
            Err(err) => {
                info!("Error during download: {}", err);
                break;
            }
        }
    }

    info!("Database seeding completed.");
    Ok(())
}
