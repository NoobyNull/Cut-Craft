//! Toolbars of your own: made from any tool's or group's right-click menu, shown in every
//! layout, opened, floated and renamed.

use super::*;
use crate::bars::{Floating, Place, Side};
use crate::custom_toolbars::Item;
use crate::options::{CanvasFlyouts, NewBars, SnappedBars, WindowLayout};

/// Clicks the tab named `text` in the row of tabs holding the toolbar's (the menu bar
/// has a Model too).
fn click_tab(h: &mut Harness, text: &str) {
    let row = h.text_at("My Tools").expect("the toolbar's tab").y;
    let at = h
        .texts_at(text)
        .into_iter()
        .find(|at| (at.y - row).abs() < 1.0)
        .unwrap_or_else(|| panic!("no {text} tab"));
    h.click(at, Modifiers::NONE);
}

fn click_text(h: &mut Harness, text: &str) {
    let at = h
        .text_at(text)
        .unwrap_or_else(|| panic!("{text} not shown"));
    h.click(at, Modifiers::NONE);
}

/// A job in `layout` with a toolbar made from Draw Circle's right-click menu.
fn with_toolbar(layout: WindowLayout) -> Harness {
    let mut h = Harness::new();
    h.given_options(|o| {
        o.general.layout = layout;
        o.general.new_bars = NewBars::InTheLayout;
    });
    h.choose(|app| app.row_menu("draw_circle"), &["Add to New Toolbar"]);
    h.frame(vec![]);
    h.frame(vec![]);
    h
}

#[test]
fn a_tools_menu_makes_a_toolbar_then_adds_to_it_by_name() {
    let mut h = with_toolbar(WindowLayout::Ribbon);
    let toolbars = &h.options().toolbars;
    assert_eq!(toolbars.len(), 1);
    assert_eq!(toolbars[0].name, "My Tools");
    assert_eq!(toolbars[0].items, [Item::Tool("draw_circle".into())]);
    let menu = h.row_menu("draw_circle");
    assert!(
        menu.iter().any(|i| i.name == "Remove from My Tools"),
        "on it already"
    );
    // another tool goes on the same toolbar from its menu
    h.choose(|app| app.row_menu("draw_rectangle"), &["Add to My Tools"]);
    assert_eq!(h.options().toolbars[0].items.len(), 2);
    assert_eq!(h.options().toolbars.len(), 1);
}

#[test]
fn the_ribbon_shows_a_toolbar_as_a_tab_after_its_own() {
    let mut h = with_toolbar(WindowLayout::Ribbon);
    // the new toolbar's tab is open at once
    assert_eq!(h.ribbon_toolbar(), Some(0));
    let tab = h.text_at("My Tools").expect("its tab");
    let favourites = h.text_at("Favourites").expect("the built-in tabs");
    assert!(tab.x > favourites.x, "after the built-in tabs");
    click_tab(&mut h, "Model");
    assert_eq!(h.ribbon_toolbar(), None);
    assert_eq!(h.ribbon_tab(), "Model");
    click_tab(&mut h, "My Tools");
    assert_eq!(h.ribbon_toolbar(), Some(0));
}

#[test]
fn the_rail_lists_a_toolbar_and_a_pick_runs_the_tool() {
    let mut h = with_toolbar(WindowLayout::Rail);
    assert!(h.text_at("Draw Circle").is_none(), "no list open at first");
    h.show_rail_toolbar(0);
    h.frame(vec![]);
    h.frame(vec![]);
    assert!(h.text_at("My Tools").is_some(), "the list is named");
    click_text(&mut h, "Draw Circle");
    assert_eq!(h.form_id(), Some("draw_circle"));
    assert_eq!(h.rail_toolbar(), None, "the list closes on a pick");
}

#[test]
fn a_group_on_a_toolbar_opens_its_tools() {
    let mut h = Harness::new();
    h.given_options(|o| {
        o.general.layout = WindowLayout::Rail;
        o.general.canvas_flyouts = CanvasFlyouts::OnClick;
        o.general.new_bars = NewBars::InTheLayout;
    });
    h.invoke("toolbar:new:group:Create Vectors");
    h.show_rail_toolbar(0);
    h.frame(vec![]);
    h.frame(vec![]);
    assert!(h.text_at("Draw Circle").is_none(), "shut at first");
    click_text(&mut h, "Create Vectors ▸");
    assert_eq!(h.toolbar_group_open(), Some((0, 0)));
    h.frame(vec![]);
    click_text(&mut h, "Draw Circle");
    assert_eq!(h.form_id(), Some("draw_circle"));
    assert_eq!(h.toolbar_group_open(), None);
}

