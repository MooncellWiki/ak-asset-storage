//! Resource extraction over parsed story lines.
//!
//! Ports arkwaifu's `story_pictures.go` / `story_characters.go` semantics:
//! first-appearance ordering per script and speaker-name attribution to the
//! currently focused character slot.

use std::collections::{HashMap, HashSet};
use std::sync::LazyLock;

use regex::Regex;

use super::parser::ParsedLine;

pub const TYPE_BACKGROUND: &str = "background";
pub const TYPE_IMAGE: &str = "image";
pub const TYPE_ITEM: &str = "item";
pub const TYPE_CHARACTER: &str = "character";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResourceUsage {
    pub resource_type: String,
    pub resource_id: String,
    pub display_names: Vec<String>,
    /// First-appearance order of the resource inside the script.
    pub sort_order: usize,
}

/// `^(.*?)(?:#(\d+))?(?:\$(\d+))?$` — `{base}#{face}${body}` with face/body
/// defaulting to 1.
static CHARACTER_ID_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^(.*?)(?:#(\d+))?(?:\$(\d+))?$").expect("character id regex must compile")
});

pub fn normalize_character_id(id: &str) -> String {
    if id.is_empty() {
        return String::new();
    }
    let caps = CHARACTER_ID_REGEX
        .captures(id)
        .expect("fully optional regex always matches non-empty input");
    let base = caps
        .get(1)
        .map_or(String::new(), |m| m.as_str().to_string());
    let face = caps.get(2).map_or("1", |m| m.as_str());
    let body = caps.get(3).map_or("1", |m| m.as_str());
    format!("{base}#{face}${body}")
}

/// Accumulates usages for a single script, keyed by `(type, id)` in
/// first-appearance order. Peak memory scales with one script's resources.
#[derive(Default)]
struct UsageAccumulator {
    order: Vec<(String, String)>,
    index: HashMap<(String, String), usize>,
    display_names: Vec<Vec<String>>,
}

impl UsageAccumulator {
    fn add_resource(&mut self, resource_type: &str, resource_id: &str) {
        if resource_id.is_empty() {
            return;
        }
        let key = (resource_type.to_string(), resource_id.to_string());
        if self.index.contains_key(&key) {
            return;
        }
        self.index.insert(key.clone(), self.order.len());
        self.order.push(key);
        self.display_names.push(Vec::new());
    }

    fn add_display_name(&mut self, character_id: &str, name: &str) {
        let key = (TYPE_CHARACTER.to_string(), character_id.to_string());
        if let Some(&position) = self.index.get(&key)
            && !self.display_names[position]
                .iter()
                .any(|existing| existing == name)
        {
            self.display_names[position].push(name.to_string());
        }
    }

    fn finish(self) -> Vec<ResourceUsage> {
        self.order
            .into_iter()
            .zip(self.display_names)
            .enumerate()
            .map(
                |(position, ((resource_type, resource_id), display_names))| ResourceUsage {
                    resource_type,
                    resource_id,
                    display_names,
                    sort_order: position,
                },
            )
            .collect()
    }
}

/// Ports arkwaifu's `characterStage`: the spotlight slot determines which
/// on-stage character a `[name="..."]` dialogue line is attributed to.
#[derive(Default)]
struct CharacterStage {
    spotlight: String,
    slots: HashMap<String, String>,
    seen: HashSet<String>,
}

impl CharacterStage {
    fn take(&mut self, slot: &str, id: &str, usages: &mut UsageAccumulator) {
        if id.is_empty() {
            self.slots.remove(slot);
            return;
        }
        self.slots.insert(slot.to_string(), id.to_string());
        if self.seen.insert(id.to_string()) {
            usages.add_resource(TYPE_CHARACTER, id);
        }
    }

    fn focus(&mut self, slot: &str, _default_slot: &str) {
        if !slot.is_empty() {
            self.spotlight = slot.to_string();
        } else if self.slots.len() == 1 {
            self.spotlight = self.slots.keys().next().expect("len checked").clone();
        } else {
            self.spotlight.clear();
        }
    }

    fn exit(&mut self) {
        self.spotlight.clear();
        self.slots.clear();
    }

    fn protagonist(&self) -> Option<&str> {
        self.slots.get(self.spotlight.as_str()).map(String::as_str)
    }

    fn record_name(&self, name: &str, usages: &mut UsageAccumulator) {
        if name.trim().is_empty() {
            return;
        }
        if let Some(id) = self.protagonist() {
            usages.add_display_name(id, name);
        }
    }
}

