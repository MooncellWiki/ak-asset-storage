use crate::error::{any_anyhow, Error, Result};
use crate::mailers::Mailer;
use crate::models::_entities::{file_metas, files, versions};
use object_store::aws::AmazonS3;
use object_store::path::Path;
use object_store::{ObjectStore, WriteMultipart};
use sea_orm::ActiveValue::NotSet;
use sea_orm::{ActiveModelTrait, DatabaseConnection, IntoActiveModel, Set, TransactionTrait};
use serde::Deserialize;
use sha256::digest;
use std::sync::Arc;
use tracing::{debug, info};

#[derive(Deserialize, Debug)]
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
        let path = self.name.replace('/', "_").replace('#', "__");
        if let Some((left, _)) = path.rsplit_once('.') {
            format!("{left}.dat")
        } else {
            path.clone()
        }
    }
}
#[derive(Deserialize, Debug)]
pub struct UpdateList {
    #[serde(rename = "abInfos")]
    pub ab_infos: Vec<UpdateInfo>,
    #[serde(skip)]
    pub raw: String,
}

#[derive(Debug)]
pub struct Download {
    pub asset_url: String,
    pub client: reqwest::Client,
    pub conn: DatabaseConnection,
    pub s3: Arc<AmazonS3>,
    pub mailer: Option<Arc<Mailer>>,
}

impl Download {
    pub async fn perform(&self) {
        if let Err(e) = self.sync_all().await {
            if let Some(mailer) = &self.mailer {
                mailer.notify_error(&e);
            }
        }
    }
    pub async fn sync_all(&self) -> Result<()> {
        let version = versions::Model::first_unready(&self.conn).await?;
        if let Some(version) = version {
            let info: UpdateList = serde_json::from_str(&version.hot_update_list)?;
            for info in info.ab_infos {
                let local =
                    files::Model::find_by_version_path(&self.conn, version.id, &info.name).await?;
                if local.is_some() {
                    info!("{} is already downloaded, skip", info.name);
                    continue;
                }
                if let Some((_, Some(file))) =
                    file_metas::Model::find_by_md5(&self.conn, &info.md5).await?
                {
                    info!("{} MD5 matches, skipping download", info.name);
                    let file = files::ActiveModel {
                        id: NotSet,
                        path: Set(info.name),
                        hash: Set(file.hash.clone()),
                        version: Set(version.id),
                    }
                    .insert(&self.conn)
                    .await?;
                    file_metas::Model::set_md5(&self.conn, file.id, &info.md5).await?;
                    continue;
                }
                let url = format!("{}/{}/{}", self.asset_url, version.res, info.url());
                let sha = self.sync_file(&url).await?;
                // 跳过时我们只检查files表 不检查file_metas表，所以这里用事务保证两个一起成功
                self.conn
                    .transaction::<_, (), Error>(|txn| {
                        Box::pin(async move {
                            let file = files::ActiveModel {
                                id: NotSet,
                                path: Set(info.name),
                                hash: Set(sha),
                                version: Set(version.id),
                            }
                            .insert(txn)
                            .await?;
                            file_metas::Model::set_md5(txn, file.id, &info.md5).await
                        })
                    })
                    .await
                    .map_err(any_anyhow)?;
            }
            let mut active = version.clone().into_active_model();
            active.is_ready = Set(true);
            active.save(&self.conn).await?;
            info!("sync version {} finished ", version.res);
            if let Some(mailer) = &self.mailer {
                mailer.notify_download_finished(&version.client, &version.res);
            }
        }
        Ok(())
    }
    async fn sync_file(&self, url: &str) -> Result<String> {
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
        debug!("sync file {} finished", url);
        Ok(sha)
    }
}
