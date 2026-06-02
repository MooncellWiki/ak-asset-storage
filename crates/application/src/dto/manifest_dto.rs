use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ManifestNodeDto {
    pub name: String,
    pub path: String,
    pub node_type: String,
}

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct AssetMappingDetailDto {
    pub asset_name: String,
    pub bundle_path: String,
    pub asset_path: Option<String>,
    pub short_name: Option<String>,
    pub bundle_size: Option<i32>,
    pub bundle_hash: Option<String>,
}

#[derive(Debug, Deserialize, IntoParams)]
pub struct ManifestChildrenParams {
    pub dir: Option<String>,
}

#[derive(Debug, Deserialize, IntoParams)]
pub struct ManifestSearchParams {
    pub q: String,
}

#[derive(Debug, Deserialize, IntoParams)]
pub struct ManifestDetailParams {
    pub asset_name: String,
}
