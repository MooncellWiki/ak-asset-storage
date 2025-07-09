use crate::dto::RemoteVersion;
use crate::error::AppResult;
use async_trait::async_trait;

#[async_trait]
pub trait AkApiClient: Send + Sync {
    async fn get_version(&self) -> AppResult<RemoteVersion>;
    async fn get_hot_update_list(&self, res_version: &str) -> AppResult<String>;
    async fn download_file(&self, res_version: &str, path: &str) -> AppResult<Vec<u8>>;
}

#[async_trait]
pub trait StorageService: Send + Sync {
    async fn upload(&self, path: &str, data: &[u8]) -> AppResult<()>;
    async fn download(&self, path: &str) -> AppResult<Vec<u8>>;
    async fn delete(&self, path: &str) -> AppResult<()>;
    async fn exists(&self, path: &str) -> AppResult<bool>;
}

#[async_trait]
pub trait NotificationService: Send + Sync {
    async fn notify_update(
        &self,
        old_client: &str,
        old_res: &str,
        new_client: &str,
        new_res: &str,
    ) -> AppResult<()>;

    async fn notify_download_finished(
        &self,
        client_version: &str,
        res_version: &str,
    ) -> AppResult<()>;
}
