//! Story resource usage import (phase 1): watches torappu's GameData-ready
//! marker, parses every story txt under the resolved version, and replaces
//! the `story_resource_usages` snapshot in one transaction.

pub mod character_links;
pub mod extract;
pub mod marker;
pub mod parser;

use std::{
    fs,
    path::{Path, PathBuf},
};

use anyhow::{Context, bail};
use character_links::CharacterLinks;
use walkdir::WalkDir;

use crate::{
    AppError, AppResult,
    database::{Database, row::StoryUsageRow},
};
use marker::{MARKER_FILE_NAME, parse_marker, validate_marker};

const UC_INFO_PREFIX: &str = "[uc]info/";

#[derive(Debug, Clone)]
pub struct StoryUsageImportService {
    pub database: Database,
    pub gamedata_root: PathBuf,
}

impl StoryUsageImportService {
    pub async fn import(&self) -> AppResult<()> {
        let asset_root = self
            .gamedata_root
            .parent()
            .context("gamedata root has no parent asset root")?
            .to_path_buf();
        let links = tokio::task::spawn_blocking(move || CharacterLinks::load(&asset_root))
            .await
            .map_err(|err| {
                AppError::Application(anyhow::anyhow!(
                    "character link map load task failed: {err}"
                ))
            })?
            .map_err(AppError::Application)?;

        let gamedata_root = self.gamedata_root.clone();
        let rows = tokio::task::spawn_blocking(move || build_snapshot(&gamedata_root, &links))
            .await
            .map_err(|err| {
                AppError::Application(anyhow::anyhow!("story usage snapshot task failed: {err}"))
            })?
            .map_err(AppError::Application)?;

        let script_count = rows
            .iter()
            .map(|row| row.script_path.as_str())
            .collect::<std::collections::HashSet<_>>()
            .len();
        self.database.replace_story_resource_usages(&rows).await?;
        tracing::info!(
            scripts = script_count,
            rows = rows.len(),
            "story resource usage import finished"
        );
        Ok(())
    }
}

/// Resolves `gamedata/latest/.gamedata-ready.json`, validates the marker, then
/// builds the full snapshot from that version's `story/` directory. All file
/// reads happen outside any database transaction.
fn build_snapshot(
    gamedata_root: &Path,
    links: &CharacterLinks,
) -> anyhow::Result<Vec<StoryUsageRow>> {
    let logical_marker = gamedata_root.join("latest").join(MARKER_FILE_NAME);
    let resolved_marker = fs::canonicalize(&logical_marker).with_context(|| {
        format!(
            "gamedata ready marker is not reachable: {}",
            logical_marker.display()
        )
    })?;

    let marker = parse_marker(&resolved_marker)?;
    validate_marker(&marker)?;

    let version_dir = resolved_marker
        .parent()
        .context("marker path has no parent directory")?;
    let story_dir = version_dir.join("story");
    if !story_dir.is_dir() {
        bail!(
            "story directory does not exist under {}: {}",
            version_dir.display(),
            story_dir.display()
        );
    }

    let files = discover_story_files(&story_dir)?;
    let mut rows = Vec::new();
    for (script_path, path) in files {
        let content = fs::read_to_string(&path)
            .with_context(|| format!("failed to read story script: {}", path.display()))?;
        let parsed = parser::parse_script(&content);
        drop(content);
        for usage in extract::extract_usages(&parsed) {
            rows.push(StoryUsageRow {
                script_path: script_path.clone(),
                listing_id: links.listing_id(&usage.resource_type, &usage.resource_id),
                resource_type: usage.resource_type,
                resource_id: usage.resource_id,
                display_names: usage.display_names,
                sort_order: i32::try_from(usage.sort_order)
                    .expect("per-script resource count fits i32"),
            });
        }
    }
    Ok(rows)
}

