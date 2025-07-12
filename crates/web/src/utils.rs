use axum::{
    response::{IntoResponse, Response},
    Json,
};
use serde::Serialize;

pub fn json<T: Serialize>(json: T) -> Response {
    Json(json).into_response()
}
