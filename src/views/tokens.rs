use serde::Serialize;
use utoipa::ToSchema;

#[derive(Debug, Serialize, ToSchema)]
pub struct TokenDto {
    pub id: i32,
    pub name: String,
}
