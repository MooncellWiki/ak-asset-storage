use crate::support::{self, TestEnv};
use ak_asset_storage::database::row::StoryUsageRow;
use axum::http::StatusCode;
use std::{path::Path, time::Duration};
use tokio::time::sleep;

const V1_RES: &str = "26-06-01-00-00-00_storyv1";
const V2_RES: &str = "26-06-02-00-00-00_storyv2";

const V1_SCRIPT: &str = r#"[HEADER(key="test")] 测试
[Background(image="bg_test_1", fadetime=1)]
[Character(name="avg_npc_001")]
[name="测试者甲"]   你好
[ShowItem(image="item_test_1")]
"#;

const V1_INFO_SCRIPT: &str = "[Background(image=\"bg_info_ignored\")]\n";

const V2_SCRIPT: &str = r#"[Background(image="bg_test_2", fadetime=1)]
[Image(image="img_test_2")]
[Character(name="avg_npc_002#3")]
[name="测试者乙"]   再见
"#;

fn write_file(path: &Path, content: &str) {
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, content).unwrap();
}

fn marker_json() -> String {
    r#"{"schema_version":1,"completed_at":"2026-08-17T10:00:00Z"}"#.to_string()
}

fn gamedata_root(env: &TestEnv) -> std::path::PathBuf {
    env.runtime_dir().join("asset/gamedata")
}

/// Writes the version directory (story scripts + marker) and points the
/// `latest` symlink at it, mirroring torappu's publish order.
fn publish_version(env: &TestEnv, res_version: &str) {
    let version_dir = gamedata_root(env).join(res_version);
    match res_version {
        V1_RES => {
            write_file(
                &version_dir.join("story/activities/test/level_test_01_beg.txt"),
                V1_SCRIPT,
            );
            write_file(
                &version_dir.join("story/[uc]info/activities/test/level_test_01_beg.txt"),
                V1_INFO_SCRIPT,
            );
        }
        V2_RES => {
            write_file(
                &version_dir.join("story/obt/main/level_main_test.txt"),
                V2_SCRIPT,
            );
        }
        other => panic!("unknown fixture version {other}"),
    }
    write_file(&version_dir.join(".gamedata-ready.json"), &marker_json());

    let latest = gamedata_root(env).join("latest");
    let _ = std::fs::remove_file(&latest);
    std::os::unix::fs::symlink(res_version, &latest).unwrap();
}

async fn script_paths(
    database: &ak_asset_storage::database::Database,
    resource_type: &str,
    resource_id: &str,
) -> Vec<String> {
    database
        .query_story_resource_usages(resource_type, resource_id, None, 200)
        .await
        .unwrap()
        .into_iter()
        .map(|row| row.script_path)
        .collect()
}

async fn wait_for_usages(
    database: &ak_asset_storage::database::Database,
    resource_type: &str,
    resource_id: &str,
    expected_script: &str,
    timeout: Duration,
) -> Result<(), String> {
    support::wait_for(timeout, Duration::from_secs(2), || async {
        script_paths(database, resource_type, resource_id)
            .await
            .iter()
            .any(|path| path == expected_script)
    })
    .await
    .map_err(|()| {
        format!("usage rows for {resource_type}/{resource_id} did not appear within {timeout:?}")
    })
}

