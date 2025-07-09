use application::ConfigProvider;
use axum::extract::FromRef;
use infrastructure::{PostgresBundleRepository, PostgresVersionRepository};

#[derive(Debug, Clone)]
pub struct AppState {
    pub bundle_repository: PostgresBundleRepository,
    pub version_repository: PostgresVersionRepository,
}

pub async fn init_state_with_pg(config: &impl ConfigProvider) -> AppState {
    let pool = sqlx::postgres::PgPoolOptions::new()
        .connect(&config.database_config().uri)
        .await
        .expect("Failed to connect to the database");

    let bundle_repository = PostgresBundleRepository::new(pool.clone());
    let version_repository = PostgresVersionRepository::new(pool);

    AppState {
        bundle_repository,
        version_repository,
    }
}

impl FromRef<AppState> for PostgresBundleRepository {
    fn from_ref(input: &AppState) -> Self {
        input.bundle_repository.clone()
    }
}

impl FromRef<AppState> for PostgresVersionRepository {
    fn from_ref(input: &AppState) -> Self {
        input.version_repository.clone()
    }
}
