use axum::{
    Json,
    response::{IntoResponse, Response},
};
use serde::Serialize;

pub fn json<T: Serialize>(json: T) -> Response {
    Json(json).into_response()
}

/// Escapes LIKE metacharacters so `q` matches a literal substring; the
/// default LIKE/ILIKE escape character is the backslash.
pub fn escape_like(input: &str) -> String {
    let mut escaped = String::with_capacity(input.len());
    for ch in input.chars() {
        if matches!(ch, '%' | '_' | '\\') {
            escaped.push('\\');
        }
        escaped.push(ch);
    }
    escaped
}
