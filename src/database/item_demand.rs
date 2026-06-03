use crate::{AppError, AppResult, database::Database};
use sqlx::query_scalar;

impl Database {
    pub async fn query_usage_by_item_name(&self, item_name: &str) -> AppResult<Option<String>> {
        query_scalar!(
            r#"
            SELECT usage FROM item_demands WHERE name = $1
            "#,
            item_name
        )
        .fetch_optional(self.pool())
        .await
        .map_err(|err| AppError::ExternalService(err.into()))
    }

    pub async fn replace_all_demands(&self, demands: Vec<(String, String)>) -> AppResult<()> {
        let mut tx = self
            .pool()
            .begin()
            .await
            .map_err(|err| AppError::ExternalService(err.into()))?;

        sqlx::query!("DELETE FROM item_demands")
            .execute(&mut *tx)
            .await
            .map_err(|err| AppError::ExternalService(err.into()))?;

        for (name, usage) in demands {
            sqlx::query!(
                "INSERT INTO item_demands (name, usage) VALUES ($1, $2)",
                name,
                usage
            )
            .execute(&mut *tx)
            .await
            .map_err(|err| AppError::ExternalService(err.into()))?;
        }

        tx.commit()
            .await
            .map_err(|err| AppError::ExternalService(err.into()))?;
        Ok(())
    }
}
