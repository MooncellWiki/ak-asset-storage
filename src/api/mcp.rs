//! MCP (Model Context Protocol) server: read-only query tools exposing the
//! same data as the REST API (versions, manifest tree, bundles, story
//! resources, raw asset directory) to AI clients over Streamable HTTP at
//! `/mcp`.

use crate::{
    AppError,
    api::{
        cursor::{self, ResourceCursor, UsageCursor},
        error::WebError,
        state::AppState,
        story,
        utils::escape_like,
    },
    database::{
        bundle::BundleFilter,
        row::{AssetMappingStatus, StoryResourceType, VersionRow},
    },
};
use rmcp::{
    ErrorData, ServerHandler,
    handler::server::{router::tool::ToolRouter, wrapper::Parameters},
    model::{CallToolResult, ContentBlock, ErrorCode},
    schemars::JsonSchema,
    tool, tool_handler, tool_router,
};
use serde::{Deserialize, Serialize};
use tracing::warn;

const INSTRUCTIONS: &str = "\
This server indexes Arknights game assets.
- Every game `version` (a client/res version pair) owns `bundles` (asset
  bundle files identified by path, hash and size) and a manifest tree that
  maps each asset name to the bundle containing it. Not every version has
  an imported manifest yet; `list_versions` (ready_only) shows the status.
- Version-aware tools accept `version_id` or `res_version` and default to
  the latest version whose manifest is imported (status `ready`).
- `search_manifest` finds which bundle contains an asset. Manifest names
  are paths like `arts/characters/char_002_amiya/...`,
  `avg/characters/avg_npc_009`, `avg/images/ac1_0` or
  `audio/sound_beta_2/music/.../m_avg_n_1`; a `#` segment appears in some
  skin names (e.g. `illust_char_002_amiya_epoque#4`).
- Story tools index story scripts: `list_story_resources` lists resources
  of one type (`background`, `image`, `item`, `character`) and
  `get_story_resource_usages` finds the scripts that use one. Always use
  ids exactly as `list_story_resources` returns them: characters are either
  `base#expression` or the body form `base$body`.
- `list_files`/`search_files` browse the extracted raw asset directory
  (e.g. `raw/char_arts`).
- `get_item_demand` looks up demand by Chinese item name (e.g. `固源岩`).
All tools are read-only.";

#[derive(Clone)]
pub struct AkAssetMcpServer {
    state: AppState,
    tool_router: ToolRouter<Self>,
}

impl AkAssetMcpServer {
    pub fn new(state: AppState) -> Self {
        Self {
            state,
            tool_router: Self::tool_router(),
        }
    }
}

/// `version_id` wins over `res_version`; when neither is given, tools that
/// need a version fall back to the latest one with an imported manifest.
#[derive(Deserialize, JsonSchema)]
struct VersionSelector {
    /// Numeric version id from `list_versions`; validated against the
    /// database. Takes precedence over `res_version`.
    version_id: Option<i32>,
    /// Resource version string from `list_versions`, e.g. `25-01-01-...`.
    res_version: Option<String>,
}

#[derive(Deserialize, JsonSchema)]
struct ListVersionsParams {
    /// Only include versions whose manifest import finished (status `ready`).
    ready_only: Option<bool>,
    /// Maximum number of versions returned (1-200, default 50).
    limit: Option<u32>,
}

#[derive(Deserialize, JsonSchema)]
struct SearchManifestParams {
    /// Case-insensitive substring of the asset name, e.g. `avg_npc_009` or
    /// `char_002_amiya`.
    q: String,
    #[serde(flatten)]
    version: VersionSelector,
}

#[derive(Deserialize, JsonSchema)]
struct ListManifestChildrenParams {
    /// Parent directory path from a previous `list_manifest_children` or
    /// `search_manifest` result; empty or omitted lists the manifest root.
    dir: Option<String>,
    /// Opaque `next_cursor` from the previous page; keep the directory and version unchanged.
    cursor: Option<String>,
    /// Maximum number of entries returned (1-200, default 50).
    limit: Option<u32>,
    #[serde(flatten)]
    version: VersionSelector,
}

#[derive(Deserialize, JsonSchema)]
struct GetManifestDetailParams {
    /// Full asset name (the `path` field of a `search_manifest` result).
    asset_name: String,
    #[serde(flatten)]
    version: VersionSelector,
}

