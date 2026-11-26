//! View ▸ Customize Toolbars…, also on every bar's grip menu: every tool on the left (by
//! group, with a search box), your bars on the right (Favourites first, the main bar shown
//! but greyed, as it cannot change), and buttons between to add, take off, move, make,
//! rename and delete. Every change goes through the same actions as the right-click menus,
//! so a bar changes the same way from either.

use super::custom_toolbars::FAVOURITES_BAR;
use super::{CutCraftApp, egui, gui};
use crate::custom_toolbars::{self as toolbars, Item};
use crate::pages::Page;
use crate::tool_names::group_name;
use cut_craft_i18n::tr;

/// What the window has picked: on the left a tool or group, on the right a bar and one of
/// its items, and what is typed in the search box.
#[derive(Clone, Debug, Default)]
pub(super) struct Customize {
    pub open: bool,
    pub left: Option<Item>,
    /// The bar on the right: [`FAVOURITES_BAR`] or one of your toolbars.
    pub bar: usize,
    pub right: Option<usize>,
    pub search: String,
}

impl Customize {
    /// Open, with Favourites picked on the right.
    pub fn opened() -> Self {
        Self {
            open: true,
            bar: FAVOURITES_BAR,
            ..Self::default()
        }
    }
}

impl CutCraftApp {
    /// Opens the Customize Toolbars window.
    pub(super) fn open_customize(&mut self) {
        self.customize = Customize::opened();
    }

    /// Closes the Customize Toolbars window, forgetting what it had picked.
    pub(crate) fn close_customize(&mut self) {
        self.customize = Customize::default();
    }

    /// Picks `item` on the window's left, as a click there does (the help pictures).
    pub(crate) fn customize_pick(&mut self, item: Item) {
        self.customize.left = Some(item);
    }

    /// Types `text` in the window's search box (the help pictures).
    pub(crate) fn customize_find(&mut self, text: &str) {
        text.clone_into(&mut self.customize.search);
    }

    /// The items of bar `bar` (Favourites or one of yours), in order.
    fn bar_items(&self, bar: usize) -> Vec<Item> {
        if bar == FAVOURITES_BAR {
            self.favourite_items()
        } else {
            self.options
                .toolbars
                .get(bar)
                .map_or_else(Vec::new, |t| t.items.clone())
        }
    }

    /// A bar's name as the window lists it.
    fn bar_name(&self, bar: usize) -> String {
        if bar == FAVOURITES_BAR {
            self.favourites_name()
        } else {
            self.options
                .toolbars
                .get(bar)
                .map_or_else(String::new, |t| t.name.clone())
        }
    }

    /// An item's name: the tool's, or the group's with ▸.
    fn item_name(&self, item: &Item) -> String {
        match item {
            Item::Tool(id) => self.tool_label(id),
            Item::Group(name) => format!("{} ▸", group_name(name)),
        }
    }

    /// The page a favourite of `item` is kept with.
    fn item_page(&self, item: &Item) -> Option<Page> {
        let first = match item {
            Item::Tool(id) => self
                .tool_model()
                .all_groups()
                .iter()
                .find_map(|g| g.tools.iter().copied().find(|t| t == id))?,
            Item::Group(name) => *self.group_named(name)?.tools.first()?,
        };
        Some(self.tool_model().kind_of(first))
    }

    /// Favourites' action `what` (`add`, `remove`, `earlier`, `later`) for `item`.
    fn favourite_action(&self, what: &str, item: &Item) -> Option<String> {
        let page = self.item_page(item)?;
        let key = match item {
            Item::Tool(id) => id.clone(),
            Item::Group(name) => format!("group:{name}"),
        };
        Some(format!(
            "{}{}:{what}:{key}",
            gui::FAVOURITE,
            gui::page_key(page)
        ))
    }

