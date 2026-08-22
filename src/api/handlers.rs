use crate::{
    api::{
        error::{WebError, WebResult},
        state::AppState,
        types::{
            AssetSearchQuery, BundleListQuery, DockerLaunchRequest, DockerLaunchResponse, Health,
            ManifestChildrenQuery, ManifestDetailQuery, ManifestSearchQuery,
            StoryResourceListQuery, StoryResourceListResponse, StoryResourceSummary,
            StoryResourceUsageItem, StoryResourceUsageQuery, StoryResourceUsageResponse,
        },
        utils::json,
    },
    database::model::{
        AssetMappingDetails, BundleDetails, ManifestNode, VersionDetails, VersionSummary,
    },
    database::row::StoryResourceType,
};
use axum::{
    Json, debug_handler,
    extract::{Path, Query, State},
    http::header,
    response::{IntoResponse, Response},
};
#[debug_handler]
#[utoipa::path(get, path = "/_ping", responses((status = OK, body = Health)))]
pub async fn ping() -> Json<Health> {
    Json(Health { ok: true })
}

#[debug_handler]
#[utoipa::path(get, path = "/_health", responses((status = OK, body = Health)))]
pub async fn health(State(state): State<AppState>) -> Json<Health> {
    Json(Health {
        ok: state.database.health_check().await,
    })
}

#[debug_handler]
#[utoipa::path(get, path = "/version", tag = "version", responses((status = OK, body = [VersionSummary])))]
pub async fn list_version(State(state): State<AppState>) -> WebResult<Response> {
    Ok(json(state.database.query_versions().await?))
}

#[debug_handler]
#[utoipa::path(get, path = "/version/{id}", tag = "version", responses((status = OK, body = VersionDetails)))]
pub async fn get_version(
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> WebResult<Response> {
    let result = state
        .database
        .query_version_detail_by_id(id)
        .await?
        .ok_or(WebError::NotFound)?;
    Ok(json(result))
}

#[debug_handler]
#[utoipa::path(get, path = "/version/{id}/files", tag = "version", responses((status = OK, body = [BundleDetails])))]
pub async fn get_files_by_version(
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> WebResult<Response> {
    Ok(json(state.database.query_bundles_by_version_id(id).await?))
}

#[utoipa::path(get, path = "/bundle/{id}", tag="bundle", responses((status = OK, body = BundleDetails)))]
pub async fn get_bundle(State(state): State<AppState>, Path(id): Path<i32>) -> WebResult<Response> {
    let result = state
        .database
        .query_bundle_by_id_with_details(id)
        .await?
        .ok_or(WebError::NotFound)?;
    Ok(json(result))
}

#[debug_handler]
#[utoipa::path(get, path = "/bundle", tag="bundle", params(BundleListQuery), responses((status = OK, body = [BundleDetails])))]
pub async fn filter_bundle(
    State(state): State<AppState>,
    Query(query): Query<BundleListQuery>,
) -> WebResult<Response> {
    Ok(json(
        state
            .database
            .query_bundles_with_details(&query.into())
            .await?,
    ))
}

#[debug_handler]
#[utoipa::path(get, path = "/manifest/{version_id}/children", tag = "manifest", params(ManifestChildrenQuery), responses((status = OK, body = [ManifestNode])))]
pub async fn list_manifest_children(
    State(state): State<AppState>,
    Path(version_id): Path<i32>,
    Query(params): Query<ManifestChildrenQuery>,
) -> WebResult<Response> {
    let dir = params.dir.unwrap_or_default();
    Ok(json(
        state
            .database
            .list_manifest_children(version_id, &dir)
            .await?,
    ))
}

#[debug_handler]
#[utoipa::path(get, path = "/manifest/{version_id}/detail", tag = "manifest", params(ManifestDetailQuery), responses((status = OK, body = AssetMappingDetails)))]
pub async fn get_manifest_detail(
    State(state): State<AppState>,
    Path(version_id): Path<i32>,
    Query(params): Query<ManifestDetailQuery>,
) -> WebResult<Response> {
    let result = state
        .database
        .get_asset_mapping_detail(version_id, &params.asset_name)
        .await?
        .ok_or(WebError::NotFound)?;
    Ok(json(result))
}

#[debug_handler]
#[utoipa::path(get, path = "/manifest/{version_id}/search", tag = "manifest", params(ManifestSearchQuery), responses((status = OK, body = [ManifestNode])))]
pub async fn search_manifest(
    State(state): State<AppState>,
    Path(version_id): Path<i32>,
    Query(params): Query<ManifestSearchQuery>,
) -> WebResult<Response> {
    Ok(json(
        state
            .database
            .search_manifest(version_id, &params.q)
            .await?,
    ))
}

#[debug_handler]
#[utoipa::path(
    get,
    path = "/item/{item_name}/demand",
    tag = "item",
    responses(
        (status = OK, description = "Item demand found", body = String, content_type = "application/json"),
        (status = NOT_FOUND, description = "Item demand not found")
    )
)]
pub async fn get_item_demand(
    State(state): State<AppState>,
    Path(item_name): Path<String>,
) -> WebResult<Response> {
    let usage = state
        .database
        .query_usage_by_item_name(&item_name)
        .await?
        .ok_or(WebError::NotFound)?;

    Ok(([(header::CONTENT_TYPE, "application/json")], usage).into_response())
}

