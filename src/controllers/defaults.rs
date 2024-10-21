use crate::{app::Context, views::utils::json};
use axum::{extract::State, response::Response};
use serde::Serialize;
use utoipa_axum::{router::OpenApiRouter, routes};

#[derive(utoipa::ToSchema, Serialize)]
struct Health {
    pub ok: bool,
}
#[utoipa::path(get, path = "/_ping", responses((status = OK, body = Health)))]
async fn ping() -> Response {
    json(Health { ok: true })
}

#[utoipa::path(get, path = "/_health", responses((status = OK, body = Health)))]
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

pub fn route() -> OpenApiRouter<Context> {
    OpenApiRouter::new()
        .routes(routes!(ping))
        .routes(routes!(health))
}
