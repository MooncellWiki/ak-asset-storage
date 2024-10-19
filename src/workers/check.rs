use crate::{
    error::{any_anyhow, Result},
    mailers::Mailer,
    models::_entities::versions,
};
use sea_orm::{ActiveModelTrait, ActiveValue::NotSet, DatabaseConnection, Set};
use serde::Deserialize;
use std::{
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
};
use tracing::info;

#[derive(Deserialize, Debug)]
pub struct RemoteVersion {
    #[serde(rename = "clientVersion")]
    pub client_version: String,
    #[serde(rename = "resVersion")]
    pub res_version: String,
}
impl RemoteVersion {
    pub async fn get(conf_url: &str, client: &reqwest::Client) -> Result<Self> {
        let url = format!(
            "{}/{}?sign={}",
            conf_url,
            "version",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map_err(any_anyhow)?
                .as_secs()
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
    pub conn: DatabaseConnection,
    pub mailer: Option<Arc<Mailer>>,
}
impl Check {
    pub async fn perform(&self) {
        if let Err(e) = self.inner_perform().await {
            if let Some(mailer) = &self.mailer {
                mailer.notify_error(&e);
            }
        }
    }
    pub async fn inner_perform(&self) -> Result<()> {
        let remote = RemoteVersion::get(&self.conf_url, &self.client).await?;
        self.update(remote).await?;
        Ok(())
    }
    pub async fn update(&self, remote: RemoteVersion) -> Result<()> {
        let local_version = versions::Model::latest(&self.conn, false).await?;
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
        versions::ActiveModel {
            client: Set(remote.client_version),
            res: Set(remote.res_version),
            is_ready: Set(false),
            hot_update_list: Set(update),
            id: NotSet,
        }
        .insert(&self.conn)
        .await?;
        Ok(())
    }
}