#[utoipa::path(
    post,
    path = "/docker/launch",
    tag = "docker",
    request_body = DockerLaunchRequest,
    responses(
        (status = 200, description = "Container launched successfully", body = DockerLaunchResponse),
        (status = 401, description = "Unauthorized - invalid or missing authentication token"),
        (status = 400, description = "Bad request - invalid parameters"),
        (status = 500, description = "Internal server error")
    ),
    security(("torappu-auth" = []))
)]
pub async fn launch_container(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    Json(payload): Json<DockerLaunchRequest>,
) -> Result<Json<DockerLaunchResponse>, WebError> {
    let auth_header = headers
        .get("torappu-auth")
        .ok_or(WebError::Unauthorized(
            "Missing torappu-auth header".to_string(),
        ))?
        .to_str()
        .map_err(|_| WebError::Unauthorized("Invalid torappu-auth header format".to_string()))?;

    let expected_token = state.settings.torappu.token.as_str();
    if auth_header != expected_token {
        return Err(WebError::Unauthorized(
            "Invalid authentication token".to_string(),
        ));
    }

    if payload.client_version.is_empty() || payload.res_version.is_empty() {
        return Err(WebError::BadRequest(
            "client_version and res_version cannot be empty".to_string(),
        ));
    }
    if payload.prev_client_version.is_empty() || payload.prev_res_version.is_empty() {
        return Err(WebError::BadRequest(
            "prev_client_version and prev_res_version cannot be empty".to_string(),
        ));
    }

    let docker = state
        .docker
        .as_ref()
        .ok_or(WebError::ServiceUnavailable(anyhow::anyhow!(
            "Docker service is not configured or available"
        )))?;

    let container_name = docker
        .launch_container(
            &payload.client_version,
            &payload.res_version,
            &payload.prev_client_version,
            &payload.prev_res_version,
            payload.include.as_deref().filter(|value| !value.is_empty()),
            payload.exclude.as_deref().filter(|value| !value.is_empty()),
        )
        .await
        .map_err(WebError::from)?;

    Ok(Json(DockerLaunchResponse {
        container_name,
        status: "launched".to_string(),
    }))
}

/// Opaque pagination cursors: base64url(JSON), so a cursor survives being
/// echoed back inside a query string regardless of the characters (`#`, `$`,
/// `/`, spaces) embedded in ids and script paths.
mod cursor {
    use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
    use serde::{Deserialize, Serialize};

    use super::{StoryResourceType, WebError};

    pub(super) trait CursorPayload {
        fn is_valid(&self) -> bool;
    }

    #[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
    pub(super) struct UsageCursor {
        pub script_path: String,
    }

    impl CursorPayload for UsageCursor {
        fn is_valid(&self) -> bool {
            !self.script_path.contains('\0')
        }
    }

    #[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
    pub(super) struct ResourceCursor {
        pub resource_type: StoryResourceType,
        pub resource_id: String,
    }

    impl CursorPayload for ResourceCursor {
        fn is_valid(&self) -> bool {
            !self.resource_id.contains('\0')
        }
    }

    pub(super) fn encode<T: Serialize>(value: &T) -> String {
        let json = serde_json::to_string(value).expect("cursor serialization is infallible");
        URL_SAFE_NO_PAD.encode(json)
    }

