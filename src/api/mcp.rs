//! MCP (Model Context Protocol) server: read-only query tools exposing the
//! same data as the REST API (versions, manifest tree, bundles, story
//! resources, raw asset directory) plus gamedata file reads to AI clients
//! over Streamable HTTP at `/mcp`.

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
        model::VersionSummary,
        row::{AssetMappingStatus, StoryResourceType, VersionRow},
    },
    external::{
        gamedata::{self, GamedataError, GamedataResult, GamedataStatus, Outline, TextSlice},
        torappu::TorappuClient,
        types::AssetEntry,
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
use std::{
    fmt::Write as _,
    path::{Component, Path},
    sync::Arc,
};
use tokio::sync::Semaphore;
use tracing::warn;

const INSTRUCTIONS: &str = "\
This server indexes Arknights game assets.
- Every game `version` (a client/res version pair) owns `bundles` (asset
  bundle files identified by path, hash and size) and a manifest tree that
  maps each asset name to the bundle containing it. Not every version has
  an imported manifest yet; `list_versions` (ready_only) shows the status.
- Version-aware tools accept `version_id` or `res_version` and default to
  the latest version whose manifest is imported (status `ready`);
  `read_gamedata` instead defaults to the latest version whose gamedata
  extraction finished (`gamedataStatus` `ready` in `list_versions`).
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
- `read_gamedata` reads decoded game tables such as
  `excel/character_table.json` (several MB each): outline a JSON value
  with `keys_only`, then read just the part you need with `pointer`
  (RFC 6901, e.g. `/char_002_amiya/name`). Story scripts and other text
  files are read in pages via `offset`.
- `list_files`/`search_files` browse the extracted asset tree: `raw/`
  holds art and audio (e.g. `raw/char_arts`), `gamedata/<res_version>/`
  the decoded tables. Gamedata repeats once per version, so give
  `search_files` a `path_prefix` such as `gamedata/latest/` or `raw/`.
  File entries carry a public `url`.
- `get_item_demand` looks up demand by Chinese item name (e.g. `固源岩`).
All tools are read-only.";

#[derive(Clone)]
pub struct AkAssetMcpServer {
    state: Arc<AppState>,
    tool_router: Arc<ToolRouter<Self>>,
    /// Bounds concurrent `read_gamedata` calls: each may hold a parsed
    /// multi-megabyte table. Shared by every clone (one per request).
    gamedata_gate: Arc<Semaphore>,
}

impl AkAssetMcpServer {
    pub fn new(state: AppState) -> Self {
        let gamedata_gate = Arc::new(Semaphore::new(state.settings.mcp.gamedata_read_concurrency));
        Self {
            state: Arc::new(state),
            tool_router: Arc::new(Self::tool_router()),
            gamedata_gate,
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
    /// Only include versions whose gamedata extraction finished
    /// (`gamedataStatus` `ready`), i.e. readable with `read_gamedata`.
    gamedata_ready_only: Option<bool>,
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
    /// Case-sensitive literal substring of the relative asset path.
    q: String,
    /// Only return paths starting with this literal relative prefix, e.g.
    /// `raw/` or `gamedata/<res_version>/`. `gamedata/latest/` resolves to
    /// the version the latest alias points at.
    path_prefix: Option<String>,
    /// Maximum entries returned (1-200, default 100). No pagination.
    limit: Option<u32>,
}

/// `read_gamedata` response budget bounds, in bytes.
const GAMEDATA_DEFAULT_MAX_BYTES: u32 = 20_000;
const GAMEDATA_MIN_MAX_BYTES: u32 = 1_000;
const GAMEDATA_MAX_MAX_BYTES: u32 = 100_000;

#[derive(Deserialize, JsonSchema)]
struct ReadGamedataParams {
    /// File path inside the version's gamedata directory, e.g.
    /// `excel/character_table.json` or a story script such as
    /// `story/obt/main/level_main_00-01_beg.txt`.
    path: String,
    /// RFC 6901 JSON Pointer into a JSON file, e.g. `/char_002_amiya` or
    /// `/char_002_amiya/phases/0`; empty or omitted selects the whole file.
    pointer: Option<String>,
    /// For JSON files: list the children of the value at `pointer` (key,
    /// full pointer, type, size) instead of returning the value. Object
    /// keys are listed in sorted order, array items by index.
    keys_only: Option<bool>,
    /// `next_offset` from the previous response: the first child listed by
    /// `keys_only`, or the byte offset into a text file.
    offset: Option<u64>,
    /// Budget in bytes for the returned value, outline entries or text
    /// (1000-100000, default 20000); file metadata adds a few hundred bytes.
    /// Larger JSON values are refused with a hint instead of being cut.
    max_bytes: Option<u32>,
    #[serde(flatten)]
    version: VersionSelector,
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

/// `list_versions` entry: the REST summary plus the gamedata status.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct McpVersionSummary {
    #[serde(flatten)]
    summary: VersionSummary,
    gamedata_status: GamedataStatus,
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
    /// `ready` once torappu finished extracting this version's gamedata;
    /// `unmarked`: a directory without the ready marker (still extracting,
    /// or extracted before markers existed); `missing`: no directory.
    gamedata_status: GamedataStatus,
    /// Per-key summary of the hot-update list: array values become
    /// `{"count": n}`, scalar values pass through.
    hot_update_summary: Option<serde_json::Value>,
}

/// An asset entry plus its public download URL (files only).
#[derive(Serialize)]
struct McpAssetEntry {
    #[serde(flatten)]
    entry: AssetEntry,
    #[serde(skip_serializing_if = "Option::is_none")]
    url: Option<String>,
}

impl McpAssetEntry {
    fn new(host: &str, entry: AssetEntry) -> Self {
        let url = asset_url(host, &entry.path, entry.is_dir);
        Self { entry, url }
    }
}

#[derive(Serialize)]
struct McpSearchResults {
    results: Vec<McpAssetEntry>,
    truncated: bool,
    /// The prefix actually applied, after resolving `gamedata/latest`.
    #[serde(skip_serializing_if = "Option::is_none")]
    path_prefix: Option<String>,
}

/// Which gamedata file a `read_gamedata` response came from.
#[derive(Serialize)]
struct GamedataFile {
    res_version: String,
    gamedata_status: GamedataStatus,
    path: String,
    url: Option<String>,
}

#[derive(Serialize)]
#[serde(untagged)]
enum GamedataContent {
    Value {
        pointer: String,
        value: serde_json::Value,
    },
    Outline {
        pointer: String,
        outline: Outline,
    },
    Text(TextSlice),
}

#[derive(Serialize)]
struct ReadGamedataResponse {
    #[serde(flatten)]
    file: GamedataFile,
    #[serde(flatten)]
    content: GamedataContent,
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

/// Public URL of an extracted file, following the static routes in
/// `router.rs`: `/assets` serves `raw/` and `/gamedata` serves `gamedata/`.
/// Directories get none — the static routes do not list them.
fn asset_url(host: &str, path: &str, is_dir: bool) -> Option<String> {
    if is_dir {
        return None;
    }
    let (route, rest) = path
        .strip_prefix("raw/")
        .map(|rest| ("assets", rest))
        .or_else(|| {
            path.strip_prefix("gamedata/")
                .map(|rest| ("gamedata", rest))
        })?;
    if rest.is_empty() {
        return None;
    }
    let mut url = format!("{}/{route}", host.trim_end_matches('/'));
    for segment in rest.split('/') {
        url.push('/');
        percent_encode_segment(&mut url, segment);
    }
    Some(url)
}

/// Percent-encodes everything but RFC 3986 unreserved characters, so names
/// such as `illust_char_002_amiya_epoque#4` or `[uc]lua` stay one segment.
fn percent_encode_segment(out: &mut String, segment: &str) {
    for byte in segment.bytes() {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'.' | b'_' | b'~') {
            out.push(char::from(byte));
        } else {
            let _ = write!(out, "%{byte:02X}");
        }
    }
}

/// A `path_prefix` for `search_files`: relative, without `.`/`..` segments.
/// A leading `gamedata/latest` is replaced by the version the alias links
/// to, because the index skips the alias itself.
fn resolve_search_prefix(torappu: &TorappuClient, prefix: &str) -> Result<String, ErrorData> {
    ensure_no_nul("path_prefix", prefix)?;
    let plain = Path::new(prefix)
        .components()
        .all(|component| matches!(component, Component::Normal(_)));
    if !plain {
        return Err(bad_request(
            "path_prefix must be a relative path such as raw/ or gamedata/<res_version>/ \
             without . or .. segments",
        ));
    }
    let alias = format!("gamedata/{}", gamedata::LATEST_ALIAS);
    let Some(rest) = prefix
        .strip_prefix(&alias)
        .filter(|rest| rest.is_empty() || rest.starts_with('/'))
    else {
        return Ok(prefix.to_string());
    };
    let Some(target) = torappu.gamedata_latest_target() else {
        return Err(bad_request(
            "gamedata/latest does not link to a version; use gamedata/<res_version>/ \
             with a res_version from list_versions",
        ));
    };
    Ok(format!("gamedata/{target}{rest}"))
}

/// Reads one gamedata file for `read_gamedata`. Blocking: parses JSON.
fn read_gamedata_file(
    torappu: &TorappuClient,
    host: &str,
    res_version: &str,
    params: &ReadGamedataParams,
    max_bytes: usize,
) -> GamedataResult<ReadGamedataResponse> {
    let resolved = torappu.gamedata_file(res_version, &params.path)?;
    // `gamedata_file` accepted the path, so it has only plain segments
    // (and `.`, dropped here) — use it to name the file and build its URL.
    let path = Path::new(&params.path)
        .components()
        .filter_map(|component| match component {
            Component::Normal(segment) => segment.to_str(),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join("/");
    let is_json = Path::new(&path)
        .extension()
        .is_some_and(|extension| extension.eq_ignore_ascii_case("json"));
    let file = GamedataFile {
        res_version: res_version.to_string(),
        gamedata_status: torappu.gamedata_status(res_version),
        url: asset_url(host, &format!("gamedata/{res_version}/{path}"), false),
        path,
    };
    let keys_only = params.keys_only.unwrap_or(false);
    let content = if is_json {
        if params.offset.is_some() && !keys_only {
            return Err(AppError::InvalidInput(
                "offset pages keys_only listings for JSON files; select parts with pointer"
                    .to_string(),
            )
            .into());
        }
        let root = gamedata::read_json(&resolved)?;
        let pointer = params.pointer.clone().unwrap_or_default();
        let Some(value) = root.pointer(&pointer) else {
            let parent = pointer.rsplit_once('/').map_or("", |(parent, _)| parent);
            return Err(GamedataError::Miss(format!(
                "no value at pointer {pointer:?}; list the children of {parent:?} with \
                 keys_only (keys containing `/` or `~` are escaped as ~1 and ~0)"
            )));
        };
        if keys_only {
            let offset = usize::try_from(params.offset.unwrap_or(0)).unwrap_or(usize::MAX);
            GamedataContent::Outline {
                outline: gamedata::outline(value, &pointer, offset, max_bytes),
                pointer,
            }
        } else {
            let size = gamedata::json_size(value);
            if size > max_bytes {
                return Err(GamedataError::Miss(oversized_value_message(
                    &pointer, value, size, max_bytes,
                )));
            }
            GamedataContent::Value {
                value: value.clone(),
                pointer,
            }
        }
    } else {
        if params
            .pointer
            .as_deref()
            .is_some_and(|pointer| !pointer.is_empty())
            || keys_only
        {
            return Err(AppError::InvalidInput(format!(
                "pointer and keys_only apply to JSON files; {:?} is read as text, page it with offset",
                params.path
            ))
            .into());
        }
        GamedataContent::Text(gamedata::read_text(
            &resolved,
            params.offset.unwrap_or(0),
            max_bytes,
        )?)
    };
    Ok(ReadGamedataResponse { file, content })
}

fn oversized_value_message(
    pointer: &str,
    value: &serde_json::Value,
    size: usize,
    max_bytes: usize,
) -> String {
    let shape = match value {
        serde_json::Value::Object(map) => format!("an object with {} keys", map.len()),
        serde_json::Value::Array(items) => format!("an array of {} items", items.len()),
        other => format!("a {}", gamedata::json_type(other)),
    };
    format!(
        "value at pointer {pointer:?} is {size} bytes, over max_bytes {max_bytes}; it is {shape}. \
         Retry with keys_only: true to list its children with their pointers and sizes, \
         then read a deeper pointer (or raise max_bytes, at most {GAMEDATA_MAX_MAX_BYTES})."
    )
}

fn join_error(error: tokio::task::JoinError) -> ErrorData {
    mcp_error(&AppError::Application(error.into()))
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
    /// client/res version strings, readiness, manifest import status and
    /// gamedata status. Start here to pick a version for other tools; pass
    /// `ready_only` to list only versions with an imported manifest, or
    /// `gamedata_ready_only` for versions readable with `read_gamedata`.
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
        let gamedata_ready_only = params.gamedata_ready_only.unwrap_or(false);
        let torappu = self.state.torappu.clone();
        // One or two stats per version; a few hundred versions in total.
        let versions = tokio::task::spawn_blocking(move || {
            let mut versions: Vec<_> = versions
                .into_iter()
                .map(|summary| McpVersionSummary {
                    gamedata_status: torappu.gamedata_status(&summary.res_version),
                    summary,
                })
                .collect();
            if gamedata_ready_only {
                versions.retain(|version| version.gamedata_status == GamedataStatus::Ready);
            }
            versions
        })
        .await
        .map_err(join_error)?;
        json_result(TruncatedResults::new(versions, limit))
    }

    /// Get one version's details, including its gamedata status. The
    /// hot-update list is summarized to per-key entry counts — the raw list
    /// is megabytes. Defaults to the latest version with an imported
    /// manifest.
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
            gamedata_status: self.state.torappu.gamedata_status(&row.res),
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
    /// file id and/or an explicit version, newest version first and then by
    /// path ascending. At least one of `path`, `hash`, `file_id` or
    /// `version_id`/`res_version` is required — omitting the version
    /// searches every version.
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
        // Blank path/hash match everything, so they count as absent
        // (surrounding whitespace is trimmed) — same rule as the REST
        // /bundle endpoint.
        let path_filter = params
            .path
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty());
        let hash_filter = params
            .hash
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty());
        if version_filter.is_none()
            && path_filter.is_none()
            && hash_filter.is_none()
            && params.file_id.is_none()
        {
            return Err(bad_request(
                "provide at least one of path, hash, file_id or an explicit version; \
                 without a filter every bundle in the database matches",
            ));
        }
        // limit + 1 rows so `truncated` reflects whether more matches exist
        // beyond the page, without ever loading them. Same query and order
        // (version DESC, path ASC, id ASC) as the REST /bundle endpoint.
        let bundles = self
            .state
            .database
            .query_bundles_with_details_page(
                &BundleFilter {
                    path: path_filter.map(escape_like),
                    hash: hash_filter.map(str::to_string),
                    file: params.file_id,
                    version: version_filter,
                },
                None,
                i64::from(limit) + 1,
            )
            .await
            .map_err(|error| mcp_error(&error))?;
        json_result(TruncatedResults::new(bundles, limit))
    }

    /// List the entries of one directory in the extracted asset tree, e.g.
    /// `raw/char_arts` or `gamedata/latest/excel`. Omit `path` to list the
    /// root. File entries carry a public `url`. Pass `next_cursor` back as
    /// `cursor` with the same path to continue.
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
        let host = &self.state.settings.server.host;
        json_result(directory_page(
            dir.children
                .into_iter()
                .map(|entry| McpAssetEntry::new(host, entry))
                .collect(),
            limit,
            params.cursor.as_deref(),
            DirectoryScope::Files { path },
        )?)
    }

    /// Search extracted assets by relative path substring, at any depth.
    /// Returns results (file entries carry a public `url`) and truncated,
    /// without a total or pagination; narrow `q` or `path_prefix` when
    /// `truncated` is true. Gamedata exists once per version and unscoped
    /// matches come oldest version first, so to search current gamedata
    /// pass `path_prefix` `gamedata/latest/` (or `gamedata/<res_version>/`);
    /// use `raw/` for art and audio.
    #[tool]
    async fn search_files(
        &self,
        Parameters(params): Parameters<SearchFilesParams>,
    ) -> Result<CallToolResult, ErrorData> {
        ensure_no_nul("q", &params.q)?;
        let path_prefix = params
            .path_prefix
            .as_deref()
            .filter(|prefix| !prefix.is_empty())
            .map(|prefix| resolve_search_prefix(&self.state.torappu, prefix))
            .transpose()?;
        // Fail fast when the shared search budget (also used by REST) is
        // exhausted; the permit moves into the blocking task so it is held
        // until the search finishes.
        let permit = self
            .state
            .search_gate
            .try_acquire()
            .map_err(|error| mcp_error(&error))?;
        let index = self.state.plocate.clone();
        let prefix = path_prefix.clone();
        let entries = tokio::task::spawn_blocking(move || {
            let _permit = permit;
            index.search(&params.q, prefix.as_deref(), params.limit)
        })
        .await
        .map_err(join_error)?
        .map_err(|error| mcp_error(&error))?;
        let host = &self.state.settings.server.host;
        json_result(McpSearchResults {
            results: entries
                .results
                .into_iter()
                .map(|entry| McpAssetEntry::new(host, entry))
                .collect(),
            truncated: entries.truncated,
            path_prefix,
        })
    }

    /// Read one decoded gamedata file of a version. JSON files are parsed:
    /// `keys_only` lists the children at `pointer` with each child's full
    /// pointer, type and size (page with `offset`/`next_offset`); without
    /// it the value at `pointer` (RFC 6901, e.g. `/char_002_amiya/name`)
    /// is returned, or refused with a size hint when over `max_bytes`.
    /// Other files (story `.txt`, lua) come back as UTF-8 text pages from
    /// byte `offset`. Defaults to the newest version whose gamedata is
    /// ready; the response names the `res_version` read and a public `url`.
    #[tool]
    async fn read_gamedata(
        &self,
        Parameters(params): Parameters<ReadGamedataParams>,
    ) -> Result<CallToolResult, ErrorData> {
        ensure_no_nul("path", &params.path)?;
        if let Some(pointer) = params.pointer.as_deref() {
            ensure_no_nul("pointer", pointer)?;
            if !pointer.is_empty() && !pointer.starts_with('/') {
                return Err(bad_request(
                    "pointer must be empty or start with '/', e.g. /char_002_amiya",
                ));
            }
        }
        let max_bytes = params.max_bytes.unwrap_or(GAMEDATA_DEFAULT_MAX_BYTES);
        if !(GAMEDATA_MIN_MAX_BYTES..=GAMEDATA_MAX_MAX_BYTES).contains(&max_bytes) {
            return Err(bad_request(format!(
                "max_bytes must be between {GAMEDATA_MIN_MAX_BYTES} and {GAMEDATA_MAX_MAX_BYTES}"
            )));
        }
        let res_version = match self.resolve_gamedata_version(&params.version).await? {
            Ok(res_version) => res_version,
            Err(message) => return Ok(not_found(message)),
        };
        // Fail fast like the search gate: the permit moves into the
        // blocking task so it is held until parsing finishes.
        let permit = self
            .gamedata_gate
            .clone()
            .try_acquire_owned()
            .map_err(|_| {
                mcp_error(&AppError::Unavailable(
                    "gamedata read capacity is busy; retry shortly".to_string(),
                ))
            })?;
        let torappu = self.state.torappu.clone();
        let host = self.state.settings.server.host.clone();
        let outcome = tokio::task::spawn_blocking(move || {
            let _permit = permit;
            read_gamedata_file(&torappu, &host, &res_version, &params, max_bytes as usize)
        })
        .await
        .map_err(join_error)?;
        match outcome {
            Ok(response) => json_result(response),
            Err(GamedataError::Miss(message)) => Ok(not_found(message)),
            Err(GamedataError::Failed(error)) => Err(mcp_error(&error)),
        }
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

    /// The gamedata directory `read_gamedata` reads: the selected version,
    /// or (no selector, or `res_version` `latest`) the newest version whose
    /// gamedata is ready. `Err` carries a recoverable miss message.
    async fn resolve_gamedata_version(
        &self,
        version: &VersionSelector,
    ) -> Result<Result<String, String>, ErrorData> {
        if version.version_id.is_some() {
            return Ok(self
                .fetch_version(version)
                .await?
                .map(|row| row.res)
                .ok_or_else(|| version_miss_message(version)));
        }
        if let Some(res_version) = version
            .res_version
            .as_deref()
            .filter(|res| !res.is_empty() && *res != gamedata::LATEST_ALIAS)
        {
            ensure_no_nul("res_version", res_version)?;
            return Ok(Ok(res_version.to_string()));
        }
        let versions = self
            .state
            .database
            .query_versions()
            .await
            .map_err(|error| mcp_error(&error))?;
        let torappu = self.state.torappu.clone();
        let newest = tokio::task::spawn_blocking(move || {
            versions
                .into_iter()
                .rev()
                .map(|version| version.res_version)
                .find(|res| torappu.gamedata_status(res) == GamedataStatus::Ready)
        })
        .await
        .map_err(join_error)?;
        Ok(newest.ok_or_else(|| "no version has finished gamedata extraction yet".to_string()))
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

#[expect(
    clippy::unused_async_trait_impl,
    reason = "rmcp's tool_handler macro generates async trait methods without await"
)]
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

    #[test]
    fn asset_urls_follow_static_routes_and_encode_segments() {
        let host = "https://torappu.prts.wiki/";
        assert_eq!(
            asset_url(
                host,
                "raw/char_arts/illust_char_002_amiya_epoque#4.png",
                false
            )
            .as_deref(),
            Some("https://torappu.prts.wiki/assets/char_arts/illust_char_002_amiya_epoque%234.png")
        );
        assert_eq!(
            asset_url(host, "gamedata/26-10-14/[uc]lua/a b+c.lua", false).as_deref(),
            Some("https://torappu.prts.wiki/gamedata/26-10-14/%5Buc%5Dlua/a%20b%2Bc.lua")
        );
        assert_eq!(
            asset_url(host, "raw/audio/语音.mp3", false).as_deref(),
            Some("https://torappu.prts.wiki/assets/audio/%E8%AF%AD%E9%9F%B3.mp3")
        );
        assert!(
            asset_url(host, "raw/char_arts", true).is_none(),
            "directories"
        );
        assert!(asset_url(host, "raw/", false).is_none());
        assert!(
            asset_url(host, "assetbundle/x.ab", false).is_none(),
            "not served"
        );
    }

    #[test]
    fn mcp_asset_entries_add_url_next_to_entry_fields() {
        let entry = AssetEntry {
            name: "a.png".to_string(),
            path: "raw/a.png".to_string(),
            size: 1,
            create_at: String::new(),
            modified_at: String::new(),
            is_dir: false,
        };
        let json = serde_json::to_value(McpAssetEntry::new("http://h", entry.clone())).unwrap();
        assert_eq!(json["path"], "raw/a.png");
        assert_eq!(json["url"], "http://h/assets/a.png");
        let dir = AssetEntry {
            is_dir: true,
            ..entry
        };
        let json = serde_json::to_value(McpAssetEntry::new("http://h", dir)).unwrap();
        assert!(json.get("url").is_none());
    }

    #[test]
    fn version_summaries_gain_gamedata_status() {
        let json = serde_json::to_value(McpVersionSummary {
            summary: VersionSummary {
                id: 1,
                client_version: "2.7.81".to_string(),
                res_version: "26-10-14".to_string(),
                is_ready: true,
                asset_mapping_status: "ready".to_string(),
            },
            gamedata_status: GamedataStatus::Unmarked,
        })
        .unwrap();
        assert_eq!(json["resVersion"], "26-10-14");
        assert_eq!(json["gamedataStatus"], "unmarked");
    }

    struct GamedataFixture {
        root: std::path::PathBuf,
        torappu: TorappuClient,
    }

    impl GamedataFixture {
        fn new() -> Self {
            let root = std::env::temp_dir().join(format!(
                "ak-mcp-gamedata-{}-{}",
                std::process::id(),
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_nanos()
            ));
            let torappu = TorappuClient {
                asset_base_path: root.join("assets"),
            };
            let write = |path: &str, content: &str| {
                let path = torappu.asset_base_path.join(path);
                std::fs::create_dir_all(path.parent().unwrap()).unwrap();
                std::fs::write(path, content).unwrap();
            };
            write(
                "gamedata/v1/.gamedata-ready.json",
                r#"{"schema_version":1}"#,
            );
            write(
                "gamedata/v1/excel/character_table.json",
                &serde_json::json!({
                    "char_002_amiya": {"name": "阿米娅", "rarity": "TIER_5", "phases": [1, 2, 3]},
                    "char_003_kalts": {"name": "凯尔希", "description": "x".repeat(5000)}
                })
                .to_string(),
            );
            write(
                "gamedata/v1/story/intro.txt",
                "[name=\"阿米娅\"]博士，欢迎回来。",
            );
            std::os::unix::fs::symlink("./v1", torappu.asset_base_path.join("gamedata/latest"))
                .unwrap();
            Self { root, torappu }
        }

        fn read(&self, params: serde_json::Value) -> GamedataResult<serde_json::Value> {
            let params: ReadGamedataParams = serde_json::from_value(params).unwrap();
            let max_bytes = params.max_bytes.unwrap_or(GAMEDATA_DEFAULT_MAX_BYTES) as usize;
            read_gamedata_file(&self.torappu, "http://h", "v1", &params, max_bytes)
                .map(|response| serde_json::to_value(response).unwrap())
        }
    }

    impl Drop for GamedataFixture {
        fn drop(&mut self) {
            std::fs::remove_dir_all(&self.root).unwrap();
        }
    }

    #[test]
    fn read_gamedata_selects_json_parts_by_pointer() {
        let fixture = GamedataFixture::new();
        let response = fixture
            .read(serde_json::json!({
                "path": "./excel/character_table.json",
                "pointer": "/char_002_amiya/name"
            }))
            .unwrap();
        assert_eq!(response["value"], "阿米娅");
        assert_eq!(response["res_version"], "v1");
        assert_eq!(response["gamedata_status"], "ready");
        assert_eq!(response["path"], "excel/character_table.json");
        assert_eq!(
            response["url"],
            "http://h/gamedata/v1/excel/character_table.json"
        );

        let whole = fixture
            .read(serde_json::json!({"path": "excel/character_table.json"}))
            .unwrap();
        assert_eq!(whole["pointer"], "");
        assert_eq!(whole["value"]["char_002_amiya"]["phases"][2], 3);

        assert!(matches!(
            fixture.read(serde_json::json!({
                "path": "excel/character_table.json",
                "pointer": "/char_999"
            })),
            Err(GamedataError::Miss(_))
        ));
    }

    #[test]
    fn read_gamedata_refuses_oversized_values_with_an_outline_hint() {
        let fixture = GamedataFixture::new();
        let Err(GamedataError::Miss(message)) = fixture.read(serde_json::json!({
            "path": "excel/character_table.json",
            "max_bytes": 1000
        })) else {
            panic!("oversized value must be refused");
        };
        assert!(message.contains("an object with 2 keys"), "{message}");
        assert!(message.contains("keys_only"), "{message}");

        let outline = fixture
            .read(serde_json::json!({
                "path": "excel/character_table.json",
                "keys_only": true,
                "max_bytes": 1000
            }))
            .unwrap();
        assert_eq!(outline["outline"]["type"], "object");
        assert_eq!(outline["outline"]["len"], 2);
        let entries = outline["outline"]["entries"].as_array().unwrap();
        assert_eq!(entries[0]["pointer"], "/char_002_amiya");
        assert_eq!(entries[1]["key"], "char_003_kalts");
        assert!(entries[1]["bytes"].as_u64().unwrap() > 5000);

        let second = fixture
            .read(serde_json::json!({
                "path": "excel/character_table.json",
                "pointer": "/char_002_amiya",
                "keys_only": true,
                "offset": 1
            }))
            .unwrap();
        assert_eq!(second["outline"]["offset"], 1);
        // Object keys are listed in sorted order: name, phases, rarity.
        assert_eq!(second["outline"]["entries"][0]["key"], "phases");
    }

    #[test]
    fn read_gamedata_pages_text_files_and_rejects_mismatched_options() {
        let fixture = GamedataFixture::new();
        let text = fixture
            .read(serde_json::json!({"path": "story/intro.txt"}))
            .unwrap();
        assert_eq!(text["text"], "[name=\"阿米娅\"]博士，欢迎回来。");
        assert!(text["next_offset"].is_null());
        assert_eq!(text["url"], "http://h/gamedata/v1/story/intro.txt");

        for params in [
            serde_json::json!({"path": "story/intro.txt", "pointer": "/a"}),
            serde_json::json!({"path": "story/intro.txt", "keys_only": true}),
            serde_json::json!({"path": "excel/character_table.json", "offset": 10}),
        ] {
            assert!(
                matches!(
                    fixture.read(params.clone()),
                    Err(GamedataError::Failed(AppError::InvalidInput(_)))
                ),
                "{params}"
            );
        }
        assert!(matches!(
            fixture.read(serde_json::json!({"path": "story/missing.txt"})),
            Err(GamedataError::Miss(_))
        ));
    }

    #[test]
    fn read_gamedata_params_take_top_level_version_fields() {
        let params: ReadGamedataParams = serde_json::from_value(serde_json::json!({
            "path": "excel/item_table.json",
            "res_version": "latest",
            "keys_only": true
        }))
        .expect("flatten");
        assert_eq!(params.version.res_version.as_deref(), Some("latest"));
        assert_eq!(params.keys_only, Some(true));
    }

    #[test]
    fn search_prefixes_resolve_the_latest_alias() {
        let fixture = GamedataFixture::new();
        let resolve = |prefix| resolve_search_prefix(&fixture.torappu, prefix);
        assert_eq!(resolve("raw/").unwrap(), "raw/");
        assert_eq!(resolve("gamedata/latest/").unwrap(), "gamedata/v1/");
        assert_eq!(resolve("gamedata/latest").unwrap(), "gamedata/v1");
        assert_eq!(
            resolve("gamedata/latest/excel").unwrap(),
            "gamedata/v1/excel"
        );
        assert_eq!(
            resolve("gamedata/latest_backup/").unwrap(),
            "gamedata/latest_backup/",
            "only the alias segment itself is replaced"
        );
        for invalid in ["../secret", "/etc", "./raw", "raw/../gamedata", "raw\0"] {
            assert!(resolve(invalid).is_err(), "{invalid:?}");
        }
        std::fs::remove_file(fixture.torappu.asset_base_path.join("gamedata/latest")).unwrap();
        assert!(resolve("gamedata/latest/").is_err());
    }
}
