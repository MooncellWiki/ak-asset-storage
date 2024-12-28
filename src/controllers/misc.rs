use crate::{app::AppState, views::utils::json};
use axum::{
    debug_handler,
    http::{header, StatusCode, Uri},
    response::{Html, IntoResponse, Response},
};
use rust_embed::Embed;
use serde::Serialize;

#[derive(utoipa::ToSchema, Serialize)]
struct Health {
    pub ok: bool,
}

#[debug_handler]
#[utoipa::path(get, path = "/_ping", responses((status = OK, body = Health)))]
pub async fn ping() -> Response {
    json(Health { ok: true })
}

#[debug_handler(state = AppState)]
#[utoipa::path(get, path = "/_health", responses((status = OK, body = Health)))]
pub async fn health(ctx: AppState) -> Response {
    if let Err(error) = ctx.database.acquire().await {
        tracing::error!(err.msg = %error, err.detail = ?error, "db_acquire_error");
        return json(Health { ok: false });
    }
    json(Health { ok: true })
}

#[derive(Embed)]
#[folder = "./dist"]
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
        None => Assets::get(INDEX_HTML).map_or(
            (StatusCode::NOT_FOUND, "404").into_response(),
            |content| {
                (
                    [(header::CONTENT_TYPE, "text/html; charset=utf-8")],
                    content.data,
                )
                    .into_response()
            },
        ),
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
