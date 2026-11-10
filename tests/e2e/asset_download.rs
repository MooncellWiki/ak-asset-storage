use crate::support::{self, TestEnv};
use ak_asset_storage::{
    config::{AkApiConfig, AppSettings},
    database::{
        Database,
        row::{AssetMappingStatus, VersionRow},
    },
    external::{ak_api::AkApi, notification::NotificationClient, s3::S3Storage},
    service::asset_download::AssetDownloadService,
};
use md5::{Digest, Md5};
use serde_json::json;
use std::{collections::HashMap, fs, io::Write, path::Path};
use tokio::net::TcpListener;
use tower_http::services::ServeDir;
use zip::{ZipWriter, write::SimpleFileOptions};

async fn create_version(database: &Database, res: &str, assets: &[(&str, &[u8])]) -> i32 {
    let ab_infos: Vec<_> = assets
        .iter()
        .map(|&(name, bytes)| {
            let md5 = if name == "short.ab" {
                "1234".to_string()
            } else {
                hex::encode(Md5::digest(bytes))
            };
            json!({
                "name": name,
                "hash": "same-game-hash",
                "md5": md5,
                "abSize": bytes.len(),
                "totalSize": bytes.len(),
            })
        })
        .collect();
    database
        .create_version(VersionRow {
            id: None,
            res: res.to_string(),
            client: "2.7.91".to_string(),
            is_ready: false,
            asset_mapping_status: AssetMappingStatus::Pending,
            hot_update_list: json!({ "abInfos": ab_infos }).to_string(),
        })
        .await
        .unwrap()
}

fn write_asset(directory: &Path, name: &str, bytes: &[u8]) {
    let path = directory.join(name).with_extension("dat");
    let mut zip = ZipWriter::new(fs::File::create(path).unwrap());
    zip.start_file(name, SimpleFileOptions::default()).unwrap();
    zip.write_all(bytes).unwrap();
    zip.finish().unwrap();
}

async fn bundle_file_ids(database: &Database, version_id: i32) -> HashMap<String, i32> {
    database
        .query_bundles_by_version_id(version_id)
        .await
        .unwrap()
        .into_iter()
        .map(|bundle| (bundle.path, bundle.file_id))
        .collect()
}

#[tokio::test]
#[ignore = "e2e: requires PostgreSQL, RustFS, rc and plocate; see e2e/README.md"]
async fn reuses_unchanged_bundle_before_download_and_retries_missing_assets() {
    let env = TestEnv::bootstrap_worker().await;
    let settings: AppSettings =
        toml::from_str(&fs::read_to_string(env.config_path()).unwrap()).unwrap();
    let database = support::connect_database().await;

    let old_assets: &[(&str, &[u8])] = &[
        ("unchanged.ab", b"unchanged"),
        ("changed.ab", b"before"),
        ("short.ab", b"short0"),
        ("deleted.ab", b"old only"),
    ];
    let new_assets: &[(&str, &[u8])] = &[
        ("unchanged.ab", b"unchanged"),
        ("changed.ab", b"after!"),
        ("short.ab", b"short1"),
        ("new.ab", b"new only"),
    ];
    let old_id = create_version(&database, "old", old_assets).await;
    let new_id = create_version(&database, "new", new_assets).await;

    let cdn = env.runtime_dir().join("cdn");
    let old_dir = cdn.join("old");
    let new_dir = cdn.join("new");
    fs::create_dir_all(&old_dir).unwrap();
    fs::create_dir_all(&new_dir).unwrap();
    for &(name, bytes) in old_assets {
        write_asset(&old_dir, name, bytes);
    }
    // The CDN has neither the unchanged AB nor the new one on the first try.
    for &(name, bytes) in &new_assets[1..3] {
        write_asset(&new_dir, name, bytes);
    }
    let listener = TcpListener::bind(("127.0.0.1", 0)).await.unwrap();
    let download = AssetDownloadService {
        database: database.clone(),
        ak_api: AkApi::new(&AkApiConfig {
            asset_url: format!("http://{}", listener.local_addr().unwrap()),
            conf_url: settings.ak.conf_url,
        })
        .unwrap(),
        notification: NotificationClient::Empty,
        storage: S3Storage::new(&settings.s3).unwrap(),
        concurrent: 1,
    };
    let cdn_task = tokio::spawn(async move {
        axum::serve(
            listener,
            axum::Router::new().fallback_service(ServeDir::new(cdn)),
        )
        .await
        .unwrap();
    });

    assert!(download.perform_download().await.unwrap());
    let old_files = bundle_file_ids(&database, old_id).await;
    assert_eq!(old_files.len(), old_assets.len());

    assert!(download.perform_download().await.is_err());
    let partial = bundle_file_ids(&database, new_id).await;
    assert_eq!(
        partial.get("unchanged.ab"),
        old_files.get("unchanged.ab"),
        "the unchanged AB must reuse its file_id even without a new CDN object"
    );
    assert_ne!(partial["changed.ab"], old_files["changed.ab"]);
    assert_ne!(partial["short.ab"], old_files["short.ab"]);
    assert_eq!(partial.len(), 3);
    assert!(
        !database
            .get_version_by_id(new_id)
            .await
            .unwrap()
            .unwrap()
            .is_ready
    );

    // Make any re-download of the completed entries fail during the retry.
    fs::remove_file(new_dir.join("changed.dat")).unwrap();
    fs::remove_file(new_dir.join("short.dat")).unwrap();
    write_asset(&new_dir, "new.ab", b"new only");
    assert!(download.perform_download().await.unwrap());
    assert!(!download.perform_download().await.unwrap());

    let mut new_files = bundle_file_ids(&database, new_id).await;
    assert!(new_files.remove("new.ab").is_some());
    assert_eq!(new_files, partial);
    assert_eq!(bundle_file_ids(&database, old_id).await, old_files);
    env.assert_s3_state().await;
    cdn_task.abort();
    env.shutdown().await;
}
