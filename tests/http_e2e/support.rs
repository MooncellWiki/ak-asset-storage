use ak_asset_storage::{
    config::DatabaseConfig,
    database::{
        Database,
        row::{AssetMappingStatus, VersionRow},
    },
};
use reqwest::{Client, Response};
use serde_json::{Value, json};
use sqlx::{ConnectOptions, PgPool};
use std::{
    fs,
    net::TcpListener,
    path::PathBuf,
    process::{Child, Command, Stdio},
    time::{Duration, Instant},
};

/// Runs the real server binary against the isolated database created by
/// `sqlx::test`. Only query fixtures are inserted directly; all assertions
/// exercise HTTP, including MCP's JSON-RPC transport and static downloads.
pub struct TestServer {
    pub database: Database,
    pub client: Client,
    pub base_url: String,
    pub assets: PathBuf,
    root: PathBuf,
    server: Child,
}

impl TestServer {
    pub async fn start(pool: &PgPool) -> Self {
        let _ = rustls::crypto::ring::default_provider().install_default();
        let options = pool.connect_options();
        let uri = options.to_url_lossy().to_string();
        let root = std::env::temp_dir().join(format!(
            "ak-http-e2e-{}-{}",
            std::process::id(),
            options.get_database().unwrap()
        ));
        let assets = root.join("assets");
        fs::create_dir_all(assets.join("raw")).unwrap();
        fs::create_dir_all(assets.join("gamedata")).unwrap();
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let base_url = format!("http://127.0.0.1:{port}");
        let config = json!({
            "logger": {"enable": false, "level": "warn", "format": "compact"},
            "server": {"binding": "127.0.0.1", "port": port, "host": base_url},
            "database": {"uri": uri, "max_connections": 2},
            "ak": {"asset_url": "http://127.0.0.1:9", "conf_url": "http://127.0.0.1:9"},
            "s3": {
                "endpoint": "http://127.0.0.1:9", "bucket_name": "unused",
                "access_key_id": "unused", "secret_access_key": "unused",
                "with_virtual_hosted_style_request": false
            },
            "sentry": {"dsn": "https://test@127.0.0.1:9/1", "traces_sample_rate": 0.0},
            "torappu": {
                "token": "http-e2e-token", "asset_base_path": assets,
                "plocate": {"database_path": root.join("plocate.db")}
            },
            "mcp": {"enable": true}
        });
        let config_path = root.join("config.toml");
        fs::write(&config_path, toml::to_string(&config).unwrap()).unwrap();
        let database = Database::connect(&DatabaseConfig {
            uri,
            max_connections: Some(1),
            connection_timeout_seconds: Some(5),
        })
        .await
        .unwrap();
        let mut command = Command::new(env!("CARGO_BIN_EXE_ak-asset-storage"));
        // Match the full E2E harness: ambient secret overrides must not
        // redirect this subprocess away from its isolated test resources.
        for key in [
            "DATABASE_URI",
            "S3_ACCESS_KEY_ID",
            "S3_SECRET_ACCESS_KEY",
            "MAILER_AUTH_PASSWORD",
            "TORAPPU_TOKEN",
            "TORAPPU_GITHUB_TOKEN",
        ] {
            command.env_remove(key);
        }
        drop(listener);
        let server = command
            .args(["--worker-threads", "1", "server", "-c"])
            .arg(config_path)
            .stdout(Stdio::null())
            .stderr(Stdio::inherit())
            .spawn()
            .unwrap();
        let mut env = Self {
            database,
            client: Client::builder()
                .timeout(Duration::from_secs(5))
                .build()
                .unwrap(),
            base_url,
            assets,
            root,
            server,
        };
        let deadline = Instant::now() + Duration::from_secs(30);
        loop {
            assert!(
                env.server.try_wait().unwrap().is_none(),
                "server exited during startup"
            );
            if let Ok(response) = env
                .client
                .get(format!("{}/api/v1/_health", env.base_url))
                .send()
                .await
                && response.status().is_success()
                && response.json::<Value>().await.unwrap()["ok"] == true
            {
                return env;
            }
            assert!(Instant::now() < deadline, "server did not become healthy");
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
    }

    pub async fn version(&self, res: &str, status: AssetMappingStatus) -> i32 {
        self.database
            .create_version(VersionRow {
                id: None,
                res: res.to_owned(),
                client: "2.7.41".to_owned(),
                is_ready: true,
                asset_mapping_status: status,
                hot_update_list: r#"{"abInfos":[{"name":"fixture.ab"}]}"#.to_owned(),
            })
            .await
            .unwrap()
    }

    pub fn write_asset(&self, path: &str, content: &str) {
        let path = self.assets.join(path);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, content).unwrap();
    }

    pub async fn get(&self, path: &str, query: &[(&str, &str)]) -> Response {
        let mut url = reqwest::Url::parse(&format!("{}{path}", self.base_url)).unwrap();
        url.query_pairs_mut().extend_pairs(query.iter().copied());
        self.client.get(url).send().await.unwrap()
    }

    pub async fn rpc(&self, method: &str, params: Value) -> Value {
        let response = self
            .client
            .post(format!("{}/mcp", self.base_url))
            .header("Accept", "application/json, text/event-stream")
            .header("MCP-Protocol-Version", "2025-03-26")
            .json(&json!({"jsonrpc": "2.0", "id": 1, "method": method, "params": params}))
            .send()
            .await
            .unwrap();
        assert_eq!(response.status(), reqwest::StatusCode::OK);
        assert!(
            response.headers()["content-type"]
                .to_str()
                .unwrap()
                .starts_with("application/json")
        );
        // Stateless requests must work without retaining a session id.
        assert!(!response.headers().contains_key("mcp-session-id"));
        let body: Value = response.json().await.unwrap();
        assert_eq!(body["jsonrpc"], "2.0");
        assert_eq!(body["id"], 1);
        body
    }

    pub async fn tool(&self, name: &str, arguments: Value) -> Value {
        let body = self
            .rpc("tools/call", json!({"name": name, "arguments": arguments}))
            .await;
        assert!(body.get("error").is_none(), "{body}");
        assert_ne!(body["result"]["isError"], true, "{body}");
        serde_json::from_str(body["result"]["content"][0]["text"].as_str().unwrap()).unwrap()
    }

    pub async fn close(mut self) {
        self.stop();
        self.database.pool().close().await;
    }

    fn stop(&mut self) {
        let _ = self.server.kill();
        let _ = self.server.wait();
    }
}

impl Drop for TestServer {
    fn drop(&mut self) {
        self.stop();
        let _ = fs::remove_dir_all(&self.root);
    }
}
