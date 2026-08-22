//! Character link map (torappu `avg/character.json`): the source of truth
//! for whether a character reference is a face overlay composited onto a
//! shared body texture or a standalone full image.
//!
//! Overlay entries (`face`, names like `N$M`) share one `base$body` png, so
//! the resource listing collapses them to body form. Full-image entries
//! (`image`, names like `base_2`, empty `groups`) are one png per expression,
//! so the listing keeps the resolved `character.json` entry name.
//!
//! Lookups are case-insensitive: `Torappu.AVG` preserves the case a script
//! spells, but `ABResourceManager._PreprocessAssetPath` lowercases the whole
//! asset path before the bundle lookup and every `avg/characters/*.ab` entry
//! is lowercase, so refs like `avg_1012_skadiSP_1` resolve against the
//! `avg_1012_skadisp_1` key in-game. Map keys therefore fold to lowercase at
//! parse time; `name`/`image`/`face` values stay verbatim.

use std::{collections::HashMap, fs, path::Path};

use anyhow::{Context, bail};
use serde::Deserialize;

#[derive(Debug)]
pub struct CharacterLinks {
    nodes: HashMap<String, LinkNode>,
    /// `StoryPlayer`'s web-only `base-expression` references, precomputed to
    /// avoid scanning every character for every story reference.
    explicit_refs: HashMap<String, (String, String)>,
}

#[derive(Debug, Deserialize)]
struct LinkNode {
    array: Vec<CharacterEntry>,
}

#[derive(Debug, Deserialize)]
struct CharacterEntry {
    name: String,
    #[serde(default)]
    alias: Option<String>,
    group: i32,
    image: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedCharacterId {
    /// Face-level identity consumed by the Explorer (`base#entry.name`).
    pub resource_id: String,
    /// Overlay entries collapse to `base$body`; full images keep `resource_id`.
    pub listing_id: String,
}

struct NativeCharacterRef<'a> {
    alias: Option<&'a str>,
    base: &'a str,
    /// Native `$N` suffix, kept 1-based for entry-name matching.
    group: Option<i32>,
    /// Native `#N` suffix converted to the selected zero-based index.
    index: usize,
}

const fn is_dotnet_whitespace(ch: char) -> bool {
    matches!(ch, '\u{20}' | '\u{09}'..='\u{0d}')
}

/// `System.Int32.TryParse(NumberStyles.Integer)` as used by `StoryPlayer`'s
/// native-character-ref port.
fn try_parse_i32(raw: &str) -> Option<i32> {
    raw.trim_matches(is_dotnet_whitespace).parse().ok()
}

fn parse_native_ref(raw: &str) -> NativeCharacterRef<'_> {
    let mut value = raw;
    let mut group = None;
    if let Some(dollar) = value.rfind('$')
        && let Some(parsed) = try_parse_i32(&value[dollar + 1..])
    {
        group = Some(parsed);
        value = &value[..dollar];
    }

    if let Some(at) = value.rfind('@') {
        return NativeCharacterRef {
            alias: Some(&value[at + 1..]),
            base: &value[..at],
            group,
            index: 0,
        };
    }

    let mut index = 0;
    if let Some(hash) = value.rfind('#')
        && let Some(parsed) = try_parse_i32(&value[hash + 1..])
    {
        index = usize::try_from(parsed.saturating_sub(1).max(0)).unwrap_or(0);
        value = &value[..hash];
    }
    NativeCharacterRef {
        alias: None,
        base: value,
        group,
        index,
    }
}

