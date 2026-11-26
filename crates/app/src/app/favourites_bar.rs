//! Favourites as a bar: the first of the bars, always there (it can be renamed but never
//! deleted), holding the favourite tools and groups of both pages, Create/Edit ones first.
//! It sits wherever its grip puts it like any bar; attached to the layout it shows as it
//! always has (the Ribbon and Canvas Favourites tab, the Rail's ★, Classic's Show
//! Favourites).

use super::bars::{Bar, Room};
use super::custom_toolbars::{FAVOURITES, FAVOURITES_BAR, Strip};
use super::{CutCraftApp, TOOL_ICON_ROWS, egui, gui};
use crate::bars::{self, Place};
use crate::custom_toolbars::{self as toolbars, Item};
use crate::pages::Page;
use crate::tool_model::Place as MenuPlace;
use cut_craft_i18n::tr;

/// A whole group as Favourites keep it, among the tools' ids.
const GROUP: &str = "group:";

impl CutCraftApp {
    /// What Favourites is called: what it was renamed to, or Favourites.
    pub(super) fn favourites_name(&self) -> String {
        self.options
            .favourites
            .name
            .clone()
            .unwrap_or_else(|| tr!("bars-favourites"))
    }

    /// Whether Favourites is attached to the layout (its tab, the Rail's ★) rather than a
    /// bar of its own.
    pub(super) fn favourites_in_layout(&self) -> bool {
        self.options.favourites.bar == Place::Layout
    }

    /// Favourites' tools and groups in order, Create/Edit ones first; any this machine or
    /// this version no longer has are left out.
    pub(super) fn favourite_items(&self) -> Vec<Item> {
        let model = self.tool_model();
        let groups = model.all_groups();
        let mut items = vec![];
        for page in [Page::Design, Page::Toolpaths] {
            let tools = model.favourites(page, &self.options.favourites);
            for key in self.options.favourites.of(page).0 {
                let item = match key.strip_prefix(GROUP) {
                    Some(name) if groups.iter().any(|g| g.name == name) => {
                        Item::Group(name.to_owned())
                    }
                    Some(_) => continue,
                    None => match tools.iter().find(|&&id| id == key) {
                        Some(&id) if self.offered(id) => Item::Tool(id.to_owned()),
                        _ => continue,
                    },
                };
                items.push(item);
            }
        }
        items
    }

    /// The page a group's favourite is kept with: its first tool's.
    fn group_page(&self, name: &str) -> Option<Page> {
        let group = self.group_named(name)?;
        let first = group.tools.first()?;
        Some(self.tool_model().kind_of(first))
    }

    /// A group's Add to (or Remove from) Favourites.
    pub(super) fn group_favourite_item(&self, name: &str) -> Option<gui::MenuItem> {
        let page = self.group_page(name)?;
        let key = format!("{GROUP}{name}");
        let favourite = self.options.favourites.of(page).0.contains(&key);
        let item = gui::favourite_item(page, &key, favourite);
        Some(match &self.options.favourites.name {
            Some(bar) => gui::favourite_named(item, favourite, bar),
            None => item,
        })
    }

    /// What a right-click on an item of the Favourites bar offers: take it off, move it
    /// along, and add it to your toolbars.
    pub(super) fn favourite_item_menu(&self, item: &Item) -> Vec<gui::MenuItem> {
        let menu = match item {
            Item::Tool(id) => self.tool_model().menu(
                id,
                MenuPlace::Favourites,
                &self.options.favourites,
                self.options.general.fold_tool_groups,
            ),
            Item::Group(name) => {
                let Some(page) = self.group_page(name) else {
                    return vec![];
                };
                let key = format!("{GROUP}{name}");
                let mut menu = gui::tool_menu(page, &key, true, true, false);
                if let Some(bar) = &self.options.favourites.name {
                    menu[0] = gui::favourite_named(menu[0].clone(), true, bar);
                }
                menu
            }
        };
        self.with_toolbar_menu(menu, item)
    }

    /// Favourites renamed to `name` (back to Favourites when it is that again, or empty).
    pub(super) fn rename_favourites(&mut self, name: &str) {
        let name = name.trim();
        self.options.favourites.name =
            (!name.is_empty() && name != tr!("bars-favourites")).then(|| name.to_owned());
        self.save_options();
    }

    /// The Favourites bar's grip menu: where it sits, then Rename (never Delete).
    pub(super) fn favourites_menu(&self) -> Vec<gui::MenuItem> {
        let mut menu = bars::menu(
            &self.options.favourites.bar,
            &format!("{}{FAVOURITES}", toolbars::ACTION),
            Some(self.bar_home()),
        );
        menu.push(gui::MenuItem::separator());
        menu.push(gui::MenuItem::with_action(
            "Rename Toolbar",
            &format!("{}{FAVOURITES_BAR}", toolbars::RENAME),
        ));
        menu
    }

    /// A right-click on Favourites' tab or ★ in the layout: its grip's menu.
    pub(super) fn favourites_tab_menu(&mut self, response: &egui::Response) {
        if let Some(action) = self.popup(response, || self.favourites_menu()) {
            self.toolbars.pending = Some(action);
        }
    }

    /// The Favourites bar when it is snapped (no `start`) or floating (from `start`). How
    /// many floating bars it drew (so the next starts under it).
    pub(super) fn placed_favourites(
        &mut self,
        ui: &mut egui::Ui,
        room: &mut Room,
        start: Option<egui::Pos2>,
    ) -> usize {
        let place = self.options.favourites.bar;
        let name = self.favourites_name();
        let menu = self.favourites_menu();
        let bar = Bar {
            id: favourites_bar_id(),
            name: &name,
            menu: &menu,
        };
        let mut contents = |app: &mut Self, ui: &mut egui::Ui, across: bool| {
            let strip = if across { Strip::Across } else { Strip::Down };
            let _ = app.toolbar_strip(ui, FAVOURITES_BAR, strip);
        };
        let shown = match (place, start) {
            (Place::Snapped { side }, None) => {
                self.snapped_bar(ui, room, &bar, side, &mut contents)
            }
            (Place::Floating(floating), Some(start)) => {
                self.floating_bar(ui.ctx(), &bar, floating, start, &mut contents)
            }
            _ => return 0,
        };
        if let Some(action) = shown.action {
            self.toolbars.pending = Some(action);
        }
        if let Some(moved) = shown.moved {
            self.options.favourites.bar = Place::Floating(moved);
            self.save_options();
        }
        usize::from(start.is_some())
    }
}

/// The id of the Favourites bar while it is snapped or floating.
pub(super) fn favourites_bar_id() -> egui::Id {
    egui::Id::unique("favourites_bar")
}

/// How far down the next floating bar starts, one bar's height.
pub(super) fn bar_step(ui: &egui::Ui) -> f32 {
    1.5 * TOOL_ICON_ROWS * ui.spacing().interact_size.y
}
