use crate::{
    app::AppState,
    error::Result,
    views::{
        bundles::BundleDetail,
        utils::json,
        versions::{LatestFlag, VersionDetail, VersionListItem},
    },
};
use axum::{debug_handler, extract::Path, response::Response};
use sqlx::query_as;

#[debug_handler(state = AppState)]
#[utoipa::path(get, path = "/version",tag = "version", responses((status = OK, body = [VersionListItem])))]
pub async fn list(ctx: AppState) -> Result<Response> {
    let result = query_as!(
        VersionListItem,
        r#"SELECT id, client, res, is_ready FROM versions"#
    )
    .fetch_all(&ctx.database)
    .await?;
    Ok(json(result))
}

#[debug_handler(state = AppState)]
#[utoipa::path(get, path = "/version/{id}",tag = "version", responses((status = OK, body = VersionDetail)))]
pub async fn get(Path(id): Path<i32>, ctx: AppState) -> Result<Response> {
    let result = query_as!(VersionDetail, r#"select * from versions where id = $1"#, id)
        .fetch_one(&ctx.database)
        .await?;
    Ok(json(result))
}

#[debug_handler(state = AppState)]
#[utoipa::path(get, path = "/version/{id}/files",tag = "version", responses((status = OK, body = [BundleDetail])))]
pub async fn list_files(Path(id): Path<i32>, ctx: AppState) -> Result<Response> {
    let result = query_as!(
        BundleDetail,
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
            b.version = $1
            "#,
        id
    )
    .fetch_all(&ctx.database)
    .await?;
    Ok(json(result))
}

#[debug_handler(state = AppState)]
#[utoipa::path(get, path = "/version/latest", tag = "version", params(LatestFlag), responses((status = OK, body = VersionDetail)))]
pub async fn latest(query: LatestFlag, ctx: AppState) -> Result<Response> {
    let result = query_as!(
        VersionDetail,
        r#"select * from versions where ($1::bool IS NULL OR is_ready = $1)"#,
        query.ready
    )
    .fetch_one(&ctx.database)
    .await?;
    Ok(json(result))
}
