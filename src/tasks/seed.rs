use crate::{
    config::Ak,
    db,
    workers::{
        check::{self, RemoteVersion},
        download,
    },
};
use anyhow::Result;
use object_store::aws::AmazonS3;
use sqlx::query;
use std::{fs, path::PathBuf, sync::Arc, time::Duration};
use tracing::info;

pub async fn seed(path: PathBuf, conn: db::Pool, s3: Arc<AmazonS3>, ak: Ak) -> Result<()> {
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
    let client = reqwest::ClientBuilder::new()
        .timeout(Duration::from_secs(60 * 5))
        .build()?;
    let checker = check::Check {
        client: client.clone(),
        conn: conn.clone(),
        mailer: None,
        conf_url: ak.conf_url.clone(),
        asset_url: ak.asset_url.clone(),
    };
    let downloader = download::Download {
        client: client.clone(),
        conn: conn.clone(),
        s3: s3.clone(),
        mailer: None,
        asset_url: ak.asset_url.clone(),
    };
    for remote in versions {
        if query!(
            "SELECT * FROM versions where client = $1 and res = $2",
            remote.client_version,
            remote.res_version
        )
        .fetch_optional(&conn)
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
            .fetch_optional(&conn)
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
