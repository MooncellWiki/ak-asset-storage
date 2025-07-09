use domain::error::DomainError;
use thiserror::Error;

/// Application layer errors - use case and service errors
#[derive(Debug, Error)]
pub enum AppError {
    #[error("Domain error")]
    Domain(#[from] DomainError),

    #[error("External service error")]
    ExternalService(anyhow::Error),

    #[error("Application error")]
    Application(#[from] anyhow::Error),
}

/// Application Result type
pub type AppResult<T> = std::result::Result<T, AppError>;

impl From<serde_json::Error> for AppError {
    fn from(err: serde_json::Error) -> Self {
        Self::Application(anyhow::anyhow!(err))
    }
}
impl From<std::io::Error> for AppError {
    fn from(err: std::io::Error) -> Self {
        Self::Application(anyhow::anyhow!(err))
    }
}
