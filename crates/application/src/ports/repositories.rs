use crate::{error::AppResult, BundleDetailsDto, BundleFilterDto, VersionDetailDto, VersionDto};
use async_trait::async_trait;
use domain::entities::{Bundle, File, Version};

#[async_trait]
pub trait VersionRepository: Send + Sync + Clone + 'static {
    async fn create(&self, version: Version) -> AppResult<i32>;
    async fn get_by_id(&self, id: i32) -> AppResult<Option<Version>>;
    async fn get_latest(&self) -> AppResult<Option<Version>>;
    async fn is_client_and_res_exist(&self, client: &str, res: &str) -> AppResult<bool>;
    async fn get_oldest_unready_version(&self) -> AppResult<Option<Version>>;
    async fn mark_ready(&self, id: i32) -> AppResult<()>;

    async fn query(&self) -> AppResult<Vec<VersionDto>>;
    async fn query_detail_by_id(&self, id: i32) -> AppResult<Option<VersionDetailDto>>;
}

#[async_trait]
pub trait FileRepository: Send + Sync + Clone + 'static {
    async fn create(&self, file: File) -> AppResult<i32>;
    async fn get_by_id(&self, id: i32) -> AppResult<Option<File>>;
    async fn get_by_hash(&self, hash: &str) -> AppResult<Option<File>>;
    async fn get_all(&self) -> AppResult<Vec<File>>;
    async fn get_orphaned_files(&self) -> AppResult<Vec<File>>;
    async fn delete(&self, id: i32) -> AppResult<()>;
}

#[async_trait]
pub trait BundleRepository: Send + Sync + Clone + 'static {
    async fn create(&self, bundle: Bundle) -> AppResult<i32>;
    async fn get_by_version_and_path(
        &self,
        version_id: i32,
        path: &str,
    ) -> AppResult<Option<Bundle>>;

    // queries
    async fn query_by_id_with_details(&self, id: i32) -> AppResult<Option<BundleDetailsDto>>;
    async fn query_with_details(&self, query: BundleFilterDto) -> AppResult<Vec<BundleDetailsDto>>;
    async fn query_by_version_id(&self, version_id: i32) -> AppResult<Vec<BundleDetailsDto>>;
}
