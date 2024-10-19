use axum::{extract::State, response::Response, routing::get, Router};
use serde::Serialize;

use crate::{app::Context, views::utils::json};

#[derive(Serialize)]
struct Health {
    pub ok: bool,
}

async fn ping() -> Response {
    json(Health { ok: true })
}

async fn health(State(ctx): State<Context>) -> Response {
    let is_ok = match ctx.conn.ping().await {
        Ok(()) => true,
        Err(error) => {
            tracing::error!(err.msg = %error, err.detail = ?error, "health_db_ping_error");
            false
        }
    };
    json(Health { ok: is_ok })
}

pub fn route() -> Router<Context> {
    Router::new()
        .route("/_ping", get(ping))
        .route("/_health", get(health))
}
