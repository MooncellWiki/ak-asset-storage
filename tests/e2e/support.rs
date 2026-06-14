use ak_asset_storage::database::{
    Database,
    bundle::BundleFilter,
    model::{AssetMappingDetails, ManifestNode},
    row::{AssetMappingStatus, VersionRow},
};
use axum::{
    Router,
    extract::{Path, State},
    http::StatusCode,
    routing::get,
};
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use std::{
    collections::{HashMap, HashSet},
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

type TestResult<T> = Result<T, Box<dyn std::error::Error + Send + Sync>>;

const SERVER_PORT: u16 = 25150;
const FAKE_AK_PORT: u16 = 25151;
const BUCKET_NAME: &str = "ak-asset-storage-e2e";
const RC_ALIAS_NAME: &str = "ak-asset-storage-e2e";
const DATABASE_NAME: &str = "ak_asset_storage_e2e";
const DATABASE_URI: &str = "postgres://ak:ak@localhost:25432/ak_asset_storage_e2e";
const POSTGRES_ADMIN_URI: &str = "postgres://ak:ak@localhost:25432/postgres";
const MANIFEST_NAME: &str = "resource_manifest_idx.json";

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
    server: Option<Child>,
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
        let (mut env, config_path) = Self::bootstrap_common().await;
        let server = spawn_server(&config_path).await;
        wait_for_http_ok(&format!("http://127.0.0.1:{SERVER_PORT}/api/v1/_health")).await;

        env.server = Some(server);
        env
    }

    pub async fn bootstrap_worker() -> Self {
        let (env, _config_path) = Self::bootstrap_common().await;
        env
    }

    async fn bootstrap_common() -> (Self, PathBuf) {
        install_rustls_provider();
        let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let runtime_dir = repo_root.join("e2e/runtime");
        recreate_dir(&runtime_dir);

        let asset_dir = runtime_dir.join("asset");
        fs::create_dir_all(&asset_dir).unwrap();
        let gamedata_dir = asset_dir.join("gamedata");
        fs::create_dir_all(&gamedata_dir).unwrap();

        let fixture = load_fixture(&repo_root);
        ensure_dependencies_ready(&repo_root).await;
        recreate_bucket(&repo_root).await;

        let fake_ak_task = spawn_fake_ak_server(fixture.clone()).await;
        let config_path = write_config(&runtime_dir, &asset_dir).unwrap();

        let env = Self {
            fixture,
            runtime_dir,
            config_path: config_path.clone(),
            client: reqwest::Client::new(),
            fake_ak_task,
            server: None,
        };
        (env, config_path)
    }

    pub fn config_path(&self) -> &StdPath {
        &self.config_path
    }

    pub fn runtime_dir(&self) -> &StdPath {
        &self.runtime_dir
    }

    pub async fn run_seed(&self) {
        let status = build_binary_command()
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

    pub async fn run_import_manifest(&self, res_version: &str) {
        let status = build_binary_command()
            .arg("import-manifest")
            .arg("-c")
            .arg(&self.config_path)
            .arg("--res-version")
            .arg(res_version)
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .status()
            .await
            .unwrap();
        assert!(status.success(), "import-manifest command failed: {status}");
    }

    pub async fn run_import_item_demand(&self) {
        let status = build_binary_command()
            .arg("import-item-demand")
            .arg("-c")
            .arg(&self.config_path)
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .status()
            .await
            .unwrap();
        assert!(
            status.success(),
            "import-item-demand command failed: {status}"
        );
    }

    pub fn copy_item_demand_fixture<P: AsRef<StdPath>>(&self, source: P) {
        let target_dir = self.runtime_dir.join("asset/raw");
        fs::create_dir_all(&target_dir).unwrap();
        fs::copy(source, target_dir.join("itemDemand.json")).unwrap();
    }

    pub async fn create_version_for_manifest_test(&self, res_version: &str, is_ready: bool) -> i32 {
        let version = self
            .fixture
            .versions
            .iter()
            .find(|version| version.res_version == res_version)
            .unwrap();
        let database = connect_database().await;
        database
            .create_version(VersionRow {
                id: None,
                res: version.res_version.clone(),
                client: version.client_version.clone(),
                is_ready,
                asset_mapping_status: AssetMappingStatus::Pending,
                hot_update_list: version.hot_update_list.clone(),
            })
            .await
            .unwrap()
    }

    pub fn copy_manifest_fixture(&self, res_version: &str) {
        let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let source = repo_root
            .join("e2e/fixtures/manifests")
            .join(res_version)
            .join(MANIFEST_NAME);
        let target_dir = self.runtime_dir.join("asset/gamedata").join(res_version);
        fs::create_dir_all(&target_dir).unwrap();
        fs::copy(source, target_dir.join(MANIFEST_NAME)).unwrap();
    }

    pub async fn get_json<T: DeserializeOwned>(&self, path: &str) -> (StatusCode, T) {
        let response = self
            .client
            .get(format!("http://127.0.0.1:{SERVER_PORT}{path}"))
            .send()
            .await
            .unwrap();
        let status = response.status();
        let body = response.json().await.unwrap();
        (status, body)
    }

    pub async fn get_text(&self, path: &str) -> (StatusCode, String) {
        let response = self
            .client
            .get(format!("http://127.0.0.1:{SERVER_PORT}{path}"))
            .send()
            .await
            .unwrap();
        let status = response.status();
        let body = response.text().await.unwrap();
        (status, body)
    }

    pub async fn assert_database_state(&self) {
        let database = connect_database().await;
        let versions = database.query_versions().await.unwrap();
        let bundles = database
            .query_bundles_with_details(&all_bundles_filter())
            .await
            .unwrap();

        assert_eq!(versions.len(), self.fixture.versions.len());
        assert_eq!(bundles.len(), self.fixture.all_bundle_names.len());

        let mut file_id_by_hash: HashMap<String, HashSet<i32>> = HashMap::new();
        for bundle in bundles {
            file_id_by_hash
                .entry(bundle.file_hash)
                .or_default()
                .insert(bundle.file_id);
        }
        assert!(file_id_by_hash.values().all(|ids| ids.len() == 1));
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

        let database = connect_database().await;
        let bundles = database
            .query_bundles_with_details(&all_bundles_filter())
            .await
            .unwrap();

        let unique_hashes: HashSet<String> =
            bundles.into_iter().map(|bundle| bundle.file_hash).collect();

        assert!(
            stdout.lines().count() >= unique_hashes.len(),
            "expected at least {} S3 objects, got {}\n{stdout}",
            unique_hashes.len(),
            stdout.lines().count()
        );
    }
}

