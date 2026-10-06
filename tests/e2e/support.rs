use ak_asset_storage::database::{
    Database,
    bundle::BundleFilter,
    model::{AssetMappingDetails, BundleDetails as BundleDetailsRow, ManifestNode},
    row::{AssetMappingStatus, VersionRow},
};
use axum::{
    Json, Router,
    extract::{Path, State},
    http::StatusCode,
    routing::{get, post},
};
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use std::fmt::Write as _;
use std::{
    collections::{HashMap, HashSet},
    fs,
    path::{Path as StdPath, PathBuf},
    process::Stdio,
    sync::{Arc, Mutex},
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
const FAKE_KUBE_PORT: u16 = 25155;
const BUCKET_NAME: &str = "ak-asset-storage-e2e";
const RC_ALIAS_NAME: &str = "ak-asset-storage-e2e";
const DATABASE_NAME: &str = "ak_asset_storage_e2e";
// Dev dependencies live in the local k3s (deploy/k3s/dev): PostgreSQL on
// NodePort 32432, RustFS on 31000, both bound to 127.0.0.1 only.
const DEV_NAMESPACE: &str = "ak-dev";
const DATABASE_URI: &str = "postgres://ak:ak@localhost:32432/ak_asset_storage_e2e";
const POSTGRES_ADMIN_URI: &str = "postgres://ak:ak@localhost:32432/postgres";
const S3_ENDPOINT: &str = "http://127.0.0.1:31000";
const MANIFEST_NAME: &str = "resource_manifest_idx.json";

const K8S_NAMESPACE: &str = "e2e";
const K8S_JOB_NAME: &str = "ak-asset-storage-e2e-job";
const K8S_IMAGE: &str = "alpine:3.20";
const K8S_ENV_MARKER: &str = "E2E_MARKER=launch_container_e2e";

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
    /// Live when the Job-launch feature is enabled: the in-process fake
    /// Kubernetes API the worker talks to via KUBECONFIG, plus the
    /// kubeconfig file handed to spawned processes.
    fake_kube: Option<FakeKube>,
}

#[derive(Debug)]
pub struct FakeKube {
    pub server_task: JoinHandle<()>,
    pub state: Arc<FakeKubeState>,
    pub kubeconfig_path: PathBuf,
}

/// In-memory state of the fake Kubernetes API server.
#[derive(Debug, Default)]
pub struct FakeKubeState {
    /// Job objects received via POST, in arrival order.
    pub created_jobs: Mutex<Vec<serde_json::Value>>,
}

/// Fields observed from the Job created by the worker, used to assert that
/// `launch_container` forwards the expected args and environment.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LaunchedJobConfig {
    pub image: String,
    pub args: Vec<String>,
    pub env: Vec<(String, String)>,
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

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BundleListResponse {
    pub bundles: Vec<BundleDetails>,
    pub next_cursor: Option<String>,
}

impl TestEnv {
    pub async fn bootstrap() -> Self {
        let (mut env, config_path) = Self::bootstrap_common(false).await;
        let server = spawn_server(&config_path);
        wait_for_http_ok(&format!("http://127.0.0.1:{SERVER_PORT}/api/v1/_health")).await;

        env.server = Some(server);
        env
    }

    pub async fn bootstrap_worker() -> Self {
        let (env, _config_path) = Self::bootstrap_common(false).await;
        env
    }

    /// Bootstraps a worker environment with the Kubernetes Job-launch feature
    /// enabled. An in-process fake Kubernetes API replaces the real cluster:
    /// the worker resolves its credentials from a generated KUBECONFIG
    /// pointing at the fake server, and `wait_for_created_job` observes the
    /// Job the worker creates.
    pub async fn bootstrap_worker_with_kubernetes() -> Self {
        let (env, _config_path) = Self::bootstrap_common(true).await;
        env
    }

    async fn bootstrap_common(include_kubernetes: bool) -> (Self, PathBuf) {
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

        let fake_kube = if include_kubernetes {
            Some(spawn_fake_kube_api(&runtime_dir).await)
        } else {
            None
        };

        let fake_ak_task = spawn_fake_ak_server(fixture.clone()).await;
        let config_path = write_config(&runtime_dir, &asset_dir, include_kubernetes).unwrap();

        let env = Self {
            fixture,
            runtime_dir,
            config_path: config_path.clone(),
            client: reqwest::Client::new(),
            fake_ak_task,
            server: None,
            fake_kube,
        };
        (env, config_path)
    }