pub fn extract_usages(lines: &[ParsedLine]) -> Vec<ResourceUsage> {
    let mut usages = UsageAccumulator::default();
    let mut stage = CharacterStage::default();

    for line in lines {
        match line {
            ParsedLine::Dialogue { speaker } => stage.record_name(speaker, &mut usages),
            ParsedLine::Command { name, args } => match name.as_str() {
                "character" => {
                    let id1 = args
                        .get("name")
                        .map_or(String::new(), |value| normalize_character_id(value));
                    let id2 = args
                        .get("name2")
                        .map_or(String::new(), |value| normalize_character_id(value));
                    stage.take("1", &id1, &mut usages);
                    stage.take("2", &id2, &mut usages);
                    stage.focus(args.get("focus").map_or("", String::as_str), "1");
                }
                "charslot" => {
                    let id = args
                        .get("name")
                        .map_or(String::new(), |value| normalize_character_id(value));
                    if id.is_empty() {
                        stage.exit();
                    } else {
                        let slot = args.get("slot").map_or("", String::as_str);
                        stage.take(slot, &id, &mut usages);
                        let focus = args.get("focus").map_or("", String::as_str);
                        stage.focus(focus, slot);
                    }
                }
                "dialog" => stage.exit(),
                "image" => {
                    usages.add_resource(TYPE_IMAGE, args.get("image").map_or("", String::as_str));
                }
                "background" => {
                    usages.add_resource(
                        TYPE_BACKGROUND,
                        args.get("image").map_or("", String::as_str),
                    );
                }
                "showitem" => {
                    usages.add_resource(TYPE_ITEM, args.get("image").map_or("", String::as_str));
                }
                // Background tile groups; `imagegroup` holds `/`-joined ids.
                "largebg" | "gridbg" | "verticalbg" => {
                    if let Some(group) = args.get("imagegroup") {
                        for id in group.split('/') {
                            usages.add_resource(TYPE_BACKGROUND, id);
                        }
                    }
                }
                _ => {}
            },
            ParsedLine::Narration => {}
        }
    }

    usages.finish()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::service::story_usage::parser::parse_script;

    fn usages(source: &str) -> Vec<ResourceUsage> {
        extract_usages(&parse_script(source))
    }

    fn find<'a>(all: &'a [ResourceUsage], ty: &str, id: &str) -> &'a ResourceUsage {
        all.iter()
            .find(|usage| usage.resource_type == ty && usage.resource_id == id)
            .unwrap_or_else(|| panic!("missing {ty}/{id} in {all:?}"))
    }

    #[test]
    fn normalizes_character_ids_with_default_face_and_body() {
        assert_eq!(normalize_character_id(""), "");
        assert_eq!(normalize_character_id("avg_npc_009"), "avg_npc_009#1$1");
        assert_eq!(
            normalize_character_id("char_220_grani#5"),
            "char_220_grani#5$1"
        );
        assert_eq!(
            normalize_character_id("avg_npc_416_1#1$1"),
            "avg_npc_416_1#1$1"
        );
        assert_eq!(
            normalize_character_id("avg_1014_nearl2_1#2$2"),
            "avg_1014_nearl2_1#2$2"
        );
    }

    #[test]
    fn extracts_picture_commands() {
        let all = usages(concat!(
            r#"[Image(image="ac1_0",x=0, y=0, xScale=1)]"#,
            "\n",
            r#"[Image(fadetime=0)]"#,
            "\n",
            r#"[Image]"#,
            "\n",
            r#"[Background(image="bg_med", fadetime=2,block=true)]"#,
            "\n",
            r#"[Background]"#,
            "\n",
            r#"[ShowItem(image="item_caster", fadetime=0.1)]"#,
            "\n",
            r#"[showitem(image="item_act70_1")]"#,
            "\n",
        ));

        assert!(
            all.iter()
                .any(|u| u.resource_type == "image" && u.resource_id == "ac1_0")
        );
        // Image without an `image` parameter only hides the current one.
        assert_eq!(all.iter().filter(|u| u.resource_type == "image").count(), 1);
        assert!(
            all.iter()
                .any(|u| u.resource_type == "background" && u.resource_id == "bg_med")
        );
        assert_eq!(
            all.iter()
                .filter(|u| u.resource_type == "background")
                .count(),
            1
        );
        assert!(
            all.iter()
                .any(|u| u.resource_type == "item" && u.resource_id == "item_caster")
        );
        assert!(
            all.iter()
                .any(|u| u.resource_type == "item" && u.resource_id == "item_act70_1")
        );
    }

    #[test]
    fn splits_imagegroup_into_backgrounds() {
        let all = usages(r#"[verticalbg(imagegroup="50_g22_1/50_g22_2", solidwidth=1280)]"#);
        assert_eq!(all.len(), 2);
        assert_eq!(all[0].resource_id, "50_g22_1");
        assert_eq!(all[1].resource_id, "50_g22_2");
        assert!(all.iter().all(|u| u.resource_type == "background"));

        let all = usages(r#"[gridbg(imagegroup="a/b/c")]"#);
        assert_eq!(all.len(), 3);
    }

    #[test]
    fn attributes_display_names_to_focused_character() {
        let all = usages(concat!(
            r#"[Character(name="avg_npc_009",name2="avg_npc_003",focus=1)]"#,
            "\n",
            r#"[name="赏金猎人"]   text"#,
            "\n",
            r#"[Character(name="avg_npc_009",name2="avg_npc_003",focus=2)]"#,
            "\n",
            r#"[name="可萝尔"]   text"#,
            "\n",
        ));

        let hunter = find(&all, "character", "avg_npc_009#1$1");
        let korul = find(&all, "character", "avg_npc_003#1$1");
        assert_eq!(hunter.display_names, vec!["赏金猎人"]);
        assert_eq!(korul.display_names, vec!["可萝尔"]);
    }

    #[test]
    fn names_without_focus_and_two_characters_are_dropped() {
        let all = usages(concat!(
            r#"[Character(name="avg_npc_009",name2="avg_npc_003")]"#,
            "\n",
            r#"[name="赏金猎人"]   text"#,
            "\n",
        ));
        let hunter = find(&all, "character", "avg_npc_009#1$1");
        assert_eq!(hunter.display_names, Vec::<String>::new());
    }

    #[test]
    fn single_character_on_stage_gets_names_without_focus() {
        let all = usages(concat!(
            r#"[Character(name="char_220_grani#5",fadetime=1.5,block=true)]"#,
            "\n",
            r#"[name="？？？"]   所以，得把你们全部解决掉才行？"#,
        ));
        let granhi = find(&all, "character", "char_220_grani#5$1");
        assert_eq!(granhi.display_names, vec!["？？？"]);
    }

    #[test]
    fn dialog_clears_stage_and_names_are_dropped() {
        let all = usages(concat!(
            r#"[Character(name="avg_npc_008")]"#,
            "\n",
            r#"[Dialog]"#,
            "\n",
            r#"[name="赏金猎人"]   text"#,
            "\n",
        ));
        let npc = find(&all, "character", "avg_npc_008#1$1");
        assert_eq!(npc.display_names, Vec::<String>::new());
    }

    #[test]
    fn chslot_slots_and_focus() {
        let all = usages(concat!(
            r#"[charslot(slot="l",name="avg_npc_416_1#1$1",duration=0.5)]"#,
            "\n",
            r#"[charslot(slot="r",name="avg_npc_242",duration=0.5)]"#,
            "\n",
            r#"[charslot(slot="r",name="avg_npc_242",focus="r")]"#,
            "\n",
            r#"[name="流浪者"]   text"#,
            "\n",
            r#"[charslot(slot="r")]"#,
            "\n",
        ));
        let focused = find(&all, "character", "avg_npc_242#1$1");
        assert_eq!(focused.display_names, vec!["流浪者"]);
        let left = find(&all, "character", "avg_npc_416_1#1$1");
        assert_eq!(left.display_names, Vec::<String>::new());
    }

    #[test]
    fn dedupes_display_names_keeping_first_order() {
        let all = usages(concat!(
            r#"[Character(name="avg_npc_009")]"#,
            "\n",
            r#"[name="赏金猎人"]   one"#,
            "\n",
            r#"[name="粗暴的赏金猎人"]   two"#,
            "\n",
            r#"[name="赏金猎人"]   three"#,
            "\n",
        ));
        let hunter = find(&all, "character", "avg_npc_009#1$1");
        assert_eq!(hunter.display_names, vec!["赏金猎人", "粗暴的赏金猎人"]);
    }

    #[test]
    fn skips_blank_display_names() {
        let all = usages(concat!(
            r#"[Character(name="avg_npc_009")]"#,
            "\n",
            r#"[name=""]   text"#,
            "\n",
            r#"[name="   "]   text"#,
            "\n",
            r#"[name="赏金猎人"]   text"#,
            "\n",
        ));
        let hunter = find(&all, "character", "avg_npc_009#1$1");
        assert_eq!(hunter.display_names, vec!["赏金猎人"]);
    }

    #[test]
    fn sort_order_follows_first_appearance_across_types() {
        let all = usages(concat!(
            r#"[Background(image="bg_med")]"#,
            "\n",
            r#"[Character(name="avg_npc_009")]"#,
            "\n",
            r#"[Image(image="ac1_0")]"#,
            "\n",
            r#"[Background(image="bg_tower")]"#,
            "\n",
            r#"[Character(name="avg_npc_009")]"#,
            "\n",
        ));

        assert_eq!(find(&all, "background", "bg_med").sort_order, 0);
        assert_eq!(find(&all, "character", "avg_npc_009#1$1").sort_order, 1);
        assert_eq!(find(&all, "image", "ac1_0").sort_order, 2);
        assert_eq!(find(&all, "background", "bg_tower").sort_order, 3);
    }

    #[test]
    fn character_with_empty_name_leaves_other_slot_as_sole_spotlight() {
        let all = usages(concat!(
            r#"[Character(name="",name2="avg_npc_003")]"#,
            "\n",
            r#"[name="可萝尔"]   text"#,
            "\n",
        ));
        // take("1", "") removes slot 1; slot 2 is the only character left, so
        // the no-focus rule spotlights it and the name is attributed.
        let korul = find(&all, "character", "avg_npc_003#1$1");
        assert_eq!(korul.display_names, vec!["可萝尔"]);
    }
}
