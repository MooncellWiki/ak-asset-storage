use crate::support::TestEnv;
use axum::http::StatusCode;

const ITEM_DEMAND_GET_PATH: &str = "/api/v1/item/技巧概要·卷1/demand";

fn load_fixture() -> serde_json::Value {
    let repo_root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let content = std::fs::read_to_string(repo_root.join("e2e/fixtures/item_demand.json"))
        .expect("fixture file missing");
    serde_json::from_str(&content).expect("invalid fixture json")
}

fn load_second_fixture() -> serde_json::Value {
    let mut fixture = load_fixture();
    let obj = fixture.as_object_mut().unwrap();
    obj.retain(|k, _| k == "源岩");
    fixture
}

#[tokio::test]
#[ignore = "manual e2e test requiring docker, rc, and fixture assets"]
async fn import_and_get_success() {
    let env = TestEnv::bootstrap().await;
    env.copy_item_demand_fixture("item_demand.json");

    env.run_import_item_demand().await;

    let (status, body) = env.get_text(ITEM_DEMAND_GET_PATH).await;
    assert_eq!(status, StatusCode::OK);

    let fixture = load_fixture();
    let expected = serde_json::to_string(fixture.get("技巧概要·卷1").unwrap()).unwrap();
    assert_eq!(body, expected);
}

#[tokio::test]
#[ignore = "manual e2e test requiring docker, rc, and fixture assets"]
async fn import_replaces_existing_data() {
    let env = TestEnv::bootstrap().await;

    // First import: full fixture contains 技巧概要·卷1 and 源岩
    env.copy_item_demand_fixture("item_demand.json");
    env.run_import_item_demand().await;

    // Second import: reduced fixture only contains 源岩
    let second = load_second_fixture();
    let repo_root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let fixture_path = repo_root.join("e2e/fixtures/item_demand_second.json");
    std::fs::write(&fixture_path, serde_json::to_string(&second).unwrap()).unwrap();
    env.copy_item_demand_fixture("item_demand_second.json");
    env.run_import_item_demand().await;

    // First fixture contained 技巧概要·卷1, second fixture does not => should be gone
    let (status, _body) = env.get_text(ITEM_DEMAND_GET_PATH).await;
    assert_eq!(status, StatusCode::NOT_FOUND);

    // 源岩 should exist
    let (status, body) = env.get_text("/api/v1/item/源岩/demand").await;
    assert_eq!(status, StatusCode::OK);
    let expected = serde_json::to_string(second.get("源岩").unwrap()).unwrap();
    assert_eq!(body, expected);

    // Cleanup temp fixture
    let _ = std::fs::remove_file(&fixture_path);
}

#[tokio::test]
#[ignore = "manual e2e test requiring docker, rc, and fixture assets"]
async fn get_nonexistent_item_returns_404() {
    let env = TestEnv::bootstrap().await;
    let (status, _body) = env.get_text("/api/v1/item/不存在的材料/demand").await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}
