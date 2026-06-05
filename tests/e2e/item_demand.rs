use crate::support::TestEnv;
use axum::http::StatusCode;

const ITEM_DEMAND_PATH: &str = "/api/v1/item/demand";
const ITEM_DEMAND_GET_PATH: &str = "/api/v1/item/技巧概要·卷1/demand";
const VALID_TOKEN: &str = "e2e-token";

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
async fn update_without_auth_returns_401() {
    let env = TestEnv::bootstrap().await;
    let fixture = load_fixture();
    let status = env
        .post_json_with_auth(ITEM_DEMAND_PATH, None, fixture)
        .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
#[ignore = "manual e2e test requiring docker, rc, and fixture assets"]
async fn update_with_wrong_token_returns_401() {
    let env = TestEnv::bootstrap().await;
    let fixture = load_fixture();
    let status = env
        .post_json_with_auth(ITEM_DEMAND_PATH, Some("wrong-token"), fixture)
        .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
#[ignore = "manual e2e test requiring docker, rc, and fixture assets"]
async fn update_and_get_success() {
    let env = TestEnv::bootstrap().await;
    let fixture = load_fixture();

    let status = env
        .post_json_with_auth(ITEM_DEMAND_PATH, Some(VALID_TOKEN), fixture.clone())
        .await;
    assert_eq!(status, StatusCode::OK);

    let (status, body) = env.get_text(ITEM_DEMAND_GET_PATH).await;
    assert_eq!(status, StatusCode::OK);

    let expected = serde_json::to_string(fixture.get("技巧概要·卷1").unwrap()).unwrap();
    assert_eq!(body, expected);
}

#[tokio::test]
#[ignore = "manual e2e test requiring docker, rc, and fixture assets"]
async fn update_replaces_existing_data() {
    let env = TestEnv::bootstrap().await;
    let first = load_fixture();
    let second = load_second_fixture();

    let status = env
        .post_json_with_auth(ITEM_DEMAND_PATH, Some(VALID_TOKEN), first)
        .await;
    assert_eq!(status, StatusCode::OK);

    let status = env
        .post_json_with_auth(ITEM_DEMAND_PATH, Some(VALID_TOKEN), second.clone())
        .await;
    assert_eq!(status, StatusCode::OK);

    // First fixture contained 技巧概要·卷1, second fixture does not
    let (status, _body) = env.get_text(ITEM_DEMAND_GET_PATH).await;
    assert_eq!(status, StatusCode::NOT_FOUND);

    // 源岩 should exist
    let (status, body) = env.get_text("/api/v1/item/源岩/demand").await;
    assert_eq!(status, StatusCode::OK);
    let expected = serde_json::to_string(second.get("源岩").unwrap()).unwrap();
    assert_eq!(body, expected);
}

#[tokio::test]
#[ignore = "manual e2e test requiring docker, rc, and fixture assets"]
async fn get_nonexistent_item_returns_404() {
    let env = TestEnv::bootstrap().await;
    let (status, _body) = env.get_text("/api/v1/item/不存在的材料/demand").await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}
