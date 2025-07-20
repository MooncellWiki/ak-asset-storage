use crate::{InfraError, PostgresRepository};
use ak_asset_storage_application::{AppResult, ItemDemandRepository};
use async_trait::async_trait;
use sqlx::query_scalar;

#[async_trait]
impl ItemDemandRepository for PostgresRepository {
    async fn query_usage_by_item_name(&self, item_name: &str) -> AppResult<Option<String>> {
        let usage = query_scalar!(
            r#"
            SELECT usage FROM item_demands WHERE name = $1
            "#,
            item_name
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| InfraError::Database {
            message: "Failed to query item demand".to_string(),
            source: e,
        })?;

        Ok(usage)
    }

    async fn replace_all_demands(&self, demands: Vec<(String, String)>) -> AppResult<()> {
        // Start a transaction
        let mut tx = self.pool.begin().await.map_err(|e| InfraError::Database {
            message: "Failed to start transaction".to_string(),
            source: e,
        })?;

        // Delete all existing demands
        sqlx::query!("DELETE FROM item_demands")
            .execute(&mut *tx)
            .await
            .map_err(|e| InfraError::Database {
                message: "Failed to delete existing item demands".to_string(),
                source: e,
            })?;

        // Insert new demands
        for (name, usage) in demands {
            sqlx::query!(
                "INSERT INTO item_demands (name, usage) VALUES ($1, $2)",
                name,
                usage
            )
            .execute(&mut *tx)
            .await
            .map_err(|e| InfraError::Database {
                message: format!("Failed to insert item demand for {name}"),
                source: e,
            })?;
        }

        // Commit the transaction
        tx.commit().await.map_err(|e| InfraError::Database {
            message: "Failed to commit transaction".to_string(),
            source: e,
        })?;

        Ok(())
    }
}
