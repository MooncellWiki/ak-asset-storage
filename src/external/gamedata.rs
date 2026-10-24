//! Per-version status of extracted gamedata and the `gamedata/latest` alias.
//!
//! torappu decodes a version's gamedata into
//! `<asset_base>/gamedata/<res_version>/` and writes `.gamedata-ready.json`
//! once every table is written, so that marker — not `versions.is_ready`,
//! which only tracks bundle downloads — says whether the data is complete.
//! `gamedata/latest` is a symlink torappu repoints at the version it is
//! extracting *before* writing the marker, so it can name an unfinished
//! version.

use crate::{external::torappu::TorappuClient, service::story_usage::marker::MARKER_FILE_NAME};
use serde::Serialize;
use std::path::{Component, Path, PathBuf};

pub const LATEST_ALIAS: &str = "latest";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum GamedataStatus {
    /// The ready marker exists: extraction finished.
    Ready,
    /// The directory exists without the marker: extraction is still
    /// running, or the version was extracted before torappu wrote markers.
    Unmarked,
    /// No gamedata directory for this version.
    Missing,
}

/// A version directory name: exactly one plain path segment, never the
/// floating `latest` alias.
fn is_version_name(name: &str) -> bool {
    let mut components = Path::new(name).components();
    matches!(
        (components.next(), components.next()),
        (Some(Component::Normal(_)), None)
    ) && !name.contains(['/', '\\', '\0'])
        && name != LATEST_ALIAS
}

impl TorappuClient {
    fn gamedata_root(&self) -> PathBuf {
        self.asset_base_path.join("gamedata")
    }

    /// Blocking: one or two `stat` calls.
    #[must_use]
    pub fn gamedata_status(&self, res_version: &str) -> GamedataStatus {
        if !is_version_name(res_version) {
            return GamedataStatus::Missing;
        }
        let dir = self.gamedata_root().join(res_version);
        if dir.join(MARKER_FILE_NAME).is_file() {
            GamedataStatus::Ready
        } else if dir.is_dir() {
            GamedataStatus::Unmarked
        } else {
            GamedataStatus::Missing
        }
    }

    /// The version directory `gamedata/latest` links to, if it is a symlink
    /// to a sibling version directory (torappu writes `./<res_version>`).
    #[must_use]
    pub fn gamedata_latest_target(&self) -> Option<String> {
        let target = std::fs::read_link(self.gamedata_root().join(LATEST_ALIAS)).ok()?;
        let sibling = target
            .parent()
            .is_none_or(|parent| parent.as_os_str().is_empty() || parent == Path::new("."));
        let name = target.file_name()?.to_str()?;
        (sibling && is_version_name(name)).then(|| name.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Fixture {
        root: PathBuf,
        client: TorappuClient,
    }

    impl Fixture {
        fn new() -> Self {
            let root = std::env::temp_dir().join(format!(
                "ak-gamedata-{}-{}",
                std::process::id(),
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_nanos()
            ));
            std::fs::create_dir_all(root.join("assets/gamedata")).unwrap();
            let client = TorappuClient {
                asset_base_path: root.join("assets"),
            };
            Self { root, client }
        }

        fn write(&self, path: &str, content: &[u8]) -> PathBuf {
            let path = self.client.asset_base_path.join(path);
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(&path, content).unwrap();
            path
        }
    }

    impl Drop for Fixture {
        fn drop(&mut self) {
            std::fs::remove_dir_all(&self.root).unwrap();
        }
    }

    #[test]
    fn status_distinguishes_ready_unmarked_and_missing() {
        let fixture = Fixture::new();
        fixture.write("gamedata/v2/.gamedata-ready.json", b"{}");
        fixture.write("gamedata/v1/excel/item_table.json", b"{}");
        assert_eq!(fixture.client.gamedata_status("v2"), GamedataStatus::Ready);
        assert_eq!(
            fixture.client.gamedata_status("v1"),
            GamedataStatus::Unmarked
        );
        assert_eq!(
            fixture.client.gamedata_status("v0"),
            GamedataStatus::Missing
        );
        for invalid in ["", "latest", "..", "v2/excel", "/v2"] {
            assert_eq!(
                fixture.client.gamedata_status(invalid),
                GamedataStatus::Missing,
                "{invalid:?}"
            );
        }
    }

    #[test]
    fn latest_alias_resolves_only_sibling_version_links() {
        let fixture = Fixture::new();
        let gamedata = fixture.client.asset_base_path.join("gamedata");
        std::fs::create_dir_all(gamedata.join("v2")).unwrap();
        assert_eq!(fixture.client.gamedata_latest_target(), None);
        std::os::unix::fs::symlink("./v2", gamedata.join("latest")).unwrap();
        assert_eq!(
            fixture.client.gamedata_latest_target().as_deref(),
            Some("v2")
        );
        std::fs::remove_file(gamedata.join("latest")).unwrap();
        std::os::unix::fs::symlink("/etc", gamedata.join("latest")).unwrap();
        assert_eq!(fixture.client.gamedata_latest_target(), None);
    }
}
