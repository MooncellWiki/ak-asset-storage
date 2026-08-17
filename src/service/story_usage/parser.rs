//! Minimal story script lexer for resource extraction.
//!
//! Ported from the `prts-widgets` `StoryPlayer` engine parser
//! (`src/widgets/StoryPlayer/engine/parser.ts`), covering only what resource
//! extraction needs: lowercased command names, argument maps, and dialogue
//! speaker names. Native provenance is `Torappu.AVG.AVGParser`.

use std::collections::HashMap;
use std::sync::LazyLock;

use regex::Regex;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParsedLine {
    Command {
        name: String,
        args: HashMap<String, String>,
    },
    Dialogue {
        speaker: String,
    },
    Narration,
}

/// `^\[\s*(?:(.*?)\((.*)\)|(?:([.|\w]*)|(.*)))\s*\]\s*(.*)`
///
/// `paren`/`args` capture `Name(params)`, `bare` captures plain word commands
/// such as `[Dialog]`, and `fallback` captures the rest (notably
/// `[name="..."]`). Keys are lowercased like arkwaifu's directive parser; the
/// native dictionary keeps source case but every resource-bearing command in
/// the corpus uses lowercase keys.
static COMMAND_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"^\[\s*(?:(?P<paren>.*?)\((?P<args>.*)\)|(?:(?P<bare>[.|\w]*)|(?P<fallback>.*?)))\s*\]\s*(?P<content>.*)$",
    )
    .expect("story command regex must compile")
});

