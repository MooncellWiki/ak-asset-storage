// CLI main entry point
use ak_asset_storage_cli::app::run;
use mimalloc::MiMalloc;

#[global_allocator]
static GLOBAL: MiMalloc = MiMalloc;
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Parse CLI arguments and run
    run().await?;

    Ok(())
}
