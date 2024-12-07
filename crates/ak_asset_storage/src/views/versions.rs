use sea_orm::FromQueryResult;
use serde::Serialize;
use utoipa::ToSchema;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, ToSchema, FromQueryResult)]
#[serde(rename_all = "camelCase")]
pub struct VersionListItem {
    pub id: i32,
    pub res: String,
    pub client: String,
    pub is_ready: bool,
}
