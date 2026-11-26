//! Toolbars of your own: each named, holding single tools and whole tool groups in the order
//! you put them, shown in every window layout (a tab in Ribbon, a button in Rail, a section
//! in Classic and the Canvas palette) or floating anywhere over the drawing. Kept in the
//! options; changed only through the actions here, so every layout changes them the same way.

use crate::bars::{self, Floating, Home, Place};
use crate::gui::MenuItem;
use serde::{Deserialize, Serialize};

/// A toolbar of your own.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(from = "Kept")]
pub struct Toolbar {
    pub name: String,
    #[serde(default)]
    pub items: Vec<Item>,
    /// Where it sits: in the layout, snapped to a side or floating.
    #[serde(flatten)]
    pub place: Place,
}

/// A toolbar as the options keep it, read from before it had a [`Place`] too: then it
/// was floating (where it was left) or in the layout.
#[derive(Deserialize)]
struct Kept {
    name: String,
    #[serde(default)]
    items: Vec<Item>,
    #[serde(default)]
    place: Option<String>,
    #[serde(flatten)]
    rest: serde_json::Map<String, serde_json::Value>,
}

impl From<Kept> for Toolbar {
    fn from(kept: Kept) -> Self {
        let mut rest = kept.rest;
        let place = match kept.place {
            Some(place) => {
                rest.insert("place".into(), place.into());
                serde_json::from_value(rest.into()).unwrap_or(Place::Layout)
            }
            None if rest.get("floating").and_then(serde_json::Value::as_bool) == Some(true) => {
                let offset = rest
                    .get("at")
                    .and_then(|at| serde_json::from_value::<[f32; 2]>(at.clone()).ok());
                Place::Floating(Floating {
                    offset,
                    ..Floating::default()
                })
            }
            None => Place::Layout,
        };
        Self {
            name: kept.name,
            items: kept.items,
            place,
        }
    }
}

/// What a toolbar holds: one tool (by id) or a tool group (by its English name), which
/// opens as a list of its tools.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Item {
    Tool(String),
    Group(String),
}

impl Item {
    /// This item as an action names it: `tool:<id>` or `group:<name>`.
    pub fn key(&self) -> String {
        match self {
            Self::Tool(id) => format!("tool:{id}"),
            Self::Group(name) => format!("group:{name}"),
        }
    }

    /// The item an action names, from `tool:<id>` or `group:<name>`.
    pub fn from_key(key: &str) -> Option<Self> {
        match key.split_once(':')? {
            ("tool", id) if !id.is_empty() => Some(Self::Tool(id.to_owned())),
            ("group", name) if !name.is_empty() => Some(Self::Group(name.to_owned())),
            _ => None,
        }
    }
}

/// A toolbar action: then what to do and what to.
///
/// - `add:<toolbar>:<item>` puts an item on the end of a toolbar (once);
/// - `new:<item>` makes a new toolbar holding it;
/// - `remove:<toolbar>:<at>` takes an item off;
/// - `move:<toolbar>:<from>:<to>` moves an item to another place on its toolbar;
/// - `place:<toolbar>:<what>` snaps, floats, locks, turns, detaches or attaches it
///   ([`bars::apply`]);
/// - `rename:<toolbar>:<name>` and `delete:<toolbar>`.
pub const ACTION: &str = "toolbar:";

/// The name a new toolbar gets: the first "My Tools", "My Tools 2", … not taken (in the
/// language in use).
pub fn new_name(toolbars: &[Toolbar]) -> String {
    let taken = |name: &str| toolbars.iter().any(|t| t.name == name);
    let base = cut_craft_i18n::tr("toolbars-new-name");
    // one of these is free, as there are only so many toolbars
    (1..=toolbars.len() + 1)
        .map(|n| {
            if n == 1 {
                base.clone()
            } else {
                format!("{base} {n}")
            }
        })
        .find(|name| !taken(name))
        .unwrap_or_default()
}

/// Carries out toolbar action `action` (after [`ACTION`]); whether it changed anything.
pub fn apply(toolbars: &mut Vec<Toolbar>, action: &str) -> bool {
    let Some((what, rest)) = action.split_once(':') else {
        return false;
    };
    let index = |s: &str| s.parse::<usize>().ok();
    match what {
        "new" => {
            let Some(item) = Item::from_key(rest) else {
                return false;
            };
            toolbars.push(Toolbar {
                name: new_name(toolbars),
                items: vec![item],
                place: Place::default(),
            });
            true
        }
        "add" => {
            let Some((n, key)) = rest.split_once(':') else {
                return false;
            };
            let (Some(toolbar), Some(item)) = (
                index(n).and_then(|n| toolbars.get_mut(n)),
                Item::from_key(key),
            ) else {
                return false;
            };
            if toolbar.items.contains(&item) {
                return false;
            }
            toolbar.items.push(item);
            true
        }
        "remove" => {
            let Some((n, at)) = rest.split_once(':') else {
                return false;
            };
            match (index(n).and_then(|n| toolbars.get_mut(n)), index(at)) {
                (Some(toolbar), Some(at)) if at < toolbar.items.len() => {
                    toolbar.items.remove(at);
                    true
                }
                _ => false,
            }
        }
        "move" => {
            let mut parts = rest.splitn(3, ':');
            let (Some(n), Some(from), Some(to)) = (
                parts.next().and_then(index),
                parts.next().and_then(index),
                parts.next().and_then(index),
            ) else {
                return false;
            };
            let Some(toolbar) = toolbars.get_mut(n) else {
                return false;
            };
            let len = toolbar.items.len();
            if from >= len || from == to {
                return false;
            }
            let item = toolbar.items.remove(from);
            toolbar.items.insert(to.min(len - 1), item);
            true
        }
        "place" => {
            let Some((n, what)) = rest.split_once(':') else {
                return false;
            };
            index(n)
                .and_then(|n| toolbars.get_mut(n))
                .is_some_and(|toolbar| bars::apply(&mut toolbar.place, what, true))
        }
        "rename" => {
            let Some((n, name)) = rest.split_once(':') else {
                return false;
            };
            let name = name.trim();
            match index(n).and_then(|n| toolbars.get_mut(n)) {
                Some(toolbar) if !name.is_empty() && toolbar.name != name => {
                    name.clone_into(&mut toolbar.name);
                    true
                }
                _ => false,
            }
        }
        "delete" => match index(rest) {
            Some(n) if n < toolbars.len() => {
                toolbars.remove(n);
                true
            }
            _ => false,
        },
        _ => false,
    }
}