#[derive(Deserialize, JsonSchema)]
struct SearchBundlesParams {
    /// Substring matched against bundle paths (matched literally).
    path: Option<String>,
    /// Exact bundle file hash.
    hash: Option<String>,
    /// Bundle file id.
    file_id: Option<i32>,
    /// Restrict to this version id. Unlike the other tools, omitting both
    /// `version_id` and `res_version` searches every version — then one of
    /// `path`/`hash`/`file_id` is required.
    version_id: Option<i32>,
    /// Restrict to this res version string; `version_id` wins when both
    /// are given.
    res_version: Option<String>,
    /// Maximum number of bundles returned (1-200, default 50).
    limit: Option<u32>,
}

#[derive(Deserialize, JsonSchema)]
struct ListFilesParams {
    /// Directory below the raw asset root, e.g. `raw/char_arts`; empty or
    /// omitted lists the root. Must be relative without `..`.
    path: Option<String>,
    /// Maximum number of entries returned (1-200, default 50).
    limit: Option<u32>,
    /// Opaque `next_cursor` from the previous page; keep the path unchanged.
    cursor: Option<String>,
}

#[derive(Deserialize, JsonSchema)]
struct SearchFilesParams {
    /// Substring matched against raw asset paths.
    q: String,
    /// Maximum number of entries returned (1-200, default 50).
    limit: Option<u32>,
}

#[derive(Deserialize, JsonSchema)]
struct ListStoryResourcesParams {
    /// Restrict to one resource type: `background`, `image`, `item` or `character`.
    resource_type: Option<String>,
    /// Case-insensitive substring filter on the resource id.
    q: Option<String>,
    /// Opaque cursor from a previous response (`next_cursor`).
    cursor: Option<String>,
    /// Page size (1-200, default 50).
    limit: Option<u32>,
}

#[derive(Deserialize, JsonSchema)]
struct GetStoryResourceUsagesParams {
    /// Resource type: `background`, `image`, `item` or `character`.
    resource_type: String,
    /// Exact resource id as returned by `list_story_resources`. Characters
    /// are `base#expression` for one expression or the body form `base$body`
    /// for every expression of the body.
    id: String,
    /// Opaque cursor from a previous response (`next_cursor`).
    cursor: Option<String>,
    /// Page size (1-200, default 50).
    limit: Option<u32>,
}

#[derive(Deserialize, JsonSchema)]
struct GetItemDemandParams {
    /// Item name in Chinese, e.g. `固源岩` or `技巧概要·卷3`.
    item_name: String,
}

/// Version details for the MCP surface: the REST `hot_update_list` raw
/// string is megabytes, so it is replaced by per-key entry counts.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct McpVersionDetails {
    id: i32,
    client_version: String,
    res_version: String,
    is_ready: bool,
    asset_mapping_status: String,
    /// Per-key summary of the hot-update list: array values become
    /// `{"count": n}`, scalar values pass through.
    hot_update_summary: Option<serde_json::Value>,
}

/// Wraps an unbounded query result, cutting it at `limit` so a single tool
/// call cannot flood the model context.
#[derive(Serialize)]
struct TruncatedResults<T> {
    results: Vec<T>,
    total: usize,
    truncated: bool,
}

impl<T> TruncatedResults<T> {
    fn new(results: Vec<T>, limit: u32) -> Self {
        let total = results.len();
        Self {
            results: results.into_iter().take(limit as usize).collect(),
            total,
            truncated: total > limit as usize,
        }
    }
}

#[derive(Deserialize, Serialize, PartialEq, Eq)]
enum DirectoryScope {
    Files { path: String },
    Manifest { dir: String, version_id: i32 },
}

#[derive(Deserialize, Serialize)]
struct DirectoryCursor {
    scope: DirectoryScope,
    offset: usize,
}

impl cursor::CursorPayload for DirectoryCursor {
    fn is_valid(&self) -> bool {
        let path = match &self.scope {
            DirectoryScope::Files { path } => path,
            DirectoryScope::Manifest { dir, .. } => dir,
        };
        !path.contains('\0')
    }
}

#[derive(Serialize)]
struct DirectoryPage<T> {
    results: Vec<T>,
    total: usize,
    truncated: bool,
    next_cursor: Option<String>,
}