fn select_entry<'a>(
    node: &'a LinkNode,
    parsed: &NativeCharacterRef<'_>,
) -> Option<&'a CharacterEntry> {
    if let Some(alias) = parsed.alias {
        return node
            .array
            .iter()
            .find(|entry| {
                entry
                    .alias
                    .as_deref()
                    .is_some_and(|candidate| candidate.to_lowercase() == alias)
            })
            .or_else(|| node.array.first());
    }

    if let Some(group) = parsed.group {
        let suffix = format!("${group}");
        let mut grouped = node
            .array
            .iter()
            .filter(|entry| entry.name.ends_with(&suffix));
        let first = grouped.next()?;
        return if parsed.index == 0 {
            Some(first)
        } else {
            grouped.nth(parsed.index - 1).or(Some(first))
        };
    }

    node.array.get(parsed.index).or_else(|| node.array.first())
}

fn resolved_id(base: &str, entry: &CharacterEntry) -> ResolvedCharacterId {
    let resource_id = format!("{base}#{}", entry.name);
    let listing_id = if entry.image.is_some() || entry.group < 0 {
        resource_id.clone()
    } else {
        format!("{base}${}", entry.group + 1)
    };
    ResolvedCharacterId {
        resource_id,
        listing_id,
    }
}

impl CharacterLinks {
    /// Loads `raw/avg/character.json` from the torappu asset root.
    pub fn load(asset_root: &Path) -> anyhow::Result<Self> {
        let path = asset_root.join("raw").join("avg").join("character.json");
        if !path.is_file() {
            bail!(
                "character link map is missing: {} (sync torappu assets first)",
                path.display()
            );
        }
        let content = fs::read_to_string(&path)
            .with_context(|| format!("failed to read {}", path.display()))?;
        Self::parse(&content).with_context(|| format!("failed to parse {}", path.display()))
    }

    pub fn parse(json: &str) -> anyhow::Result<Self> {
        let raw: HashMap<String, LinkNode> = serde_json::from_str(json)?;
        let nodes: HashMap<String, LinkNode> = raw
            .into_iter()
            .map(|(key, node)| (key.to_lowercase(), node))
            .collect();
        let explicit_refs = nodes
            .iter()
            .flat_map(|(base, node)| {
                node.array.iter().map(move |entry| {
                    (
                        format!("{base}-{}", entry.name.to_lowercase()),
                        (base.clone(), entry.name.clone()),
                    )
                })
            })
            .collect();
        Ok(Self {
            nodes,
            explicit_refs,
        })
    }

    /// Resolves a raw story ref with the same direct / explicit-expression /
    /// native suffix order as `StoryPlayer`. Unknown refs are omitted because
    /// the player cannot display them either.
    #[must_use]
    pub fn resolve(&self, raw_ref: &str) -> Option<ResolvedCharacterId> {
        if raw_ref.is_empty() {
            return None;
        }
        let normalized = raw_ref.to_lowercase();

        if let Some(node) = self.nodes.get(&normalized) {
            return node
                .array
                .first()
                .map(|entry| resolved_id(&normalized, entry));
        }

        if let Some((base, expression)) = self.explicit_refs.get(&normalized) {
            let entry = self
                .nodes
                .get(base)?
                .array
                .iter()
                .find(|entry| entry.name == expression.as_str())?;
            return Some(resolved_id(base, entry));
        }

        let parsed = parse_native_ref(&normalized);
        let node = self.nodes.get(parsed.base)?;
        let entry = select_entry(node, &parsed)?;
        Some(resolved_id(parsed.base, entry))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const LINKS_JSON: &str = r#"{
        "avg_overlay_1": {
            "pos": {"x": 0, "y": 190}, "size": {"x": 970, "y": 970},
            "groups": [{"mode": "face_overlay", "base": "avg_overlay_1/avg_overlay_1$1",
                        "faceRect": {"x": 459, "y": 159, "w": 130, "h": 110}}],
            "array": [
                {"name": "10$1", "alias": "normal", "group": 0, "face": "avg_overlay_1/10$1"},
                {"name": "20$1", "alias": "smile", "group": 0, "face": "avg_overlay_1/20$1"}
            ]
        },
        "char_full_1": {
            "pos": {"x": 0, "y": 190}, "size": {"x": 970, "y": 970}, "groups": [],
            "array": [
                {"name": "char_full_1", "alias": "normal", "group": -1,
                 "image": "char_full_1/char_full_1"},
                {"name": "char_full_1_2", "alias": "smile", "group": -1,
                 "image": "char_full_1/char_full_1_2"}
            ]
        },
        "avg_Mix_1": {
            "pos": {"x": 0, "y": 190}, "size": {"x": 970, "y": 970},
            "groups": [{"mode": "face_overlay", "base": "avg_Mix_1/avg_Mix_1$1",
                        "faceRect": {"x": 459, "y": 159, "w": 130, "h": 110}}],
            "array": [
                {"name": "1$1", "alias": "", "group": 0, "face": "avg_Mix_1/1$1"},
                {"name": "2$1", "alias": "", "group": 0, "face": "avg_Mix_1/2$1"}
            ]
        }
    }"#;

