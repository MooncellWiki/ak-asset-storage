use crate::{
    AppError, AppResult,
    database::{
        Database,
        model::{VersionDetails, VersionSummary},
        row::{AssetMappingStatus, VersionRow},
    },
};
use sqlx::{Row, query_as};

fn build_version(
    id: i32,
    res: String,
    client: String,
    is_ready: bool,
    hot_update_list: &str,
    asset_mapping_status: &str,
) -> VersionRow {
    VersionRow {
        id: Some(id),
        res,
        client,
        is_ready,
        asset_mapping_status: AssetMappingStatus::from_str_lossy(asset_mapping_status),
        hot_update_list: hot_update_list.to_string(),
    }
}

impl Database {
    pub async fn create_version(&self, version: VersionRow) -> AppResult<i32> {
        let row = sqlx::query(
            "INSERT INTO versions (res, client, is_ready, hot_update_list, asset_mapping_status) VALUES ($1, $2, $3, $4, $5::asset_mapping_status) RETURNING id",
        )
        .bind(version.res.as_str())
        .bind(version.client.as_str())
        .bind(version.is_ready)
        .bind(version.hot_update_list.as_str())
        .bind(version.asset_mapping_status.as_str())
        .fetch_one(self.pool())
        .await
        .map_err(|err| AppError::ExternalService(err.into()))?;

        Ok(row.get("id"))
    }

    pub async fn get_version_by_id(&self, id: i32) -> AppResult<Option<VersionRow>> {
        let result = sqlx::query!(
            "SELECT id, res, client, is_ready, hot_update_list, asset_mapping_status::text AS \"asset_mapping_status!\" FROM versions WHERE id = $1",
            id
        )
        .fetch_optional(self.pool())
        .await
        .map_err(|err| AppError::ExternalService(err.into()))?;

        Ok(result.map(|row| {
            build_version(
                row.id,
                row.res,
                row.client,
                row.is_ready,
                &row.hot_update_list,
                &row.asset_mapping_status,
            )
        }))
    }

    pub async fn get_version_by_res(&self, res: &str) -> AppResult<Option<VersionRow>> {
        let result = sqlx::query!(
            "SELECT id, res, client, is_ready, hot_update_list, asset_mapping_status::text AS \"asset_mapping_status!\" FROM versions WHERE res = $1",
            res
        )
        .fetch_optional(self.pool())
        .await
        .map_err(|err| AppError::ExternalService(err.into()))?;

        Ok(result.map(|row| {
            build_version(
                row.id,
                row.res,
                row.client,
                row.is_ready,
                &row.hot_update_list,
                &row.asset_mapping_status,
            )
        }))
    }

    pub async fn get_latest_version(&self) -> AppResult<Option<VersionRow>> {
        let result = sqlx::query!(
            "SELECT id, res, client, is_ready, hot_update_list, asset_mapping_status::text AS \"asset_mapping_status!\" FROM versions ORDER BY id DESC LIMIT 1"
        )
        .fetch_optional(self.pool())
        .await
        .map_err(|err| AppError::ExternalService(err.into()))?;

        Ok(result.map(|row| {
            build_version(
                row.id,
                row.res,
                row.client,
                row.is_ready,
                &row.hot_update_list,
                &row.asset_mapping_status,
            )
        }))
    }

    pub async fn is_client_and_res_exist(&self, client: &str, res: &str) -> AppResult<bool> {
        let result = sqlx::query!(
            "SELECT id FROM versions WHERE client = $1 AND res = $2",
            client,
            res
        )
        .fetch_optional(self.pool())
        .await
        .map_err(|err| AppError::ExternalService(err.into()))?;

        Ok(result.is_some())
    }

    pub async fn get_oldest_unready_version(&self) -> AppResult<Option<VersionRow>> {
        let result = sqlx::query!(
            "SELECT id, res, client, is_ready, hot_update_list, asset_mapping_status::text AS \"asset_mapping_status!\" FROM versions WHERE is_ready = false ORDER BY id ASC LIMIT 1"
        )
        .fetch_optional(self.pool())
        .await
        .map_err(|err| AppError::ExternalService(err.into()))?;

        Ok(result.map(|row| {
            build_version(
                row.id,
                row.res,
                row.client,
                row.is_ready,
                &row.hot_update_list,
                &row.asset_mapping_status,
            )
        }))
    }

    pub async fn mark_version_ready(&self, id: i32) -> AppResult<()> {
        sqlx::query!("UPDATE versions SET is_ready = true WHERE id = $1", id)
            .execute(self.pool())
            .await
            .map_err(|err| AppError::ExternalService(err.into()))?;
        Ok(())
    }

    pub async fn set_asset_mapping_status(
        &self,
        id: i32,
        status: AssetMappingStatus,
    ) -> AppResult<()> {
        sqlx::query(
            "UPDATE versions SET asset_mapping_status = $2::asset_mapping_status WHERE id = $1",
        )
        .bind(id)
        .bind(status.as_str())
        .execute(self.pool())
        .await
        .map_err(|err| AppError::ExternalService(err.into()))?;
        Ok(())
    }

    pub async fn query_versions(&self) -> AppResult<Vec<VersionSummary>> {
        query_as!(
            VersionSummary,
            r#"SELECT id, client as "client_version", res as "res_version", is_ready, asset_mapping_status::text AS "asset_mapping_status!" FROM versions ORDER BY id ASC"#
        )
        .fetch_all(self.pool())
        .await
        .map_err(|err| AppError::ExternalService(err.into()))
    }

    pub async fn query_version_detail_by_id(&self, id: i32) -> AppResult<Option<VersionDetails>> {
        query_as!(
            VersionDetails,
            r#"SELECT id, client as "client_version", res as "res_version", is_ready, hot_update_list FROM versions WHERE id = $1"#,
            id
        )
        .fetch_optional(self.pool())
        .await
        .map_err(|err| AppError::ExternalService(err.into()))
    }
}
