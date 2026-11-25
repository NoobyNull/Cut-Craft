//! What a test does to the app and reads back from it, in a user's terms.
//!
//! Tests reach the app only through these verbs and the harness's input ones (`click`,
//! `drag`, `text_at`, ...), never its fields: so the app's insides can be rearranged without
//! touching the tests, which then fail only when what the user sees or makes changes.
//!
//! Each area's verbs are in its own file under `verbs/`.

mod bit_library;
mod checks;
mod files;
mod help;
mod interface;
mod job;
mod machines;
mod panels;
mod preview;
mod settings;
mod snapping;
mod toolpaths;
mod tools;
mod unsigned_code;
mod view_3d;
mod views;
mod window;

/// The test files moved onto the verbs, each with its source: they never reach into the app.
const THROUGH_THE_HARNESS: [(&str, &str); 26] = [
    ("layers_sheets.rs", include_str!("layers_sheets.rs")),
    (
        "accessibility_tests.rs",
        include_str!("../accessibility_tests.rs"),
    ),
    ("canvas.rs", include_str!("canvas.rs")),
    ("split_views.rs", include_str!("split_views.rs")),
    ("updating.rs", include_str!("updating.rs")),
    ("locked_face.rs", include_str!("locked_face.rs")),
    ("rotary_wrap.rs", include_str!("rotary_wrap.rs")),
    ("unsigned_code.rs", include_str!("unsigned_code.rs")),
    ("rotary.rs", include_str!("rotary.rs")),
    ("moving.rs", include_str!("moving.rs")),
    ("calculating.rs", include_str!("calculating.rs")),
    ("keys_and_mouse.rs", include_str!("keys_and_mouse.rs")),
    ("machines.rs", include_str!("machines.rs")),
    ("toolpath_list.rs", include_str!("toolpath_list.rs")),
    ("drawing.rs", include_str!("drawing.rs")),
    ("materials.rs", include_str!("materials.rs")),
    ("tool_database.rs", include_str!("tool_database.rs")),
    ("backups.rs", include_str!("backups.rs")),
    ("custom_toolbars.rs", include_str!("custom_toolbars.rs")),
    ("view_3d.rs", include_str!("view_3d.rs")),
    ("help_pages.rs", include_str!("help_pages.rs")),
    ("did_you_know.rs", include_str!("did_you_know.rs")),
    ("interface.rs", include_str!("interface.rs")),
    ("toolpaths.rs", include_str!("toolpaths.rs")),
    (
        "snapping_and_guides.rs",
        include_str!("snapping_and_guides.rs"),
    ),
    ("collisions.rs", include_str!("collisions.rs")),
];

/// The lines of `source` that reach into the app past the harness.
fn reaching_in(source: &str) -> Vec<(usize, &str)> {
    source
        .lines()
        .enumerate()
        .map(|(n, line)| (n + 1, line.trim()))
        .filter(|(_, line)| reaches_app(line) || line.contains("CutCraftApp"))
        .collect()
}

/// Whether `line` names a harness's `.app` (`h.app.doc`, `next.app`, `hs[0].app`, a wrapped
/// `.app.form`), not a path such as `crate::app::` or a name such as `h.app_state`.
fn reaches_app(line: &str) -> bool {
    let word = |c: char| c.is_alphanumeric() || c == '_';
    line.match_indices(".app").any(|(at, _)| {
        let before = line[..at].chars().next_back();
        let after = line[at + ".app".len()..].chars().next();
        after.is_none_or(|c| !word(c)) && before.is_none_or(|c| word(c) || c == ')' || c == ']')
    })
}

#[test]
fn reaching_in_finds_the_app_however_the_line_is_wrapped() {
    let source = "h.app.doc.notes = x;\nlet m = h\n    .app\n    .form;\nh\n    .app.apply(a);\n|app: &CutCraftApp| 1\nh.doc().notes\nnext.app.session\nuse crate::app::job_files;\nh.app_state()\nharnesses[0].app.doc\n";
    let lines: Vec<usize> = reaching_in(source).iter().map(|(n, _)| *n).collect();
    assert_eq!(lines, [1, 3, 6, 7, 9, 12]);
}

#[test]
fn moved_test_files_reach_the_app_only_through_the_harness() {
    for (name, source) in THROUGH_THE_HARNESS {
        let lines = reaching_in(source);
        assert!(
            lines.is_empty(),
            "{name} reaches into the app; use a verb in tests/verbs.rs (add one if none \
             fits):\n{}",
            lines
                .iter()
                .map(|(n, line)| format!("  {name}:{n}: {line}"))
                .collect::<Vec<_>>()
                .join("\n")
        );
    }
}
