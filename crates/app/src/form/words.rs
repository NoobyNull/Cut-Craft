//! Every form's text has its English words in `locales/en/settings.ftl`, so a language pack
//! translates the forms too. The forms are made from the commands' own descriptions (and the
//! Options, Snapping Options, Job Setup Sheet, Part Labels and STL windows' settings), so the
//! words are kept in the code and copied to Fluent: `write_settings_words` writes the file,
//! and these tests say what to change when a form changes.

use super::{Field, Kind, english_fields, setting_key};
use cut_craft_i18n::english;
use serde_json::Value;
use std::fmt::Write as _;

/// Every schema a form is made from, each with what it is, for the file's comments.
fn schemas() -> Vec<(String, Value)> {
    let as_value = |s: schemars::Schema| serde_json::to_value(s).expect("a schema serialises");
    let mut all = vec![
        (
            "Edit ▸ Options".to_owned(),
            as_value(schemars::schema_for!(crate::options::Options)),
        ),
        (
            "Snapping Options".to_owned(),
            as_value(schemars::schema_for!(cut_craft_core::snap::SnapOptions)),
        ),
        (
            "Job Setup Sheet".to_owned(),
            as_value(schemars::schema_for!(fmt_create_job_sheet::Style)),
        ),
        (
            "Part Labels".to_owned(),
            as_value(schemars::schema_for!(fmt_part_labels::Style)),
        ),
        (
            "Export as STL".to_owned(),
            as_value(schemars::schema_for!(fmt_export_as_stl::Settings)),
        ),
    ];
    let registry = crate::commands::registry();
    for c in registry.commands() {
        all.push((format!("{} ({})", c.meta().label, c.id()), c.schema()));
    }
    all
}

fn field_words(f: &Field, out: &mut Vec<String>) {
    out.push(f.label.clone());
    out.extend(f.help.clone());
    out.extend(f.group.clone());
    out.extend(f.tab.clone());
    if let Some(t) = &f.titles {
        out.extend(t.by.iter().map(|(_, title)| title.clone()));
    }
    kind_words(&f.kind, out);
}

fn kind_words(kind: &Kind, out: &mut Vec<String>) {
    match kind {
        Kind::Choice(options, _) => {
            for o in options {
                out.push(o.label.clone());
                out.extend(o.help.clone());
            }
        }
        Kind::Group(inner) => inner.iter().for_each(|f| field_words(f, out)),
        Kind::List(item) => kind_words(item, out),
        _ => {}
    }
}

/// Each form's words, in order, with what the form is; each text once, under the first form
/// that has it.
fn every_text() -> Vec<(String, Vec<String>)> {
    let mut seen: Vec<String> = vec![];
    let mut forms = vec![];
    for (what, schema) in schemas() {
        let mut words: Vec<String> = schema["description"]
            .as_str()
            .map(String::from)
            .into_iter()
            .collect();
        for f in english_fields(&schema) {
            field_words(&f, &mut words);
        }
        let new: Vec<String> = words
            .into_iter()
            .filter(|w| !w.trim().is_empty())
            .filter(|w| {
                let fresh = !seen.contains(w);
                if fresh {
                    seen.push(w.clone());
                }
                fresh
            })
            .collect();
        if !new.is_empty() {
            forms.push((what, new));
        }
    }
    forms
}

const HEADER: &str = "\
# The words of every tool's form and of the Options, Snapping Options, Job Setup Sheet, Part
# Labels and STL windows: each setting's name and tooltip, the choices it offers, and the
# headings over groups and tabs of settings. A key is `setting-`, the text's first words and a
# check of the whole text, so a key changes when its English words do: translate the words
# after `=`, keeping any { \"{\" } as it is. Each text is here once, under the first form that
# has it; a later form with the same words shows the same translation.
";

fn file() -> String {
    let mut out = String::from(HEADER);
    for (what, words) in every_text() {
        let _ = write!(out, "\n## {what}\n");
        for w in words {
            let _ = writeln!(
                out,
                "{} ={}",
                setting_key(&w),
                cut_craft_i18n::ftl_value(&w)
            );
        }
    }
    out
}

/// The English words Fluent gives back for a text, as they compare with the code's: a line's
/// spaces at its ends are not kept by Fluent, so they are left out of both.
fn plain(text: &str) -> String {
    text.lines()
        .map(str::trim)
        .collect::<Vec<_>>()
        .join("\n")
        .trim()
        .to_owned()
}

#[test]
fn every_form_text_has_its_english_words() {
    let mut problems = vec![];
    for (what, words) in every_text() {
        for w in words {
            let key = setting_key(&w);
            match english().words(&key, None) {
                Some(got) if plain(&got) == plain(&w) => {}
                Some(got) => {
                    problems.push(format!("{what}: {key} = {got:?}, but the code says {w:?}"));
                }
                None => problems.push(format!("{what}: {key} is missing ({w:?})")),
            }
        }
    }
    assert!(
        problems.is_empty(),
        "crates/i18n/locales/en/settings.ftl does not match the forms; run `cargo test -p \
         cut-craft --bin cut-craft write_settings_words -- --ignored` to write it again:\n{}",
        problems.join("\n")
    );
}

#[test]
fn no_form_text_is_left_over() {
    let keys: Vec<String> = every_text()
        .into_iter()
        .flat_map(|(_, words)| words)
        .map(|w| setting_key(&w))
        .collect();
    let stale: Vec<&String> = english()
        .keys()
        .iter()
        .filter(|k| k.starts_with("setting-") && !keys.contains(k))
        .collect();
    assert!(
        stale.is_empty(),
        "no form has these any more (write_settings_words writes the file again): {stale:?}"
    );
}

#[test]
fn a_form_text_reads_back_from_fluent_as_written() {
    for text in [
        "Depth",
        "A {curly} value",
        "[1] first\n\n*bold* second\n.dot line",
    ] {
        let source = format!("k ={}\n", cut_craft_i18n::ftl_value(text));
        let (words, problems) = cut_craft_i18n::Catalogue::parse("en", &[("a.ftl", &source)]);
        assert!(problems.is_empty(), "{source}: {problems:?}");
        assert_eq!(words.words("k", None).as_deref(), Some(text), "{source}");
    }
}

/// Writes `locales/en/settings.ftl` from the forms (`cargo test -p cut-craft --bin cut-craft
/// write_settings_words -- --ignored`), after a form's words change.
#[test]
#[ignore = "writes a file; run by hand"]
fn write_settings_words() {
    let path =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../i18n/locales/en/settings.ftl");
    std::fs::write(path, file()).unwrap();
}