#[test]
fn classic_shows_a_toolbar_as_a_section_of_the_design_tab() {
    let mut h = with_toolbar(WindowLayout::Classic);
    // after the Design tab's own tools, so none of them moves for it
    let first = h.text_at("Create Vectors").expect("the tab's tools");
    let section = h.scroll_to(first, "My Tools").expect("its section");
    let now = h.text_at("Create Vectors");
    assert!(now.is_none_or(|now| now.y < section.y), "below the rest");
}

#[test]
fn the_canvas_header_has_its_tab_too() {
    let mut h = with_toolbar(WindowLayout::Canvas);
    assert!(h.text_at("My Tools").is_some(), "in the header");
    click_tab(&mut h, "Design");
    assert_eq!(h.ribbon_toolbar(), None);
    click_tab(&mut h, "My Tools");
    assert_eq!(h.ribbon_toolbar(), Some(0));
    assert!(h.open_canvas_group().is_none());
}

/// Where a bar's grip is: the dots at its start, inside its frame.
fn grip(bar: egui::Rect) -> egui::Pos2 {
    egui::pos2(bar.left() + 7.0, bar.center().y)
}

fn floating(h: &Harness, n: usize) -> Floating {
    match h.options().toolbars[n].place {
        Place::Floating(floating) => floating,
        place => panic!("not floating: {place:?}"),
    }
}

#[test]
fn a_new_toolbar_floats_over_the_drawing_with_no_tab() {
    let mut h = Harness::new();
    h.given_options(|o| o.general.layout = WindowLayout::Ribbon);
    h.choose(|app| app.row_menu("draw_circle"), &["Add to New Toolbar"]);
    h.frame(vec![]);
    h.frame(vec![]);
    assert!(matches!(h.options().toolbars[0].place, Place::Floating(_)));
    assert_eq!(h.ribbon_toolbar(), None, "no tab for it");
    assert!(h.text_at("My Tools").is_none(), "its grip has no words");
    assert!(h.toolbar_bar_rect(0).is_some(), "it is over the drawing");
}

#[test]
fn a_floating_toolbar_goes_where_its_grip_is_dragged_and_stays_when_locked() {
    let mut h = with_toolbar(WindowLayout::Ribbon);
    h.invoke("toolbar:place:0:float");
    h.frame(vec![]);
    h.frame(vec![]);
    assert_eq!(h.ribbon_toolbar(), None, "no tab for it now");
    let bar = h.toolbar_bar_rect(0).expect("over the drawing");
    h.drag(grip(bar), grip(bar) + egui::vec2(150.0, 120.0));
    h.frame(vec![]);
    let moved = h.toolbar_bar_rect(0).expect("still there");
    let by = moved.left_top() - bar.left_top();
    assert!(
        (by.x - 150.0).abs() < 2.0 && (by.y - 120.0).abs() < 2.0,
        "followed the pointer: {by:?}"
    );
    assert!(floating(&h, 0).offset.is_some(), "kept where it was left");
    // a drag on a tool does not move the bar
    let circle = moved.right_center() - egui::vec2(20.0, 0.0);
    h.drag(circle, circle + egui::vec2(100.0, 100.0));
    h.frame(vec![]);
    assert_eq!(h.toolbar_bar_rect(0), Some(moved), "a tool is not a handle");
    // locked, its grip leaves it where it is
    h.invoke("toolbar:place:0:lock");
    h.frame(vec![]);
    h.drag(grip(moved), grip(moved) + egui::vec2(-100.0, 50.0));
    h.frame(vec![]);
    assert_eq!(h.toolbar_bar_rect(0), Some(moved), "locked");
    h.invoke("toolbar:place:0:attach");
    h.frame(vec![]);
    h.frame(vec![]);
    assert!(h.text_at("My Tools").is_some(), "its tab is back");
    assert_eq!(h.options().toolbars[0].place, Place::Layout);
}

