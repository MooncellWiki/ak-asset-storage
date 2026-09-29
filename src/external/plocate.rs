use crate::{AppError, AppResult, config::PlocateConfig};
use anyhow::Context;
use std::{
    path::{Path, PathBuf},
    process::Command,
    time::{Duration, Instant},
};
use tracing::{debug, info, warn};

/// plocate-backed search index over the extracted asset tree.
///
/// `updatedb` scans `asset_root` into `database_path` (an incremental build —
/// unchanged directories are skipped) and installs the result with an atomic
/// rename, so a failed build keeps the previous database and searches never
/// observe a half-written one. Searches therefore return a snapshot as of the
/// last successful build, not the live tree. The database directory itself is
/// pruned from the index.
#[derive(Debug, Clone)]
pub struct PlocateIndex {
    asset_root: PathBuf,
    database_path: PathBuf,
    search_limit: usize,
}

impl PlocateIndex {
    pub fn new(asset_base_path: &Path, config: &PlocateConfig) -> AppResult<Self> {
        Self::require_binary("updatedb")?;
        Self::require_binary("plocate")?;
        let asset_root = asset_base_path
            .canonicalize()
            .with_context(|| {
                format!(
                    "failed to resolve asset base path {}; create it or set torappu.plocate.enabled = false",
                    asset_base_path.display()
                )
            })
            .map_err(AppError::Application)?;
        let database_path = config.database_path.as_ref().map_or_else(
            || asset_root.join(".catalog").join("plocate.db"),
            PathBuf::from,
        );
        let parent = database_path.parent().unwrap_or_else(|| Path::new("/"));
        std::fs::create_dir_all(parent)
            .with_context(|| {
                format!(
                    "failed to create plocate database directory {}",
                    parent.display()
                )
            })
            .map_err(AppError::Application)?;
        Ok(Self {
            asset_root,
            database_path,
            search_limit: config.search_limit,
        })
    }

    /// Canonical absolute root recorded in the index; matches are reported
    /// below it and relative entry paths are derived from it.
    #[must_use]
    pub fn asset_root(&self) -> &Path {
        &self.asset_root
    }

    /// `updatedb` and `plocate` must both exist: one builds the index, the
    /// other queries it. Failing fast at startup beats failing per request.
    fn require_binary(name: &str) -> AppResult<()> {
        let output = Command::new(name)
            .arg("--version")
            .output()
            .with_context(|| {
                format!("`{name}` not found in PATH: install the plocate package or set torappu.plocate.enabled = false")
            })
            .map_err(AppError::Application)?;
        if output.status.success() {
            Ok(())
        } else {
            Err(AppError::Application(anyhow::anyhow!(
                "`{name} --version` exited with {}",
                output.status
            )))
        }
    }

    /// Rebuild the database. Blocking (subprocess + filesystem scan); call
    /// from `spawn_blocking`. The system `updatedb.conf` is bypassed so the
    /// result only depends on the flags below, and the database directory is
    /// pruned to keep the index (and its rewrite churn) out of itself.
    pub fn update(&self) -> AppResult<()> {
        let prune_dir = self
            .database_path
            .parent()
            .unwrap_or_else(|| Path::new("/"));
        let output = Command::new("updatedb")
            .arg("--config-file")
            .arg("/dev/null")
            .arg("--require-visibility")
            .arg("no")
            .arg("--prune-bind-mounts")
            .arg("no")
            .arg("--database-root")
            .arg(&self.asset_root)
            .arg("--output")
            .arg(&self.database_path)
            .arg("--add-single-prunepath")
            .arg(prune_dir)
            .output()
            .context("failed to spawn updatedb")
            .map_err(AppError::Application)?;
        if !output.status.success() {
            return Err(AppError::Application(anyhow::anyhow!(
                "updatedb exited with {}: {}",
                output.status,
                String::from_utf8_lossy(&output.stderr).trim()
            )));
        }
        Ok(())
    }

    /// Look up raw path matches for `query`. Blocking (subprocess); call from
    /// `spawn_blocking`. Returns at most `search_limit` absolute paths; the
    /// caller applies entry-level filtering and metadata lookups.
    pub fn lookup(&self, query: &str) -> AppResult<Vec<PathBuf>> {
        if !self.database_path.is_file() {
            return Err(AppError::ExternalService(anyhow::anyhow!(
                "plocate database {} does not exist yet; the periodic updatedb task has not finished a first build",
                self.database_path.display()
            )));
        }
        let output = Command::new("plocate")
            .arg("--database")
            .arg(&self.database_path)
            .arg("--limit")
            .arg(self.search_limit.to_string())
            .arg("--null")
            .arg("--")
            .arg(query)
            .output()
            .context("failed to spawn plocate")
            .map_err(AppError::Application)?;
        // plocate exits 1 both for "no matches" and for real errors; only
        // the latter writes to stderr.
        if !output.status.success() && !output.stderr.is_empty() {
            return Err(AppError::Application(anyhow::anyhow!(
                "plocate exited with {}: {}",
                output.status,
                String::from_utf8_lossy(&output.stderr).trim()
            )));
        }
        let stdout = output.stdout;
        let mut paths = Vec::new();
        for raw in stdout.split(|byte| *byte == 0) {
            if raw.is_empty() {
                continue;
            }
            let Ok(path) = std::str::from_utf8(raw) else {
                debug!(path = ?raw, "skipping non-UTF-8 plocate match");
                continue;
            };
            let path = Path::new(path);
            if !path.starts_with(&self.asset_root) {
                debug!(path = %path.display(), "skipping plocate match outside the asset root");
                continue;
            }
            paths.push(path.to_path_buf());
        }
        Ok(paths)
    }
}

