//! Toolbars of your own on screen: the same strip of tools and groups wherever a layout puts
//! it (a Ribbon tab, a Rail list, a Classic section, the Canvas palette) or floating over the
//! drawing. A group opens its list of tools as the Canvas palette's do (Options ▸ General ▸
//! Canvas tool lists open); a drag moves an item along its toolbar; a right-click moves it,
//! takes it off, or renames, floats or deletes the toolbar.

use super::installed::GADGETS;
use super::{CutCraftApp, RIBBON_TABS, TOOL_ICON_ROWS, egui, gui};
use crate::custom_toolbars::{self as toolbars, Item};
use crate::options::CanvasFlyouts;
use crate::tool_names::{group_name, page_tab_name};
use cut_craft_bug_report::hint::HoverHint;
use cut_craft_i18n::tr;

/// How a strip lays out its items.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Strip {
    /// Icons in a row, wrapping (Ribbon, Classic, floating).
    Across,
    /// Icons in a column (the Canvas palette).
    Down,
    /// Icon and name, one to a row (Rail's list).
    Rows,
}

/// An item being dragged along a toolbar: the toolbar and where the item was.
#[derive(Clone, Copy, Debug)]
struct Dragged {
    toolbar: usize,
    at: usize,
}

/// What a toolbar has open or in hand while it is shown.
#[derive(Clone, Debug, Default)]
pub(super) struct ToolbarState {
    /// The Ribbon tab showing a toolbar instead of one of `RIBBON_TABS`.
    pub ribbon: Option<usize>,
    /// The Rail list showing a toolbar.
    pub rail: Option<usize>,
    /// The group whose list is open: its toolbar and place on it.
    pub group: Option<(usize, usize)>,
    /// The toolbar being renamed and the name typed so far.
    pub rename: Option<(usize, String)>,
    /// A change to the toolbars picked while they are being drawn, made once the frame's
    /// toolbars are all drawn (so none is deleted or moved under the loop drawing them).
    pending: Option<String>,
}

impl CutCraftApp {
    /// The toolbars shown in the layout, not floating, by index.
    pub(super) fn docked_toolbars(&self) -> Vec<usize> {
        (0..self.options.toolbars.len())
            .filter(|&n| !self.options.toolbars[n].floating)
            .collect()
    }

    /// A tool's or group's right-click menu with Add to Toolbar after what it has already.
    pub(super) fn with_toolbar_menu(
        &self,
        mut menu: Vec<gui::MenuItem>,
        item: &Item,
    ) -> Vec<gui::MenuItem> {
        if !menu.is_empty() {
            menu.push(gui::MenuItem::separator());
        }
        menu.push(toolbars::add_menu(&self.options.toolbars, item));
        menu
    }

    /// A group button's right-click menu: Add to Toolbar for the whole group. The action
    /// picked, if any.
    pub(super) fn group_menu_on(&self, response: &egui::Response, name: &str) -> Option<String> {
        self.popup(response, || {
            self.with_toolbar_menu(vec![], &Item::Group(name.to_owned()))
        })
    }

    /// A toolbar's name with its menu (rename, float, delete) on a right-click.
    pub(super) fn toolbar_name_menu(&mut self, response: &egui::Response, n: usize) {
        if let Some(action) = self.popup(response, || {
            toolbars::toolbar_menu(&self.options.toolbars, n)
        }) {
            self.toolbars.pending = Some(action);
        }
    }

    /// What was picked from a toolbar's menus this frame, now they are all drawn.
    pub(super) fn toolbar_changes(&mut self, ctx: &egui::Context) {
        if let Some(action) = self.toolbars.pending.take() {
            self.invoke(ctx, &action);
        }
    }

    /// The tab row the Ribbon and Canvas share: the built-in tabs (Gadgets once something
    /// is installed), then one tab per toolbar of your own, its name right-clicked for its
    /// menu. Whether a tab was clicked.
    pub(super) fn tab_row(&mut self, ui: &mut egui::Ui) -> bool {
        let gadgets = !self.gadget_tools().is_empty();
        if !gadgets && RIBBON_TABS[self.ribbon_tab] == GADGETS {
            self.ribbon_tab = 0;
        }
        // a toolbar floated away, or deleted, has no tab
        let docked = self.docked_toolbars();
        if self.toolbars.ribbon.is_some_and(|n| !docked.contains(&n)) {
            self.toolbars.ribbon = None;
        }
        let mut clicked = false;
        for (i, tab) in RIBBON_TABS.iter().enumerate() {
            if *tab == GADGETS && !gadgets {
                continue;
            }
            let on = self.toolbars.ribbon.is_none() && self.ribbon_tab == i;
            if ui.selectable_label(on, page_tab_name(tab)).clicked() {
                (self.ribbon_tab, self.toolbars.ribbon) = (i, None);
                clicked = true;
            }
        }
        for n in docked {
            let on = self.toolbars.ribbon == Some(n);
            let response = ui.selectable_label(on, self.options.toolbars[n].name.as_str());
            if response.clicked() {
                self.toolbars.ribbon = Some(n);
                self.toolbars.group = None;
                clicked = true;
            }
            self.toolbar_name_menu(&response, n);
        }
        clicked
    }

