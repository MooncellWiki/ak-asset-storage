use ak_asset_storage_application::AppError;
use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde::Serialize;
use utoipa::ToSchema;

#[derive(ToSchema, Serialize)]
pub struct Health {
    pub ok: bool,
}

#[derive(Debug, thiserror::Error)]
pub enum WebError {
    #[error("Application error: {0}")]
    App(#[from] AppError),

    #[error("Resource not found: {0}")]
    NotFound(String),
}

impl From<WebError> for Response {
    fn from(val: WebError) -> Self {
        match val {
            WebError::App(app_error) => match app_error {
                AppError::Application(error) => {
                    (StatusCode::INTERNAL_SERVER_ERROR, error.to_string())
                }
                AppError::ExternalService(error) => {
                    (StatusCode::SERVICE_UNAVAILABLE, error.to_string())
                }
            },
            WebError::NotFound(str) => {
                (StatusCode::NOT_FOUND, format!("Resource not found: {str}"))
            }
        }
        .into_response()
    }
}
