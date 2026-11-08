use crate::{
    AppResult,
    config::LauncherConfig,
    database::{
        Database,
        launcher::{LauncherBlobJob, LauncherEntry},
    },
    external::{launcher::LauncherClient, s3::S3Storage},
};
use futures::{StreamExt, stream};
use tracing::{error, info};

#[derive(Clone)]
pub struct LauncherArchiveService {
    pub database: Database,
    pub client: LauncherClient,
    pub storage: S3Storage,
    pub concurrency: usize,
    pub retry_interval_seconds: i32,
}

impl LauncherArchiveService {
    pub fn new(config: &LauncherConfig, database: Database, storage: S3Storage) -> AppResult<Self> {
        Ok(Self {
            database,
            client: LauncherClient::new(config)?,
            storage,
            concurrency: config.concurrency,
            retry_interval_seconds: config.retry_interval_seconds,
        })
    }
    /// No extraction jobs or Android version records are created here.
    pub async fn discover(&self) -> AppResult<()> {
        let publication = self.client.latest().await?;
        if self
            .database
            .launcher_release_exists(&publication.package_version, &publication.manifest_md5)
            .await?
        {
            return Ok(());
        }
        let (ciphertext, entries) = self.client.manifest(&publication).await?;
        let id = self
            .database
            .import_launcher_release(
                &publication.package_version,
                &publication.game_version,
                &publication.file_path,
                &publication.manifest_md5,
                &ciphertext,
                &entries,
            )
            .await?;
        info!(id, files = entries.len(), "Discovered PC launcher release");
        Ok(())
    }

    /// Each task repeatedly claims one unique blob. A failed file is delayed,
    /// so other files and later publications continue to make progress.
    pub async fn archive_pending(&self) -> AppResult<()> {
        stream::iter(0..self.concurrency)
            .for_each_concurrent(self.concurrency, |_| async {
                loop {
                    let job = match self.database.claim_launcher_blob().await {
                        Ok(Some(job)) => job,
                        Ok(None) => break,
                        Err(err) => {
                            error!(?err, "Failed to claim launcher blob");
                            break;
                        }
                    };
                    let result = self.archive_blob(&job).await;
                    let recorded = match result {
                        Ok(hash) => job.complete(&hash).await,
                        Err(err) => {
                            error!(path = job.path, ?err, "Launcher file will be retried");
                            job.fail(&err.to_string(), self.retry_interval_seconds)
                                .await
                        }
                    };
                    if let Err(err) = recorded {
                        error!(?err, "Failed to save launcher blob progress");
                        break;
                    }
                    if let Err(err) = self.database.refresh_launcher_ready().await {
                        error!(?err, "Failed to refresh launcher release status");
                    }
                }
            })
            .await;
        // Also repairs a shutdown between the final blob commit and readiness.
        self.database.refresh_launcher_ready().await
    }

    async fn archive_blob(&self, job: &LauncherBlobJob) -> AppResult<String> {
        let entry = LauncherEntry {
            path: job.path.clone(),
            md5: job.md5.clone(),
            size: job.size,
        };
        let (mut file, hash) = self.client.download(&job.file_path, &entry).await?;
        self.storage
            .upload_file(&format!("launcher/blobs/{hash}"), &mut file)
            .await?;
        Ok(hash)
    }
}
