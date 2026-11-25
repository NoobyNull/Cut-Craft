use super::*;
use crate::gui::{self, MenuSystem, Toolbar};
use crate::pages::Page;
use cut_craft_i18n::english;

/// Every tool on the pages, in group order, then those with no button of their own: every
/// command, each once.
fn every_tool() -> Vec<&'static str> {
    let toolbar = Toolbar::new();
    let mut ids: Vec<&'static str> = Vec::new();
    let groups = toolbar
        .groups
        .iter()
        .chain(&toolbar.modeling_groups)
        .chain(&toolbar.toolpath_groups);
    for id in groups.flat_map(|g| g.tools.iter().copied()) {
        if !ids.contains(&id) {
            ids.push(id);
        }
    }
    let registry = crate::commands::registry();
    for c in registry.commands() {
        if !ids.contains(&c.id()) {
            ids.push(c.id());
        }
    }
    for b in crate::built_ins::BUILT_INS {
        if !ids.contains(&b.id) {
            ids.push(b.id);
        }
    }
    ids
}

/// Every name shown over a group of tools: the groups, the tool boxes, and the lists the
/// layouts show as a group (Favourites, Gadgets).
fn every_group() -> Vec<&'static str> {
    let toolbar = Toolbar::new();
    let mut names: Vec<&'static str> = Vec::new();
    let groups = toolbar
        .groups
        .iter()
        .chain(&toolbar.modeling_groups)
        .chain(&toolbar.toolpath_groups);
    for name in groups
        .map(|g| g.name)
        .chain(crate::flyout::FLYOUTS.iter().map(|f| f.name))
        .chain([
            "Favourites",
            "Gadgets",
            "Design Favourites",
            "Toolpath Favourites",
        ])
    {
        if !names.contains(&name) {
            names.push(name);
        }
    }
    names
}

fn every_page_tab() -> Vec<&'static str> {
    let mut names: Vec<&'static str> = Vec::new();
    for name in crate::app::page_tabs() {
        if !names.contains(&name) {
            names.push(name);
        }
    }
    names
}

/// Every menu item the menu bar shows, with and without a job, the right-click menus of a
/// tool and of a sub-tab, and those of a job's toolpaths, layers, sheets, levels, components
/// and groups.
fn every_menu_item() -> Vec<String> {
    let menus = MenuSystem::new();
    let mut items: Vec<gui::MenuItem> = Vec::new();
    for menu in menus.menus.iter().chain(&menus.without_job().menus) {
        items.extend(menu.items.iter().cloned());
    }
    for page in [Page::Design, Page::Toolpaths] {
        for favourite in [false, true] {
            for among in [false, true] {
                for folded in [false, true] {
                    items.extend(gui::tool_menu(
                        page,
                        "draw_circle",
                        favourite,
                        among,
                        folded,
                    ));
                }
            }
            items.push(gui::favourite_item(page, "draw_circle", favourite));
        }
    }
    for set in [gui::DESIGN, gui::TOOLPATHS] {
        let name = set.tabs[0].name;
        items.extend(set.menu(name, &[]));
        items.extend(set.menu(name, &[name.to_owned()]));
    }
    // toolbars of your own, with none made yet (one's own name is its own)
    let item = crate::custom_toolbars::Item::Tool("draw_circle".to_owned());
    items.push(crate::custom_toolbars::add_menu(&[], &item));
    items.extend(crate::custom_toolbars::item_menu(&[], 0, 0));
    let (document, own) = crate::app::document_menus();
    items.extend(document);
    let mut names = Vec::new();
    menu_item_names(&items, &mut names);
    // the job's own names (a layer's, a sheet's) are the person's words
    names.retain(|n| !own.contains(n));
    names
}

/// The English names that need a key of their own in `menu-items.ftl`: every named item of
/// `items` and their submenus, but for those named as the tool they run. That is worked out
/// from the code rather than from `tools.ftl` as built, so `write_english_names` agrees with
/// itself in one run.
fn menu_item_names(items: &[gui::MenuItem], out: &mut Vec<String>) {
    let registry = crate::commands::registry();
    let tools = every_tool();
    let named_as_tool = |item: &gui::MenuItem| {
        let built_in =
            crate::built_ins::get(&item.action).is_some_and(|b| item.name == b.menu_name());
        built_in
            || (tools.contains(&item.action.as_str())
                && english_name(&registry, &item.action) == item.name)
    };
    for item in items {
        if !item.name.is_empty() && !item.named && !named_as_tool(item) && !out.contains(&item.name)
        {
            out.push(item.name.clone());
        }
        menu_item_names(&item.children, out);
    }
}