/// How many of your toolbars a tool's menu lists one by one before it folds them all into
/// Add to Toolbar ▸.
pub const LISTED: usize = 6;

/// What a tool's or group's right-click menu offers for your toolbars: Add to (or Remove
/// from) each by its name, then Add to New Toolbar. `folded` (Options ▸ General ▸ Add to
/// menus), or with more than [`LISTED`] toolbars, all of that is under Add to Toolbar ▸.
pub fn add_items(toolbars: &[Toolbar], item: &Item, folded: bool) -> Vec<MenuItem> {
    if folded || toolbars.len() > LISTED {
        return vec![add_menu(toolbars, item)];
    }
    let key = item.key();
    let mut items: Vec<MenuItem> = toolbars
        .iter()
        .enumerate()
        .map(|(n, t)| {
            let bar = t.name.as_str();
            // called what you called it, in every language
            match t.items.iter().position(|i| i == item) {
                Some(at) => MenuItem::with_action(
                    &cut_craft_i18n::tr!("bars-remove-from", bar = bar),
                    &format!("{ACTION}remove:{n}:{at}"),
                ),
                None => MenuItem::with_action(
                    &cut_craft_i18n::tr!("bars-add-to", bar = bar),
                    &format!("{ACTION}add:{n}:{key}"),
                ),
            }
            .named()
        })
        .collect();
    items.push(MenuItem::with_action(
        "Add to New Toolbar",
        &format!("{ACTION}new:{key}"),
    ));
    items
}

/// The Add to Toolbar submenu for `item`: each toolbar by name (ticked and greyed where it
/// is on it already), then New Toolbar.
pub fn add_menu(toolbars: &[Toolbar], item: &Item) -> MenuItem {
    let key = item.key();
    let mut children: Vec<MenuItem> = toolbars
        .iter()
        .enumerate()
        .map(|(n, t)| {
            let on = t.items.contains(item);
            // called what you called it, in every language
            MenuItem {
                is_checked: on,
                is_enabled: !on,
                ..MenuItem::with_action(&t.name, &format!("{ACTION}add:{n}:{key}")).named()
            }
        })
        .collect();
    if !children.is_empty() {
        children.push(MenuItem::separator());
    }
    children.push(MenuItem::with_action(
        "New Toolbar",
        &format!("{ACTION}new:{key}"),
    ));
    MenuItem::submenu("Add to Toolbar", children)
}

/// What a right-click on an item of toolbar `n` offers: moving it along, taking it off,
/// and the toolbar's own menu.
pub fn item_menu(toolbars: &[Toolbar], n: usize, at: usize, home: Home) -> Vec<MenuItem> {
    let len = toolbars.get(n).map_or(0, |t| t.items.len());
    let mut menu = vec![
        MenuItem {
            is_enabled: at > 0,
            ..MenuItem::with_action(
                "Move Earlier",
                &format!("{ACTION}move:{n}:{at}:{}", at.saturating_sub(1)),
            )
        },
        MenuItem {
            is_enabled: at + 1 < len,
            ..MenuItem::with_action("Move Later", &format!("{ACTION}move:{n}:{at}:{}", at + 1))
        },
        MenuItem::with_action("Remove from Toolbar", &format!("{ACTION}remove:{n}:{at}")),
        MenuItem::separator(),
    ];
    menu.extend(toolbar_menu(toolbars, n, home));
    menu
}

/// A toolbar's own menu (right-click its name or grip): where it sits ([`bars::menu`],
/// `home` being where the layout puts it back), then rename it or delete it. Renaming opens
/// a box for the name ([`RENAME`]).
pub fn toolbar_menu(toolbars: &[Toolbar], n: usize, home: Home) -> Vec<MenuItem> {
    let mut menu = toolbars.get(n).map_or_else(Vec::new, |toolbar| {
        bars::menu(&toolbar.place, &format!("{ACTION}place:{n}:"), Some(home))
    });
    menu.push(MenuItem::separator());
    menu.push(MenuItem::with_action(
        "Rename Toolbar",
        &format!("{RENAME}{n}"),
    ));
    menu.push(MenuItem::with_action(
        "Delete Toolbar",
        &format!("{ACTION}delete:{n}"),
    ));
    menu
}

/// Opening the box to rename toolbar `n`: then its index.
pub const RENAME: &str = "toolbar_rename:";

#[cfg(test)]
mod tests;
