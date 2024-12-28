use crate::{config, db, db::Pool};
use anyhow::Result;
use axum::extract::{FromRequestParts, State};
use derive_more::derive::Deref;
use std::sync::Arc;
#[derive(Clone)]
pub struct Context {
    pub database: Pool,
}
impl Context {
    pub async fn new(config: &config::Config) -> Result<Self> {
        Ok(Self {
            database: db::connect(&config.database).await?,
        })
    }
}

#[derive(Clone, FromRequestParts, Deref)]
#[from_request(via(State))]
pub struct AppState(pub Arc<Context>);
