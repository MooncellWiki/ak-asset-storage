use axum::extract::{FromRequestParts, Query};
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct BundleDetailsDto {
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

#[derive(Debug, Deserialize, IntoParams, FromRequestParts)]
#[from_request(via(Query))]
#[into_params(parameter_in = Query)]
pub struct BundleFilterDto {
    pub path: Option<String>,
    pub hash: Option<String>,
    pub file: Option<i32>,
    pub version: Option<i32>,
}
