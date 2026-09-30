use crate::support::{BundleDetails, TestEnv, VersionDetails, VersionSummary};
use std::collections::{HashMap, HashSet};

#[tokio::test]
#[ignore = "manual e2e test requiring docker, rc, and fixture assets"]
async fn seed_two_versions_then_query_real_server() {
    let env = TestEnv::bootstrap().await;

    env.run_seed().await;

    let (status, version_list): (_, Vec<VersionSummary>) = env.get_json("/api/v1/version").await;
    assert_eq!(status, axum::http::StatusCode::OK);
    assert_eq!(version_list.len(), env.fixture.versions.len());

    let expected_by_res = env
        .fixture
        .versions
        .iter()
        .map(|version| (version.res_version.as_str(), version))
        .collect::<HashMap<_, _>>();

    for version in &version_list {
        let expected = expected_by_res.get(version.res_version.as_str()).unwrap();
        assert_eq!(version.client_version, expected.client_version);
        assert!(version.is_ready);

        let (status, version_detail): (_, VersionDetails) = env
            .get_json(&format!("/api/v1/version/{}", version.id))
            .await;
        assert_eq!(status, axum::http::StatusCode::OK);
        assert_eq!(version_detail.hot_update_list, expected.hot_update_list);

        let (status, bundles): (_, Vec<BundleDetails>) = env
            .get_json(&format!("/api/v1/version/{}/files", version.id))
            .await;
        assert_eq!(status, axum::http::StatusCode::OK);
        assert_eq!(bundles.len(), expected.bundle_names.len());

        let mut bundle_paths = bundles
            .iter()
            .map(|bundle| bundle.path.clone())
            .collect::<Vec<_>>();
        bundle_paths.sort();
        assert_eq!(bundle_paths, expected.bundle_names);
        assert!(bundle_paths.iter().any(|path| path.contains('#')));
        assert!(bundle_paths.iter().any(|path| path.contains('/')));

        let query_bundles = env
            .get_all_bundle_pages(&format!("version={}", version.id))
            .await;
        assert_eq!(query_bundles.len(), bundles.len());

        for bundle in &query_bundles {
            let (status, detail): (_, BundleDetails) =
                env.get_json(&format!("/api/v1/bundle/{}", bundle.id)).await;
            assert_eq!(status, axum::http::StatusCode::OK);
            assert_eq!(detail.id, bundle.id);
            assert_eq!(detail.path, bundle.path);
            assert_eq!(detail.file_id, bundle.file_id);
            assert_eq!(detail.file_hash, bundle.file_hash);
            assert_eq!(detail.file_size, bundle.file_size);
            assert_eq!(detail.version_id, version.id);
            assert_eq!(detail.version_res, version.res_version);
            assert_eq!(detail.version_client, version.client_version);
            assert!(detail.version_is_ready);
        }
    }

    // The unfiltered dump is rejected; the whole-table invariants are
    // asserted against the database by assert_database_state below.
    let (status, body) = env.get_text("/api/v1/bundle").await;
    assert_eq!(status, axum::http::StatusCode::BAD_REQUEST);
    assert!(body.contains("provide at least one"));

    env.assert_database_state().await;
    env.assert_s3_state().await;
}
