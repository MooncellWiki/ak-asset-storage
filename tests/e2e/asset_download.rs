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
use serde_json::{Value, json};
use std::{
    collections::HashMap,
    fs,
    io::Write,
    path::{Path, PathBuf},
};
use tokio::{net::TcpListener, task::JoinHandle};
use tower_http::services::ServeDir;
use zip::{ZipWriter, write::SimpleFileOptions};

fn ab_info(name: &str, bytes: &[u8]) -> Value {
    json!({
        "name": name,
        "hash": "same-game-hash",
        "md5": hex::encode(Md5::digest(bytes)),
        "abSize": bytes.len(),
        "totalSize": bytes.len(),
    })
}

async fn create_version(database: &Database, res: &str, ab_infos: Vec<Value>) -> i32 {
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

/// `short.ab` gets a short MD5, which is never reused.
async fn create_assets_version(database: &Database, res: &str, assets: &[(&str, &[u8])]) -> i32 {
    let ab_infos = assets
        .iter()
        .map(|&(name, bytes)| {
            let mut info = ab_info(name, bytes);
            if name == "short.ab" {
                info["md5"] = json!("1234");
            }
            info
        })
        .collect();
    create_version(database, res, ab_infos).await
}

/// A download service whose asset CDN serves `directory`.
async fn download_service(
    env: &TestEnv,
    database: &Database,
    directory: PathBuf,
) -> (AssetDownloadService, JoinHandle<()>) {
    let settings: AppSettings =
        toml::from_str(&fs::read_to_string(env.config_path()).unwrap()).unwrap();
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
            axum::Router::new().fallback_service(ServeDir::new(directory)),
        )
        .await
        .unwrap();
    });
    (download, cdn_task)
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
    let old_id = create_assets_version(&database, "old", old_assets).await;
    let new_id = create_assets_version(&database, "new", new_assets).await;

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
    let (download, cdn_task) = download_service(&env, &database, cdn).await;

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

#[tokio::test]
#[ignore = "e2e: requires PostgreSQL, RustFS, rc and plocate; see e2e/README.md"]
async fn rejects_downloads_that_do_not_match_the_manifest() {
    let env = TestEnv::bootstrap_worker().await;
    let database = support::connect_database().await;

    // An IGNORE_MD5 entry carries a short signature and is checked by size.
    let mut anon = ab_info("anon.ab", b"anon");
    anon["md5"] = json!("155c");
    anon["meta"] = json!(1);
    let version_id =
        create_version(&database, "verify", vec![ab_info("full.ab", b"full"), anon]).await;

    let cdn = env.runtime_dir().join("cdn");
    let dir = cdn.join("verify");
    fs::create_dir_all(&dir).unwrap();
    write_asset(&dir, "full.ab", b"fulL");
    write_asset(&dir, "anon.ab", b"anon!");
    let (download, cdn_task) = download_service(&env, &database, cdn).await;

    let error = download.perform_download().await.unwrap_err();
    assert!(format!("{error:?}").contains("full.ab"), "{error:?}");
    assert!(bundle_file_ids(&database, version_id).await.is_empty());

    write_asset(&dir, "full.ab", b"full");
    let error = download.perform_download().await.unwrap_err();
    assert!(format!("{error:?}").contains("anon.ab"), "{error:?}");
    let partial = bundle_file_ids(&database, version_id).await;
    assert_eq!(partial.keys().collect::<Vec<_>>(), ["full.ab"]);
    assert!(
        !database
            .get_version_by_id(version_id)
            .await
            .unwrap()
            .unwrap()
            .is_ready
    );

    write_asset(&dir, "anon.ab", b"anon");
    assert!(download.perform_download().await.unwrap());
    assert!(!download.perform_download().await.unwrap());
    assert_eq!(bundle_file_ids(&database, version_id).await.len(), 2);
    env.assert_s3_state().await;
    cdn_task.abort();
    env.shutdown().await;
}
