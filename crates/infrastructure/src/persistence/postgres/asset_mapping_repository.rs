use crate::{InfraError, PostgresRepository};
use ak_asset_storage_application::{
    AppResult, AssetMapping, AssetMappingDetailDto, AssetMappingRepository, AssetMappingStatus,
    ManifestNodeDto,
};
use async_trait::async_trait;
use sqlx::{Acquire, Postgres, pool::PoolConnection, query_as};

impl PostgresRepository {
    async fn import_asset_mappings_with_lock(
        conn: &mut PoolConnection<Postgres>,
        version_id: i32,
        mappings: &[AssetMapping],
    ) -> AppResult<()> {
        sqlx::query!(
            "UPDATE versions SET asset_mapping_status = $2 WHERE id = $1",
            version_id,
            AssetMappingStatus::Importing.as_str()
        )
        .execute(&mut **conn)
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

        for mapping in mappings {
            sqlx::query!(
                r#"
INSERT INTO asset_to_bundle_mappings
    (version_id, asset_name, bundle_path, asset_path, short_name, dir_name, node_type)
VALUES
    ($1, $2, $3, $4, $5, $6, $7::node_type)
                "#,
                mapping.version_id,
                &mapping.asset_name,
                &mapping.bundle_path,
                mapping.asset_path.as_deref(),
                mapping.short_name.as_deref(),
                &mapping.dir_name,
                mapping.node_type.as_str() as &str
            )
            .execute(&mut *tx)
            .await
            .map_err(|e| InfraError::Database {
                message: "Failed to insert asset mapping".to_string(),
                source: e,
            })?;
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
        .execute(&mut **conn)
        .await
        .map_err(|e| InfraError::Database {
            message: "Failed to update asset mapping status".to_string(),
            source: e,
        })?;

        Ok(())
    }
}

#[allow(clippy::too_many_lines)]
#[async_trait]
impl AssetMappingRepository for PostgresRepository {
    async fn import_asset_mappings(
        &self,
        version_id: i32,
        mappings: &[AssetMapping],
    ) -> AppResult<bool> {
        let mut conn = self
            .pool
            .acquire()
            .await
            .map_err(|e| InfraError::Database {
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

        let import_result =
            Self::import_asset_mappings_with_lock(&mut conn, version_id, mappings).await;

        let reset_result = if import_result.is_err() {
            Some(
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
                }),
            )
        } else {
            None
        };

        sqlx::query!("SELECT pg_advisory_unlock($1)", i64::from(version_id))
            .fetch_one(&mut *conn)
            .await
            .map_err(|e| InfraError::Database {
                message: "Failed to release asset mapping lock".to_string(),
                source: e,
            })?;

        if let Some(reset_result) = reset_result {
            reset_result?;
        }

        import_result?;
        Ok(true)
    }

    async fn list_manifest_children(
        &self,
        version_id: i32,
        dir_name: &str,
    ) -> AppResult<Vec<ManifestNodeDto>> {
        let result = query_as!(
            ManifestNodeDto,
            r#"
SELECT name::text as "name!", path::text as "path!", node_type::text as "node_type!"
FROM (
    SELECT
        split_part(asset_name, '/', array_length(string_to_array(asset_name, '/'), 1)) as name,
        asset_name as path,
        node_type
    FROM asset_to_bundle_mappings
    WHERE version_id = $1 AND dir_name = $2
) sub
ORDER BY (node_type IN ('directory', 'both')) DESC, name ASC
            "#,
            version_id,
            dir_name
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| InfraError::Database {
            message: "Failed to list manifest children".to_string(),
            source: e,
        })?;
        Ok(result)
    }

    async fn get_asset_mapping_detail(
        &self,
        version_id: i32,
        asset_name: &str,
    ) -> AppResult<Option<AssetMappingDetailDto>> {
        let result = query_as!(
            AssetMappingDetailDto,
            r#"
SELECT
    m.asset_name AS "asset_name!",
    m.bundle_path AS "bundle_path!",
    m.asset_path,
    m.short_name,
    f.size AS "bundle_size",
    f.hash AS "bundle_hash"
FROM asset_to_bundle_mappings m
LEFT JOIN bundles b ON m.bundle_path = b.path AND m.version_id = b.version
LEFT JOIN files f ON b.file = f.id
WHERE m.version_id = $1 AND m.asset_name = $2 AND m.node_type IN ('file', 'both')
            "#,
            version_id,
            asset_name
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| InfraError::Database {
            message: "Failed to get asset mapping detail".to_string(),
            source: e,
        })?;
        Ok(result)
    }

    async fn search_manifest(
        &self,
        version_id: i32,
        query: &str,
    ) -> AppResult<Vec<ManifestNodeDto>> {
        let pattern = format!("%{query}%");
        let result = query_as!(
            ManifestNodeDto,
            r#"
SELECT
    split_part(asset_name, '/', array_length(string_to_array(asset_name, '/'), 1)) AS "name!",
    asset_name AS "path!",
    node_type::text AS "node_type!"
FROM asset_to_bundle_mappings
WHERE version_id = $1 AND node_type IN ('file', 'both') AND asset_name ILIKE $2
ORDER BY asset_name
LIMIT 200
            "#,
            version_id,
            pattern
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| InfraError::Database {
            message: "Failed to search manifest".to_string(),
            source: e,
        })?;
        Ok(result)
    }
}
