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

/// MCP (Model Context Protocol) endpoint configuration. The endpoint only
/// mirrors the public read-only queries, but stays opt-in so existing
/// deployments keep serving exactly the routes they had before.
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct McpConfig {
    #[serde(default)]
    pub enable: bool,
    /// When set, `/mcp` requires `Authorization: Bearer <auth_token>`.
    #[serde(default)]
    pub auth_token: Option<String>,
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
    #[serde(default)]
    pub mcp: McpConfig,
}

impl AppSettings {
    pub fn load(path: &Path) -> AppResult<Self> {
        info!(selected_path = ?path, "loading config");
        let content = fs::read_to_string(path).map_err(|err| AppError::Application(err.into()))?;
        let settings: Self =
            toml::from_str(&content).map_err(|err| AppError::Application(err.into()))?;
        settings.validate()?;
        Ok(settings)
    }

    /// Rejects configurations that would silently weaken security at runtime.
    fn validate(&self) -> AppResult<()> {
        if self.torappu.token.trim().is_empty() {
            return Err(AppError::Application(anyhow::anyhow!(
                "torappu.token must not be empty: it guards the Docker launch endpoint"
            )));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn settings_with_token(token: &str) -> AppSettings {
        AppSettings {
            logger: LoggerConfig::default(),
            server: ServerConfig {
                binding: default_binding(),
                port: 5150,
                host: "http://localhost".to_string(),
            },
            database: DatabaseConfig {
                uri: "postgres://localhost/db".to_string(),
                max_connections: None,
                connection_timeout_seconds: None,
            },
            mailer: None,
            ak: AkApiConfig {
                conf_url: String::new(),
                asset_url: String::new(),
            },
            s3: S3Config {
                endpoint: String::new(),
                access_key_id: String::new(),
                secret_access_key: String::new(),
                bucket_name: String::new(),
                with_virtual_hosted_style_request: false,
            },
            sentry: SentryConfig {
                dsn: String::new(),
                traces_sample_rate: 0.0,
            },
            torappu: TorappuConfig {
                token: token.to_string(),
                asset_base_path: "/assets".to_string(),
                docker: None,
                github: None,
            },
            mcp: McpConfig::default(),
        }
    }

    #[test]
    fn validate_rejects_blank_torappu_token() {
        assert!(settings_with_token("").validate().is_err());
        assert!(settings_with_token("   ").validate().is_err());
        assert!(settings_with_token("s3cret").validate().is_ok());
    }
}
