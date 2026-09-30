use crate::AppError;
use crate::{
    AppResult,
    external::types::{AssetDirInfo, AssetEntry},
};
use anyhow::Context;
use std::path::{Component, Path, PathBuf};

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
        };

        let listing = client.list_asset("raw").expect("in-root listing works");
        assert_eq!(listing.dir.path, "raw");

        let err = client.list_asset("../secret").unwrap_err();
        assert!(matches!(err, AppError::InvalidInput(_)), "{err:?}");
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
