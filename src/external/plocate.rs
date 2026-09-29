use crate::{AppError, AppResult, config::PlocateConfig};
use anyhow::Context;
use std::{
    io::{BufRead, BufReader, Read as _},
    path::{Path, PathBuf},
    process::{Command, Stdio},
    time::{Duration, Instant},
};
use tracing::{debug, info, warn};

/// plocate-backed search index over the extracted asset tree.
///
/// `updatedb` scans `asset_root` into `database_path` (an incremental build —
/// unchanged directories are skipped) and installs the result with an atomic
/// rename, so a failed build keeps the previous database and searches never
/// observe a half-written one. Searches therefore return a snapshot as of the
/// last successful build, not the live tree. Only the database file itself is
/// pruned from the index, so a custom database location inside the asset
/// tree cannot silently exclude assets.
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
    /// from `spawn_blocking`. Every prune setting of the system
    /// `updatedb.conf` is overridden so the result only depends on the flags
    /// below — its defaults skip network/FUSE filesystems (`PRUNEFS`) and
    /// paths such as `/tmp` (`PRUNEPATHS`), which would silently drop an
    /// asset volume. Explicit empty lists are used instead of
    /// `--config-file /dev/null`, which only exists since plocate 1.1.24;
    /// the runtime image (Debian 13) ships 1.1.23. Pruning targets the
    /// database file itself (not its directory) so the database location can
    /// never exclude surrounding assets from the index.
    pub fn update(&self) -> AppResult<()> {
        let output = Command::new("updatedb")
            .arg("--prunefs")
            .arg("")
            .arg("--prunenames")
            .arg("")
            .arg("--prunepaths")
            .arg("")
            .arg("--require-visibility")
            .arg("no")
            .arg("--prune-bind-mounts")
            .arg("no")
            .arg("--database-root")
            .arg(&self.asset_root)
            .arg("--output")
            .arg(&self.database_path)
            .arg("--add-single-prunepath")
            .arg(&self.database_path)
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

    /// Look up matches for `query` under the asset root. Blocking
    /// (subprocess); call from `spawn_blocking`.
    ///
    /// Streams plocate's output and applies `keep` to each candidate path,
    /// returning the first `limit` candidates that pass — so the limit is
    /// enforced after caller-side filtering and cannot be consumed by
    /// candidates that the caller would discard anyway. Once enough
    /// candidates are collected the subprocess is killed instead of being
    /// allowed to enumerate the remaining matches.
    pub fn lookup_filtered(
        &self,
        query: &str,
        limit: usize,
        keep: &dyn Fn(&str) -> bool,
    ) -> AppResult<Vec<PathBuf>> {
        if !self.is_built() {
            return Err(AppError::Unavailable(
                "search index is not built yet; retry shortly".to_string(),
            ));
        }
        let mut child = Command::new("plocate")
            .arg("--database")
            .arg(&self.database_path)
            .arg("--null")
            .arg("--")
            .arg(escape_glob_metacharacters(query))
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .context("failed to spawn plocate")
            .map_err(AppError::Application)?;
        let stdout = child
            .stdout
            .take()
            .context("plocate stdout was not piped")
            .map_err(AppError::Application)?;
        let matches = read_matches(BufReader::new(stdout), &self.asset_root, limit, keep);
        // Stop the enumeration instead of draining the remaining matches once
        // enough are kept. A read error kills it too, so no plocate process is
        // left running or unreaped after this call.
        let stopped_early = matches.as_ref().map_or(true, |kept| kept.len() >= limit);
        if stopped_early {
            let _ = child.kill();
        }
        let status = child
            .wait()
            .context("failed to wait for plocate")
            .map_err(AppError::Application)?;
        let kept = matches?;
        // plocate exits 1 both for "no matches" and for real errors; only
        // the latter writes to stderr. Skip the check when we killed it.
        if !stopped_early && !status.success() {
            let mut stderr = String::new();
            if let Some(mut pipe) = child.stderr.take() {
                let _ = pipe.read_to_string(&mut stderr);
            }
            if !stderr.trim().is_empty() {
                return Err(AppError::Application(anyhow::anyhow!(
                    "plocate exited with {}: {}",
                    status,
                    stderr.trim()
                )));
            }
        }
        Ok(kept)
    }

    /// Configured upper bound on entries returned per search.
    #[must_use]
    pub const fn search_limit(&self) -> usize {
        self.search_limit
    }

    /// Whether a database exists to query. updatedb installs it with an
    /// atomic rename, so once present it never disappears between builds.
    #[must_use]
    pub fn is_built(&self) -> bool {
        self.database_path.is_file()
    }
}