    pub(super) fn decode<T: for<'de> Deserialize<'de> + CursorPayload>(
        cursor: Option<&str>,
    ) -> Result<Option<T>, WebError> {
        let Some(cursor) = cursor.filter(|value| !value.is_empty()) else {
            return Ok(None);
        };
        let json = URL_SAFE_NO_PAD
            .decode(cursor)
            .map_err(|_| WebError::BadRequest("invalid cursor".to_string()))?;
        let decoded: T = serde_json::from_slice(&json)
            .map_err(|_| WebError::BadRequest("invalid cursor".to_string()))?;
        if !decoded.is_valid() {
            return Err(WebError::BadRequest("invalid cursor".to_string()));
        }
        Ok(Some(decoded))
    }
}

/// Escapes LIKE metacharacters so `q` matches a literal substring; the
/// default LIKE/ILIKE escape character is the backslash.
fn escape_like(input: &str) -> String {
    let mut escaped = String::with_capacity(input.len());
    for ch in input.chars() {
        if matches!(ch, '%' | '_' | '\\') {
            escaped.push('\\');
        }
        escaped.push(ch);
    }
    escaped
}

#[debug_handler]
#[utoipa::path(
    get,
    path = "/story-resource-usages",
    tag = "story",
    params(StoryResourceUsageQuery),
    responses(
        (status = 200, description = "Scripts using the resource", body = StoryResourceUsageResponse),
        (status = 400, description = "Invalid resource type, id, cursor or limit")
    )
)]
pub async fn get_story_resource_usages(
    State(state): State<AppState>,
    Query(query): Query<StoryResourceUsageQuery>,
) -> WebResult<Response> {
    if query.id.contains('\0') {
        return Err(WebError::BadRequest(
            "id must not contain NUL characters".to_string(),
        ));
    }
    let limit = page_limit(query.limit)?;
    let after = cursor::decode::<cursor::UsageCursor>(query.cursor.as_deref())?;
    let after_path = after.as_ref().map(|c| c.script_path.as_str());

    // Fetch one extra row as a cheap has-next probe; a full page of exactly
    // `limit` rows does not prove another page exists.
    let (usages, next_cursor) =
        if query.resource_type == StoryResourceType::Character && !query.id.contains('#') {
            let rows = state
                .database
                .query_story_character_body_usages(&query.id, after_path, i64::from(limit) + 1)
                .await?;
            let next_cursor = page_cursor(&rows, limit, |row| cursor::UsageCursor {
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
            let rows = state
                .database
                .query_story_resource_usages(
                    query.resource_type,
                    &query.id,
                    after_path,
                    i64::from(limit) + 1,
                )
                .await?;
            let next_cursor = page_cursor(&rows, limit, |row| cursor::UsageCursor {
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

    Ok(json(StoryResourceUsageResponse {
        usages,
        next_cursor,
    }))
}

/// Lists distinct resources with their script counts, keyed by ascending
/// `(resource_type, listing id)` for cursor pagination. `type` filters and
/// `q` substring-matches (case-insensitive) when given. Face-overlay
/// characters list at body granularity (`base$body`, face suffix stripped);
/// standalone full-image characters keep their resolved expression ids.
#[debug_handler]
#[utoipa::path(
    get,
    path = "/story-resources",
    tag = "story",
    params(StoryResourceListQuery),
    responses(
        (status = 200, description = "One page of resources with script counts", body = StoryResourceListResponse),
        (status = 400, description = "Invalid resource type, query, cursor or limit")
    )
)]
pub async fn list_story_resources(
    State(state): State<AppState>,
    Query(query): Query<StoryResourceListQuery>,
) -> WebResult<Response> {
    if query.q.as_deref().is_some_and(|value| value.contains('\0')) {
        return Err(WebError::BadRequest(
            "q must not contain NUL characters".to_string(),
        ));
    }
    let limit = page_limit(query.limit)?;
    let pattern = query
        .q
        .as_deref()
        .filter(|value| !value.is_empty())
        .map(escape_like);
    let after = cursor::decode::<cursor::ResourceCursor>(query.cursor.as_deref())?;

    // Fetch one extra row as a cheap has-next probe; a full page of exactly
    // `limit` rows does not prove another page exists.
    let rows = state
        .database
        .list_story_resources(
            query.resource_type,
            pattern.as_deref(),
            after
                .as_ref()
                .map(|c| (c.resource_type.as_str(), c.resource_id.as_str())),
            i64::from(limit) + 1,
        )
        .await?;

    let next_cursor = page_cursor(&rows, limit, |row| cursor::ResourceCursor {
        resource_type: row.resource_type,
        resource_id: row.resource_id.clone(),
    });
    Ok(json(StoryResourceListResponse {
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
    }))
}

const DEFAULT_PAGE_LIMIT: u32 = 50;
const MAX_PAGE_LIMIT: u32 = 200;

fn page_limit(limit: Option<u32>) -> Result<u32, WebError> {
    let limit = limit.unwrap_or(DEFAULT_PAGE_LIMIT);
    if !(1..=MAX_PAGE_LIMIT).contains(&limit) {
        return Err(WebError::BadRequest(format!(
            "limit must be between 1 and {MAX_PAGE_LIMIT}"
        )));
    }
    Ok(limit)
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

#[debug_handler]
#[utoipa::path(
    get,
    path = "/files",
    tag = "files",
    params(("path" = String, Query, description = "Search path pattern")),
    responses((status = 200, description = "List of matching entries"))
)]
pub async fn search_assets_by_path(
    State(state): State<AppState>,
    Query(AssetSearchQuery { path }): Query<AssetSearchQuery>,
) -> WebResult<Response> {
    Ok(json(state.torappu.search_assets_by_path(&path)?))
}

#[utoipa::path(
    get,
    path = "/files/{path}",
    tag = "files",
    params(("path" = String, Path, description = "Directory path to list")),
    responses((status = 200, description = "Directory listing"))
)]
pub async fn list_asset(
    State(state): State<AppState>,
    Path(path): Path<String>,
) -> WebResult<Response> {
    Ok(json(state.torappu.list_asset(&path)?))
}