    /// The action the window's button `button` takes now, if it can take one: `add`,
    /// `remove`, `up`, `down`, `new`, `rename` or `delete`.
    pub(super) fn customize_action(&self, button: &str) -> Option<String> {
        let c = &self.customize;
        let favourites = c.bar == FAVOURITES_BAR;
        let items = self.bar_items(c.bar);
        let picked = c.right.and_then(|at| items.get(at).map(|item| (at, item)));
        let act = toolbars::ACTION;
        let n = c.bar;
        match button {
            "add" => {
                let item = c.left.as_ref()?;
                if items.contains(item) {
                    return None;
                }
                if favourites {
                    self.favourite_action("add", item)
                } else {
                    Some(format!("{act}add:{n}:{}", item.key()))
                }
            }
            "remove" => {
                let (at, item) = picked?;
                if favourites {
                    self.favourite_action("remove", item)
                } else {
                    Some(format!("{act}remove:{n}:{at}"))
                }
            }
            "up" | "down" => {
                let (at, item) = picked?;
                let to = if button == "up" {
                    at.checked_sub(1)?
                } else {
                    (at + 1 < items.len()).then_some(at + 1)?
                };
                if favourites {
                    // Favourites move along within their page
                    let page = self.item_page(item);
                    if items.get(to).and_then(|other| self.item_page(other)) != page {
                        return None;
                    }
                    let what = if button == "up" { "earlier" } else { "later" };
                    self.favourite_action(what, item)
                } else {
                    Some(format!("{act}move:{n}:{at}:{to}"))
                }
            }
            "new" => {
                let item = c.left.as_ref()?;
                Some(format!("{act}new:{}", item.key()))
            }
            "rename" => Some(format!("{}{n}", toolbars::RENAME)),
            "delete" => (!favourites).then(|| format!("{act}delete:{n}")),
            _ => None,
        }
    }

    /// Carries out the window's button `button`, keeping what is picked in step.
    pub(crate) fn customize_press(&mut self, ctx: &egui::Context, button: &str) {
        let Some(action) = self.customize_action(button) else {
            return;
        };
        let before = self.options.toolbars.len();
        let bar = self.customize.bar;
        let added = self.customize.left.clone();
        self.invoke(ctx, &action);
        // where the item added went: Favourites keep Create/Edit ones ahead of toolpath ones
        let at = added.and_then(|item| self.bar_items(bar).iter().position(|i| *i == item));
        let c = &mut self.customize;
        match button {
            "up" => c.right = c.right.map(|at| at.saturating_sub(1)),
            "down" => c.right = c.right.map(|at| at + 1),
            "remove" => c.right = None,
            "add" => c.right = at,
            "new" if self.options.toolbars.len() > before => {
                self.customize.bar = before;
                self.customize.right = Some(0);
            }
            "delete" => {
                self.customize.bar = FAVOURITES_BAR;
                self.customize.right = None;
            }
            _ => {}
        }
    }

    /// The window, while it is open.
    pub(super) fn customize_window(&mut self, ctx: &egui::Context) {
        if !self.customize.open {
            return;
        }
        let mut open = true;
        let row = ctx.global_style().spacing.interact_size.y;
        let mut pressed = None;
        egui::Window::new(tr!("customize-title"))
            .id(egui::Id::unique("customize_toolbars"))
            .open(&mut open)
            .collapsible(false)
            // over the floating bars it changes
            .order(egui::Order::Foreground)
            .default_width(36.0 * row)
            .show(ctx, |ui| {
                cut_craft_bug_report::mark(ui, "Customize toolbars");
                ui.columns(3, |cols| {
                    self.customize_tools(&mut cols[0], 18.0 * row);
                    pressed = self.customize_buttons(&mut cols[1]);
                    self.customize_bars(&mut cols[2], 18.0 * row);
                });
            });
        if let Some(button) = pressed {
            self.customize_press(ctx, button);
        }
        self.customize.open &= open;
    }

