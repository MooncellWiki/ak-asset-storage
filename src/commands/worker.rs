use crate::{
    AppResult,
    config::AppSettings,
    database::Database,
    external::{
        ak_api::AkApi, docker::DockerClient, github::GithubClient,
        notification::NotificationClient, s3::S3Storage,
    },
    runtime,
    service::{
        asset_download::AssetDownloadService, asset_mapping_import::AssetMappingImportService,
        version_check::VersionCheckService,
    },
    worker::{manifest_watcher::ManifestWatcher, sync::SyncWorker},
};
use std::{path::PathBuf, time::Duration};
use tracing::info;

pub async fn execute(settings: &AppSettings, concurrent: usize) -> AppResult<()> {
    info!("Starting worker...");
    let database = Database::connect(&settings.database).await?;
    let ak_api = AkApi::new(&settings.ak)?;
    let notification = NotificationClient::new(&settings.mailer)?;
    let s3 = S3Storage::new(&settings.s3)?;

    let docker = if let Some(docker_config) = &settings.torappu.docker {
        info!("Docker configuration found, creating Docker client");
        Some(DockerClient::new(docker_config.clone())?)
    } else {
        info!("Docker configuration not found, skipping Docker service");
        None
    };

    let github = if let Some(github_config) = &settings.torappu.github {
        info!("GitHub configuration found, creating GitHub client");
        Some(GithubClient::new(github_config.clone())?)
    } else {
        info!("GitHub configuration not found, skipping GitHub service");
        None
    };

    let sync_worker = SyncWorker::new(
        VersionCheckService {
            database: database.clone(),
            ak_api: ak_api.clone(),
            notification: notification.clone(),
            docker,
            github,
        },
        AssetDownloadService {
            database: database.clone(),
            ak_api,
            notification,
            storage: s3,
            concurrent,
        },
        Duration::from_mins(2),
    );

    let gamedata_root = PathBuf::from(&settings.torappu.asset_base_path).join("gamedata");
    let import_service = AssetMappingImportService {
        database,
        gamedata_root: gamedata_root.clone(),
    };
    let manifest_watcher = ManifestWatcher::new(import_service, &gamedata_root)
        .map_err(crate::AppError::Application)?;

    info!("Worker is running. Press Ctrl+C to stop.");
    tokio::select! {
        () = sync_worker.run() => {
            info!("Worker loop exited.");
        }
        () = runtime::shutdown_signal() => {
            info!("Shutdown signal received, stopping worker...");
            sync_worker.stop();
        }
    }

    drop(manifest_watcher);
    info!("Worker has stopped.");
    Ok(())
}
