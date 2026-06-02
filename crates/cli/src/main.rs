// CLI main entry point
use ak_asset_storage_cli::app::{Cli, run};
use anyhow::anyhow;
use clap::Parser;
use mimalloc::MiMalloc;

#[global_allocator]
static GLOBAL: MiMalloc = MiMalloc;
fn main() -> Result<(), Box<dyn std::error::Error>> {
    rustls::crypto::ring::default_provider()
        .install_default()
        .map_err(|_| anyhow!("failed to install rustls crypto provider"))?;

    let cli = Cli::parse();
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(cli.worker_threads)
        .enable_all()
        .build()?;

    // Parse CLI arguments and run
    runtime.block_on(run(cli))?;

    Ok(())
}
