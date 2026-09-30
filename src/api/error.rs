use crate::AppError;
use axum::{Json, http::StatusCode, response::IntoResponse};
use serde::Serialize;

#[derive(thiserror::Error, Debug)]
pub enum WebError {
    #[error("Internal Server Error:\n{0}")]
    CustomApiError(AppError),
    #[error("Not found")]
    NotFound,
    #[error("Service Unavailable:\n{0}")]
    ServiceUnavailable(anyhow::Error),
    #[error("Service Unavailable: {0}")]
    Unavailable(String),
    #[error("Unauthorized: {0}")]
    Unauthorized(String),
    #[error("Bad Request: {0}")]
    BadRequest(String),
}

#[derive(Debug, Serialize)]
pub struct ApiErrorDetail {
    detail: String,
}

impl From<WebError> for ApiErrorDetail {
    /// Internal failures are reduced to a fixed message: the full error chain
    /// (SQL, file paths, upstream responses) is logged in `into_response` and
    /// must not be echoed back to anonymous callers.
    fn from(value: WebError) -> Self {
        let detail = match value {
            WebError::CustomApiError(..) => "Internal Server Error".to_string(),
            WebError::ServiceUnavailable(..) => "Service Unavailable".to_string(),
            other => other.to_string(),
        };
        Self { detail }
    }
}

impl From<AppError> for WebError {
    fn from(err: AppError) -> Self {
        match err {
            err @ AppError::Application(..) => Self::CustomApiError(err),
            err @ AppError::ExternalService(..) => Self::ServiceUnavailable(err.into()),
            AppError::Unavailable(message) => Self::Unavailable(message),
            AppError::InvalidInput(message) => Self::BadRequest(message),
        }
    }
}

impl IntoResponse for WebError {
    fn into_response(self) -> axum::response::Response {
        tracing::error!(error.msg = %self, error.details = ?self, "controller_error");
        match self {
            err @ Self::CustomApiError(..) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiErrorDetail::from(err)),
            )
                .into_response(),
            err @ Self::NotFound => {
                (StatusCode::NOT_FOUND, Json(ApiErrorDetail::from(err))).into_response()
            }
            err @ (Self::ServiceUnavailable(..) | Self::Unavailable(..)) => (
                StatusCode::SERVICE_UNAVAILABLE,
                Json(ApiErrorDetail::from(err)),
            )
                .into_response(),
            err @ Self::Unauthorized(..) => {
                (StatusCode::UNAUTHORIZED, Json(ApiErrorDetail::from(err))).into_response()
            }
            err @ Self::BadRequest(..) => {
                (StatusCode::BAD_REQUEST, Json(ApiErrorDetail::from(err))).into_response()
            }
        }
    }
}

pub type WebResult<T> = std::result::Result<T, WebError>;
