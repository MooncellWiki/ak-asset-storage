use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct VersionDto {
    pub id: i32,
    pub client_version: String,
    pub res_version: String,
    pub is_ready: bool,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct VersionDetailDto {
    pub id: i32,
    pub client_version: String,
    pub res_version: String,
    pub is_ready: bool,
    pub hot_update_list: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RemoteVersion {
    pub client_version: String,
    pub res_version: String,
}
