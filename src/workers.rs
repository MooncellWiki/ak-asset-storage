use crate::{
    config::{Ak, Config},
    db,
    mailers::Mailer,
    sentry,
    utils::{self, shutdown::shutdown_signal},
};
use anyhow::Result;
use object_store::aws::AmazonS3;
use reqwest::Client;
use std::{sync::Arc, time::Duration};
use tokio::{
    spawn,
    time::{interval, MissedTickBehavior},
};
use tracing::info;
pub mod check;
pub mod download;

#[derive(Clone, Debug)]
pub struct WorkerContext {
    pub mailer: Option<Arc<Mailer>>,
    pub conn: db::Pool,
    pub s3: AmazonS3,
    pub ak: Ak,
    pub client: Client,
}
impl WorkerContext {
    pub async fn new(config: &Config) -> Result<Self> {
        Ok(Self {
            mailer: Some(Arc::new(Mailer::new(&config.mailer, &config.server.host)?)),
            conn: db::connect(&config.database).await?,
            s3: config.s3.client().unwrap(),
            ak: config.ak.clone(),
            client: reqwest::ClientBuilder::new()
                .timeout(Duration::from_secs(60 * 5))
                .build()?,
        })
    }
}

pub async fn start(config: Config) -> Result<()> {
    let _sentry = sentry::init(&config.sentry);
    utils::tracing::init(&config.logger);
    let ctx = WorkerContext::new(&config).await?;
    let checker = check::Check { ctx: ctx.clone() };
    let check_handle = spawn(async move {
        let mut interval = interval(Duration::from_secs(2 * 60));
        interval.set_missed_tick_behavior(MissedTickBehavior::Skip);
        loop {
            interval.tick().await;
            checker.perform().await;
        }
    });
    let downloader = download::Download { ctx };
    let download_handle = spawn(async move {
        let mut interval = interval(Duration::from_secs(2 * 60));
        interval.set_missed_tick_behavior(MissedTickBehavior::Skip);
        loop {
            interval.tick().await;
            downloader.perform().await;
        }
    });
    shutdown_signal().await;
    info!("shutdown signal received");
    check_handle.abort();
    info!("checker aborted");
    download_handle.abort();
    info!("downloader aborted");
    Ok(())
}
