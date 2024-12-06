use serde::Deserialize;
use utoipa::IntoParams;

#[derive(IntoParams, Deserialize, Debug)]
pub struct Filter {
    pub path: Option<String>,
    pub hash: Option<String>,
    pub file: Option<i32>,
    pub version: Option<i32>,
}
