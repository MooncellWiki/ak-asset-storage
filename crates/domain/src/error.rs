use thiserror::Error;

/// Domain layer errors - pure business logic errors
#[derive(Debug, Error)]
pub enum DomainError {
    #[error("Invalid value: {message}")]
    InvalidValue { message: String },

    #[error("Invalid state: {message}")]
    InvalidState { message: String },

    #[error("Invalid version format: {message}")]
    InvalidVersionFormat { message: String },

    #[error("Invalid file hash: {message}")]
    InvalidFileHash { message: String },

    #[error("Invalid file path: {message}")]
    InvalidFilePath { message: String },
}

/// Domain Result type
pub type DomainResult<T> = std::result::Result<T, DomainError>;