/// What is wrong with `key`'s English words, which should say `want`.
fn mismatch(key: &str, want: &str) -> Option<String> {
    match english().words(key, None) {
        Some(got) if got == want => None,
        Some(got) => Some(format!("{key} = {got}, but the code says {want}")),
        None => Some(format!("{key} = {want} is missing")),
    }
}

fn check(problems: &[String], file: &str) {
    assert!(
        problems.is_empty(),
        "crates/i18n/locales/en/{file} does not match the code; put these lines there:\n{}",
        problems.join("\n")
    );
}

#[test]
fn every_tool_has_its_name_in_english() {
    let registry = crate::commands::registry();
    let problems: Vec<String> = every_tool()
        .into_iter()
        .filter_map(|id| mismatch(&tool_key(id), &english_name(&registry, id)))
        .collect();
    check(&problems, "tools.ftl");
}

/// Every id that says what it does: the hints, the command crates, the built-in tools and the
/// tool box variants, each once.
fn every_tooltip() -> Vec<(&'static str, &'static str)> {
    let mut ids: Vec<&'static str> = crate::hints::HINTS.iter().map(|(id, _)| *id).collect();
    ids.extend(gui::command_ids());
    ids.extend(crate::built_ins::BUILT_INS.iter().map(|b| b.id));
    ids.extend(crate::flyout::VARIANTS.iter().map(|v| v.id));
    ids.extend(every_tool());
    let mut out: Vec<(&'static str, &'static str)> = vec![];
    for id in ids {
        if out.iter().any(|(i, _)| *i == id) {
            continue;
        }
        if let Some(about) = gui::english_about(id) {
            out.push((id, about));
        }
    }
    out
}

#[test]
fn every_tooltip_has_its_words_in_english() {
    let problems: Vec<String> = every_tooltip()
        .into_iter()
        .filter_map(|(id, about)| mismatch(&tooltip_key(id), about))
        .collect();
    check(&problems, "tooltips.ftl");
}

#[test]
fn every_group_has_its_name_in_english() {
    let problems: Vec<String> = every_group()
        .into_iter()
        .filter_map(|name| mismatch(&group_key(name), name))
        .collect();
    check(&problems, "tool-groups.ftl");
}

#[test]
fn every_subtab_has_its_name_and_hint_in_english() {
    let mut problems = Vec::new();
    for (page, set) in [
        (Page::Design, gui::DESIGN),
        (Page::Toolpaths, gui::TOOLPATHS),
    ] {
        for tab in set.tabs {
            let key = subtab_key(page, tab.name);
            problems.extend(mismatch(&key, tab.name));
            problems.extend(mismatch(&format!("{key}.hint"), tab.hint));
        }
    }
    check(&problems, "subtabs.ftl");
}

#[test]
fn every_page_tab_has_its_name_in_english() {
    let problems: Vec<String> = every_page_tab()
        .into_iter()
        .filter_map(|name| mismatch(&page_tab_key(name), name))
        .collect();
    check(&problems, "subtabs.ftl");
}

#[test]
fn every_menu_item_has_its_name_in_english() {
    let problems: Vec<String> = every_menu_item()
        .iter()
        .filter_map(|name| mismatch(&menu_item_key(name), name))
        .collect();
    check(&problems, "menu-items.ftl");
}

#[test]
fn no_english_name_is_left_over() {
    // a key for a tool, group or menu item that is gone would be translated for nothing
    let tools: Vec<String> = every_tool().into_iter().map(tool_key).collect();
    let groups: Vec<String> = every_group().into_iter().map(group_key).collect();
    let tabs: Vec<String> = every_page_tab().into_iter().map(page_tab_key).collect();
    let items: Vec<String> = every_menu_item().iter().map(|n| menu_item_key(n)).collect();
    let tooltips: Vec<String> = every_tooltip()
        .into_iter()
        .map(|(id, _)| tooltip_key(id))
        .collect();
    let subtabs: Vec<String> = [
        (Page::Design, gui::DESIGN),
        (Page::Toolpaths, gui::TOOLPATHS),
    ]
    .into_iter()
    .flat_map(|(page, set)| set.tabs.iter().map(move |t| subtab_key(page, t.name)))
    .collect();
    let known = |key: &str| {
        let message = key.split('.').next().unwrap_or(key);
        [&tools, &groups, &tabs, &items, &subtabs, &tooltips]
            .iter()
            .any(|keys| keys.iter().any(|k| k == message))
    };
    let stale: Vec<&String> = english()
        .keys()
        .iter()
        .filter(|k| {
            [
                "tools-",
                "group-",
                "page-tab-",
                "menu-item-",
                "design-subtab-",
                "toolpaths-subtab-",
                "tooltip-",
            ]
            .iter()
            .any(|p| k.starts_with(p))
        })
        .filter(|k| !known(k))
        .collect();
    assert!(stale.is_empty(), "no longer shown: {stale:?}");
}

