use ak_asset_storage::{
    config::{DatabaseConfig, LauncherConfig},
    database::{Database, launcher::LauncherEntry},
    external::launcher::{decrypt_manifest, file_url, parse_latest, parse_manifest},
};
use md5::{Digest, Md5};
use sqlx::PgPool;

const OLD: &str = include_str!("fixtures/launcher/old.jsonl");
const NEW: &str = include_str!("fixtures/launcher/new.jsonl");

#[tokio::test]
#[ignore = "reads the public official-launcher API and encrypted manifest only"]
async fn validates_live_official_manifest() {
    let _ = rustls::crypto::ring::default_provider().install_default();
    let client =
        ak_asset_storage::external::launcher::LauncherClient::new(&LauncherConfig::default())
            .unwrap();
    let publication = client.latest().await.unwrap();
    let (_, entries) = client.manifest(&publication).await.unwrap();
    assert!(entries.len() > 1000);
    assert!(entries.iter().any(|entry| entry.path.ends_with(".dll")));
    assert!(entries.iter().any(|entry| entry.path.ends_with(".exe")));
    println!(
        "package={} game={} files={} bytes={}",
        publication.package_version,
        publication.game_version,
        entries.len(),
        entries.iter().map(|e| e.size).sum::<i64>()
    );
}

#[test]
fn launcher_protocol_and_ciphertext_validation() {
    let publication =
        parse_latest(serde_json::from_str(include_str!("fixtures/launcher/latest.json")).unwrap())
            .unwrap();
    assert_eq!(publication.package_version, "79.0.0");
    assert_eq!(publication.game_version, "2.7.91");
    let config = LauncherConfig::default();
    // Generated independently with OpenSSL AES-256-CBC, with PKCS7 padding.
    let encrypted = include_bytes!("fixtures/launcher/game_files");
    let md5 = hex::encode(Md5::digest(encrypted));
    assert_eq!(
        decrypt_manifest(encrypted, &md5, &config.aes_key, &config.aes_iv).unwrap(),
        parse_manifest(OLD).unwrap()
    );
    assert!(decrypt_manifest(encrypted, &"0".repeat(32), &config.aes_key, &config.aes_iv).is_err());
    let mut truncated = encrypted.to_vec();
    truncated.pop();
    assert!(
        decrypt_manifest(
            &truncated,
            &hex::encode(Md5::digest(&truncated)),
            &config.aes_key,
            &config.aes_iv
        )
        .is_err()
    );
    assert!(parse_latest(serde_json::json!({"proxy_rsps": []})).is_err());
}

