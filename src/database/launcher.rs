use crate::{AppError, AppResult, database::Database};
use chrono::{DateTime, Utc};
use serde::Serialize;
use utoipa::ToSchema;

#[derive(Debug, Clone, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct LauncherRelease {
    pub id: i64,
    pub package_version: String,
    pub game_version: String,
    pub file_path: String,
    pub manifest_md5: String,
    pub discovered_at: DateTime<Utc>,
    pub ready_at: Option<DateTime<Utc>>,
    pub total_files: i64,
    pub archived_files: i64,
    pub total_bytes: i64,
    pub archived_bytes: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Deserialize, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct LauncherEntry {
    pub path: String,
    pub md5: String,
    pub size: i64,
}

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct LauncherFile {
    pub path: String,
    pub md5: String,
    pub size: i64,
    pub sha256: Option<String>,
    pub attempts: i32,
    pub last_error: Option<String>,
    /// Failed every retry. A worker restart or a new publication listing this content requeues it.
    pub retries_exhausted: bool,
}

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct LauncherDiff {
    pub path: String,
    pub change: String,
    pub old_md5: Option<String>,
    pub old_size: Option<i64>,
    pub new_md5: Option<String>,
    pub new_size: Option<i64>,
}

/// A lease on one pending blob.
///
/// Claiming pushes `retry_at` into the future, so no transaction or connection
/// is held while the content is transferred. An interrupted claim becomes
/// claimable again once its lease expires.
///
/// That `retry_at` also identifies the lease. Renewing or failing only applies
/// while the row still holds it, so an attempt whose blob was reset for a new
/// source, or claimed again after its lease expired, cannot overwrite the
/// newer state.
pub struct LauncherBlobJob {
    database: Database,
    pub md5: String,
    pub size: i64,
    lease: DateTime<Utc>,
}

pub struct LauncherBlobSource {
    pub path: String,
    pub file_path: String,
}

impl Database {
    pub async fn launcher_release_exists(&self, version: &str, md5: &str) -> AppResult<bool> {
        sqlx::query_scalar!(
            "SELECT EXISTS(SELECT 1 FROM launcher_releases WHERE package_version = $1 AND manifest_md5 = $2) AS \"exists!\"",
            version, md5
        ).fetch_one(self.pool()).await.map_err(db_error)
    }

    /// The ciphertext and the complete manifest become visible together.
    pub async fn import_launcher_release(
        &self,
        package_version: &str,
        game_version: &str,
        file_path: &str,
        manifest_md5: &str,
        ciphertext: &[u8],
        entries: &[LauncherEntry],
    ) -> AppResult<i64> {
        let mut tx = self.pool().begin().await.map_err(db_error)?;
        let inserted = sqlx::query_scalar!(
            "INSERT INTO launcher_releases (package_version, game_version, file_path, manifest_md5, encrypted_manifest) VALUES ($1, $2, $3, $4, $5) ON CONFLICT (package_version, manifest_md5) DO NOTHING RETURNING id",
            package_version, game_version, file_path, manifest_md5, ciphertext
        ).fetch_optional(&mut *tx).await.map_err(db_error)?;
        let Some(id) = inserted else {
            return sqlx::query_scalar!(
                "SELECT id FROM launcher_releases WHERE package_version = $1 AND manifest_md5 = $2",
                package_version,
                manifest_md5
            )
            .fetch_one(&mut *tx)
            .await
            .map_err(db_error);
        };
        let paths: Vec<_> = entries.iter().map(|e| e.path.clone()).collect();
        let md5s: Vec<_> = entries.iter().map(|e| e.md5.clone()).collect();
        let sizes: Vec<_> = entries.iter().map(|e| e.size).collect();
        sqlx::query!(
            "INSERT INTO launcher_blobs (md5, size) SELECT DISTINCT * FROM UNNEST($1::text[], $2::bigint[]) ORDER BY 1, 2 ON CONFLICT DO NOTHING",
            &md5s, &sizes
        ).execute(&mut *tx).await.map_err(db_error)?;
        sqlx::query!(
            "INSERT INTO launcher_entries (release_id, path, md5, size) SELECT $1, * FROM UNNEST($2::text[], $3::text[], $4::bigint[])",
            id, &paths, &md5s, &sizes
        ).execute(&mut *tx).await.map_err(db_error)?;
        // This release is a new source for content that is waiting out the
        // backoff of earlier failures or has exhausted its retries: retry it
        // now, with fresh retries. A retry in flight loses its lease, so its
        // failure is not recorded; its download may still finish and complete
        // the blob, which is content-addressed.
        sqlx::query!(
            "UPDATE launcher_blobs b SET retry_at = now(), attempts = 0, last_error = NULL
            FROM launcher_entries e
            WHERE e.release_id = $1 AND (b.md5, b.size) = (e.md5, e.size)
                AND b.sha256 IS NULL AND b.last_error IS NOT NULL AND b.retry_at > now()",
            id
        )
        .execute(&mut *tx)
        .await
        .map_err(db_error)?;
        tx.commit().await.map_err(db_error)?;
        self.refresh_launcher_ready().await?;
        Ok(id)
    }

