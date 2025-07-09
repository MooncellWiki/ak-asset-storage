use crate::{routes::build_router, state::init_state_with_pg};
use anyhow::Result;
use application::ConfigProvider;
use infrastructure::shutdown_signal;
use tokio::net::TcpListener;
use tracing::info;

pub async fn start(config: &impl ConfigProvider) -> Result<()> {
    // // Build router
    let listener = TcpListener::bind(config.server_config().full_url()).await?;

    let router = build_router(init_state_with_pg(config).await);
    axum::serve(listener, router)
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    info!("Web server has gracefully shutdown");
    Ok(())
}
