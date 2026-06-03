#[derive(Debug, Clone)]
pub struct VersionRow {
    pub id: Option<i32>,
    pub res: String,
    pub client: String,
    pub is_ready: bool,
    pub asset_mapping_status: AssetMappingStatus,
    pub hot_update_list: String,
}

#[derive(Debug, Clone)]
pub struct BundleRow {
    pub id: Option<i32>,
    pub path: String,
    pub version_id: i32,
    pub file_id: i32,
}

#[derive(Debug, Clone)]
pub struct FileRow {
    pub id: Option<i32>,
    pub hash: String,
    pub size: i32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AssetMappingRow {
    pub id: Option<i32>,
    pub version_id: i32,
    pub asset_name: String,
    pub bundle_path: String,
    pub asset_path: Option<String>,
    pub short_name: Option<String>,
    pub dir_name: String,
    pub node_type: NodeType,
}

#[derive(sqlx::Type, Debug, Clone, Copy, PartialEq, Eq)]
#[sqlx(type_name = "node_type")]
pub enum NodeType {
    File,
    Directory,
    Both,
}

#[derive(sqlx::Type, Debug, Clone, Copy, PartialEq, Eq)]
#[sqlx(type_name = "asset_mapping_status")]
pub enum AssetMappingStatus {
    Pending,
    Importing,
    Ready,
}
