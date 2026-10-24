//! Read access to extracted gamedata: per-version status, file lookup, JSON
//! parsing with pointer outlines, and UTF-8 text slices.
//!
//! torappu decodes a version's gamedata into
//! `<asset_base>/gamedata/<res_version>/` and writes `.gamedata-ready.json`
//! once every table is written, so that marker — not `versions.is_ready`,
//! which only tracks bundle downloads — says whether the data is complete.
//! `gamedata/latest` is a symlink torappu repoints at the version it is
//! extracting *before* writing the marker, so it can name an unfinished
//! version.

use crate::{
    AppError,
    external::torappu::{TorappuClient, relative_asset_path},
    service::story_usage::marker::MARKER_FILE_NAME,
};
use serde::Serialize;
use std::{
    fs::File,
    io::{Read as _, Seek as _, SeekFrom, Write},
    path::{Component, Path, PathBuf},
};

pub const LATEST_ALIAS: &str = "latest";

/// Largest file parsed as JSON. A parsed `serde_json::Value` takes several
/// times the file size; the largest tables are around 10 MB.
pub const MAX_JSON_FILE_BYTES: u64 = 64 * 1024 * 1024;

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

#[derive(Debug)]
pub enum GamedataError {
    /// The request named something that does not exist or cannot be read
    /// this way; the message (safe to echo) says what to try instead.
    Miss(String),
    Failed(AppError),
}

impl From<AppError> for GamedataError {
    fn from(error: AppError) -> Self {
        Self::Failed(error)
    }
}

impl From<std::io::Error> for GamedataError {
    fn from(error: std::io::Error) -> Self {
        Self::Failed(error.into())
    }
}

pub type GamedataResult<T> = Result<T, GamedataError>;

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

    /// Resolves `path` inside one version's gamedata to an existing regular
    /// file. Symlinks are followed but must stay inside the asset root.
    /// Blocking.
    pub fn gamedata_file(&self, res_version: &str, path: &str) -> GamedataResult<PathBuf> {
        if !is_version_name(res_version) {
            return Err(AppError::InvalidInput(
                "res_version must be a single version directory name".to_string(),
            )
            .into());
        }
        let relative = relative_asset_path(path)?;
        if relative
            .components()
            .all(|component| component == Component::CurDir)
        {
            return Err(AppError::InvalidInput(
                "path must name a file inside the version's gamedata, \
                 e.g. excel/character_table.json"
                    .to_string(),
            )
            .into());
        }
        let version_dir = self.gamedata_root().join(res_version);
        if !version_dir.is_dir() {
            return Err(GamedataError::Miss(format!(
                "no gamedata directory for version {res_version:?}; \
                 pick one whose gamedataStatus is ready in list_versions"
            )));
        }
        let file = version_dir.join(relative);
        let metadata = match std::fs::metadata(&file) {
            Ok(metadata) => metadata,
            Err(err)
                if matches!(
                    err.kind(),
                    std::io::ErrorKind::NotFound | std::io::ErrorKind::NotADirectory
                ) =>
            {
                return Err(GamedataError::Miss(format!(
                    "no file {path:?} in gamedata/{res_version}; browse it with list_files \
                     (path \"gamedata/{res_version}\") or search_files with \
                     path_prefix \"gamedata/{res_version}/\""
                )));
            }
            Err(err) => return Err(err.into()),
        };
        if metadata.is_dir() {
            return Err(GamedataError::Miss(format!(
                "{path:?} is a directory; list it with list_files \
                 (path \"gamedata/{res_version}/{path}\")"
            )));
        }
        let root = self.asset_base_path.canonicalize()?;
        let resolved = file.canonicalize()?;
        if !resolved.starts_with(&root) {
            return Err(GamedataError::Miss(format!(
                "{path:?} links outside the asset tree"
            )));
        }
        Ok(resolved)
    }
}

/// Parses a gamedata JSON file (a leading UTF-8 BOM is ignored). Blocking.
pub fn read_json(file: &Path) -> GamedataResult<serde_json::Value> {
    let size = std::fs::metadata(file)?.len();
    if size > MAX_JSON_FILE_BYTES {
        return Err(GamedataError::Miss(format!(
            "file is {size} bytes, over the {MAX_JSON_FILE_BYTES}-byte parse limit; \
             download it from url instead"
        )));
    }
    let bytes = std::fs::read(file)?;
    let bytes = bytes.strip_prefix(b"\xEF\xBB\xBF").unwrap_or(&bytes);
    serde_json::from_slice(bytes).map_err(|err| {
        GamedataError::Miss(format!(
            "file is not valid JSON ({err}); download it from url instead"
        ))
    })
}

