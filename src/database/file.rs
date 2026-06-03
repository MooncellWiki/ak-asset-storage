use crate::{
    AppError, AppResult,
    database::{Database, row::FileRow},
};
use sqlx::{query, query_as};

impl Database {
    pub async fn create_file(&self, file: FileRow) -> AppResult<i32> {
        let row = query!(
            "INSERT INTO files (hash, size) VALUES ($1, $2) RETURNING id",
            file.hash.as_str(),
            file.size
        )
        .fetch_one(self.pool())
        .await
        .map_err(|err| AppError::ExternalService(err.into()))?;
        Ok(row.id)
    }

    pub async fn get_file_by_hash(&self, hash: &str) -> AppResult<Option<FileRow>> {
        query_as!(
            FileRow,
            "SELECT id, hash, size FROM files WHERE hash = $1",
            hash
        )
        .fetch_optional(self.pool())
        .await
        .map_err(|err| AppError::ExternalService(err.into()))
    }
}
