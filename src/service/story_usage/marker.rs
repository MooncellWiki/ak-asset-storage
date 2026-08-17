//! `.gamedata-ready.json` marker parsing and validation
//! (docs/story-reference-phase-1-design.md §3, §6.1).

use std::path::Path;

use anyhow::{Context, anyhow, bail};
use serde::Deserialize;

pub const MARKER_FILE_NAME: &str = ".gamedata-ready.json";
pub const SUPPORTED_SCHEMA_VERSION: i64 = 1;

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GameDataReadyMarker {
    pub schema_version: i64,
    #[serde(default)]
    pub task: String,
    #[serde(default)]
    pub client_version: String,
    pub res_version: String,
    #[serde(default)]
    pub completed_at: Option<String>,
    #[serde(default)]
    pub producer: Option<MarkerProducer>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MarkerProducer {
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub revision: String,
}

pub fn parse_marker(path: &Path) -> anyhow::Result<GameDataReadyMarker> {
    let content = std::fs::read_to_string(path)
        .with_context(|| format!("failed to read gamedata ready marker: {}", path.display()))?;
    serde_json::from_str(&content)
        .with_context(|| format!("failed to parse gamedata ready marker: {}", path.display()))
}

/// Validates the marker against the resolved marker file path. Fails before
/// any database transaction is opened.
pub fn validate_marker(
    marker: &GameDataReadyMarker,
    resolved_marker_path: &Path,
) -> anyhow::Result<()> {
    if marker.schema_version != SUPPORTED_SCHEMA_VERSION {
        bail!(
            "unsupported gamedata ready marker schemaVersion {} (expected {SUPPORTED_SCHEMA_VERSION})",
            marker.schema_version
        );
    }
    if marker.task != "GameData" {
        bail!(
            "gamedata ready marker task must be \"GameData\", got {:?}",
            marker.task
        );
    }
    if marker.res_version.is_empty() {
        bail!("gamedata ready marker resVersion must not be empty");
    }

    let version_dir_name = resolved_marker_path
        .parent()
        .and_then(Path::file_name)
        .and_then(std::ffi::OsStr::to_str)
        .ok_or_else(|| {
            anyhow!(
                "failed to extract version directory from marker path: {}",
                resolved_marker_path.display()
            )
        })?;
    if version_dir_name != marker.res_version {
        bail!(
            "gamedata ready marker resVersion {:?} does not match version directory {:?}",
            marker.res_version,
            version_dir_name
        );
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn marker_path() -> std::path::PathBuf {
        Path::new("/data/gamedata/26-08-07-14-53-29_30b8f0").join(MARKER_FILE_NAME)
    }

    fn valid_marker_json() -> String {
        r#"{
  "schemaVersion": 1,
  "task": "GameData",
  "clientVersion": "2.6.01",
  "resVersion": "26-08-07-14-53-29_30b8f0",
  "completedAt": "2026-08-17T10:00:00Z",
  "producer": { "name": "torappu", "revision": "test" }
}"#
        .to_string()
    }

    #[test]
    fn parses_full_marker() {
        let marker: GameDataReadyMarker =
            serde_json::from_str(&valid_marker_json()).expect("valid marker");
        assert_eq!(marker.schema_version, 1);
        assert_eq!(marker.task, "GameData");
        assert_eq!(marker.res_version, "26-08-07-14-53-29_30b8f0");
        assert_eq!(marker.client_version, "2.6.01");
        assert_eq!(marker.producer.as_ref().expect("producer").name, "torappu");
        validate_marker(&marker, &marker_path()).expect("marker should validate");
    }

    #[test]
    fn rejects_unsupported_schema_version() {
        let marker: GameDataReadyMarker = serde_json::from_str(
            &valid_marker_json().replace("\"schemaVersion\": 1", "\"schemaVersion\": 2"),
        )
        .expect("parses");
        assert!(validate_marker(&marker, &marker_path()).is_err());
    }

    #[test]
    fn rejects_wrong_task() {
        let marker: GameDataReadyMarker = serde_json::from_str(
            &valid_marker_json().replace("\"task\": \"GameData\"", "\"task\": \"Other\""),
        )
        .expect("parses");
        assert!(validate_marker(&marker, &marker_path()).is_err());
    }

    #[test]
    fn rejects_empty_or_missing_res_version() {
        let missing: Result<GameDataReadyMarker, _> = serde_json::from_str(
            &valid_marker_json().replace("\"resVersion\": \"26-08-07-14-53-29_30b8f0\",", ""),
        );
        assert!(missing.is_err());

        let marker: GameDataReadyMarker =
            serde_json::from_str(&valid_marker_json().replace("26-08-07-14-53-29_30b8f0", ""))
                .expect("parses");
        assert!(validate_marker(&marker, &marker_path()).is_err());
    }

    #[test]
    fn rejects_res_version_directory_mismatch() {
        let marker: GameDataReadyMarker =
            serde_json::from_str(&valid_marker_json()).expect("parses");
        let other = Path::new("/data/gamedata/26-07-08-12-06-40_24a544").join(MARKER_FILE_NAME);
        assert!(validate_marker(&marker, &other).is_err());
    }

    #[test]
    fn rejects_snake_case_marker_from_old_producers() {
        let legacy = r#"{"schema_version": 1, "completed_at": "2026-08-17T10:00:00Z"}"#;
        assert!(serde_json::from_str::<GameDataReadyMarker>(legacy).is_err());
    }
}
