use crate::{
    app::AppState,
    error::Result,
    utils::token::{check_version_is_create_by_token, TokenId},
    views::{
        unpack::{
            CreateUnpackVersionReq, CreateUnpackVersionResp, SearchUnpackVersionReq,
            UnpackVersionDetailDto, UnpackVersionDto,
        },
        utils::json,
    },
};
use axum::{
    debug_handler,
    extract::Path,
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use sqlx::{query, query_as};

#[debug_handler(state = AppState)]
#[utoipa::path(
    get,
    path = "/unpack-version",
    tag="unpack-version",
    params(SearchUnpackVersionReq),
    responses((status = OK, body = Vec<UnpackVersionDetailDto>))
)]
pub async fn list_unpack_versions(
    ctx: AppState,
    params: SearchUnpackVersionReq,
) -> Result<Response> {
    let versions = query_as!(
        UnpackVersionDetailDto,
        r#"
        SELECT uv.id as id, uv.token as token, uv.bundle_version as bundle_version, uv.start_time as start_time, uv.end_time as end_time, t.name as token_name, v.res as res, v.client as client
        FROM unpack_versions uv
        INNER JOIN tokens t ON uv.token = t.id
        INNER JOIN versions v ON uv.bundle_version = v.id
        WHERE
            ($1::timestamp IS NULL OR start_time >= $1)
            AND ($2::timestamp IS NULL OR end_time <= $2)
            AND ($3::int IS NULL OR bundle_version = $3)
            AND ($4::int IS NULL OR uv.token = $4)
        ORDER BY start_time DESC
        "#,
        params.start_time,
        params.end_time,
        params.bundle_version,
        params.token
    )
    .fetch_all(&ctx.database)
    .await?;

    Ok((StatusCode::OK, Json(versions)).into_response())
}

#[debug_handler(state = AppState)]
#[utoipa::path(
    get,
    path = "/unpack-version/latest-finished",
    tag="unpack-version",
    responses((status = OK, body = UnpackVersionDto))
)]
pub async fn get_latest_finished_version(ctx: AppState) -> Result<Response> {
    let version = query_as!(
        UnpackVersionDto,
        r#"
        SELECT *
        FROM unpack_versions
        WHERE end_time IS NOT NULL
        ORDER BY end_time DESC
        LIMIT 1
        "#
    )
    .fetch_one(&ctx.database)
    .await?;

    Ok((StatusCode::OK, Json(version)).into_response())
}

#[debug_handler(state = AppState)]
#[utoipa::path(
    post,
    path = "/unpack-version",
    tag="unpack-version",
    responses(
        (status = OK, body = CreateUnpackVersionResp),
        (status = CONFLICT, body = UnpackVersionDto)
    )
)]
pub async fn create_unpack_version(
    ctx: AppState,
    TokenId(token): TokenId,
    Json(body): Json<CreateUnpackVersionReq>,
) -> Result<Response> {
    let mut conn = ctx.acquire().await?;
    let unfinished = query!("SELECT * from unpack_versions where end_time IS NULL")
        .fetch_optional(&mut *conn)
        .await?;
    if let Some(unfinished) = unfinished {
        return Ok((
            StatusCode::CONFLICT,
            json(UnpackVersionDto {
                id: unfinished.id,
                token: unfinished.token,
                bundle_version: unfinished.bundle_version,
                start_time: unfinished.start_time,
                end_time: unfinished.end_time,
            }),
        )
            .into_response());
    }
    let result = query!(
        "INSERT INTO unpack_versions (bundle_version, token) VALUES ($1, $2) RETURNING id",
        body.bundle_version,
        token
    )
    .fetch_one(&ctx.database)
    .await?;
    Ok(json(CreateUnpackVersionResp { version: result.id }))
}

#[debug_handler(state = AppState)]
#[utoipa::path(
    post,
    path = "/unpack-version/{id}",
    tag="unpack-version",
    responses((status = OK))
)]
pub async fn finish_unpack_version(
    ctx: AppState,
    TokenId(token): TokenId,
    Path(id): Path<i32>,
) -> Result<Response> {
    let mut conn = ctx.acquire().await?;
    if let Some(resp) = check_version_is_create_by_token(id, token, &mut conn).await? {
        return Ok(resp);
    }
    let result = query!(
        "UPDATE unpack_versions SET end_time = CURRENT_TIMESTAMP WHERE id = $1 AND end_time IS NULL",
        id
    )
    .execute(&mut* conn)
    .await?;
    if result.rows_affected() == 0 {
        return Ok(StatusCode::NOT_FOUND.into_response());
    }
    Ok(StatusCode::OK.into_response())
}
