use crate::InfraError;
use ak_asset_storage_application::{AppResult, Repository};
use async_trait::async_trait;
use sqlx::{Pool, Postgres};
use tracing::info;

#[derive(Debug, Clone)]
pub struct PostgresRepository {
    pub pool: Pool<Postgres>,
}

#[async_trait]
impl Repository for PostgresRepository {
    async fn health_check(&self) -> bool {
        sqlx::query("SELECT 1").execute(&self.pool).await.is_ok()
    }

    async fn migrate(&self) -> AppResult<()> {
        info!("Running database migrations");
        sqlx::migrate!("../../migrations")
            .run(&self.pool)
            .await
            .map_err(InfraError::DatabaseMigration)?;
        info!("Database migrations completed");
        Ok(())
    }
}
