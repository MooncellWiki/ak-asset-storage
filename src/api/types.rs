use crate::database::bundle::BundleFilter;
use serde::Deserialize;
use utoipa::{IntoParams, ToSchema};

#[derive(Debug, Deserialize, IntoParams)]
pub struct AssetSearchQuery {
    pub path: String,
}

#[derive(Debug, Deserialize, IntoParams)]
pub struct ManifestChildrenQuery {
    pub dir: Option<String>,
}

#[derive(Debug, Deserialize, IntoParams)]
pub struct ManifestSearchQuery {
    pub q: String,
}

#[derive(Debug, Deserialize, IntoParams)]
pub struct ManifestDetailQuery {
    pub asset_name: String,
}

#[derive(Debug, Deserialize, IntoParams)]
pub struct BundleListQuery {
    pub path: Option<String>,
    pub hash: Option<String>,
    pub file: Option<i32>,
    pub version: Option<i32>,
}

impl From<BundleListQuery> for BundleFilter {
    fn from(value: BundleListQuery) -> Self {
        Self {
            path: value.path,
            hash: value.hash,
            file: value.file,
            version: value.version,
        }
    }
}

#[derive(ToSchema, serde::Serialize)]
pub struct Health {
    pub ok: bool,
}

#[allow(clippy::struct_field_names)]
#[derive(Debug, Deserialize, serde::Serialize, ToSchema)]
pub struct DockerLaunchRequest {
    pub client_version: String,
    pub res_version: String,
    pub prev_client_version: String,
    pub prev_res_version: String,
}

#[derive(Debug, serde::Serialize, ToSchema)]
pub struct DockerLaunchResponse {
    pub container_name: String,
    pub status: String,
}
