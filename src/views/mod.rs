use sea_orm::FromQueryResult;
use serde::Serialize;

pub mod utils;

#[derive(Debug, Clone, Serialize, utoipa::ToSchema, FromQueryResult)]
pub struct FileDetail {
    path: String,

    file: i32,
    hash: String,
    size: i32,

    version: i32,
    res: String,
    client: String,
    is_ready: bool,
}