    pub fn config_path(&self) -> &StdPath {
        &self.config_path
    }

    /// KUBECONFIG pointing at the fake Kubernetes API, for processes spawned
    /// from this test. `None` when the Job-launch feature is disabled.
    pub fn kubeconfig(&self) -> Option<&StdPath> {
        self.fake_kube
            .as_ref()
            .map(|kube| kube.kubeconfig_path.as_path())
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

    pub async fn run_import_story_usage(&self) {
        let status = self.try_import_story_usage().await;
        assert!(
            status.success(),
            "import-story-usage command failed: {status}"
        );
    }

    pub async fn try_import_story_usage(&self) -> std::process::ExitStatus {
        build_binary_command()
            .arg("import-story-usage")
            .arg("-c")
            .arg(&self.config_path)
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .status()
            .await
            .unwrap()
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

    /// Walks `/api/v1/bundle?{query}` pages with a tiny limit so real
    /// pagination is exercised, returning every matching bundle.
    pub async fn get_all_bundle_pages(&self, query: &str) -> Vec<BundleDetails> {
        let mut all = Vec::new();
        let mut cursor = None;
        loop {
            let cursor_suffix = cursor
                .as_ref()
                .map(|cursor| format!("&cursor={cursor}"))
                .unwrap_or_default();
            let (status, page): (_, BundleListResponse) = self
                .get_json(&format!("/api/v1/bundle?{query}&limit=2{cursor_suffix}"))
                .await;
            assert_eq!(status, StatusCode::OK);
            let done = page.next_cursor.is_none();
            all.extend(page.bundles);
            if done {
                return all;
            }
            cursor = page.next_cursor;
        }
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
        let bundles = all_bundles(&database, &all_bundles_filter()).await;

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
        let bundles = all_bundles(&database, &all_bundles_filter()).await;

        let unique_hashes: HashSet<String> =
            bundles.into_iter().map(|bundle| bundle.file_hash).collect();

        assert!(
            stdout.lines().count() >= unique_hashes.len(),
            "expected at least {} S3 objects, got {}\n{stdout}",
            unique_hashes.len(),
            stdout.lines().count()
        );
    }

    /// Waits for the worker to create the launch Job (`K8S_JOB_NAME`) on the
    /// fake Kubernetes API and returns the job container's image, args, and
    /// environment as recorded by the fake server.
    pub async fn wait_for_created_job(&self, timeout: Duration) -> TestResult<LaunchedJobConfig> {
        let kube = self
            .fake_kube
            .as_ref()
            .ok_or_else(|| "kubernetes launch feature is not enabled".to_string())?;

        wait_for(timeout, Duration::from_secs(1), || async {
            !kube.state.created_jobs.lock().unwrap().is_empty()
        })
        .await
        .map_err(|()| "worker did not create the launch Job within timeout".to_string())?;

        // Clone the recorded Job out of the fake server so the mutex guard
        // is released immediately.
        let job = {
            let jobs = kube.state.created_jobs.lock().unwrap();
            jobs.last()
                .cloned()
                .ok_or_else(|| "created job disappeared".to_string())?
        };
        let container = &job["spec"]["template"]["spec"]["containers"][0];

        let string_array = |value: &serde_json::Value| -> Vec<String> {
            value
                .as_array()
                .map(|entries| {
                    entries
                        .iter()
                        .map(|entry| entry.as_str().unwrap_or_default().to_string())
                        .collect()
                })
                .unwrap_or_default()
        };

        Ok(LaunchedJobConfig {
            image: container["image"].as_str().unwrap_or_default().to_string(),
            args: string_array(&container["args"]),
            env: container["env"]
                .as_array()
                .map(|entries| {
                    entries
                        .iter()
                        .map(|entry| {
                            (
                                entry["name"].as_str().unwrap_or_default().to_string(),
                                entry["value"].as_str().unwrap_or_default().to_string(),
                            )
                        })
                        .collect()
                })
                .unwrap_or_default(),
        })
    }
}

impl Drop for TestEnv {
    fn drop(&mut self) {
        if let Some(ref mut server) = self.server {
            let _ = server.start_kill();
        }
        self.fake_ak_task.abort();
        if let Some(fake_kube) = self.fake_kube.take() {
            fake_kube.server_task.abort();
        }
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

/// Dev dependencies (`PostgreSQL`, `RustFS`) run in the local k3s via
/// `deploy/k3s/dev`. Re-applying the manifests is idempotent and replaces the
/// old `docker compose up`; no Docker daemon is involved anywhere in the e2e
/// flow.
async fn ensure_dependencies_ready(repo_root: &StdPath) {
    let status = Command::new("kubectl")
        .arg("apply")
        .arg("-k")
        .arg(repo_root.join("deploy/k3s/dev"))
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status()
        .await
        .unwrap();
    assert!(status.success(), "kubectl apply -k deploy/k3s/dev failed");

    wait_for_postgres().await;
    wait_for_rustfs().await;
    recreate_database().await;
    let database = connect_database().await;
    database.migrate().await.unwrap();
}

async fn recreate_bucket(repo_root: &StdPath) {
    let status = Command::new("rc")
        .arg("alias")
        .arg("set")
        .arg(RC_ALIAS_NAME)
        .arg(S3_ENDPOINT)
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

/// Starts the fake Kubernetes API server and writes a KUBECONFIG pointing
/// at it. Only the batch/jobs endpoints the launcher touches are served:
/// GET/DELETE answer NotFound/Success so each launch sees a clean slate,
/// POST records the submitted Job for `wait_for_created_job`.
async fn spawn_fake_kube_api(runtime_dir: &StdPath) -> FakeKube {
    let state = Arc::new(FakeKubeState::default());

    let router = Router::new()
        .route(
            "/apis/batch/v1/namespaces/{namespace}/jobs",
            post(fake_create_job),
        )
        .route(
            "/apis/batch/v1/namespaces/{namespace}/jobs/{name}",
            get(fake_get_job).delete(fake_delete_job),
        )
        .with_state(state.clone());

    let listener = TcpListener::bind(("127.0.0.1", FAKE_KUBE_PORT))
        .await
        .unwrap();
    let server_task = tokio::spawn(async move {
        axum::serve(listener, router).await.unwrap();
    });

    let kubeconfig_path = runtime_dir.join("kubeconfig");
    fs::write(
        &kubeconfig_path,
        format!(
            "apiVersion: v1\n\
             kind: Config\n\
             clusters:\n\
             - name: fake\n\
             \x20 cluster:\n\
             \x20   server: http://127.0.0.1:{FAKE_KUBE_PORT}\n\
             contexts:\n\
             - name: fake\n\
             \x20 context:\n\
             \x20   cluster: fake\n\
             \x20   user: fake\n\
             current-context: fake\n\
             users:\n\
             - name: fake\n\
             \x20 user: {{}}\n"
        ),
    )
    .unwrap();

    FakeKube {
        server_task,
        state,
        kubeconfig_path,
    }
}

fn k8s_status_failure(message: &str, reason: &str, code: u16) -> serde_json::Value {
    serde_json::json!({
        "kind": "Status",
        "apiVersion": "v1",
        "metadata": {},
        "status": "Failure",
        "message": message,
        "reason": reason,
        "code": code,
    })
}

async fn fake_get_job(
    Path((_namespace, name)): Path<(String, String)>,
) -> (StatusCode, Json<serde_json::Value>) {
    (
        StatusCode::NOT_FOUND,
        Json(k8s_status_failure(
            &format!("jobs.batch \"{name}\" not found"),
            "NotFound",
            404,
        )),
    )
}

async fn fake_create_job(
    State(state): State<Arc<FakeKubeState>>,
    Json(job): Json<serde_json::Value>,
) -> (StatusCode, Json<serde_json::Value>) {
    state.created_jobs.lock().unwrap().push(job.clone());
    (StatusCode::CREATED, Json(job))
}

async fn fake_delete_job(
    Path((_namespace, _name)): Path<(String, String)>,
) -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "kind": "Status",
        "apiVersion": "v1",
        "metadata": {},
        "status": "Success",
    }))
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

const fn all_bundles_filter() -> BundleFilter {
    BundleFilter {
        path: None,
        hash: None,
        file: None,
        version: None,
    }
}

/// Fetches every bundle matching `filter` by walking the keyset pages, so
/// assertions see exactly what the paginated query returns.
async fn all_bundles(database: &Database, filter: &BundleFilter) -> Vec<BundleDetailsRow> {
    const PAGE: i64 = 100;
    let page_len = usize::try_from(PAGE).expect("PAGE fits usize");
    let mut all = Vec::new();
    let mut after: Option<(i32, String, i32)> = None;
    loop {
        let page = database
            .query_bundles_with_details_page(
                filter,
                after
                    .as_ref()
                    .map(|(version, path, id)| (*version, path.as_str(), *id)),
                PAGE,
            )
            .await
            .unwrap();
        let last_page = page.len() < page_len;
        if let Some(last) = page.last() {
            after = Some((last.version_id, last.path.clone(), last.id));
        }
        all.extend(page);
        if last_page {
            return all;
        }
    }
}

pub async fn wait_for_ready_version(database: &Database, timeout: Duration) -> TestResult<()> {
    wait_for(timeout, Duration::from_secs(1), || async {
        database
            .query_versions()
            .await
            .is_ok_and(|versions| versions.into_iter().any(|version| version.is_ready))
    })
    .await
    .map_err(|()| "worker did not finish downloading within timeout".into())
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
            Ok(None) | Err(_) => false,
        }
    })
    .await
    .map_err(|()| {
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
        &database
            .search_manifest(version_id, "amiya", 200)
            .await
            .unwrap(),
        &[(
            "char_002_amiya_1",
            "arts/charportraits/char_002_amiya_1",
            "file",
        )],
    );
}