/// Escapes one JSON Pointer reference token (RFC 6901: `~` then `/`).
fn escape_pointer_token(token: &str) -> String {
    token.replace('~', "~0").replace('/', "~1")
}

/// Compact-serialized size of a value, without allocating the output.
#[must_use]
pub fn json_size(value: &serde_json::Value) -> usize {
    struct Counter(usize);
    impl Write for Counter {
        fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
            self.0 += buf.len();
            Ok(buf.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
    let mut counter = Counter(0);
    // Serializing a `Value` into an infallible writer cannot fail.
    let _ = serde_json::to_writer(&mut counter, value);
    counter.0
}

#[must_use]
pub const fn json_type(value: &serde_json::Value) -> &'static str {
    match value {
        serde_json::Value::Null => "null",
        serde_json::Value::Bool(_) => "boolean",
        serde_json::Value::Number(_) => "number",
        serde_json::Value::String(_) => "string",
        serde_json::Value::Array(_) => "array",
        serde_json::Value::Object(_) => "object",
    }
}

/// One child of an outlined object or array.
#[derive(Debug, Serialize, PartialEq, Eq)]
pub struct OutlineEntry {
    /// Object key, or the array index as a string.
    pub key: String,
    /// Full JSON Pointer of this child, ready to pass back as `pointer`.
    pub pointer: String,
    #[serde(rename = "type")]
    pub kind: &'static str,
    /// Number of children, for objects and arrays.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub len: Option<usize>,
    /// Compact-serialized size, to judge whether reading it fits.
    pub bytes: usize,
}

/// A page of the children of the value at a pointer.
#[derive(Debug, Serialize, PartialEq, Eq)]
pub struct Outline {
    #[serde(rename = "type")]
    pub kind: &'static str,
    /// Number of children (objects and arrays) or 0 for scalars.
    pub len: usize,
    /// Compact-serialized size of the whole value.
    pub bytes: usize,
    pub entries: Vec<OutlineEntry>,
    /// Index of the first entry listed.
    pub offset: usize,
    /// Pass back as `offset` to list the next entries.
    pub next_offset: Option<usize>,
}

/// Lists a page of the children of `value`, starting at `offset`.
///
/// The page stops before the entries' combined serialized size would exceed
/// `budget` bytes. At least one entry is listed when any remain, so paging
/// always advances.
#[must_use]
pub fn outline(value: &serde_json::Value, pointer: &str, offset: usize, budget: usize) -> Outline {
    let children: Vec<(String, &serde_json::Value)> = match value {
        serde_json::Value::Object(map) => map
            .iter()
            .skip(offset)
            .map(|(key, child)| (key.clone(), child))
            .collect(),
        serde_json::Value::Array(items) => items
            .iter()
            .enumerate()
            .skip(offset)
            .map(|(index, child)| (index.to_string(), child))
            .collect(),
        _ => Vec::new(),
    };
    let len = match value {
        serde_json::Value::Object(map) => map.len(),
        serde_json::Value::Array(items) => items.len(),
        _ => 0,
    };
    let mut entries = Vec::new();
    let mut used = 0;
    for (key, child) in children {
        let entry = OutlineEntry {
            pointer: format!("{pointer}/{}", escape_pointer_token(&key)),
            key,
            kind: json_type(child),
            len: match child {
                serde_json::Value::Object(map) => Some(map.len()),
                serde_json::Value::Array(items) => Some(items.len()),
                _ => None,
            },
            bytes: json_size(child),
        };
        let cost = serde_json::to_vec(&entry).map_or(0, |encoded| encoded.len() + 1);
        if !entries.is_empty() && used + cost > budget {
            break;
        }
        used += cost;
        entries.push(entry);
    }
    let start = offset.min(len);
    let next = start + entries.len();
    Outline {
        kind: json_type(value),
        len,
        bytes: json_size(value),
        entries,
        offset: start,
        next_offset: (next < len).then_some(next),
    }
}

/// A UTF-8 window of a text file.
#[derive(Debug, Serialize, PartialEq, Eq)]
pub struct TextSlice {
    /// Total file size in bytes.
    pub size: u64,
    /// Byte offset the text starts at (moved forward to a character
    /// boundary when the requested offset fell inside a character).
    pub offset: u64,
    pub text: String,
    /// Pass back as `offset` to read on; `None` at the end of the file.
    pub next_offset: Option<u64>,
}

/// Bytes a UTF-8 character can span beyond its first byte.
const MAX_CHAR_TAIL: usize = 3;

/// Reads up to `max_len` bytes of text starting at `offset`, trimmed to
/// character boundaries. `max_len` must exceed 3 so a character always fits
/// and paging advances. Blocking.
pub fn read_text(file: &Path, offset: u64, max_len: usize) -> GamedataResult<TextSlice> {
    debug_assert!(max_len > MAX_CHAR_TAIL);
    let not_text =
        || GamedataError::Miss("file is not UTF-8 text; download it from url instead".to_string());
    let mut handle = File::open(file)?;
    let size = handle.metadata()?.len();
    if offset > size {
        return Err(GamedataError::Miss(format!(
            "offset {offset} is past the end of the file ({size} bytes)"
        )));
    }
    handle.seek(SeekFrom::Start(offset))?;
    // Extra bytes: a partial character skipped at the start, plus one byte
    // past the cut to tell whether the cut splits a character.
    let mut window = Vec::with_capacity(max_len + MAX_CHAR_TAIL + 1);
    handle
        .take((max_len + MAX_CHAR_TAIL + 1) as u64)
        .read_to_end(&mut window)?;
    let (start, end) = utf8_window(&window, max_len).ok_or_else(not_text)?;
    let text = std::str::from_utf8(&window[start..end])
        .map_err(|_| not_text())?
        .to_string();
    let next = offset + end as u64;
    Ok(TextSlice {
        size,
        offset: offset + start as u64,
        text,
        next_offset: (next < size).then_some(next),
    })
}

const fn is_continuation(byte: u8) -> bool {
    byte & 0b1100_0000 == 0b1000_0000
}

/// Picks `[start, end)` inside `window` so the slice starts and ends on
/// character boundaries and spans at most `max_len` bytes. A byte after
/// `end` is a continuation byte exactly when the cut splits a character.
/// `None` when the bytes cannot be UTF-8.
fn utf8_window(window: &[u8], max_len: usize) -> Option<(usize, usize)> {
    let start = window
        .iter()
        .take_while(|&&byte| is_continuation(byte))
        .count();
    if start > MAX_CHAR_TAIL {
        return None;
    }
    let mut end = (start + max_len).min(window.len());
    let mut backed = 0;
    while end < window.len() && is_continuation(window[end]) {
        if backed == MAX_CHAR_TAIL || end == start + 1 {
            return None;
        }
        end -= 1;
        backed += 1;
    }
    Some((start, end))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

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

    #[test]
    fn gamedata_file_rejects_escapes_and_explains_misses() {
        let fixture = Fixture::new();
        fixture.write("gamedata/v1/excel/item_table.json", b"{}");
        fixture.write("secret", b"x");
        assert!(
            fixture
                .client
                .gamedata_file("v1", "excel/item_table.json")
                .is_ok()
        );
        for (version, path) in [
            ("v1", "../../secret"),
            ("v1", "/etc/passwd"),
            ("v1", ""),
            ("v1", "."),
            ("..", "secret"),
            ("latest", "excel/item_table.json"),
        ] {
            assert!(
                matches!(
                    fixture.client.gamedata_file(version, path),
                    Err(GamedataError::Failed(AppError::InvalidInput(_)))
                ),
                "{version:?} {path:?}"
            );
        }
        for (version, path) in [
            ("v9", "excel/item_table.json"),
            ("v1", "excel/missing.json"),
            ("v1", "excel"),
            ("v1", "excel/item_table.json/child"),
        ] {
            assert!(
                matches!(
                    fixture.client.gamedata_file(version, path),
                    Err(GamedataError::Miss(_))
                ),
                "{version:?} {path:?}"
            );
        }
        std::os::unix::fs::symlink(
            fixture.root.join("assets/secret"),
            fixture.client.asset_base_path.join("gamedata/v1/inside"),
        )
        .unwrap();
        assert!(fixture.client.gamedata_file("v1", "inside").is_ok());
        std::os::unix::fs::symlink(
            fixture.root.join("outside"),
            fixture.client.asset_base_path.join("gamedata/v1/outside"),
        )
        .unwrap();
        std::fs::write(fixture.root.join("outside"), b"x").unwrap();
        assert!(matches!(
            fixture.client.gamedata_file("v1", "outside"),
            Err(GamedataError::Miss(_))
        ));
    }

    #[test]
    fn read_json_skips_bom_and_reports_invalid_json() {
        let fixture = Fixture::new();
        let file = fixture.write("table.json", b"\xEF\xBB\xBF{\"a\":1}");
        assert_eq!(read_json(&file).unwrap(), json!({"a": 1}));
        let file = fixture.write("broken.json", b"{\"a\":");
        assert!(matches!(read_json(&file), Err(GamedataError::Miss(_))));
    }

    #[test]
    fn outline_pages_children_with_escaped_pointers() {
        let value = json!({"a/b": [1, 2], "c~d": {"x": "y"}, "e": "text"});
        let page = outline(&value, "/root", 0, usize::MAX);
        assert_eq!(page.kind, "object");
        assert_eq!(page.len, 3);
        assert_eq!(page.next_offset, None);
        assert_eq!(page.entries[0].pointer, "/root/a~1b");
        assert_eq!(page.entries[0].len, Some(2));
        assert_eq!(page.entries[1].pointer, "/root/c~0d");
        assert_eq!(page.entries[2].kind, "string");
        assert_eq!(page.entries[2].bytes, 6);
        for entry in &page.entries {
            assert_eq!(
                value.pointer(entry.pointer.strip_prefix("/root").unwrap()),
                value.get(&entry.key)
            );
        }

        let first = outline(&value, "", 0, 1);
        assert_eq!(first.entries.len(), 1, "at least one entry per page");
        assert_eq!(first.next_offset, Some(1));
        let rest = outline(&value, "", 1, usize::MAX);
        assert_eq!(rest.offset, 1);
        assert_eq!(rest.entries.len(), 2);
        let past = outline(&value, "", 10, usize::MAX);
        assert!(past.entries.is_empty());
        assert_eq!(past.next_offset, None);

        let array = outline(&json!([{"k": 1}, null]), "", 0, usize::MAX);
        assert_eq!(array.kind, "array");
        assert_eq!(array.entries[1].pointer, "/1");
        assert_eq!(array.entries[1].kind, "null");

        let scalar = outline(&json!(3), "", 0, usize::MAX);
        assert_eq!((scalar.kind, scalar.len), ("number", 0));
        assert!(scalar.entries.is_empty());
    }

    #[test]
    fn json_size_matches_compact_serialization() {
        let value = json!({"名": "阿米娅", "n": [1, 2.5, null]});
        assert_eq!(json_size(&value), serde_json::to_vec(&value).unwrap().len());
    }

    #[test]
    fn text_slices_respect_character_boundaries() {
        let fixture = Fixture::new();
        // "ab" + 3-byte "阿" + "cd" + 4-byte emoji
        let content = "ab阿cd😀".as_bytes();
        let file = fixture.write("story.txt", content);
        let whole = read_text(&file, 0, 1024).unwrap();
        assert_eq!(whole.text, "ab阿cd😀");
        assert_eq!(whole.next_offset, None);

        let mut offset = 0;
        let mut collected = String::new();
        loop {
            let slice = read_text(&file, offset, 4).unwrap();
            assert!(slice.text.len() <= 4);
            collected.push_str(&slice.text);
            match slice.next_offset {
                Some(next) => {
                    assert!(next > offset, "paging advances");
                    offset = next;
                }
                None => break,
            }
        }
        assert_eq!(collected, "ab阿cd😀");

        // An offset inside "阿" moves forward to the next character.
        let inside = read_text(&file, 3, 1024).unwrap();
        assert_eq!(inside.offset, 5);
        assert_eq!(inside.text, "cd😀");

        assert!(matches!(
            read_text(&file, 100, 16),
            Err(GamedataError::Miss(_))
        ));
        let binary = fixture.write("blob.bin", &[0x80, 0x80, 0x80, 0x80, 0x80]);
        assert!(matches!(
            read_text(&binary, 0, 16),
            Err(GamedataError::Miss(_))
        ));
        let invalid = fixture.write("latin1.txt", b"caf\xe9 au lait");
        assert!(matches!(
            read_text(&invalid, 0, 64),
            Err(GamedataError::Miss(_))
        ));
    }
}
