use axum::{
    Router,
    http::{HeaderValue, Response, header::CONTENT_TYPE},
    routing::{MethodRouter, get_service},
};
use std::{path::PathBuf, time::Duration};
use tower::ServiceBuilder;
use tower_http::{
    compression::{CompressionLayer, DefaultPredicate, Predicate, predicate::NotForContentType},
    services::ServeDir,
    set_header::SetResponseHeaderLayer,
    timeout::RequestBodyTimeoutLayer,
};

pub fn apply_axum_middleware(router: Router) -> Router {
    router
        .layer(RequestBodyTimeoutLayer::new(Duration::from_secs(10)))
        // Raw downloads are mostly compressed already; leaving them alone
        // also keeps their Content-Length, so browsers can show progress.
        .layer(CompressionLayer::new().compress_when(
            DefaultPredicate::new().and(NotForContentType::const_new("application/octet-stream")),
        ))
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
    use super::apply_axum_middleware;
    use axum::{
        Router,
        body::Body,
        http::{
            Request,
            header::{ACCEPT_ENCODING, CONTENT_ENCODING, CONTENT_LENGTH, CONTENT_TYPE},
        },
        routing::get,
    };
    use tower::ServiceExt;

    #[tokio::test]
    async fn raw_downloads_keep_their_length_while_text_is_compressed() {
        let router = apply_axum_middleware(
            Router::new()
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
        let raw = fetch("/raw").await;
        assert!(!raw.headers().contains_key(CONTENT_ENCODING));
        assert_eq!(raw.headers()[CONTENT_LENGTH], "4096");
        assert_eq!(fetch("/text").await.headers()[CONTENT_ENCODING], "gzip");
    }
}
