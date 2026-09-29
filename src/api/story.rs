//! Story resource listing/usage pagination shared by the REST handlers and
//! the MCP tools, so the page-size rules, the has-next probe and the
//! character body-granularity special case cannot drift apart.

use crate::{
    AppError, AppResult,
    api::{
        cursor::{self, UsageCursor},
        types::{
            StoryResourceListResponse, StoryResourceSummary, StoryResourceUsageItem,
            StoryResourceUsageResponse,
        },
    },
    database::{Database, row::StoryResourceType},
};
use serde::Serialize;

pub const DEFAULT_PAGE_LIMIT: u32 = 50;
pub const MAX_PAGE_LIMIT: u32 = 200;

/// Page size for a list endpoint: defaults to `DEFAULT_PAGE_LIMIT`, bounded
/// to `1..=MAX_PAGE_LIMIT`.
pub fn page_limit(limit: Option<u32>) -> Result<u32, AppError> {
    let limit = limit.unwrap_or(DEFAULT_PAGE_LIMIT);
    if !(1..=MAX_PAGE_LIMIT).contains(&limit) {
        return Err(AppError::InvalidInput(format!(
            "limit must be between 1 and {MAX_PAGE_LIMIT}"
        )));
    }
    Ok(limit)
}

/// Builds the opaque cursor for the next page from an over-fetched page of
/// `limit + 1` rows; `None` when the page is the last one.
pub fn page_cursor<R, C: Serialize>(
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

/// One page of `list_story_resources`. Fetches `limit + 1` rows as a
/// cheap has-next probe and returns the response with the next-page cursor.
///
/// `id_pattern` is an already LIKE-escaped substring (`None` lists
/// everything); `after` is an exclusive keyset bound `(resource_type,
/// listing id)` (`None` starts from the beginning).
pub async fn list_story_resources_page(
    database: &Database,
    resource_type: Option<StoryResourceType>,
    id_pattern: Option<&str>,
    after: Option<(&str, &str)>,
    limit: u32,
) -> AppResult<StoryResourceListResponse> {
    let rows = database
        .list_story_resources(resource_type, id_pattern, after, i64::from(limit) + 1)
        .await?;
    let next_cursor = page_cursor(&rows, limit, |row| cursor::ResourceCursor {
        resource_type: row.resource_type,
        resource_id: row.resource_id.clone(),
    });
    Ok(StoryResourceListResponse {
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
    })
}

/// One page of the resource reverse lookup. A character id without `#` is
/// the body form (`base$body`): scripts using any expression of the body
/// collapse into one row with the union of display names and the resolved
/// expression ids each script uses.
///
/// `after_path` is an exclusive keyset bound (script path); `None` starts
/// from the beginning.
pub async fn story_resource_usages_page(
    database: &Database,
    resource_type: StoryResourceType,
    resource_id: &str,
    after_path: Option<&str>,
    limit: u32,
) -> AppResult<StoryResourceUsageResponse> {
    let (usages, next_cursor) =
        if resource_type == StoryResourceType::Character && !resource_id.contains('#') {
            let rows = database
                .query_story_character_body_usages(resource_id, after_path, i64::from(limit) + 1)
                .await?;
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
            let rows = database
                .query_story_resource_usages(
                    resource_type,
                    resource_id,
                    after_path,
                    i64::from(limit) + 1,
                )
                .await?;
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
    Ok(StoryResourceUsageResponse {
        usages,
        next_cursor,
    })
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
    fn page_cursor_emits_cursor_only_when_more_pages() {
        let rows = vec!["a", "b", "c"];
        assert!(page_cursor(&rows, 2, |row| row.to_string()).is_some());
        assert!(page_cursor(&rows, 3, |row| row.to_string()).is_none());
    }
}
