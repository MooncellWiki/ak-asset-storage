use crate::{
    app::AppState,
    error::Result,
    utils::token::{check_version_is_create_by_token, TokenId},
    views::{
        assets::{AssetsDetail, CreateAssetReq, ReuseAssetFromBundleReq},
        utils::json,
    },
};
use axum::{
    debug_handler,
    extract::Path,
    response::{IntoResponse, Response},
    Json,
};
use reqwest::StatusCode;
use sqlx::QueryBuilder;
use sqlx::{query, query_as};

#[debug_handler(state = AppState)]
#[utoipa::path(
    get,
    path = "/asset/{id}",
    tag = "asset",
    responses((status = OK, body = AssetsDetail))
)]
pub async fn get_asset_by_id(ctx: AppState, Path(id): Path<i32>) -> Result<Response> {
    let asset = query_as!(
        AssetsDetail,
        r#"
        SELECT
            a.id as "id!",
            a.path as "path!",
            a.version as "unpack_version!",
            a.file as "file!",
            f.hash as "hash!",
            r.bundle as "bundle!"
        FROM assets a
        INNER JOIN unpack_relation r ON a.id = r.asset
        INNER JOIN files f ON a.file = f.id
        WHERE a.id = $1
        "#,
        id,
    )
    .fetch_one(&ctx.database)
    .await?;
    Ok(json(asset))
}

#[debug_handler(state = AppState)]
#[utoipa::path(
    post,
    path = "/asset/bundle",
    tag = "asset",
    description = "把这个bundle某个unpack_version拆出来的所有asset复制到新的unpack_version",
    request_body(content = ReuseAssetFromBundleReq, content_type = "application/json"),
    responses((status = OK))
)]
pub async fn reuse_assets_by_bundle(
    ctx: AppState,
    TokenId(token): TokenId,
    Json(body): Json<ReuseAssetFromBundleReq>,
) -> Result<Response> {
    let mut conn = ctx.begin().await?;
    if let Some(resp) =
        check_version_is_create_by_token(body.new_unpack_version, token, &mut conn).await?
    {
        return Ok(resp);
    }
    let ids = query!(
        "WITH assets_data AS (
            SELECT a.id as id, a.file as file, a.path as path
            FROM assets a
            INNER JOIN unpack_relation r ON a.id = r.asset
            INNER JOIN bundles b ON b.id = r.bundle
            WHERE a.version = $1 AND b.id = $2
        )
        INSERT INTO assets(file, path, version)
        SELECT file, path, $3 FROM assets_data
        RETURNING id",
        body.old_unpack_version,
        body.bundle_id,
        body.new_unpack_version
    )
    .fetch_all(&mut *conn)
    .await?;

    let mut query_builder = QueryBuilder::new("INSERT INTO unpack_relation(asset, bundle) ");
    query_builder.push_values(ids, |mut b, id| {
        b.push_bind(id.id).push_bind(body.bundle_id);
    });
    query_builder.build().execute(&mut *conn).await?;

    conn.commit().await?;

    Ok(StatusCode::OK.into_response())
}

#[debug_handler(state = AppState)]
#[utoipa::path(
    post,
    path = "/asset",
    tag = "asset",
    request_body(content = CreateAssetReq, content_type = "application/json"),
    responses((status = OK))
)]
pub async fn create_asset(
    ctx: AppState,
    TokenId(token): TokenId,
    Json(body): Json<CreateAssetReq>,
) -> Result<Response> {
    let mut conn = ctx.acquire().await?;
    if let Some(resp) =
        check_version_is_create_by_token(body.unpack_version, token, &mut conn).await?
    {
        return Ok(resp);
    }
    query!(
        "INSERT INTO assets(file, path, version) VALUES ($1, $2, $3)",
        body.file,
        body.path,
        body.unpack_version
    )
    .execute(&mut *conn)
    .await?;
    Ok(StatusCode::OK.into_response())
}
