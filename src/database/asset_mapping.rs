use crate::{
    AppError, AppResult,
    database::{
        Database,
        model::{AssetMappingDetails, ManifestNode},
        row::{AssetMappingRow, AssetMappingStatus, NodeType},
    },
};
use sqlx::{Acquire, Postgres, pool::PoolConnection, query_as};

impl Database {
    async fn import_asset_mappings_with_lock(
        conn: &mut PoolConnection<Postgres>,
        version_id: i32,
        mappings: &[AssetMappingRow],
    ) -> AppResult<()> {
        let mut tx = conn
            .begin()
            .await
            .map_err(|err| AppError::ExternalService(err.into()))?;

        sqlx::query!(
            "UPDATE versions SET asset_mapping_status = $2::asset_mapping_status WHERE id = $1",
            version_id,
            AssetMappingStatus::Importing as AssetMappingStatus
        )
        .execute(&mut *tx)
        .await
        .map_err(|err| AppError::ExternalService(err.into()))?;

        sqlx::query!(
            "DELETE FROM asset_to_bundle_mappings WHERE version_id = $1",
            version_id
        )
        .execute(&mut *tx)
        .await
        .map_err(|err| AppError::ExternalService(err.into()))?;

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
                mapping.node_type as NodeType
            )
            .execute(&mut *tx)
            .await
            .map_err(|err| AppError::ExternalService(err.into()))?;
        }

        sqlx::query!(
            "UPDATE versions SET asset_mapping_status = $2::asset_mapping_status WHERE id = $1",
            version_id,
            AssetMappingStatus::Ready as AssetMappingStatus
        )
        .execute(&mut *tx)
        .await
        .map_err(|err| AppError::ExternalService(err.into()))?;

        tx.commit()
            .await
            .map_err(|err| AppError::ExternalService(err.into()))?;
        Ok(())
    }

    pub async fn import_asset_mappings(
        &self,
        version_id: i32,
        mappings: &[AssetMappingRow],
    ) -> AppResult<bool> {
        let mut conn = self
            .pool()
            .acquire()
            .await
            .map_err(|err| AppError::ExternalService(err.into()))?;

        let locked = sqlx::query_scalar!("SELECT pg_try_advisory_lock($1)", i64::from(version_id))
            .fetch_one(&mut *conn)
            .await
            .map_err(|err| AppError::ExternalService(err.into()))?
            .unwrap_or(false);

        if !locked {
            return Ok(false);
        }

        let result = Self::import_asset_mappings_with_lock(&mut conn, version_id, mappings).await;

        sqlx::query!("SELECT pg_advisory_unlock($1)", i64::from(version_id))
            .fetch_one(&mut *conn)
            .await
            .map_err(|err| AppError::ExternalService(err.into()))?;

        result?;
        Ok(true)
    }

    pub async fn list_manifest_children(
        &self,
        version_id: i32,
        dir_name: &str,
    ) -> AppResult<Vec<ManifestNode>> {
        query_as!(
            ManifestNode,
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
        .fetch_all(self.pool())
        .await
        .map_err(|err| AppError::ExternalService(err.into()))
    }

    pub async fn get_asset_mapping_detail(
        &self,
        version_id: i32,
        asset_name: &str,
    ) -> AppResult<Option<AssetMappingDetails>> {
        query_as!(
            AssetMappingDetails,
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
        .fetch_optional(self.pool())
        .await
        .map_err(|err| AppError::ExternalService(err.into()))
    }

    pub async fn search_manifest(
        &self,
        version_id: i32,
        query: &str,
    ) -> AppResult<Vec<ManifestNode>> {
        let pattern = format!("%{query}%");
        query_as!(
            ManifestNode,
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
        .fetch_all(self.pool())
        .await
        .map_err(|err| AppError::ExternalService(err.into()))
    }
}
