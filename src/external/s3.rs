use crate::{AppError, AppResult, config::S3Config};
use bytes::Bytes;
use object_store::{
    ObjectStoreExt,
    aws::{AmazonS3, AmazonS3Builder},
};
use tracing::{info, instrument};

#[derive(Debug, Clone)]
pub struct S3Storage {
    store: AmazonS3,
}

impl S3Storage {
    pub fn new(config: &S3Config) -> AppResult<Self> {
        let store = AmazonS3Builder::new()
            .with_allow_http(true)
            .with_endpoint(&config.endpoint)
            .with_bucket_name(&config.bucket_name)
            .with_access_key_id(&config.access_key_id)
            .with_secret_access_key(&config.secret_access_key)
            .with_virtual_hosted_style_request(config.with_virtual_hosted_style_request)
            .build()
            .map_err(|err| AppError::ExternalService(err.into()))?;
        Ok(Self { store })
    }

    #[instrument(name = "s3.upload", skip(self, data))]
    pub async fn upload(&self, path: &str, data: &[u8]) -> AppResult<()> {
        let object_path = object_store::path::Path::from(path);
        let bytes = Bytes::copy_from_slice(data);

        self.store
            .put(&object_path, bytes.into())
            .await
            .map_err(|err| AppError::ExternalService(err.into()))?;

        info!("Uploaded file to S3 {path}");
        Ok(())
    }
}
