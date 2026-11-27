//! A choice's picture and help: what the drop-downs show under and above them.

use super::*;

#[test]
fn a_groups_picture_shows_what_is_picked_and_a_choice_shown_for_it_draws_both() {
    let f = fields(&serde_json::json!({
        "type": "object",
        "properties": {
            "style": { "enum": ["plain", "fancy"], "x-widget": "drop-down",
                       "x-group": "Style", "x-diagram": "diagrams/style_{}.png" },
            "order": { "enum": ["round", "nearest"], "x-widget": "drop-down", "x-group": "Style",
                       "x-diagram": "diagrams/order_{}.png", "x-show-when": { "style": "fancy" } }
        }
    }));
    let asked = |style: &str| {
        let mut params = serde_json::json!({ "style": style, "order": "nearest" });
        let (_, mut asked) = drawn_with_pictures(&f, &mut params);
        asked.dedup();
        asked
    };
    assert_eq!(asked("plain"), ["diagrams/style_plain.png"]);
    assert_eq!(asked("fancy"), ["diagrams/order_nearest.png"]);
}

#[test]
fn a_drop_downs_help_wraps_as_one_paragraph() {
    assert_eq!(
        super::super::inputs::flowed(
            "Carve the whole shape: as deep as its width needs, so\nthe sides meet"
        ),
        "Carve the whole shape: as deep as its width needs, so the sides meet"
    );
    assert_eq!(super::super::inputs::flowed("One.\n\nTwo"), "One.\n\nTwo");
}
