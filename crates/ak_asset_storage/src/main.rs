use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

use ak_asset_storage::{
    app::{boot, boot_server_and_worker},
    config::Config,
    error::Result,
    tasks::seed::seed,
};
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
#[command(propagate_version = true)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Start an app
    Start {
        #[arg(short, long, action)]
        config: Option<String>,
    },
    Version {},
    Seed {
        #[arg(short, long, action)]
        config: Option<String>,
        #[arg(long, action)]
        csv_path: String,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Commands::Start { config } => {
            let config = Config::new(Path::new(
                &config.unwrap_or_else(|| "config.toml".to_string()),
            ))?;
            let conn = boot(&config).await?;
            boot_server_and_worker(&config, conn).await?;
        }
        Commands::Version {} => println!(
            "{} ({})",
            env!("CARGO_PKG_VERSION"),
            option_env!("BUILD_SHA")
                .or(option_env!("GITHUB_SHA"))
                .unwrap_or("dev")
        ),
        Commands::Seed { config, csv_path } => {
            let config = Config::new(Path::new(
                &config.unwrap_or_else(|| "config.toml".to_string()),
            ))?;
            let conn = boot(&config).await?;
            seed(
                PathBuf::from(&csv_path),
                conn,
                Arc::new(config.s3.client()?),
                config.ak,
            )
            .await?;
        }
    }

    Ok(())
}
