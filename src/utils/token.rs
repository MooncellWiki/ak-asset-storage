use crate::app::AppState;
use anyhow::Result;
use axum::{
    async_trait,
    extract::FromRequestParts,
    http::{header::AUTHORIZATION, request::Parts, StatusCode},
    response::{IntoResponse, Response},
};
use rand_chacha::{rand_core::SeedableRng, ChaCha20Rng};
use rand_core::RngCore;
use sqlx::{query, PgConnection};

/// 用chacha20生成一个256位（64个hex）长的随机数
pub fn create_token() -> Result<String> {
    let mut random = ChaCha20Rng::from_os_rng();
    let random = format!(
        "{:x}{:x}{:x}{:x}",
        random.next_u64(),
        random.next_u64(),
        random.next_u64(),
        random.next_u64()
    );
    Ok(random)
}

pub struct TokenId(pub i32);
#[async_trait]
impl FromRequestParts<AppState> for TokenId {
    type Rejection = StatusCode;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        if let Some(token) = parts.headers.get(AUTHORIZATION) {
            if let Ok(token) = token.to_str() {
                let token = token.replace("Bearer ", "");
                let id = sqlx::query!("SELECT id FROM tokens WHERE token = $1", token)
                    .fetch_one(&state.as_ref().database)
                    .await
                    .map_err(|_| StatusCode::UNAUTHORIZED)?;
                Ok(Self(id.id))
            } else {
                Err(StatusCode::BAD_REQUEST)
            }
        } else {
            Err(StatusCode::UNAUTHORIZED)
        }
    }
}

/// 检查这个 `unpack_version` 是否是这个token创建的, 如果不是, 返回Some(Response)
pub async fn check_version_is_create_by_token(
    unpack_verion_id: i32,
    token_id: i32,
    conn: &mut PgConnection,
) -> Result<Option<Response>> {
    let result = query!(
        "SELECT token FROM unpack_versions WHERE id = $1",
        unpack_verion_id
    )
    .fetch_optional(conn)
    .await?;
    if let Some(result) = result {
        if result.token != token_id {
            return Ok(Some(StatusCode::UNAUTHORIZED.into_response()));
        }
    } else {
        return Ok(Some(StatusCode::NOT_FOUND.into_response()));
    }
    Ok(None)
}
