use crate::{
    error::{WebError, WebResult},
    state::AppState,
};
use ak_asset_storage_application::{ConfigProvider, ItemDemandRepository};
use axum::{
    debug_handler,
    extract::{Path, State},
    http::header,
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use std::collections::HashMap;

/// `/item/:item_name/demand`
#[debug_handler]
#[utoipa::path(
    get,
    path = "/item/{item_name}/demand",
    tag = "item",
    responses(
        (status = OK, description = "Item demand found", body = String, content_type = "application/json"),
        (status = NOT_FOUND, description = "Item demand not found")
    )
)]
pub async fn get_item_demand(
    State(state): State<AppState>,
    Path(item_name): Path<String>,
) -> WebResult<Response> {
    let usage = state
        .repository
        .query_usage_by_item_name(&item_name)
        .await?
        .ok_or(WebError::NotFound)?;

    Ok(([(header::CONTENT_TYPE, "application/json")], usage).into_response())
}

/// POST /item/demand
#[debug_handler]
#[utoipa::path(
    post,
    path = "/item/demand",
    tag = "item",
    request_body = HashMap<String, serde_json::Value>,
    responses(
        (status = OK, description = "Item demands updated successfully"),
        (status = 401, description = "Unauthorized - invalid authentication token"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn update_item_demands(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    Json(demands): Json<HashMap<String, serde_json::Value>>,
) -> WebResult<impl IntoResponse> {
    // Get the torappu-auth header
    let auth_header = headers
        .get("torappu-auth")
        .ok_or(WebError::Unauthorized(
            "Missing torappu-auth header".to_string(),
        ))?
        .to_str()
        .map_err(|_| WebError::Unauthorized("Invalid torappu-auth header format".to_string()))?;

    // Compare with configured token
    let expected_token = state.config.torappu_config().token.as_str();
    if auth_header != expected_token {
        return Err(WebError::Unauthorized(
            "Invalid authentication token".to_string(),
        ));
    }

    // Convert HashMap to Vec<(String, String)> with JSON serialization
    let demands: Result<Vec<(String, String)>, serde_json::Error> = demands
        .into_iter()
        .map(|(key, value)| serde_json::to_string(&value).map(|serialized| (key, serialized)))
        .collect();

    let demands = demands.map_err(|e| WebError::BadRequest(format!("Invalid JSON value: {e}")))?;

    // Replace all demands in transaction
    state.repository.replace_all_demands(demands).await?;

    Ok(StatusCode::OK)
}
