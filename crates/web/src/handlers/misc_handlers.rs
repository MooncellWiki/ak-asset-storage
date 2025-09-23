use crate::{dto::responses::Health, state::AppState};
use ak_asset_storage_application::Repository;
use axum::{
    debug_handler,
    extract::State,
    http::{header, StatusCode, Uri},
    response::{Html, IntoResponse, Response},
    Json,
};
use rust_embed::Embed;

/// /_ping
#[debug_handler]
#[utoipa::path(get, path = "/_ping", responses((status = OK, body = Health)))]
pub async fn ping() -> Json<Health> {
    Json(Health { ok: true })
}
/// /_health
#[debug_handler]
#[utoipa::path(get, path = "/_health", responses((status = OK, body = Health)))]
pub async fn health(State(state): State<AppState>) -> Json<Health> {
    Json(Health {
        ok: state.repository.health_check().await,
    })
}

#[derive(Embed)]
#[folder = "../../dist"]
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
        None => Assets::get(INDEX_HTML).map_or_else(
            || (StatusCode::NOT_FOUND, "404").into_response(),
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
