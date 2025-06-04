use super::WorkerContext;
use anyhow::{bail, Result};
use futures::{stream, StreamExt, TryStreamExt};
use itertools::Itertools;
use object_store::{path::Path, ObjectStore, WriteMultipart};
use serde::Deserialize;
use sha256::digest;
use sqlx::query;
use std::io::{Cursor, Read};
use tracing::{debug, error, info, instrument};

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
    pub ctx: WorkerContext,
}

impl Download {
    #[instrument(name = "worker.download", skip_all)]
    pub async fn perform(&self) {
        if let Err(e) = self.sync_all().await {
            error!("download failed: {e:?}");
        }
    }
    pub async fn sync_all(&self) -> Result<()> {
        let version = query!("SELECT * FROM versions where is_ready = false ORDER BY id ASC")
            .fetch_optional(&self.ctx.conn)
            .await?;
        if let Some(version) = version {
            info!("start sync {}-{}", version.res, version.client);
            let info: UpdateList = serde_json::from_str(&version.hot_update_list)?;

            let version_id = version.id;

            stream::iter(info.ab_infos)
                .map(Ok)
                .try_for_each_concurrent(5, |info| {
                    self.skip_or_download(info, version_id, &version.res)
                })
                .await?;

            query!(
                "UPDATE versions SET is_ready = true WHERE id = $1",
                version.id
            )
            .execute(&self.ctx.conn)
            .await?;
            info!("sync version {} finished ", version.res);
            if let Some(mailer) = &self.ctx.mailer {
                mailer.notify_download_finished(&version.client, &version.res);
            }
        }
        Ok(())
    }

    async fn skip_or_download(&self, info: UpdateInfo, version_id: i32, res: &str) -> Result<()> {
        let local = query!(
            "SELECT * FROM bundles WHERE version = $1 AND path = $2",
            version_id,
            info.name
        )
        .fetch_optional(&self.ctx.conn)
        .await?;

        if local.is_some() {
            info!("{} is already downloaded, skip", info.name);
            return Ok(());
        }

        let url = format!("{}/{}/{}", self.ctx.ak.asset_url, res, info.url());
        let file_id = self.sync_file(&url).await?;

        query!(
            "INSERT INTO bundles (path, version, file) VALUES ($1, $2, $3)",
            info.name,
            version_id,
            file_id
        )
        .execute(&self.ctx.conn)
        .await?;

        info!("{} sync finished", info.name);
        Ok(())
    }

    async fn sync_file(&self, url: &str) -> Result<i32> {
        let resp = self.ctx.client.get(url).send().await?;
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
            .fetch_optional(&self.ctx.conn)
            .await?;
        if let Some(file) = file {
            return Ok(file.id);
        }

        let len = i32::try_from(bytes.len())?;
        // 5MiB
        if len > 5 * 1024 * 1024 {
            let upload = self.ctx.s3.put_multipart(&path).await?;
            let mut write = WriteMultipart::new(upload);
            write.write(&bytes);
            write.finish().await?;
        } else {
            self.ctx.s3.put(&path, bytes.into()).await?;
        }

        let resp = query!(
            "INSERT INTO files (hash, size) VALUES ($1, $2) RETURNING id",
            sha,
            len
        )
        .fetch_one(&self.ctx.conn)
        .await?;
        debug!("sync file {} finished", url);
        Ok(resp.id)
    }
}
