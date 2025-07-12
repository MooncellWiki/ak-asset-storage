// Repository Mock implementations using Vec for storage
use ak_asset_storage_application::{
    AppResult, Bundle, BundleDetailsDto, BundleFilterDto, File, Version, VersionDetailDto,
    VersionDto,
};
use ak_asset_storage_application::{BundleRepository, FileRepository, VersionRepository};
use async_trait::async_trait;
use std::sync::{Arc, Mutex};

// Mock Version Repository
#[derive(Clone, Debug)]
pub struct MockVersionRepository {
    pub versions: Arc<Mutex<Vec<Version>>>,
    pub next_id: Arc<Mutex<i32>>,
}

impl MockVersionRepository {
    pub fn new() -> Self {
        Self {
            versions: Arc::new(Mutex::new(Vec::new())),
            next_id: Arc::new(Mutex::new(1)),
        }
    }
}

#[async_trait]
impl VersionRepository for MockVersionRepository {
    async fn create_version(&self, mut version: Version) -> AppResult<i32> {
        let mut versions = self.versions.lock().unwrap();
        let mut next_id = self.next_id.lock().unwrap();

        let id = *next_id;
        *next_id += 1;
        drop(next_id);
        version.id = Some(id);
        versions.push(version);
        drop(versions);
        Ok(id)
    }

    async fn get_version_by_id(&self, id: i32) -> AppResult<Option<Version>> {
        let versions = self.versions.lock().unwrap();
        Ok(versions.iter().find(|v| v.id == Some(id)).cloned())
    }

    async fn get_latest_version(&self) -> AppResult<Option<Version>> {
        let versions = self.versions.lock().unwrap();
        Ok(versions.iter().max_by_key(|v| v.id).cloned())
    }

    async fn is_client_and_res_exist(&self, client: &str, res: &str) -> AppResult<bool> {
        let versions = self.versions.lock().unwrap();
        Ok(versions.iter().any(|v| v.client == client && v.res == res))
    }

    async fn get_oldest_unready_version(&self) -> AppResult<Option<Version>> {
        let versions = self.versions.lock().unwrap();
        Ok(versions
            .iter()
            .filter(|v| !v.is_ready)
            .min_by_key(|v| v.id)
            .cloned())
    }

    async fn mark_version_ready(&self, id: i32) -> AppResult<()> {
        let mut versions = self.versions.lock().unwrap();
        if let Some(version) = versions.iter_mut().find(|v| v.id == Some(id)) {
            version.is_ready = true;
        }
        drop(versions);
        Ok(())
    }

    async fn query_versions(&self) -> AppResult<Vec<VersionDto>> {
        unimplemented!("Not used in service tests")
    }

    async fn query_version_detail_by_id(&self, _id: i32) -> AppResult<Option<VersionDetailDto>> {
        unimplemented!("Not used in service tests")
    }
}

// Mock File Repository
#[derive(Clone, Debug)]
pub struct MockFileRepository {
    pub files: Arc<Mutex<Vec<File>>>,
    pub next_id: Arc<Mutex<i32>>,
}

impl MockFileRepository {
    pub fn new() -> Self {
        Self {
            files: Arc::new(Mutex::new(Vec::new())),
            next_id: Arc::new(Mutex::new(1)),
        }
    }
}

#[async_trait]
impl FileRepository for MockFileRepository {
    async fn create_file(&self, mut file: File) -> AppResult<i32> {
        let mut files = self.files.lock().unwrap();
        let mut next_id = self.next_id.lock().unwrap();

        let id = *next_id;
        *next_id += 1;
        drop(next_id);
        file.id = Some(id);
        files.push(file);
        drop(files);
        Ok(id)
    }

    async fn get_file_by_hash(&self, hash: &str) -> AppResult<Option<File>> {
        let files = self.files.lock().unwrap();
        Ok(files.iter().find(|f| f.hash == hash).cloned())
    }
}

// Mock Bundle Repository
#[derive(Clone, Debug)]
pub struct MockBundleRepository {
    pub bundles: Arc<Mutex<Vec<Bundle>>>,
    pub next_id: Arc<Mutex<i32>>,
}

impl MockBundleRepository {
    pub fn new() -> Self {
        Self {
            bundles: Arc::new(Mutex::new(Vec::new())),
            next_id: Arc::new(Mutex::new(1)),
        }
    }
}

