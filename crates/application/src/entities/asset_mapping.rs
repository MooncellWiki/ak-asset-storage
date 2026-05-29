/// Maps entries from `resource_manifest_idx.json`:
///
/// ```json
/// {
///   "bundles": [{ "name": "..." }, ...],
///   "assetToBundleList": [{ "assetName": "...", "bundleIndex": N, "name": "...", "path": "..." }, ...]
/// }
/// ```
///
/// Field mapping:
/// - `asset_name`  ← `assetToBundleList[].assetName`
/// - `bundle_path` ← `bundles[assetToBundleList[].bundleIndex].name`
/// - `asset_path`  ← `assetToBundleList[].path`      (may be null → `None`)
/// - `short_name`  ← `assetToBundleList[].name`       (may be null → `None`)
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AssetMapping {
    pub id: Option<i32>,
    pub version_id: i32,
    pub asset_name: String,
    pub bundle_path: String,
    pub asset_path: Option<String>,
    pub short_name: Option<String>,
    pub dir_name: String,
    pub node_type: NodeType,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NodeType {
    File,
    Directory,
    Both,
}

impl NodeType {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::File => "file",
            Self::Directory => "directory",
            Self::Both => "both",
        }
    }

    #[must_use]
    pub const fn is_directory(self) -> bool {
        matches!(self, Self::Directory | Self::Both)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AssetMappingStatus {
    Pending,
    Importing,
    Ready,
}

impl AssetMappingStatus {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Importing => "importing",
            Self::Ready => "ready",
        }
    }

    #[must_use]
    pub fn from_str_lossy(s: &str) -> Self {
        match s {
            "importing" => Self::Importing,
            "ready" => Self::Ready,
            _ => Self::Pending,
        }
    }
}
