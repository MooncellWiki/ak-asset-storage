//! Opaque pagination cursors shared by the REST handlers and the MCP tools.
//!
//! base64url(JSON), so a cursor survives being echoed back inside a query
//! string or a tool argument regardless of the characters (`#`, `$`, `/`,
//! spaces) embedded in ids and script paths.

use crate::{api::error::WebError, database::row::StoryResourceType};
use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use serde::{Deserialize, Serialize};

pub trait CursorPayload {
    fn is_valid(&self) -> bool;
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct UsageCursor {
    pub script_path: String,
}

impl CursorPayload for UsageCursor {
    fn is_valid(&self) -> bool {
        !self.script_path.contains('\0')
    }
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct ResourceCursor {
    pub resource_type: StoryResourceType,
    pub resource_id: String,
}

impl CursorPayload for ResourceCursor {
    fn is_valid(&self) -> bool {
        !self.resource_id.contains('\0')
    }
}

pub fn encode<T: Serialize>(value: &T) -> String {
    let json = serde_json::to_string(value).expect("cursor serialization is infallible");
    URL_SAFE_NO_PAD.encode(json)
}

pub fn decode<T: for<'de> Deserialize<'de> + CursorPayload>(
    cursor: Option<&str>,
) -> Result<Option<T>, WebError> {
    let Some(cursor) = cursor.filter(|value| !value.is_empty()) else {
        return Ok(None);
    };
    let json = URL_SAFE_NO_PAD
        .decode(cursor)
        .map_err(|_| WebError::BadRequest("invalid cursor".to_string()))?;
    let decoded: T = serde_json::from_slice(&json)
        .map_err(|_| WebError::BadRequest("invalid cursor".to_string()))?;
    if !decoded.is_valid() {
        return Err(WebError::BadRequest("invalid cursor".to_string()));
    }
    Ok(Some(decoded))
}
