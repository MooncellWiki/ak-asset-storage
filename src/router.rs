use crate::{
    app::AppState,
    controllers::{bundles, misc, versions},
    openapi::BaseOpenApi,
};
use axum::{routing::get, Json, Router};
use utoipa_axum::{router::OpenApiRouter, routes};
use utoipa_scalar::{Scalar, Servable};

pub fn build_axum_router(state: AppState) -> Router<()> {
    let (router, openapi) = BaseOpenApi::router()
        .nest(
            "/api/v1",
            OpenApiRouter::new()
                .routes(routes!(misc::ping))
                .routes(routes!(misc::health))
                .routes(routes!(bundles::get_one))
                .routes(routes!(bundles::filter))
                .routes(routes!(versions::list))
                .routes(routes!(versions::get))
                .routes(routes!(versions::list_files))
                .routes(routes!(versions::latest)),
        )
        .split_for_parts();
    router
        .merge(Scalar::with_url("/api/v1/scalar", openapi.clone()))
        .route("/api/v1/openapi.json", get(|| async move { Json(openapi) }))
        .fallback(misc::static_handler)
        .with_state(state)
}
