use crate::{
    app::AppState,
    controllers::{assets, bundles, file, misc, tokens, unpack, versions},
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
                .routes(routes!(bundles::get_bundle_by_id))
                .routes(routes!(bundles::list_bundle))
                .routes(routes!(versions::list))
                .routes(routes!(versions::get))
                .routes(routes!(versions::list_files))
                .routes(routes!(versions::latest))
                .routes(routes!(assets::get_asset_by_id))
                .routes(routes!(assets::reuse_assets_by_bundle))
                .routes(routes!(assets::create_asset))
                .routes(routes!(assets::list_assets))
                .routes(routes!(file::upload))
                .routes(routes!(
                    unpack::list_unpack_versions,
                    unpack::create_unpack_version
                ))
                .routes(routes!(unpack::get_latest_finished_version))
                .routes(routes!(unpack::finish_unpack_version))
                .routes(routes!(tokens::list_tokens)),
        )
        .split_for_parts();
    router
        .merge(Scalar::with_url("/api/v1/scalar", openapi.clone()))
        .route("/api/v1/openapi.json", get(|| async move { Json(openapi) }))
        .fallback(misc::static_handler)
        .with_state(state)
}
