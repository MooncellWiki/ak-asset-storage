use crate::{
    error::{WebError, WebResult},
    state::AppState,
    utils::json,
};
use ak_asset_storage_application::{
    AssetMappingDetailDto, AssetMappingRepository, ManifestChildrenParams, ManifestDetailParams,
    ManifestNodeDto, ManifestSearchParams,
};
use axum::{
    debug_handler,
    extract::{Path, Query, State},
    response::Response,
};

#[allow(clippy::doc_markdown)]
/// /manifest/{version_id}/children
#[debug_handler]
#[utoipa::path(get, path = "/manifest/{version_id}/children", tag = "manifest", params(ManifestChildrenParams), responses((status = OK, body = [ManifestNodeDto])))]
pub async fn list_children(
    State(state): State<AppState>,
    Path(version_id): Path<i32>,
    Query(params): Query<ManifestChildrenParams>,
) -> WebResult<Response> {
    let dir = params.dir.unwrap_or_default();
    let result = state
        .repository
        .list_manifest_children(version_id, &dir)
        .await?;
    Ok(json(result))
}

#[allow(clippy::doc_markdown)]
/// /manifest/{version_id}/detail
#[debug_handler]
#[utoipa::path(get, path = "/manifest/{version_id}/detail", tag = "manifest", params(ManifestDetailParams), responses((status = OK, body = AssetMappingDetailDto)))]
pub async fn get_detail(
    State(state): State<AppState>,
    Path(version_id): Path<i32>,
    Query(params): Query<ManifestDetailParams>,
) -> WebResult<Response> {
    let result = state
        .repository
        .get_asset_mapping_detail(version_id, &params.asset_name)
        .await?
        .ok_or(WebError::NotFound)?;
    Ok(json(result))
}

#[allow(clippy::doc_markdown)]
/// /manifest/{version_id}/search
#[debug_handler]
#[utoipa::path(get, path = "/manifest/{version_id}/search", tag = "manifest", params(ManifestSearchParams), responses((status = OK, body = [ManifestNodeDto])))]
pub async fn search(
    State(state): State<AppState>,
    Path(version_id): Path<i32>,
    Query(params): Query<ManifestSearchParams>,
) -> WebResult<Response> {
    let result = state
        .repository
        .search_manifest(version_id, &params.q)
        .await?;
    Ok(json(result))
}