/// Both directory queries already return a deterministically sorted listing.
/// Scope the offset to its directory/version so a cursor cannot silently skip
/// entries in a different listing.
fn directory_page<T>(
    entries: Vec<T>,
    limit: u32,
    cursor_value: Option<&str>,
    scope: DirectoryScope,
) -> Result<DirectoryPage<T>, ErrorData> {
    let after = decode_cursor::<DirectoryCursor>(cursor_value)?;
    let offset = if let Some(after) = after {
        if after.scope != scope {
            return Err(bad_request(
                "cursor does not match this directory or version",
            ));
        }
        after.offset
    } else {
        0
    };
    let total = entries.len();
    if offset > total {
        return Err(bad_request(
            "directory changed or cursor is out of range; restart without cursor",
        ));
    }
    let results: Vec<_> = entries
        .into_iter()
        .skip(offset)
        .take(limit as usize)
        .collect();
    let next_offset = offset + results.len();
    let truncated = next_offset < total;
    let next_cursor = truncated.then(|| {
        cursor::encode(&DirectoryCursor {
            scope,
            offset: next_offset,
        })
    });
    Ok(DirectoryPage {
        results,
        total,
        truncated,
        next_cursor,
    })
}

fn mcp_error(error: &AppError) -> ErrorData {
    match error {
        // Rejected input is safe to echo back and recoverable.
        AppError::InvalidInput(message) => bad_request(message.clone()),
        // Transient conditions (search budget exhausted, index not built yet)
        // are retryable; a distinct code in the JSON-RPC server-error range
        // lets clients back off and retry instead of treating the call as
        // permanently failed. Database and upstream failures
        // (`ExternalService`) are not retry hints and fall through below.
        AppError::Unavailable(message) => {
            warn!(reason = %message, "MCP tool temporarily unavailable");
            ErrorData::new(ErrorCode(-32003), message.clone(), None)
        }
        // Everything else may carry SQL text or internals; log the details
        // and return a fixed message, mirroring the REST error policy.
        other => {
            warn!(error = %other, "MCP tool failed");
            ErrorData::internal_error("internal error".to_string(), None)
        }
    }
}

fn bad_request(message: impl Into<String>) -> ErrorData {
    ErrorData::invalid_params(message.into(), None)
}

/// A miss the caller can recover from (unknown id, no match): returned as a
/// tool result with `isError` rather than a protocol error so the model can
/// adjust its arguments and retry.
fn not_found(message: impl Into<String>) -> CallToolResult {
    CallToolResult::error(vec![ContentBlock::text(message.into())])
}

fn json_result<T: Serialize>(value: T) -> Result<CallToolResult, ErrorData> {
    Ok(CallToolResult::success(vec![ContentBlock::json(value)?]))
}

fn ensure_no_nul(field: &str, value: &str) -> Result<(), ErrorData> {
    if value.contains('\0') {
        return Err(bad_request(format!(
            "{field} must not contain NUL characters"
        )));
    }
    Ok(())
}

fn parse_resource_type(value: &str) -> Result<StoryResourceType, ErrorData> {
    match value {
        "background" => Ok(StoryResourceType::Background),
        "image" => Ok(StoryResourceType::Image),
        "item" => Ok(StoryResourceType::Item),
        "character" => Ok(StoryResourceType::Character),
        _ => Err(bad_request(
            "resource_type must be one of background, image, item, character",
        )),
    }
}

/// A character id that is neither the expression form (`base#expression`)
/// nor the body form (`base$body`) — most likely a manifest-style bare name
/// that will silently match nothing.
fn is_bare_character_id(id: &str) -> bool {
    !id.contains('#') && !id.contains('$')
}

fn decode_cursor<T: for<'de> Deserialize<'de> + cursor::CursorPayload>(
    cursor_value: Option<&str>,
) -> Result<Option<T>, ErrorData> {
    cursor::decode(cursor_value).map_err(|err: WebError| bad_request(err.to_string()))
}

/// The version a selector resolved to.
struct ResolvedVersion {
    id: i32,
    status: AssetMappingStatus,
}

impl ResolvedVersion {
    fn from_row(row: &VersionRow) -> Option<Self> {
        Some(Self {
            id: row.id?,
            status: row.asset_mapping_status,
        })
    }
}

