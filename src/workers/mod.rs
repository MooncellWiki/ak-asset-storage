use crate::{
    config::{Ak, Config},
    db,
    mailers::Mailer,
    utils::shutdown::shutdown_signal,
};
use anyhow::Result;
use object_store::aws::AmazonS3;
use std::{sync::Arc, time::Duration};
use tokio::{
    spawn,
    time::{interval, MissedTickBehavior},
};
use tracing::info;
pub mod check;
pub mod download;

#[derive(Clone, Debug)]
pub struct WorkerOptions {
    pub mailer: Arc<Mailer>,
    pub conn: db::Pool,
    pub s3: Arc<AmazonS3>,
    pub ak: Ak,
}
impl WorkerOptions {
    pub async fn new(config: Config) -> Result<Self> {
        Ok(Self {
            mailer: Arc::new(Mailer::new(&config.mailer, &config.server.host).unwrap()),
            conn: db::connect(&config.database).await?,
            s3: Arc::new(config.s3.client().unwrap()),
            ak: config.ak,
        })
    }
}

pub async fn start(config: Config) -> Result<()> {
    let opt = WorkerOptions::new(config).await?;
    let client = reqwest::ClientBuilder::new()
        .timeout(Duration::from_secs(60 * 5))
        .build()?;
    let checker = check::Check {
        conf_url: opt.ak.conf_url.clone(),
        asset_url: opt.ak.asset_url.clone(),
        client: client.clone(),
        conn: opt.conn.clone(),
        mailer: Some(opt.mailer.clone()),
    };
    let check_handle = spawn(async move {
        let mut interval = interval(Duration::from_secs(2 * 60));
        interval.set_missed_tick_behavior(MissedTickBehavior::Skip);
        loop {
            interval.tick().await;
            checker.perform().await;
        }
    });
    let downloader = download::Download {
        asset_url: opt.ak.asset_url.clone(),
        client,
        conn: opt.conn,
        s3: opt.s3,
        mailer: Some(opt.mailer),
    };
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
