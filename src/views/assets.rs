use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct AssetsDetail {
    pub id: i32,
    pub path: String,
    pub unpack_version: i32,
    pub file: i32,
    pub hash: String,
    pub bundle: i32,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct ReuseAssetFromBundleReq {
    pub bundle_id: i32,
    pub old_unpack_version: i32,
    pub new_unpack_version: i32,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateAssetReq {
    pub file: i32,
    pub path: String,
    pub unpack_version: i32,
}
