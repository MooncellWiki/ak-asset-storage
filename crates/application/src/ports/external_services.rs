use crate::{AppResult, RemoteVersion};
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
    async fn notify_update(
        &self,
        old_client: &str,
        old_res: &str,
        new_client: &str,
        new_res: &str,
    );

    async fn notify_download_finished(
        &self,
        client_version: &str,
        res_version: &str,
    );
}
