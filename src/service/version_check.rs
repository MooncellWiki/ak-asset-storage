use crate::{
    AppResult,
    database::{
        Database,
        row::{AssetMappingStatus, VersionRow},
    },
    external::{
        ak_api::AkApi, docker::DockerClient, github::GithubClient, notification::NotificationClient,
    },
    service::types::{HotUpdateList, RemoteVersion},
};
use tracing::{error, info, instrument};

#[derive(Clone)]
pub struct VersionCheckService {
    pub database: Database,
    pub ak_api: AkApi,
    pub notification: NotificationClient,
    pub docker: Option<DockerClient>,
    pub github: Option<GithubClient>,
}

impl VersionCheckService {
    #[instrument(name = "services.asset_check", skip_all)]
    pub async fn perform_check(&self) -> AppResult<bool> {
        match self.inner_perform().await {
            Ok(has_update) => Ok(has_update),
            Err(err) => {
                error!("check failed: {err:?}");
                Err(err)
            }
        }
    }

    async fn inner_perform(&self) -> AppResult<bool> {
        let remote = self.ak_api.get_version().await?;
        info!(
            "remote version {} {}",
            &remote.client_version, &remote.res_version
        );
        self.check_and_save(remote).await
    }

    #[allow(clippy::cognitive_complexity)]
    pub async fn check_and_save(&self, remote: RemoteVersion) -> AppResult<bool> {
        let exists = self
            .database
            .is_client_and_res_exist(&remote.client_version, &remote.res_version)
            .await?;

        if exists {
            info!("no change, skip");
            return Ok(false);
        }

        let prev = self.database.get_latest_version().await?;
        if let Some(prev) = &prev {
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

        let hot_update_list = self.ak_api.get_hot_update_list(&remote.res_version).await?;
        HotUpdateList::new(&hot_update_list)?;
        let RemoteVersion {
            res_version,
            client_version,
        } = &remote;
        let version = VersionRow {
            id: None,
            res: res_version.clone(),
            client: client_version.clone(),
            hot_update_list,
            is_ready: false,
            asset_mapping_status: AssetMappingStatus::Pending,
        };

        self.database.create_version(version).await?;
        info!("new version created and ready for download");

        if let Some(github) = &self.github {
            info!("Attempting to dispatch GitHub workflow for new version");
            match github.dispatch_workflow().await {
                Ok(()) => info!("GitHub workflow dispatched successfully"),
                Err(err) => error!("Failed to dispatch GitHub workflow: {err}"),
            }
        }

        if let Some(docker) = &self.docker
            && let Some(prev) = &prev
        {
            info!("Attempting to launch Docker container for new version");
            match docker
                .launch_container(client_version, res_version, &prev.client, &prev.res)
                .await
            {
                Ok(container_name) => {
                    info!("Docker container launched successfully: {container_name}");
                }
                Err(err) => error!("Failed to launch Docker container: {err}"),
            }
        }

        Ok(true)
    }
}