#[tokio::test]
#[ignore = "manual e2e test requiring docker, rc, and fixture assets"]
async fn manual_import_replaces_snapshot_and_serves_api() {
    let env = TestEnv::bootstrap().await;
    publish_version(&env, V1_RES);
    env.run_import_story_usage().await;

    // background + [uc]info exclusion + character display names
    let (status, body) = env
        .get_text("/api/v1/story-resource-usages?type=character&id=avg_npc_001%231%241")
        .await;
    assert_eq!(status, StatusCode::OK);
    assert!(
        body.contains(r#""scriptPath":"activities/test/level_test_01_beg""#),
        "{body}"
    );
    assert!(body.contains(r#""displayNames":["测试者甲"]"#), "{body}");

    let (status, body) = env
        .get_text("/api/v1/story-resource-usages?type=background&id=bg_test_1")
        .await;
    assert_eq!(status, StatusCode::OK);
    assert!(body.contains("activities/test/level_test_01_beg"), "{body}");

    // [uc]info/** must not be imported.
    let (status, body) = env
        .get_text("/api/v1/story-resource-usages?type=background&id=bg_info_ignored")
        .await;
    assert_eq!(status, StatusCode::OK);
    assert!(body.contains(r#""items":[]"#), "{body}");

    // Invalid type / limit are rejected.
    let (status, _) = env
        .get_text("/api/v1/story-resource-usages?type=unknown&id=x")
        .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    let (status, _) = env
        .get_text("/api/v1/story-resource-usages?type=item&id=x&limit=201")
        .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);

    // Switching latest to a new version and re-importing replaces the snapshot.
    publish_version(&env, V2_RES);
    env.run_import_story_usage().await;

    let (status, body) = env
        .get_text("/api/v1/story-resource-usages?type=background&id=bg_test_1")
        .await;
    assert_eq!(status, StatusCode::OK);
    assert!(body.contains(r#""items":[]"#), "{body}");

    let (status, body) = env
        .get_text("/api/v1/story-resource-usages?type=character&id=avg_npc_002%233%241")
        .await;
    assert_eq!(status, StatusCode::OK);
    assert!(
        body.contains(r#""scriptPath":"obt/main/level_main_test""#),
        "{body}"
    );
    assert!(body.contains(r#""displayNames":["测试者乙"]"#), "{body}");
}

#[tokio::test]
#[ignore = "manual e2e test requiring docker, rc, and fixture assets"]
async fn failed_import_keeps_previous_snapshot() {
    let env = TestEnv::bootstrap().await;
    publish_version(&env, V1_RES);
    env.run_import_story_usage().await;

    // A marker with an unsupported schema version must fail validation
    // before any transaction opens.
    write_file(
        &gamedata_root(&env)
            .join(V1_RES)
            .join(".gamedata-ready.json"),
        r#"{"schema_version":2,"completed_at":"2026-08-17T10:00:00Z"}"#,
    );
    let status = env.try_import_story_usage().await;
    assert!(!status.success(), "import should fail on invalid marker");

    let database = support::connect_database().await;
    let paths = script_paths(&database, "background", "bg_test_1").await;
    assert_eq!(paths, vec!["activities/test/level_test_01_beg"]);

    // A transaction-level failure (duplicate PK) must roll back and keep the
    // previous snapshot readable.
    let duplicate = StoryUsageRow {
        script_path: "activities/test/level_test_01_beg".to_string(),
        resource_type: "background".to_string(),
        resource_id: "bg_test_1".to_string(),
        display_names: Vec::new(),
        sort_order: 0,
    };
    let result = database
        .replace_story_resource_usages(&[duplicate.clone(), duplicate])
        .await;
    assert!(
        result.is_err(),
        "duplicate rows must violate the primary key"
    );

    let paths = script_paths(&database, "background", "bg_test_1").await;
    assert_eq!(paths, vec!["activities/test/level_test_01_beg"]);
}

#[tokio::test]
#[ignore = "manual e2e test requiring docker, rc, and fixture assets"]
async fn gamedata_ready_watcher_imports_and_follows_latest() {
    let env = TestEnv::bootstrap_worker().await;
    publish_version(&env, V1_RES);
    let mut worker = support::spawn_worker(env.config_path(), 1);

    let database = support::connect_database().await;
    wait_for_usages(
        &database,
        "background",
        "bg_test_1",
        "activities/test/level_test_01_beg",
        Duration::from_mins(2),
    )
    .await
    .unwrap();

    // Switch latest to a new finished version; the watcher must re-resolve
    // the symlink and replace the snapshot.
    publish_version(&env, V2_RES);
    wait_for_usages(
        &database,
        "background",
        "bg_test_2",
        "obt/main/level_main_test",
        Duration::from_mins(3),
    )
    .await
    .unwrap();

    sleep(Duration::from_secs(2)).await;
    let old_paths = script_paths(&database, "background", "bg_test_1").await;
    assert!(
        old_paths.is_empty(),
        "old snapshot rows must be replaced after latest switch: {old_paths:?}"
    );

    let _ = worker.start_kill();
}