/// Drops and recreates the e2e database inside the k3s dev postgres.
async fn recreate_database() {
    let drop_statement = format!("DROP DATABASE IF EXISTS {DATABASE_NAME} WITH (FORCE);");
    let create_statement = format!("CREATE DATABASE {DATABASE_NAME};");

    for statement in [drop_statement, create_statement] {
        let status = Command::new("kubectl")
            .args([
                "-n",
                DEV_NAMESPACE,
                "exec",
                "deploy/postgres",
                "--",
                "psql",
                "-U",
                "ak",
                "-d",
                "postgres",
                "-c",
                &statement,
            ])
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .status()
            .await
            .unwrap();
        assert!(status.success(), "database operation failed");
    }
}

fn write_config(
    runtime_dir: &StdPath,
    asset_dir: &StdPath,
    include_kubernetes: bool,
) -> std::io::Result<PathBuf> {
    let mut config = format!(
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
endpoint = "http://127.0.0.1:31000"
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

[torappu.plocate]
database_path = "{}"
"#,
        asset_dir.display(),
        runtime_dir.join("plocate.db").display()
    );

    if include_kubernetes {
        write!(
            config,
            r#"
[torappu.kubernetes]
image_url = "{K8S_IMAGE}"
namespace = "{K8S_NAMESPACE}"
job_name = "{K8S_JOB_NAME}"
env_vars = ["{K8S_ENV_MARKER}"]
"#
        )
        .unwrap();
    }

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

fn spawn_server(config_path: &StdPath) -> Child {
    build_binary_command()
        .arg("server")
        .arg("-c")
        .arg(config_path)
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .spawn()
        .unwrap()
}

/// Spawns the worker binary. `kubeconfig` (when the launch feature is
/// enabled) points the worker's Kubernetes client at the fake API server
/// via the standard `KUBECONFIG` variable.
pub fn spawn_worker(
    config_path: &StdPath,
    kubeconfig: Option<&StdPath>,
    poll_interval_seconds: u64,
) -> Child {
    let mut cmd = build_binary_command();
    cmd.arg("worker")
        .arg("-c")
        .arg(config_path)
        .arg("--concurrent")
        .arg("1")
        .arg("--poll-interval-seconds")
        .arg(poll_interval_seconds.to_string());
    if let Some(kubeconfig) = kubeconfig {
        cmd.env("KUBECONFIG", kubeconfig);
    }
    cmd.stdout(Stdio::inherit())
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
    std::env::var_os("CARGO_BIN_EXE_ak-asset-storage").map_or_else(
        || PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target/debug/ak-asset-storage"),
        PathBuf::from,
    )
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
    wait_for_http_success(&format!("{S3_ENDPOINT}/health"))
        .await
        .expect("rustfs did not become ready");
}

async fn wait_for_http_ok(url: &str) {
    wait_for_http_success(url)
        .await
        .unwrap_or_else(|()| panic!("service did not become ready: {url}"));
}

async fn wait_for_http_success(url: &str) -> Result<(), ()> {
    let client = reqwest::Client::new();
    wait_for(
        Duration::from_secs(30),
        Duration::from_millis(500),
        || async {
            client
                .get(url)
                .send()
                .await
                .is_ok_and(|response| response.status().is_success())
        },
    )
    .await
}

pub async fn wait_for<F, Fut>(
    timeout: Duration,
    interval: Duration,
    mut condition: F,
) -> Result<(), ()>
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
