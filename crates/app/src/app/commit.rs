//! A tool form's commit button: named for what pressing it does, and shown only when
//! pressing it would do something.
//!
//! - A form whose result already follows its settings (a drawn shape still selected, a model
//!   made as the form opens, an ornament remade as it changes) has no commit button: the
//!   change is already on screen.
//! - A form that makes something new says **Create**; a toolpath's says **Calculate**.
//! - A form that changes what is already there (the selection, a layer's or a job's
//!   settings) says **Apply**.
//!
//! Close (keep what is there) and Cancel (take back what the form did) are always offered.

use super::{LIVE_FORMS, REMAKE_FORMS};
use crate::pages::Page;

/// What a form's commit button does.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Commit {
    /// Makes a new shape, copy, pattern or piece.
    Create,
    /// Works out a toolpath (again, for one being edited).
    Calculate,
    /// Changes what is already there.
    Apply,
}

impl Commit {
    pub(super) fn label(self) -> &'static str {
        match self {
            Self::Create => "Create",
            Self::Calculate => "Calculate",
            Self::Apply => "Apply",
        }
    }
}

/// Tools used only on the canvas: a stroke, a click or a drag is what they do, so their form
/// holds settings for the next one and nothing to press.
pub(super) const GESTURE_TOOLS: &[&str] = &["sculpt", "draw_freehand", "interactive_vector_trim"];

/// Forms that make new pieces from their settings, each press another.
pub(super) const CREATE_FORMS: &[&str] = &[
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
];

/// The commit button form `id` shows, if any. `drawing`: the tool can be drawn with the
/// mouse; `made`: what the form made still follows its settings.
pub(super) fn commit(id: &str, page: Page, drawing: bool, made: bool) -> Option<Commit> {
    if GESTURE_TOOLS.contains(&id) || LIVE_FORMS.contains(&id) {
        return None;
    }
    if drawing || REMAKE_FORMS.contains(&id) {
        return (!made).then_some(Commit::Create);
    }
    if page == Page::Toolpaths && id.ends_with("_toolpath") {
        return Some(Commit::Calculate);
    }
    if CREATE_FORMS.contains(&id) {
        return Some(Commit::Create);
    }
    Some(Commit::Apply)
}

/// What the commit button's hint says it will do.
pub(super) fn hint(commit: Commit, drawing: bool, editing: bool) -> &'static str {
    match commit {
        Commit::Create if drawing => {
            "Make the shape from these numbers. You can also drag on the canvas to draw it"
        }
        Commit::Create => "Make a new one with these settings",
        Commit::Calculate if editing => "Work out this toolpath again with these settings",
        Commit::Calculate => "Work out the toolpath with these settings",
        Commit::Apply => "Make these changes",
    }
}

/// The line a form with no commit button shows where the button would be, saying why there
/// is none.
pub(super) fn live_note(id: &str, drawing: bool) -> &'static str {
    if GESTURE_TOOLS.contains(&id) {
        "Use it on the canvas: each change shows as you make it"
    } else if drawing {
        "Changes show on the selected shape as you make them"
    } else {
        "Changes show as you make them"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_drawing_tool_offers_create_until_its_shape_follows_the_form() {
        let c = |made| commit("draw_circle", Page::Design, true, made);
        assert_eq!(c(false), Some(Commit::Create));
        assert_eq!(c(true), None, "the shape already follows the form");
    }

    #[test]
    fn canvas_only_tools_and_live_forms_have_no_commit_button() {
        for id in GESTURE_TOOLS {
            for made in [false, true] {
                assert_eq!(commit(id, Page::Design, true, made), None, "{id}");
            }
        }
        for id in LIVE_FORMS {
            assert_eq!(commit(id, Page::Design, false, false), None, "{id}");
        }
    }

    #[test]
    fn a_remaking_form_creates_once_then_follows_its_settings() {
        for id in REMAKE_FORMS {
            assert_eq!(commit(id, Page::Design, false, false), Some(Commit::Create));
            assert_eq!(commit(id, Page::Design, false, true), None, "{id}");
        }
    }

    #[test]
    fn toolpaths_calculate_and_the_toolpath_pages_settings_apply() {
        for id in ["profile_toolpath", "pocket_toolpath", "merged_toolpath"] {
            assert_eq!(
                commit(id, Page::Toolpaths, false, false),
                Some(Commit::Calculate),
                "{id}"
            );
        }
        for id in ["material_setup", "keep_out_zones", "job_setup"] {
            assert_eq!(
                commit(id, Page::Toolpaths, false, false),
                Some(Commit::Apply),
                "{id}"
            );
        }
        // a toolpath command that only edits the list is no calculation
        assert_eq!(
            commit("delete_toolpath", Page::Design, false, false),
            Some(Commit::Apply)
        );
    }

    #[test]
    fn making_forms_create_and_changing_forms_apply() {
        for id in CREATE_FORMS {
            assert_eq!(
                commit(id, Page::Design, false, false),
                Some(Commit::Create),
                "{id}"
            );
        }
        for id in ["move", "offset_vectors", "layer_properties", "set_size"] {
            assert_eq!(
                commit(id, Page::Design, false, false),
                Some(Commit::Apply),
                "{id}"
            );
        }
    }

    #[test]
    fn every_listed_tool_is_one_the_app_has() {
        let registry = crate::commands::registry();
        for id in GESTURE_TOOLS.iter().chain(CREATE_FORMS) {
            assert!(registry.get(id).is_some(), "{id} is not a command");
        }
        for id in GESTURE_TOOLS {
            assert!(registry.get(id).unwrap().interactive(), "{id} is drawn");
        }
    }

    #[test]
    fn the_labels_and_hints_say_what_pressing_does() {
        assert_eq!(Commit::Calculate.label(), "Calculate");
        assert!(hint(Commit::Create, true, false).contains("drag on the canvas"));
        assert!(hint(Commit::Calculate, false, true).contains("again"));
        assert!(live_note("sculpt", true).contains("canvas"));
        assert!(live_note("draw_circle", true).contains("selected shape"));
    }
}
