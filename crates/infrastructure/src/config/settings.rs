use application::{
    AkApiConfig, AppResult, DatabaseConfig, LoggerConfig, S3Config, SentryConfig, ServerConfig,
    SmtpConfig,
};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;
use thiserror::Error;
use tracing::info;

use crate::InfraError;

/// Complete application settings that combines all configuration layers
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AppSettings {
    pub logger: LoggerConfig,
    pub server: ServerConfig,
    pub database: DatabaseConfig,
    pub mailer: SmtpConfig,
    pub ak: AkApiConfig,
    pub s3: S3Config,
    pub sentry: SentryConfig,
}

impl AppSettings {
    pub fn new(config: &Path) -> AppResult<Self> {
        info!(selected_path =? config, "loading environment from");
        let content = fs::read_to_string(config).into_app_result()?;
        toml::from_str(&content).into_app_result()
    }
}

impl std::fmt::Display for AppSettings {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let content = toml::to_string(self).unwrap_or_default();
        write!(f, "{content}")
    }
}

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("Failed to read configuration file: {0}")]
    ReadError(#[from] std::io::Error),
    #[error("Failed to parse configuration: {0}")]
    ParseError(#[from] toml::de::Error),
}

trait IntoAppResult<T, E> {
    fn into_app_result(self) -> AppResult<T>;
}
impl<T, E1: Into<ConfigError>> IntoAppResult<T, E1> for Result<T, E1> {
    fn into_app_result(self) -> AppResult<T> {
        self.map_err(|e| InfraError::Config(e.into()).into())
    }
}
