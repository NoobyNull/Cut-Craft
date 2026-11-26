use super::HandleDrag;
use crate::app::Handle3d;
use cmd_component_properties::Params;
use egui::pos2;

fn begun() -> (Params, f64) {
    (Params::default(), 12.0)
}

#[test]
fn a_drag_holds_its_handle_until_let_go() {
    let mut drag = HandleDrag::default();
    assert!(!drag.is_held());
    drag.start(Handle3d::Height, pos2(5.0, 6.0));
    assert_eq!(drag.held(), Some((Handle3d::Height, pos2(5.0, 6.0))));
    drag.let_go();
    assert!(!drag.is_held());
}

#[test]
fn a_height_drag_keeps_what_it_began_from() {
    let mut drag = HandleDrag::default();
    drag.start(Handle3d::Total, pos2(0.0, 0.0));
    assert!(!drag.is_setting_height());
    let (_, top, applied) = drag.height(begun);
    assert_eq!((top, applied), (12.0, None));
    drag.applied(Some(serde_json::json!({"shape_height": 150.0})), 1.0);
    // later frames keep the start, not take it again
    let (_, top, applied) = drag.height(|| panic!("taken once"));
    assert_eq!(top, 12.0);
    assert_eq!(applied, Some(&serde_json::json!({"shape_height": 150.0})));
    assert!(drag.is_setting_height());
}

#[test]
fn the_model_is_remade_by_the_time_since_it_last_was() {
    let mut drag = HandleDrag::default();
    assert_eq!(drag.since_applied(3.0), f64::INFINITY, "not yet");
    drag.height(begun);
    drag.applied(None, 3.0);
    assert!((drag.since_applied(3.25) - 0.25).abs() < 1e-9);
}

#[test]
fn escape_says_whether_a_height_was_applied_to_undo() {
    let mut drag = HandleDrag::default();
    assert_eq!(drag.cancel(), None, "nothing held");
    drag.start(Handle3d::Base, pos2(0.0, 0.0));
    assert_eq!(drag.cancel(), Some(false), "nothing applied yet");
    drag.start(Handle3d::Base, pos2(0.0, 0.0));
    drag.height(begun);
    drag.applied(Some(serde_json::json!({})), 1.0);
    assert_eq!(drag.cancel(), Some(true));
    assert!(!drag.is_held() && !drag.is_setting_height());
    assert_eq!(drag.since_applied(2.0), f64::INFINITY, "forgotten");
}

#[test]
fn an_ended_drag_keeps_nothing() {
    let mut drag = HandleDrag::default();
    drag.start(Handle3d::Height, pos2(0.0, 0.0));
    drag.height(begun);
    drag.applied(None, 1.0);
    drag.end();
    assert!(!drag.is_held() && !drag.is_setting_height());
    assert_eq!(drag.since_applied(2.0), f64::INFINITY);
}
