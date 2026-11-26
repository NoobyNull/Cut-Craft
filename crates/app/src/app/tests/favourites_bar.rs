//! Favourites as the first bar: floating over the drawing at first in every layout, holding
//! favourite tools and whole groups, renamed but never deleted, attached back to the layout.

use super::*;
use crate::bars::Place;
use crate::custom_toolbars::Item;
use crate::options::WindowLayout;

/// A job in `layout` with Favourites where a new install has it: floating.
fn floating_favourites(layout: WindowLayout) -> Harness {
    let mut h = Harness::new();
    h.given_options(|o| {
        o.general.layout = layout;
        o.favourites.bar = Place::default();
    });
    h.frame(vec![]);
    h.frame(vec![]);
    h
}

#[test]
fn favourites_float_over_the_drawing_in_every_layout_with_no_tab() {
    for layout in WindowLayout::ALL {
        let mut h = floating_favourites(layout);
        h.choose(|app| app.row_menu("draw_circle"), &["Add to Favourites"]);
        h.frame(vec![]);
        h.frame(vec![]);
        assert!(h.favourites_bar_rect().is_some(), "{layout:?}: a bar");
        assert_eq!(h.favourite_items(), [Item::Tool("draw_circle".into())]);
        assert!(
            h.text_at("Favourites").is_none(),
            "{layout:?}: no tab or ★ list"
        );
    }
}

#[test]
fn a_whole_group_can_be_a_favourite() {
    let mut h = floating_favourites(WindowLayout::Canvas);
    h.choose(
        |app| app.group_menu("Create Vectors"),
        &["Add to Favourites"],
    );
    assert_eq!(h.favourite_items(), [Item::Group("Create Vectors".into())]);
    let menu = h.group_menu("Create Vectors");
    assert_eq!(menu[0].name, "Remove from Favourites");
    h.choose(
        |app| app.group_menu("Create Vectors"),
        &["Remove from Favourites"],
    );
    assert_eq!(h.favourite_items(), [] as [Item; 0]);
}

#[test]
fn favourites_are_renamed_never_deleted_and_menus_say_the_new_name() {
    let mut h = floating_favourites(WindowLayout::Ribbon);
    let menu = h.favourites_menu();
    let names: Vec<&str> = menu.iter().map(|i| i.name.as_str()).collect();
    assert!(names.contains(&"Rename Toolbar"));
    assert!(!names.contains(&"Delete Toolbar"), "Favourites stays");
    h.choose(CutCraftApp::favourites_menu, &["Rename Toolbar"]);
    h.frame(vec![]);
    h.frame(vec![]);
    h.frame(vec![Event::Key {
        key: Key::A,
        physical_key: None,
        pressed: true,
        repeat: false,
        modifiers: Modifiers::COMMAND,
    }]);
    h.frame(vec![Event::Text("Shop".into())]);
    h.frame(vec![Event::Key {
        key: Key::Enter,
        physical_key: None,
        pressed: true,
        repeat: false,
        modifiers: Modifiers::NONE,
    }]);
    h.frame(vec![]);
    assert_eq!(h.options().favourites.name.as_deref(), Some("Shop"));
    let menu = h.row_menu("draw_circle");
    assert_eq!(menu[0].name, "Add to Shop");
    h.choose(|app| app.row_menu("draw_circle"), &["Add to Shop"]);
    assert_eq!(h.row_menu("draw_circle")[0].name, "Remove from Shop");
}

#[test]
fn attached_favourites_are_the_ribbon_tab_again_and_detach_floats_them() {
    let mut h = floating_favourites(WindowLayout::Ribbon);
    h.choose(CutCraftApp::favourites_menu, &["Attach to Header"]);
    h.frame(vec![]);
    h.frame(vec![]);
    assert_eq!(h.options().favourites.bar, Place::Layout);
    assert!(h.text_at("Favourites").is_some(), "its tab is back");
    assert!(h.favourites_bar_rect().is_none() || h.text_at("Favourites").is_some());
    h.choose(CutCraftApp::favourites_menu, &["Detach"]);
    h.frame(vec![]);
    h.frame(vec![]);
    assert!(matches!(h.options().favourites.bar, Place::Floating(_)));
    assert!(h.text_at("Favourites").is_none(), "no tab while floating");
}

#[test]
fn favourites_snap_to_a_side_like_any_bar() {
    let mut h = floating_favourites(WindowLayout::Canvas);
    h.choose(|app| app.row_menu("draw_circle"), &["Add to Favourites"]);
    let before = h.view_rect();
    h.choose(CutCraftApp::favourites_menu, &["Snap to Bottom"]);
    h.frame(vec![]);
    h.frame(vec![]);
    assert!(
        h.view_rect().bottom() < before.bottom() - 10.0,
        "the drawing made room"
    );
}
