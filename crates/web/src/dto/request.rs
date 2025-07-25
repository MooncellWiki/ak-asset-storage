use serde::Deserialize;
use utoipa::IntoParams;

#[derive(Debug, Clone, Deserialize, IntoParams)]
pub struct TorappuSearchAssetsByPathReq {
    pub path: String,
}
