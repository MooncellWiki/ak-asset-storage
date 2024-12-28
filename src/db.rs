use crate::config;
use anyhow::Result;
use sqlx::{postgres::PgPoolOptions, Postgres};
pub type Pool = sqlx::Pool<Postgres>;

pub async fn connect(config: &config::Database) -> Result<Pool> {
    Ok(PgPoolOptions::new().connect(&config.uri).await?)
}

pub async fn migrate(pool: &Pool) -> Result<()> {
    sqlx::migrate!("./migrations").run(pool).await?;
    Ok(())
}
