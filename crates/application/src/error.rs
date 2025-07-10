use thiserror::Error;

/// Application layer errors - use case and service errors
#[derive(Debug, Error)]
pub enum AppError {
    #[error("External service error:\n{0}")]
    ExternalService(anyhow::Error),

    #[error("Application error:\n{0}")]
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
