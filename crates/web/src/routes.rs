#![allow(clippy::needless_for_each)]
use crate::{
    handlers::{
        bundle_handlers, docker_handlers, item_demand_handlers, misc_handlers, torappu_handlers,
        version_handlers,
    },
    middleware::{apply_axum_middleware, serve_dir_with_charset},
    state::AppState,
};
use ak_asset_storage_application::ConfigProvider;
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
        (name = "fs", description = "File system endpoints"),
        (name = "docker", description = "Docker container management endpoints"),
    ),
)]
pub struct ApiDoc;

pub fn build_router(state: AppState) -> Router {
    let (api_routes, mut openapi) = OpenApiRouter::with_openapi(ApiDoc::openapi())
        // Health endpoints
        .routes(routes!(misc_handlers::ping))
        .routes(routes!(misc_handlers::health))
        // Torappu assets endpoints
        .routes(routes!(torappu_handlers::list_asset))
        .route("/files/", get(torappu_handlers::list_root_asset))
        .routes(routes!(torappu_handlers::search_assets_by_path))
        // Version endpoints
        .routes(routes!(version_handlers::list_version))
        .routes(routes!(version_handlers::get_version))
        .routes(routes!(version_handlers::get_files_by_version))
        // Bundle endpoints
        .routes(routes!(bundle_handlers::get_one))
        .routes(routes!(bundle_handlers::filter))
        // Item endpoints
        .routes(routes!(item_demand_handlers::get_item_demand))
        .routes(routes!(item_demand_handlers::update_item_demands))
        // Docker endpoints
        .routes(routes!(docker_handlers::launch_container))
        .split_for_parts();

    openapi.paths.paths = openapi
        .paths
        .paths
        .into_iter()
        .map(|(path, item)| (format!("/api/v1{path}"), item))
        .collect::<utoipa::openapi::path::PathsMap<_, _>>();
    let asset_path = PathBuf::from(&state.config.torappu_config().asset_base_path);
    let full_router = Router::new()
        .nest("/api/v1", api_routes)
        .merge(Scalar::with_url("/api/v1/scalar", openapi.clone()))
        .route("/api/v1/openapi.json", get(|| async move { Json(openapi) }))
        .nest_service("/assets", serve_dir_with_charset(asset_path.join("raw")))
        .nest_service(
            "/gamedata",
            serve_dir_with_charset(asset_path.join("gamedata")),
        )
        .fallback(misc_handlers::static_handler)
        .with_state(state);

    // Apply middleware
    apply_axum_middleware(full_router)
}
