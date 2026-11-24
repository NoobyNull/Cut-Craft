//! The commit button, by the kind of form each command says it has.

use super::*;
use std::collections::BTreeMap;

/// Every command whose form is not Apply, by kind: what each command declares in its `Meta`,
/// checked against the buttons it showed before commands said so themselves.
fn declared() -> BTreeMap<String, Vec<&'static str>> {
    let mut kinds = BTreeMap::<String, Vec<_>>::new();
    for c in crate::commands::registry().commands() {
        let kind = c.meta().form;
        if kind != Form::Apply && kind != Form::Toolpath {
            kinds.entry(format!("{kind:?}")).or_default().push(c.id());
        }
    }
    for ids in kinds.values_mut() {
        ids.sort_unstable();
    }
    kinds
}

#[test]
fn a_drawing_tool_offers_create_until_its_shape_follows_the_form() {
    let c = |made| commit(Form::Apply, Page::Design, true, made);
    assert_eq!(c(false), Some(Commit::Create));
    assert_eq!(c(true), None, "the shape already follows the form");
}

#[test]
fn canvas_only_tools_and_live_forms_have_no_commit_button() {
    for made in [false, true] {
        for drawing in [false, true] {
            let c = |kind| commit(kind, Page::Design, drawing, made);
            assert_eq!(c(Form::Gesture), None);
            assert_eq!(
                c(Form::Live {
                    create_first: false
                }),
                None
            );
        }
    }
}

/// Relief from Photo made nothing at all: with no button and nothing made yet, its
/// settings had nothing to follow (Matthew, 2026-11-22: "nothing happens after I import").
#[test]
fn a_live_form_that_creates_first_offers_create_until_its_piece_follows_the_form() {
    let c = |made| commit(Form::Live { create_first: true }, Page::Design, false, made);
    assert_eq!(c(false), Some(Commit::Create));
    assert_eq!(c(true), None, "the relief already follows the form");
}

#[test]
fn a_remaking_form_creates_once_then_follows_its_settings() {
    let c = |keeps_apply, made| commit(Form::Remake { keeps_apply }, Page::Design, false, made);
    assert_eq!(c(false, false), Some(Commit::Create));
    assert_eq!(c(false, true), None);
    assert_eq!(c(true, false), Some(Commit::Create));
    assert_eq!(
        c(true, true),
        Some(Commit::Apply),
        "Embellish keeps its Apply"
    );
}

#[test]
fn toolpaths_calculate_on_the_toolpaths_page() {
    assert_eq!(
        commit(Form::Toolpath, Page::Toolpaths, false, false),
        Some(Commit::Calculate)
    );
    assert_eq!(
        commit(Form::Apply, Page::Toolpaths, false, false),
        Some(Commit::Apply),
        "the toolpath page's settings (Material Setup, Job Setup) apply"
    );
}

#[test]
fn making_forms_create_generating_forms_generate_and_changing_forms_apply() {
    let c = |kind| commit(kind, Page::Design, false, false);
    assert_eq!(c(Form::Create), Some(Commit::Create));
    assert_eq!(c(Form::Generate), Some(Commit::Generate));
    assert_eq!(c(Form::Apply), Some(Commit::Apply));
}

/// What each command says its form is, as the app's lists of names said before the commands
/// said it themselves.
#[test]
fn every_command_declares_the_form_it_had() {
    let kinds = declared();
    let ids = |kind: Form| kinds.get(&format!("{kind:?}")).cloned().unwrap_or_default();
    assert_eq!(
        ids(Form::Gesture),
        ["draw_freehand", "interactive_vector_trim", "sculpt"]
    );
    assert_eq!(
        ids(Form::Create),
        [
            "add_layer",
            "add_sheet",
            "array_copy",
            "celtic_weave",
            "circular_copy",
            "copy_along_vectors",
            "create_component_from_bitmap",
            "create_shape",
            "draw_arc",
            "draw_curve",
            "engrave_part_names",
            "extrude_and_weave",
            "insert_level",
            "plate_production",
            "quick_trace",
            "texture_area",
            "turn_and_spin",
            "two_rail_sweep",
            "vector_boundary",
            "wrapped_fluting_layout",
            "wrapped_spiral_layout",
        ]
    );
    assert_eq!(ids(Form::Generate), ["generate_border"]);
    assert_eq!(
        ids(Form::Live {
            create_first: false
        }),
        ["orientate_3d_model"]
    );
    assert_eq!(
        ids(Form::Live { create_first: true }),
        ["relief_from_photo"]
    );
    assert_eq!(ids(Form::Remake { keeps_apply: true }), ["embellish"]);
    assert_eq!(
        ids(Form::Remake { keeps_apply: false }),
        ["decorative_relief", "vector_texture"]
    );
}

/// The toolpath strategies calculate; the commands that only edit the toolpath list apply.
#[test]
fn every_toolpath_strategy_and_only_those_calculate() {
    for c in crate::commands::registry().commands() {
        let toolpath = c.meta().form == Form::Toolpath;
        let edits_list = [
            "delete_toolpath",
            "duplicate_toolpath",
            "reorder_toolpath",
            "ungroup_merged_toolpath",
            "copy_toolpath_to_sheet",
        ]
        .contains(&c.id());
        let strategy = c.id().ends_with("_toolpath") && !edits_list;
        assert_eq!(toolpath, strategy, "{}", c.id());
    }
}

#[test]
fn gesture_tools_are_drawn_on_the_canvas() {
    for c in crate::commands::registry().commands() {
        if c.meta().form == Form::Gesture {
            assert!(c.interactive(), "{} is drawn", c.id());
        }
    }
}

#[test]
fn the_labels_and_hints_say_what_pressing_does() {
    assert_eq!(Commit::Calculate.label(), "Calculate");
    assert!(hint(Commit::Create, true, false).contains("drag on the canvas"));
    assert!(hint(Commit::Calculate, false, true).contains("again"));
    assert!(live_note(Form::Gesture, true).contains("canvas"));
    assert!(live_note(Form::Apply, true).contains("selected shape"));
}
