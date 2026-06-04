use crate::support::{self, TestEnv};
use std::time::Duration;

#[tokio::test]
#[ignore = "manual e2e test requiring docker, rc, and fixture assets"]
async fn worker_polls_new_version_and_downloads_assets() {
    let env = TestEnv::bootstrap_worker().await;
    let mut worker = support::spawn_worker(env.config_path(), 1).await;

    let database = support::connect_database().await;
    support::wait_for_ready_version(&database, Duration::from_secs(60))
        .await
        .unwrap();

    let _ = worker.start_kill();

    let versions = database.query_versions().await.unwrap();
    assert_eq!(versions.len(), 1);

    let bundles = database
        .query_bundles_by_version_id(versions.first().unwrap().id)
        .await
        .unwrap();
    assert_eq!(bundles.len(), 3);
    assert_eq!(
        bundles
            .iter()
            .map(|bundle| bundle.file_hash.as_str())
            .collect::<std::collections::HashSet<_>>()
            .len(),
        3
    );

    env.assert_s3_state().await;
}
