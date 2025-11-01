use ak_asset_storage_application::{AppResult, AssetDirInfo, AssetEntry, TorappuAssetService};
use anyhow::Context;
use async_trait::async_trait;
use std::{path::PathBuf, str::from_utf8};
use walkdir::WalkDir;

#[derive(Debug, Clone)]
pub struct TorappuAssetClient {
    pub asset_base_path: PathBuf,
}

#[async_trait]
impl TorappuAssetService for TorappuAssetClient {
    async fn list_asset(&self, path: &str) -> AppResult<AssetDirInfo> {
        let target_path = self.asset_base_path.join(path);
        let mut children = vec![];
        let entries = std::fs::read_dir(&target_path).context("Failed to read directory")?;
        for entry in entries {
            let entry = entry.context("Failed to read directory entry")?;
            let asset_entry = AssetEntry::new(&entry.path(), &self.asset_base_path)?;
            children.push(asset_entry);
        }
        children.sort_by(|a, b| a.name.cmp(&b.name));
        let dir = AssetEntry::new(&target_path, &self.asset_base_path)?;
        Ok(AssetDirInfo { dir, children })
    }

    async fn search_assets_by_path(&self, query: &str) -> AppResult<Vec<AssetEntry>> {
        let mut result = vec![];
        for entry in WalkDir::new(self.asset_base_path.clone())
            .follow_links(true)
            .into_iter()
            .filter_map(std::result::Result::ok)
        {
            let path = entry.path().to_string_lossy();

            if let Some(pos) = path.find(query) {
                let mut count = 0;
                for c in from_utf8(&path.as_bytes()[pos + query.len()..])
                    .context("Failed to convert path to UTF-8")?
                    .chars()
                {
                    if c == '/' {
                        count += 1;
                    }
                    if count == 2 {
                        break;
                    }
                }
                if count == 2 {
                    continue;
                }
                let asset_entry = AssetEntry::new(entry.path(), &self.asset_base_path)?;
                result.push(asset_entry);
            }
        }
        Ok(result)
    }
}
