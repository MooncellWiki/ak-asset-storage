use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::Serialize;
#[derive(thiserror::Error, Debug)]
pub enum Error {
    /// 501 Internal Server Error
    #[error(transparent)]
    CustomApiError(#[from] anyhow::Error),
    /// 404 Not Found
    #[error("Not found")]
    NotFound,

    /// 503 Service Unavailable
    #[error("Service Unavailable")]
    ServiceUnavailable,
}
impl From<sqlx::Error> for Error {
    fn from(value: sqlx::Error) -> Self {
        match value {
            sqlx::Error::RowNotFound => Self::NotFound,
            sqlx::Error::Io(_)
            | sqlx::Error::Tls(_)
            | sqlx::Error::Protocol(_)
            | sqlx::Error::PoolTimedOut
            | sqlx::Error::PoolClosed => Self::ServiceUnavailable,
            _ => Self::CustomApiError(anyhow::anyhow!(value)),
        }
    }
}

#[derive(Debug, Serialize)]
pub struct ApiErrorDetail {
    detail: String,
}

impl<T: ToString> From<T> for ApiErrorDetail {
    fn from(value: T) -> Self {
        Self {
            detail: value.to_string(),
        }
    }
}

pub type Result<T> = std::result::Result<T, Error>;

impl IntoResponse for Error {
    fn into_response(self) -> Response {
        tracing::error!(error.msg = %self,error.details = ?self,"controller_error");

        match self {
            Self::CustomApiError(error) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiErrorDetail::from(error)),
            )
                .into_response(),
            Self::NotFound => StatusCode::NOT_FOUND.into_response(),
            Self::ServiceUnavailable => StatusCode::SERVICE_UNAVAILABLE.into_response(),
        }
    }
}
