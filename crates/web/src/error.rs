use application::AppError;
use axum::{http::StatusCode, response::IntoResponse, Json};
use serde::Serialize;

#[derive(thiserror::Error, Debug)]
pub enum WebError {
    /// 501 Internal Server Error
    #[error("Internal Server Error: {0}")]
    CustomApiError(AppError),
    /// 404 Not Found
    #[error("Not found")]
    NotFound,

    /// 503 Service Unavailable
    #[error("Service Unavailable")]
    ServiceUnavailable(anyhow::Error),
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
            err @ (AppError::Domain(..) | AppError::Application(..)) => Self::CustomApiError(err),
            err @ AppError::ExternalService(..) => Self::ServiceUnavailable(err.into()),
        }
    }
}

impl IntoResponse for WebError {
    fn into_response(self) -> axum::response::Response {
        tracing::error!(error.msg = %self,error.details = ?self,"controller_error");
        match self {
            err @ WebError::CustomApiError(..) => {
                (StatusCode::NOT_IMPLEMENTED, Json(ApiErrorDetail::from(err))).into_response()
            }
            err @ WebError::NotFound => {
                (StatusCode::NOT_FOUND, Json(ApiErrorDetail::from(err))).into_response()
            }
            err @ WebError::ServiceUnavailable(..) => (
                StatusCode::SERVICE_UNAVAILABLE,
                Json(ApiErrorDetail::from(err)),
            )
                .into_response(),
        }
    }
}

pub type WebResult<T> = std::result::Result<T, WebError>;
