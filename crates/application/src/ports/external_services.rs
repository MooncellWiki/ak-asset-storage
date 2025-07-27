use crate::{AppResult, AssetDirInfo, AssetEntry, RemoteVersion};
use async_trait::async_trait;

#[async_trait]
pub trait AkApiClient: Send + Sync + Clone + 'static {
    async fn get_version(&self) -> AppResult<RemoteVersion>;
    async fn get_hot_update_list(&self, res_version: &str) -> AppResult<String>;
    async fn download_file(&self, res_version: &str, path: &str) -> AppResult<Vec<u8>>;
}

#[async_trait]
pub trait StorageService: Send + Sync + Clone + 'static {
    async fn upload(&self, path: &str, data: &[u8]) -> AppResult<()>;
}

#[async_trait]
pub trait NotificationService: Send + Sync + Clone + 'static {
    async fn notify_update(&self, old_client: &str, old_res: &str, new_client: &str, new_res: &str);

    async fn notify_download_finished(&self, client_version: &str, res_version: &str);
}

#[async_trait]
pub trait TorappuAssetService: Send + Sync + Clone + 'static {
    async fn list_asset(&self, path: &str) -> AppResult<AssetDirInfo>;
    async fn search_assets_by_path(&self, path: &str) -> AppResult<Vec<AssetEntry>>;
}

#[async_trait]
pub trait DockerService: Send + Sync + Clone + 'static {
    async fn launch_container(
        &self,
        client_version: &str,
        res_version: &str,
        prev_client_version: &str,
        prev_res_version: &str,
    ) -> AppResult<String>;
}

#[async_trait]
pub trait GithubService: Send + Sync + Clone + 'static {
    async fn dispatch_workflow(&self) -> AppResult<()>;
}
