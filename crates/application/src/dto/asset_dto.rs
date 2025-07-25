use anyhow::Context;
use chrono::{DateTime, Utc};
use serde::Serialize;
use std::path::Path;
use utoipa::ToSchema;

use crate::AppResult;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, ToSchema)]
pub struct AssetEntry {
    pub name: String,
    pub path: String,
    pub size: u64,
    pub create_at: String,
    pub modified_at: String,
    pub is_dir: bool,
}
impl AssetEntry {
    pub fn new(target_path: &Path, base_path: &Path) -> AppResult<Self> {
        let meta = std::fs::metadata(target_path).context("Failed to retrieve metadata")?;
        let name = target_path
            .file_name()
            .and_then(|n| n.to_str())
            .context("Failed to convert file name to string")?
            .to_string();

        let create_at: DateTime<Utc> = DateTime::from(meta.created()?);
        let modified_at: DateTime<Utc> = DateTime::from(meta.modified()?);

        Ok(Self {
            name,
            path: target_path
                .strip_prefix(base_path)
                .context("Failed to strip base path")?
                .to_str()
                .context("Failed to convert target path to string")?
                .to_string(),
            size: meta.len(),
            is_dir: meta.is_dir(),
            create_at: create_at.to_rfc3339(),
            modified_at: modified_at.to_rfc3339(),
        })
    }
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct AssetDirInfo {
    pub dir: AssetEntry,
    pub children: Vec<AssetEntry>,
}
