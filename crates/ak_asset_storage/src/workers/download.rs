use crate::error::{any_anyhow, Result};
use crate::mailers::Mailer;
use crate::models::{bundles, files, versions};
use itertools::Itertools;
use object_store::aws::AmazonS3;
use object_store::path::Path;
use object_store::{ObjectStore, WriteMultipart};
use sea_orm::ActiveValue::NotSet;
use sea_orm::{ActiveModelTrait, DatabaseConnection, IntoActiveModel, Set, TryIntoModel};
use serde::Deserialize;
use sha256::digest;
use std::io::{Cursor, Read};
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
                    bundles::Model::find_by_version_path(&self.conn, version.id, &info.name)
                        .await?;
                if local.is_some() {
                    info!("{} is already downloaded, skip", info.name);
                    continue;
                }
                let url = format!("{}/{}/{}", self.asset_url, version.res, info.url());
                let file_id = self.sync_file(&url).await?;
                bundles::ActiveModel {
                    id: NotSet,
                    path: Set(info.name),
                    version: Set(version.id),
                    file: Set(file_id),
                }
                .insert(&self.conn)
                .await?;
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
    async fn sync_file(&self, url: &str) -> Result<i32> {
        let bytes = self.client.get(url).send().await?.bytes().await?;
        let mut zip = zip::ZipArchive::new(Cursor::new(&bytes)).map_err(any_anyhow)?;
        let mut buffer = Vec::new();
        let name_list: Vec<String> = zip
            .file_names()
            .sorted()
            .map(std::string::ToString::to_string)
            .collect();
        for name in &name_list {
            let mut file = zip.by_name(name).map_err(any_anyhow)?;
            file.read_to_end(&mut buffer)?;
        }
        let sha = digest(&buffer);

        let path = Path::from(format!("/{}/{}/{}", &sha[..2], &sha[2..4], &sha[4..]));
        if let Ok(Some(file)) = files::Model::find_by_hash(&self.conn, &sha).await {
            return Ok(file.id);
        };
        let len = bytes.len();
        // 5MiB
        if len > 5 * 1024 * 1024 {
            let upload = self.s3.put_multipart(&path).await?;
            let mut write = WriteMultipart::new(upload);
            write.write(&bytes);
            write.finish().await?;
        } else {
            self.s3.put(&path, bytes.into()).await?;
        }
        let resp = files::files::ActiveModel {
            id: NotSet,
            hash: Set(sha),
            size: Set(len.try_into().unwrap()),
        }
        .save(&self.conn)
        .await?;
        let resp = resp.try_into_model()?;
        debug!("sync file {} finished", url);
        Ok(resp.id)
    }
}
