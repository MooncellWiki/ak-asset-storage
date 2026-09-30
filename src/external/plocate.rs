use crate::{
    AppError, AppResult,
    config::PlocateConfig,
    external::types::{AssetEntry, AssetSearchResults},
};
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
/// last successful build, not the live tree. The database must live outside
/// the asset tree so internal index files never appear in asset listings.
#[derive(Debug, Clone)]
pub struct PlocateIndex {
    asset_root: PathBuf,
    database_path: PathBuf,
}

impl PlocateIndex {
    pub fn new(asset_base_path: &Path, config: &PlocateConfig) -> AppResult<Self> {
        Self::require_binary("updatedb")?;
        Self::require_binary("plocate")?;
        let asset_root = asset_base_path
            .canonicalize()
            .with_context(|| {
                format!(
                    "failed to resolve asset base path {}; create it before starting the server",
                    asset_base_path.display()
                )
            })
            .map_err(AppError::Application)?;
        let database_path = config.database_path.as_ref().map_or_else(
            || PathBuf::from("/var/lib/ak-asset-storage/plocate.db"),
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
        let database_path = parent
            .canonicalize()
            .context("failed to resolve plocate database directory")?
            .join(
                database_path
                    .file_name()
                    .context("plocate database path must name a file")?,
            );
        if database_path.starts_with(&asset_root) {
            return Err(AppError::Application(anyhow::anyhow!(
                "torappu.plocate.database_path must be outside the asset base path"
            )));
        }
        Ok(Self {
            asset_root,
            database_path,
        })
    }

    /// `updatedb` and `plocate` must both exist: one builds the index, the
    /// other queries it. Failing fast at startup beats failing per request.
    fn require_binary(name: &str) -> AppResult<()> {
        let output = Command::new(name)
            .arg("--version")
            .output()
            .with_context(|| format!("`{name}` not found in PATH: install the plocate package"))
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
    /// the runtime image (Debian 13) ships 1.1.23. The database is outside
    /// the asset root, so it does not need a prune rule.
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

    /// Search relative asset paths by case-sensitive literal substring.
    /// Blocking: run behind the shared search gate in `spawn_blocking`.
    /// Collect one extra valid entry to detect truncation without counting
    /// every match. There is no pagination or exact total.
    pub fn search(&self, query: &str, limit: Option<u32>) -> AppResult<AssetSearchResults> {
        if query.is_empty() || query.contains('\0') {
            return Err(AppError::InvalidInput(
                "search path must not be empty or contain NUL characters".to_string(),
            ));
        }
        let limit = limit.unwrap_or(100);
        if !(1..=200).contains(&limit) {
            return Err(AppError::InvalidInput(
                "search limit must be between 1 and 200".to_string(),
            ));
        }
        let limit = limit as usize;
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
            .arg(search_pattern(query))
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
        let matches = read_matches(BufReader::new(stdout), &self.asset_root, query, limit + 1);
        // Stop the enumeration instead of draining the remaining matches once
        // enough are kept. A read error kills it too, so no plocate process is
        // left running or unreaped after this call.
        let stopped_early = matches.as_ref().map_or(true, |kept| kept.len() > limit);
        if stopped_early {
            let _ = child.kill();
        }
        let status = child
            .wait()
            .context("failed to wait for plocate")
            .map_err(AppError::Application)?;
        let mut results = matches?;
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
        let truncated = results.len() > limit;
        results.truncate(limit);
        Ok(AssetSearchResults { results, truncated })
    }

    /// Whether a database exists to query. updatedb installs it with an
    /// atomic rename, so once present it never disappears between builds.
    #[must_use]
    pub fn is_built(&self) -> bool {
        self.database_path.is_file()
    }
}

/// Count only existing entries with matching relative paths toward the limit.
/// Ignore the floating latest alias, including entries from older indexes.
fn read_matches(
    mut reader: impl BufRead,
    asset_root: &Path,
    query: &str,
    limit: usize,
) -> AppResult<Vec<AssetEntry>> {
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
        let path = Path::new(candidate);
        let Ok(relative) = path.strip_prefix(asset_root) else {
            continue;
        };
        if relative.starts_with("gamedata/latest")
            || !relative.to_str().is_some_and(|path| path.contains(query))
        {
            continue;
        }
        match AssetEntry::new(path, asset_root) {
            Ok(entry) => kept.push(entry),
            Err(err) => debug!(path = candidate, error = %err, "skipping unavailable asset"),
        }
        if kept.len() >= limit {
            return Ok(kept);
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

/// Escape glob syntax for literal substring matching. plocate cannot reliably
/// match a literal backslash, so use the longest segment as a candidate query;
/// `read_matches` always checks the complete literal query on relative paths.
fn search_pattern(query: &str) -> String {
    let query = query
        .split('\\')
        .max_by_key(|part| part.len())
        .unwrap_or("");
    if query.is_empty() {
        return "*".to_string();
    }
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

    struct Fixture {
        dir: PathBuf,
        index: PlocateIndex,
    }

    impl Fixture {
        fn new() -> Self {
            let dir = std::env::temp_dir().join(format!(
                "ak-plocate-{}-{}",
                std::process::id(),
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_nanos()
            ));
            let root = dir.join("assets");
            std::fs::create_dir_all(&root).unwrap();
            let index = PlocateIndex {
                asset_root: root,
                database_path: dir.join("plocate.db"),
            };
            Self { dir, index }
        }

        fn write(&self, path: &str) {
            let path = self.index.asset_root.join(path);
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(path, b"asset").unwrap();
        }

        fn paths(&self, query: &str) -> Vec<String> {
            let result = self.index.search(query, None).unwrap();
            assert!(!result.truncated);
            let mut paths: Vec<_> = result.results.into_iter().map(|entry| entry.path).collect();
            paths.sort();
            paths
        }
    }

    impl Drop for Fixture {
        fn drop(&mut self) {
            std::fs::remove_dir_all(&self.dir).unwrap();
        }
    }

    #[test]
    fn search_rejects_invalid_input_before_querying() {
        let fixture = Fixture::new();
        for (query, limit) in [
            ("", None),
            ("a\0b", None),
            ("file", Some(0)),
            ("file", Some(201)),
        ] {
            assert!(matches!(
                fixture.index.search(query, limit),
                Err(AppError::InvalidInput(_))
            ));
        }
        assert!(matches!(
            fixture.index.search("file", None),
            Err(AppError::Unavailable(_))
        ));
    }

    #[test]
    fn only_valid_relative_matches_consume_the_limit() {
        let fixture = Fixture::new();
        fixture.write("raw/set/deep/valid.png");
        fixture.write("raw/set/deeper/other.png");
        fixture.write("raw/unrelated.png");
        fixture.write("gamedata/latest/set.json");
        // Simulate index entries that have disappeared, live only under latest,
        // or match "set" only in the absolute /assets prefix.
        let candidates = [
            fixture.dir.join("outside-set.json"),
            fixture.index.asset_root.join("raw/unrelated.png"),
            fixture.index.asset_root.join("gamedata/latest/set.json"),
            fixture.index.asset_root.join("raw/set/deleted.png"),
            fixture.index.asset_root.join("raw/set/deep/valid.png"),
            fixture.index.asset_root.join("raw/set/deeper/other.png"),
        ];
        let mut stream = Vec::new();
        for path in candidates {
            stream.extend_from_slice(path.to_str().unwrap().as_bytes());
            stream.push(0);
        }
        let entries = read_matches(stream.as_slice(), &fixture.index.asset_root, "set", 2).unwrap();
        assert_eq!(
            entries
                .iter()
                .map(|entry| entry.path.as_str())
                .collect::<Vec<_>>(),
            vec!["raw/set/deep/valid.png", "raw/set/deeper/other.png"]
        );
    }

    #[test]
    fn search_is_literal_relative_and_unrestricted_by_depth() {
        if !binaries_available_for_tests() {
            return;
        }
        let fixture = Fixture::new();
        for path in [
            "raw/amiya/deep/sub/portrait[1].png",
            "raw/star*name.png",
            "raw/question?mark.png",
            "raw/plain.png",
            "raw/back\\slash.png",
        ] {
            fixture.write(path);
        }
        fixture.index.update().unwrap();
        assert!(
            fixture.paths("assets").is_empty(),
            "absolute root must not match"
        );
        assert!(
            fixture.paths("AMIYA").is_empty(),
            "matching is case sensitive"
        );
        assert_eq!(
            fixture.paths("portrait[1]"),
            vec!["raw/amiya/deep/sub/portrait[1].png"]
        );
        assert_eq!(fixture.paths("star*"), vec!["raw/star*name.png"]);
        assert_eq!(fixture.paths("question?"), vec!["raw/question?mark.png"]);
        assert_eq!(fixture.paths("back\\slash"), vec!["raw/back\\slash.png"]);
        assert_eq!(fixture.paths("\\"), vec!["raw/back\\slash.png"]);
        assert_eq!(fixture.paths("*"), vec!["raw/star*name.png"]);
        assert!(
            fixture
                .paths("amiya")
                .contains(&"raw/amiya/deep/sub/portrait[1].png".to_string())
        );
        assert!(!fixture.index.asset_root.join(".catalog").exists());
    }

    #[test]
    fn search_excludes_latest_but_keeps_versioned_paths_and_file_symlinks() {
        if !binaries_available_for_tests() {
            return;
        }
        let fixture = Fixture::new();
        fixture.write("gamedata/v1/deep/table.json");
        fixture.write("gamedata/latest_backup/table.json");
        fixture.write("raw/audio/source.mp3");
        std::os::unix::fs::symlink("v1", fixture.index.asset_root.join("gamedata/latest")).unwrap();
        std::os::unix::fs::symlink(
            "source.mp3",
            fixture.index.asset_root.join("raw/audio/alias.mp3"),
        )
        .unwrap();
        fixture.index.update().unwrap();
        assert_eq!(
            fixture.paths("latest"),
            vec![
                "gamedata/latest_backup",
                "gamedata/latest_backup/table.json"
            ]
        );
        assert!(fixture.paths("latest/deep").is_empty());
        assert_eq!(
            fixture.paths("table.json"),
            vec![
                "gamedata/latest_backup/table.json",
                "gamedata/v1/deep/table.json"
            ]
        );
        assert_eq!(fixture.paths("alias.mp3"), vec!["raw/audio/alias.mp3"]);
    }

    #[test]
    fn truncation_uses_one_extra_existing_entry() {
        if !binaries_available_for_tests() {
            return;
        }
        let fixture = Fixture::new();
        fixture.write("raw/00-match.png");
        fixture.write("raw/01-match.png");
        fixture.write("raw/02-match.png");
        fixture.index.update().unwrap();
        let first = fixture.index.search("match", Some(2)).unwrap();
        assert_eq!(first.results.len(), 2);
        assert!(first.truncated);
        std::fs::remove_file(fixture.index.asset_root.join("raw/00-match.png")).unwrap();
        let exact = fixture.index.search("match", Some(2)).unwrap();
        assert_eq!(exact.results.len(), 2);
        assert!(
            !exact.truncated,
            "deleted entries do not count as more results"
        );
        let json = serde_json::to_value(exact).unwrap();
        assert!(json.get("total").is_none());
        assert!(fixture.paths("no-such-file").is_empty());
    }

    #[test]
    fn default_and_maximum_result_limits() {
        if !binaries_available_for_tests() {
            return;
        }
        let fixture = Fixture::new();
        for i in 0..201 {
            fixture.write(&format!("raw/match-{i:03}.png"));
        }
        fixture.index.update().unwrap();
        let default = fixture.index.search("match", None).unwrap();
        assert_eq!(default.results.len(), 100);
        assert!(default.truncated);
        let maximum = fixture.index.search("match", Some(200)).unwrap();
        assert_eq!(maximum.results.len(), 200);
        assert!(maximum.truncated);
    }

    #[test]
    fn database_must_be_outside_the_asset_tree() {
        if !binaries_available_for_tests() {
            return;
        }
        let fixture = Fixture::new();
        let config = PlocateConfig {
            database_path: Some(
                fixture
                    .index
                    .asset_root
                    .join("index.db")
                    .to_str()
                    .unwrap()
                    .to_string(),
            ),
            ..PlocateConfig::default()
        };
        assert!(PlocateIndex::new(&fixture.index.asset_root, &config).is_err());
    }

    #[test]
    fn index_refreshes_and_keeps_previous_database_on_failure() {
        if !binaries_available_for_tests() {
            return;
        }
        let fixture = Fixture::new();
        let config = PlocateConfig {
            database_path: Some(fixture.dir.join("index.db").to_str().unwrap().to_string()),
            ..PlocateConfig::default()
        };
        let index = PlocateIndex::new(&fixture.index.asset_root, &config).unwrap();
        fixture.write("raw/one.json");
        index.update().unwrap();
        fixture.write("raw/two.json");
        index.update().unwrap();
        assert_eq!(index.search("two.json", None).unwrap().results.len(), 1);
        assert!(index.search("index.db", None).unwrap().results.is_empty());
        // A failed replacement must leave the prior database searchable.
        let mut broken = index.clone();
        broken.asset_root = fixture.dir.join("missing");
        assert!(broken.update().is_err());
        assert_eq!(index.search("two.json", None).unwrap().results.len(), 1);
    }
}
