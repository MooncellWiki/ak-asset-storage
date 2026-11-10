use crate::{AppError, AppResult, config::S3Config};
use bytes::Bytes;
use object_store::{
    ObjectStoreExt,
    aws::{AmazonS3, AmazonS3Builder},
};
use tokio::io::AsyncReadExt;
use tracing::{info, instrument};

#[derive(Debug, Clone)]
pub struct S3Storage {
    store: AmazonS3,
}

fn launcher_blob_path(hash: &str) -> String {
    format!(
        "launcher/blobs/{}/{}/{}",
        &hash[..2],
        &hash[2..4],
        &hash[4..]
    )
}

impl S3Storage {
    pub async fn upload_launcher_blob(
        &self,
        hash: &str,
        file: &mut tokio::fs::File,
    ) -> AppResult<()> {
        self.upload_file(&launcher_blob_path(hash), file).await
    }

    pub async fn download_launcher_blob(&self, hash: &str) -> AppResult<object_store::GetResult> {
        self.download(&launcher_blob_path(hash)).await
    }

    /// Upload a verified raw file with bounded memory and multipart backpressure.
    pub async fn upload_file(&self, path: &str, file: &mut tokio::fs::File) -> AppResult<()> {
        if file.metadata().await?.len() == 0 {
            return self.upload(path, &[]).await;
        }
        let object_path = object_store::path::Path::from(path);
        let upload = self
            .store
            .put_multipart(&object_path)
            .await
            .map_err(|err| AppError::ExternalService(err.into()))?;
        let mut writer = object_store::WriteMultipart::new(upload);
        let mut buffer = vec![0_u8; 1024 * 1024];
        loop {
            let read = match file.read(&mut buffer).await {
                Ok(read) => read,
                Err(err) => {
                    let _ = writer.abort().await;
                    return Err(err.into());
                }
            };
            if read == 0 {
                break;
            }
            if let Err(err) = writer.wait_for_capacity(2).await {
                let _ = writer.abort().await;
                return Err(AppError::ExternalService(err.into()));
            }
            writer.write(&buffer[..read]);
        }
        writer
            .finish()
            .await
            .map_err(|err| AppError::ExternalService(err.into()))?;
        Ok(())
    }

    /// Streams objects directly from the configured private bucket.
    pub async fn download(&self, path: &str) -> AppResult<object_store::GetResult> {
        self.store
            .get(&object_store::path::Path::from(path))
            .await
            .map_err(|err| AppError::ExternalService(err.into()))
    }

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
