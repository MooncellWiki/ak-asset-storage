use crate::{app::Context, views::utils::json};
use axum::{
    extract::State,
    http::{header, StatusCode, Uri},
    response::{Html, IntoResponse, Response},
};
use rust_embed::Embed;
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

#[derive(Embed)]
#[folder = "../../packages/ui/dist"]
struct Assets;
static INDEX_HTML: &str = "index.html";
pub async fn static_handler(uri: Uri) -> impl IntoResponse {
    let path = uri.path().trim_start_matches('/');

    if path.is_empty() || path == INDEX_HTML {
        return index_html();
    }

    match Assets::get(path) {
        Some(content) => {
            let mime = mime_guess::from_path(path).first_or_octet_stream();

            ([(header::CONTENT_TYPE, mime.as_ref())], content.data).into_response()
        }
        None => Assets::get(INDEX_HTML)
            .map(|content| {
                let mime = mime_guess::from_path(path).first_or_octet_stream();
                ([(header::CONTENT_TYPE, mime.as_ref())], content.data).into_response()
            })
            .unwrap_or((StatusCode::NOT_FOUND, "404").into_response()),
    }
}
fn index_html() -> Response {
    match Assets::get(INDEX_HTML) {
        Some(content) => Html(content.data).into_response(),
        None => not_found(),
    }
}
fn not_found() -> Response {
    (StatusCode::NOT_FOUND, "404").into_response()
}

pub fn route() -> OpenApiRouter<Context> {
    OpenApiRouter::new()
        .routes(routes!(ping))
        .routes(routes!(health))
}