/// Periodically refresh the index for the lifetime of the server process.
///
/// The first build runs immediately, then every `interval`. Builds are
/// sequential by construction; a failed build is logged and retried on the
/// next tick, leaving the previous database in place for searches.
pub fn spawn_update_task(index: PlocateIndex, interval: Duration) {
    tokio::spawn(async move {
        loop {
            let started = Instant::now();
            let database = index.database_path.clone();
            let task_index = index.clone();
            let result = tokio::task::spawn_blocking(move || task_index.update()).await;
            match result {
                Ok(Ok(())) => info!(
                    duration_secs = started.elapsed().as_secs_f64(),
                    database = %database.display(),
                    "plocate index refreshed"
                ),
                Ok(Err(err)) => warn!(
                    error = %err,
                    database = %database.display(),
                    "plocate index refresh failed; keeping the previous database"
                ),
                Err(err) => warn!(error = %err, "plocate index refresh task panicked"),
            }
            tokio::time::sleep(interval).await;
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    fn binaries_available() -> bool {
        Command::new("updatedb").arg("--version").output().is_ok()
            && Command::new("plocate").arg("--version").output().is_ok()
    }

    fn tempdir(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "ak-asset-storage-plocate-{tag}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn test_config() -> PlocateConfig {
        PlocateConfig {
            enabled: true,
            database_path: None,
            update_interval_seconds: 600,
            search_limit: 1000,
        }
    }

    #[test]
    fn update_and_lookup_round_trip() {
        if !binaries_available() {
            return;
        }
        let root = tempdir("roundtrip");
        std::fs::create_dir_all(root.join("raw/char_100/sub/deeper")).unwrap();
        std::fs::write(root.join("raw/char_100/avg_npc_009.png"), b"x").unwrap();
        std::fs::write(root.join("raw/char_100/sub/avg_npc_009_2.png"), b"x").unwrap();
        std::fs::write(root.join("raw/char_100/sub/deeper/avg_npc_009_3.png"), b"x").unwrap();
        std::fs::create_dir_all(root.join(".catalog")).unwrap();
        std::fs::write(root.join(".catalog/leaked"), b"x").unwrap();

        let index = PlocateIndex::new(&root, &test_config()).unwrap();
        index.update().unwrap();

        let matches = index.lookup("avg_npc_009").unwrap();
        let relative = |path: &Path| {
            path.strip_prefix(&index.asset_root)
                .unwrap()
                .to_str()
                .unwrap()
                .to_string()
        };
        let mut found: Vec<String> = matches.iter().map(|path| relative(path)).collect();
        found.sort();
        assert_eq!(
            found,
            vec![
                "raw/char_100/avg_npc_009.png".to_string(),
                "raw/char_100/sub/avg_npc_009_2.png".to_string(),
                "raw/char_100/sub/deeper/avg_npc_009_3.png".to_string(),
            ]
        );

        // The pruned database directory must not be indexed.
        assert!(index.lookup("leaked").unwrap().is_empty());
        // Misses are empty results, not errors.
        assert!(index.lookup("zzz_no_match_zzz").unwrap().is_empty());

        std::fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn lookup_without_database_is_an_external_error() {
        if !binaries_available() {
            return;
        }
        let root = tempdir("nodb");
        let config = PlocateConfig {
            database_path: Some(
                root.join("missing/plocate.db")
                    .to_str()
                    .unwrap()
                    .to_string(),
            ),
            ..test_config()
        };
        let index = PlocateIndex::new(&root, &config).unwrap();
        let err = index.lookup("anything").unwrap_err();
        assert!(matches!(err, AppError::ExternalService(_)), "{err:?}");
        std::fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn update_is_incremental_and_survives_a_second_run() {
        if !binaries_available() {
            return;
        }
        let root = tempdir("incremental");
        std::fs::create_dir_all(root.join("gamedata/v1")).unwrap();
        std::fs::write(root.join("gamedata/v1/one.json"), b"x").unwrap();

        let index = PlocateIndex::new(&root, &test_config()).unwrap();
        index.update().unwrap();
        std::fs::write(root.join("gamedata/v1/two.json"), b"x").unwrap();
        index.update().unwrap();

        assert_eq!(index.lookup("two.json").unwrap().len(), 1);
        std::fs::remove_dir_all(&root).unwrap();
    }
}
