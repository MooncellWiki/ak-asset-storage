use axum::extract::{FromRequestParts, Query};
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};

#[derive(IntoParams, Deserialize, FromRequestParts, Debug)]
#[from_request(via(Query))]
#[into_params(parameter_in = Query)]
pub struct Filter {
    pub path: Option<String>,
    pub hash: Option<String>,
    pub file: Option<i32>,
    pub version: Option<i32>,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct BundleDetail {
    pub path: String,

    pub file: i32,
    pub hash: String,
    pub size: i32,

    pub version: i32,
    pub res: String,
    pub client: String,
    pub is_ready: bool,
}
