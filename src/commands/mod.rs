mod import_manifest;
mod seed;
mod worker;

use crate::{api, config::AppSettings, runtime};
use anyhow::Result;
use clap::Parser;
use std::{
    path::{Path, PathBuf},
    sync::Arc,
};
use tracing::info;

fn parse_worker_threads(value: &str) -> Result<usize, String> {
    let value = value
        .parse::<usize>()
        .map_err(|_| String::from("worker-threads must be a positive integer"))?;
    if value == 0 {
        return Err(String::from("worker-threads must be greater than 0"));
    }
    Ok(value)
}

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
#[command(propagate_version = true)]
pub struct Cli {
    #[arg(long, global = true, default_value_t = 4, value_parser = parse_worker_threads)]
    pub worker_threads: usize,
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Parser, Debug)]
pub enum Commands {
    Server {
        #[arg(short, long, default_value = "config.toml")]
        config: String,
    },
    Worker {
        #[arg(short, long, default_value = "config.toml")]
        config: String,
        #[arg(long, default_value = "5")]
        concurrent: usize,
        #[arg(long, default_value = "120")]
        poll_interval_seconds: u64,
    },
    Seed {
        #[arg(short, long, default_value = "config.toml")]
        config: String,
        #[arg(long)]
        csv_path: PathBuf,
        #[arg(long, default_value = "5")]
        concurrent: usize,
    },
    ImportManifest {
        #[arg(short, long, default_value = "config.toml")]
        config: String,
        #[arg(long)]
        res_version: String,
    },
    Version,
}

fn init(config: &str) -> Result<(Arc<AppSettings>, sentry::ClientInitGuard)> {
    let settings = Arc::new(AppSettings::load(Path::new(config))?);
    let sentry = runtime::init_tracing(&settings.logger, &settings.sentry)?;
    Ok((settings, sentry))
}

pub async fn run(cli: Cli) -> Result<()> {
    match cli.command {
        Commands::Server { config } => {
            let (settings, _sentry) = init(&config)?;
            let state = api::AppState::from_settings(settings.clone()).await?;
            state.database.migrate().await?;
            let listener = tokio::net::TcpListener::bind(settings.server.full_url()).await?;
            info!("Server is running on {}", settings.server.full_url());
            axum::serve(listener, api::build_router(state))
                .with_graceful_shutdown(runtime::shutdown_signal())
                .await?;
            Ok(())
        }
        Commands::Worker {
            config,
            concurrent,
            poll_interval_seconds,
        } => {
            let (settings, _sentry) = init(&config)?;
            worker::execute(settings.as_ref(), concurrent, poll_interval_seconds)
                .await
                .map_err(anyhow::Error::from)
        }
        Commands::Seed {
            config,
            csv_path,
            concurrent,
        } => {
            let (settings, _sentry) = init(&config)?;
            seed::execute(settings.as_ref(), &csv_path, concurrent)
                .await
                .map_err(anyhow::Error::from)
        }
        Commands::ImportManifest {
            config,
            res_version,
        } => {
            let (settings, _sentry) = init(&config)?;
            import_manifest::execute(settings.as_ref(), &res_version)
                .await
                .map_err(anyhow::Error::from)
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
