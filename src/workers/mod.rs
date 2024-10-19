use crate::{config::AkConfig, error::Result, mailers::Mailer};
use object_store::aws::AmazonS3;
use sea_orm::DatabaseConnection;
use std::{sync::Arc, time::Duration};
use tokio::{
    spawn,
    task::AbortHandle,
    time::{interval, MissedTickBehavior},
};
use tracing::error;

pub mod check_and_download;

#[derive(Clone, Debug)]
pub struct WorkerOptions {
    pub mailer: Arc<Mailer>,
    pub conn: DatabaseConnection,
    pub s3: Arc<AmazonS3>,
    pub ak: AkConfig,
}

pub async fn start(opt: WorkerOptions) -> Result<AbortHandle> {
    let worker = check_and_download::CheckAndDownload::new(opt.clone())?;
    let handle = spawn(async move {
        let mut interval = interval(Duration::from_secs(2 * 60));
        interval.set_missed_tick_behavior(MissedTickBehavior::Skip);
        loop {
            interval.tick().await;
            if let Err(e) = worker.perform().await {
                opt.mailer.notify_error(&e).unwrap();
                error!("check and download error: {}", e);
            }
        }
    });
    Ok(handle.abort_handle())
}
