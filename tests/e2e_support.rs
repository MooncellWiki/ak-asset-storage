use ak_asset_storage::database::Database;
use axum::{
    Router,
    extract::{Path, State},
    http::StatusCode,
    routing::get,
};
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use std::{
    collections::HashMap,
    fs,
    path::{Path as StdPath, PathBuf},
    process::Stdio,
    sync::Arc,
    time::{Duration, Instant},
};
use tokio::{
    net::TcpListener,
    process::{Child, Command},
    task::JoinHandle,
    time::sleep,
};

const SERVER_PORT: u16 = 25150;
const FAKE_AK_PORT: u16 = 25151;
const BUCKET_NAME: &str = "ak-asset-storage-e2e";
const RC_ALIAS_NAME: &str = "ak-asset-storage-e2e";
const DATABASE_NAME: &str = "ak_asset_storage_e2e";
const DATABASE_URI: &str = "postgres://ak:ak@localhost:25432/ak_asset_storage_e2e";
const POSTGRES_ADMIN_URI: &str = "postgres://ak:ak@localhost:25432/postgres";
const EXPECTED_UNIQUE_HASHES: [&str; 3] = [
    "100d6f5e408d3b70785b4d736616293a61480431d57a6789c3be640febe11442",
    "92c139ea3f7fdf34777723b0bb8b194e4112c6ce4aa34364cd5cb3acc7b9f7bc",
    "e7997e26db3e1957dfbd96b84d11a89d5c484422850fb83e591727a9ebb9e7c4",
];

