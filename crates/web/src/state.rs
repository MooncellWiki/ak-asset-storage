use ak_asset_storage_application::ConfigProvider;
use ak_asset_storage_infrastructure::{config::InfraConfigProvider, PostgresRepository};

#[derive(Debug, Clone)]
pub struct AppState {
    pub repository: PostgresRepository,
    pub config: InfraConfigProvider,
}

pub async fn init_state_with_pg(config: InfraConfigProvider) -> AppState {
    let pool = sqlx::postgres::PgPoolOptions::new()
        .connect(&config.database_config().uri)
        .await
        .expect("Failed to connect to the database");

    AppState {
        repository: PostgresRepository { pool },
        config,
    }
}
