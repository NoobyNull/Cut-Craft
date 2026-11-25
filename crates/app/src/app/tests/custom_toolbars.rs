//! Toolbars of your own: made from any tool's or group's right-click menu, shown in every
//! layout, opened, floated and renamed.

use super::*;
use crate::custom_toolbars::Item;
use crate::options::{CanvasFlyouts, WindowLayout};

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
    h.app.options.general.layout = layout;
    h.choose(
        |app| app.row_menu("draw_circle"),
        &["Add to Toolbar", "New Toolbar"],
    );
    h.frame(vec![]);
    h.frame(vec![]);
    h
}

#[test]
fn a_tools_menu_makes_a_toolbar_and_then_ticks_it() {
    let mut h = with_toolbar(WindowLayout::Ribbon);
    let toolbars = &h.app.options.toolbars;
    assert_eq!(toolbars.len(), 1);
    assert_eq!(toolbars[0].name, "My Tools");
    assert_eq!(toolbars[0].items, [Item::Tool("draw_circle".into())]);
    let menu = h.app.row_menu("draw_circle");
    let on = leaf(&menu, &["Add to Toolbar", "My Tools"]);
    assert!(on.is_checked && !on.is_enabled, "on it already");
    // another tool goes on the same toolbar from its menu
    h.choose(
        |app| app.row_menu("draw_rectangle"),
        &["Add to Toolbar", "My Tools"],
    );
    assert_eq!(h.app.options.toolbars[0].items.len(), 2);
    assert_eq!(h.app.options.toolbars.len(), 1);
}

#[test]
fn the_ribbon_shows_a_toolbar_as_a_tab_after_its_own() {
    let mut h = with_toolbar(WindowLayout::Ribbon);
    // the new toolbar's tab is open at once
    assert_eq!(h.app.toolbars.ribbon, Some(0));
    let tab = h.text_at("My Tools").expect("its tab");
    let favourites = h.text_at("Favourites").expect("the built-in tabs");
    assert!(tab.x > favourites.x, "after the built-in tabs");
    click_tab(&mut h, "Model");
    assert_eq!(h.app.toolbars.ribbon, None);
    assert_eq!(RIBBON_TABS[h.app.ribbon_tab], "Model");
    click_tab(&mut h, "My Tools");
    assert_eq!(h.app.toolbars.ribbon, Some(0));
}

#[test]
fn the_rail_lists_a_toolbar_and_a_pick_runs_the_tool() {
    let mut h = with_toolbar(WindowLayout::Rail);
    assert!(h.text_at("Draw Circle").is_none(), "no list open at first");
    h.app.toolbars.rail = Some(0);
    h.frame(vec![]);
    h.frame(vec![]);
    assert!(h.text_at("My Tools").is_some(), "the list is named");
    click_text(&mut h, "Draw Circle");
    assert_eq!(h.app.form.as_ref().map(|f| f.id), Some("draw_circle"));
    assert_eq!(h.app.toolbars.rail, None, "the list closes on a pick");
}

#[test]
fn a_group_on_a_toolbar_opens_its_tools() {
    let mut h = Harness::new();
    h.app.options.general.layout = WindowLayout::Rail;
    h.app.options.general.canvas_flyouts = CanvasFlyouts::OnClick;
    h.app.invoke(&h.ctx, "toolbar:new:group:Create Vectors");
    h.app.toolbars.rail = Some(0);
    h.frame(vec![]);
    h.frame(vec![]);
    assert!(h.text_at("Draw Circle").is_none(), "shut at first");
    click_text(&mut h, "Create Vectors ▸");
    assert_eq!(h.app.toolbars.group, Some((0, 0)));
    h.frame(vec![]);
    click_text(&mut h, "Draw Circle");
    assert_eq!(h.app.form.as_ref().map(|f| f.id), Some("draw_circle"));
    assert_eq!(h.app.toolbars.group, None);
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
    assert_eq!(h.app.toolbars.ribbon, None);
    click_tab(&mut h, "My Tools");
    assert_eq!(h.app.toolbars.ribbon, Some(0));
    assert!(h.app.canvas.group.is_none());
}

