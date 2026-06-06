use axum::{
    Json,
    response::{IntoResponse, Response},
};
use serde::Serialize;

pub fn json<T: Serialize>(json: T) -> Response {
    Json(json).into_response()
}
