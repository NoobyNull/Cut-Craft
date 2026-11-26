//! View ▸ Customize Toolbars…: every tool on the left, the bars on the right, and the
//! buttons between doing what the right-click menus do.

use super::*;
use crate::app::custom_toolbars::FAVOURITES_BAR;
use crate::custom_toolbars::Item;

fn click_text(h: &mut Harness, text: &str) {
    let at = h
        .text_at(text)
        .unwrap_or_else(|| panic!("{text} not shown"));
    h.click(at, Modifiers::NONE);
}

fn customizing() -> Harness {
    let mut h = Harness::new();
    h.invoke("customize_toolbars");
    h.frame(vec![]);
    h.frame(vec![]);
    h
}

#[test]
fn the_view_menu_and_every_bars_menu_open_it() {
    let h = customizing();
    assert!(h.customizing());
    assert!(h.text_at("Customize Toolbars").is_some(), "its title");
    assert!(
        h.text_at("Main tools (every tool)").is_some(),
        "the main bar, greyed"
    );
    let menu = crate::bars::menu(&crate::bars::Place::default(), "x:", None);
    assert_eq!(menu.last().unwrap().action, crate::bars::CUSTOMIZE);
    assert!(
        crate::app::built_ins::get(crate::bars::CUSTOMIZE).is_some(),
        "a built-in action, so View lists it"
    );
}

#[test]
fn a_favourite_added_is_picked_where_it_went_and_kept_through_options_apply() {
    let mut h = customizing();
    h.customize_pick(Item::Tool("3d_roughing_toolpath".into()));
    h.frame(vec![]);
    click_text(&mut h, "Add →");
    // Options open with a change waiting while Favourites change
    h.open_options_at("general");
    h.frame(vec![]);
    h.options_edit_waiting(|o| o.general.recent_file_list_size += 1);
    h.customize_pick(Item::Tool("draw_circle".into()));
    h.customize_press("add");
    let items = h.favourite_items();
    assert_eq!(
        items[0],
        Item::Tool("draw_circle".into()),
        "Create/Edit first"
    );
    assert_eq!(h.customize_picked(), Some(0), "the circle, just added");
    h.apply_options_window();
    assert_eq!(
        h.favourite_items().len(),
        2,
        "Apply keeps the Favourites added"
    );
}

#[test]
fn options_general_opens_it_too() {
    let mut h = Harness::new();
    h.open_options_at("general");
    h.frame(vec![]);
    h.frame(vec![]);
    click_text(&mut h, "Customize Toolbars...");
    h.frame(vec![]);
    assert!(h.customizing());
}

#[test]
fn add_puts_the_tool_on_favourites_and_remove_takes_it_off() {
    let mut h = customizing();
    assert!(!h.customize_can("add"), "nothing picked yet");
    assert!(!h.customize_can("delete"), "Favourites is never deleted");
    h.customize_pick(Item::Tool("draw_circle".into()));
    h.frame(vec![]);
    click_text(&mut h, "Add →");
    assert_eq!(h.favourite_tools(Page::Design), ["draw_circle"]);
    assert!(!h.customize_can("add"), "on it already");
    // a whole group too, after it
    h.customize_pick(Item::Group("Create Vectors".into()));
    h.frame(vec![]);
    click_text(&mut h, "Add →");
    h.customize_pick_bar(FAVOURITES_BAR, Some(0));
    h.frame(vec![]);
    click_text(&mut h, "← Remove");
    assert_eq!(h.favourite_tools(Page::Design), [] as [&str; 0]);
    assert_eq!(h.favourite_items(), [Item::Group("Create Vectors".into())]);
}

#[test]
fn a_new_toolbar_is_made_filled_moved_along_and_deleted() {
    let mut h = customizing();
    h.customize_pick(Item::Tool("draw_circle".into()));
    h.frame(vec![]);
    click_text(&mut h, "New Toolbar");
    assert_eq!(h.options().toolbars.len(), 1);
    h.customize_pick(Item::Tool("draw_line".into()));
    h.frame(vec![]);
    click_text(&mut h, "Add →");
    assert_eq!(
        h.options().toolbars[0].items,
        [
            Item::Tool("draw_circle".into()),
            Item::Tool("draw_line".into())
        ]
    );
    // the line, picked as it was added, moves up
    click_text(&mut h, "Move Up");
    assert_eq!(
        h.options().toolbars[0].items[0],
        Item::Tool("draw_line".into())
    );
    assert!(!h.customize_can("up"), "first now");
    click_text(&mut h, "Delete");
    assert_eq!(h.options().toolbars.len(), 0);
}