/// Recoverable-miss message for a version selector that matched nothing.
fn version_miss_message(version: &VersionSelector) -> String {
    version.version_id.map_or_else(
        || {
            version.res_version.as_deref().map_or_else(
                || "no version with an imported manifest yet".to_owned(),
                |res_version| {
                    format!("no version with res_version {res_version:?}; call list_versions")
                },
            )
        },
        |id| format!("version {id} not found; call list_versions for ids"),
    )
}

/// `isError` result for a version whose manifest import has not finished.
fn manifest_unready_error(version: &ResolvedVersion) -> Option<CallToolResult> {
    (version.status != AssetMappingStatus::Ready).then(|| {
        not_found(format!(
            "version {} has no imported manifest yet (status: {}); pick a ready version from list_versions",
            version.id,
            version.status.as_str()
        ))
    })
}

#[tool_router]
impl AkAssetMcpServer {
    /// List game resource versions, newest first, with their numeric id,
    /// client/res version strings, readiness and manifest import status.
    /// Start here to pick a version for other tools; pass `ready_only` to
    /// list only versions with an imported manifest.
    #[tool]
    async fn list_versions(
        &self,
        Parameters(params): Parameters<ListVersionsParams>,
    ) -> Result<CallToolResult, ErrorData> {
        let limit = story::page_limit(params.limit).map_err(|error| mcp_error(&error))?;
        let mut versions = self
            .state
            .database
            .query_versions()
            .await
            .map_err(|error| mcp_error(&error))?;
        versions.reverse();
        if params.ready_only.unwrap_or(false) {
            versions.retain(|version| version.asset_mapping_status == "ready");
        }
        json_result(TruncatedResults::new(versions, limit))
    }

    /// Get one version's details. The hot-update list is summarized to
    /// per-key entry counts — the raw list is megabytes. Defaults to the
    /// latest version with an imported manifest.
    #[tool]
    async fn get_version(
        &self,
        Parameters(version): Parameters<VersionSelector>,
    ) -> Result<CallToolResult, ErrorData> {
        let Some(row) = self.fetch_version(&version).await? else {
            return Ok(not_found(version_miss_message(&version)));
        };
        let Some(id) = row.id else {
            return Err(ErrorData::internal_error(
                "version row without id".to_string(),
                None,
            ));
        };
        json_result(McpVersionDetails {
            id,
            client_version: row.client,
            res_version: row.res,
            is_ready: row.is_ready,
            asset_mapping_status: row.asset_mapping_status.as_str().to_string(),
            hot_update_summary: summarize_hot_update_list(&row.hot_update_list),
        })
    }

    /// Search manifest entries by asset name substring and get the bundle
    /// each asset lives in. The primary way to locate an asset. `q` matches
    /// literally; returns at most 200 matches, with `truncated` set when
    /// that cap was hit — refine `q` then.
    #[tool]
    async fn search_manifest(
        &self,
        Parameters(params): Parameters<SearchManifestParams>,
    ) -> Result<CallToolResult, ErrorData> {
        ensure_no_nul("q", &params.q)?;
        let Some(version) = self.resolve_version(&params.version).await? else {
            return Ok(not_found(version_miss_message(&params.version)));
        };
        if let Some(error) = manifest_unready_error(&version) {
            return Ok(error);
        }
        // One row past the cap as the has-more probe, same trick as the
        // paginated tools.
        let nodes = self
            .state
            .database
            .search_manifest(
                version.id,
                &escape_like(&params.q),
                i64::from(story::MAX_PAGE_LIMIT) + 1,
            )
            .await
            .map_err(|error| mcp_error(&error))?;
        json_result(TruncatedResults::new(nodes, story::MAX_PAGE_LIMIT))
    }

    /// Browse the manifest directory tree one level at a time: given a
    /// directory path, list its files and subdirectories. Pass `next_cursor`
    /// back as `cursor` with the same directory and version to continue.
    #[tool]
    async fn list_manifest_children(
        &self,
        Parameters(params): Parameters<ListManifestChildrenParams>,
    ) -> Result<CallToolResult, ErrorData> {
        let limit = story::page_limit(params.limit).map_err(|error| mcp_error(&error))?;
        let dir = params.dir.unwrap_or_default();
        ensure_no_nul("dir", &dir)?;
        let Some(version) = self.resolve_version(&params.version).await? else {
            return Ok(not_found(version_miss_message(&params.version)));
        };
        if let Some(error) = manifest_unready_error(&version) {
            return Ok(error);
        }
        let nodes = self
            .state
            .database
            .list_manifest_children(version.id, &dir)
            .await
            .map_err(|error| mcp_error(&error))?;
        json_result(directory_page(
            nodes,
            limit,
            params.cursor.as_deref(),
            DirectoryScope::Manifest {
                dir,
                version_id: version.id,
            },
        )?)
    }

