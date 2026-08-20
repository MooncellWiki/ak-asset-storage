use crate::{
    AppError, AppResult,
    database::{Database, row::StoryUsageRow},
};

/// Query result item for the resource reverse-lookup API.
#[derive(Debug, Clone)]
pub struct StoryUsageItemRow {
    pub script_path: String,
    pub display_names: Vec<String>,
}

/// Body-granularity usage row: one script using any face of the body, with
/// the face-level ids (`base#face$body`) it uses.
#[derive(Debug, Clone)]
pub struct StoryCharacterBodyUsageRow {
    pub script_path: String,
    pub display_names: Vec<String>,
    pub faces: Vec<String>,
}

/// Listing result item: one distinct resource with its usage count.
#[derive(Debug, Clone)]
pub struct StoryResourceSummaryRow {
    pub resource_type: String,
    pub resource_id: String,
    pub script_count: i64,
}

const INSERT_BATCH_SIZE: usize = 1000;

impl Database {
    /// Replaces the whole `story_resource_usages` snapshot and touches
    /// `story_dataset.updated_at` in one transaction. On failure the previous
    /// snapshot stays readable.
    pub async fn replace_story_resource_usages(&self, rows: &[StoryUsageRow]) -> AppResult<()> {
        let mut tx = self
            .pool()
            .begin()
            .await
            .map_err(|err| AppError::ExternalService(err.into()))?;

        sqlx::query!("DELETE FROM story_resource_usages")
            .execute(&mut *tx)
            .await
            .map_err(|err| AppError::ExternalService(err.into()))?;

        for chunk in rows.chunks(INSERT_BATCH_SIZE) {
            // jsonb_to_recordset matches record field names exactly, so the
            // JSON keys use the table's snake_case names.
            let batch: Vec<serde_json::Value> = chunk
                .iter()
                .map(|row| {
                    serde_json::json!({
                        "script_path": row.script_path,
                        "resource_type": row.resource_type,
                        "resource_id": row.resource_id,
                        "listing_id": row.listing_id,
                        "display_names": row.display_names,
                        "sort_order": row.sort_order,
                    })
                })
                .collect();
            let batch = serde_json::Value::Array(batch);

            sqlx::query!(
                r#"
                INSERT INTO story_resource_usages
                    (script_path, resource_type, resource_id, listing_id, display_names, sort_order)
                SELECT batch.script_path, batch.resource_type, batch.resource_id,
                       batch.listing_id, batch.display_names, batch.sort_order
                FROM jsonb_to_recordset($1::jsonb)
                    AS batch(
                        script_path text,
                        resource_type text,
                        resource_id text,
                        listing_id text,
                        display_names text[],
                        sort_order int4
                    )
                "#,
                batch
            )
            .execute(&mut *tx)
            .await
            .map_err(|err| AppError::ExternalService(err.into()))?;
        }

        sqlx::query!(
            r#"
            INSERT INTO story_dataset (singleton, updated_at)
            VALUES (TRUE, now())
            ON CONFLICT (singleton) DO UPDATE SET updated_at = now()
            "#
        )
        .execute(&mut *tx)
        .await
        .map_err(|err| AppError::ExternalService(err.into()))?;

        tx.commit()
            .await
            .map_err(|err| AppError::ExternalService(err.into()))?;
        Ok(())
    }

    /// Reverse lookup: which scripts use `resource_type`/`resource_id`, keyed
    /// by ascending `script_path` for cursor pagination.
    pub async fn query_story_resource_usages(
        &self,
        resource_type: &str,
        resource_id: &str,
        cursor: Option<&str>,
        limit: i64,
    ) -> AppResult<Vec<StoryUsageItemRow>> {
        sqlx::query_as!(
            StoryUsageItemRow,
            r#"
            SELECT script_path, display_names
            FROM story_resource_usages
            WHERE resource_type = $1
              AND resource_id = $2
              AND ($3::text IS NULL OR script_path > $3)
            ORDER BY script_path
            LIMIT $4
            "#,
            resource_type,
            resource_id,
            cursor,
            limit
        )
        .fetch_all(self.pool())
        .await
        .map_err(|err| AppError::ExternalService(err.into()))
    }

    /// Reverse lookup at character body granularity: `body_id` is the face
    /// suffix-stripped `base$body` form (the `listing_id`), and scripts
    /// using any face of that body collapse into one row with the union of
    /// their display names and the face-level ids each script uses. Keyed
    /// by ascending `script_path` for cursor pagination.
    pub async fn query_story_character_body_usages(
        &self,
        body_id: &str,
        cursor: Option<&str>,
        limit: i64,
    ) -> AppResult<Vec<StoryCharacterBodyUsageRow>> {
        sqlx::query_as!(
            StoryCharacterBodyUsageRow,
            r#"
            SELECT script_path,
                   COALESCE(
                       array_agg(DISTINCT name) FILTER (WHERE name IS NOT NULL),
                       '{}'::text[]
                   ) AS "display_names!: Vec<String>",
                   array_agg(DISTINCT resource_id ORDER BY resource_id) AS "faces!: Vec<String>"
            FROM story_resource_usages
            LEFT JOIN LATERAL unnest(display_names) AS name ON TRUE
            WHERE resource_type = 'character'
              AND listing_id = $1
              AND ($2::text IS NULL OR script_path > $2)
            GROUP BY script_path
            ORDER BY script_path
            LIMIT $3
            "#,
            body_id,
            cursor,
            limit
        )
        .fetch_all(self.pool())
        .await
        .map_err(|err| AppError::ExternalService(err.into()))
    }

    /// Lists distinct resources with per-resource distinct-script counts,
    /// keyed by ascending `(resource_type, listing_id)` for cursor
    /// pagination. Characters collapse to body granularity — the renderer
    /// composites each face onto a shared `base$body` texture, so the `#face`
    /// overlay suffix is stripped and faces of one body merge; other types
    /// list their ids verbatim.
    ///
    /// `id_pattern` is an already-escaped ILIKE fragment without the
    /// surrounding `%` wildcards and matches the listing id; `after` is an
    /// exclusive keyset bound.
    pub async fn list_story_resources(
        &self,
        resource_type: Option<&str>,
        id_pattern: Option<&str>,
        after: Option<(&str, &str)>,
        limit: i64,
    ) -> AppResult<Vec<StoryResourceSummaryRow>> {
        let (after_type, after_id) = after.map_or((None, None), |(resource_type, resource_id)| {
            (Some(resource_type), Some(resource_id))
        });
        sqlx::query_as!(
            StoryResourceSummaryRow,
            r#"
            SELECT resource_type, listing_id AS "resource_id!: String",
                   count(DISTINCT script_path) AS "script_count!: i64"
            FROM story_resource_usages
            WHERE ($1::text IS NULL OR resource_type = $1)
              AND ($2::text IS NULL OR listing_id ILIKE '%' || $2 || '%')
              AND ($3::text IS NULL OR (resource_type, listing_id) > ($3, $4))
            GROUP BY resource_type, listing_id
            ORDER BY resource_type, listing_id
            LIMIT $5
            "#,
            resource_type,
            id_pattern,
            after_type,
            after_id,
            limit
        )
        .fetch_all(self.pool())
        .await
        .map_err(|err| AppError::ExternalService(err.into()))
    }
}
