use ak_asset_storage::commands;
use anyhow::anyhow;
use clap::Parser;
use mimalloc::MiMalloc;

#[global_allocator]
static GLOBAL: MiMalloc = MiMalloc;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    rustls::crypto::ring::default_provider()
        .install_default()
        .map_err(|_| anyhow!("failed to install rustls crypto provider"))?;

    let cli = commands::Cli::parse();
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(cli.worker_threads)
        .enable_all()
        .build()?;

    runtime.block_on(commands::run(cli))?;
    Ok(())
}
