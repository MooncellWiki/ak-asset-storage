use crate::{config, db, db::Pool};
use anyhow::Result;
use axum::extract::{FromRequestParts, State};
use derive_more::derive::Deref;
use object_store::aws::AmazonS3;
use sqlx::{pool::PoolConnection, Postgres, Transaction};
use std::sync::Arc;
#[derive(Clone)]
pub struct Context {
    pub database: Pool,
    pub s3: AmazonS3,
}
impl Context {
    pub async fn new(config: &config::Config) -> Result<Self> {
        Ok(Self {
            database: db::connect(&config.database).await?,
            s3: config.s3.client().unwrap(),
        })
    }
}

#[derive(Clone, FromRequestParts, Deref)]
#[from_request(via(State))]
pub struct AppState(pub Arc<Context>);
impl AppState {
    pub async fn acquire(&self) -> Result<PoolConnection<Postgres>> {
        Ok(self.0.database.acquire().await?)
    }
    pub async fn begin(&self) -> Result<Transaction<'_, Postgres>> {
        Ok(self.0.database.begin().await?)
    }
}
