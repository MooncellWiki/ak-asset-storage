use crate::{
    AkApiClient, AppResult, DockerService, GithubService, HotUpdateList, NotificationService,
    RemoteVersion, Version, repositories::VersionRepository,
};
use tracing::{error, info, instrument};

pub struct VersionCheckService<V, A, N, D, G>
where
    V: VersionRepository,
    A: AkApiClient,
    N: NotificationService,
    D: DockerService,
    G: GithubService,
{
    version_repo: V,
    ak_client: A,
    notification: N,
    docker_service: Option<D>,
    github_service: Option<G>,
}

impl<V, A, N, D, G> VersionCheckService<V, A, N, D, G>
where
    V: VersionRepository,
    A: AkApiClient,
    N: NotificationService,
    D: DockerService,
    G: GithubService,
{
    pub const fn new(
        version_repo: V,
        ak_client: A,
        notification: N,
        docker_service: Option<D>,
        github_service: Option<G>,
    ) -> Self {
        Self {
            version_repo,
            ak_client,
            notification,
            docker_service,
            github_service,
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

    #[allow(clippy::cognitive_complexity)]
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
        if let Some(ref prev) = prev {
            self.notification
                .notify_update(
                    &prev.client,
                    &prev.res,
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
        } = &remote;
        let version = Version {
            id: None,
            res: res_version.clone(),
            client: client_version.clone(),
            hot_update_list: HotUpdateList::new(&hot_update_list)?,
            is_ready: false,
        };

        self.version_repo.create_version(version).await?;
        info!("new version created and ready for download");

        // 如果启用了GitHub Actions工作流，触发工作流
        if let Some(github_service) = &self.github_service {
            info!("Attempting to dispatch GitHub workflow for new version");
            match github_service.dispatch_workflow().await {
                Ok(..) => {
                    info!("GitHub workflow dispatched successfully");
                }
                Err(e) => {
                    error!("Failed to dispatch GitHub workflow: {e}");
                }
            }
        }

        // 如果启用了Docker容器功能，启动新容器
        if let Some(docker_service) = &self.docker_service
            && let Some(ref prev) = prev
        {
            info!("Attempting to launch Docker container for new version");
            match docker_service
                .launch_container(client_version, res_version, &prev.client, &prev.res)
                .await
            {
                Ok(container_name) => {
                    info!("Docker container launched successfully: {container_name}");
                }
                Err(e) => {
                    error!("Failed to launch Docker container: {e}");
                }
            }
        }

        Ok(true)
    }
}
