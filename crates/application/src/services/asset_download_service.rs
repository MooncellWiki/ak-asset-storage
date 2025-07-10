use crate::{
    ABInfo, AkApiClient, AppResult, Bundle, BundleRepository, File, FileRepository,
    NotificationService, StorageService, Version, VersionRepository,
};
use futures::{stream, StreamExt, TryStreamExt};
use sha256::digest;
use tracing::{debug, error, info, instrument};

pub struct AssetDownloadService<R, A, N, S>
where
    R: VersionRepository + FileRepository + BundleRepository,
    A: AkApiClient,
    N: NotificationService,
    S: StorageService,
{
    repo: R,
    ak_client: A,
    notification: N,
    storage: S,
}

impl<R, A, N, S> AssetDownloadService<R, A, N, S>
where
    R: VersionRepository + FileRepository + BundleRepository,
    A: AkApiClient,
    N: NotificationService,
    S: StorageService,
{
    pub const fn new(repo: R, ak_client: A, notification: N, storage: S) -> Self {
        Self {
            repo,
            ak_client,
            notification,
            storage,
        }
    }

    /// 执行下载任务（事件驱动，由Check UseCase触发）
    /// 返回true 如果没有没完成的版本
    #[instrument(name = "usecase.asset_download", skip(self))]
    pub async fn perform_download(&self) -> AppResult<bool> {
        match self.sync_oldest_version().await {
            Ok(has_more) => Ok(has_more),
            Err(e) => {
                error!("download failed: {e:?}");
                Err(e)
            }
        }
    }

    /// 手动触发下载（用于Web API）
    pub async fn manual_download(&self, version_id: Option<i32>) -> AppResult<()> {
        if let Some(id) = version_id {
            self.sync_specific_version(id).await
        } else {
            self.sync_oldest_version().await?;
            Ok(())
        }
    }

    /// 同步所有最老的版本
    /// 返回true 如果没有没完成的版本
    async fn sync_oldest_version(&self) -> AppResult<bool> {
        // 获取未完成的版本
        let version = self.repo.get_oldest_unready_version().await?;

        if let Some(version) = version {
            info!(
                "start sync {}-{}",
                version.res.as_str(),
                version.client.as_str()
            );
            self.sync_version(&version).await?;
            return Ok(false);
        }
        info!("no pending version to sync");
        Ok(true)
    }

    async fn sync_specific_version(&self, version_id: i32) -> AppResult<()> {
        let version = self
            .repo
            .get_version_by_id(version_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Version not found: {}", version_id))?;

        info!(
            "start sync specific version {}-{}",
            version.res.as_str(),
            version.client.as_str()
        );
        self.sync_version(&version).await
    }

    async fn sync_version(&self, version: &Version) -> AppResult<()> {
        // 并发下载文件，限制并发数为5
        let version_id = version
            .id
            .ok_or_else(|| anyhow::anyhow!("Version ID is missing"))?;
        stream::iter(version.hot_update_list.ab_infos())
            .map(Ok)
            .try_for_each_concurrent(5, |info| {
                self.skip_or_download(info.clone(), version_id, version.res.as_str())
            })
            .await?;

        // 标记版本为已完成
        self.repo.mark_version_ready(version_id).await?;

        info!("sync version {} finished", version.res.as_str());

        // 发送下载完成通知
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
        // 检查bundle是否已存在
        let existing_bundle = self
            .repo
            .get_bundle_by_version_and_path(version_id, &info.name)
            .await?;

        if existing_bundle.is_some() {
            info!("{} is already downloaded, skip", info.name);
            return Ok(());
        }

        // 下载文件
        let file_id = self.sync_file(res_version, &info.url()).await?;

        // 创建bundle记录
        let bundle_path = info.name.clone();
        let bundle = Bundle {
            id: None,
            path: bundle_path.clone(),
            version_id,
            file_id,
        };

        self.repo.create_bundle(bundle).await?;
        info!("{} sync finished", bundle_path);

        Ok(())
    }

    async fn sync_file(&self, res_version: &str, path: &str) -> AppResult<i32> {
        // 下载文件
        let bytes = self.ak_client.download_file(res_version, path).await?;

        let sha = digest(&bytes);

        // 检查文件是否已存在
        if let Some(file) = self.repo.get_file_by_hash(&sha).await? {
            return Ok(file
                .id
                .ok_or_else(|| anyhow::anyhow!("File ID is missing"))?);
        }

        // 上传到存储服务
        let storage_path = format!("/{}/{}/{}", &sha[..2], &sha[2..4], &sha[4..]);
        self.storage.upload(&storage_path, &bytes).await?;

        // 创建文件记录
        let file = File {
            id: None,
            hash: sha,
            size: bytes.len() as i32,
        };

        let file_id = self.repo.create_file(file).await?;
        debug!("sync file {} finished", path);

        Ok(file_id)
    }
}
