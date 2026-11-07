use crate::{AppError, AppResult, database::Database};

pub struct StoryExploreOptions<'a> {
    pub resource_type: &'a str,
    pub terms: &'a [String],
    pub category: Option<&'a str>,
    pub merged: bool,
    pub sort_count: bool,
    pub descending: bool,
    pub page: i64,
    pub page_size: i64,
    pub focus: Option<&'a str>,
}

impl Database {
    /// All results and facets come from one database snapshot. Search and
    /// grouping precede paging; unrelated types contribute counts, not rows.
    pub async fn explore_story_resources(
        &self,
        options: &StoryExploreOptions<'_>,
    ) -> AppResult<serde_json::Value> {
        sqlx::query_scalar!(
            r#"
            WITH catalog AS MATERIALIZED (
                SELECT c.*,
                    CASE WHEN $4::boolean THEN coalesce(base, id) ELSE id END AS group_id,
                    CASE WHEN $4 THEN group_script_count ELSE script_count END AS count,
                    jsonb_build_object('type', resource_type, 'id', id, 'base', base,
                        'category', category, 'scriptCount', script_count) AS resource
                FROM story_resource_catalog c
            ), matched AS (
                SELECT *, CASE WHEN cardinality($2::text[]) = 1
                    AND $2[1] = ANY(aliases) AND position('#' IN id) = 0
                    AND resource_type = 'character'
                    THEN nullif(split_part($2[1], '#', 2), '') ELSE NULL END AS face
                FROM catalog c
                WHERE NOT EXISTS (SELECT FROM unnest($2::text[]) term
                                  WHERE position(term IN lower(c.id)) = 0)
                   OR (cardinality($2::text[]) = 1 AND $2[1] = ANY(c.aliases))
            ), grouped AS MATERIALIZED (
                SELECT DISTINCT ON (resource_type, group_id) *
                FROM matched ORDER BY resource_type, group_id, sort_key, id COLLATE "C"
            ), filtered AS MATERIALIZED (
                SELECT * FROM grouped WHERE resource_type <> 'character'
                    OR $3::text IS NULL OR category = $3
            ), ordered AS MATERIALIZED (
                SELECT *, row_number() OVER (ORDER BY
                    CASE WHEN $5::boolean AND NOT $6::boolean THEN count END ASC,
                    CASE WHEN $5 AND $6 THEN count END DESC,
                    CASE WHEN NOT $5 AND $6 THEN sort_key END DESC,
                    CASE WHEN $5 OR NOT $6 THEN sort_key END ASC,
                    id COLLATE "C" ASC) - 1 AS position
                FROM filtered WHERE resource_type = $1
            ), focus AS (
                SELECT * FROM catalog WHERE resource_type = $1 AND id = $9::text
            ), stats AS (
                SELECT count(*) AS matched_total,
                    (SELECT count(DISTINCT group_id) FROM catalog WHERE resource_type = $1) AS total,
                    (SELECT o.position FROM ordered o JOIN focus f USING (resource_type, group_id)) AS focus_index
                FROM ordered
            ), paging AS (
                SELECT *, greatest(1, (matched_total + $8::bigint - 1) / $8) AS page_count,
                    coalesce(focus_index / $8 + 1,
                        least($7::bigint, greatest(1, (matched_total + $8 - 1) / $8))) AS page
                FROM stats
            ), selected AS (
                SELECT o.*, false AS selection FROM ordered o, paging p
                WHERE o.position >= (p.page - 1) * $8 AND o.position < p.page * $8
                UNION ALL
                SELECT f.*, NULL::text AS face, NULL::bigint AS position, true AS selection FROM focus f
            ), members AS (
                SELECT c.resource_type, coalesce(c.base, c.id) AS member_key,
                    jsonb_agg(c.resource ORDER BY c.sort_key, c.id COLLATE "C") AS members
                FROM catalog c
                WHERE (c.resource_type, coalesce(c.base, c.id)) IN (
                    SELECT resource_type, coalesce(base, id) FROM selected
                )
                GROUP BY c.resource_type, coalesce(c.base, c.id)
            ), entries AS (
                SELECT s.position, s.selection, jsonb_build_object(
                    'resource', s.resource, 'face', s.face, 'count', s.count,
                    'merged', CASE WHEN $4 THEN jsonb_array_length(m.members) ELSE 1 END,
                    'members', m.members) AS entry
                FROM selected s JOIN members m
                    ON m.resource_type = s.resource_type AND m.member_key = coalesce(s.base, s.id)
            )
            SELECT jsonb_build_object(
                'entries', coalesce((SELECT jsonb_agg(entry ORDER BY position) FROM entries WHERE NOT selection), '[]'),
                'selection', (SELECT entry FROM entries WHERE selection),
                'focusIndex', p.focus_index,
                'total', p.total, 'matchedTotal', p.matched_total,
                'page', p.page, 'pageSize', $8::bigint, 'pageCount', p.page_count,
                'counts', jsonb_build_object(
                    'character', (SELECT count(*) FROM filtered WHERE resource_type = 'character'),
                    'background', (SELECT count(*) FROM filtered WHERE resource_type = 'background'),
                    'image', (SELECT count(*) FROM filtered WHERE resource_type = 'image'),
                    'item', (SELECT count(*) FROM filtered WHERE resource_type = 'item')),
                'categoryCounts', jsonb_build_object(
                    'operator', (SELECT count(*) FROM grouped WHERE category = 'operator'),
                    'npc', (SELECT count(*) FROM grouped WHERE category = 'npc'),
                    'other', (SELECT count(*) FROM grouped WHERE category = 'other')),
                'updatedAt', (SELECT updated_at::text FROM story_dataset)
            ) AS "response!: serde_json::Value" FROM paging p
            "#,
            options.resource_type,
            options.terms,
            options.category,
            options.merged,
            options.sort_count,
            options.descending,
            options.page,
            options.page_size,
            options.focus,
        )
        .fetch_one(self.pool())
        .await
        .map_err(|err| AppError::ExternalService(err.into()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::row::{StoryResourceType, StoryUsageRow};
    use std::collections::HashMap;

    fn options() -> StoryExploreOptions<'static> {
        StoryExploreOptions {
            resource_type: "character",
            terms: &[],
            category: None,
            merged: true,
            sort_count: false,
            descending: false,
            page: 1,
            page_size: 1,
            focus: None,
        }
    }

    fn usage(script: &str, kind: StoryResourceType, id: &str, listing: &str) -> StoryUsageRow {
        StoryUsageRow {
            script_path: script.to_string(),
            resource_type: kind,
            resource_id: id.to_string(),
            listing_id: listing.to_string(),
            display_names: vec![],
            sort_order: 0,
        }
    }

    async fn fixture(pool: sqlx::PgPool) -> Database {
        use StoryResourceType::{Background, Character, Image};
        let database = Database { pool };
        database
            .replace_story_resource_usages(
                &[
                    usage("a", Character, "avg_npc_9#1$1", "avg_npc_9$1"),
                    usage("a", Character, "avg_npc_9#2$1", "avg_npc_9$1"),
                    usage("b", Character, "avg_npc_9#1$1", "avg_npc_9$1"),
                    usage("b", Character, "avg_npc_9#1$2", "avg_npc_9$2"),
                    usage("c", Character, "avg_npc_9#1$2", "avg_npc_9$2"),
                    usage("a", Character, "avg_npc_10#1$1", "avg_npc_10$1"),
                    usage("a", Character, "char_1_a#first", "char_1_a#first"),
                    usage("a", Background, "bg_npc_room", "bg_npc_room"),
                    usage("a", Image, "img_test", "img_test"),
                ],
                &HashMap::from([("avg_npc_9#unused".to_string(), "avg_npc_9$2".to_string())]),
            )
            .await
            .expect("import fixture");
        database
    }

    // These create isolated databases; they never replace the developer's dataset.
    #[sqlx::test]
    #[ignore = "requires local PostgreSQL; run with DATABASE_URL and --ignored"]
    async fn browse_groups_before_paging_and_counts_distinct_scripts(pool: sqlx::PgPool) {
        let database = fixture(pool).await;
        let mut query = options();
        let result = database
            .explore_story_resources(&query)
            .await
            .expect("browse");
        assert_eq!(result["entries"].as_array().expect("entries").len(), 1);
        assert_eq!(result["entries"][0]["resource"]["id"], "avg_npc_9$1");
        assert_eq!(result["entries"][0]["resource"]["scriptCount"], 2);
        assert_eq!(result["entries"][0]["count"], 3); // overlap b is counted once
        assert_eq!(result["entries"][0]["merged"], 2);
        assert_eq!(
            result["entries"][0]["members"]
                .as_array()
                .expect("members")
                .len(),
            2
        );
        assert_eq!(result["total"], 3);
        assert_eq!(result["matchedTotal"], 3);
        assert_eq!(
            result["counts"],
            serde_json::json!({"character": 3, "background": 1, "image": 1, "item": 0})
        );
        assert_eq!(
            result["categoryCounts"],
            serde_json::json!({"operator": 1, "npc": 2, "other": 0})
        );

        query.page = 2;
        let result = database
            .explore_story_resources(&query)
            .await
            .expect("second page");
        assert_eq!(result["entries"][0]["resource"]["id"], "avg_npc_10$1");
        query.page = 99;
        let result = database
            .explore_story_resources(&query)
            .await
            .expect("clamp page");
        assert_eq!(result["page"], 3);

        query.merged = false;
        query.page = 1;
        let result = database
            .explore_story_resources(&query)
            .await
            .expect("ungrouped");
        assert_eq!(result["total"], 4);
        assert_eq!(result["entries"][0]["count"], 2);
        assert_eq!(result["entries"][0]["merged"], 1);

        query.merged = true;
        query.sort_count = true;
        query.descending = true;
        let result = database
            .explore_story_resources(&query)
            .await
            .expect("usage sort");
        assert_eq!(result["entries"][0]["count"], 3);
        query.descending = false;
        let result = database
            .explore_story_resources(&query)
            .await
            .expect("usage ascending");
        assert_eq!(result["entries"][0]["resource"]["id"], "avg_npc_10$1");
    }

    #[sqlx::test]
    #[ignore = "requires local PostgreSQL; run with DATABASE_URL and --ignored"]
    async fn browse_search_facets_aliases_and_focused_details(pool: sqlx::PgPool) {
        let database = fixture(pool).await;
        let mut query = options();
        let terms = vec!["npc".to_string()];
        query.terms = &terms;
        query.category = Some("operator");
        let result = database
            .explore_story_resources(&query)
            .await
            .expect("facets");
        assert_eq!(result["total"], 3);
        assert_eq!(result["matchedTotal"], 0);
        assert_eq!(result["counts"]["background"], 1);
        assert_eq!(result["counts"]["character"], 0);
        assert_eq!(result["categoryCounts"]["npc"], 2);

        query.category = None;
        query.focus = Some("char_1_a#first");
        let result = database
            .explore_story_resources(&query)
            .await
            .expect("detail outside search");
        assert_eq!(result["selection"]["resource"]["id"], "char_1_a#first");
        assert!(result["focusIndex"].is_null());
        query.terms = &[];
        let result = database
            .explore_story_resources(&query)
            .await
            .expect("locate detail");
        assert_eq!(result["page"], 3);
        assert_eq!(result["focusIndex"], 2);

        query.focus = None;
        let terms = vec!["avg_npc_9#unused".to_string()];
        query.terms = &terms;
        let result = database
            .explore_story_resources(&query)
            .await
            .expect("unused face alias");
        assert_eq!(result["matchedTotal"], 1);
        assert_eq!(result["entries"][0]["resource"]["id"], "avg_npc_9$2");
        assert_eq!(result["entries"][0]["face"], "unused");
        assert_eq!(result["entries"][0]["count"], 3);

        let terms = vec!["npc".to_string(), "10".to_string()];
        query.terms = &terms;
        let result = database
            .explore_story_resources(&query)
            .await
            .expect("AND terms");
        assert_eq!(result["matchedTotal"], 1);
        assert_eq!(result["entries"][0]["resource"]["id"], "avg_npc_10$1");
        let terms = vec!["%".to_string()];
        query.terms = &terms;
        let result = database
            .explore_story_resources(&query)
            .await
            .expect("literal search");
        assert_eq!(result["matchedTotal"], 0);
    }

    #[sqlx::test]
    #[ignore = "requires local PostgreSQL; run with DATABASE_URL and --ignored"]
    async fn catalog_updates_atomically_with_usage_snapshot(pool: sqlx::PgPool) {
        let database = fixture(pool).await;
        let before = database
            .explore_story_resources(&options())
            .await
            .expect("before");
        let duplicate = usage(
            "new",
            StoryResourceType::Character,
            "npc_new#1",
            "npc_new$1",
        );
        assert!(
            database
                .replace_story_resource_usages(&[duplicate.clone(), duplicate], &HashMap::new())
                .await
                .is_err()
        );
        assert_eq!(
            database
                .explore_story_resources(&options())
                .await
                .expect("rollback"),
            before
        );
        database
            .replace_story_resource_usages(&[], &HashMap::new())
            .await
            .expect("empty snapshot");
        let result = database
            .explore_story_resources(&options())
            .await
            .expect("after");
        assert_eq!(result["total"], 0);
        assert_eq!(result["counts"]["background"], 0);
        assert_eq!(result["entries"], serde_json::json!([]));
        assert_eq!(result["pageCount"], 1);
    }

    #[sqlx::test]
    #[ignore = "requires local PostgreSQL; run with DATABASE_URL and --ignored"]
    async fn catalog_keeps_the_widget_order_and_categories(pool: sqlx::PgPool) {
        use StoryResourceType::{Background, Character};
        let database = Database { pool };
        let listing = |kind, id: &str| usage("a", kind, id, id);
        let mut rows = [
            "bg_desert03_d",
            "bg_desert_2",
            "bg_desert_1",
            "bg_desert_10",
        ]
        .map(|id| listing(Background, id))
        .to_vec();
        let characters = [
            ("avg_npc_009", "npc"),
            ("avg_npc_1898_1$1", "npc"),
            ("npc_2001_aya_1#npc_2001_Aya_1", "npc"),
            ("avgnew_npc_140_1$1", "npc"),
            ("avg_003_kalts_1$1", "operator"),
            ("char_211_adnach_1#char_211_adnach_6", "operator"),
            ("avgnew_112_siege_1$1", "operator"),
            ("avg_char_501_durin_1#x", "operator"),
            ("avg_doc_1#avg_doc_1", "other"),
            ("char_empty#char_empty", "other"),
            ("avg_4091_ulika2_1$1", "operator"),
            ("avg_4091_ulika_1$1", "operator"),
        ];
        rows.extend(characters.map(|(id, _)| listing(Character, id)));
        database
            .replace_story_resource_usages(&rows, &HashMap::new())
            .await
            .expect("import");

        let ids = |result: &serde_json::Value| {
            result["entries"]
                .as_array()
                .expect("entries")
                .iter()
                .map(|entry| entry["resource"]["id"].as_str().expect("id").to_string())
                .collect::<Vec<_>>()
        };
        let mut query = options();
        query.page_size = 50;
        query.merged = false;
        query.resource_type = "background";
        let result = database
            .explore_story_resources(&query)
            .await
            .expect("backgrounds");
        assert_eq!(
            ids(&result),
            [
                "bg_desert_1",
                "bg_desert_2",
                "bg_desert_10",
                "bg_desert03_d"
            ]
        );

        query.resource_type = "character";
        let result = database
            .explore_story_resources(&query)
            .await
            .expect("characters");
        let listed = ids(&result);
        let at = |id: &str| listed.iter().position(|item| item == id).expect("listed");
        assert!(at("avg_4091_ulika_1$1") < at("avg_4091_ulika2_1$1"));
        for entry in result["entries"].as_array().expect("entries") {
            let id = entry["resource"]["id"].as_str().expect("id");
            let (_, category) = characters
                .iter()
                .find(|(character, _)| *character == id)
                .expect("known id");
            assert_eq!(entry["resource"]["category"], *category, "{id}");
        }
    }
}
