use axum::{
    Router,
    http::{
        Extensions, HeaderMap, HeaderValue, Response, StatusCode, Version, header::CONTENT_TYPE,
    },
    routing::{MethodRouter, get_service},
};
use std::{path::PathBuf, time::Duration};
use tower::ServiceBuilder;
use tower_http::{
    compression::{CompressionLayer, DefaultPredicate, Predicate},
    services::ServeDir,
    set_header::SetResponseHeaderLayer,
    timeout::RequestBodyTimeoutLayer,
};

/// Response extension that skips compression, for bodies that would not
/// shrink and should keep their `Content-Length` so browsers show progress.
#[derive(Clone, Copy, Debug)]
pub struct Uncompressed;

pub fn apply_axum_middleware(router: Router) -> Router {
    router
        .layer(RequestBodyTimeoutLayer::new(Duration::from_secs(10)))
        .layer(
            CompressionLayer::new().compress_when(DefaultPredicate::new().and(
                |_: StatusCode, _: Version, _: &HeaderMap, extensions: &Extensions| {
                    extensions.get::<Uncompressed>().is_none()
                },
            )),
        )
}

fn set_text_plain_charset<B>(response: &Response<B>) -> Option<HeaderValue> {
    response
        .headers()
        .get(CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .filter(|content_type| content_type.starts_with("text/plain"))
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

#[cfg(test)]
mod tests {
    use super::{Uncompressed, apply_axum_middleware};
    use axum::{
        Extension, Router,
        body::Body,
        http::{
            Request,
            header::{ACCEPT_ENCODING, CONTENT_ENCODING, CONTENT_LENGTH, CONTENT_TYPE},
        },
        routing::get,
    };
    use tower::ServiceExt;

    #[tokio::test]
    async fn only_marked_responses_skip_compression() {
        let router = apply_axum_middleware(
            Router::new()
                .route(
                    "/marked",
                    get(|| async {
                        (
                            Extension(Uncompressed),
                            [(CONTENT_TYPE, "application/octet-stream")],
                            vec![0; 4096],
                        )
                    }),
                )
                // Served files with an unknown extension are octet-streams too.
                .route(
                    "/raw",
                    get(|| async { ([(CONTENT_TYPE, "application/octet-stream")], vec![0; 4096]) }),
                )
                .route("/text", get(|| async { "a".repeat(4096) })),
        );
        let fetch = |uri: &'static str| {
            let router = router.clone();
            async move {
                router
                    .oneshot(
                        Request::get(uri)
                            .header(ACCEPT_ENCODING, "gzip")
                            .body(Body::empty())
                            .expect("request"),
                    )
                    .await
                    .expect("response")
            }
        };
        let marked = fetch("/marked").await;
        assert!(!marked.headers().contains_key(CONTENT_ENCODING));
        assert_eq!(marked.headers()[CONTENT_LENGTH], "4096");
        assert_eq!(fetch("/raw").await.headers()[CONTENT_ENCODING], "gzip");
        assert_eq!(fetch("/text").await.headers()[CONTENT_ENCODING], "gzip");
    }
}