#[test]
fn a_menu_item_named_as_its_tool_shares_the_tools_words() {
    let item = gui::MenuItem::with_action("Draw Circle", "draw_circle");
    assert!(named_as_its_tool(&item));
    assert_eq!(menu_item_name(&item), "Draw Circle");
    // a recent file or a gadget is never looked up
    let recent = gui::MenuItem::with_action("Save As", "open_recent:0");
    assert_eq!(menu_item_name(&recent), "Save As");
    let gadget = gui::MenuItem::with_action("Open", "gadget_open");
    assert_eq!(menu_item_name(&gadget), "Open");
}

#[test]
fn a_name_with_no_key_shows_as_given() {
    assert_eq!(
        tool_name("gadget:hole_pattern", "Hole Pattern"),
        "Hole Pattern"
    );
    assert_eq!(group_name("My Add-on Tools"), "My Add-on Tools");
    let registry = crate::commands::registry();
    assert_eq!(name_of(&registry, "draw_circle"), "Draw Circle");
    assert_eq!(
        form_name(&registry, "draw_rectangle", "Draw Rectangle from Center"),
        "Draw Rectangle from Center"
    );
}

/// Writes the English files from the code (`cargo test -p cut-craft --bin cut-craft
/// write_english_names -- --ignored`), when adding many names at once.
#[test]
#[ignore = "writes files; run by hand"]
fn write_english_names() {
    use std::fmt::Write as _;
    let registry = crate::commands::registry();
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../i18n/locales/en");
    let mut tools = String::from(TOOLS_HEADER);
    for id in every_tool() {
        let _ = writeln!(tools, "{} = {}", tool_key(id), english_name(&registry, id));
    }
    std::fs::write(dir.join("tools.ftl"), tools).unwrap();
    let mut groups = String::from(GROUPS_HEADER);
    for name in every_group() {
        let _ = writeln!(groups, "{} = {name}", group_key(name));
    }
    std::fs::write(dir.join("tool-groups.ftl"), groups).unwrap();
    let mut tabs = String::from(SUBTABS_HEADER);
    for (page, set) in [
        (Page::Design, gui::DESIGN),
        (Page::Toolpaths, gui::TOOLPATHS),
    ] {
        for tab in set.tabs {
            let _ = writeln!(tabs, "{} = {}", subtab_key(page, tab.name), tab.name);
            let _ = writeln!(tabs, "    .hint = {}", tab.hint);
        }
    }
    tabs.push_str(
        "\n# The side tabs, the Rail's and Ribbon's browser tabs and the Ribbon's tabs.\n",
    );
    for name in every_page_tab() {
        let _ = writeln!(tabs, "{} = {name}", page_tab_key(name));
    }
    std::fs::write(dir.join("subtabs.ftl"), tabs).unwrap();
    let mut items = String::from(MENU_ITEMS_HEADER);
    for name in every_menu_item() {
        let _ = writeln!(items, "{} = {name}", menu_item_key(&name));
    }
    std::fs::write(dir.join("menu-items.ftl"), items).unwrap();
    let mut tips = String::from(TOOLTIPS_HEADER);
    for (id, about) in every_tooltip() {
        let _ = writeln!(
            tips,
            "{} ={}",
            tooltip_key(id),
            cut_craft_i18n::ftl_value(about)
        );
    }
    std::fs::write(dir.join("tooltips.ftl"), tips).unwrap();
}

const TOOLS_HEADER: &str = "\
# Every tool's name, as its button, Find Tool (Ctrl+K), the menus and its help page show it.
# A tool's key is `tools-` and its id, so it stays the same when its English name changes.
# A menu item named just as the tool it runs uses these words too.

";
const GROUPS_HEADER: &str = "\
# The names over groups of tools: the tool groups of every layout, the captioned boxes of tools
# that do one job different ways, and the lists shown as a group. A group's key is `group-` and
# its English name.

";
const SUBTABS_HEADER: &str = "\
# The Design and Toolpaths pages' sub-tabs (the Ribbon and Classic layouts' rows of tabs over
# the tools), each with its tooltip (.hint): what its tools are for.

";
const TOOLTIPS_HEADER: &str = "\
# What each tool does: its tooltip, and the status line while it is pointed at. A tooltip's
# key is `tooltip-` and the tool's id, as its name's is `tools-` and its id.

";
const MENU_ITEMS_HEADER: &str = "\
# The menu items of the menu bar and of a tool's and a sub-tab's right-click menus. An item
# named just as the tool it runs is not here: it shows the tool's name from tools.ftl.
# A key is `menu-item-` and the item's English name.

";
