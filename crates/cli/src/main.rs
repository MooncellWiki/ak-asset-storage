// CLI main entry point

use cli::app::run;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Parse CLI arguments and run
    run().await?;

    Ok(())
}
