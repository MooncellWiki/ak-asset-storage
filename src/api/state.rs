use crate::{
    AppResult,
    config::AppSettings,
    database::Database,
    external::{docker::DockerClient, torappu::TorappuClient},
};
use std::{path::PathBuf, sync::Arc};
use tracing::{info, warn};

#[derive(Debug, Clone)]
pub struct AppState {
    pub database: Database,
    pub settings: Arc<AppSettings>,
    pub torappu: TorappuClient,
    pub docker: Option<DockerClient>,
}

impl AppState {
    pub async fn from_settings(settings: Arc<AppSettings>) -> AppResult<Self> {
        let database = Database::connect(&settings.database).await?;
        let docker = settings.torappu.docker.as_ref().map_or_else(
            || {
                info!("Docker configuration not found, skipping Docker service");
                Ok(None)
            },
            |docker_config| {
                info!("Docker configuration found, creating Docker client");
                DockerClient::new(docker_config.clone())
                    .map(Some)
                    .map_err(|err| {
                        warn!("Failed to create Docker client: {err}");
                        err
                    })
            },
        )?;

        Ok(Self {
            database,
            torappu: TorappuClient {
                asset_base_path: PathBuf::from(&settings.torappu.asset_base_path),
            },
            settings,
            docker,
        })
    }
}
