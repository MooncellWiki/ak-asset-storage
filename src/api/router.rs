#![allow(clippy::needless_for_each)]

use crate::api::{
    AppState, handlers,
    middleware::{apply_axum_middleware, serve_dir_with_charset},
};
use axum::{Json, Router, routing::get};
use std::path::PathBuf;
use utoipa::OpenApi;
use utoipa_axum::{router::OpenApiRouter, routes};
use utoipa_scalar::{Scalar, Servable};

#[derive(OpenApi)]
#[openapi(
    tags(
        (name = "version", description = "Version management endpoints"),
        (name = "bundle", description = "Bundle management endpoints"),
        (name = "item", description = "Item demand endpoints"),
        (name = "health", description = "Health check endpoints"),
        (name = "files", description = "File system endpoints"),
        (name = "docker", description = "Docker container management endpoints"),
        (name = "manifest", description = "Manifest browser endpoints"),
    ),
)]
pub struct ApiDoc;

pub fn build_router(state: AppState) -> Router {
    let (api_routes, mut openapi) = OpenApiRouter::with_openapi(ApiDoc::openapi())
        .routes(routes!(handlers::ping))
        .routes(routes!(handlers::health))
        .routes(routes!(handlers::list_asset))
        .route("/files/", get(handlers::list_root_asset))
        .routes(routes!(handlers::search_assets_by_path))
        .routes(routes!(handlers::list_version))
        .routes(routes!(handlers::get_version))
        .routes(routes!(handlers::get_files_by_version))
        .routes(routes!(handlers::get_bundle))
        .routes(routes!(handlers::filter_bundle))
        .routes(routes!(handlers::list_manifest_children))
        .routes(routes!(handlers::get_manifest_detail))
        .routes(routes!(handlers::search_manifest))
        .routes(routes!(handlers::get_item_demand))
        .routes(routes!(handlers::launch_container))
        .split_for_parts();

    openapi.paths.paths = openapi
        .paths
        .paths
        .into_iter()
        .map(|(path, item)| (format!("/api/v1{path}"), item))
        .collect::<utoipa::openapi::path::PathsMap<_, _>>();

    let asset_path = PathBuf::from(&state.settings.torappu.asset_base_path);
    let router = Router::new()
        .nest("/api/v1", api_routes)
        .merge(Scalar::with_url("/api/v1/scalar", openapi.clone()))
        .route("/api/v1/openapi.json", get(|| async move { Json(openapi) }))
        .nest_service("/assets", serve_dir_with_charset(asset_path.join("raw")))
        .nest_service(
            "/gamedata",
            serve_dir_with_charset(asset_path.join("gamedata")),
        )
        .fallback(handlers::static_handler)
        .with_state(state);

    apply_axum_middleware(router)
}
