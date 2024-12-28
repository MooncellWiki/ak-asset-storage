use crate::utils;
use anyhow::{bail, Result};
use object_store::aws::{AmazonS3, AmazonS3Builder};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;
use tracing::info;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Config {
    pub logger: Logger,
    pub server: Server,
    pub database: Database,
    pub mailer: Mailer,
    pub ak: Ak,
    pub s3: S3,
    pub sentry: Sentry,
}
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Ak {
    pub asset_url: String,
    pub conf_url: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct Logger {
    /// Enable log write to stdout
    pub enable: bool,

    /// Set the logger level.
    ///
    /// * options: `trace` | `debug` | `info` | `warn` | `error`
    pub level: utils::tracing::LogLevel,

    /// Set the logger format.
    ///
    /// * options: `compact` | `pretty` | `json`
    pub format: utils::tracing::Format,

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
        format!("{}:{}", self.binding, self.port)
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
pub struct S3 {
    pub endpoint: String,
    pub access_key_id: String,
    pub secret_access_key: String,
    pub bucket_name: String,
    pub with_virtual_hosted_style_request: bool,
}
impl S3 {
    pub fn client(&self) -> Result<AmazonS3> {
        let s3 = AmazonS3Builder::new()
            .with_allow_http(true)
            .with_endpoint(self.endpoint.clone())
            .with_bucket_name(self.bucket_name.clone())
            .with_access_key_id(self.access_key_id.clone())
            .with_secret_access_key(self.secret_access_key.clone())
            .with_virtual_hosted_style_request(self.with_virtual_hosted_style_request)
            .build()?;
        Ok(s3)
    }
}

impl Config {
    pub fn new(config: &Path) -> Result<Self> {
        if !config.exists() {
            bail!("no configuration file found");
        }
        info!(selected_path =? config, "loading environment from");
        let content = fs::read_to_string(config)?;
        Ok(toml::from_str(&content)?)
    }
}
impl std::fmt::Display for Config {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let content = toml::to_string(self).unwrap_or_default();
        write!(f, "{content}")
    }
}
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Sentry {
    pub dsn: String,
    /// The sample rate for tracing transactions.
    pub traces_sample_rate: f32,
}
