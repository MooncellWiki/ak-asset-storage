use crate::support::{self, TestEnv};
use ak_asset_storage::database::row::AssetMappingStatus;
use std::time::Duration;
use tokio::time::sleep;

const RES_VERSION: &str = "26-05-27-13-32-37_d44f28";

#[tokio::test]
#[ignore = "manual e2e test requiring docker, rc, and fixture assets"]
async fn manifest_watcher_imports_new_manifest_file() {
    let env = TestEnv::bootstrap_worker().await;
    let version_id = env
        .create_version_for_manifest_test(RES_VERSION, false)
        .await;
    let mut worker = support::spawn_worker(env.config_path(), 1).await;

    sleep(Duration::from_secs(2)).await;
    env.copy_manifest_fixture(RES_VERSION);

    let database = support::connect_database().await;
    let mapping_result = support::wait_for_asset_mapping_status(
        &database,
        RES_VERSION,
        AssetMappingStatus::Ready,
        Duration::from_secs(90),
    )
    .await;

    // Wait for bundles to be downloaded (seed logic)
    let ready_result = support::wait_for_ready_version(&database, Duration::from_secs(60)).await;
    let _ = worker.start_kill();

    mapping_result.unwrap();
    ready_result.unwrap();
    support::assert_manifest_fixture_imported(&database, version_id).await;
}
