use crate::support::{BundleDetails, TestEnv, VersionDetails, VersionSummary};
use std::collections::{HashMap, HashSet};

#[tokio::test]
#[ignore = "manual e2e test requiring docker, rc, and fixture assets"]
async fn seed_two_versions_then_query_real_server() {
    let env = TestEnv::bootstrap().await;

    env.run_seed().await;

    let version_list: Vec<VersionSummary> = env.get_json("/api/v1/version").await;
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

        let version_detail: VersionDetails = env
            .get_json(&format!("/api/v1/version/{}", version.id))
            .await;
        assert_eq!(version_detail.hot_update_list, expected.hot_update_list);

        let bundles: Vec<BundleDetails> = env
            .get_json(&format!("/api/v1/version/{}/files", version.id))
            .await;
        assert_eq!(bundles.len(), expected.bundle_names.len());

        let mut bundle_paths = bundles
            .iter()
            .map(|bundle| bundle.path.clone())
            .collect::<Vec<_>>();
        bundle_paths.sort();
        assert_eq!(bundle_paths, expected.bundle_names);
        assert!(bundle_paths.iter().any(|path| path.contains('#')));
        assert!(bundle_paths.iter().any(|path| path.contains('/')));

        let query_bundles: Vec<BundleDetails> = env
            .get_json(&format!("/api/v1/bundle?version={}", version.id))
            .await;
        assert_eq!(query_bundles.len(), bundles.len());

        for bundle in &query_bundles {
            let detail: BundleDetails =
                env.get_json(&format!("/api/v1/bundle/{}", bundle.id)).await;
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

    let all_bundles: Vec<BundleDetails> = env.get_json("/api/v1/bundle").await;
    assert_eq!(all_bundles.len(), env.fixture.all_bundle_names.len());

    let unique_hashes = all_bundles
        .iter()
        .map(|bundle| bundle.file_hash.clone())
        .collect::<HashSet<_>>();
    assert_eq!(unique_hashes.len(), 3);

    let bundle_count_by_hash = all_bundles.iter().fold(HashMap::new(), |mut acc, bundle| {
        *acc.entry(bundle.file_hash.clone()).or_insert(0) += 1;
        acc
    });
    assert_eq!(bundle_count_by_hash.len(), 3);
    assert!(bundle_count_by_hash.values().all(|count| *count == 2));

    let file_id_by_hash = all_bundles.iter().fold(HashMap::new(), |mut acc, bundle| {
        acc.entry(bundle.file_hash.clone())
            .or_insert_with(HashSet::new)
            .insert(bundle.file_id);
        acc
    });
    assert!(file_id_by_hash.values().all(|ids| ids.len() == 1));

    env.assert_database_state().await;
    env.assert_s3_state().await;
}
