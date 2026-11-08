mod import_item_demand;
mod import_launcher_manifest;
mod import_manifest;
mod import_story_usage;
mod seed;
mod worker;

use crate::{api, config::AppSettings, external::plocate, runtime};
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
        /// Seconds between scans of the watched gamedata, manifest and item demand files
        #[arg(long, default_value = "10", value_parser = clap::value_parser!(u64).range(1..))]
        watch_interval_seconds: u64,
        /// Seconds a watched file must stay unchanged before it is imported;
        /// keep it above --watch-interval-seconds
        #[arg(long, default_value = "30")]
        watch_debounce_seconds: u64,
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
    /// Import one or more historical PC launcher manifests.
    ImportLauncherManifest {
        #[arg(short, long, default_value = "config.toml")]
        config: String,
        /// Release metadata JSON files, in import order
        #[arg(required = true, num_args = 1.., value_name = "META_JSON")]
        metadata: Vec<PathBuf>,
    },
    ImportItemDemand {
        #[arg(short, long, default_value = "config.toml")]
        config: String,
    },
    ImportStoryUsage {
        #[arg(short, long, default_value = "config.toml")]
        config: String,
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
            plocate::spawn_update_task(
                state.plocate.clone(),
                std::time::Duration::from_secs(settings.torappu.plocate.update_interval_seconds),
            );
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
            watch_interval_seconds,
            watch_debounce_seconds,
        } => {
            let (settings, _sentry) = init(&config)?;
            let watch_timing = crate::worker::WatchTiming {
                interval: std::time::Duration::from_secs(watch_interval_seconds),
                debounce: std::time::Duration::from_secs(watch_debounce_seconds),
            };
            worker::execute(
                settings.as_ref(),
                concurrent,
                poll_interval_seconds,
                watch_timing,
            )
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
        Commands::ImportLauncherManifest { config, metadata } => {
            let (settings, _sentry) = init(&config)?;
            import_launcher_manifest::execute(settings.as_ref(), &metadata).await
        }
        Commands::ImportItemDemand { config } => {
            let (settings, _sentry) = init(&config)?;
            import_item_demand::execute(settings.as_ref())
                .await
                .map_err(anyhow::Error::from)
        }
        Commands::ImportStoryUsage { config } => {
            let (settings, _sentry) = init(&config)?;
            import_story_usage::execute(settings.as_ref())
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
