use crate::{InfraError, PostgresRepository};
use ak_asset_storage_application::{
    AppResult, AssetMapping, AssetMappingRepository, AssetMappingStatus,
};
use async_trait::async_trait;
use sqlx::Acquire;

const INSERT_BATCH_SIZE: usize = 5000;

#[async_trait]
impl AssetMappingRepository for PostgresRepository {
    async fn import_asset_mappings(
        &self,
        version_id: i32,
        mappings: &[AssetMapping],
    ) -> AppResult<bool> {
        let mut conn = self.pool.acquire().await.map_err(|e| InfraError::Database {
            message: "Failed to acquire connection for asset mapping import".to_string(),
            source: e,
        })?;

        let locked = sqlx::query_scalar!("SELECT pg_try_advisory_lock($1)", i64::from(version_id))
            .fetch_one(&mut *conn)
            .await
            .map_err(|e| InfraError::Database {
                message: "Failed to acquire asset mapping lock".to_string(),
                source: e,
            })?;

        let locked = locked.unwrap_or(false);
        if !locked {
            return Ok(false);
        }

        let import_result: AppResult<()> = async {
            sqlx::query!(
                "UPDATE versions SET asset_mapping_status = $2 WHERE id = $1",
                version_id,
                AssetMappingStatus::Importing.as_str()
            )
            .execute(&mut *conn)
            .await
            .map_err(|e| InfraError::Database {
                message: "Failed to update asset mapping status".to_string(),
                source: e,
            })?;

            let mut tx = conn.begin().await.map_err(|e| InfraError::Database {
                message: "Failed to start asset mapping transaction".to_string(),
                source: e,
            })?;

            sqlx::query!(
                "DELETE FROM asset_to_bundle_mappings WHERE version_id = $1",
                version_id
            )
            .execute(&mut *tx)
            .await
            .map_err(|e| InfraError::Database {
                message: "Failed to delete old asset mappings".to_string(),
                source: e,
            })?;

            for batch in mappings.chunks(INSERT_BATCH_SIZE) {
                for mapping in batch {
                    sqlx::query!(
                        r#"
INSERT INTO asset_to_bundle_mappings
    (version_id, asset_name, bundle_path, asset_path, short_name)
VALUES
    ($1, $2, $3, $4, $5)
                        "#,
                        mapping.version_id,
                        &mapping.asset_name,
                        &mapping.bundle_path,
                        mapping.asset_path.as_deref().unwrap_or_default(),
                        mapping.short_name.as_deref().unwrap_or_default()
                    )
                    .execute(&mut *tx)
                    .await
                    .map_err(|e| InfraError::Database {
                        message: "Failed to insert asset mapping".to_string(),
                        source: e,
                    })?;
                }
            }

            tx.commit().await.map_err(|e| InfraError::Database {
                message: "Failed to commit asset mapping transaction".to_string(),
                source: e,
            })?;

            sqlx::query!(
                "UPDATE versions SET asset_mapping_status = $2 WHERE id = $1",
                version_id,
                AssetMappingStatus::Ready.as_str()
            )
            .execute(&mut *conn)
            .await
            .map_err(|e| InfraError::Database {
                message: "Failed to update asset mapping status".to_string(),
                source: e,
            })?;

            Ok(())
        }
        .await;

        if import_result.is_err() {
            sqlx::query!(
                "UPDATE versions SET asset_mapping_status = $2 WHERE id = $1",
                version_id,
                AssetMappingStatus::Pending.as_str()
            )
            .execute(&mut *conn)
            .await
            .map_err(|e| InfraError::Database {
                message: "Failed to reset asset mapping status".to_string(),
                source: e,
            })?;
        }

        sqlx::query!("SELECT pg_advisory_unlock($1)", i64::from(version_id))
            .fetch_one(&mut *conn)
            .await
            .map_err(|e| InfraError::Database {
                message: "Failed to release asset mapping lock".to_string(),
                source: e,
            })?;

        import_result?;
        Ok(true)
    }
}
