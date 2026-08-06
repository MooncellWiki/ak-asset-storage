//! End-to-end coverage for the "worker detects a new version and launches a
//! Docker container" flow. The worker is started against a fake AK API that
//! reports `26-05-27-13-32-37_d44f28` / `2.7.41` as the latest version, and the
//! previous version `26-05-20-12-59-09_e8f456` / `2.7.31` is pre-seeded in the
//! database so that `check_and_save` has a `prev` version to forward.

use crate::support::{self, TestEnv};
use std::time::Duration;

/// `res_version` / `client_version` of the older fixture, used as the "previous"
/// version that gets forwarded to the launched container as `-c` / `-r`.
const PREV_RES_VERSION: &str = "26-05-20-12-59-09_e8f456";
const PREV_CLIENT_VERSION: &str = "2.7.31";

/// Latest version advertised by the fake AK API; this is what the worker
/// detects and passes as the first positional cmd arguments.
const REMOTE_RES_VERSION: &str = "26-05-27-13-32-37_d44f28";
const REMOTE_CLIENT_VERSION: &str = "2.7.41";

#[tokio::test]
#[ignore = "manual e2e test requiring docker, rc, and fixture assets"]
async fn worker_launches_container_on_new_version() {
    let env = TestEnv::bootstrap_worker_with_docker().await;

    // Pre-seed the previous version so check_and_save has a `prev` to forward.
    // `is_ready` does not affect container launch; the version just needs to be
    // the latest row before the worker detects the remote version.
    env.create_version_for_manifest_test(PREV_RES_VERSION, true)
        .await;

    let mut worker = support::spawn_worker(env.config_path(), 1);

    let container = env
        .wait_for_launched_container(Duration::from_secs(90))
        .await
        .expect("worker did not launch container within timeout");

    let _ = worker.start_kill();

    // launch_container builds cmd as
    //   [client_version, res_version, "-c", prev_client, "-r", prev_res_version]
    assert_eq!(
        container.cmd,
        [
            REMOTE_CLIENT_VERSION,
            REMOTE_RES_VERSION,
            "-c",
            PREV_CLIENT_VERSION,
            "-r",
            PREV_RES_VERSION,
        ]
    );
    // env_vars from the config are forwarded to the container.
    assert!(
        container
            .env
            .iter()
            .any(|var| var == "E2E_MARKER=launch_container_e2e"),
        "launched container env did not include the marker var: {:?}",
        container.env
    );
    // The image configured for the worker is the one that gets pulled and run.
    assert_eq!(container.image, "alpine:3.20");
}
