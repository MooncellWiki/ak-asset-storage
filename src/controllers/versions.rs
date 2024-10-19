use axum::{
    debug_handler,
    extract::{Path, Query, State},
    response::Response,
    routing::get,
    Router,
};
use sea_orm::EntityTrait;
use serde::Deserialize;

use crate::{
    app::Context,
    error::{Error, Result},
    models::_entities::{
        files,
        versions::{Entity, Model},
    },
    views::utils::json,
};

#[debug_handler]
pub async fn list(State(ctx): State<Context>) -> Result<Response> {
    let resp = Entity::find().all(&ctx.conn).await?;
    Ok(json(resp))
}

async fn load_item(ctx: &Context, id: i32) -> Result<Model> {
    let item = Entity::find_by_id(id).one(&ctx.conn).await?;
    item.ok_or_else(|| Error::NotFound)
}

#[debug_handler]
pub async fn get_one(Path(id): Path<i32>, State(ctx): State<Context>) -> Result<Response> {
    load_item(&ctx, id).await.map(json)
}

#[debug_handler]
pub async fn list_files(Path(id): Path<i32>, State(ctx): State<Context>) -> Result<Response> {
    files::Model::list_by_version_id(&ctx.conn, id)
        .await
        .map(json)
}

#[derive(Debug, Deserialize)]
pub struct LatestFlag {
    latest: bool,
}
#[debug_handler]
pub async fn latest(
    Query(query): Query<LatestFlag>,
    State(ctx): State<Context>,
) -> Result<Response> {
    Model::latest(&ctx.conn, query.latest).await.map(json)
}

pub fn routes() -> Router<Context> {
    Router::new().nest(
        "version",
        Router::new()
            .route("/", get(list))
            .route("/:id", get(get_one))
            .route("/:id/files", get(list_files))
            .route("/latest", get(latest)),
    )
}