#[test]
fn a_turned_toolbar_runs_down() {
    let mut h = with_toolbar(WindowLayout::Ribbon);
    h.invoke("toolbar:add:0:tool:draw_rectangle");
    h.invoke("toolbar:place:0:float");
    h.frame(vec![]);
    h.frame(vec![]);
    let across = h.toolbar_bar_rect(0).expect("floating");
    assert!(across.width() > across.height());
    h.invoke("toolbar:place:0:turn");
    h.frame(vec![]);
    h.frame(vec![]);
    let down = h.toolbar_bar_rect(0).expect("floating");
    assert!(down.height() > down.width(), "{down:?}");
}

#[test]
fn a_snapped_toolbar_pushes_the_drawing_over_or_lies_over_its_edge() {
    let mut h = with_toolbar(WindowLayout::Rail);
    h.frame(vec![]);
    let before = h.view_rect();
    h.invoke("toolbar:place:0:snap:right");
    h.frame(vec![]);
    h.frame(vec![]);
    let pushed = h.view_rect();
    assert!(
        pushed.right() < before.right() - 10.0,
        "the drawing made room"
    );
    assert!(
        h.toolbar_bar_rect(0).is_none(),
        "its own strip, not over the drawing"
    );
    assert_eq!(
        h.options().toolbars[0].place,
        Place::Snapped { side: Side::Right }
    );
    // Over the Drawing: nothing changes size, the bar lies along the drawing's right edge
    h.given_options(|o| o.general.snapped_bars = SnappedBars::OverTheDrawing);
    h.frame(vec![]);
    h.frame(vec![]);
    assert_eq!(h.view_rect(), before, "the drawing keeps its size");
    let bar = h.toolbar_bar_rect(0).expect("over the drawing");
    assert!(
        bar.left() < before.right() && bar.right() >= before.right() - 2.0,
        "along the drawing's right edge: {bar:?} {before:?}"
    );
}

#[test]
fn bars_lying_over_the_drawing_stay_under_its_controls_and_off_each_other() {
    let mut h = with_toolbar(WindowLayout::Rail);
    h.choose(
        |app| app.row_menu("draw_rectangle"),
        &["Add to New Toolbar"],
    );
    h.given_options(|o| o.general.snapped_bars = SnappedBars::OverTheDrawing);
    h.invoke("toolbar:place:0:snap:top");
    h.invoke("toolbar:place:1:snap:left");
    h.frame(vec![]);
    h.frame(vec![]);
    h.frame(vec![]);
    let view = h.view_rect();
    let top = h.toolbar_bar_rect(0).expect("over the drawing");
    let left = h.toolbar_bar_rect(1).expect("over the drawing");
    assert!(
        top.top() >= view.top() - 1.0,
        "under the view's own bar: {top:?} {view:?}"
    );
    assert!(!top.shrink(0.5).intersects(left), "{top:?} {left:?}");
}

#[test]
fn old_floating_toolbars_still_float_where_they_were() {
    let mut h = Harness::new();
    let old =
        r#"[{"name":"Signs","items":[{"tool":"draw_circle"}],"floating":true,"at":[300.0,200.0]}]"#;
    let toolbars: Vec<crate::custom_toolbars::Toolbar> = serde_json::from_str(old).unwrap();
    h.given_options(|o| o.toolbars = toolbars);
    h.frame(vec![]);
    h.frame(vec![]);
    let bar = h.toolbar_bar_rect(0).expect("floating");
    assert_eq!(bar.left_top(), egui::pos2(300.0, 200.0));
}

