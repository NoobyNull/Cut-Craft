//! The names of the tools, the groups and boxes they sit in, the sub-tabs and the menu items,
//! in the language in use. The English names are kept in Fluent (`locales/en/tools.ftl`,
//! `tool-groups.ftl`, `subtabs.ftl` and `menu-items.ftl`) like every other word the app shows,
//! so a language pack translates them too.
//!
//! Each is found by a key made from what never changes on screen: a tool by its id
//! (`tools-draw-circle`), a group, a box or a menu item by its English name (`group-create-vectors`,
//! `menu-item-save-as`), a sub-tab by its page and English name (`design-subtab-model`). A tool
//! an add-on or a gadget brings has no key, and shows the name it gives.
//!
//! A tool's tooltip (what it does) is found by its id too: `tooltip-draw-circle`, in
//! `tooltips.ftl`.
//!
//! Every layout, menu, Find Tool and help page asks here, so a tool is named the same way
//! wherever it shows.

use crate::gui::{MenuItem, SubTab};
use crate::pages::Page;
use cut_craft_core::Registry;
use cut_craft_i18n::{english, key_of, tr, tr_or};

/// A tool's key: `tools-` and its id, hyphenated (`draw_circle` is `tools-draw-circle`).
pub fn tool_key(id: &str) -> String {
    key_of("tools", id)
}

/// Tool `id`'s name in the language in use; `english` (the name its command gives) when it
/// has no key, as an add-on's tool or a gadget has none.
pub fn tool_name(id: &str, english: &str) -> String {
    tr_or(&tool_key(id), english)
}

/// Tool `id`'s English name as the code gives it: a tool box variant's, a built-in tool's,
/// its command's label, or for a tool not made yet its id in words. `locales/en/tools.ftl`
/// says the same for every tool on the pages (a test checks), and is what is shown.
pub fn english_name(registry: &Registry, id: &str) -> String {
    if let Some(variant) = crate::flyout::variant(id) {
        return variant.label.to_owned();
    }
    if let Some(label) = crate::gui::built_in_label(id) {
        return label.to_owned();
    }
    registry
        .get(id)
        .map_or_else(|| crate::gui::title_case(id), |c| c.meta().label.to_owned())
}

/// Tool `id`'s name in the language in use, the command's own name where it has no key.
pub fn name_of(registry: &Registry, id: &str) -> String {
    tool_name(id, &english_name(registry, id))
}

/// The name of the tool whose form is open, which the form knows by its command `id` and
/// the English name it was opened under (a tool box variant such as Draw Rectangle from
/// Center opens its command's form under its own name, as Setup Material opens Job Setup's).
pub fn form_name(registry: &Registry, id: &str, english: &str) -> String {
    let variant = crate::flyout::VARIANTS
        .iter()
        .find(|v| v.command == id && v.label == english)
        .map(|v| v.id);
    let built_in = crate::app::built_ins::BUILT_INS
        .iter()
        .find(|b| b.name == english)
        .map(|b| b.id);
    match variant.or(built_in) {
        Some(tool) => name_of(registry, tool),
        None => tool_name(id, english),
    }
}

/// A tool's tooltip key: `tooltip-` and its id (`tooltip-draw-circle`), what it does as its
/// tooltip and the status line say it.
pub fn tooltip_key(id: &str) -> String {
    key_of("tooltip", id)
}

/// What tool `id` does (`english`, as [`crate::gui::english_about`] says it), in the language
/// in use.
pub fn tooltip(id: &str, english: &str) -> String {
    tr_or(&tooltip_key(id), english)
}

/// A page tab's key, from its English name (`page-tab-layers`): the side tabs, the Rail's
/// and Ribbon's browser tabs and the Ribbon's tabs.
pub fn page_tab_key(name: &str) -> String {
    key_of("page-tab", name)
}

/// A page tab's name in the language in use.
pub fn page_tab_name(name: &str) -> String {
    tr_or(&page_tab_key(name), name)
}

/// A tool group's or a tool box's key, from its English name: `group-create-vectors`.
pub fn group_key(name: &str) -> String {
    key_of("group", name)
}

/// A tool group's or a tool box's name in the language in use. The code keeps finding a
/// group by its English name; only what is shown changes.
pub fn group_name(name: &str) -> String {
    tr_or(&group_key(name), name)
}

/// A sub-tab's key: its page and its English name (`design-subtab-create`,
/// `toolpaths-subtab-create`), as both pages have a Create sub-tab, each its own words.
pub fn subtab_key(page: Page, name: &str) -> String {
    let prefix = match page {
        Page::Design => "design-subtab",
        Page::Toolpaths => "toolpaths-subtab",
    };
    key_of(prefix, name)
}

/// A sub-tab's name in the language in use.
pub fn subtab_name(page: Page, tab: &SubTab) -> String {
    tr_or(&subtab_key(page, tab.name), tab.name)
}

/// What a sub-tab's tools are for (its tooltip), in the language in use.
pub fn subtab_hint(page: Page, tab: &SubTab) -> String {
    tr_or(&format!("{}.hint", subtab_key(page, tab.name)), tab.hint)
}

/// A menu item's key, from its English name: `menu-item-save-as`.
pub fn menu_item_key(name: &str) -> String {
    key_of("menu-item", name)
}

/// Whether a menu item is named just as the tool it runs is: then it shows the tool's name,
/// so translators translate it once.
fn named_as_its_tool(item: &MenuItem) -> bool {
    !item.action.is_empty()
        && english().words(&tool_key(&item.action), None).as_deref() == Some(item.name.as_str())
}

/// A menu item's name in the language in use: its tool's name when it is named as that
/// tool, else its own words. A recent file shows its own name, and so does a gadget the user
/// added.
pub fn menu_item_name(item: &MenuItem) -> String {
    if item.named || item.action.starts_with("open_recent:") {
        return item.name.clone();
    }
    // a gadget that comes with PowerWhittle has its name in tools.ftl; the user's own shows
    // the name it gives
    if item.action.starts_with("gadget_") {
        return tool_name(&item.action, &item.name);
    }
    if named_as_its_tool(item) {
        return tr(&tool_key(&item.action));
    }
    // a built-in action's item is named as the action is everywhere, "..." on one that opens
    // a window
    if let Some(b) = crate::app::built_ins::get(&item.action).filter(|b| item.name == b.menu_name())
    {
        let name = tr(&tool_key(b.id));
        return if b.window { format!("{name}...") } else { name };
    }
    tr_or(&menu_item_key(&item.name), &item.name)
}

/// Words besides its name that Find Tool (Ctrl+K) finds a tool by, in English: the words
/// people knew it by before (the Bit Library was the Tool Database), or call it by elsewhere.
const ALSO_FOUND_BY: &[(&str, &[&str])] = &[(
    "tool_database",
    &[
        "tool database",
        "tool library",
        "tooling",
        "cutter",
        "end mill",
        "endmill",
        "router bit",
        "bits",
    ],
)];

/// Whether Find Tool lists tool `id`, shown as `name`, for `typed`: its name or one of its
/// other words holds what was typed, in any case.
pub fn found_by(id: &str, name: &str, typed: &str) -> bool {
    let typed = typed.trim().to_lowercase();
    name.to_lowercase().contains(&typed)
        || ALSO_FOUND_BY
            .iter()
            .filter(|(tool, _)| *tool == id)
            .flat_map(|(_, words)| words.iter())
            .any(|word| word.contains(&typed))
}

#[cfg(test)]
mod bit_words;
#[cfg(test)]
mod tests;
