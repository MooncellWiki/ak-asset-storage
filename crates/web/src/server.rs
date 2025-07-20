use crate::{routes::build_router, state::init_state_with_pg};
use ak_asset_storage_application::ConfigProvider;
use ak_asset_storage_application::Repository;
use ak_asset_storage_infrastructure::{config::InfraConfigProvider, shutdown_signal};
use anyhow::Result;
use tokio::net::TcpListener;
use tracing::info;

pub async fn start(config: &InfraConfigProvider) -> Result<()> {
    // // Build router
    let listener = TcpListener::bind(config.server_config().full_url()).await?;
    info!("Server is running on {}", config.server_config().full_url());
    let state = init_state_with_pg(config.clone()).await;
    state.repository.migrate().await?;
    let router = build_router(state);
    axum::serve(listener, router)
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    info!("Web server has gracefully shutdown");
    Ok(())
}
