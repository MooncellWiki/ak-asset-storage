//! Resource extraction over parsed story lines.
//!
//! Ports arkwaifu's `story_pictures.go` / `story_characters.go` semantics:
//! first-appearance ordering per script and speaker-name attribution to the
//! currently focused character slot.

use std::collections::HashMap;

use super::parser::ParsedLine;
use crate::database::row::StoryResourceType;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResourceUsage {
    pub resource_type: StoryResourceType,
    pub resource_id: String,
    pub display_names: Vec<String>,
    /// First-appearance order of the resource inside the script.
    pub sort_order: usize,
}

/// Argument value as `&str`, empty when the argument is absent.
fn str_arg<'a>(args: &'a HashMap<String, String>, key: &str) -> &'a str {
    args.get(key).map_or("", String::as_str)
}

/// Image-like resource identity follows `StoryPlayer`'s asset routing: trim the
/// script value and fold the effective bundle path to lowercase.
fn normalize_image_key(raw: &str) -> String {
    raw.trim().to_lowercase()
}

/// Accumulates usages for a single script, keyed by `(type, id)` in
/// first-appearance order. Peak memory scales with one script's resources.
#[derive(Default)]
struct UsageAccumulator {
    order: Vec<(StoryResourceType, String)>,
    index: HashMap<(StoryResourceType, String), usize>,
    display_names: Vec<Vec<String>>,
}

impl UsageAccumulator {
    fn add_resource(&mut self, resource_type: StoryResourceType, resource_id: &str) {
        let normalized;
        let resource_id = if resource_type == StoryResourceType::Character {
            resource_id
        } else {
            normalized = normalize_image_key(resource_id);
            &normalized
        };
        if resource_id.is_empty() {
            return;
        }
        let key = (resource_type, resource_id.to_string());
        if self.index.contains_key(&key) {
            return;
        }
        self.index.insert(key.clone(), self.order.len());
        self.order.push(key);
        self.display_names.push(Vec::new());
    }

    fn add_display_name(&mut self, character_id: &str, name: &str) {
        let key = (StoryResourceType::Character, character_id.to_string());
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
}

impl CharacterStage {
    fn take(&mut self, slot: &str, id: &str, usages: &mut UsageAccumulator) {
        if id.is_empty() {
            self.slots.remove(slot);
            return;
        }
        self.slots.insert(slot.to_string(), id.to_string());
        usages.add_resource(StoryResourceType::Character, id);
    }

