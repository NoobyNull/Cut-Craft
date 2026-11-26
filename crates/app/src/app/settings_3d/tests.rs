use super::{Anchor, Settings3d};
use cmd_component_properties::Params;
use cut_craft_core::document::EntityId;
use cut_craft_core::geometry::Point;
use cut_craft_core::params::Xy;

fn popup() -> Settings3d {
    Settings3d::open(EntityId(1), Params::default(), egui::pos2(0.0, 0.0))
}

#[test]
fn nothing_is_applied_until_something_is_edited() {
    let mut s = popup();
    assert!(s.take_edits(false).is_none());
    s.changed(false);
    assert!(s.take_edits(false).is_none());
    s.changed(true);
    assert!(s.take_edits(false).is_some());
    assert!(s.take_edits(false).is_none(), "applied once");
}

#[test]
fn a_dragged_value_takes_back_its_last_step_first() {
    let mut s = popup();
    s.changed(true);
    assert_eq!(s.take_edits(true).map(|(_, back)| back), Some(None));
    s.applied(true, true, 7);
    s.changed(true);
    assert_eq!(s.take_edits(true).map(|(_, back)| back), Some(Some(7)));
}

#[test]
fn letting_go_ends_the_drag() {
    let mut s = popup();
    s.changed(true);
    s.take_edits(true);
    s.applied(true, true, 7);
    s.changed(true);
    assert_eq!(s.take_edits(false).map(|(_, back)| back), Some(None));
}

#[test]
fn a_click_or_a_step_not_made_is_not_taken_back() {
    let mut s = popup();
    s.changed(true);
    s.take_edits(false);
    s.applied(false, true, 3);
    s.changed(true);
    assert_eq!(s.take_edits(true).map(|(_, back)| back), Some(None));
    s.applied(true, false, 4);
    s.changed(true);
    assert_eq!(s.take_edits(true).map(|(_, back)| back), Some(None));
}

#[test]
fn two_clicks_set_the_fade_between_them() {
    let mut s = popup();
    s.anchors = Some((Anchor::Fade, None));
    s.place_anchor(Anchor::Fade, None, Point::new(1.0, 2.0));
    assert_eq!(s.anchors, Some((Anchor::Fade, Some(Point::new(1.0, 2.0)))));
    assert!(
        s.take_edits(false).is_none(),
        "nothing changes at the first click"
    );
    s.place_anchor(
        Anchor::Fade,
        Some(Point::new(1.0, 2.0)),
        Point::new(5.0, 6.0),
    );
    assert_eq!(s.anchors, None);
    let (p, _) = s.take_edits(false).expect("the fade is applied");
    assert!(p.fade.on);
    assert_eq!(
        (p.fade.from, p.fade.to),
        (Xy::new(1.0, 2.0), Xy::new(5.0, 6.0))
    );
}

#[test]
fn two_clicks_set_the_tilt_from_its_pivot() {
    let mut s = popup();
    s.place_anchor(
        Anchor::Tilt,
        Some(Point::new(0.0, 0.0)),
        Point::new(3.0, 0.0),
    );
    let (p, _) = s.take_edits(false).expect("the tilt is applied");
    assert!(p.tilt.on);
    assert_eq!(
        (p.tilt.pivot, p.tilt.toward),
        (Xy::new(0.0, 0.0), Xy::new(3.0, 0.0))
    );
}