#[async_trait]
impl BundleRepository for MockBundleRepository {
    async fn create_bundle(&self, mut bundle: Bundle) -> AppResult<i32> {
        let mut bundles = self.bundles.lock().unwrap();
        let mut next_id = self.next_id.lock().unwrap();

        let id = *next_id;
        *next_id += 1;
        drop(next_id);
        bundle.id = Some(id);
        bundles.push(bundle);
        drop(bundles);
        Ok(id)
    }

    async fn get_bundle_by_version_and_path(
        &self,
        version_id: i32,
        path: &str,
    ) -> AppResult<Option<Bundle>> {
        let bundles = self.bundles.lock().unwrap();
        Ok(bundles
            .iter()
            .find(|b| b.version_id == version_id && b.path == path)
            .cloned())
    }

    async fn query_bundle_by_id_with_details(
        &self,
        _id: i32,
    ) -> AppResult<Option<BundleDetailsDto>> {
        unimplemented!("Not used in service tests")
    }

    async fn query_bundles_with_details(
        &self,
        _query: BundleFilterDto,
    ) -> AppResult<Vec<BundleDetailsDto>> {
        unimplemented!("Not used in service tests")
    }

    async fn query_bundles_by_version_id(
        &self,
        _version_id: i32,
    ) -> AppResult<Vec<BundleDetailsDto>> {
        unimplemented!("Not used in service tests")
    }
}

// Combined Repository Mock implementing all repository traits
#[derive(Clone, Debug)]
pub struct MockRepository {
    pub version: MockVersionRepository,
    pub file: MockFileRepository,
    pub bundle: MockBundleRepository,
}

impl MockRepository {
    pub fn new() -> Self {
        Self {
            version: MockVersionRepository::new(),
            file: MockFileRepository::new(),
            bundle: MockBundleRepository::new(),
        }
    }
}

#[async_trait]
impl VersionRepository for MockRepository {
    async fn create_version(&self, version: Version) -> AppResult<i32> {
        self.version.create_version(version).await
    }

    async fn get_version_by_id(&self, id: i32) -> AppResult<Option<Version>> {
        self.version.get_version_by_id(id).await
    }

    async fn get_latest_version(&self) -> AppResult<Option<Version>> {
        self.version.get_latest_version().await
    }

    async fn is_client_and_res_exist(&self, client: &str, res: &str) -> AppResult<bool> {
        self.version.is_client_and_res_exist(client, res).await
    }

    async fn get_oldest_unready_version(&self) -> AppResult<Option<Version>> {
        self.version.get_oldest_unready_version().await
    }

    async fn mark_version_ready(&self, id: i32) -> AppResult<()> {
        self.version.mark_version_ready(id).await
    }

    async fn query_versions(&self) -> AppResult<Vec<VersionDto>> {
        unimplemented!("Not used in service tests")
    }

    async fn query_version_detail_by_id(&self, _id: i32) -> AppResult<Option<VersionDetailDto>> {
        unimplemented!("Not used in service tests")
    }
}

#[async_trait]
impl FileRepository for MockRepository {
    async fn create_file(&self, file: File) -> AppResult<i32> {
        self.file.create_file(file).await
    }

    async fn get_file_by_hash(&self, hash: &str) -> AppResult<Option<File>> {
        self.file.get_file_by_hash(hash).await
    }
}

#[async_trait]
impl BundleRepository for MockRepository {
    async fn create_bundle(&self, bundle: Bundle) -> AppResult<i32> {
        self.bundle.create_bundle(bundle).await
    }

    async fn get_bundle_by_version_and_path(
        &self,
        version_id: i32,
        path: &str,
    ) -> AppResult<Option<Bundle>> {
        self.bundle
            .get_bundle_by_version_and_path(version_id, path)
            .await
    }

    async fn query_bundle_by_id_with_details(
        &self,
        _id: i32,
    ) -> AppResult<Option<BundleDetailsDto>> {
        unimplemented!("Not used in service tests")
    }

    async fn query_bundles_with_details(
        &self,
        _query: BundleFilterDto,
    ) -> AppResult<Vec<BundleDetailsDto>> {
        unimplemented!("Not used in service tests")
    }

    async fn query_bundles_by_version_id(
        &self,
        _version_id: i32,
    ) -> AppResult<Vec<BundleDetailsDto>> {
        unimplemented!("Not used in service tests")
    }
}
