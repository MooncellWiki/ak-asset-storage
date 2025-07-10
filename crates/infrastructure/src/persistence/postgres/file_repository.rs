use crate::{InfraError, PostgresRepository};
use application::{AppResult, File, FileRepository};
use async_trait::async_trait;
use sqlx::{query, query_as};

#[async_trait]
impl FileRepository for PostgresRepository {
    async fn create_file(&self, file: File) -> AppResult<i32> {
        let row = query!(
            "INSERT INTO files (hash, size) VALUES ($1, $2) RETURNING id",
            file.hash.as_str(),
            file.size
        )
        .fetch_one(&self.pool)
        .await
        .map_err(|e| InfraError::Database {
            message: "Failed to create file".to_string(),
            source: e,
        })?;

        Ok(row.id)
    }

    async fn get_file_by_hash(&self, hash: &str) -> AppResult<Option<File>> {
        let result = query_as!(
            File,
            "SELECT id, hash, size FROM files WHERE hash = $1",
            hash
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| InfraError::Database {
            message: "Failed to get file by hash".to_string(),
            source: e,
        })?;
        Ok(result)
    }
}
