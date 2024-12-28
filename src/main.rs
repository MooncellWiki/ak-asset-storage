use ak_asset_storage::{config::Config, error::Result, server::start, tasks::seed::seed, workers};
use clap::{command, Parser};
use std::path::{Path, PathBuf};

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
#[command(propagate_version = true)]
enum Commands {
    /// Start an app
    Server {
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
    Worker {
        #[arg(short, long, action)]
        config: Option<String>,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Commands::parse();
    match cli {
        Commands::Server { config } => {
            let config = Config::new(Path::new(
                &config.unwrap_or_else(|| "config.toml".to_string()),
            ))?;
            start(&config).await?;
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
            seed(PathBuf::from(&csv_path), &config).await?;
        }
        Commands::Worker { config } => {
            let config = Config::new(Path::new(
                &config.unwrap_or_else(|| "config.toml".to_string()),
            ))?;
            workers::start(config).await?;
        }
    }

    Ok(())
}
