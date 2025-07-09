use application::error::AppError;
use thiserror::Error;

use crate::{ak_api_client::AkApiClientError, smtp_client::EmailError, ConfigError};
/// Infrastructure layer errors - external dependencies errors
#[derive(Debug, Error)]
pub enum InfraError {
    #[error("Database error {}", message)]
    Database {
        message: String,
        #[source]
        source: sqlx::Error,
    },

    #[error("AkApiClient error")]
    AkApiClient(#[from] AkApiClientError),

    #[error("Database migration error")]
    DatabaseMigration(#[from] sqlx::migrate::MigrateError),

    #[error("S3 error")]
    S3(#[from] object_store::Error),

    #[error("Email error")]
    Email(#[from] EmailError),

    #[error("Configuration error")]
    Config(#[from] ConfigError),
}

/// Convert Infrastructure errors to Application errors
impl From<InfraError> for AppError {
    fn from(err: InfraError) -> Self {
        Self::ExternalService(err.into())
    }
}

/// Infrastructure Result type
pub type InfraResult<T> = std::result::Result<T, InfraError>;
