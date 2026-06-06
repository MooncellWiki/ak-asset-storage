use serde::Serialize;
use utoipa::ToSchema;

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct VersionSummary {
    pub id: i32,
    pub client_version: String,
    pub res_version: String,
    pub is_ready: bool,
    pub asset_mapping_status: String,
}

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct VersionDetails {
    pub id: i32,
    pub client_version: String,
    pub res_version: String,
    pub is_ready: bool,
    pub hot_update_list: String,
}

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct BundleDetails {
    pub id: i32,
    pub path: String,
    pub file_id: i32,
    pub file_hash: String,
    pub file_size: i32,
    pub version_id: i32,
    pub version_res: String,
    pub version_client: String,
    pub version_is_ready: bool,
}

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ManifestNode {
    pub name: String,
    pub path: String,
    pub node_type: String,
}

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct AssetMappingDetails {
    pub asset_name: String,
    pub bundle_path: String,
    pub asset_path: Option<String>,
    pub short_name: Option<String>,
    pub bundle_size: Option<i32>,
    pub bundle_hash: Option<String>,
}
