use super::Drawn3d;
use crate::app::Handle3d;
use cut_craft_core::document::EntityId;
use egui::{Rect, pos2};

#[test]
fn nothing_is_drawn_until_the_view_is() {
    let drawn = Drawn3d::default();
    assert_eq!(drawn.fit(), None);
    assert_eq!(drawn.handles(), &[]);
    assert_eq!(drawn.under(), &[]);
}

#[test]
fn the_fit_is_the_last_one_drawn() {
    let mut drawn = Drawn3d::default();
    let first = Rect::from_min_max(pos2(0.0, 0.0), pos2(10.0, 10.0));
    let second = Rect::from_min_max(pos2(5.0, 5.0), pos2(20.0, 30.0));
    drawn.fitted(first);
    drawn.fitted(second);
    assert_eq!(drawn.fit(), Some(second));
}

#[test]
fn each_frame_draws_its_handles_afresh() {
    let mut drawn = Drawn3d::default();
    drawn.add_handle(Handle3d::Total, pos2(1.0, 2.0));
    drawn.add_handle(Handle3d::Base, pos2(3.0, 4.0));
    assert_eq!(drawn.handle_at(Handle3d::Base), Some(pos2(3.0, 4.0)));
    drawn.clear_handles();
    drawn.add_handle(Handle3d::Height, pos2(7.0, 8.0));
    assert_eq!(drawn.handles(), &[(Handle3d::Height, pos2(7.0, 8.0))]);
    assert_eq!(drawn.handle_at(Handle3d::Base), None);
}

#[test]
fn a_right_click_keeps_what_is_under_it_and_picks_the_topmost() {
    let mut drawn = Drawn3d::default();
    assert_eq!(
        drawn.right_clicked(vec![EntityId(4), EntityId(2)]),
        Some(EntityId(4))
    );
    assert_eq!(drawn.under(), &[EntityId(4), EntityId(2)]);
    assert_eq!(drawn.right_clicked(vec![]), None);
    assert_eq!(drawn.under(), &[]);
}
