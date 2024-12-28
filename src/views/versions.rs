use axum::extract::{FromRequestParts, Query};
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct VersionListItem {
    pub id: i32,
    pub res: String,
    pub client: String,
    pub is_ready: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct VersionDetail {
    pub id: i32,
    pub res: String,
    pub client: String,
    pub is_ready: bool,
    pub hot_update_list: String,
}

#[derive(Debug, Deserialize, IntoParams, FromRequestParts)]
#[from_request(via(Query))]
#[into_params(parameter_in = Query)]
pub struct LatestFlag {
    pub ready: bool,
}
