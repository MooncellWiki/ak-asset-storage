use axum::{
    http::{header::CONTENT_TYPE, HeaderValue, Response},
    routing::{get_service, MethodRouter},
    Router,
};
use std::{path::PathBuf, time::Duration};
use tower::ServiceBuilder;
use tower_http::{
    compression::CompressionLayer, services::ServeDir, set_header::SetResponseHeaderLayer,
    timeout::RequestBodyTimeoutLayer,
};

pub fn apply_axum_middleware(router: Router) -> Router {
    router
        .layer(RequestBodyTimeoutLayer::new(Duration::from_secs(10)))
        .layer(CompressionLayer::new())
}

fn set_text_plain_charset<B>(response: &Response<B>) -> Option<HeaderValue> {
    response
        .headers()
        .get(CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .filter(|ct| ct.starts_with("text/plain"))
        .map(|_| HeaderValue::from_static("text/plain; charset=utf-8"))
}

pub fn serve_dir_with_charset(path: PathBuf) -> MethodRouter {
    let header_layer = SetResponseHeaderLayer::overriding(CONTENT_TYPE, set_text_plain_charset);

    get_service(
        ServiceBuilder::new()
            .layer(header_layer)
            .service(ServeDir::new(path)),
    )
}
