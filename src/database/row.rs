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

#[derive(
    sqlx::Type,
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Hash,
    serde::Deserialize,
    serde::Serialize,
    utoipa::ToSchema,
)]
#[sqlx(type_name = "text", rename_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub enum StoryResourceType {
    Background,
    Image,
    Item,
    Character,
}

impl StoryResourceType {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Background => "background",
            Self::Image => "image",
            Self::Item => "item",
            Self::Character => "character",
        }
    }
}

/// One `story_resource_usages` row produced by the story usage importer.
#[derive(Debug, Clone, serde::Serialize)]
pub struct StoryUsageRow {
    pub script_path: String,
    pub resource_type: StoryResourceType,
    pub resource_id: String,
    /// Listing-granularity key (overlay characters use body form, full-image
    /// characters keep the resolved expression, others use the normalized
    /// image key); computed at extraction time.
    pub listing_id: String,
    pub display_names: Vec<String>,
    pub sort_order: i32,
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
#[sqlx(type_name = "node_type", rename_all = "lowercase")]
pub enum NodeType {
    File,
    Directory,
    Both,
}

#[derive(sqlx::Type, Debug, Clone, Copy, PartialEq, Eq)]
#[sqlx(type_name = "asset_mapping_status", rename_all = "lowercase")]
pub enum AssetMappingStatus {
    Pending,
    Importing,
    Ready,
}
