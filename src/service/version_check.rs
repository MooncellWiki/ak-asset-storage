use crate::{
    AppResult,
    database::{
        Database,
        row::{AssetMappingStatus, VersionRow},
    },
    external::{ak_api::AkApi, kubernetes::KubernetesClient, notification::NotificationClient},
    service::types::{HotUpdateList, RemoteVersion},
};
use tracing::{error, info, instrument};

#[derive(Clone)]
pub struct VersionCheckService {
    pub database: Database,
    pub ak_api: AkApi,
    pub notification: NotificationClient,
    pub kubernetes: Option<KubernetesClient>,
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
        let existing = self
            .database
            .get_version_import_state(&remote.client_version, &remote.res_version)
            .await?;

        if existing.is_some() {
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

        self.save_version(&remote).await?;
        let RemoteVersion {
            res_version,
            client_version,
        } = &remote;

        if let Some(kubernetes) = &self.kubernetes
            && let Some(prev) = &prev
        {
            info!("Attempting to launch Kubernetes Job for new version");
            match kubernetes
                .launch_container(
                    client_version,
                    res_version,
                    &prev.client,
                    &prev.res,
                    None,
                    None,
                )
                .await
            {
                Ok(job_name) => {
                    info!("Kubernetes Job launched successfully: {job_name}");
                }
                Err(err) => error!("Failed to launch Kubernetes Job: {err}"),
            }
        }

        Ok(true)
    }

    /// Return the id to download, or skip a version whose bundles are already ready.
    pub(crate) async fn prepare_backfill(&self, remote: &RemoteVersion) -> AppResult<Option<i32>> {
        match self
            .database
            .get_version_import_state(&remote.client_version, &remote.res_version)
            .await?
        {
            Some((_, true)) => Ok(None),
            Some((id, false)) => Ok(Some(id)),
            None => self.save_version(remote).await.map(Some),
        }
    }

    async fn save_version(&self, remote: &RemoteVersion) -> AppResult<i32> {
        let hot_update_list = self.ak_api.get_hot_update_list(&remote.res_version).await?;
        HotUpdateList::new(&hot_update_list)?;
        let id = self
            .database
            .create_version(VersionRow {
                id: None,
                res: remote.res_version.clone(),
                client: remote.client_version.clone(),
                hot_update_list,
                is_ready: false,
                asset_mapping_status: AssetMappingStatus::Pending,
            })
            .await?;
        info!(id, "new version created and ready for download");
        Ok(id)
    }
}
