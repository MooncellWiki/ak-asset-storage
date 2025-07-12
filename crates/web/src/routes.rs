use crate::{
    handlers::{bundle_handlers, misc_handlers, version_handlers},
    middleware::apply_axum_middleware,
    state::AppState,
};
use axum::{routing::get, Json, Router};
use utoipa::OpenApi;
use utoipa_axum::{router::OpenApiRouter, routes};
use utoipa_scalar::{Scalar, Servable};

#[derive(OpenApi)]
#[openapi(
    tags(
        (name = "version", description = "Version management endpoints"),
        (name = "bundle", description = "Bundle management endpoints"),
        (name = "health", description = "Health check endpoints"),
    ),
)]
pub struct ApiDoc;

pub fn build_router(state: AppState) -> Router {
    let (api_routes, openapi) = OpenApiRouter::with_openapi(ApiDoc::openapi())
        // Health endpoints
        .routes(routes!(misc_handlers::ping))
        .routes(routes!(misc_handlers::health))
        // Version endpoints
        .routes(routes!(version_handlers::list))
        .routes(routes!(version_handlers::get))
        .routes(routes!(version_handlers::get_files))
        // Bundle endpoints
        .routes(routes!(bundle_handlers::get_one))
        .routes(routes!(bundle_handlers::filter))
        .split_for_parts();

    let full_router = Router::new()
        .nest("/api/v1", api_routes)
        .merge(Scalar::with_url("/api/v1/scalar", openapi.clone()))
        .route("/api/v1/openapi.json", get(|| async move { Json(openapi) }))
        .fallback(misc_handlers::static_handler)
        .with_state(state);

    // Apply middleware
    apply_axum_middleware(full_router)
}
