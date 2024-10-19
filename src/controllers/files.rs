use crate::error::{Error, Result};
use crate::views::utils::json;
use crate::{
    app::Context,
    models::{
        _entities::files::{Entity, Model},
        files::Filter,
    },
};
use axum::extract::{Path, State};
use axum::response::Response;
use axum::routing::get;
use axum::Router;
use axum::{debug_handler, extract::Query};
use sea_orm::EntityTrait;

async fn load_item(ctx: &Context, id: i32) -> Result<Model> {
    let item = Entity::find_by_id(id).one(&ctx.conn).await?;
    item.ok_or_else(|| Error::NotFound)
}

#[debug_handler]
pub async fn get_one(Path(id): Path<i32>, State(ctx): State<Context>) -> Result<Response> {
    load_item(&ctx, id).await.map(json)
}

#[debug_handler]
pub async fn filter(Query(query): Query<Filter>, State(ctx): State<Context>) -> Result<Response> {
    Model::filter(&ctx.conn, query).await.map(json)
}

pub fn routes() -> Router<Context> {
    Router::new().nest(
        "/files",
        Router::new()
            .route("/", get(filter))
            .route("//:id", get(get_one)),
    )
}
