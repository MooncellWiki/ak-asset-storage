use serde::Serialize;

#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct FileDetail {
    pub path: String,

    pub file: i32,
    pub hash: String,
    pub size: i32,

    pub version: i32,
    pub res: String,
    pub client: String,
    pub is_ready: bool,
}
