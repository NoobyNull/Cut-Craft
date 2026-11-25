use super::*;
use crate::gui::{MenuItem, MenuSystem};

fn every_item(items: &[MenuItem], out: &mut Vec<MenuItem>) {
    for item in items {
        out.push(item.clone());
        every_item(&item.children, out);
    }
}

/// Every item of the menu bar, with a job open and without.
fn menu_bar_items() -> Vec<MenuItem> {
    let menus = MenuSystem::new();
    let mut out = vec![];
    for menu in menus.menus.iter().chain(&menus.without_job().menus) {
        every_item(&menu.items, &mut out);
    }
    out
}

#[test]
fn each_action_is_listed_once() {
    let mut ids: Vec<&str> = BUILT_INS.iter().map(|b| b.id).collect();
    ids.sort_unstable();
    let before = ids.len();
    ids.dedup();
    assert_eq!(
        ids.len(),
        before,
        "an id listed twice: only the first is read"
    );
}

#[test]
fn every_action_has_a_name_and_says_what_it_does() {
    for b in BUILT_INS {
        assert!(!b.name.trim().is_empty(), "{} has no name", b.id);
        assert!(!b.about.trim().is_empty(), "{} says nothing", b.id);
        assert!(
            !b.name.ends_with("...") && !b.name.ends_with('…'),
            "{}: a window's \"...\" is the menu's, set with window()",
            b.id
        );
        assert_eq!(get(b.id), Some(b));
    }
    assert_eq!(
        get("draw_circle"),
        None,
        "a command is not a built-in action"
    );
}

#[test]
fn every_shortcut_is_one_the_keyboard_reads() {
    for b in BUILT_INS.iter().filter(|b| !b.keys.is_empty()) {
        assert!(
            crate::shortcut::parse(b.keys).is_some(),
            "{}: {} is not a shortcut",
            b.id,
            b.keys
        );
    }
    let mut keys: Vec<&str> = (BUILT_INS.iter().map(|b| b.keys))
        .filter(|k| !k.is_empty())
        .collect();
    keys.sort_unstable();
    let before = keys.len();
    keys.dedup();
    assert_eq!(keys.len(), before, "two built-in actions share a shortcut");
}

#[test]
fn the_menus_name_each_action_and_show_its_shortcut_as_the_table_says() {
    let items = menu_bar_items();
    for item in &items {
        if let Some(b) = get(&item.action) {
            assert_eq!(item.name, b.menu_name(), "{}", b.id);
            assert_eq!(item.shortcut, b.keys, "{}", b.id);
        }
    }
    // a window's item ends in "...", and the action's own name is the same as on its button
    let open = items.iter().find(|i| i.action == "open").unwrap();
    assert_eq!(open.name, "Open...");
    assert_eq!(open.shortcut, "Ctrl+O");
    let import = (items.iter())
        .find(|i| i.action == "import_vectors_bitmap")
        .unwrap();
    assert_eq!(import.name, "Import Vectors or Picture...");
    assert_eq!(
        crate::gui::built_in_label("import_vectors_bitmap"),
        Some("Import Vectors or Picture")
    );
}

#[test]
fn a_menu_item_of_an_action_shows_the_actions_name_in_the_language_in_use() {
    let item = MenuItem::built_in("save_as");
    assert_eq!(crate::tool_names::menu_item_name(&item), "Save As...");
    let undo = MenuItem::built_in("undo");
    assert_eq!(crate::tool_names::menu_item_name(&undo), "Undo");
}

#[test]
#[should_panic(expected = "is no built-in action")]
fn a_menu_item_for_an_action_not_listed_is_caught_when_the_menus_are_made() {
    let _ = MenuItem::built_in("not_an_action");
}