    fn links() -> CharacterLinks {
        CharacterLinks::parse(LINKS_JSON).expect("fixture json")
    }

    fn resolved(resource_id: &str, listing_id: &str) -> Option<ResolvedCharacterId> {
        Some(ResolvedCharacterId {
            resource_id: resource_id.to_string(),
            listing_id: listing_id.to_string(),
        })
    }

    #[test]
    fn resolves_numeric_indices_to_actual_overlay_entry_names() {
        let links = links();
        assert_eq!(
            links.resolve("avg_overlay_1#2$1"),
            resolved("avg_overlay_1#20$1", "avg_overlay_1$1")
        );
        assert_eq!(
            links.resolve("avg_overlay_1#9$1"),
            resolved("avg_overlay_1#10$1", "avg_overlay_1$1")
        );
        assert_eq!(
            links.resolve("avg_overlay_1"),
            resolved("avg_overlay_1#10$1", "avg_overlay_1$1")
        );
    }

    #[test]
    fn full_image_ids_use_the_resolved_entry_name() {
        let links = links();
        assert_eq!(
            links.resolve("char_full_1#1"),
            resolved("char_full_1#char_full_1", "char_full_1#char_full_1")
        );
        assert_eq!(
            links.resolve("char_full_1#2"),
            resolved("char_full_1#char_full_1_2", "char_full_1#char_full_1_2")
        );
        assert_eq!(
            links.resolve("char_full_1#7"),
            resolved("char_full_1#char_full_1", "char_full_1#char_full_1")
        );
    }

    #[test]
    fn supports_storyplayer_explicit_expression_and_alias_forms() {
        let links = links();
        assert_eq!(
            links.resolve("char_full_1-char_full_1_2"),
            resolved("char_full_1#char_full_1_2", "char_full_1#char_full_1_2")
        );
        assert_eq!(
            links.resolve("char_full_1@SMILE"),
            resolved("char_full_1#char_full_1_2", "char_full_1#char_full_1_2")
        );
    }

    #[test]
    fn matches_case_insensitively_and_preserves_entry_name_case() {
        let links = links();
        assert_eq!(
            links.resolve("AVG_MIX_1#2$1"),
            resolved("avg_mix_1#2$1", "avg_mix_1$1")
        );
        assert_eq!(
            links.resolve("CHAR_FULL_1#2"),
            resolved("char_full_1#char_full_1_2", "char_full_1#char_full_1_2")
        );
    }

    #[test]
    fn matches_storyplayer_int32_suffix_whitespace_and_rejects_unknown_refs() {
        let links = links();
        assert_eq!(
            links.resolve("avg_overlay_1#2 $1 "),
            resolved("avg_overlay_1#20$1", "avg_overlay_1$1")
        );
        assert_eq!(links.resolve("avg_overlay_1#2 3$1"), None);
        assert_eq!(links.resolve("avg_unknown_1#1$1"), None);
        assert_eq!(links.resolve(""), None);
    }
}
