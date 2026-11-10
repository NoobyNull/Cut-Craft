//! Keeps every command's error messages honest: run each command the way a beginner might
//! (on an empty job, on shapes, with every number at 0) and each message it gives must read as
//! a sentence and say what to do about it, not only what is wrong.

use cut_craft_core::document::Document;
use cut_craft_core::testing;
use serde_json::Value;
use std::collections::BTreeSet;

/// Words that start a next step: a message holding one of them tells the person what to do.
const STEPS: &[&str] = &[
    "add",
    "calculate",
    "check",
    "choose",
    "click",
    "close",
    "create",
    "delete",
    "drag",
    "draw",
    "edit",
    "enter",
    "export",
    "give",
    "hide",
    "import",
    "install",
    "join",
    "lower",
    "make",
    "move",
    "open",
    "pick",
    "press",
    "raise",
    "rename",
    "run",
    "save",
    "select",
    "set",
    "show",
    "try",
    "turn",
    "type",
    "use",
];

/// Messages (or how they start) that rightly give no step, each with why: the person has
/// nothing to fix.
const NO_STEP: &[&str] = &[
    // Move Up and Move Down at the end of the list: nothing is wrong
    "It is already at the top of the list.",
    "It is already at the bottom of the list.",
    "It is already at the top of its level.",
    "It is already at the bottom of its level.",
    "It is already at the end of the list.",
    // Vector Validator's all-clear
    "No problems found in",
];

/// `params` with every number 0, as a form cleared by a beginner would send.
fn zeroed(params: &Value) -> Value {
    match params {
        Value::Number(_) => Value::from(0),
        Value::Array(items) => Value::Array(items.iter().map(zeroed).collect()),
        Value::Object(fields) => Value::Object(
            fields
                .iter()
                .map(|(key, value)| (key.clone(), zeroed(value)))
                .collect(),
        ),
        other => other.clone(),
    }
}

/// Whether `message` holds a next step: one of [`STEPS`] as a word of its own.
fn says_what_to_do(message: &str) -> bool {
    message
        .split(|c: char| !c.is_alphabetic())
        .any(|word| STEPS.contains(&word.to_lowercase().as_str()))
}

/// Every message every command gives, run on a few jobs with its default settings and with
/// every number at 0, as (command, message).
fn every_message() -> BTreeSet<(String, String)> {
    let jobs = [
        Document::default(),
        testing::sample_document(),
        testing::with_toolpath(),
        testing::two_components(),
        testing::domed(),
    ];
    let mut messages = BTreeSet::new();
    for command in crate::commands::registry().iter() {
        let default = command.default_params();
        for job in &jobs {
            for params in [default.clone(), zeroed(&default)] {
                if let Err(e) = command.run_json(job, &params) {
                    messages.insert((command.id().to_owned(), e.to_string()));
                }
            }
        }
    }
    messages
}

#[test]
fn every_error_says_what_to_do_about_it() {
    let messages = every_message();
    assert!(messages.len() > 100, "only {} messages", messages.len());
    let dead_ends: Vec<String> = messages
        .iter()
        .filter(|(_, message)| {
            !says_what_to_do(message) && !NO_STEP.iter().any(|ok| message.starts_with(ok))
        })
        .map(|(id, message)| format!("{id}: {message}"))
        .collect();
    assert!(
        dead_ends.is_empty(),
        "these say what is wrong but not what to do; add the step (\"Pick a smaller tool\"):\n{}",
        dead_ends.join("\n")
    );
}

#[test]
fn every_error_reads_as_a_sentence_without_a_raw_prefix() {
    for (id, message) in every_message() {
        assert!(
            message.starts_with(|c: char| !c.is_lowercase()),
            "{id}: {message}"
        );
        assert!(message.ends_with(['.', '!', '?']), "{id}: {message}");
        assert!(
            !message.to_lowercase().contains("invalid parameters"),
            "{id}: {message}"
        );
    }
}

#[test]
fn a_step_is_found_as_a_word_of_its_own() {
    assert!(says_what_to_do("Pick a smaller tool."));
    assert!(says_what_to_do("Nothing fits. Set Width to more than 0."));
    assert!(
        !says_what_to_do("The settings are unset."),
        "not inside another word"
    );
    assert!(!says_what_to_do("The tool is too big for the shapes."));
}