    /// The Rail's buttons for the toolbars of your own, one each after Favourites: its
    /// first item's icon, or as much of its name as fits.
    pub(super) fn rail_toolbar_buttons(&mut self, ui: &mut egui::Ui, edge: f32) {
        for n in self.docked_toolbars() {
            let toolbar = &self.options.toolbars[n];
            let name = toolbar.name.clone();
            let icon = toolbar.items.iter().find_map(|item| match item {
                Item::Tool(id) => self.icons.find(self.explicit_icon(id), id),
                Item::Group(group) => self.group_named(group).and_then(|g| {
                    g.tools
                        .iter()
                        .find_map(|&id| self.icons.find(self.explicit_icon(id), id))
                }),
            });
            let button = match icon {
                Some(path) => egui::Button::image(
                    self.icons
                        .image(path)
                        .fit_to_exact_size(egui::vec2(edge, edge)),
                ),
                None => egui::Button::new(name.as_str()).wrap_mode(egui::TextWrapMode::Truncate),
            }
            .min_size(egui::vec2(edge, edge))
            .selected(self.toolbars.rail == Some(n));
            let response = ui.add(button).on_hover_hint(&name);
            if response.clicked() {
                self.toolbars.rail = (self.toolbars.rail != Some(n)).then_some(n);
                self.toolbars.group = None;
                self.rail_group = None;
                (self.rail_favourites, self.rail_gadgets) = (false, false);
            }
            self.toolbar_name_menu(&response, n);
        }
    }

    /// The Rail's open toolbar, if one is open and still docked.
    pub(super) fn rail_toolbar(&mut self) -> Option<usize> {
        let docked = self.docked_toolbars();
        self.toolbars.rail = self.toolbars.rail.filter(|n| docked.contains(n));
        if self.rail_group.is_some() || self.rail_favourites || self.rail_gadgets {
            self.toolbars.rail = None;
        }
        self.toolbars.rail
    }

    /// Classic: each toolbar of your own as a section of the Design tab, after the rest
    /// so nothing above moves for it; its name right-clicked for its menu.
    pub(super) fn toolbar_sections(&mut self, ui: &mut egui::Ui) {
        for n in self.docked_toolbars() {
            ui.separator();
            self.toolbar_title(ui, n);
            self.toolbar_strip(ui, n, Strip::Across);
        }
    }

    /// Canvas: the toolbar whose tab is chosen, in the palette's place, its items one
    /// under another.
    pub(super) fn canvas_toolbar(&mut self, ui: &mut egui::Ui, at: egui::Pos2, frame: egui::Frame) {
        let Some(n) = self.toolbars.ribbon else {
            return;
        };
        egui::Area::new(egui::Id::unique("canvas_toolbar"))
            .default_pos(at)
            .show(ui.ctx(), |ui| {
                frame.show(ui, |ui| {
                    cut_craft_bug_report::mark(ui, "Canvas toolbar");
                    // down to just above the chips; a longer toolbar scrolls
                    let row = ui.spacing().interact_size.y;
                    let room = ui.ctx().content_rect().bottom() - at.y - 3.0 * row;
                    egui::ScrollArea::vertical()
                        .max_height(room.max(row))
                        .show(ui, |ui| self.toolbar_strip(ui, n, Strip::Down));
                });
            });
    }

    /// Toolbar `n`'s name as a heading, right-clicked for its menu.
    pub(super) fn toolbar_title(&mut self, ui: &mut egui::Ui, n: usize) {
        let name = self.options.toolbars[n].name.clone();
        let title = ui.add(
            egui::Label::new(egui::RichText::new(name).strong())
                .selectable(false)
                .sense(egui::Sense::click()),
        );
        self.toolbar_name_menu(&title, n);
    }

