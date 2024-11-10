use crate::{config::Ak, error::Result, mailers::Mailer};
use object_store::aws::AmazonS3;
use sea_orm::DatabaseConnection;
use std::{sync::Arc, time::Duration};
use tokio::{
    spawn,
    task::AbortHandle,
    time::{interval, MissedTickBehavior},
};
pub mod check;
pub mod download;

#[derive(Clone, Debug)]
pub struct WorkerOptions {
    pub mailer: Arc<Mailer>,
    pub conn: DatabaseConnection,
    pub s3: Arc<AmazonS3>,
    pub ak: Ak,
}

pub fn start(opt: WorkerOptions) -> Result<(AbortHandle, AbortHandle)> {
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
    Ok((check_handle.abort_handle(), download_handle.abort_handle()))
}