/// Recursively finds `story/**/*.txt`, excluding `[uc]info/**`. `script_path`
/// is relative to `story/`, without the `.txt` extension, always `/`-joined.
fn discover_story_files(story_dir: &Path) -> anyhow::Result<Vec<(String, PathBuf)>> {
    let mut files = Vec::new();
    for entry in WalkDir::new(story_dir).follow_links(false) {
        let entry = entry
            .with_context(|| format!("failed to walk story directory: {}", story_dir.display()))?;
        if !entry.file_type().is_file() {
            continue;
        }
        if entry.path().extension().is_none_or(|ext| ext != "txt") {
            continue;
        }

        let relative = entry
            .path()
            .strip_prefix(story_dir)
            .expect("walkdir entries are under the root");
        let mut script_path = String::new();
        for component in relative.components() {
            if !script_path.is_empty() {
                script_path.push('/');
            }
            script_path.push_str(
                component
                    .as_os_str()
                    .to_str()
                    .with_context(|| format!("non-utf8 story path: {}", entry.path().display()))?,
            );
        }
        let script_path = script_path
            .strip_suffix(".txt")
            .unwrap_or(&script_path)
            .to_string();

        if script_path.starts_with(UC_INFO_PREFIX) {
            continue;
        }
        files.push((script_path, entry.path().to_path_buf()));
    }

    files.sort_by(|a, b| a.0.cmp(&b.0));
    Ok(files)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write(path: &Path, content: &str) {
        std::fs::create_dir_all(path.parent().expect("parent")).expect("create dir");
        std::fs::write(path, content).expect("write file");
    }

    fn marker_json() -> String {
        r#"{"schema_version":1,"completed_at":"2026-08-17T10:00:00Z"}"#.to_string()
    }

    /// Link map with one full-image character; every other fixture base is
    /// unknown and keeps the body-collapse fallback.
    fn links() -> CharacterLinks {
        CharacterLinks::parse(
            r#"{
                "char_img_1": {
                    "pos": {"x": 0, "y": 190}, "size": {"x": 970, "y": 970}, "groups": [],
                    "array": [
                        {"name": "char_img_1", "alias": "normal", "group": -1,
                         "image": "char_img_1/char_img_1"},
                        {"name": "char_img_1_2", "alias": "smile", "group": -1,
                         "image": "char_img_1/char_img_1_2"}
                    ]
                }
            }"#,
        )
        .expect("fixture link map")
    }

    fn setup_gamedata(root: &Path, res_version: &str) {
        let version_dir = root.join(res_version);
        write(
            &version_dir.join("story/activities/a001/level_a001_01_beg.txt"),
            concat!(
                "[HEADER(key=\"title_test\", is_skippable=true)] 第一关（前）\n",
                "[Dialog]\n",
                "[Background(image=\"bg_med\", fadetime=2,block=true)]\n",
                "[Image(image=\"ac1_0\")]\n",
                "[ShowItem(image=\"item_caster\")]\n",
                "[Character(name=\"avg_npc_009\")]\n",
                "[name=\"赏金猎人\"]   这女人，还不肯说吗？\n",
                "[Character(name=\"char_img_1#2\")]\n",
            ),
        );
        write(
            &version_dir.join("story/[uc]info/activities/a001/level_a001_01_beg.txt"),
            "[Dialog]\n[Background(image=\"bg_should_be_ignored\")]\n",
        );
        write(&version_dir.join("story/story_variables.json"), "{}");
        write(&version_dir.join(MARKER_FILE_NAME), &marker_json());
        #[cfg(unix)]
        std::os::unix::fs::symlink(res_version, root.join("latest")).expect("symlink latest");
    }

    #[test]
    fn builds_snapshot_from_latest_version() {
        let root =
            std::env::temp_dir().join(format!("ak-story-usage-snapshot-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        setup_gamedata(&root, "26-08-07-14-53-29_30b8f0");

        let rows = build_snapshot(&root, &links()).expect("snapshot");

        let background = rows
            .iter()
            .find(|row| row.resource_type == "background")
            .expect("background row");
        assert_eq!(background.resource_id, "bg_med");
        assert_eq!(background.script_path, "activities/a001/level_a001_01_beg");

        let character = rows
            .iter()
            .find(|row| row.resource_id == "avg_npc_009#1$1")
            .expect("character row");
        assert_eq!(character.display_names, vec!["赏金猎人"]);
        // Unknown base keeps the body-collapse fallback.
        assert_eq!(character.listing_id, "avg_npc_009$1");

        // Full-image characters keep their face-level ids: every `#face` ref
        // is a standalone png, not a face overlay on a shared body.
        let full_image = rows
            .iter()
            .find(|row| row.resource_id == "char_img_1#2$1")
            .expect("full-image character row");
        assert_eq!(full_image.listing_id, "char_img_1#2$1");

        assert!(rows.iter().any(|row| row.resource_id == "ac1_0"));
        assert!(rows.iter().any(|row| row.resource_id == "item_caster"));
        // [uc]info/** and non-txt files must not contribute rows.
        assert!(
            !rows
                .iter()
                .any(|row| row.resource_id == "bg_should_be_ignored")
        );

        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn snapshot_fails_when_latest_switches_to_incomplete_version() {
        let root =
            std::env::temp_dir().join(format!("ak-story-usage-incomplete-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        setup_gamedata(&root, "26-08-07-14-53-29_30b8f0");

        // New version dir with a story dir but no marker yet; latest still
        // points at the old complete version.
        write(
            &root.join("26-09-01-00-00-00_abcdef/story/obt/main/level_main_00-01.txt"),
            "[Background(image=\"bg_black\")]\n",
        );

        let rows = build_snapshot(&root, &links()).expect("snapshot from old latest");
        assert!(
            rows.iter()
                .all(|row| row.script_path.starts_with("activities/"))
        );

        // Point latest at a directory without a marker → import must fail.
        std::fs::remove_file(root.join("latest")).expect("remove latest");
        #[cfg(unix)]
        std::os::unix::fs::symlink("26-09-01-00-00-00_abcdef", root.join("latest"))
            .expect("relink latest");
        assert!(build_snapshot(&root, &links()).is_err());

        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn discovery_excludes_uc_info_and_sorts() {
        let story_dir =
            std::env::temp_dir().join(format!("ak-story-usage-discover-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&story_dir);
        write(&story_dir.join("obt/main/b.txt"), "");
        write(&story_dir.join("activities/a001/a.txt"), "");
        write(&story_dir.join("[uc]info/obt/skip.txt"), "");
        write(&story_dir.join("obt/notes.json"), "");

        let files = discover_story_files(&story_dir).expect("discover");
        let paths: Vec<&str> = files.iter().map(|(path, _)| path.as_str()).collect();
        assert_eq!(paths, vec!["activities/a001/a", "obt/main/b"]);

        let _ = std::fs::remove_dir_all(&story_dir);
    }
}
