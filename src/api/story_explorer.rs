//! A complete browse response: callers never need to collect the catalog to
//! search, sort, group, count facets, jump pages, or open a shared resource.
use std::{collections::HashSet, sync::LazyLock};

use axum::{
    extract::{Query, State},
    http::{HeaderValue, header::CACHE_CONTROL},
    response::Response,
};
use regex::Regex;
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};

use crate::{
    api::{
        AppState,
        error::{WebError, WebResult},
        utils::json,
    },
    database::{row::StoryResourceType, story_explorer::StoryExploreOptions},
};

#[derive(Debug, Deserialize, IntoParams)]
#[serde(rename_all = "camelCase")]
pub struct ExploreQuery {
    #[serde(rename = "type")]
    pub resource_type: StoryResourceType,
    /// Case-insensitive AND keywords; accepts script commands and wiki filenames.
    pub q: Option<String>,
    pub category: Option<CharacterCategory>,
    /// Defaults to base. Merges only characters, before counting/paging.
    #[param(inline)]
    pub group_by: Option<GroupBy>,
    #[param(inline)]
    pub sort: Option<Sort>,
    #[param(inline)]
    pub order: Option<Order>,
    /// 1-based, clamped to the last page. Defaults to 1.
    pub page: Option<u32>,
    /// Defaults to 60, maximum 500.
    pub page_size: Option<u32>,
    /// Exact listing id to open. Locates its page when it matches the filters;
    /// otherwise returns selection separately without changing the filters.
    pub focus: Option<String>,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum CharacterCategory {
    Operator,
    Npc,
    Other,
}

impl CharacterCategory {
    const fn as_str(self) -> &'static str {
        match self {
            Self::Operator => "operator",
            Self::Npc => "npc",
            Self::Other => "other",
        }
    }
}

#[derive(Debug, Deserialize, ToSchema, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum GroupBy {
    Resource,
    Base,
}

#[derive(Debug, Deserialize, ToSchema, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum Sort {
    Id,
    ScriptCount,
}

#[derive(Debug, Deserialize, ToSchema, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Order {
    Asc,
    Desc,
}

#[derive(Debug, Deserialize, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ExploreResource {
    #[serde(rename = "type")]
    pub resource_type: StoryResourceType,
    pub id: String,
    pub base: Option<String>,
    pub category: Option<CharacterCategory>,
    pub script_count: i64,
}

#[derive(Debug, Deserialize, Serialize, ToSchema)]
pub struct ExploreEntry {
    pub resource: ExploreResource,
    /// All indexed variants of this base, including those not matching q.
    pub members: Vec<ExploreResource>,
    pub merged: i64,
    /// Distinct scripts across the whole group, never sum/max of variants.
    pub count: i64,
    pub face: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, ToSchema)]
pub struct TypeCounts {
    pub character: i64,
    pub background: i64,
    pub image: i64,
    pub item: i64,
}

#[derive(Debug, Deserialize, Serialize, ToSchema)]
pub struct CategoryCounts {
    pub operator: i64,
    pub npc: i64,
    pub other: i64,
}

#[derive(Debug, Deserialize, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ExploreResponse {
    pub entries: Vec<ExploreEntry>,
    pub selection: Option<ExploreEntry>,
    /// Zero-based position within the filtered results, if focus matches.
    pub focus_index: Option<i64>,
    /// Current type, with grouping but without q/category filters.
    pub total: i64,
    pub matched_total: i64,
    /// Matching counts ignore selected type; category affects characters only.
    pub counts: TypeCounts,
    /// Matching character counts ignore the selected category.
    pub category_counts: CategoryCounts,
    pub page: i64,
    pub page_size: i64,
    pub page_count: i64,
    pub updated_at: Option<String>,
}

fn normalize_query(raw: &str) -> String {
    static QUOTED: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r#"(?i)\b(?:name|image|key|res)\s*=\s*"([^"]+)""#).expect("regex")
    });
    static PREFIX: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"(?i)^(?:文件|file|image)\s*:\s*").expect("regex"));
    static EXT: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"(?i)\.(?:png|jpe?g|webp)$").expect("regex"));
    let quoted = QUOTED.captures(raw);
    let text = quoted
        .as_ref()
        .map_or(raw, |c| c.get(1).expect("capture").as_str())
        .trim();
    let text = if PREFIX.is_match(text) || EXT.is_match(text) {
        let text = PREFIX.replace(text, "");
        let text = EXT.replace(&text, "").replace(' ', "_").to_lowercase();
        text.strip_prefix("avg_").unwrap_or(&text).to_string()
    } else {
        text.to_lowercase()
    };
    text.replace('-', "#")
}

