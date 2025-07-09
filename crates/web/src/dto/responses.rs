use application::AppError;
use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde::Serialize;
use utoipa::ToSchema;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct VersionListItem {
    pub id: i32,
    pub res: String,
    pub client: String,
    pub is_ready: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct VersionDetail {
    pub id: i32,
    pub res: String,
    pub client: String,
    pub is_ready: bool,
    pub hot_update_list: String,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct BundleDetail {
    pub path: String,
    pub file: i32,
    pub hash: String,
    pub size: i32,
    pub version: i32,
    pub res: String,
    pub client: String,
    pub is_ready: bool,
}

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
                AppError::Domain(domain_error) => {
                    (StatusCode::INTERNAL_SERVER_ERROR, domain_error.to_string())
                }
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