#[test]
fn rename_asks_for_the_name_and_delete_takes_it_away() {
    let mut h = with_toolbar(WindowLayout::Ribbon);
    h.choose(
        |app| crate::custom_toolbars::toolbar_menu(&app.options.toolbars, 0, app.bar_home()),
        &["Rename Toolbar"],
    );
    h.frame(vec![]);
    h.frame(vec![]);
    assert!(h.text_at("Rename Toolbar").is_some(), "the box asks");
    h.frame(vec![Event::Text(" Shop".into())]);
    h.frame(vec![Event::Key {
        key: Key::Enter,
        physical_key: None,
        pressed: true,
        repeat: false,
        modifiers: Modifiers::NONE,
    }]);
    h.frame(vec![]);
    assert_eq!(h.options().toolbars[0].name, "My Tools Shop");
    assert!(!h.renaming_toolbar(), "the box closed");
    h.choose(
        |app| crate::custom_toolbars::toolbar_menu(&app.options.toolbars, 0, app.bar_home()),
        &["Delete Toolbar"],
    );
    h.frame(vec![]);
    assert_eq!(h.options().toolbars.len(), 0);
    assert_eq!(h.ribbon_toolbar(), None);
    assert!(h.text_at("My Tools Shop").is_none());
}

#[test]
fn a_group_heading_offers_the_whole_group() {
    let mut h = Harness::new();
    h.given_options(|o| o.general.layout = WindowLayout::Classic);
    h.frame(vec![]);
    let heading = h.text_at("Create Vectors").expect("the group's heading");
    h.right_click(heading);
    assert!(h.text_at("Add to New Toolbar").is_some(), "its menu");
}

#[test]
fn a_drag_moves_a_tool_along_its_toolbar() {
    let mut h = with_toolbar(WindowLayout::Rail);
    h.invoke("toolbar:add:0:tool:draw_rectangle");
    h.show_rail_toolbar(0);
    h.frame(vec![]);
    h.frame(vec![]);
    let line = h.text_at("Draw Rectangle").expect("listed");
    let circle = h.text_at("Draw Circle").expect("listed");
    h.drag(line, circle);
    let items = &h.options().toolbars[0].items;
    assert_eq!(
        items,
        &[
            Item::Tool("draw_rectangle".into()),
            Item::Tool("draw_circle".into())
        ]
    );
    assert!(h.form_id().is_none(), "a drag runs nothing");
}

#[test]
fn a_ribbon_group_name_offers_the_group_on_a_right_click() {
    let mut h = Harness::new();
    h.given_options(|o| o.general.layout = WindowLayout::Ribbon);
    h.frame(vec![]);
    h.frame(vec![]);
    let name = h.text_at("Create Vectors ▾").expect("the group's name");
    h.right_click(name);
    assert!(h.text_at("Add to New Toolbar").is_some(), "its menu");
    assert!(h.text_at("Draw Circle").is_none(), "not its list");
}

#[test]
fn deleting_a_toolbar_from_its_tab_leaves_the_others() {
    let mut h = with_toolbar(WindowLayout::Ribbon);
    h.invoke("toolbar:new:tool:draw_rectangle");
    h.frame(vec![]);
    h.frame(vec![]);
    let tab = h.text_at("My Tools").expect("the first toolbar's tab");
    h.right_click(tab);
    click_text(&mut h, "Delete Toolbar");
    h.frame(vec![]);
    let names: Vec<&str> = (h.options().toolbars)
        .iter()
        .map(|t| t.name.as_str())
        .collect();
    assert_eq!(names, ["My Tools 2"]);
    assert!(h.text_at("My Tools 2").is_some(), "its tab is still there");
}

#[test]
fn a_tool_dragged_later_lands_before_the_one_it_is_dropped_on() {
    let mut h = with_toolbar(WindowLayout::Rail);
    for id in ["draw_rectangle", "draw_ellipse"] {
        h.invoke(&format!("toolbar:add:0:tool:{id}"));
    }
    h.show_rail_toolbar(0);
    h.frame(vec![]);
    h.frame(vec![]);
    let circle = h.text_at("Draw Circle").expect("listed");
    let ellipse = h.text_at("Draw Ellipse").expect("listed");
    h.drag(circle, ellipse);
    let ids: Vec<String> = h.options().toolbars[0]
        .items
        .iter()
        .map(Item::key)
        .collect();
    assert_eq!(
        ids,
        [
            "tool:draw_rectangle",
            "tool:draw_circle",
            "tool:draw_ellipse"
        ]
    );
}

#[test]
fn applying_options_keeps_a_toolbar_made_while_they_were_open() {
    let mut h = Harness::new();
    let draft = h.options().clone();
    h.invoke("toolbar:new:tool:draw_circle");
    h.options_changed(draft);
    assert_eq!(h.options().toolbars.len(), 1);
}