    /// The left: every tool by group, the groups themselves pickable, narrowed by search.
    fn customize_tools(&mut self, ui: &mut egui::Ui, height: f32) {
        ui.strong(tr!("customize-every-tool"));
        ui.add(
            egui::TextEdit::singleline(&mut self.customize.search)
                .hint_text(tr!("customize-search")),
        );
        let search = self.customize.search.trim().to_lowercase();
        let groups = self.tool_model().all_groups();
        egui::ScrollArea::vertical()
            .id_salt("customize tools")
            .max_height(height)
            .show(ui, |ui| {
                for group in &groups {
                    let tools: Vec<(&'static str, String)> = group
                        .tools
                        .iter()
                        .filter(|id| self.offered(id))
                        .map(|&id| (id, self.tool_label(id)))
                        .filter(|(_, name)| {
                            search.is_empty() || name.to_lowercase().contains(&search)
                        })
                        .collect();
                    if tools.is_empty() {
                        continue;
                    }
                    let whole = Item::Group(group.name.to_owned());
                    let on = self.customize.left.as_ref() == Some(&whole);
                    if ui
                        .selectable_label(on, egui::RichText::new(self.item_name(&whole)).strong())
                        .clicked()
                    {
                        self.customize.left = Some(whole);
                    }
                    ui.indent(group.name, |ui| {
                        for (id, name) in tools {
                            let item = Item::Tool(id.to_owned());
                            let on = self.customize.left.as_ref() == Some(&item);
                            if ui.selectable_label(on, name).clicked() {
                                self.customize.left = Some(item);
                            }
                        }
                    });
                }
            });
    }

    /// The buttons between: which was pressed. Each is greyed while it can do nothing.
    fn customize_buttons(&self, ui: &mut egui::Ui) -> Option<&'static str> {
        ui.add_space(3.0 * ui.spacing().interact_size.y);
        let mut pressed = None;
        for (button, key) in [
            ("add", "customize-add"),
            ("remove", "customize-remove"),
            ("up", "customize-up"),
            ("down", "customize-down"),
            ("new", "customize-new"),
            ("rename", "customize-rename"),
            ("delete", "customize-delete"),
        ] {
            let can = self.customize_action(button).is_some();
            let response = ui.add_enabled(can, egui::Button::new(tr!(key)));
            crate::marks::note(ui, &format!("button:customize-{button}"), response.rect);
            if response.clicked() {
                pressed = Some(button);
            }
            if matches!(button, "down" | "new") {
                ui.separator();
            }
        }
        pressed
    }

    /// The right: the bars, then the picked bar's items in order.
    fn customize_bars(&mut self, ui: &mut egui::Ui, height: f32) {
        ui.strong(tr!("customize-your-bars"));
        let bars: Vec<usize> = std::iter::once(FAVOURITES_BAR)
            .chain(0..self.options.toolbars.len())
            .collect();
        for bar in bars {
            let on = self.customize.bar == bar;
            if ui.selectable_label(on, self.bar_name(bar)).clicked() && !on {
                self.customize.bar = bar;
                self.customize.right = None;
            }
        }
        // the main bar holds every tool and cannot change
        ui.add_enabled(
            false,
            egui::Button::selectable(false, tr!("customize-main")),
        )
        .on_disabled_hover_text(tr!("customize-main-why"));
        ui.separator();
        ui.strong(self.bar_name(self.customize.bar));
        let items = self.bar_items(self.customize.bar);
        egui::ScrollArea::vertical()
            .id_salt("customize bar")
            .max_height(height)
            .show(ui, |ui| {
                if items.is_empty() {
                    ui.weak(tr!("customize-empty"));
                }
                for (at, item) in items.iter().enumerate() {
                    let on = self.customize.right == Some(at);
                    if ui.selectable_label(on, self.item_name(item)).clicked() {
                        self.customize.right = Some(at);
                    }
                }
            });
    }
}
