use crate::{db, mailers::Mailer};
use anyhow::Result;
use serde::Deserialize;
use sqlx::query;
use std::{
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
};
use tracing::{error, info};

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct RemoteVersion {
    pub client_version: String,
    pub res_version: String,
}
impl RemoteVersion {
    pub async fn get(conf_url: &str, client: &reqwest::Client) -> Result<Self> {
        let url = format!(
            "{}/{}?sign={}",
            conf_url,
            "version",
            SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs()
        );
        info!("req version {}", url);

        let ver: Self = client.get(url).send().await?.json().await?;
        info!(
            "remote version {} {}",
            &ver.client_version, &ver.res_version
        );
        Ok(ver)
    }
}

pub async fn get_hot_update_list(
    url: &str,
    res_ver: &str,
    client: &reqwest::Client,
) -> Result<String> {
    let url = format!("{url}/{res_ver}/hot_update_list.json");
    info!("req hot update list {}", url);
    let resp = client.get(url).send().await?.text().await?;
    Ok(resp)
}

#[derive(Debug)]
pub struct Check {
    pub conf_url: String,
    pub asset_url: String,
    pub client: reqwest::Client,
    pub conn: db::Pool,
    pub mailer: Option<Arc<Mailer>>,
}
impl Check {
    pub async fn perform(&self) {
        if let Err(e) = self.inner_perform().await {
            error!("check failed: {e:?}");
        }
    }
    pub async fn inner_perform(&self) -> Result<()> {
        let remote = RemoteVersion::get(&self.conf_url, &self.client).await?;
        self.update(remote).await?;
        Ok(())
    }
    pub async fn update(&self, remote: RemoteVersion) -> Result<()> {
        let local_version = query!(
            r#"
        SELECT res, client FROM versions ORDER BY ID DESC"#,
        )
        .fetch_optional(&self.conn)
        .await?;
        match local_version {
            Some(local) => {
                if local.client == remote.client_version && local.res == remote.res_version {
                    info!("no change, skip");
                    return Ok(());
                }
                if let Some(mailer) = &self.mailer {
                    mailer.notify_update(
                        &local.client,
                        &local.res,
                        &remote.client_version,
                        &remote.res_version,
                    );
                }
            }
            None => {
                if let Some(mailer) = &self.mailer {
                    mailer.notify_update("", "", &remote.client_version, &remote.res_version);
                }
            }
        }
        let update =
            get_hot_update_list(&self.asset_url, &remote.res_version, &self.client).await?;
        query!(
            "INSERT INTO versions (client, res, is_ready, hot_update_list) VALUES ($1, $2, $3, $4)",
            remote.client_version,
            remote.res_version,
            false,
            update
        )
        .execute(&self.conn)
        .await?;

        Ok(())
    }
}
