//! Checks that no new words are written straight into the app's code, where a language pack
//! cannot reach them.
//!
//! The words the app shows belong in `crates/i18n/locales/en/*.ftl`, under a key the code
//! asks for with `tr!("key")`. This scans the app's code for text handed straight to a button,
//! label, tooltip, menu or window, and fails on any not listed in
//! `crates/app/i18n/hard-coded-baseline.txt`: the text that was there before it moved to
//! Fluent, waiting its turn. Moving one empties its line, and a stale line fails too, so the
//! list only shrinks.
//!
//! It also checks that every key the code asks for has English words.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// The text already known to be written into the code, one `file | "text"` a line.
const BASELINE: &str = include_str!("../../i18n/hard-coded-baseline.txt");

/// Calls whose text is shown to the person: a literal anywhere in their brackets (or in a
/// `format!` there) is written into the code.
const SHOWN: &[&str] = &[
    "button",
    "small_button",
    "label",
    "heading",
    "strong",
    "weak",
    "small",
    "monospace",
    "menu_button",
    "checkbox",
    "radio",
    "radio_value",
    "selectable_label",
    "selectable_value",
    "on_hover_text",
    "on_disabled_hover_text",
    "on_hover_hint",
    "on_disabled_hover_hint",
    "on_hover_text_at_pointer",
    "hyperlink_to",
    "collapsing",
    "hint_text",
    "image_and_text",
    "Button::new",
    "Label::new",
    "RichText::new",
    "Window::new",
    "CollapsingHeader::new",
    "Checkbox::new",
    "SelectableLabel::new",
];

fn sources() -> Vec<PathBuf> {
    fn walk(dir: &Path, files: &mut Vec<PathBuf>) {
        for entry in std::fs::read_dir(dir).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                walk(&path, files);
            } else if path.extension().is_some_and(|e| e == "rs") {
                files.push(path);
            }
        }
    }
    let mut files = Vec::new();
    walk(
        &Path::new(env!("CARGO_MANIFEST_DIR")).join("src"),
        &mut files,
    );
    files.sort();
    files
        .into_iter()
        .filter(|f| !f.ends_with("app/tests.rs") && !f.ends_with("web_api_tests.rs"))
        .collect()
}

/// A source file's path from the app crate, with `/` on every computer.
fn short(path: &Path) -> String {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    path.strip_prefix(root)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/")
}

