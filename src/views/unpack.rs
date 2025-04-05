use crate::utils::time::{serialize_option_primitive_date_time, serialize_primitive_date_time};
use axum::extract::{FromRequestParts, Query};
use serde::{Deserialize, Serialize};
use time::PrimitiveDateTime;
use utoipa::{IntoParams, ToSchema};

#[derive(Debug, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CreateUnpackVersionReq {
    pub bundle_version: i32,
}

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CreateUnpackVersionResp {
    pub version: i32,
}

#[derive(IntoParams, Deserialize, Debug, FromRequestParts)]
#[into_params(parameter_in = Query)]
#[from_request(via(Query))]
#[serde(rename_all = "camelCase")]
pub struct SearchUnpackVersionReq {
    #[serde(serialize_with = "serialize_option_primitive_date_time")]
    #[param(value_type = Option<String>)]
    pub start_time: Option<PrimitiveDateTime>,
    #[serde(serialize_with = "serialize_option_primitive_date_time")]
    #[param(value_type = Option<String>)]
    pub end_time: Option<PrimitiveDateTime>,
    pub bundle_version: Option<i32>,
    pub token: Option<i32>,
}

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct UnpackVersionDto {
    pub id: i32,
    pub bundle_version: i32,
    pub token: i32,
    #[serde(serialize_with = "serialize_primitive_date_time")]
    #[schema(value_type = String)]
    pub start_time: PrimitiveDateTime,
    #[serde(serialize_with = "serialize_option_primitive_date_time")]
    #[schema(value_type = String)]
    pub end_time: Option<PrimitiveDateTime>,
}

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct UnpackVersionDetailDto {
    pub id: i32,
    pub bundle_version: i32,
    pub token: i32,
    pub token_name: String,
    pub res: String,
    pub client: String,
    #[serde(serialize_with = "serialize_primitive_date_time")]
    #[schema(value_type = String)]
    pub start_time: PrimitiveDateTime,
    #[serde(serialize_with = "serialize_option_primitive_date_time")]
    #[schema(value_type = String)]
    pub end_time: Option<PrimitiveDateTime>,
}
#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct AssetsDto {
    pub id: i32,
    pub path: String,
    pub unpack_version: i32,
    pub file: i32,
    pub hash: String,
    pub size: i32,
}

#[derive(Debug, Deserialize, IntoParams, FromRequestParts)]
#[from_request(via(Query))]
#[into_params(parameter_in = Query)]
pub struct AssetsSearch {
    pub path: Option<String>,
    pub hash: Option<String>,
}
