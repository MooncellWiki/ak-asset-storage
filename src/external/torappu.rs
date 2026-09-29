use crate::AppError;
use crate::{
    AppResult,
    external::{
        plocate::PlocateIndex,
        types::{AssetDirInfo, AssetEntry},
    },
};
use anyhow::Context;
use std::path::{Component, Path, PathBuf};
use tracing::debug;
use walkdir::WalkDir;

/// Validates a caller-supplied path as a plain relative path below the asset root.
///
/// `Path::join` does not normalize, so `..` segments (which arrive
/// percent-encoded as one route segment) or an absolute path would otherwise
/// escape `asset_base_path`.
pub fn relative_asset_path(path: &str) -> AppResult<&Path> {
    if path.contains('\0') {
        return Err(AppError::InvalidInput(
            "path must not contain NUL characters".to_string(),
        ));
    }
    let candidate = Path::new(path);
    let is_plain = candidate
        .components()
        .all(|component| matches!(component, Component::Normal(_) | Component::CurDir));
    if !is_plain {
        return Err(AppError::InvalidInput(
            "path must be relative and must not contain '..'".to_string(),
        ));
    }
    Ok(candidate)
}

#[derive(Debug, Clone)]
pub struct TorappuClient {
    pub asset_base_path: PathBuf,
    /// plocate-backed search index. `None` keeps the legacy behavior of
    /// walking the whole tree per search request.
    pub plocate: Option<PlocateIndex>,
}

/// Search filter shared by both search backends.
///
/// The match must sit in (or one directory above) the final path component,
/// so `avg_npc` matches `.../avg_npc/1.png` but not
/// `.../avg_npc/sub/deep/1.png`. Kept from the original walk-based search
/// so result sets stay comparable.
#[must_use]
pub fn within_one_directory(path: &str, query: &str) -> bool {
    let Some(pos) = path.find(query) else {
        return false;
    };
    path[pos + query.len()..].matches('/').count() < 2
}

/// The `gamedata/latest` symlink points at the current game data version.
/// updatedb does not follow symlinks, so searches naming the alias get a
/// second lookup leg against the resolved target with results mapped back
/// to the alias spelling — matching the previous walk-based behavior.
const GAMEDATA_LATEST: &str = "gamedata/latest";

/// Resolves `<asset_root>/gamedata/latest` to its canonical absolute target
/// and the target's path relative to the asset root. Returns `None` when
/// the alias is absent or escapes the root.
fn resolve_gamedata_latest(asset_root: &Path) -> Option<(PathBuf, String)> {
    let link = asset_root.join(GAMEDATA_LATEST);
    let target = std::fs::read_link(&link).ok()?;
    let resolved = if target.is_absolute() {
        target
    } else {
        match link.parent() {
            Some(parent) => parent.join(target),
            None => target,
        }
    };
    let resolved = resolved.canonicalize().ok()?;
    let relative = resolved
        .strip_prefix(asset_root)
        .ok()?
        .to_str()?
        .to_string();
    Some((resolved, relative))
}

/// Rewrites `latest` path segments in the query to the alias target's final
/// segment, e.g. `latest/character` becomes `24-07/character`. Segments that
/// merely contain "latest" as a substring are left alone.
fn replace_latest_segments(query: &str, replacement: &str) -> Option<String> {
    let mut rewritten = String::with_capacity(query.len());
    let mut changed = false;
    for (idx, segment) in query.split('/').enumerate() {
        if idx > 0 {
            rewritten.push('/');
        }
        if segment == "latest" {
            rewritten.push_str(replacement);
            changed = true;
        } else {
            rewritten.push_str(segment);
        }
    }
    changed.then_some(rewritten)
}

