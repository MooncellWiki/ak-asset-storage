use crate::{InfraError, PostgresRepository};
use ak_asset_storage_application::{
    AppResult, Bundle, BundleDetailsDto, BundleFilterDto, BundleRepository,
};
use async_trait::async_trait;
use sqlx::{query, query_as};

#[async_trait]
impl BundleRepository for PostgresRepository {
    async fn create_bundle(&self, bundle: Bundle) -> AppResult<i32> {
        let row = query!(
            "INSERT INTO bundles (path, version, file) VALUES ($1, $2, $3) RETURNING id",
            bundle.path.as_str(),
            bundle.version_id,
            bundle.file_id
        )
        .fetch_one(&self.pool)
        .await
        .map_err(|e| InfraError::Database {
            message: "Failed to create bundle".to_string(),
            source: e,
        })?;

        Ok(row.id)
    }

    async fn get_bundle_by_version_and_path(
        &self,
        version_id: i32,
        path: &str,
    ) -> AppResult<Option<Bundle>> {
        let result = query_as!(
            Bundle,
            r#"SELECT id, path, version as "version_id", file as "file_id" FROM bundles WHERE version = $1 AND path = $2"#,
            version_id,
            path
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| InfraError::Database {
            message: "Failed to get bundle by version and path".to_string(),
            source: e,
        })?;
        Ok(result)
    }
    async fn query_bundle_by_id_with_details(
        &self,
        id: i32,
    ) -> AppResult<Option<BundleDetailsDto>> {
        let result = query_as!(
            BundleDetailsDto,
            r#"
SELECT
    b.id as "id!",
    b.path as "path!",
    b.file as "file_id!",
    b.version as "version_id!",
    f.hash as "file_hash",
    f.size as "file_size",
    v.client as "version_client",
    v.res as "version_res",
    v.is_ready as "version_is_ready!"
FROM
    bundles b
INNER JOIN
    files f ON b.file = f.id
INNER JOIN
    versions v ON b.version = v.id
WHERE
    b.id = $1
            "#,
            id
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| InfraError::Database {
            message: "Failed to get bundle by id with details".to_string(),
            source: e,
        })?;

        Ok(result)
    }
    async fn query_bundles_with_details(
        &self,
        query: BundleFilterDto,
    ) -> AppResult<Vec<BundleDetailsDto>> {
        let result = query_as!(
            BundleDetailsDto,
            r#"
SELECT
    b.id as "id!",
    b.path as "path!",
    b.file as "file_id!",
    b.version as "version_id!",
    f.hash as "file_hash",
    f.size as "file_size",
    v.client as "version_client",
    v.res as "version_res",
    v.is_ready as "version_is_ready!"
FROM
    bundles b
INNER JOIN
    files f ON b.file = f.id
INNER JOIN
    versions v ON b.version = v.id
WHERE
    ($1::varchar IS NULL OR b.path LIKE CONCAT('%', $1, '%'))
    AND ($2::varchar IS NULL OR f.hash = $2)
    AND ($3::int IS NULL OR b.file = $3)
    AND ($4::int IS NULL OR b.version = $4)
            "#,
            query.path,
            query.hash,
            query.file,
            query.version
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| InfraError::Database {
            message: "Failed to list bundles with details".to_string(),
            source: e,
        })?;

        Ok(result)
    }
    async fn query_bundles_by_version_id(
        &self,
        version_id: i32,
    ) -> AppResult<Vec<BundleDetailsDto>> {
        let result = query_as!(
            BundleDetailsDto,
            r#"
        SELECT
            b.id as "id!",
            b.path as "path!",
            b.file as "file_id!",
            b.version as "version_id!",
            f.hash as "file_hash",
            f.size as "file_size",
            v.client as "version_client",
            v.res as "version_res",
            v.is_ready as "version_is_ready!"
        FROM
            bundles b
        INNER JOIN
            files f ON b.file = f.id
        INNER JOIN
            versions v ON b.version = v.id
        WHERE
            b.version = $1
            "#,
            version_id
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| InfraError::Database {
            message: "Failed to query bundles by version id".to_string(),
            source: e,
        })?;
        Ok(result)
    }
}