    /// Get one manifest entry: which bundle contains the asset, and that
    /// bundle's size and hash. Use the full asset name (the `path` of a
    /// `search_manifest` result).
    #[tool]
    async fn get_manifest_detail(
        &self,
        Parameters(params): Parameters<GetManifestDetailParams>,
    ) -> Result<CallToolResult, ErrorData> {
        let asset_name = params.asset_name;
        ensure_no_nul("asset_name", &asset_name)?;
        let Some(version) = self.resolve_version(&params.version).await? else {
            return Ok(not_found(version_miss_message(&params.version)));
        };
        if let Some(error) = manifest_unready_error(&version) {
            return Ok(error);
        }
        let Some(detail) = self
            .state
            .database
            .get_asset_mapping_detail(version.id, &asset_name)
            .await
            .map_err(|error| mcp_error(&error))?
        else {
            return Ok(not_found(format!(
                "no manifest entry {asset_name:?} in this version; try search_manifest"
            )));
        };
        json_result(detail)
    }

    /// Filter bundles by path substring (matched literally), exact hash,
    /// file id and/or an explicit version, newest version first. At least
    /// one of `path`, `hash`, `file_id` or `version_id`/`res_version` is
    /// required — omitting the version searches every version.
    #[tool]
    async fn search_bundles(
        &self,
        Parameters(params): Parameters<SearchBundlesParams>,
    ) -> Result<CallToolResult, ErrorData> {
        let limit = story::page_limit(params.limit).map_err(|error| mcp_error(&error))?;
        if let Some(path) = params.path.as_deref() {
            ensure_no_nul("path", path)?;
        }
        if let Some(hash) = params.hash.as_deref() {
            ensure_no_nul("hash", hash)?;
        }
        let selector = VersionSelector {
            version_id: params.version_id,
            res_version: params.res_version,
        };
        let explicit_version = selector.version_id.is_some() || selector.res_version.is_some();
        let version_filter = if explicit_version {
            let Some(version) = self.resolve_version(&selector).await? else {
                return Ok(not_found(version_miss_message(&selector)));
            };
            Some(version.id)
        } else {
            None
        };
        if version_filter.is_none()
            && params.path.is_none()
            && params.hash.is_none()
            && params.file_id.is_none()
        {
            return Err(bad_request(
                "provide at least one of path, hash, file_id or an explicit version; \
                 without a filter every bundle in the database matches",
            ));
        }
        // limit + 1 rows so `truncated` reflects whether more matches exist
        // beyond the page, without ever loading them.
        let bundles = self
            .state
            .database
            .query_bundles_with_details_limited(
                &BundleFilter {
                    path: params.path.as_deref().map(escape_like),
                    hash: params.hash,
                    file: params.file_id,
                    version: version_filter,
                },
                i64::from(limit) + 1,
            )
            .await
            .map_err(|error| mcp_error(&error))?;
        json_result(TruncatedResults::new(bundles, limit))
    }

    /// List the entries of one directory in the extracted raw asset tree,
    /// e.g. `raw/char_arts`. Omit `path` to list the root. Pass `next_cursor`
    /// back as `cursor` with the same path to continue.
    #[tool]
    async fn list_files(
        &self,
        Parameters(params): Parameters<ListFilesParams>,
    ) -> Result<CallToolResult, ErrorData> {
        let limit = story::page_limit(params.limit).map_err(|error| mcp_error(&error))?;
        let path = params.path.unwrap_or_default();
        let dir = self
            .state
            .torappu
            .list_asset(&path)
            .map_err(|error| mcp_error(&error))?;
        json_result(directory_page(
            dir.children,
            limit,
            params.cursor.as_deref(),
            DirectoryScope::Files { path },
        )?)
    }

