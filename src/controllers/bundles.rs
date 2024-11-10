use crate::error::{Error, Result};
use crate::models::bundles::Filter;
use crate::views::utils::json;
use crate::views::FileDetail;
use crate::{app::Context, models::_entities::bundles::Model};
use axum::extract::{Path, State};
use axum::response::Response;
use axum::{debug_handler, extract::Query};
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

async fn load_item(ctx: &Context, id: i32) -> Result<FileDetail> {
    let item = Model::detail_by_id(&ctx.conn, id).await?;
    item.ok_or_else(|| Error::NotFound)
}

#[debug_handler]
#[utoipa::path(get, path = "/{id}", tag="bundle", responses((status = OK, body = FileDetail)))]
pub async fn get_one(Path(id): Path<i32>, State(ctx): State<Context>) -> Result<Response> {
    load_item(&ctx, id).await.map(json)
}

#[debug_handler]
#[utoipa::path(get, path = "", tag="bundle", params(Filter), responses((status = OK, body = [FileDetail])))]
pub async fn filter(Query(query): Query<Filter>, State(ctx): State<Context>) -> Result<Response> {
    Model::filter(&ctx.conn, query).await.map(json)
}

pub fn routes() -> OpenApiRouter<Context> {
    OpenApiRouter::new().nest(
        "/bundles",
        OpenApiRouter::new()
            .routes(routes!(filter))
            .routes(routes!(get_one)),
    )
}