#[test]
fn rejects_invalid_manifests_and_preserves_literal_paths() {
    assert!(parse_manifest("").is_err());
    assert!(parse_manifest(&format!("{OLD}{OLD}")).is_err());
    for path in [
        "../escape",
        "/absolute",
        "bad/../file",
        "bad\\file",
        "bad//file",
    ] {
        let line = serde_json::json!({"path": path, "md5": "0".repeat(32), "size": 1}).to_string();
        assert!(parse_manifest(&line).is_err(), "{path}");
    }
    let url = file_url("https://example.com/files/", "目录/a #%.dll").unwrap();
    assert_eq!(
        url.as_str(),
        "https://example.com/files/%E7%9B%AE%E5%BD%95/a%20%23%25.dll"
    );
    assert!(parse_manifest(r#"{"path":"x","md5":"invalid","size":1}"#).is_err());
    assert!(
        parse_manifest(&serde_json::json!({"path":"x","md5":"0".repeat(32),"size":-1}).to_string())
            .is_err()
    );
}

async fn database(pool: &PgPool) -> Database {
    let mut url = reqwest::Url::parse(
        &std::env::var("DATABASE_URL").expect("DATABASE_URL for launcher database tests"),
    )
    .unwrap();
    url.set_path(pool.connect_options().get_database().unwrap());
    Database::connect(&DatabaseConfig {
        uri: url.to_string(),
        max_connections: Some(10),
        connection_timeout_seconds: Some(5),
    })
    .await
    .unwrap()
}

async fn import(database: &Database, version: &str, manifest: &str) -> i64 {
    database
        .import_launcher_release(
            version,
            "2.7.91",
            "https://example.com/files",
            &hex::encode(Md5::digest(manifest)),
            manifest.as_bytes(),
            &parse_manifest(manifest).unwrap(),
        )
        .await
        .unwrap()
}

#[sqlx::test]
async fn publications_diffs_reuse_and_interrupted_claims(pool: PgPool) {
    let db = database(&pool).await;
    let old = import(&db, "78.0.0", OLD).await;
    assert_eq!(import(&db, "78.0.0", OLD).await, old);
    let new = import(&db, "79.0.0", NEW).await;
    let reissue = import(&db, "79.0.0", OLD).await;
    assert_ne!(reissue, new);
    assert_eq!(db.list_launcher_releases(None, 100).await.unwrap().len(), 3);
    let changes = db.launcher_diff(old, new, "", 100).await.unwrap();
    assert_eq!(
        changes
            .iter()
            .map(|c| (c.path.as_str(), c.change.as_str()))
            .collect::<Vec<_>>(),
        [
            ("added.exe", "added"),
            ("changed.ab", "modified"),
            ("new-name.dll", "added"),
            ("old-name.dll", "deleted"),
            ("removed.exe", "deleted"),
        ]
    );
    let first_page = db.launcher_diff(old, new, "", 2).await.unwrap();
    assert_eq!(
        db.launcher_diff(old, new, &first_page.last().unwrap().path, 100)
            .await
            .unwrap()
            .len(),
        3
    );
    assert!(
        db.get_launcher_release(new)
            .await
            .unwrap()
            .unwrap()
            .ready_at
            .is_none()
    );
    // An interrupted claim keeps its blob pending and immediately reusable.
    let interrupted = db.claim_launcher_blob().await.unwrap().unwrap();
    let interrupted_md5 = interrupted.md5.clone();
    interrupted.transaction.rollback().await.unwrap();
    let reclaimed = db.claim_launcher_blob().await.unwrap().unwrap();
    assert_eq!(reclaimed.md5, interrupted_md5);
    let parallel = db.claim_launcher_blob().await.unwrap().unwrap();
    assert_ne!(parallel.md5, reclaimed.md5);
    parallel.transaction.rollback().await.unwrap();
    reclaimed.transaction.rollback().await.unwrap();
    let mut count = 0;
    while let Some(job) = db.claim_launcher_blob().await.unwrap() {
        job.complete(&"a".repeat(64)).await.unwrap();
        count += 1;
    }
    assert_eq!(
        count, 6,
        "unchanged and renamed content have one blob across all releases"
    );
    db.refresh_launcher_ready().await.unwrap();
    for id in [old, new, reissue] {
        let release = db.get_launcher_release(id).await.unwrap().unwrap();
        assert!(release.ready_at.is_some());
        assert_eq!(release.total_files, release.archived_files);
        assert_eq!(release.total_bytes, release.archived_bytes);
    }
    assert_eq!(
        db.launcher_file(old, "old-name.dll")
            .await
            .unwrap()
            .unwrap()
            .md5,
        db.launcher_file(new, "new-name.dll")
            .await
            .unwrap()
            .unwrap()
            .md5
    );
    assert_eq!(
        db.launcher_manifest(old).await.unwrap().unwrap(),
        OLD.as_bytes()
    );
    // A failed transaction cannot expose a partial manifest or orphan release.
    let mut duplicate: Vec<LauncherEntry> = parse_manifest(OLD).unwrap();
    duplicate.push(duplicate[0].clone());
    let md5 = "b".repeat(32);
    assert!(
        db.import_launcher_release(
            "broken",
            "game",
            "https://example.com",
            &md5,
            b"ciphertext",
            &duplicate
        )
        .await
        .is_err()
    );
    assert!(!db.launcher_release_exists("broken", &md5).await.unwrap());
}

mod archival {
    use super::*;
    use aes::cipher::{BlockModeEncrypt, KeyIvInit, block_padding::Pkcs7};
    use ak_asset_storage::{
        api::{AppState, build_router},
        config::{AppSettings, S3Config},
        external::{launcher::LauncherClient, s3::S3Storage},
        service::launcher_archive::LauncherArchiveService,
    };
    use axum::{
        Json, Router,
        extract::{Path, State},
        http::StatusCode,
        response::{IntoResponse, Response},
        routing::{get, post},
    };
    use sqlx::ConnectOptions;
    use std::{
        collections::HashMap,
        sync::{
            Arc, Mutex,
            atomic::{AtomicBool, Ordering},
        },
        time::Duration,
    };
    use tokio::{net::TcpListener, task::JoinHandle};

    struct Upstream {
        latest: Mutex<serde_json::Value>,
        files: HashMap<String, Vec<u8>>,
        requests: Mutex<HashMap<String, usize>>,
        corrupt: AtomicBool,
        stall: AtomicBool,
    }

    async fn latest(
        State(state): State<Arc<Upstream>>,
        Json(request): Json<serde_json::Value>,
    ) -> Json<serde_json::Value> {
        assert_eq!(
            request["proxy_reqs"][0]["get_latest_game_req"]["version"],
            ""
        );
        Json(state.latest.lock().unwrap().clone())
    }

    async fn file(State(state): State<Arc<Upstream>>, Path(path): Path<String>) -> Response {
        *state
            .requests
            .lock()
            .unwrap()
            .entry(path.clone())
            .or_default() += 1;
        if path == "new/changed.ab" {
            if state.stall.load(Ordering::SeqCst) {
                return std::future::pending().await;
            }
            if state.corrupt.load(Ordering::SeqCst) {
                return b"wrong".to_vec().into_response();
            }
        }
        state.files.get(&path).map_or_else(
            || StatusCode::NOT_FOUND.into_response(),
            |data| data.clone().into_response(),
        )
    }

    async fn serve(router: Router) -> (String, JoinHandle<()>) {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = format!("http://{}", listener.local_addr().unwrap());
        let task = tokio::spawn(async move {
            axum::serve(listener, router).await.unwrap();
        });
        (address, task)
    }

    fn encrypt(manifest: &str) -> Vec<u8> {
        let config = LauncherConfig::default();
        cbc::Encryptor::<aes::Aes256>::new_from_slices(
            &hex::decode(config.aes_key).unwrap(),
            &hex::decode(config.aes_iv).unwrap(),
        )
        .unwrap()
        .encrypt_padded_vec::<Pkcs7>(manifest.as_bytes())
    }

    fn publication(
        base: &str,
        version: &str,
        directory: &str,
        ciphertext: &[u8],
    ) -> serde_json::Value {
        serde_json::json!({"proxy_rsps":[{"kind":"get_latest_game","get_latest_game_rsp":{
            "version":version,"client_version":"2.7.91","pkg":{"file_path":format!("{base}/{directory}"),"game_files_md5":hex::encode(Md5::digest(ciphertext))}
        }}]})
    }

    #[sqlx::test]
    async fn imports_historical_manifests_in_batch(pool: PgPool) {
        let db = database(&pool).await;
        let local_ciphertext = include_bytes!("fixtures/launcher/game_files");
        let remote_ciphertext = encrypt(NEW);
        let served_ciphertext = remote_ciphertext.clone();
        let (base, upstream) = serve(
            Router::new().route("/game_files", get(move || async move { served_ciphertext })),
        )
        .await;

        let temp = tempfile::tempdir().unwrap();
        let config = temp.path().join("config.toml");
        let local_metadata = temp.path().join("old.meta.json");
        let remote_metadata = temp.path().join("new.meta.json");
        let local_game_files = temp.path().join("game_files");
        std::fs::write(
            &config,
            include_str!("../example.toml").replace(
                "https://your-sentry-dsn@sentry.io/project-id",
                "https://test@127.0.0.1:9/1",
            ),
        )
        .unwrap();
        std::fs::write(&local_game_files, local_ciphertext).unwrap();
        let metadata = |version: &str, md5: &str| {
            serde_json::json!({
                "version": version,
                "client_version": "2.7.91",
                "file_path": base,
                "game_files_md5": md5,
            })
        };
        let mut local = metadata("78.0.0", &hex::encode_upper(Md5::digest(local_ciphertext)));
        local["game_files_path"] = "game_files".into();
        std::fs::write(&local_metadata, local.to_string()).unwrap();
        let run = || {
            let mut command = tokio::process::Command::new(env!("CARGO_BIN_EXE_ak-asset-storage"));
            command
                .env(
                    "DATABASE_URI",
                    db.pool().connect_options().to_url_lossy().as_str(),
                )
                .args(["import-launcher-manifest", "-c"])
                .args([&config, &local_metadata, &remote_metadata]);
            command
        };

        std::fs::write(
            &remote_metadata,
            metadata("79.0.0", &"0".repeat(32)).to_string(),
        )
        .unwrap();
        let failed = run().output().await.unwrap();
        assert!(!failed.status.success());
        assert!(String::from_utf8_lossy(&failed.stderr).contains("launcher manifest MD5 mismatch"));
        let releases = db.list_launcher_releases(None, 100).await.unwrap();
        assert_eq!(releases.len(), 1);
        let old = &releases[0];
        assert_eq!(old.package_version, "78.0.0");
        assert_eq!(
            db.launcher_manifest(old.id).await.unwrap().unwrap(),
            local_ciphertext
        );

        std::fs::remove_file(local_game_files).unwrap();
        let remote = metadata("79.0.0", &hex::encode(Md5::digest(&remote_ciphertext)));
        std::fs::write(&remote_metadata, remote.to_string()).unwrap();
        let resumed = run().output().await.unwrap();
        assert!(
            resumed.status.success(),
            "{}",
            String::from_utf8_lossy(&resumed.stderr)
        );
        let releases = db.list_launcher_releases(None, 100).await.unwrap();
        assert_eq!(releases.len(), 2);
        let new = &releases[0];
        assert_eq!(new.package_version, "79.0.0");
        assert_eq!(new.total_files, 4);
        assert_eq!(new.archived_files, 0);
        assert_eq!(
            db.launcher_manifest(new.id).await.unwrap().unwrap(),
            remote_ciphertext
        );
        upstream.abort();
    }

    /// This test uses a disposable SQLx database, a fake launcher, and the
    /// existing local development S3 bucket; no real game files are downloaded.
    #[sqlx::test]
    #[ignore = "requires development S3 on localhost:31000 (arknights-assets, torappu/torappu123)"]
    async fn archives_raw_bytes_and_resumes_only_missing_files(pool: PgPool) {
        let _ = rustls::crypto::ring::default_provider().install_default();
        let db = database(&pool).await;
        let old_ciphertext = encrypt(OLD);
        let new_ciphertext = encrypt(NEW);
        let mut files = HashMap::from([
            ("old/game_files".into(), old_ciphertext.clone()),
            ("new/game_files".into(), new_ciphertext.clone()),
            ("old/unchanged.ab".into(), b"shared".to_vec()),
            ("old/old-name.dll".into(), b"renamed".to_vec()),
            ("old/removed.exe".into(), b"deleted".to_vec()),
            ("old/changed.ab".into(), b"before".to_vec()),
            ("new/unchanged.ab".into(), b"shared".to_vec()),
            ("new/new-name.dll".into(), b"renamed".to_vec()),
            ("new/added.exe".into(), b"added".to_vec()),
            ("new/changed.ab".into(), b"after".to_vec()),
        ]);
        // Exercise URL encoding and zero-byte objects as well.
        let extra = format!(
            "{NEW}{{\"path\":\"目录/a #%.dll\",\"md5\":\"d41d8cd98f00b204e9800998ecf8427e\",\"size\":0}}\n"
        );
        let next_ciphertext = encrypt(&extra);
        files.insert("next/game_files".into(), next_ciphertext.clone());
        files.insert("next/目录/a #%.dll".into(), Vec::new());
        files.insert("next/changed.ab".into(), b"after".to_vec());
        let upstream = Arc::new(Upstream {
            latest: Mutex::new(serde_json::Value::Null),
            files,
            requests: Mutex::new(HashMap::new()),
            corrupt: AtomicBool::new(false),
            stall: AtomicBool::new(false),
        });
        let (base, upstream_task) = serve(
            Router::new()
                .route("/latest", post(latest))
                .route("/{*path}", get(file))
                .with_state(upstream.clone()),
        )
        .await;
        *upstream.latest.lock().unwrap() = publication(&base, "78.0.0", "old", &old_ciphertext);
        let config = LauncherConfig {
            latest_url: format!("{base}/latest"),
            retry_interval_seconds: 1,
            ..LauncherConfig::default()
        };
        let storage_config = S3Config {
            endpoint: "http://127.0.0.1:31000".into(),
            bucket_name: std::env::var("LAUNCHER_TEST_BUCKET")
                .unwrap_or_else(|_| "arknights-assets".into()),
            access_key_id: "torappu".into(),
            secret_access_key: "torappu123".into(),
            with_virtual_hosted_style_request: false,
        };
        let service = LauncherArchiveService {
            database: db.clone(),
            client: LauncherClient::new(&config).unwrap(),
            storage: S3Storage::new(&storage_config).unwrap(),
            concurrency: 2,
            retry_interval_seconds: 1,
        };
        service.discover().await.unwrap();
        service.discover().await.unwrap();
        assert!(
            db.query_versions().await.unwrap().is_empty(),
            "PC discovery must not create Android versions"
        );
        let old = db.list_launcher_releases(None, 100).await.unwrap()[0].id;
        assert_eq!(upstream.requests.lock().unwrap()["old/game_files"], 1);
        service.archive_pending().await.unwrap();
        assert!(
            db.get_launcher_release(old)
                .await
                .unwrap()
                .unwrap()
                .ready_at
                .is_some()
        );
        *upstream.latest.lock().unwrap() = publication(&base, "79.0.0", "new", &new_ciphertext);
        upstream.corrupt.store(true, Ordering::SeqCst);
        service.discover().await.unwrap();
        let new = db.list_launcher_releases(None, 100).await.unwrap()[0].id;
        assert_eq!(db.launcher_diff(old, new, "", 100).await.unwrap().len(), 5);
        service.archive_pending().await.unwrap();
        let progress = db.get_launcher_release(new).await.unwrap().unwrap();
        assert_eq!(progress.archived_files, 3);
        assert!(progress.ready_at.is_none());
        assert!(
            db.launcher_file(new, "changed.ab")
                .await
                .unwrap()
                .unwrap()
                .sha256
                .is_none()
        );
        // An in-flight download cannot block discovery of another release.
        tokio::time::sleep(Duration::from_millis(1100)).await;
        upstream.stall.store(true, Ordering::SeqCst);
        let running_service = service.clone();
        let running = tokio::spawn(async move { running_service.archive_pending().await });
        tokio::time::timeout(Duration::from_secs(5), async {
            while upstream
                .requests
                .lock()
                .unwrap()
                .get("new/changed.ab")
                .copied()
                .unwrap_or(0)
                < 2
            {
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        })
        .await
        .unwrap();
        *upstream.latest.lock().unwrap() = publication(&base, "80.0.0", "next", &next_ciphertext);
        service.discover().await.unwrap();
        assert_eq!(db.list_launcher_releases(None, 100).await.unwrap().len(), 3);
        running.abort();
        assert!(running.await.unwrap_err().is_cancelled());
        upstream.stall.store(false, Ordering::SeqCst);
        upstream.corrupt.store(false, Ordering::SeqCst);

        let mut settings: AppSettings = toml::from_str(include_str!("../example.toml")).unwrap();
        settings.database.uri = db.pool().connect_options().to_url_lossy().to_string();
        settings.s3 = storage_config;
        settings.torappu.kubernetes = None;
        let temp = tempfile::tempdir().unwrap();
        settings.torappu.asset_base_path = temp.path().to_str().unwrap().into();
        settings.torappu.plocate.database_path =
            Some(temp.path().with_extension("db").to_str().unwrap().into());
        let state = AppState::from_settings(Arc::new(settings)).await.unwrap();
        let (api, api_task) = serve(build_router(state)).await;
        let http = reqwest::Client::new();
        assert_eq!(
            http.get(format!(
                "{api}/api/v1/launcher/releases/{new}/download?path=changed.ab"
            ))
            .send()
            .await
            .unwrap()
            .status(),
            StatusCode::SERVICE_UNAVAILABLE
        );
        let renamed = http
            .get(format!(
                "{api}/api/v1/launcher/releases/{new}/download?path=new-name.dll"
            ))
            .send()
            .await
            .unwrap();
        assert_eq!(renamed.bytes().await.unwrap().as_ref(), b"renamed");

        // Reconstruct service state after cancellation; only uncommitted
        // contents are fetched, including the single file in the next release.
        let restarted = LauncherArchiveService {
            client: LauncherClient::new(&config).unwrap(),
            ..service
        };
        restarted.archive_pending().await.unwrap();
        let hash = sha256::digest(b"before");
        let key = format!(
            "launcher/blobs/{}/{}/{}",
            &hash[..2],
            &hash[2..4],
            &hash[4..]
        );
        let stored = restarted.storage.download(&key).await.unwrap();
        assert_eq!(stored.bytes().await.unwrap().as_ref(), b"before");

        let legacy_bytes = b"legacy launcher blob fixture";
        let legacy_hash = sha256::digest(legacy_bytes);
        restarted
            .storage
            .upload(&format!("launcher/blobs/{legacy_hash}"), legacy_bytes)
            .await
            .unwrap();
        let legacy = restarted
            .storage
            .download_launcher_blob(&legacy_hash)
            .await
            .unwrap();
        assert_eq!(legacy.bytes().await.unwrap().as_ref(), legacy_bytes);

        for release in db.list_launcher_releases(None, 100).await.unwrap() {
            assert!(release.ready_at.is_some());
        }
        let requests = upstream.requests.lock().unwrap().clone();
        assert!(!requests.contains_key("new/unchanged.ab"));
        assert!(!requests.contains_key("new/new-name.dll"));
        assert_eq!(requests["new/added.exe"], 1);
        assert_eq!(requests["old/changed.ab"], 1);
        assert_eq!(requests["next/changed.ab"], 1);
        for (id, path, expected) in [
            (old, "changed.ab", b"before".as_slice()),
            (new, "changed.ab", b"after".as_slice()),
            (old, "old-name.dll", b"renamed".as_slice()),
        ] {
            let mut url =
                reqwest::Url::parse(&format!("{api}/api/v1/launcher/releases/{id}/download"))
                    .unwrap();
            url.query_pairs_mut().append_pair("path", path);
            let response = http.get(url).send().await.unwrap();
            assert_eq!(response.status(), StatusCode::OK);
            assert_eq!(response.content_length(), Some(expected.len() as u64));
            let bytes = response.bytes().await.unwrap();
            assert_eq!(bytes.as_ref(), expected);
            assert_eq!(
                hex::encode(Md5::digest(&bytes)),
                db.launcher_file(id, path).await.unwrap().unwrap().md5
            );
        }
        assert_eq!(
            http.get(format!("{api}/api/v1/launcher/releases/{old}/manifest"))
                .send()
                .await
                .unwrap()
                .bytes()
                .await
                .unwrap()
                .as_ref(),
            old_ciphertext
        );
        assert_eq!(
            http.get(format!(
                "{api}/api/v1/launcher/diff?from={old}&to={new}&limit=0"
            ))
            .send()
            .await
            .unwrap()
            .status(),
            StatusCode::BAD_REQUEST
        );
        assert_eq!(
            http.get(format!("{api}/api/v1/launcher/releases/999999/files"))
                .send()
                .await
                .unwrap()
                .status(),
            StatusCode::NOT_FOUND
        );
        api_task.abort();
        upstream_task.abort();
    }
}
