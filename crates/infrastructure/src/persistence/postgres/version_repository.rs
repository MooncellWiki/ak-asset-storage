use crate::{InfraError, PostgresRepository};
use ak_asset_storage_application::{
    AppResult, AssetMappingStatus, Version, VersionDetailDto, VersionDto, VersionRepository,
};
use async_trait::async_trait;
use sqlx::query_as;

fn build_version(
    id: i32,
    res: String,
    client: String,
    is_ready: bool,
    hot_update_list: String,
    asset_mapping_status: String,
) -> AppResult<Version> {
    Ok(Version {
        asset_mapping_status: AssetMappingStatus::from_str_lossy(&asset_mapping_status),
        ..Version::with_id(id, res, client, is_ready, &hot_update_list)?
    })
}

#[async_trait]
impl VersionRepository for PostgresRepository {
    async fn create_version(&self, version: Version) -> AppResult<i32> {
        let row = sqlx::query!(
            "INSERT INTO versions (res, client, is_ready, hot_update_list, asset_mapping_status) VALUES ($1, $2, $3, $4, $5) RETURNING id",
            version.res.as_str(),
            version.client.as_str(),
            version.is_ready,
            version.hot_update_list.as_str(),
            version.asset_mapping_status.as_str()
        )
        .fetch_one(&self.pool)
        .await
        .map_err(|e| InfraError::Database {
            message: "Failed to create version".to_string(),
            source: e,
        })?;

        Ok(row.id)
    }

    async fn get_version_by_id(&self, id: i32) -> AppResult<Option<Version>> {
        let result = sqlx::query!(
            "SELECT id, res, client, is_ready, hot_update_list, asset_mapping_status FROM versions WHERE id = $1",
            id
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| InfraError::Database {
            message: "Failed to get version by id".to_string(),
            source: e,
        })?;
        result.map(|r| build_version(r.id, r.res, r.client, r.is_ready, r.hot_update_list, r.asset_mapping_status)).transpose()
    }

    async fn get_version_by_res(&self, res: &str) -> AppResult<Option<Version>> {
        let result = sqlx::query!(
            "SELECT id, res, client, is_ready, hot_update_list, asset_mapping_status FROM versions WHERE res = $1",
            res
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| InfraError::Database {
            message: "Failed to get version by res".to_string(),
            source: e,
        })?;

        result.map(|r| build_version(r.id, r.res, r.client, r.is_ready, r.hot_update_list, r.asset_mapping_status)).transpose()
    }

    async fn get_latest_version(&self) -> AppResult<Option<Version>> {
        let result = sqlx::query!(
            "SELECT id, res, client, is_ready, hot_update_list, asset_mapping_status FROM versions ORDER BY id DESC LIMIT 1"
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| InfraError::Database {
            message: "Failed to get latest version".to_string(),
            source: e
        })?;

        result.map(|r| build_version(r.id, r.res, r.client, r.is_ready, r.hot_update_list, r.asset_mapping_status)).transpose()
    }
    async fn is_client_and_res_exist(&self, client: &str, res: &str) -> AppResult<bool> {
        let result = sqlx::query!(
            "SELECT id FROM versions WHERE client = $1 AND res = $2",
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
        let result = sqlx::query!(
            "SELECT id, res, client, is_ready, hot_update_list, asset_mapping_status FROM versions WHERE is_ready = false ORDER BY id ASC LIMIT 1"
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| InfraError::Database {
            message: "Failed to get unready version".to_string(),
            source: e,
        })?;

        result.map(|r| build_version(r.id, r.res, r.client, r.is_ready, r.hot_update_list, r.asset_mapping_status)).transpose()
    }

    async fn mark_version_ready(&self, id: i32) -> AppResult<()> {
        sqlx::query!("UPDATE versions SET is_ready = true WHERE id = $1", id)
            .execute(&self.pool)
            .await
            .map_err(|e| InfraError::Database {
                message: "Failed to mark version as ready".to_string(),
                source: e,
            })?;

        Ok(())
    }

    async fn set_asset_mapping_status(&self, id: i32, status: AssetMappingStatus) -> AppResult<()> {
        sqlx::query!(
            "UPDATE versions SET asset_mapping_status = $2 WHERE id = $1",
            id,
            status.as_str()
        )
        .execute(&self.pool)
        .await
        .map_err(|e| InfraError::Database {
            message: "Failed to update asset mapping status".to_string(),
            source: e,
        })?;

        Ok(())
    }
    async fn query_versions(&self) -> AppResult<Vec<VersionDto>> {
        let result = query_as!(
            VersionDto,
            r#"SELECT id, client as "client_version", res as "res_version", is_ready FROM versions ORDER BY id ASC"#
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| InfraError::Database {
            message: "Failed to query versions".to_string(),
            source: e,
        })?;

        Ok(result)
    }

    async fn query_version_detail_by_id(&self, id: i32) -> AppResult<Option<VersionDetailDto>> {
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