    /// Newest package version first; imports of older releases do not jump the
    /// queue. `before` is the id of the last release on the previous page.
    ///
    /// The page is chosen from `launcher_releases` first, and the progress
    /// view is evaluated per listed id. `OFFSET 0` stops the planner from
    /// merging it back into one aggregate over every release's entries.
    pub async fn list_launcher_releases(
        &self,
        before: Option<i64>,
        limit: i64,
    ) -> AppResult<Vec<LauncherRelease>> {
        sqlx::query_as!(LauncherRelease,
            r#"SELECT p.id AS "id!", p.package_version AS "package_version!", p.game_version AS "game_version!",
            p.file_path AS "file_path!", p.manifest_md5 AS "manifest_md5!", p.discovered_at AS "discovered_at!", p.ready_at,
            p.total_files AS "total_files!", p.archived_files AS "archived_files!", p.total_bytes AS "total_bytes!", p.archived_bytes AS "archived_bytes!"
            FROM (
                SELECT r.id FROM launcher_releases r
                WHERE $1::bigint IS NULL OR (launcher_version_key(r.package_version), r.id) <
                    (SELECT launcher_version_key(c.package_version), c.id FROM launcher_releases c WHERE c.id = $1)
                ORDER BY launcher_version_key(r.package_version) DESC, r.id DESC LIMIT $2
            ) page
            CROSS JOIN LATERAL (SELECT * FROM launcher_release_progress v WHERE v.id = page.id OFFSET 0) p
            ORDER BY launcher_version_key(p.package_version) DESC, p.id DESC"#,
            before, limit
        ).fetch_all(self.pool()).await.map_err(db_error)
    }

    pub async fn launcher_release_id_exists(&self, id: i64) -> AppResult<bool> {
        sqlx::query_scalar!(
            "SELECT EXISTS(SELECT 1 FROM launcher_releases WHERE id = $1) AS \"exists!\"",
            id
        )
        .fetch_one(self.pool())
        .await
        .map_err(db_error)
    }

    pub async fn get_launcher_release(&self, id: i64) -> AppResult<Option<LauncherRelease>> {
        sqlx::query_as!(LauncherRelease,
            r#"SELECT id AS "id!", package_version AS "package_version!", game_version AS "game_version!",
            file_path AS "file_path!", manifest_md5 AS "manifest_md5!", discovered_at AS "discovered_at!", ready_at,
            total_files AS "total_files!", archived_files AS "archived_files!", total_bytes AS "total_bytes!", archived_bytes AS "archived_bytes!"
            FROM launcher_release_progress WHERE id = $1"#,
            id
        ).fetch_optional(self.pool()).await.map_err(db_error)
    }

    pub async fn launcher_files(
        &self,
        id: i64,
        after: &str,
        search: &str,
        limit: i64,
    ) -> AppResult<Vec<LauncherFile>> {
        sqlx::query_as!(LauncherFile,
            r#"SELECT e.path, e.md5, e.size, b.sha256, b.attempts, b.last_error,
            b.sha256 IS NULL AND b.retry_at = 'infinity' AS "retries_exhausted!"
            FROM launcher_entries e JOIN launcher_blobs b USING (md5, size)
            WHERE e.release_id = $1 AND e.path > $2 AND strpos(e.path, $3) > 0 ORDER BY e.path LIMIT $4"#,
            id, after, search, limit
        ).fetch_all(self.pool()).await.map_err(db_error)
    }

    /// A rename appears as an addition and a deletion, with the same content hash.
    pub async fn launcher_diff(
        &self,
        from: i64,
        to: i64,
        after: &str,
        limit: i64,
    ) -> AppResult<Vec<LauncherDiff>> {
        sqlx::query_as!(LauncherDiff,
            r#"SELECT coalesce(o.path, n.path) AS "path!",
            CASE WHEN o.path IS NULL THEN 'added' WHEN n.path IS NULL THEN 'deleted' ELSE 'modified' END AS "change!",
            o.md5 AS "old_md5?", o.size AS "old_size?", n.md5 AS "new_md5?", n.size AS "new_size?"
            FROM (SELECT path, md5, size FROM launcher_entries WHERE release_id = $1) o
            FULL JOIN (SELECT path, md5, size FROM launcher_entries WHERE release_id = $2) n USING (path)
            WHERE (o.md5, o.size) IS DISTINCT FROM (n.md5, n.size) AND coalesce(o.path, n.path) > $3
            ORDER BY coalesce(o.path, n.path) LIMIT $4"#,
            from, to, after, limit
        ).fetch_all(self.pool()).await.map_err(db_error)
    }

    pub async fn launcher_manifest(&self, id: i64) -> AppResult<Option<Vec<u8>>> {
        sqlx::query_scalar!(
            "SELECT encrypted_manifest FROM launcher_releases WHERE id = $1",
            id
        )
        .fetch_optional(self.pool())
        .await
        .map_err(db_error)
    }

