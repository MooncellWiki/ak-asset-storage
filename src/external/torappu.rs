use crate::{
    AppResult,
    external::types::{AssetDirInfo, AssetEntry},
};
use anyhow::Context;
use std::{path::PathBuf, str::from_utf8};
use walkdir::WalkDir;

#[derive(Debug, Clone)]
pub struct TorappuClient {
    pub asset_base_path: PathBuf,
}

impl TorappuClient {
    pub fn list_asset(&self, path: &str) -> AppResult<AssetDirInfo> {
        let target_path = self.asset_base_path.join(path);
        let mut children = Vec::new();
        let entries = std::fs::read_dir(&target_path).context("Failed to read directory")?;
        for entry in entries {
            let entry = entry.context("Failed to read directory entry")?;
            children.push(AssetEntry::new(&entry.path(), &self.asset_base_path)?);
        }
        children.sort_by(|left, right| left.name.cmp(&right.name));
        let dir = AssetEntry::new(&target_path, &self.asset_base_path)?;
        Ok(AssetDirInfo { dir, children })
    }

    pub fn search_assets_by_path(&self, query: &str) -> AppResult<Vec<AssetEntry>> {
        let mut result = Vec::new();
        for entry in WalkDir::new(self.asset_base_path.clone())
            .follow_links(true)
            .into_iter()
            .filter_map(std::result::Result::ok)
        {
            let path = entry.path().to_string_lossy();
            if let Some(pos) = path.find(query) {
                let mut count = 0;
                for ch in from_utf8(&path.as_bytes()[pos + query.len()..])
                    .context("Failed to convert path to UTF-8")?
                    .chars()
                {
                    if ch == '/' {
                        count += 1;
                    }
                    if count == 2 {
                        break;
                    }
                }
                if count == 2 {
                    continue;
                }
                result.push(AssetEntry::new(entry.path(), &self.asset_base_path)?);
            }
        }
        Ok(result)
    }
}
