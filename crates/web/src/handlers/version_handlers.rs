use crate::{
    error::{WebError, WebResult},
    state::AppState,
    utils::json,
};
use ak_asset_storage_application::{
    BundleDetailsDto, BundleRepository, VersionDetailDto, VersionDto, VersionRepository,
};
use axum::{
    debug_handler,
    extract::{Path, State},
    response::Response,
};

/// /version
#[debug_handler]
#[utoipa::path(get, path = "/version", tag = "version", responses((status = OK, body = [VersionDto])))]
pub async fn list(State(state): State<AppState>) -> WebResult<Response> {
    let result = state.repository.query_versions().await?;
    Ok(json(result))
}

/// /version/{id}
#[debug_handler]
#[utoipa::path(get, path = "/version/{id}", tag = "version", responses((status = OK, body = VersionDetailDto)))]
pub async fn get(State(state): State<AppState>, Path(id): Path<i32>) -> WebResult<Response> {
    let result = state
        .repository
        .query_version_detail_by_id(id)
        .await?
        .ok_or(WebError::NotFound)?;
    Ok(json(result))
}

/// /version/{id}/files
#[debug_handler]
#[utoipa::path(get, path = "/version/{id}/files", tag = "version", responses((status = OK, body = [BundleDetailsDto])))]
pub async fn get_files(State(state): State<AppState>, Path(id): Path<i32>) -> WebResult<Response> {
    let result = state.repository.query_bundles_by_version_id(id).await?;
    Ok(json(result))
}
