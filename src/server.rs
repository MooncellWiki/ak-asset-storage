use std::sync::Arc;

use crate::{
    app::{AppState, Context},
    config::Config,
    db,
    middleware::apply_axum_middleware,
    router::build_axum_router,
    sentry,
    utils::{self, shutdown::shutdown_signal},
};
use anyhow::Result;
use tokio::net::TcpListener;
use tracing::info;

pub async fn start(config: &Config) -> Result<()> {
    let _sentry = sentry::init(&config.sentry);
    utils::tracing::init(&config.logger);
    let state = AppState(Arc::new(Context::new(config).await?));
    db::migrate(&state.database).await?;
    let router = build_axum_router(state);
    let router = apply_axum_middleware(router);
    let listener = TcpListener::bind(config.server.full_url()).await?;
    info!("Server is running on {}", config.server.full_url());
    axum::serve(listener, router)
        .with_graceful_shutdown(shutdown_signal())
        .await?;
    info!("Server has gracefully shutdown!");
    Ok(())
}
