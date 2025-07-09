use crate::{
    error::{WebError, WebResult},
    json,
};
use application::{BundleDetailsDto, BundleFilterDto, BundleRepository};
use axum::{
    debug_handler,
    extract::{Path, State},
    response::Response,
};
use infrastructure::PostgresBundleRepository;

/// /bundle/{id}
#[utoipa::path(get, path = "/bundle/{id}", tag="bundle", responses((status = OK, body = BundleDetailsDto)))]
pub async fn get_one(
    State(bundle): State<PostgresBundleRepository>,
    Path(id): Path<i32>,
) -> WebResult<Response> {
    let result = bundle
        .query_by_id_with_details(id)
        .await?
        .ok_or(WebError::NotFound)?;
    Ok(json(result))
}

/// /bundle
#[debug_handler]
#[utoipa::path(get, path = "/bundle", tag="bundle", params(BundleFilterDto), responses((status = OK, body = [BundleDetailsDto])))]
pub async fn filter(
    State(bundle): State<PostgresBundleRepository>,
    query: BundleFilterDto,
) -> WebResult<Response> {
    let result = bundle.query_with_details(query).await?;
    Ok(json(result))
}