/// A file's program code: up to its test module (not a single test-only function above it),
/// with comments blanked out.
fn program(source: &str) -> String {
    let tests = source
        .match_indices("#[cfg(test)]")
        .find(|(at, attr)| source[at + attr.len()..].trim_start().starts_with("mod "));
    let code = &source[..tests.map_or(source.len(), |(at, _)| at)];
    code.lines()
        .map(|line| {
            let t = line.trim_start();
            if t.starts_with("//") || t.starts_with("#[") {
                ""
            } else {
                line
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// The string literals in `code`, each with where its closing quote ends.
fn literal_at(code: &[u8], at: usize) -> Option<(String, usize)> {
    if code.get(at) != Some(&b'"') {
        return None;
    }
    let mut i = at + 1;
    let mut text = Vec::new();
    while i < code.len() {
        match code[i] {
            b'\\' => {
                text.extend_from_slice(&code[i..(i + 2).min(code.len())]);
                i += 2;
            }
            b'"' => return Some((one_line(&String::from_utf8_lossy(&text)), i + 1)),
            c => {
                text.push(c);
                i += 1;
            }
        }
    }
    None
}

/// A literal as one line: a `\` line break and the next line's indent taken out, as Rust does.
fn one_line(text: &str) -> String {
    let mut out = String::new();
    let mut lines = text.split("\\\n");
    out.push_str(lines.next().unwrap_or_default());
    for line in lines {
        out.push_str(line.trim_start());
    }
    out.replace('\n', " ")
}

/// The text literals inside the brackets that open at `open`: those at the top level of the
/// call, and inside a `format!` there. A literal inside another call is that call's business
/// (it is scanned itself if it shows text).
fn shown_literals(code: &[u8], open: usize) -> Vec<String> {
    let mut found = Vec::new();
    let mut depth = 0usize;
    // the depths whose brackets belong to format!-like macros: their literals are shown too
    let mut shown_depths = vec![1usize];
    let mut i = open;
    while i < code.len() {
        match code[i] {
            b'"' => {
                let Some((text, end)) = literal_at(code, i) else {
                    break;
                };
                if shown_depths.last() == Some(&depth) {
                    found.push(text);
                }
                i = end;
                continue;
            }
            // a character literal: '"' or '\''
            b'\'' if code.get(i + 2) == Some(&b'\'') => {
                i += 3;
                continue;
            }
            b'\'' if code.get(i + 1) == Some(&b'\\') && code.get(i + 3) == Some(&b'\'') => {
                i += 4;
                continue;
            }
            b'(' | b'[' | b'{' => {
                depth += 1;
                let before = &code[..i];
                if before.ends_with(b"format!") || before.ends_with(b"concat!") {
                    shown_depths.push(depth);
                }
            }
            b')' | b']' | b'}' => {
                if shown_depths.last() == Some(&depth) && depth > 1 {
                    shown_depths.pop();
                }
                depth -= 1;
                if depth == 0 {
                    break;
                }
            }
            _ => {}
        }
        i += 1;
    }
    found
}

/// Whether a literal is words a person reads, not an id, a path or a format.
fn is_words(text: &str) -> bool {
    let letters = text.chars().any(char::is_alphabetic);
    let id = !text.contains(' ')
        && (text.contains('_') || text.contains('/') || text.contains(':') || text.contains('.'));
    let picture = [".png", ".svg", ".gif"].iter().any(|e| text.ends_with(e));
    letters && !id && !picture
}

/// Every piece of text written straight into the app's code: `file | "text"`, with how many
/// times it is there.
fn hard_coded() -> BTreeMap<String, usize> {
    let mut found = BTreeMap::new();
    for file in sources() {
        let source = std::fs::read_to_string(&file).unwrap();
        let code = program(&source);
        let bytes = code.as_bytes();
        for call in SHOWN {
            let mut from = 0;
            while let Some(at) = code[from..].find(call) {
                let start = from + at;
                from = start + call.len();
                // a whole name, called: not `fn label(`, `labels(` or `self.button_rect`
                let whole_before = start == 0
                    || !(bytes[start - 1].is_ascii_alphanumeric() || bytes[start - 1] == b'_');
                let defined = code[..start].trim_end().ends_with("fn");
                let mut open = from;
                while bytes.get(open).is_some_and(u8::is_ascii_whitespace) {
                    open += 1;
                }
                if !whole_before || defined || bytes.get(open) != Some(&b'(') {
                    continue;
                }
                for text in shown_literals(bytes, open)
                    .into_iter()
                    .filter(|t| is_words(t))
                {
                    *found
                        .entry(format!("{} | \"{text}\"", short(&file)))
                        .or_default() += 1;
                }
            }
        }
    }
    found
}

fn baseline() -> BTreeMap<String, usize> {
    let mut known = BTreeMap::new();
    for line in BASELINE.lines() {
        let line = line.trim_end();
        if !line.is_empty() && !line.starts_with('#') {
            *known.entry(line.to_owned()).or_default() += 1;
        }
    }
    known
}

/// The baseline's text, for a fresh start: `CUT_CRAFT_WRITE_TEXT_BASELINE=1`.
fn written(found: &BTreeMap<String, usize>) -> String {
    let mut out = String::from(
        "# Text written straight into the app's code, waiting to move into\n\
         # crates/i18n/locales/en/*.ftl: one `file | \"text\"` a line (a text there twice has two\n\
         # lines). The hard-coded text check fails on text not listed here. Move one into Fluent,\n\
         # then delete its line.\n\n",
    );
    for (entry, count) in found {
        for _ in 0..*count {
            out.push_str(entry);
            out.push('\n');
        }
    }
    out
}

#[test]
fn no_new_text_is_written_into_the_code() {
    let found = hard_coded();
    if std::env::var_os("CUT_CRAFT_WRITE_TEXT_BASELINE").is_some() {
        let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("i18n/hard-coded-baseline.txt");
        std::fs::write(path, written(&found)).unwrap();
        return;
    }
    let known = baseline();
    let new: Vec<String> = found
        .iter()
        .filter(|(entry, count)| known.get(*entry).copied().unwrap_or(0) < **count)
        .map(|(entry, _)| entry.clone())
        .collect();
    let moved: Vec<String> = known
        .iter()
        .filter(|(entry, count)| found.get(*entry).copied().unwrap_or(0) < **count)
        .map(|(entry, _)| entry.clone())
        .collect();
    assert!(
        new.is_empty(),
        "Words written straight into the code, where a language pack cannot reach them. Put \
         each in crates/i18n/locales/en/<area>.ftl under a key and show it with tr!(\"key\"):\n{}",
        new.join("\n")
    );
    assert!(
        moved.is_empty(),
        "These have moved into Fluent: delete their lines from \
         crates/app/i18n/hard-coded-baseline.txt:\n{}",
        moved.join("\n")
    );
}

/// The keys the code asks for by name: `tr!("key"`.
fn keys_asked_for() -> Vec<(String, String)> {
    let mut keys = Vec::new();
    for file in sources() {
        let source = program(&std::fs::read_to_string(&file).unwrap());
        for (i, _) in source.match_indices("tr!(\"") {
            let before = source[..i].chars().next_back();
            if before.is_some_and(|c| c.is_alphanumeric() || c == '_') {
                continue;
            }
            let rest = &source[i + 5..];
            if let Some(end) = rest.find('"') {
                keys.push((short(&file), rest[..end].to_owned()));
            }
        }
    }
    keys
}

#[test]
fn every_key_the_code_asks_for_has_english_words() {
    let english = cut_craft_i18n::english();
    let keys = keys_asked_for();
    assert_ne!(keys.len(), 0);
    let missing: Vec<String> = keys
        .iter()
        .filter(|(_, key)| !english.has(key))
        .map(|(file, key)| format!("{file}: {key}"))
        .collect();
    assert!(
        missing.is_empty(),
        "No English words for these keys: add them to crates/i18n/locales/en/*.ftl:\n{}",
        missing.join("\n")
    );
}

#[test]
fn every_menu_and_startup_task_has_english_words() {
    let english = cut_craft_i18n::english();
    let all = crate::gui::MenuSystem::new();
    let menus = [all.without_job(), all];
    for menu in menus.iter().flat_map(|m| &m.menus) {
        let key = super::menu_key(&menu.name);
        assert!(
            english.has(&key),
            "{}: no English words for {key}",
            menu.name
        );
    }
    for (id, _) in super::STARTUP_TASKS {
        let key = super::actions::start_task_key(id);
        assert!(english.has(&key), "no English words for {key}");
    }
    for commit in [
        super::commit::Commit::Create,
        super::commit::Commit::Calculate,
        super::commit::Commit::Apply,
    ] {
        let label = commit.label();
        assert!(!label.contains('-'), "{label}");
        for (drawing, again) in [(false, false), (true, false), (false, true)] {
            let hint = super::commit::hint(commit, drawing, again);
            assert!(!hint.contains("commit-"), "{hint}");
        }
    }
}

#[test]
fn menu_keys_are_the_english_names_lower_case() {
    assert_eq!(super::menu_key("File"), "menu-file");
    assert_eq!(super::menu_key("Tool Database..."), "menu-tool-database");
    assert_eq!(
        super::menu_key("Import Vectors / Bitmap"),
        "menu-import-vectors-bitmap"
    );
}

#[test]
fn the_scan_finds_text_and_leaves_ids_alone() {
    let code =
        br#"(ui, "Open", format!("{} toolpaths", n), self.icon("file/new.png"), 'x', "a_b")"#;
    assert_eq!(shown_literals(code, 0), ["Open", "{} toolpaths", "a_b"]);
    assert!(is_words("Open"));
    assert!(is_words("{} toolpaths"));
    assert!(!is_words("a_b"));
    assert!(!is_words("open_recent:"));
    assert!(!is_words("file/new.png"));
    assert!(!is_words("•"));
    assert!(!is_words("{}"));
}
