//! Tools and bits: a button on the screen is a **tool**, what goes in the spindle is a **bit**.
//! These check the English the app and its help show keeps the two apart, and that Find Tool
//! still finds the bits' tools by the words people used before.

use super::*;
use std::path::{Path, PathBuf};

/// Words that call a cutter a tool. Each says "bit" now (the Bit Library, Bit Number, bit
/// changes...). "Tool changer" stays: it is the machine part's own name.
const CUTTER_AS_TOOL: &[&str] = &[
    "tool database",
    "my tools",
    "hide unset tools",
    "new tool group",
    "tool number",
    "tool type",
    "tool info",
    "tool shape",
    "tool numbers",
    "tool change",
    "tool changes",
    "tool's diameter",
    "tool diameter",
    "tool's pass depth",
    "the tool goes round",
    "middle of the tool",
    "same tool",
    "file for each tool",
    "clearance tools",
    "add a tool",
    "no tool picked",
    "size from the tool",
    "pick a smaller tool",
    "tool's feed",
];

/// The files about toolbars of your own, where "My Tools" is a toolbar's name, not bits.
const TOOLBAR_FILES: &[&str] = &["toolbars.ftl", "toolbars.md"];

/// The cutter words found in `text`, each with where: `place` and the line.
fn cutter_words(place: &str, text: &str, found: &mut Vec<String>) {
    for (n, line) in text.lines().enumerate() {
        let lower = line.to_lowercase();
        // file kinds of other programs keep their own names
        let lower = lower.replace("vectric tool database", "");
        for words in CUTTER_AS_TOOL {
            if holds(&lower, words) {
                found.push(format!("{place}:{}: {words:?} in {line:?}", n + 1));
            }
        }
    }
}

/// Whether `text` holds `words` as whole words: "same tool" is not in "same tooltip".
fn holds(text: &str, words: &str) -> bool {
    let edge = |c: Option<char>| c.is_none_or(|c| !c.is_alphanumeric());
    text.match_indices(words.trim_end()).any(|(at, found)| {
        edge(text[..at].chars().next_back()) && edge(text[at + found.len()..].chars().next())
    })
}

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn files(dir: &Path, ext: &str, out: &mut Vec<PathBuf>) {
    for entry in std::fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            files(&path, ext, out);
        } else if path.extension().is_some_and(|e| e == ext) {
            out.push(path);
        }
    }
}

/// The text a Rust file shows, up to its own tests: its string literals' lines, and its doc
/// comments where they are shown (a command's `//!` is its tooltip, and the `///` of anything
/// a form or settings window draws is its help).
fn shown_lines(source: &str, docs_shown: bool) -> String {
    let source = source.split("#[cfg(test)]").next().unwrap_or_default();
    source
        .lines()
        .filter(|line| {
            let code = line.trim_start();
            let doc = code.starts_with("///") || code.starts_with("//!");
            (doc && docs_shown) || (line.contains('"') && !code.starts_with("//"))
        })
        .collect::<Vec<_>>()
        .join("\n")
}

#[test]
fn cutters_are_called_bits_in_the_english_words() {
    let root = root();
    let mut found = Vec::new();
    let mut ftl = Vec::new();
    files(&root.join("crates/i18n/locales/en"), "ftl", &mut ftl);
    let mut pages = Vec::new();
    files(&root.join("crates/help/pages"), "md", &mut pages);
    for file in ftl.iter().chain(&pages) {
        let text = std::fs::read_to_string(file).unwrap();
        // a sentence may break its line between "Tool" and "Database"
        let mut joined = text.split_whitespace().collect::<Vec<_>>().join(" ");
        // a toolbar of your own holds tools (buttons), so its first name is "My Tools"
        if TOOLBAR_FILES.iter().any(|name| file.ends_with(name)) {
            joined = joined.replace("My Tools", "");
        }
        cutter_words(&file.display().to_string(), &joined, &mut found);
    }
    let mut code = Vec::new();
    for dir in [
        "app/src",
        "tool-database/src",
        "machines/src",
        "core/src",
        "commands",
    ] {
        files(&root.join("crates").join(dir), "rs", &mut code);
    }
    // tests, and the words Find Tool finds the Bit Library by, may say the old words
    let left_out = [
        "tests.rs",
        "bit_words.rs",
        "tool_names.rs",
        "error_messages.rs",
    ];
    let is_left_out = |f: &&PathBuf| {
        left_out.iter().any(|name| f.ends_with(name))
            || f.components().any(|c| c.as_os_str() == "tests")
    };
    for file in code.iter().filter(|f| !is_left_out(f)) {
        let source = std::fs::read_to_string(file).unwrap();
        let docs_shown =
            file.components().any(|c| c.as_os_str() == "commands") || source.contains("JsonSchema");
        let lines = shown_lines(&source, docs_shown);
        cutter_words(&file.display().to_string(), &lines, &mut found);
    }
    assert!(
        found.is_empty(),
        "these call a bit a tool; say bit (CONTRIBUTING.md, Tools and bits):\n{}",
        found.join("\n")
    );
}

#[test]
fn the_check_finds_a_cutter_called_a_tool() {
    let mut found = Vec::new();
    cutter_words("x", "Pick it in the Tool Database", &mut found);
    cutter_words(
        "x",
        "the Tool Changer slots; a Vectric tool database file",
        &mut found,
    );
    cutter_words("x", "One file, pause at each tool change.", &mut found);
    cutter_words("x", "the same tooltip as another tool", &mut found);
    assert_eq!(found.len(), 2, "{found:#?}");
}

#[test]
fn find_tool_finds_the_bit_library_by_the_old_words() {
    for typed in [
        "tool",
        "Tool Database",
        "tooling",
        "cutter",
        "end mill",
        "bit",
    ] {
        assert!(
            found_by("tool_database", "Bit Library", typed),
            "{typed:?} finds the Bit Library"
        );
    }
    assert!(found_by("draw_circle", "Draw Circle", "circ"));
    assert!(!found_by("draw_circle", "Draw Circle", "cutter"));
    assert!(!found_by("tool_database", "Bit Library", "pocket"));
}