impl TorappuClient {
    pub fn list_asset(&self, path: &str) -> AppResult<AssetDirInfo> {
        let target_path = self.asset_base_path.join(relative_asset_path(path)?);
        let mut children = Vec::new();
        // A missing/non-directory path is caller input to reject, not an
        // internal failure — the message must guide the caller (model or
        // frontend) to a directory that exists.
        let entries = match std::fs::read_dir(&target_path) {
            Ok(entries) => entries,
            Err(err)
                if matches!(
                    err.kind(),
                    std::io::ErrorKind::NotFound | std::io::ErrorKind::NotADirectory
                ) =>
            {
                return Err(AppError::InvalidInput(format!(
                    "no directory at {path:?}; list its parent to see valid names"
                )));
            }
            Err(err) => return Err(AppError::Application(err.into())),
        };
        for entry in entries {
            let entry = entry.context("Failed to read directory entry")?;
            children.push(AssetEntry::new(&entry.path(), &self.asset_base_path)?);
        }
        children.sort_by(|left, right| left.name.cmp(&right.name));
        let dir = AssetEntry::new(&target_path, &self.asset_base_path)?;
        Ok(AssetDirInfo { dir, children })
    }

    pub fn search_assets_by_path(&self, query: &str) -> AppResult<Vec<AssetEntry>> {
        if query.is_empty() {
            return Err(AppError::InvalidInput(
                "search path must not be empty; browse the directory tree instead".to_string(),
            ));
        }
        if query.contains('\0') {
            return Err(AppError::InvalidInput(
                "search path must not contain NUL characters".to_string(),
            ));
        }
        if let Some(index) = &self.plocate {
            if index.is_built() {
                return Self::search_via_plocate(index, query);
            }
            // Until the first build lands (or while builds keep failing),
            // keep search working through the slow walk instead of failing
            // every request.
            debug!("plocate index not built yet; falling back to walking the tree");
        }
        self.search_by_walking(query)
    }

    /// Results come from the plocate snapshot; entries that no longer exist
    /// on disk (deleted after the last build) are skipped. The entry limit is
    /// applied to the filtered result set, and queries naming the
    /// `gamedata/latest` alias additionally search through the resolved
    /// target.
    fn search_via_plocate(index: &PlocateIndex, query: &str) -> AppResult<Vec<AssetEntry>> {
        let limit = index.search_limit();
        let canonical = index.lookup_filtered(query, limit, &|candidate| {
            within_one_directory(candidate, query)
        })?;
        let mut result = Vec::new();
        for path in canonical {
            match AssetEntry::new(&path, index.asset_root()) {
                Ok(entry) => result.push(entry),
                Err(err) => {
                    debug!(path = %path.display(), error = %err, "skipping plocate match");
                }
            }
        }

        // Alias leg: the index stores only canonical version paths, so a
        // query naming `latest` is retried against the resolved target and
        // hits are mapped back to the alias spelling.
        if query.split('/').any(|segment| segment == "latest") {
            let Some((target, target_relative)) = resolve_gamedata_latest(index.asset_root())
            else {
                debug!("query names gamedata/latest but the alias does not resolve");
                return Ok(result);
            };
            let Some(version_segment) = target_relative.rsplit('/').next() else {
                return Ok(result);
            };
            let Some(rewritten) = replace_latest_segments(query, version_segment) else {
                return Ok(result);
            };
            let alias_root = format!(
                "{}/{}",
                index.asset_root().to_string_lossy(),
                GAMEDATA_LATEST
            );
            let target_str = target.to_string_lossy().into_owned();
            let remaining = limit.saturating_sub(result.len());
            if remaining == 0 {
                return Ok(result);
            }
            let remapped = index.lookup_filtered(&rewritten, remaining, &|candidate| {
                candidate
                    .strip_prefix(target_str.as_str())
                    .is_some_and(|rest| {
                        // `rest` keeps a leading '/', which the alias root
                        // already ends with.
                        let rest = rest.strip_prefix('/').unwrap_or(rest);
                        within_one_directory(&format!("{alias_root}/{rest}"), query)
                    })
            })?;
            for path in remapped {
                let Ok(rest) = path
                    .strip_prefix(&target)
                    .map(|rest| rest.to_string_lossy().into_owned())
                else {
                    continue;
                };
                // `Path::strip_prefix` drops the separator, unlike the
                // `str` version used in the keep closure above. The alias
                // target itself is skipped: the canonical leg already
                // reports it as the `gamedata/latest` symlink entry.
                if rest.is_empty() {
                    continue;
                }
                let alias_relative = format!("{GAMEDATA_LATEST}/{rest}");
                match AssetEntry::from_parts(&path, &alias_relative) {
                    Ok(entry) => result.push(entry),
                    Err(err) => {
                        debug!(path = %path.display(), error = %err, "skipping alias match");
                    }
                }
            }
        }
        Ok(result)
    }

