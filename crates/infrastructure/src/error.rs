use ak_asset_storage_application::error::AppError;
use thiserror::Error;

use crate::{ak_api_client::AkApiClientError, smtp_client::EmailError, ConfigError};
/// Infrastructure layer errors - external dependencies errors
#[derive(Debug, Error)]
pub enum InfraError {
    #[error("Database error:\n {message} {source}")]
    Database {
        message: String,
        #[source]
        source: sqlx::Error,
    },

    #[error("AkApiClient error:\n{0}")]
    AkApiClient(#[from] AkApiClientError),

    #[error("Database migration error:\n{0}")]
    DatabaseMigration(#[from] sqlx::migrate::MigrateError),

    #[error("S3 error:\n{0}")]
    S3(#[from] object_store::Error),

    #[error("Email error:\n{0}")]
    Email(#[from] EmailError),

    #[error("Configuration error:\n{0}")]
    Config(#[from] ConfigError),

    #[error("Docker error:\n{0}")]
    Docker(String),
}

/// Convert Infrastructure errors to Application errors
impl From<InfraError> for AppError {
    fn from(err: InfraError) -> Self {
        Self::ExternalService(err.into())
    }
}

/// Infrastructure Result type
pub type InfraResult<T> = std::result::Result<T, InfraError>;
