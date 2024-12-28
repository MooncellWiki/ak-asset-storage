use std::sync::Arc;

use crate::{
    app::{AppState, Context},
    config::Config,
    db,
    router::build_axum_router,
    utils::{self, shutdown::shutdown_signal},
};
use anyhow::Result;
use tokio::net::TcpListener;
use tracing::info;

pub async fn start(config: &Config) -> Result<()> {
    utils::tracing::init(&config.logger);
    let state = AppState(Arc::new(Context::new(config).await?));
    db::migrate(&state.database).await?;
    let router = build_axum_router(state);
    let listener = TcpListener::bind(config.server.full_url()).await?;
    info!("Server is running on {}", config.server.full_url());
    axum::serve(listener, router)
        .with_graceful_shutdown(shutdown_signal())
        .await?;
    info!("Server has gracefully shutdown!");
    Ok(())
}
