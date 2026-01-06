use crate::{error::WebError, state::AppState};
use ak_asset_storage_application::{ConfigProvider, DockerService};
use axum::{Json, extract::State, http::HeaderMap};
use serde::{Deserialize, Serialize};

#[allow(clippy::struct_field_names)]
#[derive(Debug, Deserialize, Serialize, utoipa::ToSchema)]
pub struct DockerLaunchRequest {
    pub client_version: String,
    pub res_version: String,
    pub prev_client_version: String,
    pub prev_res_version: String,
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct DockerLaunchResponse {
    pub container_name: String,
    pub status: String,
}

/// Launch Docker container with game asset processing
///
/// This endpoint launches a Docker container to process game assets using the provided version parameters.
/// It uses the same authentication mechanism as `update_item_demands` endpoint.
#[utoipa::path(
    post,
    path = "/docker/launch",
    tag = "docker",
    request_body = DockerLaunchRequest,
    responses(
        (status = 200, description = "Container launched successfully", body = DockerLaunchResponse),
        (status = 401, description = "Unauthorized - invalid or missing authentication token"),
        (status = 400, description = "Bad request - invalid parameters"),
        (status = 500, description = "Internal server error")
    ),
    security(
        ("torappu-auth" = [])
    )
)]
pub async fn launch_container(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<DockerLaunchRequest>,
) -> Result<Json<DockerLaunchResponse>, WebError> {
    // Authentication - same as update_item_demands
    let auth_header = headers
        .get("torappu-auth")
        .ok_or(WebError::Unauthorized(
            "Missing torappu-auth header".to_string(),
        ))?
        .to_str()
        .map_err(|_| WebError::Unauthorized("Invalid torappu-auth header format".to_string()))?;

    let expected_token = state.config.torappu_config().token.as_str();
    if auth_header != expected_token {
        return Err(WebError::Unauthorized(
            "Invalid authentication token".to_string(),
        ));
    }

    // Validate input parameters
    if payload.client_version.is_empty() || payload.res_version.is_empty() {
        return Err(WebError::BadRequest(
            "client_version and res_version cannot be empty".to_string(),
        ));
    }

    if payload.prev_client_version.is_empty() || payload.prev_res_version.is_empty() {
        return Err(WebError::BadRequest(
            "prev_client_version and prev_res_version cannot be empty".to_string(),
        ));
    }

    // Launch container using Docker service
    let docker_service = state
        .docker_service
        .as_ref()
        .ok_or(WebError::ServiceUnavailable(anyhow::anyhow!(
            "Docker service is not configured or available"
        )))?;

    let container_name = docker_service
        .launch_container(
            &payload.client_version,
            &payload.res_version,
            &payload.prev_client_version,
            &payload.prev_res_version,
        )
        .await
        .map_err(WebError::from)?;

    Ok(Json(DockerLaunchResponse {
        container_name,
        status: "launched".to_string(),
    }))
}
