use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateFileReq {
    #[schema(content_media_type = "application/octet-stream")]
    pub file: Vec<u8>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct CreateFileResp {
    pub id: i32,
}
