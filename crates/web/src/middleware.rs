use axum::Router;
use sentry::integrations::tower as sentry_tower;
use std::time::Duration;
use tower_http::{compression::CompressionLayer, timeout::RequestBodyTimeoutLayer};

pub fn apply_axum_middleware(router: Router) -> Router {
    router
        .layer(sentry_tower::NewSentryLayer::new_from_top())
        .layer(sentry_tower::SentryHttpLayer::new().enable_transaction())
        .layer(RequestBodyTimeoutLayer::new(Duration::from_secs(10)))
        .layer(CompressionLayer::new())
}
