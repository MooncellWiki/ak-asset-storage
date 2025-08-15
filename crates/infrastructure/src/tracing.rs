use std::str::FromStr;

use ak_asset_storage_application::{LogFormat, LogLevel, LoggerConfig, SentryConfig};
use anyhow::Result;
use sentry::{integrations::tracing::EventFilter, types::Dsn};
use tracing::{level_filters::LevelFilter, Level, Metadata};
use tracing_subscriber::{
    fmt::{self, MakeWriter},
    layer::SubscriberExt,
    util::SubscriberInitExt,
    EnvFilter, Layer, Registry,
};

const MODULE_WHITELIST: &[&str] = &[
    "tower_http",
    "sqlx::query",
    "ak_asset_storage_infrastructure",
    "ak_asset_storage_application",
    "ak_asset_storage_cli",
    "ak_asset_storage_web",
];

fn init_env_filter(override_filter: Option<&String>, level: &LogLevel) -> EnvFilter {
    EnvFilter::try_from_default_env()
        .or_else(|_| {
            // user wanted a specific filter, don't care about our internal whitelist
            // or, if no override give them the default whitelisted filter (most common)
            override_filter.map_or_else(
                || {
                    EnvFilter::try_new(
                        MODULE_WHITELIST
                            .iter()
                            .map(|m| format!("{m}={level}"))
                            .collect::<Vec<_>>()
                            .join(","),
                    )
                },
                EnvFilter::try_new,
            )
        })
        .expect("logger initialization failed")
}

fn init_layer<W2>(
    make_writer: W2,
    format: &LogFormat,
    ansi: bool,
) -> Box<dyn Layer<Registry> + Sync + Send>
where
    W2: for<'writer> MakeWriter<'writer> + Sync + Send + 'static,
{
    match format {
        LogFormat::Compact => fmt::Layer::default()
            .with_ansi(ansi)
            .with_writer(make_writer)
            .compact()
            .boxed(),
        LogFormat::Pretty => fmt::Layer::default()
            .with_ansi(ansi)
            .with_writer(make_writer)
            .pretty()
            .boxed(),
        LogFormat::Json => fmt::Layer::default()
            .with_ansi(ansi)
            .with_writer(make_writer)
            .json()
            .boxed(),
    }
}

fn event_filter(metadata: &Metadata<'_>) -> EventFilter {
    match metadata.level() {
        &Level::ERROR | &Level::WARN => EventFilter::Event,
        _ => EventFilter::Ignore,
    }
}

pub fn init_tracing(
    config: &LoggerConfig,
    sentry_cfg: &SentryConfig,
) -> Result<sentry::ClientInitGuard> {
    let mut layers: Vec<Box<dyn Layer<Registry> + Sync + Send>> = Vec::new();
    if config.enable {
        let stdout_layer = init_layer(std::io::stdout, &config.format, true);
        layers.push(stdout_layer);
    }

    if !layers.is_empty() {
        let env_filter = init_env_filter(config.override_filter.as_ref(), &config.level);
        let sentry_layer = sentry::integrations::tracing::layer()
            .event_filter(event_filter)
            .with_filter(LevelFilter::INFO);

        tracing_subscriber::registry()
            .with(layers)
            .with(env_filter)
            .with(sentry_layer)
            .init();
    }
    Ok(sentry::init(sentry::ClientOptions {
        dsn: Some(Dsn::from_str(&sentry_cfg.dsn)?),
        release: sentry::release_name!(),
        traces_sample_rate: sentry_cfg.traces_sample_rate,
        ..Default::default()
    }))
}