/// `^name\s*=\s*"((?:\\.|[^"])*)"$`
static DIALOGUE_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"^name\s*=\s*"((?:\\.|[^"])*)"$"#).expect("story dialogue regex must compile")
});

#[must_use]
pub fn parse_script(source: &str) -> Vec<ParsedLine> {
    logical_lines(source)
        .iter()
        .map(|line| parse_line(line))
        .collect()
}

/// Ports `AVGParser._ReadNextBlock`: CRLF normalization, backslash
/// continuation, and blank/comment (`//`) filtering.
fn logical_lines(source: &str) -> Vec<String> {
    let normalized = source.replace("\r\n", "\n").replace('\r', "\n");
    let physical: Vec<&str> = normalized.split('\n').collect();

    let mut lines = Vec::new();
    let mut index = 0;
    while index < physical.len() {
        let mut raw = physical[index].to_string();
        while raw.ends_with('\\') && index + 1 < physical.len() {
            index += 1;
            raw.pop();
            raw.push_str(physical[index]);
        }
        index += 1;

        if raw.trim().is_empty() || raw.trim_start().starts_with("//") {
            continue;
        }
        lines.push(raw);
    }
    lines
}

#[must_use]
pub fn parse_line(raw: &str) -> ParsedLine {
    let Some(caps) = COMMAND_REGEX.captures(raw) else {
        return ParsedLine::Narration;
    };

    let fallback = caps.name("fallback").map(|m| m.as_str());
    if let Some(fallback) = fallback
        && let Some(speaker) = DIALOGUE_REGEX
            .captures(fallback.trim())
            .and_then(|caps| caps.get(1).map(|m| unescape_quoted('"', m.as_str())))
    {
        return ParsedLine::Dialogue { speaker };
    }

    let bare = caps
        .name("bare")
        .map(|m| m.as_str())
        .filter(|name| !name.is_empty());
    let paren = caps
        .name("paren")
        .map(|m| m.as_str())
        .filter(|name| !name.is_empty());
    let name = bare.or(paren).unwrap_or("dialog").trim().to_lowercase();

    let args_raw = caps
        .name("args")
        .map(|m| m.as_str())
        .or(fallback)
        .unwrap_or("");

    ParsedLine::Command {
        name,
        args: parse_args(args_raw),
    }
}

fn parse_args(raw: &str) -> HashMap<String, String> {
    let mut args = HashMap::new();
    for part in split_args(raw) {
        let Some((key, value)) = part.split_once('=') else {
            continue;
        };
        let key = key.trim().to_lowercase();
        if !key.is_empty() {
            args.insert(key, parse_value(value));
        }
    }
    args
}

/// Splits on commas that are outside quotes and nested parentheses.
fn split_args(raw: &str) -> Vec<String> {
    let mut parts = Vec::new();
    let mut current = String::new();
    let mut depth = 0usize;
    let mut quote = None;
    let mut escaped = false;

    for ch in raw.chars() {
        if escaped {
            current.push(ch);
            escaped = false;
            continue;
        }
        if ch == '\\' && quote.is_some() {
            current.push(ch);
            escaped = true;
            continue;
        }
        if let Some(open) = quote {
            current.push(ch);
            if ch == open {
                quote = None;
            }
            continue;
        }
        match ch {
            '"' | '\'' => {
                quote = Some(ch);
                current.push(ch);
            }
            '(' => {
                depth += 1;
                current.push(ch);
            }
            ')' => {
                depth = depth.saturating_sub(1);
                current.push(ch);
            }
            ',' if depth == 0 => {
                if !current.trim().is_empty() {
                    parts.push(current.trim().to_string());
                }
                current.clear();
            }
            _ => current.push(ch),
        }
    }

    if !current.trim().is_empty() {
        parts.push(current.trim().to_string());
    }
    parts
}

fn parse_value(raw: &str) -> String {
    let value = raw.trim();
    if value.chars().count() >= 2 {
        let first = value.chars().next().expect("checked non-empty");
        let last = value.chars().last().expect("checked non-empty");
        if (first == '"' && last == '"') || (first == '\'' && last == '\'') {
            let body = &value[first.len_utf8()..value.len() - last.len_utf8()];
            return unescape_quoted(first, body);
        }
    }
    value.to_string()
}

fn unescape_quoted(quote: char, body: &str) -> String {
    let mut out = String::with_capacity(body.len());
    let mut chars = body.chars();
    while let Some(ch) = chars.next() {
        if ch != '\\' {
            out.push(ch);
            continue;
        }
        let Some(escaped) = chars.next() else {
            out.push('\\');
            break;
        };
        match escaped {
            'n' => out.push('\n'),
            'r' => out.push('\r'),
            't' => out.push('\t'),
            '\\' => out.push('\\'),
            _ if escaped == quote => out.push(escaped),
            _ => {
                out.push('\\');
                out.push(escaped);
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn command(line: &str) -> (String, HashMap<String, String>) {
        let ParsedLine::Command { name, args } = parse_line(line) else {
            panic!("expected command for {line}");
        };
        (name, args)
    }

    #[test]
    fn parses_parenthesized_command_with_lowercased_name() {
        let (name, args) = command(r#"[PlayMusic(intro="$darkness01_intro", volume=0.6)]"#);
        assert_eq!(name, "playmusic");
        assert_eq!(
            args.get("intro").map(String::as_str),
            Some("$darkness01_intro")
        );
        assert_eq!(args.get("volume").map(String::as_str), Some("0.6"));
    }

    #[test]
    fn parses_bare_command() {
        let (name, args) = command("[Dialog]");
        assert_eq!(name, "dialog");
        assert!(args.is_empty());
    }

    #[test]
    fn parses_dialogue_line_with_speaker_and_text() {
        let ParsedLine::Dialogue { speaker } =
            parse_line(r#"[name="赏金猎人"]   这女人，还不肯说吗？"#)
        else {
            panic!("expected dialogue");
        };
        assert_eq!(speaker, "赏金猎人");
    }

    #[test]
    fn unquoted_name_becomes_dialog_command() {
        let (name, args) = command("[name=阿米娅] text");
        assert_eq!(name, "dialog");
        assert_eq!(args.get("name").map(String::as_str), Some("阿米娅"));
    }

    #[test]
    fn narration_lines_are_untouched() {
        assert_eq!(parse_line("   ......"), ParsedLine::Narration);
        assert_eq!(parse_line(""), ParsedLine::Narration);
    }

    #[test]
    fn skips_comments_and_blank_lines() {
        let lines = parse_script("// comment\n\n   \n[Dialog]\n// trailing");
        assert_eq!(lines.len(), 1);
        assert_eq!(
            lines[0],
            ParsedLine::Command {
                name: "dialog".to_string(),
                args: HashMap::new(),
            }
        );
    }

    #[test]
    fn joins_backslash_continuation() {
        let lines = parse_script("[Blocker(a=1, \\\nr=0, block=true)]");
        assert_eq!(lines.len(), 1);
        let (name, args) = match &lines[0] {
            ParsedLine::Command { name, args } => (name.as_str(), args),
            other => panic!("expected command, got {other:?}"),
        };
        assert_eq!(name, "blocker");
        assert_eq!(args.get("r").map(String::as_str), Some("0"));
        assert_eq!(args.get("block").map(String::as_str), Some("true"));
    }

    #[test]
    fn handles_escaped_quotes_and_nested_parens_in_args() {
        let (_, args) = command(r#"[Foo(text="a \"b\", c", pos=(1,2), key='x')]"#);
        assert_eq!(args.get("text").map(String::as_str), Some(r#"a "b", c"#));
        assert_eq!(args.get("pos").map(String::as_str), Some("(1,2)"));
        assert_eq!(args.get("key").map(String::as_str), Some("x"));
    }

    #[test]
    fn dialogue_speaker_unescapes_sequences() {
        let ParsedLine::Dialogue { speaker } = parse_line(r#"[name="a \"q\" \n \\ z"] t"#) else {
            panic!("expected dialogue");
        };
        assert_eq!(speaker, "a \"q\" \n \\ z");
    }

    #[test]
    fn lowercases_argument_keys() {
        let (_, args) = command("[Foo(Image=bg)]");
        assert_eq!(args.get("image").map(String::as_str), Some("bg"));
    }

    #[test]
    fn command_without_name_falls_back_to_dialog() {
        let (name, _) = command("[]");
        assert_eq!(name, "dialog");
    }

    #[test]
    fn crlf_line_endings_are_normalized() {
        let lines = parse_script("[Dialog]\r\n[name=\"X\"] hi\r\n");
        assert_eq!(lines.len(), 2);
    }
}
