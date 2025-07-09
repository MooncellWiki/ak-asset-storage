use crate::error::InfraError;
use crate::InfraResult;
use application::ports::external_services::StorageService;
use application::{error::AppResult, S3Config};
use async_trait::async_trait;
use bytes::Bytes;
use object_store::aws::AmazonS3Builder;
use object_store::{aws::AmazonS3, ObjectStore};
use tracing::{info, instrument};

pub struct S3StorageClient {
    store: AmazonS3,
}

impl S3StorageClient {
    pub fn new(config: &S3Config) -> InfraResult<Self> {
        let store = AmazonS3Builder::new()
            .with_allow_http(true)
            .with_endpoint(&config.endpoint)
            .with_bucket_name(&config.bucket_name)
            .with_access_key_id(&config.access_key_id)
            .with_secret_access_key(&config.secret_access_key)
            .with_virtual_hosted_style_request(config.with_virtual_hosted_style_request)
            .build()?;
        Ok(Self { store })
    }
}

#[async_trait]
impl StorageService for S3StorageClient {
    #[instrument(name = "s3.upload", skip(self, data))]
    async fn upload(&self, path: &str, data: &[u8]) -> AppResult<()> {
        let object_path = object_store::path::Path::from(path);

        // Convert to Bytes and then into PutPayload to avoid lifetime issues
        let bytes = Bytes::copy_from_slice(data);

        self.store
            .put(&object_path, bytes.into())
            .await
            .map_err(InfraError::from)?;

        info!("Uploaded file to S3 {}", path);
        Ok(())
    }

    #[instrument(name = "s3.download", skip(self))]
    async fn download(&self, path: &str) -> AppResult<Vec<u8>> {
        let object_path = object_store::path::Path::from(path);

        let result = self
            .store
            .get(&object_path)
            .await
            .map_err(InfraError::from)?;

        let bytes = result.bytes().await.map_err(InfraError::from)?;

        info!("Downloaded file from S3 {}", path);
        Ok(bytes.to_vec())
    }

    #[instrument(name = "s3.delete", skip(self))]
    async fn delete(&self, path: &str) -> AppResult<()> {
        let object_path = object_store::path::Path::from(path);

        self.store
            .delete(&object_path)
            .await
            .map_err(InfraError::from)?;

        info!("Deleted file from S3 {}", path);
        Ok(())
    }

    #[instrument(name = "s3.exists", skip(self))]
    async fn exists(&self, path: &str) -> AppResult<bool> {
        let object_path = object_store::path::Path::from(path);

        match self.store.head(&object_path).await {
            Ok(_) => Ok(true),
            Err(object_store::Error::NotFound { .. }) => Ok(false),
            Err(e) => Err(InfraError::from(e).into()),
        }
    }
}