/// Every term is a substring scan over the whole catalog, so a public request
/// must not be able to multiply the query cost with thousands of short terms.
const MAX_SEARCH_TERMS: usize = 32;

/// The catalog only changes on import. The deployment sits behind a CDN and
/// this response varies by query string, so only the browser may store it.
const CACHE_POLICY: HeaderValue = HeaderValue::from_static("private, max-age=60");

fn search_terms(raw: &str) -> Vec<String> {
    let normalized = normalize_query(raw);
    let mut seen = HashSet::new();
    normalized
        .split_whitespace()
        .filter(|term| seen.insert(*term))
        .take(MAX_SEARCH_TERMS)
        .map(str::to_string)
        .collect()
}

fn cacheable(mut response: Response) -> Response {
    response.headers_mut().insert(CACHE_CONTROL, CACHE_POLICY);
    response
}

#[utoipa::path(
    get, path = "/story-resources/explore", tag = "story", params(ExploreQuery),
    responses(
        (status = 200, description = "Page, grouping, search facets and optional focused resource from one snapshot", body = ExploreResponse),
        (status = 400, description = "Invalid filter or pagination")
    )
)]
pub async fn explore_story_resources(
    State(state): State<AppState>,
    Query(query): Query<ExploreQuery>,
) -> WebResult<Response> {
    let page = query.page.unwrap_or(1);
    let page_size = query.page_size.unwrap_or(60);
    let raw = query.q.as_deref().unwrap_or_default();
    if page == 0
        || !(1..=500).contains(&page_size)
        || raw.len() > 2048
        || raw.contains('\0')
        || query
            .focus
            .as_ref()
            .is_some_and(|id| id.len() > 1024 || id.contains('\0'))
    {
        return Err(WebError::BadRequest(
            "invalid search or pagination".to_string(),
        ));
    }
    let terms = search_terms(raw);
    let response = state
        .database
        .explore_story_resources(&StoryExploreOptions {
            resource_type: query.resource_type.as_str(),
            terms: &terms,
            category: query.category.map(CharacterCategory::as_str),
            merged: query.group_by != Some(GroupBy::Resource),
            sort_count: query.sort == Some(Sort::ScriptCount),
            descending: query.order == Some(Order::Desc),
            page: i64::from(page),
            page_size: i64::from(page_size),
            focus: query.focus.as_deref(),
        })
        .await?;
    let response: ExploreResponse =
        serde_json::from_value(response).map_err(crate::AppError::from)?;
    Ok(cacheable(json(response)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn search_accepts_script_commands_and_wiki_filenames() {
        assert_eq!(normalize_query("  BG_Indoor  room "), "bg_indoor  room");
        assert_eq!(
            normalize_query(r#"[charslot(name="avg_2026_yu_1#18$1")]"#),
            "avg_2026_yu_1#18$1"
        );
        assert_eq!(
            normalize_query("文件:Avg avg 2026 yu 1-1$1.png"),
            "avg_2026_yu_1#1$1"
        );
    }

    #[test]
    fn search_terms_are_distinct_and_bounded() {
        assert_eq!(search_terms("npc NPC  10"), ["npc", "10"]);
        assert_eq!(search_terms(&"a ".repeat(1024)), ["a"]);
        let many = (0..100)
            .map(|n| format!("t{n}"))
            .collect::<Vec<_>>()
            .join(" ");
        let terms = search_terms(&many);
        assert_eq!(terms.len(), MAX_SEARCH_TERMS);
        assert_eq!(terms[0], "t0");
    }

    #[test]
    fn responses_stay_out_of_shared_caches() {
        let response = cacheable(json(serde_json::json!({})));
        assert_eq!(
            response.headers().get(CACHE_CONTROL).expect("header"),
            "private, max-age=60"
        );
    }
}
