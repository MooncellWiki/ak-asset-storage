use crate::{
    app::Context,
    error::{Error, Result},
    models::{
        _entities::versions::{Entity, Model},
        bundles,
    },
    views::{utils::json, versions::VersionListItem, FileDetail},
};
use axum::{
    debug_handler,
    extract::{Path, Query, State},
    response::Response,
};
use sea_orm::EntityTrait;
use serde::Deserialize;
use utoipa::IntoParams;
use utoipa_axum::{router::OpenApiRouter, routes};

#[debug_handler]
#[utoipa::path(get, path = "",tag = "version", responses((status = OK, body = [VersionListItem])))]
pub async fn list(State(ctx): State<Context>) -> Result<Response> {
    let resp = Model::list(&ctx.conn).await?;
    Ok(json(resp))
}

#[debug_handler]
#[utoipa::path(get, path = "/{id}",tag = "version", responses((status = OK, body = Model)))]
pub async fn get(Path(id): Path<i32>, State(ctx): State<Context>) -> Result<Response> {
    let item = Entity::find_by_id(id).one(&ctx.conn).await?;
    item.ok_or_else(|| Error::NotFound).map(json)
}

#[debug_handler]
#[utoipa::path(get, path = "/{id}/files",tag = "version", responses((status = OK, body = [FileDetail])))]
pub async fn list_files(Path(id): Path<i32>, State(ctx): State<Context>) -> Result<Response> {
    bundles::Model::list_by_version_id(&ctx.conn, id)
        .await
        .map(json)
}

#[derive(Debug, Deserialize, IntoParams)]
pub struct LatestFlag {
    ready: bool,
}
#[debug_handler]
#[utoipa::path(get, path = "/latest", tag = "version", params(LatestFlag), responses((status = OK, body = Model)))]
pub async fn latest(
    Query(query): Query<LatestFlag>,
    State(ctx): State<Context>,
) -> Result<Response> {
    Model::latest(&ctx.conn, query.ready).await.map(json)
}

pub fn routes() -> OpenApiRouter<Context> {
    OpenApiRouter::new().nest(
        "/version",
        OpenApiRouter::new()
            .routes(routes!(list))
            .routes(routes!(get))
            .routes(routes!(list_files))
            .routes(routes!(latest)),
    )
}
