use application::{error::AppResult, ports::repositories::VersionRepository};
use application::{VersionDetailDto, VersionDto};
use async_trait::async_trait;
use domain::entities::{Version, VersionId};
use domain::value_objects::{ClientVersion, HotUpdateList, ResVersion};
use sqlx::{query, query_as, Pool, Postgres};

use crate::error::InfraError;

#[derive(Debug, Clone)]
pub struct PostgresVersionRepository {
    pool: Pool<Postgres>,
}

impl PostgresVersionRepository {
    #[must_use]
    pub const fn new(pool: Pool<Postgres>) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl VersionRepository for PostgresVersionRepository {
    async fn create(&self, version: Version) -> AppResult<i32> {
        let row = query!(
            "INSERT INTO versions (res, client, is_ready, hot_update_list) VALUES ($1, $2, $3, $4) RETURNING id",
            version.res.as_str(),
            version.client.as_str(),
            version.is_ready,
            version.hot_update_list.as_str()
        )
        .fetch_one(&self.pool)
        .await
        .map_err(|e| InfraError::Database {
            message: "Failed to create version".to_string(),
            source: e,
        })?;

        Ok(row.id)
    }

    async fn get_by_id(&self, id: i32) -> AppResult<Option<Version>> {
        let result = query!(
            "SELECT id, res, client, is_ready, hot_update_list FROM versions WHERE id = $1",
            id
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| InfraError::Database {
            message: "Failed to get version by id".to_string(),
            source: e,
        })?;

        if let Some(row) = result {
            let version = Version::with_id(
                VersionId(row.id),
                ResVersion::new(&row.res)?,
                ClientVersion::new(&row.client)?,
                row.is_ready,
                HotUpdateList::new(&row.hot_update_list)?,
            );
            Ok(Some(version))
        } else {
            Ok(None)
        }
    }

    async fn get_latest(&self) -> AppResult<Option<Version>> {
        let result = query!(
            "SELECT id, res, client, is_ready, hot_update_list FROM versions ORDER BY id DESC LIMIT 1"
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| InfraError::Database {
            message: "Failed to get latest version".to_string(),
            source: e
        })?;

        if let Some(row) = result {
            let version = Version::with_id(
                VersionId(row.id),
                ResVersion::new(&row.res)?,
                ClientVersion::new(&row.client)?,
                row.is_ready,
                HotUpdateList::new(&row.hot_update_list)?,
            );
            Ok(Some(version))
        } else {
            Ok(None)
        }
    }
    async fn is_client_and_res_exist(&self, client: &str, res: &str) -> AppResult<bool> {
        let result = query!(
            "SELECT id, res, client, is_ready, hot_update_list FROM versions WHERE client = $1 AND res = $2",
            client,
            res
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| InfraError::Database {
            message: "Failed to get version by client and res".to_string(),
            source: e,
        })?;

        Ok(result.is_some())
    }

    async fn get_oldest_unready_version(&self) -> AppResult<Option<Version>> {
        let result = query!(
            "SELECT id, res, client, is_ready, hot_update_list FROM versions WHERE is_ready = false ORDER BY id DESC LIMIT 1"
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| InfraError::Database {
            message: "Failed to get unready version".to_string(),
            source: e,
        })?;

        if let Some(row) = result {
            let version = Version::with_id(
                VersionId(row.id),
                ResVersion::new(&row.res)?,
                ClientVersion::new(&row.client)?,
                row.is_ready,
                HotUpdateList::new(&row.hot_update_list)?,
            );
            Ok(Some(version))
        } else {
            Ok(None)
        }
    }

    async fn mark_ready(&self, id: i32) -> AppResult<()> {
        query!("UPDATE versions SET is_ready = true WHERE id = $1", id)
            .execute(&self.pool)
            .await
            .map_err(|e| InfraError::Database {
                message: "Failed to mark version as ready".to_string(),
                source: e,
            })?;

        Ok(())
    }
    async fn query(&self) -> AppResult<Vec<VersionDto>> {
        let result = query_as!(
            VersionDto,
            r#"SELECT id, client as "client_version", res as "res_version", is_ready FROM versions"#
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| InfraError::Database {
            message: "Failed to query versions".to_string(),
            source: e,
        })?;

        Ok(result)
    }

    async fn query_detail_by_id(&self, id: i32) -> AppResult<Option<VersionDetailDto>> {
        let result = query_as!(
            VersionDetailDto,
            r#"SELECT id, client as "client_version", res as "res_version", is_ready, hot_update_list FROM versions WHERE id = $1"#,
            id
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| InfraError::Database {
            message: "Failed to query version by id".to_string(),
            source: e,
        })?;

        Ok(result)
    }
}
