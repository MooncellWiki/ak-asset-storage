use ak_asset_storage_application::ConfigProvider;
use ak_asset_storage_infrastructure::{
    config::InfraConfigProvider, PostgresRepository, TorappuAssetClient,
};
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct AppState {
    pub repository: PostgresRepository,
    pub config: InfraConfigProvider,
    pub torappu: TorappuAssetClient,
}

pub async fn init_state_with_pg(config: InfraConfigProvider) -> AppState {
    let pool = sqlx::postgres::PgPoolOptions::new()
        .connect(&config.database_config().uri)
        .await
        .expect("Failed to connect to the database");

    AppState {
        repository: PostgresRepository { pool },
        torappu: TorappuAssetClient {
            asset_base_path: PathBuf::from(&config.torappu_config().asset_base_path),
        },
        config,
    }
}
