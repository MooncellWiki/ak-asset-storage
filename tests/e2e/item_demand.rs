use crate::support::TestEnv;
use axum::http::StatusCode;
use std::path::PathBuf;

const ITEM_NAME: &str = "技巧概要·卷1";
const SECOND_ITEM_NAME: &str = "源岩";

fn fixture_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("e2e/fixtures")
        .join(name)
}

fn item_demand_path(item_name: &str) -> String {
    format!("/api/v1/item/{item_name}/demand")
}

fn load_fixture() -> serde_json::Value {
    let content =
        std::fs::read_to_string(fixture_path("item_demand.json")).expect("fixture file missing");
    serde_json::from_str(&content).expect("invalid fixture json")
}

fn load_second_fixture() -> serde_json::Value {
    let fixture = load_fixture();
    let value = fixture.get(SECOND_ITEM_NAME).unwrap().clone();
    serde_json::json!({ SECOND_ITEM_NAME: value })
}

#[tokio::test]
#[ignore = "manual e2e test requiring docker, rc, and fixture assets"]
async fn import_and_get_success() {
    let env = TestEnv::bootstrap().await;
    env.copy_item_demand_fixture(fixture_path("item_demand.json"));

    env.run_import_item_demand().await;

    let (status, body) = env.get_text(&item_demand_path(ITEM_NAME)).await;
    assert_eq!(status, StatusCode::OK);

    let fixture = load_fixture();
    let expected = serde_json::to_string(fixture.get(ITEM_NAME).unwrap()).unwrap();
    assert_eq!(body, expected);
}

#[tokio::test]
#[ignore = "manual e2e test requiring docker, rc, and fixture assets"]
async fn import_replaces_existing_data() {
    let env = TestEnv::bootstrap().await;

    // First import: full fixture contains 技巧概要·卷1 and 源岩
    env.copy_item_demand_fixture(fixture_path("item_demand.json"));
    env.run_import_item_demand().await;

    // Second import: reduced fixture only contains 源岩
    let second = load_second_fixture();
    let second_fixture_path = env.runtime_dir().join("item_demand_second.json");
    std::fs::write(
        &second_fixture_path,
        serde_json::to_string(&second).unwrap(),
    )
    .unwrap();
    env.copy_item_demand_fixture(&second_fixture_path);
    env.run_import_item_demand().await;

    // First fixture contained 技巧概要·卷1, second fixture does not => should be gone
    let (status, _) = env.get_text(&item_demand_path(ITEM_NAME)).await;
    assert_eq!(status, StatusCode::NOT_FOUND);

    // 源岩 should exist
    let (status, body) = env.get_text(&item_demand_path(SECOND_ITEM_NAME)).await;
    assert_eq!(status, StatusCode::OK);
    let expected = serde_json::to_string(second.get(SECOND_ITEM_NAME).unwrap()).unwrap();
    assert_eq!(body, expected);
}

#[tokio::test]
#[ignore = "manual e2e test requiring docker, rc, and fixture assets"]
async fn get_nonexistent_item_returns_404() {
    let env = TestEnv::bootstrap().await;
    let (status, _) = env.get_text("/api/v1/item/不存在的材料/demand").await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}