/// Reads plocate's NUL-separated output, keeping candidates under
/// `asset_root` that pass `keep`, until `limit` are kept or output ends.
fn read_matches(
    mut reader: impl BufRead,
    asset_root: &Path,
    limit: usize,
    keep: &dyn Fn(&str) -> bool,
) -> AppResult<Vec<PathBuf>> {
    let mut kept = Vec::new();
    let mut line = Vec::new();
    loop {
        line.clear();
        let read = reader
            .read_until(0, &mut line)
            .context("failed to read plocate output")
            .map_err(AppError::Application)?;
        if read == 0 {
            return Ok(kept);
        }
        if line.last() == Some(&0) {
            line.pop();
        }
        let Ok(candidate) = std::str::from_utf8(&line) else {
            debug!(path = ?line, "skipping non-UTF-8 plocate match");
            continue;
        };
        if !Path::new(candidate).starts_with(asset_root) {
            debug!(
                path = candidate,
                "skipping plocate match outside the asset root"
            );
            continue;
        }
        if keep(candidate) {
            kept.push(PathBuf::from(candidate));
            if kept.len() >= limit {
                return Ok(kept);
            }
        }
    }
}

/// Whether `updatedb`/`plocate` are installed, for tests that need them.
/// They skip when the binaries are missing, except under CI (which installs
/// them): a silent skip there once hid an updatedb flag that the runtime
/// image rejects.
#[cfg(test)]
pub(crate) fn binaries_available_for_tests() -> bool {
    let available = Command::new("updatedb").arg("--version").output().is_ok()
        && Command::new("plocate").arg("--version").output().is_ok();
    assert!(
        available || std::env::var_os("CI").is_none(),
        "updatedb/plocate must be installed when CI is set"
    );
    available
}

