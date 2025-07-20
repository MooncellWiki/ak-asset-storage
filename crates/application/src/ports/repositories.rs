use crate::{
    AppResult, Bundle, BundleDetailsDto, BundleFilterDto, File, Version, VersionDetailDto,
    VersionDto,
};
use async_trait::async_trait;

#[async_trait]
pub trait Repository: Send + Sync + Clone + 'static {
    async fn health_check(&self) -> bool;
    async fn migrate(&self) -> AppResult<()>;
}

#[async_trait]
pub trait VersionRepository: Send + Sync + Clone + 'static {
    async fn create_version(&self, version: Version) -> AppResult<i32>;
    async fn get_version_by_id(&self, id: i32) -> AppResult<Option<Version>>;
    async fn get_latest_version(&self) -> AppResult<Option<Version>>;
    async fn is_client_and_res_exist(&self, client: &str, res: &str) -> AppResult<bool>;
    async fn get_oldest_unready_version(&self) -> AppResult<Option<Version>>;
    async fn mark_version_ready(&self, id: i32) -> AppResult<()>;

    async fn query_versions(&self) -> AppResult<Vec<VersionDto>>;
    async fn query_version_detail_by_id(&self, id: i32) -> AppResult<Option<VersionDetailDto>>;
}

#[async_trait]
pub trait FileRepository: Send + Sync + Clone + 'static {
    async fn create_file(&self, file: File) -> AppResult<i32>;
    async fn get_file_by_hash(&self, hash: &str) -> AppResult<Option<File>>;
}

#[async_trait]
pub trait BundleRepository: Send + Sync + Clone + 'static {
    async fn create_bundle(&self, bundle: Bundle) -> AppResult<i32>;
    async fn get_bundle_by_version_and_path(
        &self,
        version_id: i32,
        path: &str,
    ) -> AppResult<Option<Bundle>>;

    // queries
    async fn query_bundle_by_id_with_details(&self, id: i32)
        -> AppResult<Option<BundleDetailsDto>>;
    async fn query_bundles_with_details(
        &self,
        query: BundleFilterDto,
    ) -> AppResult<Vec<BundleDetailsDto>>;
    async fn query_bundles_by_version_id(
        &self,
        version_id: i32,
    ) -> AppResult<Vec<BundleDetailsDto>>;
}

#[async_trait]
pub trait ItemDemandRepository: Send + Sync + Clone + 'static {
    async fn query_usage_by_item_name(&self, item_name: &str) -> AppResult<Option<String>>;
    async fn replace_all_demands(&self, demands: Vec<(String, String)>) -> AppResult<()>;
}