    /// A toolbar action (`toolbar:…` or `toolbar_rename:…`), kept in the options at once.
    pub(super) fn toolbar_action(&mut self, action: &str) {
        if let Some(n) = action.strip_prefix(toolbars::RENAME) {
            if let Some(toolbar) = n
                .parse::<usize>()
                .ok()
                .and_then(|n| self.options.toolbars.get(n).map(|t| (n, t.name.clone())))
            {
                self.toolbars.rename = Some(toolbar);
            }
            return;
        }
        let Some(action) = action.strip_prefix(toolbars::ACTION) else {
            return;
        };
        let before = self.options.toolbars.len();
        if !toolbars::apply(&mut self.options.toolbars, action) {
            return;
        }
        let after = self.options.toolbars.len();
        if after > before {
            // a new toolbar shows at once where the layout shows toolbars
            self.toolbars.ribbon = Some(after - 1);
        }
        // what was open on a toolbar that went, or moved, closes
        let state = &mut self.toolbars;
        state.group = None;
        let shown = |n: Option<usize>| n.filter(|&n| n < after);
        state.ribbon = shown(state.ribbon);
        state.rail = shown(state.rail);
        if after < before {
            (state.ribbon, state.rail) = (None, None);
        }
        self.save_options();
    }

    /// Toolbar `n`'s items as `strip` lays them out: a click on a tool runs it, a group
    /// opens its list, a drag moves an item along, a right-click offers the rest. Whether
    /// a tool was picked.
    pub(super) fn toolbar_strip(&mut self, ui: &mut egui::Ui, n: usize, strip: Strip) -> bool {
        let Some(items) = self.options.toolbars.get(n).map(|t| t.items.clone()) else {
            return false;
        };
        if items.is_empty() {
            ui.weak(tr!("toolbars-empty"));
            return false;
        }
        let edge = TOOL_ICON_ROWS * ui.spacing().interact_size.y;
        let buttons = match strip {
            Strip::Across => {
                ui.horizontal_wrapped(|ui| self.strip_buttons(ui, &items, strip, edge))
                    .inner
            }
            Strip::Down | Strip::Rows => {
                ui.vertical(|ui| self.strip_buttons(ui, &items, strip, edge))
                    .inner
            }
        };
        let mut picked = None;
        let mut action = None;
        for (at, response) in &buttons {
            let at = *at;
            match &items[at] {
                Item::Tool(id) => {
                    if response.clicked() {
                        picked = Some(id.clone());
                    }
                    let menu = self.popup(response, || {
                        let mut menu = self.row_menu(id);
                        menu.push(gui::MenuItem::separator());
                        menu.extend(toolbars::item_menu(&self.options.toolbars, n, at));
                        menu
                    });
                    action = action.or(menu);
                }
                Item::Group(_) => {
                    self.open_group_on(response, (n, at));
                    let menu = self.popup(response, || {
                        toolbars::item_menu(&self.options.toolbars, n, at)
                    });
                    action = action.or(menu);
                }
            }
            action = action.or_else(|| Self::drag_along(ui, response, n, at, strip));
        }
        let listed = self.open_group_list(ui, n, &items, &buttons, strip);
        if let Some(action) = action {
            if action.starts_with(toolbars::ACTION) || action.starts_with(toolbars::RENAME) {
                self.toolbars.pending = Some(action);
            } else {
                self.invoke(ui.ctx(), &action);
            }
        }
        if let Some(id) = picked {
            self.toolbars.group = None;
            let kind = self.tool_model().kind_of(&id);
            self.pick(ui.ctx(), kind, &id);
            return true;
        }
        listed
    }

