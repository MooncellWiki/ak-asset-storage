use crate::{db, mailers::Mailer};
use anyhow::bail;
use anyhow::Result;
use itertools::Itertools;
use object_store::aws::AmazonS3;
use object_store::path::Path;
use object_store::{ObjectStore, WriteMultipart};
use serde::Deserialize;
use sha256::digest;
use sqlx::query;
use std::io::{Cursor, Read};
use std::sync::Arc;
use tracing::{debug, error, info};

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
    pub conn: db::Pool,
    pub s3: Arc<AmazonS3>,
    pub mailer: Option<Arc<Mailer>>,
}

impl Download {
    pub async fn perform(&self) {
        if let Err(e) = self.sync_all().await {
            error!("download failed: {e:?}");
        }
    }
    pub async fn sync_all(&self) -> Result<()> {
        let version = query!("SELECT * FROM versions where is_ready = false ORDER BY id ASC")
            .fetch_optional(&self.conn)
            .await?;
        if let Some(version) = version {
            info!("start sync {}-{}", version.res, version.client);
            let info: UpdateList = serde_json::from_str(&version.hot_update_list)?;
            for info in info.ab_infos {
                let local = query!(
                    "SELECT * FROM bundles WHERE version = $1 AND path = $2",
                    version.id,
                    info.name
                )
                .fetch_optional(&self.conn)
                .await?;
                if local.is_some() {
                    info!("{} is already downloaded, skip", info.name);
                    continue;
                }
                let url = format!("{}/{}/{}", self.asset_url, version.res, info.url());
                let file_id = self.sync_file(&url).await?;
                query!(
                    "INSERT INTO bundles (path, version, file) VALUES ($1, $2, $3)",
                    info.name,
                    version.id,
                    file_id
                )
                .execute(&self.conn)
                .await?;
                info!("{} sync finished", info.name);
            }
            query!(
                "UPDATE versions SET is_ready = true WHERE id = $1",
                version.id
            )
            .execute(&self.conn)
            .await?;
            info!("sync version {} finished ", version.res);
            if let Some(mailer) = &self.mailer {
                mailer.notify_download_finished(&version.client, &version.res);
            }
        }
        Ok(())
    }
    async fn sync_file(&self, url: &str) -> Result<i32> {
        let resp = self.client.get(url).send().await?;
        let code = resp.status();
        if !code.is_success() {
            bail!("download failed with code: {}", code);
        }
        let bytes = resp.bytes().await?;
        let mut zip = zip::ZipArchive::new(Cursor::new(&bytes))?;
        let mut buffer = Vec::new();
        let name_list: Vec<String> = zip
            .file_names()
            .sorted()
            .map(std::string::ToString::to_string)
            .collect();
        for name in &name_list {
            let mut file = zip.by_name(name)?;
            file.read_to_end(&mut buffer)?;
        }
        let sha = digest(&buffer);

        let path = Path::from(format!("/{}/{}/{}", &sha[..2], &sha[2..4], &sha[4..]));
        let file = query!("SELECT * FROM files WHERE hash = $1", sha)
            .fetch_optional(&self.conn)
            .await?;
        if let Some(file) = file {
            return Ok(file.id);
        };

        let len = i32::try_from(bytes.len())?;
        // 5MiB
        if len > 5 * 1024 * 1024 {
            let upload = self.s3.put_multipart(&path).await?;
            let mut write = WriteMultipart::new(upload);
            write.write(&bytes);
            write.finish().await?;
        } else {
            self.s3.put(&path, bytes.into()).await?;
        }

        let resp = query!(
            "INSERT INTO files (hash, size) VALUES ($1, $2) RETURNING id",
            sha,
            len
        )
        .fetch_one(&self.conn)
        .await?;
        debug!("sync file {} finished", url);
        Ok(resp.id)
    }
}
