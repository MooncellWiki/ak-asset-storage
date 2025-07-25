use crate::{
    dto::request::TorappuSearchAssetsByPathReq, error::WebResult, state::AppState, utils::json,
};
use ak_asset_storage_application::{AssetDirInfo, AssetEntry, TorappuAssetService};
use axum::{
    debug_handler,
    extract::{Path, Query, State},
    response::Response,
};

/// /asset
#[debug_handler]
#[utoipa::path(
    get,
    path = "/asset",
    tag = "asset",
    params(
        ("path" = String, Query, description = "Search path pattern")
    ),
    responses(
        (status = 200, description = "List of matching entries", body = Vec<AssetEntry>),
        (status = 400, description = "Bad request"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn search_assets_by_path(
    State(state): State<AppState>,
    Query(TorappuSearchAssetsByPathReq { path }): Query<TorappuSearchAssetsByPathReq>,
) -> WebResult<Response> {
    let result = state.torappu.search_assets_by_path(&path).await?;
    Ok(json(result))
}

/// /asset/{path}
#[utoipa::path(
    get,
    path = "/asset/{path}",
    tag = "asset",
    params(
        ("path" = String, Path, description = "Directory path to list")
    ),
    responses(
        (status = 200, description = "Directory listing", body = AssetDirInfo),
        (status = 404, description = "Directory not found"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn list_asset(
    State(state): State<AppState>,
    Path(p): Path<String>,
) -> WebResult<Response> {
    let result = state.torappu.list_asset(&p).await?;
    Ok(json(result))
}

/// /asset/
/// /asset/{path}不匹配/asset/ 用这个方法来处理
pub async fn list_root_asset(state: State<AppState>) -> WebResult<Response> {
    let result = state.torappu.list_asset("").await?;
    Ok(json(result))
}
