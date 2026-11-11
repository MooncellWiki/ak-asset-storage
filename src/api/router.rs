#![allow(clippy::needless_for_each)]

use crate::api::{
    AppState, embed, handlers, launcher, mcp,
    metrics::McpMetrics,
    middleware::{apply_axum_middleware, serve_dir_with_charset},
    story_explorer,
};
use axum::{Json, Router, http::StatusCode, response::IntoResponse, routing::get};
use rmcp::transport::streamable_http_server::{
    StreamableHttpServerConfig, StreamableHttpService, session::never::NeverSessionManager,
};
use std::{path::PathBuf, sync::Arc};
use tracing::info;
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
        (name = "story", description = "Story resource usage endpoints"),
        (name = "launcher", description = "PC launcher releases and original installation files"),
    ),
)]
pub struct ApiDoc;

pub fn build_router(state: AppState) -> Router {
    let (api_routes, mut openapi) = OpenApiRouter::with_openapi(ApiDoc::openapi())
        .routes(routes!(handlers::ping))
        .routes(routes!(launcher::list_releases))
        .routes(routes!(launcher::get_release))
        .routes(routes!(launcher::list_files))
        .routes(routes!(launcher::diff))
        .routes(routes!(launcher::manifest))
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
        .routes(routes!(handlers::get_story_resource_usages))
        .routes(routes!(handlers::list_story_resources))
        .routes(routes!(story_explorer::explore_story_resources))
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
        );

    let metrics = state.settings.metrics.enable.then(McpMetrics::new);
    let router = if state.settings.mcp.enable {
        let mcp_server = mcp::AkAssetMcpServer::new(state.clone(), metrics.clone());
        // Host restrictions are enforced by the gateway, not the SDK's
        // default loopback-only allowlist.
        // Read-only tools need no session affinity when requests cross backend replicas.
        let mcp_config = StreamableHttpServerConfig::default()
            .with_legacy_session_mode(false)
            .with_json_response(true)
            .disable_allowed_hosts();
        let mcp_service = StreamableHttpService::new(
            move || Ok(mcp_server.clone()),
            Arc::new(NeverSessionManager::default()),
            mcp_config,
        );
        info!("MCP endpoint enabled at /mcp");
        router.route_service("/mcp", mcp_service)
    } else {
        router
    };

    if metrics.is_some() {
        info!("Prometheus endpoint enabled at /metrics");
    }
    let router = router
        .route(
            "/metrics",
            get(move || async move {
                // Reserve this path when disabled so the SPA fallback cannot
                // return index.html to a scrape request.
                metrics
                    .as_ref()
                    .map_or_else(|| StatusCode::NOT_FOUND.into_response(), McpMetrics::scrape)
            }),
        )
        .merge(embed::router())
        .with_state(state);

    apply_axum_middleware(router)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{config::AppSettings, external::plocate::binaries_available_for_tests};
    use axum::{
        body::{Body, to_bytes},
        http::{Request, header},
        response::Response,
    };
    use serde_json::{Value, json};
    use sqlx::ConnectOptions;
    use tower::ServiceExt;

    async fn mcp_request(router: &Router, method: &str, params: Value) -> Value {
        let body = json!({"jsonrpc": "2.0", "id": 1, "method": method, "params": params});
        let request = Request::post("/mcp")
            .header(header::HOST, "localhost")
            .header(header::CONTENT_TYPE, "application/json")
            .header(header::ACCEPT, "application/json, text/event-stream")
            .header("mcp-protocol-version", "2025-11-25")
            .body(Body::from(body.to_string()))
            .unwrap();
        let response = router.clone().oneshot(request).await.unwrap();
        let status = response.status();
        let body = to_bytes(response.into_body(), 64 * 1024).await.unwrap();
        assert_eq!(status, StatusCode::OK, "{}", String::from_utf8_lossy(&body));
        serde_json::from_slice(&body).unwrap()
    }

    async fn scrape(router: Router) -> Response {
        router
            .oneshot(Request::get("/metrics").body(Body::empty()).unwrap())
            .await
            .unwrap()
    }

    #[sqlx::test]
    async fn prometheus_reports_completed_mcp_calls(pool: sqlx::PgPool) {
        if !binaries_available_for_tests() {
            return;
        }
        let temp = tempfile::tempdir().unwrap();
        let assets = temp.path().join("assets");
        std::fs::create_dir(&assets).unwrap();
        let mut config: toml::Table = toml::from_str(include_str!("../../example.toml")).unwrap();
        config.remove("metrics");
        let mut settings: AppSettings = config.try_into().unwrap();
        settings.database.uri = pool.connect_options().to_url_lossy().to_string();
        settings.torappu.kubernetes = None;
        settings.torappu.asset_base_path = assets.to_str().unwrap().into();
        settings.torappu.plocate.database_path =
            Some(temp.path().join("plocate.db").to_str().unwrap().into());
        settings.mcp.enable = true;
        let mut state = AppState::from_settings(Arc::new(settings)).await.unwrap();
        assert_eq!(
            scrape(build_router(state.clone())).await.status(),
            StatusCode::NOT_FOUND
        );

        Arc::make_mut(&mut state.settings).metrics.enable = true;
        let router = build_router(state.clone());
        let list = mcp_request(&router, "tools/list", json!({})).await;
        assert!(list["result"]["tools"].is_array(), "{list}");

        let call = |name, arguments| json!({"name": name, "arguments": arguments});
        let success = mcp_request(&router, "tools/call", call("list_files", json!({}))).await;
        assert_eq!(success["result"]["isError"], false, "{success}");
        let invalid = mcp_request(
            &router,
            "tools/call",
            call("list_files", json!({"path": 1})),
        )
        .await;
        assert_eq!(invalid["result"]["isError"], true, "{invalid}");
        let missing = mcp_request(
            &router,
            "tools/call",
            call("list_files", json!({"path": "missing"})),
        )
        .await;
        assert!(missing["error"].is_object(), "{missing}");
        for name in ["unregistered_tool_1", "unregistered_tool_2"] {
            let unknown = mcp_request(&router, "tools/call", call(name, json!({}))).await;
            assert!(unknown["error"].is_object(), "{unknown}");
        }

        let response = scrape(router).await;
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(
            response.headers()[header::CONTENT_TYPE],
            "text/plain; version=0.0.4"
        );
        let body = to_bytes(response.into_body(), 64 * 1024).await.unwrap();
        let text = std::str::from_utf8(&body).unwrap();
        let nonzero_counts: Vec<_> = text
            .lines()
            .filter(|line| {
                line.starts_with("ak_asset_storage_mcp_tool_call_duration_seconds_count{")
                    && !line.ends_with(" 0")
            })
            .collect();
        assert_eq!(nonzero_counts.len(), 3, "{nonzero_counts:?}");
        for (tool, result, count) in [
            ("list_files", "success", 1),
            ("list_files", "error", 2),
            ("unknown", "error", 2),
        ] {
            let expected = format!(
                "ak_asset_storage_mcp_tool_call_duration_seconds_count{{result=\"{result}\",tool=\"{tool}\"}} {count}"
            );
            assert!(
                nonzero_counts.contains(&expected.as_str()),
                "{nonzero_counts:?}"
            );
        }

        Arc::make_mut(&mut state.settings).mcp.enable = false;
        let response = scrape(build_router(state)).await;
        assert_eq!(response.status(), StatusCode::OK);
        assert!(to_bytes(response.into_body(), 0).await.unwrap().is_empty());
    }
}
