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
                        "display_names": row.display_names,
                        "sort_order": row.sort_order,
                    })
                })
                .collect();
            let batch = serde_json::Value::Array(batch);

            sqlx::query!(
                r#"
                INSERT INTO story_resource_usages
                    (script_path, resource_type, resource_id, display_names, sort_order)
                SELECT batch.script_path, batch.resource_type, batch.resource_id,
                       batch.display_names, batch.sort_order
                FROM jsonb_to_recordset($1::jsonb)
                    AS batch(
                        script_path text,
                        resource_type text,
                        resource_id text,
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
}