    /// Each item's button, with where it is on the toolbar; a tool this machine lacks, or a
    /// group or tool that is gone, is left out.
    fn strip_buttons(
        &self,
        ui: &mut egui::Ui,
        items: &[Item],
        strip: Strip,
        edge: f32,
    ) -> Vec<(usize, egui::Response)> {
        let mut buttons = vec![];
        for (at, item) in items.iter().enumerate() {
            let response = match item {
                Item::Tool(id) => {
                    if !self.offered(id) || !self.is_tool(id) {
                        continue;
                    }
                    if strip == Strip::Rows {
                        let label = self.tool_label(id);
                        let button = match self.icons.find(self.explicit_icon(id), id) {
                            Some(path) => egui::Button::image_and_text(
                                self.icons
                                    .image(path)
                                    .fit_to_exact_size(egui::vec2(edge, edge)),
                                &label,
                            ),
                            None => egui::Button::new(&label),
                        }
                        .selected(self.status.current_tool == label);
                        let response = ui.add_enabled(self.available(id), button);
                        self.with_guidance(response, id)
                            .on_disabled_hover_text(self.unavailable_why(id))
                    } else {
                        self.tool_button(ui, id, edge)
                    }
                }
                Item::Group(name) => {
                    let Some(group) = self.group_named(name) else {
                        continue;
                    };
                    let icon = group
                        .tools
                        .iter()
                        .find_map(|&id| self.icons.find(self.explicit_icon(id), id));
                    let label = group_name(group.name);
                    let text = if strip == Strip::Rows {
                        format!("{label} ▸")
                    } else {
                        "▾".to_owned()
                    };
                    let button = match icon {
                        Some(path) => egui::Button::image_and_text(
                            self.icons
                                .image(path)
                                .fit_to_exact_size(egui::vec2(edge, edge)),
                            text,
                        ),
                        None => egui::Button::new(super::layouts::group_label(name)),
                    }
                    .selected(self.toolbars.group.is_some_and(|(_, a)| a == at));
                    ui.add(button).on_hover_hint(&label)
                }
            };
            // each can be dragged along its toolbar as well as clicked
            buttons.push((at, response.interact(egui::Sense::drag())));
        }
        buttons
    }

    /// Whether `id` is one of the tools (so a toolbar keeps working when one is removed).
    fn is_tool(&self, id: &str) -> bool {
        self.tool_model()
            .all_groups()
            .iter()
            .any(|g| g.tools.contains(&id))
            || self.gadget_tools().contains(&id)
    }

    /// The tool group named `name` (in English), if there still is one.
    pub(super) fn group_named(&self, name: &str) -> Option<gui::ToolGroup> {
        self.tool_model()
            .all_groups()
            .into_iter()
            .find(|g| g.name == name)
    }

    /// A group's button: pointing at it opens its list, or clicking does (Options ▸
    /// General ▸ Canvas tool lists open).
    fn open_group_on(&mut self, response: &egui::Response, group: (usize, usize)) {
        let hover = self.options.general.canvas_flyouts == CanvasFlyouts::OnHover;
        if hover && response.hovered() {
            self.toolbars.group = Some(group);
        } else if response.clicked() {
            let open = self.toolbars.group == Some(group);
            self.toolbars.group = (hover || !open).then_some(group);
        }
    }

    /// The open group's list of tools beside its button, if it is on toolbar `n`; on
    /// hover it closes once the pointer leaves both. Whether a tool was picked from it.
    fn open_group_list(
        &mut self,
        ui: &egui::Ui,
        n: usize,
        items: &[Item],
        buttons: &[(usize, egui::Response)],
        strip: Strip,
    ) -> bool {
        let Some((toolbar, at)) = self.toolbars.group else {
            return false;
        };
        if toolbar != n {
            return false;
        }
        let (Some(Item::Group(name)), Some((_, button))) =
            (items.get(at), buttons.iter().find(|(a, _)| *a == at))
        else {
            self.toolbars.group = None;
            return false;
        };
        let Some(group) = self.group_named(name) else {
            return false;
        };
        let tools: Vec<&'static str> = group
            .tools
            .iter()
            .copied()
            .filter(|id| self.offered(id))
            .collect();
        let gap = ui.spacing().item_spacing.x;
        let at_pos = if strip == Strip::Across {
            button.rect.left_bottom() + egui::vec2(0.0, gap)
        } else {
            button.rect.right_top() + egui::vec2(gap, 0.0)
        };
        let mut picked = false;
        let shown = egui::Area::new(ui.scope_id().with(("toolbar_group", n)))
            .order(egui::Order::Foreground)
            .fixed_pos(at_pos)
            .show(ui.ctx(), |ui| {
                egui::Frame::popup(ui.style()).show(ui, |ui| {
                    cut_craft_bug_report::mark(ui, "Toolbar group");
                    ui.strong(group_name(group.name));
                    ui.separator();
                    picked = self.tool_rows(ui, &tools);
                });
            });
        let hover = self.options.general.canvas_flyouts == CanvasFlyouts::OnHover;
        let near = |rect: egui::Rect, p: egui::Pos2| rect.expand(2.0 * gap).contains(p);
        if picked
            || (hover
                && ui.input(|i| i.pointer.delta() != egui::Vec2::ZERO)
                && ui
                    .ctx()
                    .pointer_hover_pos()
                    .is_some_and(|p| !near(button.rect, p) && !near(shown.response.rect, p))
                && !egui::Popup::is_any_open(ui.ctx()))
        {
            self.toolbars.group = None;
        }
        picked
    }

