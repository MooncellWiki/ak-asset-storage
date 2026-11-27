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
use anyhow::Context;
use std::{fs, path::Path};
use tracing::info;

pub async fn execute(
    settings: &AppSettings,
    csv_path: &Path,
    concurrent: usize,
    backfill: bool,
) -> AppResult<()> {
    info!("Starting database seed...");
    let content = fs::read_to_string(csv_path)?;
    let versions = content
        .lines()
        .enumerate()
        .filter(|(_, line)| !line.trim().is_empty())
        .map(|(index, line)| {
            let (res, client) = line
                .split_once(',')
                .map(|(res, client)| (res.trim(), client.trim()))
                .filter(|(res, client)| {
                    !res.is_empty() && !client.is_empty() && !client.contains(',')
                })
                .with_context(|| {
                    format!(
                        "expected res_version,client_version at CSV line {}",
                        index + 1
                    )
                })?;
            Ok(RemoteVersion {
                res_version: res.to_string(),
                client_version: client.to_string(),
            })
        })
        .collect::<anyhow::Result<Vec<_>>>()?;

    info!("Seeding database from CSV file: {:?}", csv_path);
    let database = Database::connect(&settings.database).await?;
    let ak_api = AkApi::new(&settings.ak)?;
    let storage = S3Storage::new(&settings.s3)?;
    let notification = if backfill {
        NotificationClient::Empty
    } else {
        NotificationClient::new(&settings.mailer)?
    };
    let version_check = VersionCheckService {
        database: database.clone(),
        ak_api: ak_api.clone(),
        notification: notification.clone(),
        kubernetes: None,
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
            "Importing version: {}-{}",
            remote.client_version, remote.res_version
        );
        if backfill {
            if let Some(id) = version_check.prepare_backfill(&remote).await? {
                download.manual_download(Some(id)).await?;
            }
        } else {
            version_check.check_and_save(remote).await?;
        }
    }
    if !backfill {
        while download.perform_download().await? {}
    }

    info!("Database seeding completed.");
    Ok(())
}