impl Drop for TestEnv {
    fn drop(&mut self) {
        if let Some(ref mut server) = self.server {
            let _ = server.start_kill();
        }
        self.fake_ak_task.abort();
        let _ = fs::remove_dir_all(&self.runtime_dir);
    }
}

pub fn load_fixture(repo_root: &StdPath) -> Fixture {
    let versions: Vec<FixtureVersion> = [
        ("26-05-20-12-59-09_e8f456", "2.7.31"),
        ("26-05-27-13-32-37_d44f28", "2.7.41"),
    ]
    .into_iter()
    .map(|(res_version, client_version)| {
        let root = repo_root.join("e2e/fixtures/upstream").join(res_version);
        let hot_update_list = fs::read_to_string(root.join("hot_update_list.json")).unwrap();
        let payload: serde_json::Value = serde_json::from_str(&hot_update_list).unwrap();
        let mut bundle_names: Vec<String> = payload["abInfos"]
            .as_array()
            .unwrap()
            .iter()
            .map(|entry| entry["name"].as_str().unwrap().to_string())
            .collect();
        bundle_names.sort();

        FixtureVersion {
            root,
            hot_update_list,
            res_version: res_version.to_string(),
            client_version: client_version.to_string(),
            bundle_names,
        }
    })
    .collect();

    let mut all_bundle_names: Vec<String> = versions
        .iter()
        .flat_map(|version| version.bundle_names.iter().cloned())
        .collect();
    all_bundle_names.sort();

    Fixture {
        versions,
        all_bundle_names,
    }
}

