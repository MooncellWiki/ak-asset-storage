//! MCP (Model Context Protocol) server: read-only query tools exposing the
//! same data as the REST API (versions, manifest tree, bundles, story
//! resources, raw asset directory) to AI clients over Streamable HTTP at
//! `/mcp`.

use crate::{
    AppError,
    api::{
        cursor::{self, ResourceCursor, UsageCursor},
        error::WebError,
        handlers::token_matches,
        state::AppState,
        types::{
            StoryResourceListResponse, StoryResourceSummary, StoryResourceUsageItem,
            StoryResourceUsageResponse,
        },
        utils::escape_like,
    },
    database::{bundle::BundleFilter, row::StoryResourceType},
};
use axum::{
    extract::{Request, State},
    http::{StatusCode, header},
    middleware::Next,
    response::{IntoResponse, Response},
};
use rmcp::{
    ErrorData, ServerHandler,
    handler::server::{router::tool::ToolRouter, wrapper::Parameters},
    model::{CallToolResult, ContentBlock},
    schemars::JsonSchema,
    tool, tool_handler, tool_router,
};
use serde::{Deserialize, Serialize};

const DEFAULT_PAGE_LIMIT: u32 = 50;
const MAX_PAGE_LIMIT: u32 = 200;

const INSTRUCTIONS: &str = "\
This server indexes Arknights game assets.
- Every game `version` (a client/res version pair) owns `bundles` (asset
  bundle files identified by path, hash and size) and a manifest tree that
  maps each asset name to the bundle containing it.
- Call `list_versions` first to discover versions; version-aware tools accept
  `version_id` or `res_version` and default to the latest version.
- `search_manifest` finds which bundle contains an asset; asset names follow
  game conventions such as `chararts/...`, `avg_npc_009#1` (the `#` suffix is
  an expression/face id) or `b_ac1_0` for audio.
- Story tools index story scripts: `list_story_resources` lists resources of
  one type (`background`, `image`, `item`, `character`) and
  `get_story_resource_usages` finds the scripts that use one. Characters come
  in two id forms: `base#expression` for one expression, `base$body` (no `#`)
  for every expression of the body.
- `list_files`/`search_files` browse the extracted raw asset directory.
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
/// need a version fall back to the latest one.
#[derive(Deserialize, JsonSchema)]
struct VersionSelector {
    /// Numeric version id from `list_versions`. Takes precedence over `res_version`.
    version_id: Option<i32>,
    /// Resource version string from `list_versions`, e.g. `24-10-08-...`.
    res_version: Option<String>,
}

#[derive(Deserialize, JsonSchema)]
struct SearchManifestParams {
    /// Case-insensitive substring of the asset name, e.g. `avg_npc_009` or `chararts`.
    q: String,
    #[serde(flatten)]
    version: VersionSelector,
}

#[derive(Deserialize, JsonSchema)]
struct ListManifestChildrenParams {
    /// Parent directory path from a previous `list_manifest_children` or
    /// `search_manifest` result; empty or omitted lists the manifest root.
    dir: Option<String>,
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
    /// Substring matched against bundle paths.
    path: Option<String>,
    /// Exact bundle file hash.
    hash: Option<String>,
    /// Bundle file id.
    file_id: Option<i32>,
    /// Restrict to one version; omit to search all versions.
    #[serde(flatten)]
    version: VersionSelector,
    /// Maximum number of bundles returned (1-200, default 50).
    limit: Option<u32>,
}

#[derive(Deserialize, JsonSchema)]
struct ListFilesParams {
    /// Directory below the raw asset root, e.g. `raw/chararts`; empty or
    /// omitted lists the root. Must be relative without `..`.
    path: Option<String>,
    /// Maximum number of entries returned (1-200, default 50).
    limit: Option<u32>,
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
    /// Exact resource id. Characters accept `base#expression` for one
    /// expression or `base$body` (no `#`) for every expression of the body.
    id: String,
    /// Opaque cursor from a previous response (`next_cursor`).
    cursor: Option<String>,
    /// Page size (1-200, default 50).
    limit: Option<u32>,
}

