//! Character link map (torappu `avg/character.json`): the source of truth
//! for whether a character reference is a face overlay composited onto a
//! shared body texture or a standalone full image.
//!
//! Overlay entries (`face`, names like `N$M`) share one `base$body` png, so
//! the resource listing collapses them to body form. Full-image entries
//! (`image`, names like `base_2`, empty `groups`) are one png per `#face`
//! reference with no shared body — e.g. `char_002_amiya_1#1`..`#11` are
//! eleven standalone pngs — so the listing keeps their face-level ids.
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

use super::extract::TYPE_CHARACTER;

#[derive(Debug, Deserialize)]
pub struct CharacterLinks(HashMap<String, LinkNode>);

#[derive(Debug, Deserialize)]
struct LinkNode {
    array: Vec<CharacterEntry>,
}

#[derive(Debug, Deserialize)]
struct CharacterEntry {
    name: String,
    image: Option<String>,
}

/// `base#face$body` reference split into its parts.
struct RefParts<'a> {
    base: &'a str,
    face: i32,
    body: i32,
}

fn split_ref(id: &str) -> Option<RefParts<'_>> {
    let dollar = id.rfind('$')?;
    let hash = id[..dollar].rfind('#')?;
    Some(RefParts {
        base: &id[..hash],
        face: id[hash + 1..dollar].trim().parse().unwrap_or(1),
        body: id[dollar + 1..].trim().parse().unwrap_or(1),
    })
}

/// Ports the runtime's group-then-index resolution: entries whose names end
/// in `$body` form the body's face list (1-based `face` index, falling back
/// to the first); without such names the whole array is indexed, which is
/// how full-image characters resolve.
fn resolve_entry(node: &LinkNode, face: i32, body: i32) -> Option<&CharacterEntry> {
    let suffix = format!("${body}");
    let grouped: Vec<&CharacterEntry> = node
        .array
        .iter()
        .filter(|entry| entry.name.ends_with(&suffix))
        .collect();
    let index = usize::try_from(face.max(1) - 1).unwrap_or(0);
    if grouped.is_empty() {
        node.array.get(index).or_else(|| node.array.first())
    } else {
        grouped
            .get(index)
            .copied()
            .or_else(|| grouped.first().copied())
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
        Ok(Self(
            raw.into_iter()
                .map(|(key, node)| (key.to_lowercase(), node))
                .collect(),
        ))
    }

    /// Listing-granularity id: overlay characters collapse to body form
    /// (`base$body`), full-image characters keep the face-level id, and
    /// non-characters pass through verbatim. Unknown bases keep the body
    /// grouping so broken references stay navigable. Character ids are
    /// emitted lowercase-canonical: the base folds for lookup, matching
    /// native's effective (lowercased) asset paths.
    #[must_use]
    pub fn listing_id(&self, resource_type: &str, resource_id: &str) -> String {
        if resource_type != TYPE_CHARACTER {
            return resource_id.to_owned();
        }
        let Some(parts) = split_ref(resource_id) else {
            return resource_id.to_owned();
        };
        let base = parts.base.to_lowercase();
        let collapsed = format!("{base}${}", parts.body);
        match self.0.get(&base) {
            None => collapsed,
            Some(node) => match resolve_entry(node, parts.face, parts.body) {
                Some(entry) if entry.image.is_some() => {
                    format!("{base}#{}${}", parts.face, parts.body)
                }
                _ => collapsed,
            },
        }
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
                {"name": "1$1", "alias": "", "group": 0, "face": "avg_overlay_1/1$1"},
                {"name": "2$1", "alias": "", "group": 0, "face": "avg_overlay_1/2$1"}
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

    #[test]
    fn overlay_characters_collapse_to_body_form() {
        let links = links();
        assert_eq!(
            links.listing_id("character", "avg_overlay_1#2$1"),
            "avg_overlay_1$1"
        );
        // Face index beyond the list falls back to the first entry, which is
        // still a face overlay.
        assert_eq!(
            links.listing_id("character", "avg_overlay_1#9$1"),
            "avg_overlay_1$1"
        );
    }

    #[test]
    fn full_image_characters_keep_face_level_ids() {
        let links = links();
        assert_eq!(
            links.listing_id("character", "char_full_1#1$1"),
            "char_full_1#1$1"
        );
        assert_eq!(
            links.listing_id("character", "char_full_1#2$1"),
            "char_full_1#2$1"
        );
        // Out-of-range face resolves to the first full image: still no
        // collapse.
        assert_eq!(
            links.listing_id("character", "char_full_1#7$1"),
            "char_full_1#7$1"
        );
    }

    #[test]
    fn unknown_bases_and_other_types_keep_previous_rules() {
        let links = links();
        // Unknown base: body grouping keeps broken refs navigable.
        assert_eq!(
            links.listing_id("character", "avg_unknown_1#4$1"),
            "avg_unknown_1$1"
        );
        // Verbatim broken base survives the collapse.
        assert_eq!(
            links.listing_id("character", "avg_x#3 4#1$1"),
            "avg_x#3 4$1"
        );
        // Non-characters and suffix-less ids pass through.
        assert_eq!(links.listing_id("background", "bg_med"), "bg_med");
        assert_eq!(links.listing_id("character", "nobody"), "nobody");
    }

    #[test]
    fn folds_case_for_lookup_and_emits_lowercase_ids() {
        let links = links();
        // Script spelling differs from the json key only by case, like the
        // corpus's `avg_1012_skadiSP_1` refs against the `avg_1012_skadisp_1`
        // key: native lowercases the asset path before the bundle lookup, so
        // the ref resolves and the listing id is lowercase-canonical.
        assert_eq!(
            links.listing_id("character", "AVG_MIX_1#2$1"),
            "avg_mix_1$1"
        );
        // Full-image characters keep the face-level id, folded.
        assert_eq!(
            links.listing_id("character", "CHAR_FULL_1#2$1"),
            "char_full_1#2$1"
        );
        // A mixed-case key folds too, so both spellings of one character
        // group under a single listing id.
        assert_eq!(
            links.listing_id("character", "avg_Mix_1#1$1"),
            "avg_mix_1$1"
        );
    }
}