pub async fn list_root_asset(State(state): State<AppState>) -> WebResult<Response> {
    Ok(json(state.torappu.list_asset("")?))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cursor_round_trips_ids_with_special_characters() {
        let cursor = cursor::ResourceCursor {
            resource_type: StoryResourceType::Character,
            resource_id: "avg_npc_009#1$1".to_string(),
        };
        let encoded = cursor::encode(&cursor);
        assert!(
            encoded
                .chars()
                .all(|ch| ch.is_ascii_alphanumeric() || ch == '-' || ch == '_')
        );
        assert_eq!(
            cursor::decode::<cursor::ResourceCursor>(Some(&encoded)).expect("decode"),
            Some(cursor)
        );
    }

    #[test]
    fn cursor_decode_rejects_garbage_and_treats_absent_as_none() {
        assert!(cursor::decode::<cursor::UsageCursor>(Some("not base64url!!")).is_err());
        assert_eq!(
            cursor::decode::<cursor::UsageCursor>(None).expect("decode"),
            None
        );
        assert_eq!(
            cursor::decode::<cursor::UsageCursor>(Some("")).expect("decode"),
            None
        );
        // A cursor from the other mode must not decode into this shape.
        let encoded = cursor::encode(&cursor::ResourceCursor {
            resource_type: StoryResourceType::Image,
            resource_id: "ac1_0".to_string(),
        });
        assert!(cursor::decode::<cursor::UsageCursor>(Some(&encoded)).is_err());
    }

    #[test]
    fn cursor_decode_rejects_nul_fields() {
        let encoded = cursor::encode(&cursor::UsageCursor {
            script_path: "bad\0path".to_string(),
        });
        assert!(cursor::decode::<cursor::UsageCursor>(Some(&encoded)).is_err());

        let encoded = cursor::encode(&cursor::ResourceCursor {
            resource_type: StoryResourceType::Image,
            resource_id: "bad\0id".to_string(),
        });
        assert!(cursor::decode::<cursor::ResourceCursor>(Some(&encoded)).is_err());
    }

    #[test]
    fn escape_like_neutralizes_metacharacters() {
        assert_eq!(escape_like("ac1_0"), "ac1\\_0");
        assert_eq!(escape_like("100%"), "100\\%");
        assert_eq!(escape_like(r"back\slash"), r"back\\slash");
        assert_eq!(escape_like("bgmed"), "bgmed");
    }
}
