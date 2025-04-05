use crate::{
    app::AppState,
    error::Result,
    utils::{self, token::TokenId},
    views::{
        files::{CreateFileReq, CreateFileResp},
        utils::json,
    },
};
use axum::{
    debug_handler,
    extract::Multipart,
    response::{IntoResponse, Response},
};
use reqwest::StatusCode;

#[debug_handler(state = AppState)]
#[utoipa::path(
    post,
    path = "/file",
    tag = "file",
    request_body(content = CreateFileReq, content_type = "multipart/form-data"),
    responses((status = OK, body = CreateFileResp))
)]
pub async fn upload(ctx: AppState, _token: TokenId, mut body: Multipart) -> Result<Response> {
    let file = body.next_field().await;
    if let Err(e) = file {
        return Ok(e.into_response());
    }
    let file = file.unwrap();
    if let Some(file) = file {
        let content = file.bytes().await;
        if let Err(e) = content {
            return Ok(e.into_response());
        }
        let bytes = content.unwrap().to_vec();
        let mut conn = ctx.database.acquire().await?;
        let id = utils::upload::upload(&mut conn, &ctx.s3, bytes).await?;
        Ok(json(CreateFileResp { id }))
    } else {
        Ok(StatusCode::BAD_REQUEST.into_response())
    }
}
