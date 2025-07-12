use crate::{
    repositories::VersionRepository, AkApiClient, AppResult, HotUpdateList, NotificationService,
    RemoteVersion, Version,
};
use tracing::{error, info, instrument};

pub struct VersionCheckService<V, A, N>
where
    V: VersionRepository,
    A: AkApiClient,
    N: NotificationService,
{
    version_repo: V,
    ak_client: A,
    notification: N,
}

impl<V, A, N> VersionCheckService<V, A, N>
where
    V: VersionRepository,
    A: AkApiClient,
    N: NotificationService,
{
    pub const fn new(version_repo: V, ak_client: A, notification: N) -> Self {
        Self {
            version_repo,
            ak_client,
            notification,
        }
    }

    /// 执行单次检查
    #[instrument(name = "services.asset_check", skip_all)]
    pub async fn perform_check(&self) -> AppResult<bool> {
        match self.inner_perform().await {
            Ok(has_update) => Ok(has_update),
            Err(e) => {
                error!("check failed: {e:?}");
                Err(e)
            }
        }
    }

    async fn inner_perform(&self) -> AppResult<bool> {
        let remote = self.ak_client.get_version().await?;
        info!(
            "remote version {} {}",
            &remote.client_version, &remote.res_version
        );
        self.check_and_save(remote).await
    }

    pub async fn check_and_save(&self, remote: RemoteVersion) -> AppResult<bool> {
        let exist = self
            .version_repo
            .is_client_and_res_exist(&remote.client_version, &remote.res_version)
            .await?;

        if exist {
            info!("no change, skip");
            return Ok(false);
        }
        let prev = self.version_repo.get_latest_version().await?;
        if let Some(prev) = prev {
            self.notification
                .notify_update(
                    prev.client.as_str(),
                    prev.res.as_str(),
                    &remote.client_version,
                    &remote.res_version,
                )
                .await;
        } else {
            self.notification
                .notify_update("", "", &remote.client_version, &remote.res_version)
                .await;
        }

        // 获取热更新列表
        let hot_update_list = self
            .ak_client
            .get_hot_update_list(&remote.res_version)
            .await?;

        // 创建新版本记录
        let RemoteVersion {
            res_version,
            client_version,
        } = remote;
        let version = Version {
            id: None,
            res: res_version,
            client: client_version,
            hot_update_list: HotUpdateList::new(&hot_update_list)?,
            is_ready: false,
        };

        self.version_repo.create_version(version).await?;
        info!("new version created and ready for download");

        Ok(true)
    }
}