async fn ensure_dependencies_ready(repo_root: &StdPath) {
    let status = Command::new("docker")
        .arg("compose")
        .arg("-f")
        .arg(repo_root.join("docker-compose.yaml"))
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

pub async fn connect_database() -> Database {
    Database::connect(&ak_asset_storage::config::DatabaseConfig {
        uri: DATABASE_URI.to_string(),
        max_connections: Some(5),
        connection_timeout_seconds: Some(5),
    })
    .await
    .unwrap()
}

fn all_bundles_filter() -> BundleFilter {
    BundleFilter {
        path: None,
        hash: None,
        file: None,
        version: None,
    }
}

pub async fn wait_for_ready_version(database: &Database, timeout: Duration) -> TestResult<()> {
    wait_for(timeout, Duration::from_secs(1), || async {
        match database.query_versions().await {
            Ok(versions) => versions.into_iter().any(|version| version.is_ready),
            Err(_) => false,
        }
    })
    .await
    .map_err(|_| "worker did not finish downloading within timeout".into())
}

pub async fn wait_for_asset_mapping_status(
    database: &Database,
    res_version: &str,
    expected_status: AssetMappingStatus,
    timeout: Duration,
) -> TestResult<()> {
    wait_for(timeout, Duration::from_secs(1), || async {
        match database.get_version_by_res(res_version).await {
            Ok(Some(version)) => version.asset_mapping_status == expected_status,
            Ok(None) => false,
            Err(_) => false,
        }
    })
    .await
    .map_err(|_| {
        format!(
            "asset mapping status for {res_version} did not become {expected_status:?} within timeout"
        )
        .into()
    })
}

pub async fn assert_manifest_fixture_imported(database: &Database, version_id: i32) {
    assert_manifest_children(
        &database
            .list_manifest_children(version_id, "")
            .await
            .unwrap(),
        &[
            ("arts", "arts", "directory"),
            ("scenes", "scenes", "directory"),
        ],
    );
    assert_manifest_children(
        &database
            .list_manifest_children(version_id, "arts")
            .await
            .unwrap(),
        &[
            ("avgmaterialpresets", "arts/avgmaterialpresets", "directory"),
            ("charportraits", "arts/charportraits", "directory"),
            ("avg_shader_profile", "arts/avg_shader_profile", "file"),
        ],
    );
    assert_manifest_children(
        &database
            .list_manifest_children(version_id, "scenes/activities/a001/level_a001_01")
            .await
            .unwrap(),
        &[(
            "level_a001_01",
            "scenes/activities/a001/level_a001_01/level_a001_01",
            "both",
        )],
    );
    assert_manifest_children(
        &database
            .list_manifest_children(
                version_id,
                "scenes/activities/a001/level_a001_01/level_a001_01",
            )
            .await
            .unwrap(),
        &[(
            "lightingdata",
            "scenes/activities/a001/level_a001_01/level_a001_01/lightingdata",
            "file",
        )],
    );

    let avg_shader = database
        .get_asset_mapping_detail(version_id, "arts/avg_shader_profile")
        .await
        .unwrap()
        .unwrap();
    assert_asset_mapping_details(
        &avg_shader,
        "arts/avg_shader_profile",
        "arts/avg_shader_profile.ab",
        Some("dyn/arts/avg_shader_profile.prefab"),
        Some("avg_shader_profile"),
    );

    let scene = database
        .get_asset_mapping_detail(
            version_id,
            "scenes/activities/a001/level_a001_01/level_a001_01",
        )
        .await
        .unwrap()
        .unwrap();
    assert_asset_mapping_details(
        &scene,
        "scenes/activities/a001/level_a001_01/level_a001_01",
        "scenes/activities/a001/level_a001_01/level_a001_01.ab",
        Some("dyn/scenes/activities/a001/level_a001_01/level_a001_01.unity"),
        Some("level_a001_01"),
    );

    assert_manifest_children(
        &database.search_manifest(version_id, "amiya").await.unwrap(),
        &[(
            "char_002_amiya_1",
            "arts/charportraits/char_002_amiya_1",
            "file",
        )],
    );
}

async fn recreate_database(repo_root: &StdPath) {
    let drop_statement = format!("DROP DATABASE IF EXISTS {DATABASE_NAME} WITH (FORCE);");
    let create_statement = format!("CREATE DATABASE {DATABASE_NAME};");

    for statement in [drop_statement, create_statement] {
        let status = docker_compose_exec_psql(repo_root, &statement)
            .status()
            .await
            .unwrap();
        assert!(status.success(), "database operation failed");
    }
}

fn docker_compose_exec_psql(repo_root: &StdPath, statement: &str) -> Command {
    let mut cmd = Command::new("docker");
    cmd.arg("compose")
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
        .arg(statement)
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit());
    cmd
}

