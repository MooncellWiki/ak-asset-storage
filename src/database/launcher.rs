use crate::{AppError, AppResult, database::Database};
use chrono::{DateTime, Utc};
use serde::Serialize;
use sqlx::{Postgres, Transaction};
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

/// A row lock prevents concurrent workers from fetching the same content.
/// Dropping this transaction on shutdown releases it without marking success.
pub struct LauncherBlobJob {
    pub transaction: Transaction<'static, Postgres>,
    pub md5: String,
    pub size: i64,
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
        tx.commit().await.map_err(db_error)?;
        self.refresh_launcher_ready().await?;
        Ok(id)
    }

    pub async fn list_launcher_releases(
        &self,
        before: Option<i64>,
        limit: i64,
    ) -> AppResult<Vec<LauncherRelease>> {
        sqlx::query_as!(LauncherRelease,
            r#"SELECT id AS "id!", package_version AS "package_version!", game_version AS "game_version!",
            file_path AS "file_path!", manifest_md5 AS "manifest_md5!", discovered_at AS "discovered_at!", ready_at,
            total_files AS "total_files!", archived_files AS "archived_files!", total_bytes AS "total_bytes!", archived_bytes AS "archived_bytes!"
            FROM launcher_release_progress WHERE ($1::bigint IS NULL OR id < $1) ORDER BY id DESC LIMIT $2"#,
            before, limit
        ).fetch_all(self.pool()).await.map_err(db_error)
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
            "SELECT e.path, e.md5, e.size, b.sha256, b.attempts, b.last_error FROM launcher_entries e JOIN launcher_blobs b USING (md5, size) WHERE e.release_id = $1 AND e.path > $2 AND strpos(e.path, $3) > 0 ORDER BY e.path LIMIT $4",
            id, after, search, limit
        ).fetch_all(self.pool()).await.map_err(db_error)
    }

    pub async fn launcher_file(&self, id: i64, path: &str) -> AppResult<Option<LauncherFile>> {
        sqlx::query_as!(LauncherFile,
            "SELECT e.path, e.md5, e.size, b.sha256, b.attempts, b.last_error FROM launcher_entries e JOIN launcher_blobs b USING (md5, size) WHERE e.release_id = $1 AND e.path = $2",
            id, path
        ).fetch_optional(self.pool()).await.map_err(db_error)
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

    pub async fn claim_launcher_blob(&self) -> AppResult<Option<LauncherBlobJob>> {
        let mut transaction = self.pool().begin().await.map_err(db_error)?;
        // NO KEY UPDATE excludes other archivers, while permitting the FK's
        // KEY SHARE lock when discovery inserts another release's entries.
        let row = sqlx::query!(
            r#"SELECT b.md5, b.size FROM launcher_blobs b
            WHERE b.sha256 IS NULL AND b.retry_at <= now()
                AND EXISTS (SELECT 1 FROM launcher_entries e WHERE (e.md5, e.size) = (b.md5, b.size))
            ORDER BY b.retry_at, b.md5, b.size LIMIT 1 FOR NO KEY UPDATE OF b SKIP LOCKED"#
        ).fetch_optional(&mut *transaction).await.map_err(db_error)?;
        Ok(row.map(|row| LauncherBlobJob {
            transaction,
            md5: row.md5,
            size: row.size,
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
    /// Keep the claim locked while trying each distinct source for this content.
    pub async fn sources(&mut self) -> AppResult<Vec<LauncherBlobSource>> {
        sqlx::query_as!(LauncherBlobSource,
            "SELECT e.path, r.file_path FROM launcher_entries e JOIN launcher_releases r ON r.id = e.release_id
            WHERE e.md5 = $1 AND e.size = $2
            GROUP BY e.path, r.file_path ORDER BY max(r.id) DESC, e.path, r.file_path",
            &self.md5, self.size
        ).fetch_all(&mut *self.transaction).await.map_err(db_error)
    }

    pub async fn complete(mut self, sha256: &str) -> AppResult<()> {
        sqlx::query!(
            "UPDATE launcher_blobs SET sha256 = $3, attempts = attempts + 1, last_error = NULL WHERE md5 = $1 AND size = $2",
            &self.md5, self.size, sha256
        ).execute(&mut *self.transaction).await.map_err(db_error)?;
        self.transaction.commit().await.map_err(db_error)
    }

    pub async fn fail(mut self, error: &str, retry_seconds: i32) -> AppResult<()> {
        sqlx::query!(
            "UPDATE launcher_blobs SET attempts = attempts + 1, last_error = $3, retry_at = clock_timestamp() + make_interval(secs => $4::double precision) WHERE md5 = $1 AND size = $2",
            &self.md5, self.size, error, f64::from(retry_seconds)
        ).execute(&mut *self.transaction).await.map_err(db_error)?;
        self.transaction.commit().await.map_err(db_error)
    }
}

fn db_error(err: sqlx::Error) -> AppError {
    AppError::ExternalService(err.into())
}
