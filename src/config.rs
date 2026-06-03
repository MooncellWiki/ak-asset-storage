use crate::{AppError, AppResult};
use serde::{Deserialize, Serialize};
use serde_variant::to_variant_name;
use std::{fs, path::Path};
use tracing::info;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct DatabaseConfig {
    pub uri: String,
    pub max_connections: Option<u32>,
    pub connection_timeout_seconds: Option<u64>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ServerConfig {
    #[serde(default = "default_binding")]
    pub binding: String,
    pub port: i32,
    pub host: String,
}

fn default_binding() -> String {
    "localhost".to_string()
}

impl ServerConfig {
    #[must_use]
    pub fn full_url(&self) -> String {
        format!("{}:{}", self.binding, self.port)
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AkApiConfig {
    pub conf_url: String,
    pub asset_url: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct S3Config {
    pub endpoint: String,
    pub access_key_id: String,
    pub secret_access_key: String,
    pub bucket_name: String,
    pub with_virtual_hosted_style_request: bool,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SmtpConfig {
    pub host: String,
    pub port: u16,
    pub auth: MailerAuthConfig,
    pub from_email: String,
    pub to_email: String,
    pub frontend_url: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct MailerAuthConfig {
    pub user: String,
    pub password: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum LogLevel {
    Trace,
    Debug,
    #[default]
    Info,
    Warn,
    Error,
}

impl std::fmt::Display for LogLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        to_variant_name(self).expect("only enum supported").fmt(f)
    }
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum LogFormat {
    #[default]
    Compact,
    Pretty,
    Json,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct LoggerConfig {
    pub enable: bool,
    pub level: LogLevel,
    pub format: LogFormat,
    pub override_filter: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SentryConfig {
    pub dsn: String,
    pub traces_sample_rate: f32,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TorappuConfig {
    pub token: String,
    pub asset_base_path: String,
    pub docker: Option<DockerConfig>,
    pub github: Option<GithubConfig>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct DockerConfig {
    pub image_url: String,
    pub container_name: String,
    pub env_vars: Option<Vec<String>>,
    pub volume_mapping: Option<Vec<String>>,
    pub docker_host: String,
    pub username: String,
    pub password: String,
    pub network: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct GithubConfig {
    pub owner: String,
    pub repo: String,
    pub workflow_id: String,
    pub r#ref: String,
    pub token: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AppSettings {
    pub logger: LoggerConfig,
    pub server: ServerConfig,
    pub database: DatabaseConfig,
    pub mailer: Option<SmtpConfig>,
    pub ak: AkApiConfig,
    pub s3: S3Config,
    pub sentry: SentryConfig,
    pub torappu: TorappuConfig,
}

impl AppSettings {
    pub fn load(path: &Path) -> AppResult<Self> {
        info!(selected_path = ?path, "loading config");
        let content = fs::read_to_string(path).map_err(|err| AppError::Application(err.into()))?;
        toml::from_str(&content).map_err(|err| AppError::Application(err.into()))
    }
}

impl std::fmt::Display for AppSettings {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let content = toml::to_string(self).unwrap_or_default();
        write!(f, "{content}")
    }
}
