use object_store::aws::AmazonS3;
use sea_orm::DatabaseConnection;
use tracing::info;

use crate::{
    config::Ak,
    error::Result,
    models,
    workers::{
        check::{self, RemoteVersion},
        download,
    },
};
use std::{fs, path::PathBuf, sync::Arc, time::Duration};

pub async fn seed(
    path: PathBuf,
    conn: DatabaseConnection,
    s3: Arc<AmazonS3>,
    ak: Ak,
) -> Result<()> {
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
        if models::versions::Model::find_by_client_res(
            &conn,
            &remote.client_version,
            &remote.res_version,
        )
        .await?
        .is_none()
        {
            checker.update(remote).await?;
        } else {
            info!(
                "find {}-{} in db, skip",
                remote.client_version, remote.res_version
            )
        }
    }
    loop {
        if models::versions::Model::first_unready(&conn)
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
