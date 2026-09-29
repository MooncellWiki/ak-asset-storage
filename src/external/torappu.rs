use crate::AppError;
use crate::{
    AppResult,
    external::{
        plocate::PlocateIndex,
        types::{AssetDirInfo, AssetEntry},
    },
};
use anyhow::Context;
use std::{
    path::{Component, Path, PathBuf},
    str::from_utf8,
};
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
            return Self::search_via_plocate(index, query);
        }
        self.search_by_walking(query)
    }

    /// Results come from the plocate snapshot; entries that no longer exist
    /// on disk (deleted after the last build) are skipped.
    fn search_via_plocate(index: &PlocateIndex, query: &str) -> AppResult<Vec<AssetEntry>> {
        let mut result = Vec::new();
        for path in index.lookup(query)? {
            if !within_one_directory(&path.to_string_lossy(), query) {
                continue;
            }
            match AssetEntry::new(&path, index.asset_root()) {
                Ok(entry) => result.push(entry),
                Err(err) => {
                    debug!(path = %path.display(), error = %err, "skipping plocate match");
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
            if from_utf8(path.as_bytes()).is_err() {
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
