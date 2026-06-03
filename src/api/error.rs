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
    fn from(value: WebError) -> Self {
        Self {
            detail: value.to_string(),
        }
    }
}

impl From<AppError> for WebError {
    fn from(err: AppError) -> Self {
        match err {
            err @ AppError::Application(..) => Self::CustomApiError(err),
            err @ AppError::ExternalService(..) => Self::ServiceUnavailable(err.into()),
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
            err @ Self::ServiceUnavailable(..) => (
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
