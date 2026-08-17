//! `.gamedata-ready.json` marker parsing and validation
//! (docs/story-reference-phase-1-design.md §3, §6.1).

use std::path::Path;

use anyhow::{Context, bail};
use serde::Deserialize;

pub const MARKER_FILE_NAME: &str = ".gamedata-ready.json";
pub const SUPPORTED_SCHEMA_VERSION: i64 = 1;

#[derive(Debug, Clone, Deserialize)]
pub struct GameDataReadyMarker {
    pub schema_version: i64,
    #[serde(default)]
    pub completed_at: Option<String>,
}

pub fn parse_marker(path: &Path) -> anyhow::Result<GameDataReadyMarker> {
    let content = std::fs::read_to_string(path)
        .with_context(|| format!("failed to read gamedata ready marker: {}", path.display()))?;
    serde_json::from_str(&content)
        .with_context(|| format!("failed to parse gamedata ready marker: {}", path.display()))
}

/// Validates the marker. Fails before any database transaction is opened.
pub fn validate_marker(marker: &GameDataReadyMarker) -> anyhow::Result<()> {
    if marker.schema_version != SUPPORTED_SCHEMA_VERSION {
        bail!(
            "unsupported gamedata ready marker schema_version {} (expected {SUPPORTED_SCHEMA_VERSION})",
            marker.schema_version
        );
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn valid_marker_json() -> String {
        r#"{"schema_version": 1, "completed_at": "2026-08-17T10:00:00Z"}"#.to_string()
    }

    #[test]
    fn parses_marker() {
        let marker: GameDataReadyMarker =
            serde_json::from_str(&valid_marker_json()).expect("valid marker");
        assert_eq!(marker.schema_version, 1);
        assert_eq!(marker.completed_at.as_deref(), Some("2026-08-17T10:00:00Z"));
        validate_marker(&marker).expect("marker should validate");
    }

    #[test]
    fn parses_marker_without_completed_at() {
        let marker: GameDataReadyMarker =
            serde_json::from_str(r#"{"schema_version": 1}"#).expect("valid marker");
        assert!(marker.completed_at.is_none());
        validate_marker(&marker).expect("marker should validate");
    }

    #[test]
    fn rejects_unsupported_schema_version() {
        let marker: GameDataReadyMarker = serde_json::from_str(
            &valid_marker_json().replace("\"schema_version\": 1", "\"schema_version\": 2"),
        )
        .expect("parses");
        assert!(validate_marker(&marker).is_err());
    }

    #[test]
    fn rejects_marker_without_schema_version() {
        let missing: Result<GameDataReadyMarker, _> =
            serde_json::from_str(r#"{"completed_at": "2026-08-17T10:00:00Z"}"#);
        assert!(missing.is_err());
    }
}
