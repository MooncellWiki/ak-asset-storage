use crate::error::InfraError;
use application::{error::AppResult, ports::repositories::FileRepository};
use async_trait::async_trait;
use domain::entities::{File, FileId};
use domain::value_objects::{FileHash, FileSize};
use sqlx::{query, Pool, Postgres};

#[derive(Debug, Clone)]
pub struct PostgresFileRepository {
    pool: Pool<Postgres>,
}

impl PostgresFileRepository {
    #[must_use]
    pub const fn new(pool: Pool<Postgres>) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl FileRepository for PostgresFileRepository {
    async fn create(&self, file: File) -> AppResult<i32> {
        let row = query!(
            "INSERT INTO files (hash, size) VALUES ($1, $2) RETURNING id",
            file.hash.as_str(),
            file.size.bytes() as i64
        )
        .fetch_one(&self.pool)
        .await
        .map_err(|e| InfraError::Database {
            message: "Failed to create file".to_string(),
            source: e,
        })?;

        Ok(row.id)
    }

    async fn get_by_id(&self, id: i32) -> AppResult<Option<File>> {
        let result = query!("SELECT id, hash, size FROM files WHERE id = $1", id)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| InfraError::Database {
                message: "Failed to get file by id".to_string(),
                source: e,
            })?;

        if let Some(row) = result {
            let file = File::with_id(
                FileId(row.id),
                FileHash::new(&row.hash)?,
                FileSize::new(row.size as i32)?,
            );
            Ok(Some(file))
        } else {
            Ok(None)
        }
    }

    async fn get_by_hash(&self, hash: &str) -> AppResult<Option<File>> {
        let result = query!("SELECT id, hash, size FROM files WHERE hash = $1", hash)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| InfraError::Database {
                message: "Failed to get file by hash".to_string(),
                source: e,
            })?;

        if let Some(row) = result {
            let file = File::with_id(
                FileId(row.id),
                FileHash::new(&row.hash)?,
                FileSize::new(row.size as i32)?,
            );
            Ok(Some(file))
        } else {
            Ok(None)
        }
    }

    async fn get_all(&self) -> AppResult<Vec<File>> {
        let rows = query!("SELECT id, hash, size FROM files ORDER BY id")
            .fetch_all(&self.pool)
            .await
            .map_err(|e| InfraError::Database {
                message: "Failed to get all files".to_string(),
                source: e,
            })?;

        let mut files = Vec::new();
        for row in rows {
            let file = File::with_id(
                FileId(row.id),
                FileHash::new(&row.hash)?,
                FileSize::new(row.size as i32)?,
            );
            files.push(file);
        }

        Ok(files)
    }

    async fn get_orphaned_files(&self) -> AppResult<Vec<File>> {
        let rows = query!(
            "SELECT f.id, f.hash, f.size FROM files f
             LEFT JOIN bundles b ON f.id = b.file
             WHERE b.file IS NULL
             ORDER BY f.id"
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| InfraError::Database {
            message: "Failed to get orphaned files".to_string(),
            source: e,
        })?;

        let mut files = Vec::new();
        for row in rows {
            let file = File::with_id(
                FileId(row.id),
                FileHash::new(&row.hash)?,
                FileSize::new(row.size as i32)?,
            );
            files.push(file);
        }

        Ok(files)
    }

    async fn delete(&self, id: i32) -> AppResult<()> {
        query!("DELETE FROM files WHERE id = $1", id)
            .execute(&self.pool)
            .await
            .map_err(|e| InfraError::Database {
                message: "Failed to delete file".to_string(),
                source: e,
            })?;

        Ok(())
    }
}
