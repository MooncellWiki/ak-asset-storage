use crate::{
    api::{
        error::{WebError, WebResult},
        state::AppState,
        types::{
            AssetSearchQuery, BundleListQuery, DockerLaunchRequest, DockerLaunchResponse, Health,
            ManifestChildrenQuery, ManifestDetailQuery, ManifestSearchQuery,
        },
        utils::json,
    },
    database::model::{
        AssetMappingDetails, BundleDetails, ManifestNode, VersionDetails, VersionSummary,
    },
};
use axum::{
    Json, debug_handler,
    extract::{Path, Query, State},
    http::{StatusCode, header},
    response::{IntoResponse, Response},
};
use rust_embed::Embed;
#[debug_handler]
#[utoipa::path(get, path = "/_ping", responses((status = OK, body = Health)))]
pub async fn ping() -> Json<Health> {
    Json(Health { ok: true })
}

#[debug_handler]
#[utoipa::path(get, path = "/_health", responses((status = OK, body = Health)))]
pub async fn health(State(state): State<AppState>) -> Json<Health> {
    Json(Health {
        ok: state.database.health_check().await,
    })
}

#[debug_handler]
#[utoipa::path(get, path = "/version", tag = "version", responses((status = OK, body = [VersionSummary])))]
pub async fn list_version(State(state): State<AppState>) -> WebResult<Response> {
    Ok(json(state.database.query_versions().await?))
}

#[debug_handler]
#[utoipa::path(get, path = "/version/{id}", tag = "version", responses((status = OK, body = VersionDetails)))]
pub async fn get_version(
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> WebResult<Response> {
    let result = state
        .database
        .query_version_detail_by_id(id)
        .await?
        .ok_or(WebError::NotFound)?;
    Ok(json(result))
}

#[debug_handler]
#[utoipa::path(get, path = "/version/{id}/files", tag = "version", responses((status = OK, body = [BundleDetails])))]
pub async fn get_files_by_version(
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> WebResult<Response> {
    Ok(json(state.database.query_bundles_by_version_id(id).await?))
}

#[utoipa::path(get, path = "/bundle/{id}", tag="bundle", responses((status = OK, body = BundleDetails)))]
pub async fn get_bundle(State(state): State<AppState>, Path(id): Path<i32>) -> WebResult<Response> {
    let result = state
        .database
        .query_bundle_by_id_with_details(id)
        .await?
        .ok_or(WebError::NotFound)?;
    Ok(json(result))
}

#[debug_handler]
#[utoipa::path(get, path = "/bundle", tag="bundle", params(BundleListQuery), responses((status = OK, body = [BundleDetails])))]
pub async fn filter_bundle(
    State(state): State<AppState>,
    Query(query): Query<BundleListQuery>,
) -> WebResult<Response> {
    Ok(json(
        state
            .database
            .query_bundles_with_details(&query.into())
            .await?,
    ))
}

#[debug_handler]
#[utoipa::path(get, path = "/manifest/{version_id}/children", tag = "manifest", params(ManifestChildrenQuery), responses((status = OK, body = [ManifestNode])))]
pub async fn list_manifest_children(
    State(state): State<AppState>,
    Path(version_id): Path<i32>,
    Query(params): Query<ManifestChildrenQuery>,
) -> WebResult<Response> {
    let dir = params.dir.unwrap_or_default();
    Ok(json(
        state
            .database
            .list_manifest_children(version_id, &dir)
            .await?,
    ))
}

#[debug_handler]
#[utoipa::path(get, path = "/manifest/{version_id}/detail", tag = "manifest", params(ManifestDetailQuery), responses((status = OK, body = AssetMappingDetails)))]
pub async fn get_manifest_detail(
    State(state): State<AppState>,
    Path(version_id): Path<i32>,
    Query(params): Query<ManifestDetailQuery>,
) -> WebResult<Response> {
    let result = state
        .database
        .get_asset_mapping_detail(version_id, &params.asset_name)
        .await?
        .ok_or(WebError::NotFound)?;
    Ok(json(result))
}

#[debug_handler]
#[utoipa::path(get, path = "/manifest/{version_id}/search", tag = "manifest", params(ManifestSearchQuery), responses((status = OK, body = [ManifestNode])))]
pub async fn search_manifest(
    State(state): State<AppState>,
    Path(version_id): Path<i32>,
    Query(params): Query<ManifestSearchQuery>,
) -> WebResult<Response> {
    Ok(json(
        state
            .database
            .search_manifest(version_id, &params.q)
            .await?,
    ))
}

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
        .database
        .query_usage_by_item_name(&item_name)
        .await?
        .ok_or(WebError::NotFound)?;

    Ok(([(header::CONTENT_TYPE, "application/json")], usage).into_response())
}

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
    security(("torappu-auth" = []))
)]
pub async fn launch_container(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    Json(payload): Json<DockerLaunchRequest>,
) -> Result<Json<DockerLaunchResponse>, WebError> {
    let auth_header = headers
        .get("torappu-auth")
        .ok_or(WebError::Unauthorized(
            "Missing torappu-auth header".to_string(),
        ))?
        .to_str()
        .map_err(|_| WebError::Unauthorized("Invalid torappu-auth header format".to_string()))?;

    let expected_token = state.settings.torappu.token.as_str();
    if auth_header != expected_token {
        return Err(WebError::Unauthorized(
            "Invalid authentication token".to_string(),
        ));
    }

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

    let docker = state
        .docker
        .as_ref()
        .ok_or(WebError::ServiceUnavailable(anyhow::anyhow!(
            "Docker service is not configured or available"
        )))?;

    let container_name = docker
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

#[debug_handler]
#[utoipa::path(
    get,
    path = "/files",
    tag = "files",
    params(("path" = String, Query, description = "Search path pattern")),
    responses((status = 200, description = "List of matching entries"))
)]
pub async fn search_assets_by_path(
    State(state): State<AppState>,
    Query(AssetSearchQuery { path }): Query<AssetSearchQuery>,
) -> WebResult<Response> {
    Ok(json(state.torappu.search_assets_by_path(&path)?))
}