    /// Search the extracted raw asset tree by path substring. Prefer this
    /// over walking directories with `list_files` when hunting one file.
    #[tool]
    async fn search_files(
        &self,
        Parameters(params): Parameters<SearchFilesParams>,
    ) -> Result<CallToolResult, ErrorData> {
        let limit = story::page_limit(params.limit).map_err(|error| mcp_error(&error))?;
        ensure_no_nul("q", &params.q)?;
        // Fail fast when the shared search budget (also used by REST) is
        // exhausted; the permit moves into the blocking task so it is held
        // until the search finishes.
        let permit = self
            .state
            .search_gate
            .try_acquire()
            .map_err(|error| mcp_error(&error))?;
        let torappu = self.state.torappu.clone();
        let entries = tokio::task::spawn_blocking(move || {
            let _permit = permit;
            torappu.search_assets_by_path(&params.q)
        })
        .await
        .map_err(|error| mcp_error(&AppError::Application(error.into())))?
        .map_err(|error| mcp_error(&error))?;
        json_result(TruncatedResults::new(entries, limit))
    }

    /// List story resources (backgrounds, images, items, characters) with the
    /// number of scripts using each. Characters are listed at body
    /// granularity (`base$body`). Supports cursor pagination.
    #[tool]
    async fn list_story_resources(
        &self,
        Parameters(params): Parameters<ListStoryResourcesParams>,
    ) -> Result<CallToolResult, ErrorData> {
        let resource_type = params
            .resource_type
            .as_deref()
            .map(parse_resource_type)
            .transpose()?;
        let limit = story::page_limit(params.limit).map_err(|error| mcp_error(&error))?;
        if let Some(q) = params.q.as_deref() {
            ensure_no_nul("q", q)?;
        }
        let pattern = params
            .q
            .as_deref()
            .filter(|value| !value.is_empty())
            .map(escape_like);
        let after = decode_cursor::<ResourceCursor>(params.cursor.as_deref())?;
        let response = story::list_story_resources_page(
            &self.state.database,
            resource_type,
            pattern.as_deref(),
            after
                .as_ref()
                .map(|cursor| (cursor.resource_type.as_str(), cursor.resource_id.as_str())),
            limit,
        )
        .await
        .map_err(|error| mcp_error(&error))?;
        json_result(response)
    }

    /// Find the story scripts that use one resource, with display names.
    /// For characters, `base#expression` targets one expression while
    /// `base$body` returns every expression of the body (with a `faces`
    /// list per script). Supports cursor pagination.
    #[tool]
    async fn get_story_resource_usages(
        &self,
        Parameters(params): Parameters<GetStoryResourceUsagesParams>,
    ) -> Result<CallToolResult, ErrorData> {
        let resource_type = parse_resource_type(&params.resource_type)?;
        let limit = story::page_limit(params.limit).map_err(|error| mcp_error(&error))?;
        ensure_no_nul("id", &params.id)?;
        if resource_type == StoryResourceType::Character && is_bare_character_id(&params.id) {
            return Ok(not_found(format!(
                "{:?} is not a valid character id; use list_story_resources to get the exact id \
                 — characters are either `base#expression` or the body form `base$body`",
                params.id
            )));
        }
        let after = decode_cursor::<UsageCursor>(params.cursor.as_deref())?;
        let after_path = after.as_ref().map(|cursor| cursor.script_path.as_str());
        let response = story::story_resource_usages_page(
            &self.state.database,
            resource_type,
            &params.id,
            after_path,
            limit,
        )
        .await
        .map_err(|error| mcp_error(&error))?;
        json_result(response)
    }

    /// Get the aggregated demand for one item across game systems (returned
    /// as the stored JSON document).
    #[tool]
    async fn get_item_demand(
        &self,
        Parameters(params): Parameters<GetItemDemandParams>,
    ) -> Result<CallToolResult, ErrorData> {
        let item_name = params.item_name;
        ensure_no_nul("item_name", &item_name)?;
        let Some(usage) = self
            .state
            .database
            .query_usage_by_item_name(&item_name)
            .await
            .map_err(|error| mcp_error(&error))?
        else {
            return Ok(not_found(format!("no demand data for item {item_name:?}")));
        };
        serde_json::from_str::<serde_json::Value>(&usage).map_or_else(
            |_| Ok(CallToolResult::success(vec![ContentBlock::text(usage)])),
            json_result,
        )
    }

