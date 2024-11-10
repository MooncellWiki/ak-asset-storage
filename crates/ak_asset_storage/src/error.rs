use axum::{
    extract::rejection::JsonRejection,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use lettre::{address::AddressError, transport::smtp};

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error(transparent)]
    Axum(#[from] axum::http::Error),

    #[error(transparent)]
    JSON(#[from] serde_json::Error),

    #[error(transparent)]
    JsonRejection(#[from] JsonRejection),

    #[error("cannot parse `{1}`: {0}")]
    TOMLFile(#[source] toml::de::Error, String),

    #[error(transparent)]
    TOML(#[from] toml::de::Error),

    #[error(transparent)]
    EnvVar(#[from] std::env::VarError),

    #[error(transparent)]
    Smtp(#[from] smtp::Error),

    #[error(transparent)]
    IO(#[from] std::io::Error),

    #[error(transparent)]
    DB(#[from] sea_orm::DbErr),

    #[error(transparent)]
    ParseAddress(#[from] AddressError),

    // API
    #[error("{0}")]
    Unauthorized(String),

    // API
    #[error("not found")]
    NotFound,

    #[error("{0}")]
    BadRequest(String),

    #[error(transparent)]
    Reqwest(#[from] reqwest::Error),

    #[error(transparent)]
    S3(#[from] object_store::Error),

    #[error(transparent)]
    Any(#[from] anyhow::Error),
}

pub type Result<T> = std::result::Result<T, Error>;

impl IntoResponse for Error {
    fn into_response(self) -> Response {
        tracing::error!(error.msg = %self,error.details = ?self,"controller_error");

        match self {
            Self::NotFound => StatusCode::NOT_FOUND,
            Self::Unauthorized(err) => {
                tracing::warn!(err);
                StatusCode::UNAUTHORIZED
            }
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        }
        .into_response()
    }
}

pub fn any_anyhow<T: Into<anyhow::Error>>(err: T) -> Error {
    Error::Any(err.into())
}
