use crate::{
    dto::responses::{BundleDetail, VersionDetail, VersionListItem},
    error::{WebError, WebResult},
    json,
};
use application::{BundleRepository, VersionRepository};
use axum::{
    debug_handler,
    extract::{Path, State},
    response::Response,
};
use infrastructure::{PostgresBundleRepository, PostgresVersionRepository};
/// /version
#[debug_handler]
#[utoipa::path(get, path = "/version", tag = "version", responses((status = OK, body = [VersionListItem])))]
pub async fn list(State(version): State<PostgresVersionRepository>) -> WebResult<Response> {
    let result = version.query().await?;
    Ok(json(result))
}

/// /version/{id}
#[debug_handler]
#[utoipa::path(get, path = "/version/{id}", tag = "version", responses((status = OK, body = VersionDetail)))]
pub async fn get(
    State(version): State<PostgresVersionRepository>,
    Path(id): Path<i32>,
) -> WebResult<Response> {
    let result = version
        .query_detail_by_id(id)
        .await?
        .ok_or(WebError::NotFound)?;
    Ok(json(result))
}

/// /version/{id}/files
#[debug_handler]
#[utoipa::path(get, path = "/version/{id}/files", tag = "version", responses((status = OK, body = [BundleDetail])))]
pub async fn get_files(
    State(bundle): State<PostgresBundleRepository>,
    Path(id): Path<i32>,
) -> WebResult<Response> {
    let result = bundle.query_by_version_id(id).await?;
    Ok(json(result))
}