#[derive(Debug, Clone)]
pub struct FixtureVersion {
    pub root: PathBuf,
    pub hot_update_list: String,
    pub res_version: String,
    pub client_version: String,
    pub bundle_names: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct Fixture {
    pub versions: Vec<FixtureVersion>,
    pub all_bundle_names: Vec<String>,
}

#[derive(Debug)]
pub struct TestEnv {
    pub fixture: Fixture,
    runtime_dir: PathBuf,
    config_path: PathBuf,
    client: reqwest::Client,
    fake_ak_task: JoinHandle<()>,
    server: Child,
}

#[derive(Debug, Clone)]
struct FakeAkState {
    versions: HashMap<String, FixtureVersion>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VersionSummary {
    pub id: i32,
    pub client_version: String,
    pub res_version: String,
    pub is_ready: bool,
    pub asset_mapping_status: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VersionDetails {
    pub id: i32,
    pub client_version: String,
    pub res_version: String,
    pub is_ready: bool,
    pub hot_update_list: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BundleDetails {
    pub id: i32,
    pub path: String,
    pub file_id: i32,
    pub file_hash: String,
    pub file_size: i32,
    pub version_id: i32,
    pub version_res: String,
    pub version_client: String,
    pub version_is_ready: bool,
}

impl TestEnv {
    pub async fn bootstrap() -> Self {
        install_rustls_provider();
        let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let runtime_dir = repo_root.join("e2e/runtime");
        recreate_dir(&runtime_dir);

        let asset_dir = runtime_dir.join("asset");
        fs::create_dir_all(&asset_dir).unwrap();

        let fixture = load_fixture(&repo_root);
        ensure_dependencies_ready(&repo_root).await;
        recreate_bucket(&repo_root).await;

        let fake_ak_task = spawn_fake_ak_server(fixture.clone()).await;
        let config_path = write_config(&runtime_dir, &asset_dir).unwrap();
        let server = spawn_server(&config_path).await;
        wait_for_http_ok("http://127.0.0.1:25150/api/v1/_health").await;

        Self {
            fixture,
            runtime_dir,
            config_path,
            client: reqwest::Client::new(),
            fake_ak_task,
            server,
        }
    }

    pub async fn run_seed(&self) {
        let bin = binary_path();
        let status = Command::new(bin)
            .arg("--worker-threads")
            .arg("1")
            .arg("seed")
            .arg("-c")
            .arg(&self.config_path)
            .arg("--csv-path")
            .arg(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("e2e/fixtures/versions.csv"))
            .arg("--concurrent")
            .arg("1")
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .status()
            .await
            .unwrap();
        assert!(status.success(), "seed command failed: {status}");
    }

    pub async fn get_json<T: DeserializeOwned>(&self, path: &str) -> T {
        let response = self
            .client
            .get(format!("http://127.0.0.1:{SERVER_PORT}{path}"))
            .send()
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        response.json().await.unwrap()
    }

    pub async fn assert_database_state(&self) {
        let database = connect_database().await;
        let version_count = sqlx::query_scalar!("SELECT COUNT(*) FROM versions")
            .fetch_one(database.pool())
            .await
            .unwrap()
            .unwrap_or_default();
        let file_count = sqlx::query_scalar!("SELECT COUNT(*) FROM files")
            .fetch_one(database.pool())
            .await
            .unwrap()
            .unwrap_or_default();
        let bundle_count = sqlx::query_scalar!("SELECT COUNT(*) FROM bundles")
            .fetch_one(database.pool())
            .await
            .unwrap()
            .unwrap_or_default();

        assert_eq!(
            version_count,
            i64::try_from(self.fixture.versions.len()).unwrap()
        );
        assert_eq!(
            file_count,
            i64::try_from(EXPECTED_UNIQUE_HASHES.len()).unwrap()
        );
        assert_eq!(
            bundle_count,
            i64::try_from(self.fixture.all_bundle_names.len()).unwrap()
        );

        let dedup_counts = sqlx::query!(
            r#"
SELECT f.hash, COUNT(b.id) AS "bundle_count!"
FROM files f
INNER JOIN bundles b ON b.file = f.id
GROUP BY f.hash
ORDER BY f.hash
            "#
        )
        .fetch_all(database.pool())
        .await
        .unwrap();

        assert_eq!(dedup_counts.len(), EXPECTED_UNIQUE_HASHES.len());
        for row in dedup_counts {
            assert!(EXPECTED_UNIQUE_HASHES.contains(&row.hash.as_str()));
            assert_eq!(row.bundle_count, 2);
        }
    }

    pub async fn assert_s3_state(&self) {
        let output = Command::new("rc")
            .arg("object")
            .arg("list")
            .arg("--recursive")
            .arg(format!("{RC_ALIAS_NAME}/{BUCKET_NAME}"))
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .output()
            .await
            .unwrap();
        assert!(output.status.success(), "rc object list failed");
        let stdout = String::from_utf8(output.stdout).unwrap();

        for hash in EXPECTED_UNIQUE_HASHES {
            let key = format!("{}/{}/{}", &hash[..2], &hash[2..4], &hash[4..]);
            assert!(
                stdout.contains(&key),
                "expected S3 object key missing from rc output: {key}\n{stdout}"
            );
        }
    }
}

impl Drop for TestEnv {
    fn drop(&mut self) {
        let _ = self.server.start_kill();
        self.fake_ak_task.abort();
        let _ = fs::remove_dir_all(&self.runtime_dir);
    }
}

pub fn load_fixture(repo_root: &StdPath) -> Fixture {
    let versions = [
        ("26-05-20-12-59-09_e8f456", "2.7.31"),
        ("26-05-27-13-32-37_d44f28", "2.7.41"),
    ]
    .into_iter()
    .map(|(res_version, client_version)| {
        let root = repo_root.join("e2e/fixtures/upstream").join(res_version);
        let hot_update_list = fs::read_to_string(root.join("hot_update_list.json")).unwrap();
        let payload: serde_json::Value = serde_json::from_str(&hot_update_list).unwrap();
        let mut bundle_names = payload["abInfos"]
            .as_array()
            .unwrap()
            .iter()
            .map(|entry| entry["name"].as_str().unwrap().to_string())
            .collect::<Vec<_>>();
        bundle_names.sort();

        FixtureVersion {
            root,
            hot_update_list,
            res_version: res_version.to_string(),
            client_version: client_version.to_string(),
            bundle_names,
        }
    })
    .collect::<Vec<_>>();

    let mut all_bundle_names = versions
        .iter()
        .flat_map(|version| version.bundle_names.iter().cloned())
        .collect::<Vec<_>>();
    all_bundle_names.sort();

    Fixture {
        versions,
        all_bundle_names,
    }
}

async fn ensure_dependencies_ready(repo_root: &StdPath) {
    let docker_compose = repo_root.join("docker-compose.yaml");
    let status = Command::new("docker")
        .arg("compose")
        .arg("-f")
        .arg(&docker_compose)
        .arg("up")
        .arg("-d")
        .arg("db")
        .arg("rustfs")
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status()
        .await
        .unwrap();
    assert!(status.success(), "docker compose up failed");

    wait_for_postgres().await;
    wait_for_rustfs().await;
    recreate_database(repo_root).await;
    let database = connect_database().await;
    database.migrate().await.unwrap();
}

async fn recreate_bucket(repo_root: &StdPath) {
    let status = Command::new("rc")
        .arg("alias")
        .arg("set")
        .arg(RC_ALIAS_NAME)
        .arg("http://127.0.0.1:9000")
        .arg("torappu")
        .arg("torappu123")
        .arg("--bucket-lookup")
        .arg("path")
        .current_dir(repo_root)
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status()
        .await
        .unwrap();
    assert!(status.success(), "rc alias set failed");

    let _ = Command::new("rc")
        .arg("bucket")
        .arg("remove")
        .arg("--force")
        .arg(format!("{RC_ALIAS_NAME}/{BUCKET_NAME}"))
        .current_dir(repo_root)
        .stdout(Stdio::inherit())
        .stderr(Stdio::null())
        .status()
        .await;

    let create_status = Command::new("rc")
        .arg("bucket")
        .arg("create")
        .arg("--ignore-existing")
        .arg(format!("{RC_ALIAS_NAME}/{BUCKET_NAME}"))
        .current_dir(repo_root)
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status()
        .await
        .unwrap();
    assert!(create_status.success(), "rc bucket create failed");
}

async fn connect_database() -> Database {
    Database::connect(&ak_asset_storage::config::DatabaseConfig {
        uri: DATABASE_URI.to_string(),
        max_connections: Some(5),
        connection_timeout_seconds: Some(5),
    })
    .await
    .unwrap()
}

async fn recreate_database(repo_root: &StdPath) {
    let drop_statement = format!("DROP DATABASE IF EXISTS {DATABASE_NAME} WITH (FORCE);");
    let create_statement = format!("CREATE DATABASE {DATABASE_NAME};");

    let drop_status = Command::new("docker")
        .arg("compose")
        .arg("-f")
        .arg(repo_root.join("docker-compose.yaml"))
        .arg("exec")
        .arg("-T")
        .arg("db")
        .arg("psql")
        .arg("-U")
        .arg("ak")
        .arg("-d")
        .arg("postgres")
        .arg("-c")
        .arg(&drop_statement)
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status()
        .await
        .unwrap();
    assert!(drop_status.success(), "drop e2e database failed");

    let create_status = Command::new("docker")
        .arg("compose")
        .arg("-f")
        .arg(repo_root.join("docker-compose.yaml"))
        .arg("exec")
        .arg("-T")
        .arg("db")
        .arg("psql")
        .arg("-U")
        .arg("ak")
        .arg("-d")
        .arg("postgres")
        .arg("-c")
        .arg(&create_statement)
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status()
        .await
        .unwrap();
    assert!(create_status.success(), "create e2e database failed");
}

fn write_config(runtime_dir: &StdPath, asset_dir: &StdPath) -> std::io::Result<PathBuf> {
    let config = format!(
        r#"[logger]
enable = true
level = "debug"
format = "compact"

[server]
binding = "127.0.0.1"
port = {SERVER_PORT}
host = "http://127.0.0.1:{SERVER_PORT}"

[database]
uri = "{DATABASE_URI}"

[ak]
asset_url = "http://127.0.0.1:{FAKE_AK_PORT}/assetbundle/official/Android/assets"
conf_url = "http://127.0.0.1:{FAKE_AK_PORT}/config/prod/official/Android"

[s3]
endpoint = "http://127.0.0.1:9000"
bucket_name = "{BUCKET_NAME}"
access_key_id = "torappu"
secret_access_key = "torappu123"
with_virtual_hosted_style_request = false

[sentry]
dsn = "https://public@example.com/1"
traces_sample_rate = 0.0

[torappu]
token = "e2e-token"
asset_base_path = "{}"
"#,
        asset_dir.display()
    );

    let config_path = runtime_dir.join("config.toml");
    fs::write(&config_path, config)?;
    Ok(config_path)
}

async fn spawn_fake_ak_server(fixture: Fixture) -> JoinHandle<()> {
    let versions = fixture
        .versions
        .iter()
        .cloned()
        .map(|version| (version.res_version.clone(), version))
        .collect::<HashMap<_, _>>();

    let state = Arc::new(FakeAkState { versions });

    let router = Router::new()
        .route("/config/prod/official/Android/version", get(fake_version))
        .route(
            "/assetbundle/official/Android/assets/{res_version}/hot_update_list.json",
            get(fake_hot_update_list),
        )
        .route(
            "/assetbundle/official/Android/assets/{res_version}/{*file_path}",
            get(fake_asset),
        )
        .with_state(state);

    let listener = TcpListener::bind(("127.0.0.1", FAKE_AK_PORT))
        .await
        .unwrap();
    let handle = tokio::spawn(async move {
        axum::serve(listener, router).await.unwrap();
    });

    wait_for_http_ok(&format!(
        "http://127.0.0.1:{FAKE_AK_PORT}/config/prod/official/Android/version"
    ))
    .await;

    handle
}

async fn fake_version(
    State(state): State<Arc<FakeAkState>>,
) -> Result<axum::Json<serde_json::Value>, StatusCode> {
    let latest = state
        .versions
        .values()
        .max_by(|left, right| left.res_version.cmp(&right.res_version))
        .ok_or(StatusCode::NOT_FOUND)?;

    Ok(axum::Json(serde_json::json!({
        "resVersion": latest.res_version,
        "clientVersion": latest.client_version,
    })))
}

async fn fake_hot_update_list(
    State(state): State<Arc<FakeAkState>>,
    Path(res_version): Path<String>,
) -> Result<String, StatusCode> {
    state
        .versions
        .get(&res_version)
        .map(|version| version.hot_update_list.clone())
        .ok_or(StatusCode::NOT_FOUND)
}

async fn fake_asset(
    State(state): State<Arc<FakeAkState>>,
    Path((res_version, file_path)): Path<(String, String)>,
) -> Result<Vec<u8>, StatusCode> {
    let version = state
        .versions
        .get(&res_version)
        .ok_or(StatusCode::NOT_FOUND)?;
    fs::read(version.root.join(file_path)).map_err(|_| StatusCode::NOT_FOUND)
}

async fn spawn_server(config_path: &StdPath) -> Child {
    let bin = binary_path();
    Command::new(bin)
        .arg("--worker-threads")
        .arg("1")
        .arg("server")
        .arg("-c")
        .arg(config_path)
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .spawn()
        .unwrap()
}

fn binary_path() -> PathBuf {
    std::env::var_os("CARGO_BIN_EXE_ak-asset-storage")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target/debug/ak-asset-storage")
        })
}

async fn wait_for_postgres() {
    let started = Instant::now();
    loop {
        if Database::connect(&ak_asset_storage::config::DatabaseConfig {
            uri: POSTGRES_ADMIN_URI.to_string(),
            max_connections: Some(1),
            connection_timeout_seconds: Some(1),
        })
        .await
        .is_ok()
        {
            break;
        }

        assert!(
            started.elapsed() < Duration::from_secs(30),
            "postgres did not become ready"
        );
        sleep(Duration::from_millis(500)).await;
    }
}

async fn wait_for_rustfs() {
    let client = reqwest::Client::new();
    let started = Instant::now();
    loop {
        if let Ok(response) = client.get("http://127.0.0.1:9000/health").send().await
            && response.status().is_success()
        {
            break;
        }

        assert!(
            started.elapsed() < Duration::from_secs(30),
            "rustfs did not become ready"
        );
        sleep(Duration::from_millis(500)).await;
    }
}

async fn wait_for_http_ok(url: &str) {
    let client = reqwest::Client::new();
    let started = Instant::now();
    loop {
        if let Ok(response) = client.get(url).send().await
            && response.status().is_success()
        {
            break;
        }

        assert!(
            started.elapsed() < Duration::from_secs(30),
            "service did not become ready: {url}"
        );
        sleep(Duration::from_millis(500)).await;
    }
}

fn recreate_dir(path: &StdPath) {
    let _ = fs::remove_dir_all(path);
    fs::create_dir_all(path).unwrap();
}

fn install_rustls_provider() {
    let _ = rustls::crypto::ring::default_provider().install_default();
}