    /// Fetches the selected version row: by id, by res version, or the
    /// latest one with an imported manifest. `None` means the selector
    /// matched nothing (recoverable).
    async fn fetch_version(
        &self,
        version: &VersionSelector,
    ) -> Result<Option<VersionRow>, ErrorData> {
        if let Some(id) = version.version_id {
            return self
                .state
                .database
                .get_version_by_id(id)
                .await
                .map_err(|error| mcp_error(&error));
        }
        if let Some(res_version) = version.res_version.as_deref() {
            ensure_no_nul("res_version", res_version)?;
            return self
                .state
                .database
                .get_version_by_res(res_version)
                .await
                .map_err(|error| mcp_error(&error));
        }
        self.state
            .database
            .get_latest_ready_version()
            .await
            .map_err(|error| mcp_error(&error))
    }

    /// Like `fetch_version`, reduced to what version-scoped queries need.
    async fn resolve_version(
        &self,
        version: &VersionSelector,
    ) -> Result<Option<ResolvedVersion>, ErrorData> {
        Ok(self
            .fetch_version(version)
            .await?
            .as_ref()
            .and_then(ResolvedVersion::from_row))
    }
}

/// Reduces the raw hot-update list JSON to per-key entry counts (arrays) and
/// pass-through scalars; `None` when the stored value is not a JSON object.
fn summarize_hot_update_list(raw: &str) -> Option<serde_json::Value> {
    let value: serde_json::Value = serde_json::from_str(raw).ok()?;
    let serde_json::Value::Object(map) = value else {
        return None;
    };
    let mut summary = serde_json::Map::new();
    for (key, value) in map {
        let value = match value {
            serde_json::Value::Array(items) => serde_json::json!({ "count": items.len() }),
            other => other,
        };
        summary.insert(key, value);
    }
    Some(serde_json::Value::Object(summary))
}

