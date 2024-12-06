use serde::Serialize;
use utoipa::ToSchema;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct VersionListItem {
    pub id: i32,
    pub res: String,
    pub client: String,
    pub is_ready: bool,
}
