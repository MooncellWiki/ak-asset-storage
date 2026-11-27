use crate::support::{self, BundleDetails, TestEnv, VersionDetails, VersionSummary};
use ak_asset_storage::database::row::{AssetMappingStatus, VersionRow};
use std::{collections::HashMap, fs};

#[tokio::test]
#[ignore = "e2e: requires PostgreSQL, RustFS, rc and plocate; see e2e/README.md"]
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

    env.shutdown().await;
}

#[tokio::test]
#[ignore = "e2e: requires PostgreSQL, RustFS, rc and plocate; see e2e/README.md"]
async fn backfills_an_older_version_in_release_order() {
    let env = TestEnv::bootstrap().await;
    let database = support::connect_database().await;
    let older = &env.fixture.versions[0];
    let newer = &env.fixture.versions[1];
    let csv = env.runtime_dir().join("versions.csv");

    fs::write(
        &csv,
        format!("{},{}\n", newer.res_version, newer.client_version),
    )
    .unwrap();
    assert!(env.seed_command(&csv).status().await.unwrap().success());
    let latest_id = database
        .get_latest_version()
        .await
        .unwrap()
        .unwrap()
        .id
        .unwrap();

    let pending_id = database
        .create_version(VersionRow {
            id: None,
            res: "19-04-01-12-34-56-000000".into(),
            client: "0.8.0".into(),
            is_ready: false,
            asset_mapping_status: AssetMappingStatus::Pending,
            hot_update_list: "invalid manifest for an unrelated version".into(),
        })
        .await
        .unwrap();

    fs::write(
        &csv,
        format!("{},{}\r\n", older.res_version, older.client_version),
    )
    .unwrap();
    for _ in 0..2 {
        let status = env
            .seed_command(&csv)
            .arg("--backfill")
            .status()
            .await
            .unwrap();
        assert!(status.success(), "backfill command failed: {status}");
    }

    let (status, versions): (_, Vec<VersionSummary>) = env.get_json("/api/v1/version").await;
    assert_eq!(status, axum::http::StatusCode::OK);
    let [pending, historical, latest] = versions.as_slice() else {
        panic!("expected exactly three versions: {versions:?}");
    };
    assert_eq!(pending.id, pending_id);
    assert!(!pending.is_ready);
    assert_eq!(historical.res_version, older.res_version);
    assert!(historical.is_ready);
    assert_eq!(latest.id, latest_id);
    assert!(historical.id > latest.id, "the backfill was inserted later");

    for id in [historical.id, latest.id] {
        database
            .set_asset_mapping_status(id, AssetMappingStatus::Ready)
            .await
            .unwrap();
    }
    assert_eq!(
        database.get_latest_version().await.unwrap().unwrap().id,
        Some(latest_id)
    );
    assert_eq!(
        database
            .get_latest_ready_version()
            .await
            .unwrap()
            .unwrap()
            .id,
        Some(latest_id)
    );

    let bundles = env.get_all_bundle_pages("path=.ab").await;
    let expected = [(latest, newer), (historical, older)]
        .into_iter()
        .flat_map(|(version, fixture)| {
            fixture
                .bundle_names
                .iter()
                .map(|path| (version.id, path.as_str()))
        })
        .collect::<Vec<_>>();
    let actual = bundles
        .iter()
        .map(|bundle| (bundle.version_id, bundle.path.as_str()))
        .collect::<Vec<_>>();
    assert_eq!(actual, expected);

    env.shutdown().await;
}
