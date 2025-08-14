use ak_asset_storage_application::ConfigProvider;
use ak_asset_storage_infrastructure::{
    config::InfraConfigProvider, BollardDockerClient, PostgresRepository, TorappuAssetClient,
};
use std::path::PathBuf;
use tracing::{info, warn};

#[derive(Debug, Clone)]
pub struct AppState {
    pub repository: PostgresRepository,
    pub config: InfraConfigProvider,
    pub torappu: TorappuAssetClient,
    pub docker_service: Option<BollardDockerClient>,
}

pub async fn init_state_with_pg(config: InfraConfigProvider) -> AppState {
    let pool = sqlx::postgres::PgPoolOptions::new()
        .connect(&config.database_config().uri)
        .await
        .expect("Failed to connect to the database");

    let docker_service = config.torappu_config().docker.as_ref().map_or_else(
        || {
            info!("Docker configuration not found, skipping Docker service");
            None
        },
        |docker_config| {
            info!("Docker configuration found, creating Docker client");
            match BollardDockerClient::new(docker_config.clone()) {
                Ok(client) => Some(client),
                Err(e) => {
                    warn!("Failed to create Docker client: {}", e);
                    None
                }
            }
        },
    );

    AppState {
        repository: PostgresRepository { pool },
        torappu: TorappuAssetClient {
            asset_base_path: PathBuf::from(&config.torappu_config().asset_base_path),
        },
        config,
        docker_service,
    }
}