#[test]
fn a_floating_toolbar_leaves_its_tab_and_stays_where_it_is_dragged() {
    let mut h = with_toolbar(WindowLayout::Ribbon);
    h.app.invoke(&h.ctx, "toolbar:float:0:true");
    h.frame(vec![]);
    h.frame(vec![]);
    assert_eq!(h.app.toolbars.ribbon, None, "no tab for it now");
    let name = h.text_at("My Tools").expect("its name over the drawing");
    let [x, y] = h.app.options.toolbars[0].at.expect("where it is kept");
    h.drag(name, name + egui::vec2(-150.0, 120.0));
    h.frame(vec![]);
    let [x2, y2] = h.app.options.toolbars[0].at.expect("where it was left");
    assert!(
        (x2 - x + 150.0).abs() < 2.0 && (y2 - y - 120.0).abs() < 2.0,
        "followed the pointer: {x},{y} to {x2},{y2}"
    );
    h.app.invoke(&h.ctx, "toolbar:float:0:false");
    h.frame(vec![]);
    h.frame(vec![]);
    assert!(h.text_at("My Tools").is_some(), "its tab is back");
    assert!(!h.app.options.toolbars[0].floating);
}

#[test]
fn rename_asks_for_the_name_and_delete_takes_it_away() {
    let mut h = with_toolbar(WindowLayout::Ribbon);
    h.choose(
        |app| crate::custom_toolbars::toolbar_menu(&app.options.toolbars, 0),
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
    assert_eq!(h.app.options.toolbars[0].name, "My Tools Shop");
    assert!(h.app.toolbars.rename.is_none(), "the box closed");
    h.choose(
        |app| crate::custom_toolbars::toolbar_menu(&app.options.toolbars, 0),
        &["Delete Toolbar"],
    );
    h.frame(vec![]);
    assert_eq!(h.app.options.toolbars.len(), 0);
    assert_eq!(h.app.toolbars.ribbon, None);
    assert!(h.text_at("My Tools Shop").is_none());
}

#[test]
fn a_group_heading_offers_the_whole_group() {
    let mut h = Harness::new();
    h.app.options.general.layout = WindowLayout::Classic;
    h.frame(vec![]);
    let heading = h.text_at("Create Vectors").expect("the group's heading");
    h.right_click(heading);
    assert!(h.text_at("Add to Toolbar").is_some(), "its menu");
}

#[test]
fn a_drag_moves_a_tool_along_its_toolbar() {
    let mut h = with_toolbar(WindowLayout::Rail);
    h.app.invoke(&h.ctx, "toolbar:add:0:tool:draw_rectangle");
    h.app.toolbars.rail = Some(0);
    h.frame(vec![]);
    h.frame(vec![]);
    let line = h.text_at("Draw Rectangle").expect("listed");
    let circle = h.text_at("Draw Circle").expect("listed");
    h.drag(line, circle);
    let items = &h.app.options.toolbars[0].items;
    assert_eq!(
        items,
        &[
            Item::Tool("draw_rectangle".into()),
            Item::Tool("draw_circle".into())
        ]
    );
    assert!(h.app.form.is_none(), "a drag runs nothing");
}

#[test]
fn a_ribbon_group_name_offers_the_group_on_a_right_click() {
    let mut h = Harness::new();
    h.app.options.general.layout = WindowLayout::Ribbon;
    h.frame(vec![]);
    h.frame(vec![]);
    let name = h.text_at("Create Vectors ▾").expect("the group's name");
    h.right_click(name);
    assert!(h.text_at("Add to Toolbar").is_some(), "its menu");
    assert!(h.text_at("Draw Circle").is_none(), "not its list");
}

#[test]
fn deleting_a_toolbar_from_its_tab_leaves_the_others() {
    let mut h = with_toolbar(WindowLayout::Ribbon);
    h.app.invoke(&h.ctx, "toolbar:new:tool:draw_rectangle");
    h.frame(vec![]);
    h.frame(vec![]);
    let tab = h.text_at("My Tools").expect("the first toolbar's tab");
    h.right_click(tab);
    click_text(&mut h, "Delete Toolbar");
    h.frame(vec![]);
    let names: Vec<&str> = h
        .app
        .options
        .toolbars
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
        h.app.invoke(&h.ctx, &format!("toolbar:add:0:tool:{id}"));
    }
    h.app.toolbars.rail = Some(0);
    h.frame(vec![]);
    h.frame(vec![]);
    let circle = h.text_at("Draw Circle").expect("listed");
    let ellipse = h.text_at("Draw Ellipse").expect("listed");
    h.drag(circle, ellipse);
    let ids: Vec<String> = h.app.options.toolbars[0]
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
    let draft = h.app.options.clone();
    h.app.invoke(&h.ctx, "toolbar:new:tool:draw_circle");
    h.app.options_changed(&h.ctx.clone(), draft);
    assert_eq!(h.app.options.toolbars.len(), 1);
}
