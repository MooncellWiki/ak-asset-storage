use crate::{
    app::AppState,
    error::Result,
    views::{tokens::TokenDto, utils::json},
};
use axum::{debug_handler, response::Response};
use sqlx::query_as;

#[debug_handler(state = AppState)]
#[utoipa::path(
    get,
    path = "/token",
    tag = "token",
    responses((status = OK, body = [TokenDto]))
)]
pub async fn list_tokens(ctx: AppState) -> Result<Response> {
    let result = query_as!(
        TokenDto,
        r#"
        SELECT id, name
        FROM tokens
        ORDER BY id
        "#
    )
    .fetch_all(&mut *ctx.acquire().await?)
    .await?;
    Ok(json(result))
}