    /// Give exhausted blobs a fresh retry budget when the worker starts.
    pub(crate) async fn requeue_exhausted_launcher_blobs(&self) -> AppResult<()> {
        sqlx::query!(
            "UPDATE launcher_blobs SET retry_at = now(), attempts = 0, last_error = NULL
            WHERE sha256 IS NULL AND retry_at = 'infinity'"
        )
        .execute(self.pool())
        .await
        .map_err(db_error)?;
        Ok(())
    }

    /// Lease the pending blob that has waited longest. The row lock only
    /// lasts for this statement; `FOR NO KEY UPDATE` still permits the FK's
    /// KEY SHARE lock when discovery inserts another release's entries.
    pub async fn claim_launcher_blob(
        &self,
        lease_seconds: i32,
    ) -> AppResult<Option<LauncherBlobJob>> {
        let row = sqlx::query!(
            r#"UPDATE launcher_blobs SET retry_at = now() + make_interval(secs => $1::double precision)
            WHERE (md5, size) = (
                SELECT b.md5, b.size FROM launcher_blobs b
                WHERE b.sha256 IS NULL AND b.retry_at <= now()
                    AND EXISTS (SELECT 1 FROM launcher_entries e WHERE (e.md5, e.size) = (b.md5, b.size))
                ORDER BY b.retry_at, b.md5, b.size LIMIT 1 FOR NO KEY UPDATE OF b SKIP LOCKED)
            RETURNING md5, size, retry_at"#,
            f64::from(lease_seconds)
        )
        .fetch_optional(self.pool())
        .await
        .map_err(db_error)?;
        Ok(row.map(|row| LauncherBlobJob {
            database: self.clone(),
            md5: row.md5,
            size: row.size,
            lease: row.retry_at,
        }))
    }

    pub async fn refresh_launcher_ready(&self) -> AppResult<()> {
        sqlx::query!(
            "UPDATE launcher_releases r SET ready_at = now() WHERE ready_at IS NULL AND NOT EXISTS (SELECT 1 FROM launcher_entries e JOIN launcher_blobs b USING (md5, size) WHERE e.release_id = r.id AND b.sha256 IS NULL)"
        ).execute(self.pool()).await.map_err(db_error)?;
        Ok(())
    }
}

impl LauncherBlobJob {
    /// Distinct sources for this content, newest package version first: the
    /// current release's CDN directory is the one most likely to still exist.
    pub async fn sources(&self) -> AppResult<Vec<LauncherBlobSource>> {
        sqlx::query_as!(LauncherBlobSource,
            "SELECT e.path, r.file_path FROM launcher_entries e JOIN launcher_releases r ON r.id = e.release_id
            WHERE e.md5 = $1 AND e.size = $2
            GROUP BY e.path, r.file_path
            ORDER BY max(launcher_version_key(r.package_version)) DESC, max(r.id) DESC, e.path, r.file_path",
            &self.md5, self.size
        ).fetch_all(self.database.pool()).await.map_err(db_error)
    }

    /// Extend the lease before another long step, e.g. the next source.
    /// Fails once the lease has passed to another attempt.
    pub async fn renew(&mut self, lease_seconds: i32) -> AppResult<()> {
        let renewed = sqlx::query_scalar!(
            "UPDATE launcher_blobs SET retry_at = now() + make_interval(secs => $3::double precision)
            WHERE md5 = $1 AND size = $2 AND sha256 IS NULL AND retry_at = $4 RETURNING retry_at",
            &self.md5, self.size, f64::from(lease_seconds), self.lease
        ).fetch_optional(self.database.pool()).await.map_err(db_error)?;
        self.lease = renewed.ok_or_else(|| {
            AppError::Application(anyhow::anyhow!(
                "lease on launcher blob {} passed to another attempt",
                self.md5
            ))
        })?;
        Ok(())
    }

    pub async fn complete(self, sha256: &str) -> AppResult<()> {
        sqlx::query!(
            "UPDATE launcher_blobs SET sha256 = $3, attempts = attempts + 1, last_error = NULL WHERE md5 = $1 AND size = $2 AND sha256 IS NULL",
            &self.md5, self.size, sha256
        ).execute(self.database.pool()).await.map_err(db_error)?;
        Ok(())
    }

    /// Back off exponentially from `retry_seconds` (capped at one day) for
    /// up to `max_retries` retries. After that the content waits for a worker
    /// restart or a new publication that lists it. Does nothing once the
    /// lease has passed on.
    pub async fn fail(self, error: &str, retry_seconds: i32, max_retries: i32) -> AppResult<()> {
        sqlx::query!(
            "UPDATE launcher_blobs SET attempts = attempts + 1, last_error = $3,
                retry_at = CASE WHEN attempts < $5
                    THEN now() + make_interval(secs => least($4::double precision * power(2::double precision, least(attempts, 16)), 86400))
                    ELSE 'infinity' END
            WHERE md5 = $1 AND size = $2 AND sha256 IS NULL AND retry_at = $6",
            &self.md5, self.size, error, f64::from(retry_seconds), max_retries, self.lease
        ).execute(self.database.pool()).await.map_err(db_error)?;
        Ok(())
    }
}

fn db_error(err: sqlx::Error) -> AppError {
    AppError::ExternalService(err.into())
}
