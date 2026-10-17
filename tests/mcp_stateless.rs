use ak_asset_storage::{
    api::{AppState, build_router},
    config::AppSettings,
};
use axum::{
    Router,
    body::{Body, to_bytes},
    http::{Request, StatusCode, header::CONTENT_TYPE},
};
use serde_json::{Value, json};
use std::{
    fs,
    path::PathBuf,
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
};
use tower::ServiceExt;

struct Fixture(PathBuf);

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

async fn post(
    router: &Router,
    version: &str,
    session: Option<&str>,
    message: Value,
) -> Option<Value> {
    let mut request = Request::post("/mcp")
        .header("host", "torappu.example")
        .header(CONTENT_TYPE, "application/json")
        .header("accept", "application/json, text/event-stream")
        .header("mcp-protocol-version", version);
    if let Some(session) = session {
        request = request.header("mcp-session-id", session);
    }
    let response = router
        .clone()
        .oneshot(request.body(Body::from(message.to_string())).unwrap())
        .await
        .unwrap();
    assert!(!response.headers().contains_key("mcp-session-id"));
    if response.status() == StatusCode::ACCEPTED {
        return None;
    }
    let status = response.status();
    let content_type = response.headers().get(CONTENT_TYPE).cloned();
    let body = to_bytes(response.into_body(), 1024 * 1024).await.unwrap();
    assert_eq!(
        status,
        StatusCode::OK,
        "{} ({version}): {}",
        message["method"],
        String::from_utf8_lossy(&body)
    );
    assert_eq!(content_type.unwrap(), "application/json");
    let message: Value = serde_json::from_slice(&body).unwrap();
    assert!(message.get("error").is_none(), "{message}");
    Some(message)
}

#[tokio::test]
#[ignore = "manual integration test requiring local PostgreSQL and plocate"]
async fn mcp_requests_can_move_between_backend_instances() {
    let fixture = Fixture(std::env::temp_dir().join(format!(
        "ak-mcp-stateless-{}-{}",
        std::process::id(),
        SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos()
    )));
    let assets = fixture.0.join("assets");
    fs::create_dir_all(assets.join("raw")).unwrap();
    fs::write(assets.join("raw/replica-test.txt"), "asset fixture").unwrap();

    let mut settings: AppSettings = toml::from_str(include_str!("../example.toml")).unwrap();
    settings.database.uri = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgres://ak:ak@localhost:32432/ak_asset_storage_next".to_string());
    settings.mcp.enable = true;
    settings.torappu.kubernetes = None;
    settings.torappu.asset_base_path = assets.to_str().unwrap().to_string();
    settings.torappu.plocate.database_path =
        Some(fixture.0.join("plocate.db").to_str().unwrap().to_string());
    let state = AppState::from_settings(Arc::new(settings)).await.unwrap();
    let first = build_router(state.clone());
    let second = build_router(state);

    for session in [None, Some("previous-backend-session")] {
        for version in ["2025-03-26", "2025-06-18", "2025-11-25"] {
            let initialized = post(
                &first,
                version,
                session,
                json!({
                    "jsonrpc": "2.0", "id": 1, "method": "initialize",
                    "params": {
                        "protocolVersion": version, "capabilities": {},
                        "clientInfo": { "name": "replica-test", "version": "1" }
                    }
                }),
            )
            .await
            .unwrap();
            assert_eq!(initialized["result"]["protocolVersion"], version);
            assert!(
                post(
                    &second,
                    version,
                    session,
                    json!({ "jsonrpc": "2.0", "method": "notifications/initialized" }),
                )
                .await
                .is_none()
            );
            let tools = post(
                &second,
                version,
                session,
                json!({ "jsonrpc": "2.0", "id": 2, "method": "tools/list" }),
            )
            .await
            .unwrap();
            assert!(
                tools["result"]["tools"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .any(|tool| { tool["name"] == "list_files" })
            );
            let listing = post(
                &second,
                version,
                session,
                json!({
                    "jsonrpc": "2.0", "id": 3, "method": "tools/call",
                    "params": { "name": "list_files", "arguments": { "path": "raw" } }
                }),
            )
            .await
            .unwrap();
            assert_ne!(listing["result"]["isError"], true);
            assert!(
                listing["result"]["content"]
                    .to_string()
                    .contains("replica-test.txt")
            );
        }
    }
}