    fn focus(&mut self, slot: &str) {
        // `focus=0` is native's "nobody is highlighted" and never names a slot
        // (`_ProcessSlot` compares it against ECharSlot). Treating it as a slot
        // id would park the spotlight on a key `slots` can never hold, so every
        // following speaker name would be dropped; fall through to the
        // single-character rule instead, the same as an omitted `focus`.
        let slot = if slot == "0" { "" } else { slot };
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

#[must_use]
pub fn extract_usages(lines: &[ParsedLine]) -> Vec<ResourceUsage> {
    let mut usages = UsageAccumulator::default();
    let mut stage = CharacterStage::default();

    for line in lines {
        match line {
            ParsedLine::Dialogue { speaker } => stage.record_name(speaker, &mut usages),
            ParsedLine::Command { name, args } => match name.as_str() {
                "character" => {
                    stage.take("1", str_arg(args, "name"), &mut usages);
                    stage.take("2", str_arg(args, "name2"), &mut usages);
                    stage.focus(str_arg(args, "focus"));
                }
                "charslot" => {
                    let id = str_arg(args, "name");
                    if id.is_empty() {
                        stage.exit();
                    } else {
                        let slot = str_arg(args, "slot");
                        stage.take(slot, id, &mut usages);
                        stage.focus(str_arg(args, "focus"));
                    }
                }
                "charactercutin" => {
                    if !str_arg(args, "widgetID").is_empty() {
                        // `_ExecuteCharacterCutin`'s port trims the ref before
                        // resolving it; character ids skip `normalize_image_key`,
                        // so do it here.
                        usages.add_resource(
                            StoryResourceType::Character,
                            str_arg(args, "name").trim(),
                        );
                    }
                }
                // `[Dialog]` ends the current character scene. But the sentinel
                // also collects the multi-param dialogue tags native's
                // `_ParseCommand` cannot parse as `[name="..."]` -- notably
                // `[name="X",avatarId=1]` and unquoted `[name=X]` -- and
                // `DialogPanel._ExecuteDialog` reads a speaker off those.
                "dialog" => {
                    let speaker = str_arg(args, "name");
                    if speaker.is_empty() {
                        stage.exit();
                    } else {
                        stage.record_name(speaker, &mut usages);
                    }
                }
                // `DialogPanel._ExecuteMultiline` carries the speaker in `name`
                // just like a dialogue tag; long conversations are authored
                // almost entirely with it.
                "multiline" => stage.record_name(str_arg(args, "name"), &mut usages),
                "image" | "cgitem" | "blocker" => {
                    usages.add_resource(StoryResourceType::Image, str_arg(args, "image"));
                }
                "background" => {
                    usages.add_resource(StoryResourceType::Background, str_arg(args, "image"));
                }
                "showitem" => {
                    usages.add_resource(StoryResourceType::Item, str_arg(args, "image"));
                }
                "avgdisplay" => {
                    let style = str_arg(args, "style").trim();
                    if style == "bg" || style == "5" {
                        usages.add_resource(StoryResourceType::Background, str_arg(args, "name"));
                    }
                }
                "interlude" => {
                    let interlude_type = str_arg(args, "type").trim();
                    match interlude_type {
                        "bg" | "2" => {
                            usages
                                .add_resource(StoryResourceType::Background, str_arg(args, "name"));
                        }
                        "uichar" | "1" => {
                            usages.add_resource(StoryResourceType::Image, str_arg(args, "name"));
                        }
                        "char" | "3" => {
                            usages
                                .add_resource(StoryResourceType::Character, str_arg(args, "name"));
                        }
                        _ => {}
                    }
                }
                // StoryPlayer prefers imagegroup over cggroup. The former is
                // a background family for *bg commands; the latter and every
                // largeimg group use regular story images.
                "largebg" | "gridbg" | "verticalbg" | "largeimg" => {
                    let image_group = str_arg(args, "imagegroup").trim();
                    let cg_group = str_arg(args, "cggroup").trim();
                    let (resource_type, group) = if image_group.is_empty() {
                        (StoryResourceType::Image, cg_group)
                    } else {
                        let resource_type = if name == "largeimg" {
                            StoryResourceType::Image
                        } else {
                            StoryResourceType::Background
                        };
                        (resource_type, image_group)
                    };
                    for id in group.split('/') {
                        usages.add_resource(resource_type, id);
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

    fn find<'a>(
        all: &'a [ResourceUsage],
        resource_type: StoryResourceType,
        id: &str,
    ) -> &'a ResourceUsage {
        all.iter()
            .find(|usage| usage.resource_type == resource_type && usage.resource_id == id)
            .unwrap_or_else(|| panic!("missing {resource_type:?}/{id} in {all:?}"))
    }

    #[test]
    fn normalizes_image_like_resource_ids() {
        let all = usages(concat!(
            r#"[Image(image=" AC1_0 ")]"#,
            "\n",
            r#"[Background(image="BG_23_G05")]"#,
            "\n",
            r#"[Background(image=" bg_23_g05 ")]"#,
        ));
        assert_eq!(all.len(), 2);
        assert_eq!(all[0].resource_id, "ac1_0");
        assert_eq!(all[1].resource_id, "bg_23_g05");
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
                .any(|u| u.resource_type == StoryResourceType::Image && u.resource_id == "ac1_0")
        );
        // Image without an `image` parameter only hides the current one.
        assert_eq!(
            all.iter()
                .filter(|u| u.resource_type == StoryResourceType::Image)
                .count(),
            1
        );
        assert!(
            all.iter()
                .any(|u| u.resource_type == StoryResourceType::Background
                    && u.resource_id == "bg_med")
        );
        assert_eq!(
            all.iter()
                .filter(|u| u.resource_type == StoryResourceType::Background)
                .count(),
            1
        );
        assert!(
            all.iter()
                .any(|u| u.resource_type == StoryResourceType::Item
                    && u.resource_id == "item_caster")
        );
        assert!(
            all.iter()
                .any(|u| u.resource_type == StoryResourceType::Item
                    && u.resource_id == "item_act70_1")
        );
    }

    #[test]
    fn splits_imagegroup_into_backgrounds() {
        let all = usages(r#"[verticalbg(imagegroup="50_g22_1/50_g22_2", solidwidth=1280)]"#);
        assert_eq!(all.len(), 2);
        assert_eq!(all[0].resource_id, "50_g22_1");
        assert_eq!(all[1].resource_id, "50_g22_2");
        assert!(
            all.iter()
                .all(|u| u.resource_type == StoryResourceType::Background)
        );

        let all = usages(r#"[gridbg(imagegroup="a/b/c")]"#);
        assert_eq!(all.len(), 3);
    }

    #[test]
    fn extracts_storyplayer_resource_commands() {
        let all = usages(concat!(
            r#"[CgItem(image="CGITEM_42_I11", style="cg")]"#,
            "\n",
            r#"[Blocker(image=" BLOCKER_MASK ")]"#,
            "\n",
            r#"[AvgDisplay(style=5, name="BG_AVG")]"#,
            "\n",
            r#"[Interlude(type=bg, name="BG_INTERLUDE")]"#,
            "\n",
            r#"[Interlude(type=1, name="UI_CHAR")]"#,
            "\n",
            r#"[Interlude(type=3, name="avg_interlude#2$1")]"#,
            "\n",
            r#"[CharacterCutin(widgetID="1", name="avg_cutin")]"#,
        ));

        find(&all, StoryResourceType::Image, "cgitem_42_i11");
        find(&all, StoryResourceType::Image, "blocker_mask");
        find(&all, StoryResourceType::Background, "bg_avg");
        find(&all, StoryResourceType::Background, "bg_interlude");
        find(&all, StoryResourceType::Image, "ui_char");
        find(&all, StoryResourceType::Character, "avg_interlude#2$1");
        find(&all, StoryResourceType::Character, "avg_cutin");
    }

    #[test]
    fn grouped_commands_follow_storyplayer_precedence_and_asset_family() {
        let all = usages(concat!(
            r#"[LargeBG(imagegroup="BG_A/BG_B", cggroup="CG_IGNORED")]"#,
            "\n",
            r#"[VerticalBG(cggroup="CG_A/CG_B")]"#,
            "\n",
            r#"[LargeImg(imagegroup="IMG_A/IMG_B")]"#,
        ));

        find(&all, StoryResourceType::Background, "bg_a");
        find(&all, StoryResourceType::Background, "bg_b");
        assert!(!all.iter().any(|usage| usage.resource_id == "cg_ignored"));
        find(&all, StoryResourceType::Image, "cg_a");
        find(&all, StoryResourceType::Image, "cg_b");
        find(&all, StoryResourceType::Image, "img_a");
        find(&all, StoryResourceType::Image, "img_b");
    }

    #[test]
    fn parameter_keys_remain_case_sensitive() {
        let all = usages(concat!(
            r#"[Background(Image="bg_not_loaded")]"#,
            "\n",
            r#"[Background(image="bg_loaded")]"#,
        ));
        assert_eq!(all.len(), 1);
        assert_eq!(all[0].resource_id, "bg_loaded");
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

        let hunter = find(&all, StoryResourceType::Character, "avg_npc_009");
        let korul = find(&all, StoryResourceType::Character, "avg_npc_003");
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
        let hunter = find(&all, StoryResourceType::Character, "avg_npc_009");
        assert_eq!(hunter.display_names, Vec::<String>::new());
    }

    #[test]
    fn single_character_on_stage_gets_names_without_focus() {
        let all = usages(concat!(
            r#"[Character(name="char_220_grani#5",fadetime=1.5,block=true)]"#,
            "\n",
            r#"[name="？？？"]   所以，得把你们全部解决掉才行？"#,
        ));
        let granhi = find(&all, StoryResourceType::Character, "char_220_grani#5");
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
        let npc = find(&all, StoryResourceType::Character, "avg_npc_008");
        assert_eq!(npc.display_names, Vec::<String>::new());
    }

    #[test]
    fn multiline_and_multi_param_dialog_tags_carry_speakers() {
        let all = usages(concat!(
            r#"[Character(name="avg_npc_009")]"#,
            "\n",
            r#"[multiline(name="赏金猎人",end=false)]   前半句"#,
            "\n",
            r#"[multiline(end=true)]   后半句"#,
            "\n",
            r#"[name="粗暴的赏金猎人",avatarId=1]   text"#,
            "\n",
        ));
        let hunter = find(&all, StoryResourceType::Character, "avg_npc_009");
        assert_eq!(hunter.display_names, vec!["赏金猎人", "粗暴的赏金猎人"]);
    }

    #[test]
    fn explicit_focus_zero_behaves_like_an_omitted_focus() {
        let all = usages(concat!(
            r#"[Character(name="avg_npc_009",focus=0)]"#,
            "\n",
            r#"[name="赏金猎人"]   text"#,
            "\n",
        ));
        let hunter = find(&all, StoryResourceType::Character, "avg_npc_009");
        assert_eq!(hunter.display_names, vec!["赏金猎人"]);
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
        let focused = find(&all, StoryResourceType::Character, "avg_npc_242");
        assert_eq!(focused.display_names, vec!["流浪者"]);
        let left = find(&all, StoryResourceType::Character, "avg_npc_416_1#1$1");
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
        let hunter = find(&all, StoryResourceType::Character, "avg_npc_009");
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
        let hunter = find(&all, StoryResourceType::Character, "avg_npc_009");
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

        assert_eq!(
            find(&all, StoryResourceType::Background, "bg_med").sort_order,
            0
        );
        assert_eq!(
            find(&all, StoryResourceType::Character, "avg_npc_009").sort_order,
            1
        );
        assert_eq!(find(&all, StoryResourceType::Image, "ac1_0").sort_order, 2);
        assert_eq!(
            find(&all, StoryResourceType::Background, "bg_tower").sort_order,
            3
        );
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
        let korul = find(&all, StoryResourceType::Character, "avg_npc_003");
        assert_eq!(korul.display_names, vec!["可萝尔"]);
    }
}
