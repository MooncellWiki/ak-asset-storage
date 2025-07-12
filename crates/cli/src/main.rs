// CLI main entry point
use ak_asset_storage_cli::app::run;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Parse CLI arguments and run
    run().await?;

    Ok(())
}