    fn search_by_walking(&self, query: &str) -> AppResult<Vec<AssetEntry>> {
        let mut result = Vec::new();
        for entry in WalkDir::new(self.asset_base_path.clone())
            .follow_links(true)
            .into_iter()
            .filter_map(std::result::Result::ok)
        {
            let path = entry.path().to_string_lossy();
            if !within_one_directory(&path, query) {
                continue;
            }
            result.push(AssetEntry::new(entry.path(), &self.asset_base_path)?);
        }
        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn relative_asset_path_accepts_plain_relative_paths() {
        assert!(relative_asset_path("").is_ok());
        assert!(relative_asset_path("raw").is_ok());
        assert!(relative_asset_path("raw/chararts").is_ok());
        assert!(relative_asset_path("./raw").is_ok());
        assert!(relative_asset_path("dir with space/..name").is_ok());
    }

    #[test]
    fn relative_asset_path_rejects_escapes() {
        assert!(relative_asset_path("..").is_err());
        assert!(relative_asset_path("../etc").is_err());
        assert!(relative_asset_path("raw/../../etc").is_err());
        assert!(relative_asset_path("/etc").is_err());
        assert!(relative_asset_path("raw\0").is_err());
    }

    #[test]
    fn list_asset_cannot_leave_the_asset_root() {
        let root = tempdir();
        std::fs::create_dir_all(root.join("assets/raw")).unwrap();
        std::fs::create_dir_all(root.join("secret")).unwrap();
        std::fs::write(root.join("secret/flag"), b"x").unwrap();
        let client = TorappuClient {
            asset_base_path: root.join("assets"),
            plocate: None,
        };

        let listing = client.list_asset("raw").expect("in-root listing works");
        assert_eq!(listing.dir.path, "raw");

        let err = client.list_asset("../secret").unwrap_err();
        assert!(matches!(err, AppError::InvalidInput(_)), "{err:?}");
        std::fs::remove_dir_all(&root).unwrap();
    }

    fn unbuilt_plocate_client(root: &Path) -> TorappuClient {
        let config = crate::config::PlocateConfig {
            enabled: true,
            database_path: None,
            update_interval_seconds: 600,
            search_limit: 1000,
        };
        TorappuClient {
            asset_base_path: root.to_path_buf(),
            plocate: Some(PlocateIndex::new(root, &config).unwrap()),
        }
    }

    fn plocate_client(root: &Path) -> TorappuClient {
        let client = unbuilt_plocate_client(root);
        client.plocate.as_ref().unwrap().update().unwrap();
        client
    }

    fn search_paths(client: &TorappuClient, query: &str) -> Vec<String> {
        let mut paths = client
            .search_assets_by_path(query)
            .unwrap()
            .into_iter()
            .map(|entry| entry.path)
            .collect::<Vec<_>>();
        paths.sort();
        paths
    }

    #[test]
    fn plocate_search_resolves_the_gamedata_latest_alias() {
        if !crate::external::plocate::binaries_available_for_tests() {
            return;
        }
        let root = tempdir();
        std::fs::create_dir_all(root.join("gamedata/v1")).unwrap();
        std::fs::write(root.join("gamedata/v1/character_table.json"), b"x").unwrap();
        std::os::unix::fs::symlink("v1", root.join("gamedata/latest")).unwrap();
        let client = plocate_client(&root);

        // updatedb does not follow the symlink; the alias leg must still
        // return matches under the `latest` spelling.
        let found = search_paths(&client, "latest/character");
        assert_eq!(
            found,
            vec!["gamedata/latest/character_table.json".to_string()]
        );

        let found = search_paths(&client, "gamedata/latest");
        assert_eq!(
            found,
            vec![
                "gamedata/latest".to_string(),
                "gamedata/latest/character_table.json".to_string(),
            ]
        );

        // Canonical spellings keep working through the same index.
        let found = search_paths(&client, "v1/character");
        assert_eq!(found, vec!["gamedata/v1/character_table.json".to_string()]);

        std::fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn plocate_search_walks_the_tree_until_the_index_is_built() {
        if !crate::external::plocate::binaries_available_for_tests() {
            return;
        }
        let root = tempdir();
        std::fs::create_dir_all(root.join("raw")).unwrap();
        std::fs::write(root.join("raw/avg_npc_009.png"), b"x").unwrap();
        let client = unbuilt_plocate_client(&root);

        assert_eq!(
            search_paths(&client, "avg_npc_009"),
            vec!["raw/avg_npc_009.png".to_string()]
        );

        std::fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn plocate_search_treats_queries_as_literal_substrings() {
        if !crate::external::plocate::binaries_available_for_tests() {
            return;
        }
        let root = tempdir();
        std::fs::create_dir_all(root.join("raw")).unwrap();
        std::fs::write(root.join("raw/portrait[1].png"), b"x").unwrap();
        std::fs::write(root.join("raw/plain.png"), b"x").unwrap();
        let client = plocate_client(&root);

        assert_eq!(
            search_paths(&client, "portrait[1]"),
            vec!["raw/portrait[1].png".to_string()]
        );
        assert_eq!(
            search_paths(&client, "plain"),
            vec!["raw/plain.png".to_string()]
        );

        std::fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn within_one_directory_keeps_matches_close_to_the_file_name() {
        assert!(within_one_directory(
            "/base/raw/avg_npc_009/1.png",
            "avg_npc_009"
        ));
        assert!(within_one_directory(
            "/base/raw/char_100/avg_npc_009.png",
            "avg_npc_009.png"
        ));
        assert!(within_one_directory("/base/raw/avg_npc", "avg_npc"));
        assert!(!within_one_directory(
            "/base/raw/avg_npc_009/sub/deeper/1.png",
            "avg_npc_009"
        ));
        assert!(!within_one_directory("/base/raw/other.png", "avg_npc_009"));
    }

    #[test]
    fn walk_search_rejects_empty_and_nul_queries() {
        let root = tempdir();
        std::fs::create_dir_all(root.join("assets/raw")).unwrap();
        let client = TorappuClient {
            asset_base_path: root.join("assets"),
            plocate: None,
        };

        assert!(matches!(
            client.search_assets_by_path(""),
            Err(AppError::InvalidInput(_))
        ));
        assert!(matches!(
            client.search_assets_by_path("a\0b"),
            Err(AppError::InvalidInput(_))
        ));
        std::fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn walk_search_matches_within_one_directory() {
        let root = tempdir();
        std::fs::create_dir_all(root.join("assets/avg_npc_009/sub/deeper")).unwrap();
        std::fs::create_dir_all(root.join("assets/other")).unwrap();
        std::fs::write(root.join("assets/avg_npc_009/1.png"), b"x").unwrap();
        std::fs::write(root.join("assets/avg_npc_009/sub/deeper/2.png"), b"x").unwrap();
        std::fs::write(root.join("assets/other/avg_npc_009.png"), b"x").unwrap();
        let client = TorappuClient {
            asset_base_path: root.join("assets"),
            plocate: None,
        };

        let mut found = client
            .search_assets_by_path("avg_npc_009")
            .unwrap()
            .into_iter()
            .map(|entry| entry.path)
            .collect::<Vec<_>>();
        found.sort();
        assert_eq!(
            found,
            vec![
                "avg_npc_009".to_string(),
                "avg_npc_009/1.png".to_string(),
                // `sub` itself is one level below the match and is kept;
                // everything under it is filtered out.
                "avg_npc_009/sub".to_string(),
                "other/avg_npc_009.png".to_string(),
            ]
        );
        std::fs::remove_dir_all(&root).unwrap();
    }

    fn tempdir() -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "ak-asset-storage-torappu-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }
}
