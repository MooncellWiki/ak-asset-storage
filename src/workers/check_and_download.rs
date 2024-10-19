use crate::error::{any_anyhow, Result};
use crate::models::_entities::file_metas;
use crate::models::_entities::{files, sea_orm_active_enums::StatusEnum, versions};
use object_store::aws::AmazonS3;
use object_store::path::Path;
use object_store::{ObjectStore, WriteMultipart};
use sea_orm::ActiveValue::NotSet;
use sea_orm::DatabaseConnection;
use sea_orm::{ActiveModelBehavior, ActiveModelTrait, IntoActiveModel, Set};
use serde::{Deserialize, Serialize};
use sha256::digest;
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tracing::info;

use super::WorkerOptions;

#[derive(Deserialize, Debug)]
pub struct RemoteVersion {
    #[serde(rename = "clientVersion")]
    pub client_version: String,
    #[serde(rename = "resVersion")]
    pub res_version: String,
}
impl RemoteVersion {
    pub async fn get(base_url: &str, client: &reqwest::Client) -> Result<Self> {
        let url = format!(
            "{}{}?sign={}",
            base_url,
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

#[derive(Deserialize, Debug, Serialize)]
pub struct UpdateInfo {
    #[serde(rename = "abSize")]
    pub ab_size: u64,
    pub hash: String,
    pub md5: String,
    pub name: String,
    #[serde(rename = "totalSize")]
    pub total_size: u64,
}
impl UpdateInfo {
    #[must_use]
    pub fn url(&self) -> String {
        self.name
            .replace('/', "_")
            .replace('#', "__")
            .replace(".ab", ".dat")
    }
}
#[derive(Deserialize, Debug)]
pub struct UpdateList {
    #[serde(rename = "abInfos")]
    pub ab_infos: Vec<UpdateInfo>,
    #[serde(skip)]
    pub raw: String,
}

impl UpdateList {
    pub async fn get(url: &str, res_ver: &str, client: &reqwest::Client) -> Result<Self> {
        let url = format!("{url}/{res_ver}/hot_update_list.json");
        info!("req hot update list {}", url);
        let resp = client.get(url).send().await?.text().await?;
        info!("got hot update list {}", res_ver);
        let mut result: Self = serde_json::from_str(&resp)?;
        result.raw = resp;
        Ok(result)
    }
}

#[derive(Debug)]
pub struct CheckAndDownload {
    base_url: String,
    client: reqwest::Client,
    conn: DatabaseConnection,
    s3: Arc<AmazonS3>,
}

impl CheckAndDownload {
    pub fn new(opt: WorkerOptions) -> Result<Self> {
        let client = reqwest::ClientBuilder::new()
            .timeout(Duration::from_secs(60 * 5))
            .build()?;
        Ok(Self {
            client,
            base_url: opt.ak.base_url,
            conn: opt.conn,
            s3: opt.s3,
        })
    }
    pub async fn perform(&self) -> Result<()> {
        info!("start check");
        let local_version = versions::Model::latest(&self.conn, false).await?;
        let remote_version = RemoteVersion::get(&self.base_url, &self.client).await?;
        let not_changed = match local_version {
            Some(ref local_version) => {
                local_version.res == remote_version.res_version
                    || local_version.client == remote_version.client_version
            }
            None => false,
        };
        if not_changed {
            info!("no change, skip");
            return Ok(());
        }
        let update =
            UpdateList::get(&self.base_url, &remote_version.res_version, &self.client).await?;
        let mut active = if let Some(local_version) = local_version {
            local_version.into_active_model()
        } else {
            let mut result = versions::ActiveModel::new();
            result.client = Set(remote_version.client_version.clone());
            result.res = Set(remote_version.res_version.clone());
            result.hot_update_list = Set(update.raw.to_string());
            result
        };
        active.status = Set(StatusEnum::Working);
        active = active.clone().save(&self.conn).await?;
        let sea_orm::ActiveValue::Unchanged(version_id) = active.id else {
            unreachable!("id should be set")
        };
        for ab in update.ab_infos {
            self.sync(ab, &remote_version, version_id).await?;
        }
        active.status = Set(StatusEnum::Ready);
        active.save(&self.conn).await?;
        info!("finished");
        Ok(())
    }

    pub async fn sync(
        &self,
        info: UpdateInfo,
        version: &RemoteVersion,
        version_id: i32,
    ) -> Result<()> {
        let local = files::Model::find_by_version_path(&self.conn, 0, &info.name).await?;
        if local.is_some() {
            info!("{} is already downloaded, skip", info.name);
            return Ok(());
        }
        if let Some((_, Some(file))) = file_metas::Model::find_by_md5(&self.conn, &info.md5).await?
        {
            info!("{} MD5 matches, skipping download", info.name);
            let file = files::ActiveModel {
                id: NotSet,
                path: Set(info.name),
                hash: Set(file.hash.clone()),
                version: Set(version_id),
            }
            .insert(&self.conn)
            .await?;
            file_metas::Model::set_md5(&self.conn, file.id, &info.md5).await?;
            return Ok(());
        }
        let url = format!("{}/{}/{}", self.base_url, version.res_version, info.url());
        let sha = self.download(&url).await?;
        let file = files::ActiveModel {
            id: NotSet,
            path: Set(info.name),
            hash: Set(sha),
            version: Set(version_id),
        }
        .insert(&self.conn)
        .await?;
        file_metas::Model::set_md5(&self.conn, file.id, &info.md5).await?;
        Ok(())
    }

    async fn download(&self, url: &str) -> Result<String> {
        let bytes = self.client.get(url).send().await?.bytes().await?;
        let sha = digest(&*bytes);
        let path = Path::from(format!("/{}/{}/{}", &sha[..2], &sha[2..4], &sha[4..]));
        // 5MiB
        if bytes.len() > 5 * 1024 * 1024 {
            let upload = self.s3.put_multipart(&path).await?;
            let mut write = WriteMultipart::new(upload);
            write.write(&bytes);
            write.finish().await?;
        } else {
            self.s3.put(&path, bytes.into()).await?;
        }
        Ok(sha)
    }
}