#[derive(Deserialize, JsonSchema)]
struct GetItemDemandParams {
    /// Item name, e.g. `2001` for a material id.
    item_name: String,
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

fn mcp_error(error: &AppError) -> ErrorData {
    ErrorData::internal_error(error.to_string(), None)
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

fn page_limit(limit: Option<u32>) -> Result<u32, ErrorData> {
    let limit = limit.unwrap_or(DEFAULT_PAGE_LIMIT);
    if !(1..=MAX_PAGE_LIMIT).contains(&limit) {
        return Err(bad_request(format!(
            "limit must be between 1 and {MAX_PAGE_LIMIT}"
        )));
    }
    Ok(limit)
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

fn decode_cursor<T: for<'de> Deserialize<'de> + cursor::CursorPayload>(
    cursor_value: Option<&str>,
) -> Result<Option<T>, ErrorData> {
    cursor::decode(cursor_value).map_err(|err: WebError| bad_request(err.to_string()))
}

#[tool_router]
impl AkAssetMcpServer {
    /// List every game resource version with its numeric id, client/res
    /// version strings, readiness and manifest import status. Start here to
    /// pick a version for other tools.
    #[tool]
    async fn list_versions(&self) -> Result<CallToolResult, ErrorData> {
        let versions = self
            .state
            .database
            .query_versions()
            .await
            .map_err(|error| mcp_error(&error))?;
        json_result(versions)
    }

    /// Get one version's details, including its hot-update list.
    #[tool]
    async fn get_version(
        &self,
        Parameters(version): Parameters<VersionSelector>,
    ) -> Result<CallToolResult, ErrorData> {
        let id = self.resolve_version_id(&version).await?;
        let details = self
            .state
            .database
            .query_version_detail_by_id(id)
            .await
            .map_err(|error| mcp_error(&error))?
            .ok_or_else(|| {
                bad_request(format!(
                    "version {id} not found; call list_versions for ids"
                ))
            })?;
        json_result(details)
    }

    /// Search manifest entries by asset name substring and get the bundle
    /// each asset lives in. The primary way to locate an asset.
    #[tool]
    async fn search_manifest(
        &self,
        Parameters(params): Parameters<SearchManifestParams>,
    ) -> Result<CallToolResult, ErrorData> {
        ensure_no_nul("q", &params.q)?;
        let version_id = self.resolve_version_id(&params.version).await?;
        let nodes = self
            .state
            .database
            .search_manifest(version_id, &params.q)
            .await
            .map_err(|error| mcp_error(&error))?;
        json_result(TruncatedResults::new(nodes, MAX_PAGE_LIMIT))
    }

    /// Browse the manifest directory tree one level at a time: given a
    /// directory path, list its files and subdirectories.
    #[tool]
    async fn list_manifest_children(
        &self,
        Parameters(params): Parameters<ListManifestChildrenParams>,
    ) -> Result<CallToolResult, ErrorData> {
        let limit = page_limit(params.limit)?;
        let dir = params.dir.unwrap_or_default();
        ensure_no_nul("dir", &dir)?;
        let version_id = self.resolve_version_id(&params.version).await?;
        let nodes = self
            .state
            .database
            .list_manifest_children(version_id, &dir)
            .await
            .map_err(|error| mcp_error(&error))?;
        json_result(TruncatedResults::new(nodes, limit))
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
        let version_id = self.resolve_version_id(&params.version).await?;
        let Some(detail) = self
            .state
            .database
            .get_asset_mapping_detail(version_id, &asset_name)
            .await
            .map_err(|error| mcp_error(&error))?
        else {
            return Ok(not_found(format!(
                "no manifest entry {asset_name:?} in this version; try search_manifest"
            )));
        };
        json_result(detail)
    }

    /// Filter bundles by path substring, exact hash, file id and/or version.
    /// At least one of `path`, `hash`, `file_id` or a version should be
    /// given, otherwise every bundle in the database matches.
    #[tool]
    async fn search_bundles(
        &self,
        Parameters(params): Parameters<SearchBundlesParams>,
    ) -> Result<CallToolResult, ErrorData> {
        let limit = page_limit(params.limit)?;
        if let Some(path) = params.path.as_deref() {
            ensure_no_nul("path", path)?;
        }
        let version_id = self.resolve_optional_version_id(&params.version).await?;
        let bundles = self
            .state
            .database
            .query_bundles_with_details(&BundleFilter {
                path: params.path,
                hash: params.hash,
                file: params.file_id,
                version: version_id,
            })
            .await
            .map_err(|error| mcp_error(&error))?;
        json_result(TruncatedResults::new(bundles, limit))
    }

    /// List the entries of one directory in the extracted raw asset tree,
    /// e.g. `raw/chararts`. Omit `path` to list the root.
    #[tool]
    async fn list_files(
        &self,
        Parameters(params): Parameters<ListFilesParams>,
    ) -> Result<CallToolResult, ErrorData> {
        let limit = page_limit(params.limit)?;
        let path = params.path.unwrap_or_default();
        let dir = self
            .state
            .torappu
            .list_asset(&path)
            .map_err(|error| mcp_error(&error))?;
        json_result(TruncatedResults::new(dir.children, limit))
    }

    /// Search the extracted raw asset tree by path substring. Prefer this
    /// over walking directories with `list_files` when hunting one file.
    #[tool]
    async fn search_files(
        &self,
        Parameters(params): Parameters<SearchFilesParams>,
    ) -> Result<CallToolResult, ErrorData> {
        let limit = page_limit(params.limit)?;
        ensure_no_nul("q", &params.q)?;
        let entries = self
            .state
            .torappu
            .search_assets_by_path(&params.q)
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
        let limit = page_limit(params.limit)?;
        if let Some(q) = params.q.as_deref() {
            ensure_no_nul("q", q)?;
        }
        let pattern = params
            .q
            .as_deref()
            .filter(|value| !value.is_empty())
            .map(escape_like);
        let after = decode_cursor::<ResourceCursor>(params.cursor.as_deref())?;

        // Fetch one extra row as a cheap has-next probe; a full page of
        // exactly `limit` rows does not prove another page exists.
        let rows = self
            .state
            .database
            .list_story_resources(
                resource_type,
                pattern.as_deref(),
                after
                    .as_ref()
                    .map(|cursor| (cursor.resource_type.as_str(), cursor.resource_id.as_str())),
                i64::from(limit) + 1,
            )
            .await
            .map_err(|error| mcp_error(&error))?;

        let next_cursor = page_cursor(&rows, limit, |row| ResourceCursor {
            resource_type: row.resource_type,
            resource_id: row.resource_id.clone(),
        });
        let response = StoryResourceListResponse {
            resources: rows
                .into_iter()
                .take(limit as usize)
                .map(|row| StoryResourceSummary {
                    resource_type: row.resource_type,
                    id: row.resource_id,
                    script_count: row.script_count,
                })
                .collect(),
            next_cursor,
        };
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
        let limit = page_limit(params.limit)?;
        ensure_no_nul("id", &params.id)?;
        let after = decode_cursor::<UsageCursor>(params.cursor.as_deref())?;
        let after_path = after.as_ref().map(|cursor| cursor.script_path.as_str());

        // Same character special case as the REST endpoint: a body-form id
        // (no `#`) aggregates every expression of the body.
        let (usages, next_cursor) =
            if resource_type == StoryResourceType::Character && !params.id.contains('#') {
                let rows = self
                    .state
                    .database
                    .query_story_character_body_usages(&params.id, after_path, i64::from(limit) + 1)
                    .await
                    .map_err(|error| mcp_error(&error))?;
                let next_cursor = page_cursor(&rows, limit, |row| UsageCursor {
                    script_path: row.script_path.clone(),
                });
                let usages = rows
                    .into_iter()
                    .take(limit as usize)
                    .map(|row| StoryResourceUsageItem {
                        script_path: row.script_path,
                        display_names: row.display_names,
                        faces: Some(row.faces),
                    })
                    .collect();
                (usages, next_cursor)
            } else {
                let rows = self
                    .state
                    .database
                    .query_story_resource_usages(
                        resource_type,
                        &params.id,
                        after_path,
                        i64::from(limit) + 1,
                    )
                    .await
                    .map_err(|error| mcp_error(&error))?;
                let next_cursor = page_cursor(&rows, limit, |row| UsageCursor {
                    script_path: row.script_path.clone(),
                });
                let usages = rows
                    .into_iter()
                    .take(limit as usize)
                    .map(|row| StoryResourceUsageItem {
                        script_path: row.script_path,
                        display_names: row.display_names,
                        faces: None,
                    })
                    .collect();
                (usages, next_cursor)
            };
        json_result(StoryResourceUsageResponse {
            usages,
            next_cursor,
        })
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

    /// Resolves the version selector to a database id, defaulting to the
    /// latest version when no selector is given.
    async fn resolve_version_id(&self, version: &VersionSelector) -> Result<i32, ErrorData> {
        if let Some(id) = version.version_id {
            return Ok(id);
        }
        if let Some(res_version) = version.res_version.as_deref() {
            return self.resolve_res_version(res_version).await;
        }
        self.state
            .database
            .get_latest_version()
            .await
            .map_err(|error| mcp_error(&error))?
            .and_then(|row| row.id)
            .ok_or_else(|| {
                bad_request("no version available yet; import a manifest first".to_string())
            })
    }

    /// Like `resolve_version_id`, but returns `None` when no selector was
    /// given (queries spanning all versions).
    async fn resolve_optional_version_id(
        &self,
        version: &VersionSelector,
    ) -> Result<Option<i32>, ErrorData> {
        if let Some(id) = version.version_id {
            return Ok(Some(id));
        }
        match version.res_version.as_deref() {
            Some(res_version) => self.resolve_res_version(res_version).await.map(Some),
            None => Ok(None),
        }
    }

    async fn resolve_res_version(&self, res_version: &str) -> Result<i32, ErrorData> {
        ensure_no_nul("res_version", res_version)?;
        self.state
            .database
            .get_version_by_res(res_version)
            .await
            .map_err(|error| mcp_error(&error))?
            .and_then(|row| row.id)
            .ok_or_else(|| {
                bad_request(format!(
                    "no version with res_version {res_version:?}; call list_versions"
                ))
            })
    }
}

/// Builds the opaque cursor for the next page from an over-fetched page of
/// `limit + 1` rows; `None` when the page is the last one.
fn page_cursor<R, C: serde::Serialize>(
    rows: &[R],
    limit: u32,
    make: impl Fn(&R) -> C,
) -> Option<String> {
    if rows.len() <= limit as usize {
        return None;
    }
    rows.get(limit as usize - 1)
        .map(|row| cursor::encode(&make(row)))
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

/// Bearer-token guard for `/mcp`. Unconfigured (or empty) tokens leave the
/// endpoint open — the tools only mirror the already-public read API.
pub async fn auth_middleware(
    State(state): State<AppState>,
    request: Request,
    next: Next,
) -> Response {
    let Some(expected) = state
        .settings
        .mcp
        .auth_token
        .as_deref()
        .filter(|token| !token.is_empty())
    else {
        return next.run(request).await;
    };
    let authorized = request
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.strip_prefix("Bearer "))
        .is_some_and(|provided| token_matches(provided, expected));
    if authorized {
        next.run(request).await
    } else {
        StatusCode::UNAUTHORIZED.into_response()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn page_limit_defaults_and_bounds() {
        assert_eq!(page_limit(None).expect("default"), DEFAULT_PAGE_LIMIT);
        assert_eq!(page_limit(Some(1)).expect("min"), 1);
        assert_eq!(
            page_limit(Some(MAX_PAGE_LIMIT)).expect("max"),
            MAX_PAGE_LIMIT
        );
        assert!(page_limit(Some(0)).is_err());
        assert!(page_limit(Some(MAX_PAGE_LIMIT + 1)).is_err());
    }

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
    fn version_selector_flattens_into_tool_arguments() {
        let params: SearchManifestParams = serde_json::from_value(serde_json::json!({
            "q": "chararts",
            "res_version": "24-10-08"
        }))
        .expect("flatten");
        assert_eq!(params.q, "chararts");
        assert_eq!(
            params.version.res_version.as_deref(),
            Some("24-10-08".trim())
        );
        assert!(params.version.version_id.is_none());
    }

    #[test]
    fn page_cursor_emits_cursor_only_when_more_pages() {
        let rows = vec!["a", "b", "c"];
        assert!(page_cursor(&rows, 2, |row| row.to_string()).is_some());
        assert!(page_cursor(&rows, 3, |row| row.to_string()).is_none());
    }
}
