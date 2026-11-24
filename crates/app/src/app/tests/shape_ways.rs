//! A shape drawn more than one way: one button, the ways picked at the top of its form.

use super::*;
use crate::flyout::DragWay;

/// Whether `mark` (`tool:<id>`, `way:<id>`) is drawn this frame, and where.
fn drawn(h: &mut Harness, mark: &str) -> Option<Pos2> {
    crate::marks::forget(&h.ctx, mark);
    h.frame(vec![]);
    h.frame(vec![]);
    crate::marks::get(&h.ctx, mark).map(|r| r.center())
}

fn noting() -> Harness {
    let mut h = Harness::new();
    crate::marks::switch_on(&h.ctx);
    h.app.pages.set_pinned(Page::Design, true);
    h
}

fn way(h: &Harness) -> Option<(&'static str, DragWay)> {
    h.app.form.as_ref().map(|f| (f.id, f.way))
}

#[test]
fn a_shape_has_one_button_and_its_form_picks_the_way() {
    let mut h = noting();
    assert!(drawn(&mut h, "tool:draw_rectangle").is_some());
    assert!(drawn(&mut h, "tool:draw_circle").is_some());
    assert!(drawn(&mut h, "tool:draw_ellipse").is_some(), "its own tool");
    assert!(drawn(&mut h, "tool:draw_rectangle_from_center").is_none());
    assert!(drawn(&mut h, "tool:draw_circle_from_edge").is_none());
    // the button opens the usual way, and the form offers both under Draw from
    h.app.invoke(&h.ctx, "draw_rectangle");
    assert_eq!(way(&h), Some(("draw_rectangle", DragWay::AsIs)));
    assert!(drawn(&mut h, "way:draw_rectangle").is_some());
    let center = drawn(&mut h, "way:draw_rectangle_from_center").expect("Center offered");
    h.click(center, Modifiers::NONE);
    assert_eq!(way(&h), Some(("draw_rectangle", DragWay::FromCenter)));
    assert_eq!(h.app.status.current_tool, "Draw Rectangle from Center");
    assert_eq!(
        h.app.form.as_ref().map(|f| f.label),
        Some("Draw Rectangle from Center")
    );
    // its button shows in hand, though the way has none of its own
    assert!(h.app.tool_in_hand("draw_rectangle"));
    // drawn from the middle out
    h.drag(h.at(50.0, 50.0), h.at(70.0, 60.0));
    let b = h.app.doc.vectors.values().last().expect("drawn").bounds();
    assert!(
        (b.min.x - 30.0).abs() < 0.5 && (b.max.y - 60.0).abs() < 0.5,
        "{b:?}"
    );
    // the button opens the way picked last, kept in the options
    assert_eq!(
        h.app
            .options
            .shape_ways
            .get("draw_rectangle")
            .map(String::as_str),
        Some("draw_rectangle_from_center")
    );
    h.app.form = None;
    h.frame(vec![]);
    h.app.invoke(&h.ctx, "draw_rectangle");
    assert_eq!(way(&h), Some(("draw_rectangle", DragWay::FromCenter)));
    // and back to the corner
    let corner = drawn(&mut h, "way:draw_rectangle").expect("Corner offered");
    h.click(corner, Modifiers::NONE);
    assert_eq!(way(&h), Some(("draw_rectangle", DragWay::AsIs)));
    assert_eq!(h.app.status.current_tool, "Draw Rectangle");
    assert_eq!(
        h.app
            .options
            .shape_ways
            .get("draw_rectangle")
            .map(String::as_str),
        Some("draw_rectangle")
    );
}

#[test]
fn the_circle_picks_center_or_edge_in_its_form() {
    let mut h = noting();
    h.app.invoke(&h.ctx, "draw_circle");
    let edge = drawn(&mut h, "way:draw_circle_from_edge").expect("Edge offered");
    h.click(edge, Modifiers::NONE);
    assert_eq!(way(&h), Some(("draw_circle", DragWay::AcrossEdge)));
    // the form's Next line and the status bar say how this way drags
    let next = h.app.form_guidance().expect("a next step");
    assert!(
        next.starts_with("Drag from one side straight across"),
        "{next}"
    );
    let status = h.app.next_step().expect("a status line");
    assert!(status.contains("from one side straight across"), "{status}");
    h.drag(h.at(20.0, 50.0), h.at(60.0, 50.0));
    let b = h.app.doc.vectors.values().last().expect("drawn").bounds();
    assert!(
        (b.min.x - 20.0).abs() < 0.5 && (b.max.x - 60.0).abs() < 0.5,
        "{b:?}"
    );
    // a form with one way has no Draw from
    h.app.invoke(&h.ctx, "draw_ellipse");
    assert!(drawn(&mut h, "way:draw_circle").is_none());
}

#[test]
fn each_way_gets_its_button_back_when_asked() {
    let mut h = noting();
    h.app.options.general.other_ways = crate::options::OtherWays::AsButtons;
    assert!(drawn(&mut h, "tool:draw_rectangle_from_center").is_some());
    assert!(drawn(&mut h, "tool:draw_circle_from_edge").is_some());
    // Draw Rectangle's own button is the corner way, whatever was drawn last
    h.app
        .options
        .shape_ways
        .insert("draw_rectangle".into(), "draw_rectangle_from_center".into());
    h.app.invoke(&h.ctx, "draw_rectangle");
    assert_eq!(way(&h), Some(("draw_rectangle", DragWay::AsIs)));
    assert!(h.app.ways_note("draw_rectangle").is_none());
    // the way's own button lights, not the shape's
    h.app.invoke(&h.ctx, "draw_rectangle_from_center");
    assert!(!h.app.tool_in_hand("draw_rectangle"));
}

#[test]
fn the_shapes_tooltip_says_where_its_other_ways_are() {
    let h = Harness::new();
    let note = h.app.ways_note("draw_rectangle").expect("a note");
    assert!(note.contains("Corner / Center"), "{note}");
    let note = h.app.ways_note("draw_circle").expect("a note");
    assert!(note.contains("Center / Edge"), "{note}");
    assert!(h.app.ways_note("draw_ellipse").is_none());
}

#[test]
fn find_tool_still_finds_every_way() {
    let h = Harness::new();
    let ids: Vec<_> = h
        .app
        .search_matches("from")
        .into_iter()
        .map(|(id, _)| id)
        .collect();
    assert!(ids.contains(&"draw_rectangle_from_center"), "{ids:?}");
    assert!(ids.contains(&"draw_circle_from_edge"), "{ids:?}");
}

#[test]
fn the_ribbon_shows_one_button_per_shape_even_with_every_tool() {
    use crate::options::{RibbonIcons, WindowLayout};
    let mut h = noting();
    h.app.options.general.layout = WindowLayout::Ribbon;
    h.app.options.general.ribbon_icons = RibbonIcons::EveryTool;
    h.app.ribbon_tab = 0;
    assert!(drawn(&mut h, "tool:draw_rectangle").is_some());
    assert!(drawn(&mut h, "tool:draw_ellipse").is_some());
    assert!(drawn(&mut h, "tool:draw_rectangle_from_center").is_none());
    // pinned by hand before, it still waits for the setting
    h.app
        .options
        .ribbon_pins
        .insert("draw_circle_from_edge".into(), true);
    assert!(drawn(&mut h, "tool:draw_circle_from_edge").is_none());
    h.app.options.general.other_ways = crate::options::OtherWays::AsButtons;
    assert!(drawn(&mut h, "tool:draw_rectangle_from_center").is_some());
    assert!(drawn(&mut h, "tool:draw_circle_from_edge").is_some());
}
