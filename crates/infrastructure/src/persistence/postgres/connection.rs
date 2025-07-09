use crate::error::{InfraError, InfraResult};
use sqlx::{postgres::PgPoolOptions, Pool, Postgres};
use std::time::Duration;
use tracing::info;

pub struct DatabaseConnection {
    pool: Pool<Postgres>,
}

impl DatabaseConnection {
    pub async fn new(database_url: &str) -> InfraResult<Self> {
        info!("Connecting to database: {}", database_url);

        let pool = PgPoolOptions::new()
            .max_connections(10)
            .acquire_timeout(Duration::from_secs(10))
            .idle_timeout(Duration::from_secs(600))
            .max_lifetime(Duration::from_secs(1800))
            .connect(database_url)
            .await
            .map_err(|e| InfraError::Database {
                message: "init connect failed".to_string(),
                source: e,
            })?;

        info!("Database connection established");

        Ok(Self { pool })
    }

    pub fn pool(&self) -> &Pool<Postgres> {
        &self.pool
    }

    pub async fn health_check(&self) -> InfraResult<()> {
        sqlx::query("SELECT 1")
            .execute(&self.pool)
            .await
            .map_err(|e| InfraError::Database {
                message: "health_check failed".to_string(),
                source: e,
            })?;
        Ok(())
    }

    pub async fn migrate(&self) -> InfraResult<()> {
        info!("Running database migrations");
        sqlx::migrate!("../../migrations")
            .run(&self.pool)
            .await
            .map_err(InfraError::DatabaseMigration)?;
        info!("Database migrations completed");
        Ok(())
    }
}
