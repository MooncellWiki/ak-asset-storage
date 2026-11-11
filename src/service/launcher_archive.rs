use crate::{
    AppResult,
    config::LauncherConfig,
    database::{
        Database,
        launcher::{LauncherBlobJob, LauncherEntry},
    },
    external::{
        launcher::{LauncherClient, LauncherPublication},
        s3::S3Storage,
    },
};
use futures::{StreamExt, stream};
use std::path::Path;
use tracing::{error, info, warn};

#[derive(Clone)]
pub struct LauncherArchiveService {
    pub database: Database,
    pub client: LauncherClient,
    pub storage: S3Storage,
    pub concurrency: usize,
    pub retry_interval_seconds: i32,
    pub max_retries: i32,
    /// How long a claimed blob stays hidden from other claims. Renewed per
    /// source; an interrupted claim is retried once this expires.
    pub claim_lease_seconds: i32,
}

impl LauncherArchiveService {
    pub fn new(config: &LauncherConfig, database: Database, storage: S3Storage) -> AppResult<Self> {
        Ok(Self {
            database,
            client: LauncherClient::new(config)?,
            storage,
            concurrency: config.concurrency,
            retry_interval_seconds: config.retry_interval_seconds,
            max_retries: config.max_retries,
            claim_lease_seconds: config.claim_lease_seconds(),
        })
    }
    /// No extraction jobs or Android version records are created here.
    pub async fn discover(&self) -> AppResult<()> {
        let publication = self.client.latest().await?;
        import_publication(&self.database, &self.client, &publication, None).await?;
        Ok(())
    }

    /// Each task repeatedly claims one unique blob. A failed file is delayed,
    /// so other files and later publications continue to make progress.
    pub async fn archive_pending(&self) -> AppResult<()> {
        stream::iter(0..self.concurrency)
            .for_each_concurrent(self.concurrency, |_| async {
                loop {
                    let claim = self
                        .database
                        .claim_launcher_blob(self.claim_lease_seconds)
                        .await;
                    let mut job = match claim {
                        Ok(Some(job)) => job,
                        Ok(None) => break,
                        Err(err) => {
                            error!(?err, "Failed to claim launcher blob");
                            break;
                        }
                    };
                    let result = self.archive_blob(&mut job).await;
                    let archived = result.is_ok();
                    let recorded = match result {
                        Ok(hash) => job.complete(&hash).await,
                        Err(err) => {
                            error!(md5 = job.md5, size = job.size, ?err, "Launcher file failed");
                            job.fail(
                                &err.to_string(),
                                self.retry_interval_seconds,
                                self.max_retries,
                            )
                            .await
                        }
                    };
                    if let Err(err) = recorded {
                        error!(?err, "Failed to save launcher blob progress");
                        break;
                    }
                    // Only a newly archived blob can complete a release.
                    if archived && let Err(err) = self.database.refresh_launcher_ready().await {
                        error!(?err, "Failed to refresh launcher release status");
                    }
                }
            })
            .await;
        // Also repairs a shutdown between the final blob commit and readiness.
        self.database.refresh_launcher_ready().await
    }

    async fn archive_blob(&self, job: &mut LauncherBlobJob) -> AppResult<String> {
        let mut last_error = anyhow::anyhow!("no sources for launcher blob {}", job.md5);
        for (index, source) in job.sources().await?.into_iter().enumerate() {
            if index > 0 {
                job.renew(self.claim_lease_seconds).await?;
            }
            let entry = LauncherEntry {
                path: source.path,
                md5: job.md5.clone(),
                size: job.size,
            };
            let (mut file, hash) = match self.client.download(&source.file_path, &entry).await {
                Ok(download) => download,
                Err(err) => {
                    warn!(
                        path = entry.path,
                        base = source.file_path,
                        ?err,
                        "Launcher source failed; trying another source"
                    );
                    last_error = err;
                    continue;
                }
            };
            // Once bytes are verified, a storage failure ends this attempt.
            self.storage.upload_launcher_blob(&hash, &mut file).await?;
            return Ok(hash);
        }
        Err(last_error.into())
    }
}

pub(crate) async fn import_publication(
    database: &Database,
    client: &LauncherClient,
    publication: &LauncherPublication,
    game_files_path: Option<&Path>,
) -> AppResult<bool> {
    if database
        .launcher_release_exists(&publication.package_version, &publication.manifest_md5)
        .await?
    {
        return Ok(false);
    }
    let (ciphertext, entries) = match game_files_path {
        Some(path) => client.local_manifest(publication, path).await?,
        None => client.manifest(publication).await?,
    };
    let id = database
        .import_launcher_release(
            &publication.package_version,
            &publication.game_version,
            &publication.file_path,
            &publication.manifest_md5,
            &ciphertext,
            &entries,
        )
        .await?;
    info!(
        id,
        package_version = publication.package_version,
        files = entries.len(),
        "Imported PC launcher release"
    );
    Ok(true)
}
