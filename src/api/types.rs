use crate::{
    api::utils::escape_like,
    database::{bundle::BundleFilter, row::StoryResourceType},
};
use serde::Deserialize;
use utoipa::{IntoParams, ToSchema};

#[derive(Debug, Deserialize, IntoParams)]
pub struct StoryResourceUsageQuery {
    /// Resource type: `background`, `image`, `item` or `character`.
    #[serde(rename = "type")]
    pub resource_type: StoryResourceType,
    /// Percent-encoded exact resource id (may contain `/`, `#`, `$`).
    /// Characters accept two forms: `base#expression` matches the resolved
    /// `character.json` entry, while `base$body` (no `#`) matches scripts
    /// using any expression of the body (face-overlay characters only).
    pub id: String,
    /// Page size, defaults to 50, at most 200.
    pub limit: Option<u32>,
    /// Opaque cursor from a previous response (`nextCursor`).
    pub cursor: Option<String>,
}

#[derive(Debug, Deserialize, IntoParams)]
pub struct StoryResourceListQuery {
    /// Restrict the listing to one resource type: `background`, `image`,
    /// `item` or `character`.
    #[serde(rename = "type")]
    pub resource_type: Option<StoryResourceType>,
    /// Case-insensitive substring filter on the listing id. Face-overlay
    /// characters are listed at body granularity (`base$body`); standalone
    /// full-image characters keep their resolved expression ids.
    pub q: Option<String>,
    /// Page size, defaults to 50, at most 200.
    pub limit: Option<u32>,
    /// Opaque cursor from a previous response (`nextCursor`).
    pub cursor: Option<String>,
}

#[derive(Debug, serde::Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct StoryResourceSummary {
    #[serde(rename = "type")]
    pub resource_type: StoryResourceType,
    /// Face-overlay characters use the body form `base$body`; standalone
    /// full-image characters use `base#expression`, and other types use their
    /// normalized image key.
    pub id: String,
    pub script_count: i64,
}

#[derive(Debug, serde::Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct StoryResourceUsageItem {
    pub script_path: String,
    pub display_names: Vec<String>,
    /// Resolved character expression ids (`base#expression`) this script uses.
    /// Present only for character body queries (`base$body` id form).
    pub faces: Option<Vec<String>>,
}

#[derive(Debug, serde::Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct StoryResourceListResponse {
    pub resources: Vec<StoryResourceSummary>,
    pub next_cursor: Option<String>,
}

#[derive(Debug, serde::Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct StoryResourceUsageResponse {
    pub usages: Vec<StoryResourceUsageItem>,
    pub next_cursor: Option<String>,
}

#[derive(Debug, Deserialize, IntoParams)]
pub struct AssetSearchQuery {
    /// Case-sensitive literal substring of the relative asset path.
    pub path: String,
    /// Maximum entries returned (1-200, default 100). No pagination.
    pub limit: Option<u32>,
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
    /// Bundle path substring (matched literally). Blank counts as absent.
    pub path: Option<String>,
    /// Exact file hash. Blank counts as absent.
    pub hash: Option<String>,
    pub file: Option<i32>,
    pub version: Option<i32>,
    /// Page size, defaults to 50, at most 200.
    pub limit: Option<u32>,
    /// Opaque cursor from a previous response (`nextCursor`).
    pub cursor: Option<String>,
}

impl BundleListQuery {
    /// Blank filters carry no information and would match everything, so
    /// they count as absent (surrounding whitespace is trimmed); at least
    /// one of path/hash/file/version must remain or the request would dump
    /// the whole table.
    pub(crate) fn normalized(&self) -> BundleFilter {
        BundleFilter {
            path: self
                .path
                .as_deref()
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(escape_like),
            hash: self
                .hash
                .as_deref()
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(str::to_string),
            file: self.file,
            version: self.version,
        }
    }

    pub(crate) fn has_condition(&self) -> bool {
        self.path
            .as_deref()
            .is_some_and(|value| !value.trim().is_empty())
            || self
                .hash
                .as_deref()
                .is_some_and(|value| !value.trim().is_empty())
            || self.file.is_some()
            || self.version.is_some()
    }
}

#[derive(Debug, serde::Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct BundleListResponse {
    pub bundles: Vec<crate::database::model::BundleDetails>,
    pub next_cursor: Option<String>,
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
    pub include: Option<String>,
    pub exclude: Option<String>,
}

#[derive(Debug, serde::Serialize, ToSchema)]
pub struct DockerLaunchResponse {
    pub container_name: String,
    pub status: String,
}
