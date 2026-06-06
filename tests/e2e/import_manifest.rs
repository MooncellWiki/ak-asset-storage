use crate::support::{self, TestEnv};
use ak_asset_storage::database::row::AssetMappingStatus;
use std::time::Duration;

const RES_VERSION: &str = "26-05-27-13-32-37_d44f28";

#[tokio::test]
#[ignore = "manual e2e test requiring docker, rc, and fixture assets"]
async fn import_manifest_command_imports_asset_mappings() {
    let env = TestEnv::bootstrap_worker().await;
    let version_id = env
        .create_version_for_manifest_test(RES_VERSION, false)
        .await;
    env.copy_manifest_fixture(RES_VERSION);

    env.run_import_manifest(RES_VERSION).await;

    let database = support::connect_database().await;
    support::wait_for_asset_mapping_status(
        &database,
        RES_VERSION,
        AssetMappingStatus::Ready,
        Duration::from_secs(5),
    )
    .await
    .unwrap();

    // Spawn worker to download bundles (seed logic)
    let mut worker = support::spawn_worker(env.config_path(), 1).await;
    support::wait_for_ready_version(&database, Duration::from_secs(60))
        .await
        .unwrap();
    let _ = worker.start_kill();

    support::assert_manifest_fixture_imported(&database, version_id).await;
}