    /// A drag of item `at` along toolbar `n`: a line shows where it will go, and letting go
    /// there moves it. The move, when it is let go.
    fn drag_along(
        ui: &egui::Ui,
        response: &egui::Response,
        n: usize,
        at: usize,
        strip: Strip,
    ) -> Option<String> {
        response.dnd_set_drag_payload(Dragged { toolbar: n, at });
        if response.dragged() {
            ui.ctx().set_cursor_icon(egui::CursorIcon::Grabbing);
        }
        let over = |d: &Dragged| d.toolbar == n && d.at != at;
        if response
            .dnd_hover_payload::<Dragged>()
            .is_some_and(|d| over(&d))
        {
            let stroke = egui::Stroke::new(2.0, ui.visuals().selection.bg_fill);
            let rect = response.rect;
            if strip == Strip::Across {
                ui.painter()
                    .vline(rect.left() - 1.0, rect.y_range(), stroke);
            } else {
                ui.painter().hline(rect.x_range(), rect.top() - 1.0, stroke);
            }
        }
        let dropped = response.dnd_release_payload::<Dragged>()?;
        // the line is drawn before this item, and the move takes the dragged one out first,
        // so one dragged from before it goes in one place earlier
        let to = if dropped.at < at { at - 1 } else { at };
        over(&dropped).then(|| format!("{}move:{n}:{}:{to}", toolbars::ACTION, dropped.at))
    }

    /// The floating toolbars, each a small strip under its name that can be dragged
    /// anywhere over the window and stays where it is left.
    pub(super) fn floating_toolbars(&mut self, ctx: &egui::Context) {
        for n in 0..self.options.toolbars.len() {
            let toolbar = &self.options.toolbars[n];
            if !toolbar.floating {
                continue;
            }
            let name = toolbar.name.clone();
            let start = toolbar.at.map_or_else(
                || ctx.content_rect().center() - egui::vec2(100.0, 0.0),
                |[x, y]| egui::pos2(x, y),
            );
            let shown = egui::Area::new(egui::Id::unique(("floating_toolbar", n, name.as_str())))
                .default_pos(start)
                .constrain(true)
                .show(ctx, |ui| {
                    egui::Frame::window(ui.style())
                        .inner_margin(ui.spacing().item_spacing.y)
                        .show(ui, |ui| {
                            cut_craft_bug_report::mark(ui, "Floating toolbar");
                            ui.set_max_width(12.0 * ui.spacing().interact_size.y);
                            let title = ui
                                .add(
                                    egui::Label::new(egui::RichText::new(&name).small().strong())
                                        .selectable(false)
                                        .sense(egui::Sense::click()),
                                )
                                .on_hover_hint(tr!("toolbars-floating-hint"));
                            self.toolbar_name_menu(&title, n);
                            self.toolbar_strip(ui, n, Strip::Across);
                        });
                });
            let left = shown.response.rect.left_top();
            let moved = self.options.toolbars.get(n).is_some_and(|t| {
                t.at.is_none_or(|[x, y]| (x - left.x).abs() + (y - left.y).abs() > 0.5)
            });
            if moved && !ctx.input(|i| i.pointer.any_down()) {
                if let Some(toolbar) = self.options.toolbars.get_mut(n) {
                    toolbar.at = Some([left.x, left.y]);
                }
                self.save_options();
            }
        }
    }

    /// The box naming a toolbar, while one is being renamed.
    pub(super) fn toolbar_rename_window(&mut self, ctx: &egui::Context) {
        let Some((n, mut name)) = self.toolbars.rename.take() else {
            return;
        };
        let mut done = None;
        let modal = egui::Modal::new(egui::Id::unique("toolbar_rename")).show(ctx, |ui| {
            cut_craft_bug_report::mark(ui, "Rename toolbar");
            ui.heading(tr!("toolbars-rename"));
            let field = ui.text_edit_singleline(&mut name);
            field.request_focus();
            let enter = ui.input(|i| i.key_pressed(egui::Key::Enter));
            ui.horizontal(|ui| {
                if ui.button(tr!("toolbars-rename-ok")).clicked() || enter {
                    done = Some(true);
                }
                if ui.button(tr!("toolbars-rename-cancel")).clicked() {
                    done = Some(false);
                }
            });
        });
        if done.is_none() && modal.should_close() {
            done = Some(false);
        }
        match done {
            Some(true) => {
                let action = format!("{}rename:{n}:{name}", toolbars::ACTION);
                self.toolbar_action(&action);
            }
            Some(false) => {}
            None => self.toolbars.rename = Some((n, name)),
        }
    }
}
