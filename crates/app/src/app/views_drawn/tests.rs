use super::ViewsDrawn;
use crate::view2d::ViewTransform;
use egui::{Rect, pos2, vec2};

fn rect(x: f32, y: f32, w: f32, h: f32) -> Rect {
    Rect::from_min_size(pos2(x, y), vec2(w, h))
}

fn transform() -> ViewTransform {
    ViewTransform {
        origin: pos2(10.0, 20.0),
        scale: 2.0,
    }
}

#[test]
fn nothing_is_drawn_at_first() {
    let drawn = ViewsDrawn::default();
    assert!(drawn.transform().is_none());
    assert_eq!((drawn.area_2d(), drawn.area_3d()), (None, None));
    assert!(!drawn.over_a_view(pos2(1.0, 1.0)));
}

#[test]
fn the_2d_view_keeps_its_mapping_until_it_is_not_shown() {
    let mut drawn = ViewsDrawn::default();
    drawn.drawn_2d(transform(), rect(0.0, 0.0, 100.0, 50.0));
    assert_eq!(
        drawn.transform().map(|t| (t.origin, t.scale)),
        Some((pos2(10.0, 20.0), 2.0))
    );
    drawn.forget_2d();
    assert!(drawn.transform().is_none());
    assert_eq!(
        drawn.area_2d(),
        Some(rect(0.0, 0.0, 100.0, 50.0)),
        "where it was is kept"
    );
}

#[test]
fn the_pointer_is_over_either_view() {
    let mut drawn = ViewsDrawn::default();
    drawn.drawn_2d(transform(), rect(0.0, 0.0, 100.0, 50.0));
    drawn.drawn_3d(rect(100.0, 0.0, 100.0, 50.0));
    assert!(drawn.over_a_view(pos2(50.0, 25.0)));
    assert!(drawn.over_a_view(pos2(150.0, 25.0)));
    assert!(!drawn.over_a_view(pos2(150.0, 75.0)));
}

#[test]
fn the_2d_view_moving_on_screen_says_how_far() {
    let mut drawn = ViewsDrawn::default();
    assert_eq!(
        drawn.moved_2d(pos2(5.0, 5.0)),
        None,
        "nothing to move from yet"
    );
    assert_eq!(drawn.moved_2d(pos2(8.0, 1.0)), Some(vec2(3.0, -4.0)));
    assert_eq!(drawn.moved_2d(pos2(8.0, 1.0)), Some(vec2(0.0, 0.0)));
    assert_eq!(drawn.at_2d(), Some(pos2(8.0, 1.0)));
}
