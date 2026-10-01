use crate::{
    AppError,
    api::{
        cursor,
        error::{WebError, WebResult},
        state::AppState,
        story,
        types::{
            AssetSearchQuery, BundleListQuery, BundleListResponse, DockerLaunchRequest,
            DockerLaunchResponse, Health, ManifestChildrenQuery, ManifestDetailQuery,
            ManifestSearchQuery, StoryResourceListQuery, StoryResourceListResponse,
            StoryResourceUsageQuery, StoryResourceUsageResponse,
        },
        utils::{escape_like, json},
    },
    database::model::{
        AssetMappingDetails, BundleDetails, ManifestNode, VersionDetails, VersionSummary,
    },
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
#[utoipa::path(
    get,
    path = "/bundle",
    tag = "bundle",
    params(BundleListQuery),
    responses(
        (status = 200, description = "One page of matches, newest version first, then path ascending; pass nextCursor back as cursor to continue", body = BundleListResponse),
        (status = 400, description = "No filter provided, or invalid limit/cursor")
    )
)]
pub async fn filter_bundle(
    State(state): State<AppState>,
    Query(query): Query<BundleListQuery>,
) -> WebResult<Response> {
    // An empty filter would dump the whole table; reject it like the MCP
    // tool does (blank path/hash count as absent).
    if !query.has_condition() {
        return Err(WebError::BadRequest(
            "provide at least one of path, hash, file or version".to_string(),
        ));
    }
    let limit = story::page_limit(query.limit).map_err(WebError::from)?;
    let filter = query.normalized();
    let after = cursor::decode::<cursor::BundleCursor>(query.cursor.as_deref())?;
    if let Some(after) = &after {
        // The keyset bound is only valid for the filter that produced it;
        // mixing a stale cursor into changed conditions would silently
        // skip or empty the result set.
        if after.filter != filter {
            return Err(WebError::BadRequest(
                "cursor does not match the current filter; search again without a cursor"
                    .to_string(),
            ));
        }
    }
    // limit + 1 rows: the extra row only probes whether a next page exists
    // and never reaches the response.
    let mut rows = state
        .database
        .query_bundles_with_details_page(
            &filter,
            after
                .as_ref()
                .map(|cursor| (cursor.version_id, cursor.path.as_str(), cursor.id)),
            i64::from(limit) + 1,
        )
        .await?;
    let next_cursor = story::page_cursor(&rows, limit, |row| cursor::BundleCursor {
        filter: filter.clone(),
        version_id: row.version_id,
        path: row.path.clone(),
        id: row.id,
    });
    rows.truncate(limit as usize);
    Ok(json(BundleListResponse {
        bundles: rows,
        next_cursor,
    }))
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
            .search_manifest(
                version_id,
                &escape_like(&params.q),
                i64::from(story::MAX_PAGE_LIMIT),
            )
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

    if !token_matches(auth_header, &state.settings.torappu.token) {
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

/// Constant-time token comparison so response timing does not leak how many
/// leading bytes of a guess were correct. Length is compared first because
/// `ct_eq` only accepts equal-length slices; an empty configured token is
/// rejected at config load time.
fn token_matches(provided: &str, expected: &str) -> bool {
    use subtle::ConstantTimeEq;
    let provided = provided.as_bytes();
    let expected = expected.as_bytes();
    !expected.is_empty() && provided.len() == expected.len() && bool::from(provided.ct_eq(expected))
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
    let limit = story::page_limit(query.limit).map_err(WebError::from)?;
    let after = cursor::decode::<cursor::UsageCursor>(query.cursor.as_deref())?;
    let after_path = after.as_ref().map(|c| c.script_path.as_str());

    let response = story::story_resource_usages_page(
        &state.database,
        query.resource_type,
        &query.id,
        after_path,
        limit,
    )
    .await?;
    Ok(json(response))
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
    let limit = story::page_limit(query.limit).map_err(WebError::from)?;
    let pattern = query
        .q
        .as_deref()
        .filter(|value| !value.is_empty())
        .map(escape_like);
    let after = cursor::decode::<cursor::ResourceCursor>(query.cursor.as_deref())?;

    let response = story::list_story_resources_page(
        &state.database,
        query.resource_type,
        pattern.as_deref(),
        after
            .as_ref()
            .map(|c| (c.resource_type.as_str(), c.resource_id.as_str())),
        limit,
    )
    .await?;
    Ok(json(response))
}

#[debug_handler]
#[utoipa::path(
    get,
    path = "/files",
    tag = "files",
    params(AssetSearchQuery),
    responses(
        (status = 200, description = "Matching entries; narrow the query when truncated", body = crate::external::types::AssetSearchResults),
        (status = 400, description = "Invalid query or limit"),
        (status = 503, description = "Search busy or index preparing")
    )
)]
pub async fn search_assets_by_path(
    State(state): State<AppState>,
    Query(AssetSearchQuery { path, limit }): Query<AssetSearchQuery>,
) -> WebResult<Response> {
    // Reject before any work when the shared search budget is exhausted;
    // the permit moves into the blocking task so it is held until the
    // search finishes (issue #177).
    let permit = state.search_gate.try_acquire()?;
    let index = state.plocate.clone();
    let entries = tokio::task::spawn_blocking(move || {
        let _permit = permit;
        index.search(&path, limit)
    })
    .await
    .map_err(|err| WebError::CustomApiError(AppError::Application(err.into())))?
    .map_err(WebError::from)?;
    Ok(json(entries))
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
    use crate::database::row::StoryResourceType;

    fn cursor_filter(path: &str) -> crate::database::bundle::BundleFilter {
        crate::database::bundle::BundleFilter {
            path: Some(path.to_string()),
            hash: None,
            file: None,
            version: None,
        }
    }

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
    fn bundle_cursor_round_trips_and_rejects_nul_paths() {
        let filter = cursor_filter("ab_avg");
        let cursor = cursor::BundleCursor {
            filter: filter.clone(),
            version_id: 7,
            path: "ab_avg/avg_1#1$1.bundle".to_string(),
            id: 42,
        };
        let encoded = cursor::encode(&cursor);
        assert_eq!(
            cursor::decode::<cursor::BundleCursor>(Some(&encoded)).expect("decode"),
            Some(cursor)
        );
        // A cursor issued for one filter must not pass as another filter's
        // continuation (checked in `filter_bundle` via PartialEq).
        assert_ne!(filter, cursor_filter("char_1000"));

        let encoded = cursor::encode(&cursor::BundleCursor {
            filter,
            version_id: 7,
            path: "bad\0path".to_string(),
            id: 42,
        });
        assert!(cursor::decode::<cursor::BundleCursor>(Some(&encoded)).is_err());

        let encoded = cursor::encode(&cursor::BundleCursor {
            filter: cursor_filter("bad\0filter"),
            version_id: 7,
            path: "path".to_string(),
            id: 42,
        });
        assert!(cursor::decode::<cursor::BundleCursor>(Some(&encoded)).is_err());
    }

    #[test]
    fn token_matches_requires_exact_non_empty_match() {
        assert!(token_matches("secret", "secret"));
        assert!(!token_matches("secret", "secret2"));
        assert!(!token_matches("secre", "secret"));
        assert!(!token_matches("Secret", "secret"));
        assert!(!token_matches("", ""));
        assert!(!token_matches("", "secret"));
    }

    #[test]
    fn escape_like_neutralizes_metacharacters() {
        assert_eq!(escape_like("ac1_0"), "ac1\\_0");
        assert_eq!(escape_like("100%"), "100\\%");
        assert_eq!(escape_like(r"back\slash"), r"back\\slash");
        assert_eq!(escape_like("bgmed"), "bgmed");
    }
}