#[utoipa::path(
    get,
    path = "/files/{path}",
    tag = "files",
    params(("path" = String, Path, description = "Directory path to list")),
    responses((status = 200, description = "Directory listing"))
)]
pub async fn list_asset(
    State(state): State<AppState>,
    Path(path): Path<String>,
) -> WebResult<Response> {
    Ok(json(state.torappu.list_asset(&path)?))
}

pub async fn list_root_asset(State(state): State<AppState>) -> WebResult<Response> {
    Ok(json(state.torappu.list_asset("")?))
}

#[derive(Embed)]
#[folder = "dist"]
struct Assets;

static INDEX_HTML: &str = "index.html";

pub async fn static_handler(uri: axum::http::Uri) -> impl IntoResponse {
    let path = uri.path().trim_start_matches('/');
    if path.is_empty() || path == INDEX_HTML {
        return index_html();
    }

    match Assets::get(path) {
        Some(content) => {
            let mime = mime_guess::from_path(path).first_or_octet_stream();
            ([(header::CONTENT_TYPE, mime.as_ref())], content.data).into_response()
        }
        None => Assets::get(INDEX_HTML).map_or_else(
            || (StatusCode::NOT_FOUND, "404").into_response(),
            |content| {
                (
                    [(header::CONTENT_TYPE, "text/html; charset=utf-8")],
                    content.data,
                )
                    .into_response()
            },
        ),
    }
}

fn index_html() -> Response {
    match Assets::get(INDEX_HTML) {
        Some(content) => axum::response::Html(content.data).into_response(),
        None => (StatusCode::NOT_FOUND, "404").into_response(),
    }
}
