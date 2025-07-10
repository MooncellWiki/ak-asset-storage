use crate::{
    error::{WebError, WebResult},
    state::AppState,
    utils::json,
};
use application::{BundleDetailsDto, BundleFilterDto, BundleRepository};
use axum::{
    debug_handler,
    extract::{Path, State},
    response::Response,
};

/// /bundle/{id}
#[utoipa::path(get, path = "/bundle/{id}", tag="bundle", responses((status = OK, body = BundleDetailsDto)))]
pub async fn get_one(State(state): State<AppState>, Path(id): Path<i32>) -> WebResult<Response> {
    let result = state
        .repository
        .query_bundle_by_id_with_details(id)
        .await?
        .ok_or(WebError::NotFound)?;
    Ok(json(result))
}

/// /bundle
#[debug_handler]
#[utoipa::path(get, path = "/bundle", tag="bundle", params(BundleFilterDto), responses((status = OK, body = [BundleDetailsDto])))]
pub async fn filter(State(state): State<AppState>, query: BundleFilterDto) -> WebResult<Response> {
    let result = state.repository.query_bundles_with_details(query).await?;
    Ok(json(result))
}