fn write_config(runtime_dir: &StdPath, asset_dir: &StdPath) -> std::io::Result<PathBuf> {
    let config = format!(
        r#"[logger]
enable = true
level = "warn"
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
    let versions: HashMap<String, FixtureVersion> = fixture
        .versions
        .into_iter()
        .map(|version| (version.res_version.clone(), version))
        .collect();

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
        .max_by_key(|version| &version.res_version)
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
    build_binary_command()
        .arg("server")
        .arg("-c")
        .arg(config_path)
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .spawn()
        .unwrap()
}

pub async fn spawn_worker(config_path: &StdPath, poll_interval_seconds: u64) -> Child {
    build_binary_command()
        .arg("worker")
        .arg("-c")
        .arg(config_path)
        .arg("--concurrent")
        .arg("1")
        .arg("--poll-interval-seconds")
        .arg(poll_interval_seconds.to_string())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .spawn()
        .unwrap()
}

fn build_binary_command() -> Command {
    let mut cmd = Command::new(binary_path());
    cmd.arg("--worker-threads").arg("1");
    cmd
}

fn binary_path() -> PathBuf {
    std::env::var_os("CARGO_BIN_EXE_ak-asset-storage")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target/debug/ak-asset-storage")
        })
}

async fn wait_for_postgres() {
    wait_for(
        Duration::from_secs(30),
        Duration::from_millis(500),
        || async {
            Database::connect(&ak_asset_storage::config::DatabaseConfig {
                uri: POSTGRES_ADMIN_URI.to_string(),
                max_connections: Some(1),
                connection_timeout_seconds: Some(1),
            })
            .await
            .is_ok()
        },
    )
    .await
    .expect("postgres did not become ready");
}

async fn wait_for_rustfs() {
    wait_for_http_success("http://127.0.0.1:9000/health")
        .await
        .expect("rustfs did not become ready");
}

async fn wait_for_http_ok(url: &str) {
    wait_for_http_success(url)
        .await
        .unwrap_or_else(|_| panic!("service did not become ready: {url}"));
}

async fn wait_for_http_success(url: &str) -> Result<(), ()> {
    let client = reqwest::Client::new();
    wait_for(
        Duration::from_secs(30),
        Duration::from_millis(500),
        || async {
            match client.get(url).send().await {
                Ok(response) => response.status().is_success(),
                Err(_) => false,
            }
        },
    )
    .await
}

async fn wait_for<F, Fut>(timeout: Duration, interval: Duration, mut condition: F) -> Result<(), ()>
where
    F: FnMut() -> Fut,
    Fut: std::future::Future<Output = bool>,
{
    let started = Instant::now();
    loop {
        if condition().await {
            return Ok(());
        }

        if started.elapsed() >= timeout {
            return Err(());
        }

        sleep(interval).await;
    }
}

fn recreate_dir(path: &StdPath) {
    let _ = fs::remove_dir_all(path);
    fs::create_dir_all(path).unwrap();
}

fn install_rustls_provider() {
    let _ = rustls::crypto::ring::default_provider().install_default();
}

fn assert_manifest_children(nodes: &[ManifestNode], expected: &[(&str, &str, &str)]) {
    let actual: Vec<(&str, &str, &str)> = nodes
        .iter()
        .map(|node| {
            (
                node.name.as_str(),
                node.path.as_str(),
                node.node_type.as_str(),
            )
        })
        .collect();
    assert_eq!(actual, expected);
}

fn assert_asset_mapping_details(
    details: &AssetMappingDetails,
    asset_name: &str,
    bundle_path: &str,
    asset_path: Option<&str>,
    short_name: Option<&str>,
) {
    assert_eq!(details.asset_name, asset_name);
    assert_eq!(details.bundle_path, bundle_path);
    assert_eq!(details.asset_path.as_deref(), asset_path);
    assert_eq!(details.short_name.as_deref(), short_name);
    assert!(
        details.bundle_size.is_some(),
        "bundle_size should be Some after download"
    );
    assert!(
        details.bundle_hash.is_some(),
        "bundle_hash should be Some after download"
    );
}
