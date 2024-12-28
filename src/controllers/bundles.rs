use crate::{
    app::AppState,
    error::Result,
    views::{bundles::Filter, files::FileDetail, utils::json},
};
use axum::debug_handler;
use axum::extract::Path;
use axum::response::Response;
use sqlx::query_as;

#[debug_handler(state = AppState)]
#[utoipa::path(get, path = "/bundle/{id}", tag="bundle", responses((status = OK, body = FileDetail)))]
pub async fn get_one(Path(id): Path<i32>, state: AppState) -> Result<Response> {
    let result = query_as!(
        FileDetail,
        r#"
SELECT
    b.path as "path!",
    b.file as "file!",
    b.version as "version!",
    f.hash as "hash!",
    f.size as "size!",
    v.client as "client!",
    v.res as "res!",
    v.is_ready as "is_ready!"
FROM
    bundles b
INNER JOIN
    files f ON b.file = f.id
INNER JOIN
    versions v ON b.version = v.id
WHERE
    b.id = $1
    "#,
        id
    )
    .fetch_one(&state.database)
    .await?;
    Ok(json(result))
}

#[debug_handler(state = AppState)]
#[utoipa::path(get, path = "/bundle", tag="bundle", params(Filter), responses((status = OK, body = [FileDetail])))]
pub async fn filter(query: Filter, ctx: AppState) -> Result<Response> {
    let result = query_as!(
        FileDetail,
        r#"
SELECT
    b.path as "path!",
    b.file as "file!",
    b.version as "version!",
    f.hash as "hash!",
    f.size as "size!",
    v.client as "client!",
    v.res as "res!",
    v.is_ready as "is_ready!"
FROM
    bundles b
INNER JOIN
    files f ON b.file = f.id
INNER JOIN
    versions v ON b.version = v.id
WHERE
    ($1::varchar IS NULL OR b.path = $1)
    AND ($2::varchar IS NULL OR f.hash = $2)
    AND ($3::int IS NULL OR b.file = $3)
    AND ($4::int IS NULL OR b.version = $4)
    "#,
        query.path,
        query.hash,
        query.file,
        query.version
    )
    .fetch_all(&ctx.database)
    .await?;
    Ok(json(result))
}
