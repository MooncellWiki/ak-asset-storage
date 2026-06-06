pub mod asset_mapping;
pub mod bundle;
pub mod file;
pub mod item_demand;
pub mod model;
pub mod row;
pub mod version;

use crate::{AppError, AppResult, config::DatabaseConfig};
use sqlx::{PgPool, postgres::PgPoolOptions};
use std::time::Duration;
use tracing::info;

#[derive(Debug, Clone)]
pub struct Database {
    pool: PgPool,
}

impl Database {
    pub async fn connect(config: &DatabaseConfig) -> AppResult<Self> {
        let mut options = PgPoolOptions::new();
        if let Some(max_connections) = config.max_connections {
            options = options.max_connections(max_connections);
        }
        if let Some(timeout_seconds) = config.connection_timeout_seconds {
            options = options.acquire_timeout(Duration::from_secs(timeout_seconds));
        }

        let pool = options
            .connect(&config.uri)
            .await
            .map_err(|err| AppError::ExternalService(err.into()))?;

        Ok(Self { pool })
    }

    #[must_use]
    pub const fn pool(&self) -> &PgPool {
        &self.pool
    }

    pub async fn health_check(&self) -> bool {
        sqlx::query_scalar!("SELECT 1")
            .fetch_one(&self.pool)
            .await
            .is_ok()
    }

    pub async fn migrate(&self) -> AppResult<()> {
        info!("Running database migrations");
        sqlx::migrate!("./migrations")
            .run(&self.pool)
            .await
            .map_err(|err| AppError::ExternalService(err.into()))?;
        info!("Database migrations completed");
        Ok(())
    }
}
