use axum::extract::{FromRequestParts, Query};
use serde::Deserialize;
use utoipa::IntoParams;

#[derive(IntoParams, Deserialize, FromRequestParts, Debug)]
#[from_request(via(Query))]
#[into_params(parameter_in = Query)]
pub struct Filter {
    pub path: Option<String>,
    pub hash: Option<String>,
    pub file: Option<i32>,
    pub version: Option<i32>,
}