#[tool_handler(router = self.tool_router)]
impl ServerHandler for AkAssetMcpServer {
    fn get_info(&self) -> rmcp::model::ServerConfig {
        rmcp::model::ServerConfig::new(
            rmcp::model::ServerCapabilities::builder()
                .enable_tools()
                .build(),
        )
        // `from_build_env` reports rmcp's own crate name, since the env!
        // lookups are compiled inside that crate.
        .with_server_info(rmcp::model::Implementation::new(
            "ak-asset-storage",
            env!("CARGO_PKG_VERSION"),
        ))
        .with_instructions(INSTRUCTIONS.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_resource_type_accepts_known_types_only() {
        assert!(matches!(
            parse_resource_type("background").expect("background"),
            StoryResourceType::Background
        ));
        assert!(matches!(
            parse_resource_type("character").expect("character"),
            StoryResourceType::Character
        ));
        assert!(parse_resource_type("Background").is_err());
        assert!(parse_resource_type("audio").is_err());
    }

    #[test]
    fn mcp_error_only_marks_transient_conditions_retryable() {
        let busy = mcp_error(&AppError::Unavailable(
            "search capacity is busy".to_string(),
        ));
        assert_eq!(busy.code, ErrorCode(-32003));
        assert_eq!(busy.message, "search capacity is busy");

        // Database/upstream failures are `ExternalService`; they must keep
        // the fixed internal-error reply, not the retryable search hint.
        let database = mcp_error(&AppError::ExternalService(anyhow::anyhow!(
            "relation \"versions\" does not exist"
        )));
        assert_eq!(database.code, ErrorCode::INTERNAL_ERROR);
        assert_eq!(database.message, "internal error");
    }

    #[test]
    fn truncated_results_reports_cut() {
        let results = TruncatedResults::new(vec![1, 2, 3], 2);
        assert_eq!(results.results, vec![1, 2]);
        assert_eq!(results.total, 3);
        assert!(results.truncated);

        let results = TruncatedResults::new(vec![1], 2);
        assert_eq!(results.results, vec![1]);
        assert!(!results.truncated);
    }

    #[test]
    fn directory_pages_visit_every_entry_once() {
        for manifest in [false, true] {
            for total in [0, 200, 201, 400, 401] {
                let mut cursor = None;
                let mut visited = Vec::new();
                loop {
                    let scope = if manifest {
                        DirectoryScope::Manifest {
                            dir: "chararts".into(),
                            version_id: 1,
                        }
                    } else {
                        DirectoryScope::Files {
                            path: "raw/char_arts".into(),
                        }
                    };
                    let page = directory_page((0..total).collect(), 200, cursor.as_deref(), scope)
                        .expect("valid page");
                    assert_eq!(page.total, total);
                    assert!(page.results.len() <= 200);
                    visited.extend(page.results);
                    assert_eq!(page.truncated, visited.len() < total);
                    assert_eq!(page.next_cursor.is_some(), page.truncated);
                    cursor = page.next_cursor;
                    if cursor.is_none() {
                        break;
                    }
                    assert!(!visited.is_empty());
                }
                assert_eq!(visited, (0..total).collect::<Vec<_>>());
            }
        }
    }

    #[test]
    fn directory_pages_reject_invalid_or_mismatched_cursors() {
        let scope = || DirectoryScope::Manifest {
            dir: "chararts".into(),
            version_id: 1,
        };
        let first = directory_page(vec![1, 2], 1, None, scope()).expect("first page");
        for other_scope in [
            DirectoryScope::Manifest {
                dir: "other".into(),
                version_id: 1,
            },
            DirectoryScope::Manifest {
                dir: "chararts".into(),
                version_id: 2,
            },
            DirectoryScope::Files {
                path: "chararts".into(),
            },
        ] {
            assert!(
                directory_page(vec![1, 2], 1, first.next_cursor.as_deref(), other_scope).is_err()
            );
        }
        assert!(directory_page(vec![1, 2], 1, Some("invalid!"), scope()).is_err());
        let out_of_range = cursor::encode(&DirectoryCursor {
            scope: scope(),
            offset: usize::MAX,
        });
        assert!(directory_page(vec![1, 2], 1, Some(&out_of_range), scope()).is_err());
    }

    #[test]
    fn version_selector_flattens_into_tool_arguments() {
        let params: SearchManifestParams = serde_json::from_value(serde_json::json!({
            "q": "avg_npc_009",
            "res_version": "25-01-01-10-00-00-000000"
        }))
        .expect("flatten");
        assert_eq!(params.q, "avg_npc_009");
        assert_eq!(
            params.version.res_version.as_deref(),
            Some("25-01-01-10-00-00-000000")
        );
        assert!(params.version.version_id.is_none());
    }

    #[test]
    fn search_bundles_params_take_top_level_version_fields() {
        let params: SearchBundlesParams = serde_json::from_value(serde_json::json!({
            "path": "ab_avg",
            "version_id": 7
        }))
        .expect("top-level version fields");
        assert_eq!(params.version_id, Some(7));
        assert!(params.res_version.is_none());
        assert_eq!(params.path.as_deref(), Some("ab_avg"));
    }

    #[test]
    fn bare_character_ids_are_flagged() {
        assert!(is_bare_character_id("avg_npc_009"));
        assert!(!is_bare_character_id("avg_npc_009#avg_npc_009"));
        assert!(!is_bare_character_id("avg_npc_009$body"));
    }

    #[test]
    fn hot_update_summary_counts_arrays_and_keeps_scalars() {
        let summary = summarize_hot_update_list(
            r#"{"abInfos":[{"a":1},{"a":2}],"packInfos":[1],"versionId":"25-01-01","time":1}"#,
        )
        .expect("parses");
        assert_eq!(
            summary,
            serde_json::json!({
                "abInfos": {"count": 2},
                "packInfos": {"count": 1},
                "versionId": "25-01-01",
                "time": 1
            })
        );
        assert!(summarize_hot_update_list("[]").is_none());
        assert!(summarize_hot_update_list("not json").is_none());
    }

    #[test]
    fn miss_messages_point_at_list_versions() {
        let message = version_miss_message(&VersionSelector {
            version_id: Some(42),
            res_version: None,
        });
        assert!(message.contains("42"), "{message}");
        assert!(message.contains("list_versions"), "{message}");

        let message = version_miss_message(&VersionSelector {
            version_id: None,
            res_version: Some("bogus".to_string()),
        });
        assert!(message.contains("bogus"), "{message}");

        let message = version_miss_message(&VersionSelector {
            version_id: None,
            res_version: None,
        });
        assert!(message.contains("imported manifest"), "{message}");
    }
}
