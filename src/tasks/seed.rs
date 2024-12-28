use crate::{
    config::Config,
    sentry, utils,
    workers::{
        check::{self, RemoteVersion},
        download, WorkerContext,
    },
};
use anyhow::Result;
use sqlx::query;
use std::{fs, path::PathBuf};
use tracing::info;

pub async fn seed(path: PathBuf, config: &Config) -> Result<()> {
    let _sentry = sentry::init(&config.sentry);
    utils::tracing::init(&config.logger);
    let ctx = WorkerContext::new(config).await?;
    let content = fs::read_to_string(path)?;
    let versions = content
        .split('\n')
        .filter(|line| !line.is_empty())
        .map(|line| {
            let parts = line.split(',').collect::<Vec<&str>>();
            RemoteVersion {
                res_version: parts[1].to_string(),
                client_version: parts[2].to_string(),
            }
        })
        .collect::<Vec<RemoteVersion>>();
    let checker = check::Check { ctx: ctx.clone() };
    let downloader = download::Download { ctx: ctx.clone() };
    for remote in versions {
        if query!(
            "SELECT * FROM versions where client = $1 and res = $2",
            remote.client_version,
            remote.res_version
        )
        .fetch_optional(&ctx.conn)
        .await?
        .is_none()
        {
            checker.update(remote).await?;
        } else {
            info!(
                "find {}-{} in db, skip",
                remote.client_version, remote.res_version
            );
        }
    }
    loop {
        if query!("SELECT * FROM versions where is_ready = false")
            .fetch_optional(&ctx.conn)
            .await?
            .is_some()
        {
            downloader.sync_all().await?;
        } else {
            break;
        }
    }
    Ok(())
}
