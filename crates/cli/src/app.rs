use crate::commands::{seed, worker};
use anyhow::Result;
use application::ConfigProvider;
use clap::Parser;
use infrastructure::{init_tracing, AppSettings, InfraConfigProvider};
use std::path::{Path, PathBuf};

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
#[command(propagate_version = true)]
pub enum Commands {
    /// Start the web server
    Server {
        #[arg(short, long, default_value = "config.toml")]
        config: String,
    },
    /// Start the background worker
    Worker {
        #[arg(short, long, default_value = "config.toml")]
        config: String,
    },
    /// Seed the database with initial data
    Seed {
        #[arg(short, long, default_value = "config.toml")]
        config: String,
        #[arg(long)]
        csv_path: PathBuf,
    },
    /// Show version information
    Version,
}
fn init(config: &str) -> Result<InfraConfigProvider> {
    let config = InfraConfigProvider {
        settings: AppSettings::new(Path::new(config))?,
    };
    init_tracing(config.logger_config());
    Ok(config)
}

pub async fn run() -> Result<()> {
    let cli = Commands::parse();
    match cli {
        Commands::Server { config } => {
            let config = init(&config)?;
            web::start(&config).await?;
            Ok(())
        }
        Commands::Worker { config } => {
            let config = init(&config)?;
            worker::execute(&config).await?;
            Ok(())
        }
        Commands::Seed { config, csv_path } => {
            let config = init(&config)?;
            seed::execute(&config, &csv_path).await?;
            Ok(())
        }
        Commands::Version => {
            println!(
                "{} ({})",
                env!("CARGO_PKG_VERSION"),
                option_env!("BUILD_SHA")
                    .or(option_env!("GITHUB_SHA"))
                    .unwrap_or("dev")
            );
            Ok(())
        }
    }
}
