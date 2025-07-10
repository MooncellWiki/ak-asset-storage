use application::ConfigProvider;
use infrastructure::PostgresRepository;

#[derive(Debug, Clone)]
pub struct AppState {
    pub repository: PostgresRepository,
}

pub async fn init_state_with_pg(config: &impl ConfigProvider) -> AppState {
    let pool = sqlx::postgres::PgPoolOptions::new()
        .connect(&config.database_config().uri)
        .await
        .expect("Failed to connect to the database");

    AppState {
        repository: PostgresRepository { pool },
    }
}