/// plocate treats a pattern containing unescaped `*`, `?` or `[` as a glob
/// (verified against plocate 1.1), so a literal query like `portrait[1]`
/// would be read as a character class and match nothing. Escaping those
/// three keeps every query a literal substring match.
///
/// Known limitation: plocate (1.1) has no reliable way to express a literal
/// backslash in a pattern — neither `x\y` nor `x\\y` matches a filename
/// containing one — so queries containing `\` are best-effort. Game asset
/// paths do not contain backslashes.
fn escape_glob_metacharacters(query: &str) -> String {
    let mut escaped = String::with_capacity(query.len());
    for ch in query.chars() {
        if matches!(ch, '*' | '?' | '[') {
            escaped.push('\\');
        }
        escaped.push(ch);
    }
    escaped
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

    fn lookup_all(index: &PlocateIndex, query: &str) -> Vec<String> {
        index
            .lookup_filtered(query, 1000, &|_| true)
            .unwrap()
            .iter()
            .map(|path| {
                path.strip_prefix(index.asset_root())
                    .unwrap()
                    .to_str()
                    .unwrap()
                    .to_string()
            })
            .collect()
    }

    #[test]
    fn update_and_lookup_round_trip() {
        if !binaries_available_for_tests() {
            return;
        }
        let root = tempdir("roundtrip");
        std::fs::create_dir_all(root.join("raw/char_100/sub/deeper")).unwrap();
        std::fs::write(root.join("raw/char_100/avg_npc_009.png"), b"x").unwrap();
        std::fs::write(root.join("raw/char_100/sub/avg_npc_009_2.png"), b"x").unwrap();
        std::fs::write(root.join("raw/char_100/sub/deeper/avg_npc_009_3.png"), b"x").unwrap();
        std::fs::create_dir_all(root.join(".catalog")).unwrap();
        std::fs::write(root.join(".catalog/sidecar"), b"x").unwrap();

        let index = PlocateIndex::new(&root, &test_config()).unwrap();
        index.update().unwrap();

        let mut found = lookup_all(&index, "avg_npc_009");
        found.sort();
        assert_eq!(
            found,
            vec![
                "raw/char_100/avg_npc_009.png".to_string(),
                "raw/char_100/sub/avg_npc_009_2.png".to_string(),
                "raw/char_100/sub/deeper/avg_npc_009_3.png".to_string(),
            ]
        );

        // Only the database file itself is pruned; sibling files stay
        // searchable, and the database does not match itself.
        assert_eq!(lookup_all(&index, "sidecar").len(), 1);
        assert!(lookup_all(&index, "plocate.db").is_empty());
        // Misses are empty results, not errors.
        assert!(lookup_all(&index, "zzz_no_match_zzz").is_empty());

        std::fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn glob_metacharacters_in_queries_stay_literal() {
        if !binaries_available_for_tests() {
            return;
        }
        let root = tempdir("glob");
        std::fs::create_dir_all(root.join("raw")).unwrap();
        std::fs::write(root.join("raw/portrait[1].png"), b"x").unwrap();
        std::fs::write(root.join("raw/star*name.png"), b"x").unwrap();
        std::fs::write(root.join("raw/question?mark.png"), b"x").unwrap();
        std::fs::write(root.join("raw/plain.png"), b"x").unwrap();

        let index = PlocateIndex::new(&root, &test_config()).unwrap();
        index.update().unwrap();

        // Without escaping, plocate would read these as fnmatch globs and
        // find nothing.
        assert_eq!(lookup_all(&index, "portrait[1]").len(), 1);
        assert_eq!(lookup_all(&index, "star*name").len(), 1);
        assert_eq!(lookup_all(&index, "question?mark").len(), 1);
        // An actual glob must not silently widen the match set either: the
        // escaped `star*` only matches the literal star file.
        assert_eq!(lookup_all(&index, "star*").len(), 1);

        std::fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn limit_applies_after_filtering() {
        if !binaries_available_for_tests() {
            return;
        }
        let root = tempdir("limit");
        // Sorted raw order: `aa_match` (kept), `aa_match/sub` (kept — one
        // slash after the match), `aa_match/sub/deeper` and the files below
        // it (filtered — two or more slashes), then `zz/zz_match.png`
        // (kept). A limit applied before filtering would be consumed by the
        // deep descendants and hide the zz match even at this size.
        for deep in ["aa_match/sub/deeper/f1.png", "aa_match/sub/deeper/f2.png"] {
            std::fs::create_dir_all(root.join(deep).parent().unwrap()).unwrap();
            std::fs::write(root.join(deep), b"x").unwrap();
        }
        std::fs::create_dir_all(root.join("zz")).unwrap();
        std::fs::write(root.join("zz/zz_match.png"), b"x").unwrap();

        let index = PlocateIndex::new(&root, &test_config()).unwrap();
        index.update().unwrap();

        let kept = index
            .lookup_filtered("match", 3, &|candidate| {
                crate::external::torappu::within_one_directory(candidate, "match")
            })
            .unwrap();
        let relative: Vec<String> = kept
            .iter()
            .map(|path| {
                path.strip_prefix(index.asset_root())
                    .unwrap()
                    .to_str()
                    .unwrap()
                    .to_string()
            })
            .collect();
        assert_eq!(
            relative,
            vec![
                "aa_match".to_string(),
                "aa_match/sub".to_string(),
                "zz/zz_match.png".to_string(),
            ]
        );

        std::fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn database_inside_the_asset_root_still_indexes_everything() {
        if !binaries_available_for_tests() {
            return;
        }
        let root = tempdir("rootdb");
        std::fs::create_dir_all(root.join("raw")).unwrap();
        std::fs::write(root.join("raw/avg_npc_009.png"), b"x").unwrap();

        let config = PlocateConfig {
            database_path: Some(root.join("index.db").to_str().unwrap().to_string()),
            ..test_config()
        };
        let index = PlocateIndex::new(&root, &config).unwrap();
        index.update().unwrap();

        assert_eq!(lookup_all(&index, "avg_npc_009").len(), 1);
        std::fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn lookup_without_database_is_unavailable() {
        if !binaries_available_for_tests() {
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
        assert!(!index.is_built());
        let err = index
            .lookup_filtered("anything", 10, &|_| true)
            .unwrap_err();
        assert!(matches!(err, AppError::Unavailable(_)), "{err:?}");
        std::fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn update_is_incremental_and_survives_a_second_run() {
        if !binaries_available_for_tests() {
            return;
        }
        let root = tempdir("incremental");
        std::fs::create_dir_all(root.join("gamedata/v1")).unwrap();
        std::fs::write(root.join("gamedata/v1/one.json"), b"x").unwrap();

        let index = PlocateIndex::new(&root, &test_config()).unwrap();
        index.update().unwrap();
        std::fs::write(root.join("gamedata/v1/two.json"), b"x").unwrap();
        index.update().unwrap();

        assert_eq!(lookup_all(&index, "two.json").len(), 1);
        std::fs::remove_dir_all(&root).unwrap();
    }
}
