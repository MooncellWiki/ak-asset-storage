use application::{error::AppResult, ports::repositories::BundleRepository};
use application::{BundleDetailsDto, BundleFilterDto};
use async_trait::async_trait;
use domain::entities::{Bundle, FileId, VersionId};
use domain::value_objects::FilePath;
use sqlx::{query, query_as, Pool, Postgres};

use crate::error::InfraError;

#[derive(Debug, Clone)]
pub struct PostgresBundleRepository {
    pool: Pool<Postgres>,
}

impl PostgresBundleRepository {
    #[must_use]
    pub const fn new(pool: Pool<Postgres>) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl BundleRepository for PostgresBundleRepository {
    async fn create(&self, bundle: Bundle) -> AppResult<i32> {
        let row = query!(
            "INSERT INTO bundles (path, version, file) VALUES ($1, $2, $3) RETURNING id",
            bundle.path.as_str(),
            bundle.version_id.0,
            bundle.file_id.0
        )
        .fetch_one(&self.pool)
        .await
        .map_err(|e| InfraError::Database {
            message: "Failed to create bundle".to_string(),
            source: e,
        })?;

        Ok(row.id)
    }

    async fn get_by_version_and_path(
        &self,
        version_id: i32,
        path: &str,
    ) -> AppResult<Option<Bundle>> {
        let result = query!(
            "SELECT id, path, version, file FROM bundles WHERE version = $1 AND path = $2",
            version_id,
            path
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| InfraError::Database {
            message: "Failed to get bundle by version and path".to_string(),
            source: e,
        })?;

        if let Some(row) = result {
            let bundle = Bundle::with_id(
                domain::entities::BundleId(row.id),
                FilePath::new(&row.path)?,
                VersionId(row.version),
                FileId(row.file),
            );
            Ok(Some(bundle))
        } else {
            Ok(None)
        }
    }
    async fn query_by_id_with_details(&self, id: i32) -> AppResult<Option<BundleDetailsDto>> {
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
    async fn query_with_details(&self, query: BundleFilterDto) -> AppResult<Vec<BundleDetailsDto>> {
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
    ($1::varchar IS NULL OR b.path = $1)
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
    async fn query_by_version_id(&self, version_id: i32) -> AppResult<Vec<BundleDetailsDto>> {
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
