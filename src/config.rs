use std::fs;
use std::path::Path;

use crate::error::{Error, Result};
use crate::logger;
use anyhow::anyhow;
use object_store::aws::{AmazonS3, AmazonS3Builder};
use serde::{Deserialize, Serialize};
use tracing::info;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Config {
    pub logger: Logger,
    pub server: Server,
    pub database: Database,
    pub mailer: Mailer,
    pub ak: AkConfig,
    pub s3: S3Config,
}
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AkConfig {
    pub base_url: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct Logger {
    /// Enable log write to stdout
    pub enable: bool,

    /// Set the logger level.
    ///
    /// * options: `trace` | `debug` | `info` | `warn` | `error`
    pub level: logger::LogLevel,

    /// Set the logger format.
    ///
    /// * options: `compact` | `pretty` | `json`
    pub format: logger::Format,

    /// Override our custom tracing filter.
    ///
    /// Set this to your own filter if you want to see traces from internal
    /// libraries. See more [here](https://docs.rs/tracing-subscriber/latest/tracing_subscriber/filter/struct.EnvFilter.html#directives)
    pub override_filter: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Database {
    /// The URI for connecting to the database. For example:
    /// * Postgres: `postgres://root:12341234@localhost:5432/myapp_development`
    /// * Sqlite: `sqlite://db.sqlite?mode=rwc`
    pub uri: String,

    /// Enable `SQLx` statement logging
    pub enable_logging: bool,

    /// Minimum number of connections for a pool
    pub min_connections: u32,

    /// Maximum number of connections for a pool
    pub max_connections: u32,

    /// Set the timeout duration when acquiring a connection
    pub connect_timeout: u64,

    /// Set the idle duration before closing a connection
    pub idle_timeout: u64,

    /// Set the timeout for acquiring a connection
    pub acquire_timeout: Option<u64>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Server {
    /// The address on which the server should listen on for incoming
    /// connections.
    #[serde(default = "default_binding")]
    pub binding: String,
    /// The port on which the server should listen for incoming connections.
    pub port: i32,
    /// The webserver host
    pub host: String,
}

fn default_binding() -> String {
    "localhost".to_string()
}

impl Server {
    #[must_use]
    pub fn full_url(&self) -> String {
        format!("{}:{}", self.host, self.port)
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Mailer {
    pub smtp: SmtpMailer,
}

/// SMTP mailer configuration structure.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SmtpMailer {
    /// SMTP host. for example: localhost, smtp.gmail.com etc.
    pub host: String,
    /// SMTP port/
    pub port: u16,
    /// Auth SMTP server
    pub auth: MailerAuth,
}

/// Authentication details for the mailer
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct MailerAuth {
    /// User
    pub user: String,
    /// Password
    pub password: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct S3Config {
    pub endpoint: String,
    pub access_key_id: String,
    pub secret_access_key: String,
    pub bucket_name: String,
}
impl S3Config {
    pub fn client(&self) -> Result<AmazonS3> {
        let s3 = AmazonS3Builder::new()
            .with_endpoint(self.endpoint.clone())
            .with_bucket_name(self.bucket_name.clone())
            .with_access_key_id(self.access_key_id.clone())
            .with_secret_access_key(self.secret_access_key.clone())
            .with_virtual_hosted_style_request(true)
            .build()?;
        Ok(s3)
    }
}

impl Config {
    pub fn new(config: &Path) -> Result<Self> {
        if !config.exists() {
            return Err(anyhow!("no configuration file found").into());
        }
        info!(selected_path =? config, "loading environment from");
        let content = fs::read_to_string(config)?;
        toml::from_str(&content)
            .map_err(|err| Error::TOMLFile(err, config.to_string_lossy().to_string()))
    }
}
impl std::fmt::Display for Config {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let content = toml::to_string(self).unwrap_or_default();
        write!(f, "{content}")
    }
}
