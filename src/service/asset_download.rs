use crate::{
    AppResult,
    database::{
        Database,
        row::{BundleRow, FileRow, VersionRow},
    },
    external::{ak_api::AkApi, notification::NotificationClient, s3::S3Storage},
    service::types::{ABInfo, HotUpdateList},
};
use anyhow::Context;
use futures::{StreamExt, TryStreamExt, stream};
use itertools::Itertools;
use sha256::digest;
use std::io::{Cursor, Read};
use tracing::{debug, error, info, instrument};
use zip::ZipArchive;

#[derive(Clone)]
pub struct AssetDownloadService {
    pub database: Database,
    pub ak_api: AkApi,
    pub notification: NotificationClient,
    pub storage: S3Storage,
    pub concurrent: usize,
}

impl AssetDownloadService {
    #[instrument(name = "service.asset_download", skip(self))]
    pub async fn perform_download(&self) -> AppResult<bool> {
        match self.sync_oldest_version().await {
            Ok(has_more) => Ok(has_more),
            Err(err) => {
                error!("download failed: {err:?}");
                Err(err)
            }
        }
    }

    pub async fn manual_download(&self, version_id: Option<i32>) -> AppResult<()> {
        if let Some(id) = version_id {
            self.sync_specific_version(id).await
        } else {
            self.sync_oldest_version().await?;
            Ok(())
        }
    }

    async fn sync_oldest_version(&self) -> AppResult<bool> {
        let version = self.database.get_oldest_unready_version().await?;
        if let Some(version) = version {
            info!("start sync {}-{}", version.res, version.client);
            self.sync_version(&version).await?;
            return Ok(true);
        }
        info!("no pending version to sync");
        Ok(false)
    }

    async fn sync_specific_version(&self, version_id: i32) -> AppResult<()> {
        let version = self
            .database
            .get_version_by_id(version_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Version not found: {version_id}"))?;

        info!(
            "start sync specific version {}-{}",
            version.res, version.client
        );
        self.sync_version(&version).await
    }

    async fn sync_version(&self, version: &VersionRow) -> AppResult<()> {
        let version_id = version
            .id
            .ok_or_else(|| anyhow::anyhow!("Version ID is missing"))?;
        let hot_update_list = HotUpdateList::new(&version.hot_update_list)?;

        stream::iter(hot_update_list.ab_infos())
            .map(Ok)
            .try_for_each_concurrent(self.concurrent, |info| {
                self.skip_or_download(info.clone(), version_id, version.res.as_str())
            })
            .await?;

        self.database.mark_version_ready(version_id).await?;
        info!("sync version {} finished", version.res);

        self.notification
            .notify_download_finished(version.client.as_str(), version.res.as_str())
            .await;
        Ok(())
    }

    async fn skip_or_download(
        &self,
        info: ABInfo,
        version_id: i32,
        res_version: &str,
    ) -> AppResult<()> {
        if self
            .database
            .get_bundle_by_version_and_path(version_id, &info.name)
            .await?
            .is_some()
        {
            info!("{} is already downloaded, skip", info.name);
            return Ok(());
        }

        let file_id = self.sync_file(res_version, &info.url()).await?;
        let bundle_path = info.name.clone();
        let bundle = BundleRow {
            id: None,
            path: bundle_path.clone(),
            version_id,
            file_id,
        };

        self.database.create_bundle(bundle).await?;
        info!("{} sync finished", bundle_path);
        Ok(())
    }

    fn calc_sha256(bytes: &[u8]) -> AppResult<String> {
        let mut zip =
            ZipArchive::new(Cursor::new(bytes)).context("Failed to create zip archive")?;
        let mut buffer = Vec::new();
        let name_list = zip
            .file_names()
            .sorted()
            .map(std::string::ToString::to_string)
            .collect_vec();

        for name in name_list {
            let mut file = zip.by_name(&name).context("Failed to read zip file")?;
            file.read_to_end(&mut buffer)
                .context("Failed to push zip file content to buffer")?;
        }

        Ok(digest(&buffer))
    }

    async fn sync_file(&self, res_version: &str, path: &str) -> AppResult<i32> {
        let bytes = self.ak_api.download_file(res_version, path).await?;
        let sha = Self::calc_sha256(&bytes)?;

        if let Some(file) = self.database.get_file_by_hash(&sha).await? {
            debug!("file {} already exists, skip", path);
            return file
                .id
                .ok_or_else(|| anyhow::anyhow!("File ID is missing"))
                .map_err(Into::into);
        }

        let storage_path = format!("/{}/{}/{}", &sha[..2], &sha[2..4], &sha[4..]);
        self.storage.upload(&storage_path, &bytes).await?;

        let file = FileRow {
            id: None,
            hash: sha,
            size: i32::try_from(bytes.len()).context("Failed to convert file size to i32")?,
        };

        let file_id = self.database.create_file(file).await?;
        debug!("sync file {} finished", path);
        Ok(file_id)
    }
}
