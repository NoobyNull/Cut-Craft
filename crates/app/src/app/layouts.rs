//! The Rail and Ribbon window layouts, and the Find Tool window and inspector every layout
//! can use (`docs/superpowers/specs/2026-10-27-window-layout-setting-design.md`).

use super::installed::GADGETS;
use super::{
    Align, CutCraftApp, INSPECTOR_TABS, Layout, Page, RIBBON_TABS, TOOL_ICON_ROWS, egui, gui,
    side_panel,
};
use crate::options::WindowLayout;
use crate::tool_model::{Place, ToolModel};
use crate::tool_names::{group_name, page_tab_name};
use cut_craft_bug_report::hint::HoverHint;
use cut_craft_i18n::tr;

/// View ▸ Window Layout's items and the layouts they pick.
const LAYOUTS: [(&str, WindowLayout); 3] = [
    ("layout_ribbon", WindowLayout::Ribbon),
    ("layout_rail", WindowLayout::Rail),
    ("layout_classic", WindowLayout::Classic),
];

impl CutCraftApp {
    /// Whether View ▸ Window Layout's item `id` is the layout in use; `None` for any other
    /// item.
    pub(super) fn layout_checked(&self, id: &str) -> Option<bool> {
        LAYOUTS
            .iter()
            .find(|(item, _)| *item == id)
            .map(|&(_, layout)| self.options.general.layout == layout)
    }

    /// View ▸ Window Layout: the window rearranges next frame; the choice is saved.
    pub(super) fn set_layout(&mut self, id: &str) {
        let Some(&(_, layout)) = LAYOUTS.iter().find(|(item, _)| *item == id) else {
            return;
        };
        self.options.general.layout = layout;
        self.save_options();
    }

    /// Writes the options where they are kept, if anywhere.
    pub(super) fn save_options(&mut self) {
        if let Some(path) = &self.options_path
            && let Err(e) = self.options.save_to(path)
        {
            self.status.set_message(&e);
        }
    }

    /// The tools every layout shows, read the same way by each ([`crate::tool_model`]).
    pub(super) fn tool_model(&self) -> ToolModel<'_> {
        ToolModel::new(&self.toolbar)
    }

    /// The icon a tool names: its command's, or a built-in tool's own (empty when its icon
    /// file is just named for its id).
    pub(super) fn explicit_icon(&self, id: &str) -> &'static str {
        self.registry
            .get(id)
            .map(|c| c.meta().icon)
            .or_else(|| gui::built_in_icon(id))
            .unwrap_or("")
    }

    /// A tool's name: the command's label, or its id in words.
    pub(super) fn tool_label(&self, id: &str) -> String {
        crate::tool_names::name_of(&self.registry, id)
    }

    /// The available tools offered on the active machine whose names contain `text` (any case), in group order.
    pub(super) fn search_matches(&self, text: &str) -> Vec<(&'static str, String)> {
        let text = text.trim().to_lowercase();
        self.tool_model()
            .all_groups()
            .iter()
            .flat_map(|g| g.tools.iter().copied())
            .filter(|id| self.available(id) && self.offered(id))
            .map(|id| (id, self.tool_label(id)))
            .filter(|(_, label)| label.to_lowercase().contains(&text))
            .collect()
    }

    /// Find Tool (Ctrl+K): a window over the view listing the tools whose names contain the
    /// text typed; a click runs one, Enter the first, Escape (in `shortcuts`) closes.
    pub(super) fn tool_search(&mut self, ctx: &egui::Context) {
        let Some(mut text) = self.search.clone() else {
            return;
        };
        let matches = self.search_matches(&text);
        let (mut open, mut picked) = (true, None);
        let row = ctx.global_style().spacing.interact_size.y;
        egui::Window::new(tr!("layouts-find-tool"))
            .id(egui::Id::unique("find_tool"))
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_TOP, egui::vec2(0.0, 2.0 * row))
            .open(&mut open)
            .show(ctx, |ui| {
                cut_craft_bug_report::mark(ui, "Find Tool");
                let edit = ui.add(
                    egui::TextEdit::singleline(&mut text)
                        .hint_text(tr!("layouts-part-of-a-tool-s")),
                );
                // the field takes the keyboard when the window opens
                if ui.memory(egui::Memory::focused).is_none() {
                    edit.request_focus();
                }
                if edit.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                    picked = matches.first().map(|(id, _)| *id);
                }
                ui.separator();
                if matches.is_empty() {
                    ui.weak(tr!("layouts-no-tool-has-that-in"));
                }
                egui::ScrollArea::vertical()
                    .max_height(10.0 * row)
                    .show(ui, |ui| {
                        for (id, label) in &matches {
                            if ui.button(label).clicked() {
                                picked = Some(*id);
                            }
                        }
                    });
            });
        // Escape closes it (and only it) from `shortcuts`
        if let Some(id) = picked {
            self.search = None;
            self.invoke(ctx, id);
        } else {
            self.search = open.then_some(text);
        }
    }

    /// The right panel of the Rail and Ribbon layouts: the open form with its buttons,
    /// else the browser.
    pub(super) fn inspector(&mut self, ui: &mut egui::Ui) {
        cut_craft_bug_report::mark(ui, "Inspector");
        if self.form.is_some() {
            // the form draws its own Apply and Close, pinned at the foot
            self.form_panel(ui);
            return;
        }
        self.tool_note_label(ui);
        self.browser(ui);
    }

    /// Sheets, layers, components, clipart and the toolpath list under a row of tabs.
    pub(super) fn browser(&mut self, ui: &mut egui::Ui) {
        cut_craft_bug_report::mark(ui, "Browser");
        ui.horizontal_wrapped(|ui| {
            for (i, tab) in INSPECTOR_TABS.iter().enumerate() {
                ui.selectable_value(&mut self.inspector_tab, i, page_tab_name(tab));
            }
        });
        ui.separator();
        match INSPECTOR_TABS[self.inspector_tab] {
            "Toolpaths" => self.toolpath_list(ui, false),
            tab => {
                egui::ScrollArea::vertical().show(ui, |ui| match tab {
                    "Sheets" => self.sheets_panel(ui),
                    "Layers" => self.layers_panel(ui),
                    "Components" => self.component_list(ui),
                    // the Classic Toolpaths page's Analyse tab: what would stop or spoil
                    // the cut, before Save is pressed
                    "Checks" => self.analyse(ui),
                    _ => self.clipart_panel(ui),
                });
            }
        }
    }

    /// A Rail row's right-click menu, as every layout gives a tool in a list by name.
    pub(super) fn row_menu(&self, id: &str) -> Vec<gui::MenuItem> {
        self.tool_model()
            .menu(id, Place::Row, &self.options.favourites, false)
    }

    /// Tools by icon and name, one per row, each with its right-click menu; whether one was
    /// picked.
    pub(super) fn tool_rows(&mut self, ui: &mut egui::Ui, tools: &[&'static str]) -> bool {
        let edge = ui.spacing().interact_size.y;
        let mut clicked = None;
        let mut chosen = None;
        for &id in tools {
            let label = self.tool_label(id);
            let explicit = self.explicit_icon(id);
            let button = match self.icons.find(explicit, id) {
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
            let response = self
                .with_guidance(response, id)
                .on_disabled_hover_text(self.unavailable_why(id));
            if response.clicked() {
                clicked = Some(id);
            }
            if let Some(action) = self.popup(&response, || self.row_menu(id)) {
                chosen = Some(action);
            }
        }
        if let Some(action) = chosen {
            self.invoke(ui.ctx(), &action);
        }
        if let Some(id) = clicked {
            self.pick(ui.ctx(), self.tool_model().kind_of(id), id);
        }
        clicked.is_some()
    }

    /// Rail: a column of group icons on the left opening a list of that group's tools
    /// beside it; the inspector on the right.
    pub(super) fn rail_layout(&mut self, ui: &mut egui::Ui) {
        let style = ui.ctx().global_style();
        let spacing = &style.spacing;
        let margin = spacing.item_spacing.y;
        let edge = TOOL_ICON_ROWS * spacing.interact_size.y;
        let groups = self.tool_model().all_groups();
        let gadgets = self.gadget_tools();
        // what was installed can be removed while its list is open
        self.rail_gadgets &= !gadgets.is_empty();
        egui::Panel::left("rail")
            .resizable(false)
            .exact_size(edge + 2.0 * (spacing.button_padding.x + margin))
            .frame(egui::Frame::side_top_panel(&style).inner_margin(margin))
            .show(ui, |ui| {
                cut_craft_bug_report::mark(ui, "Rail");
                egui::ScrollArea::vertical()
                    .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysHidden)
                    .show(ui, |ui| {
                        for (i, group) in groups.iter().enumerate() {
                            // the first of the group's tools with an icon stands for it;
                            // a group with none shows as much of its name as fits
                            let icon = group.tools.iter().find_map(|&id| {
                                let explicit = self.explicit_icon(id);
                                self.icons.find(explicit, id)
                            });
                            let name = group_name(group.name);
                            let button = match icon {
                                Some(path) => egui::Button::image(
                                    self.icons
                                        .image(path)
                                        .fit_to_exact_size(egui::vec2(edge, edge)),
                                ),
                                None => egui::Button::new(name.as_str())
                                    .wrap_mode(egui::TextWrapMode::Truncate),
                            }
                            .selected(self.rail_group == Some(i));
                            if ui.add(button).on_hover_hint(&name).clicked() {
                                self.rail_group = (self.rail_group != Some(i)).then_some(i);
                                (self.rail_favourites, self.rail_gadgets) = (false, false);
                            }
                        }
                        // after the groups, so none of their buttons moves for it
                        let star = egui::Button::new(egui::RichText::new("★").size(0.8 * edge))
                            .min_size(egui::vec2(edge, edge))
                            .selected(self.rail_favourites);
                        if ui
                            .add(star)
                            .on_hover_hint(tr!("layouts-favourites-the-tools-you-keep"))
                            .clicked()
                        {
                            self.rail_favourites = !self.rail_favourites;
                            self.rail_group = None;
                            self.rail_gadgets = false;
                        }
                        // last of all, once something is installed
                        self.rail_gadgets_button(ui, &gadgets, edge);
                    });
            });
        let list = if self.rail_favourites {
            Some((
                "Favourites",
                self.tool_model().every_favourite(&self.options.favourites),
            ))
        } else if self.rail_gadgets {
            Some((GADGETS, gadgets))
        } else {
            self.rail_group
                .and_then(|i| groups.get(i))
                .map(|g| (g.name, g.tools.to_vec()))
        };
        if let Some((name, tools)) = list {
            side_panel(egui::Panel::left("rail_flyout"), ui.ctx()).show(ui, |ui| {
                cut_craft_bug_report::mark(ui, "Rail flyout");
                ui.horizontal(|ui| {
                    ui.strong(group_name(name));
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        let row = ui.spacing().interact_size.y;
                        if ui
                            .add(self.pin_button(self.rail_pinned, row))
                            .on_hover_hint(tr!("layouts-keep-this-list-open-after"))
                            .clicked()
                        {
                            self.rail_pinned = !self.rail_pinned;
                        }
                    });
                });
                ui.separator();
                let picked = egui::ScrollArea::vertical()
                    .show(ui, |ui| {
                        if tools.is_empty() {
                            ui.weak(NO_FAVOURITES);
                        }
                        let picked = self.tool_rows(ui, &tools);
                        // the hovered tool's guidance under its list, as under Classic's icons
                        ui.add_space(ui.spacing().item_spacing.y);
                        self.tip_contents(ui);
                        picked
                    })
                    .inner;
                if picked && !self.rail_pinned {
                    self.rail_group = None;
                    (self.rail_favourites, self.rail_gadgets) = (false, false);
                }
            });
        }
        side_panel(egui::Panel::right("inspector"), ui.ctx()).show(ui, |ui| self.inspector(ui));
    }

    /// The Rail's last button, after Favourites, once something is installed: opens the
    /// list of the installed gadgets and add-ons' tools.
    fn rail_gadgets_button(&mut self, ui: &mut egui::Ui, gadgets: &[&'static str], edge: f32) {
        if gadgets.is_empty() {
            return;
        }
        let icon = gadgets
            .iter()
            .find_map(|&id| self.icons.find(self.explicit_icon(id), id));
        let button = match icon {
            Some(path) => egui::Button::image(
                self.icons
                    .image(path)
                    .fit_to_exact_size(egui::vec2(edge, edge)),
            ),
            None => egui::Button::new(GADGETS).wrap_mode(egui::TextWrapMode::Truncate),
        }
        .selected(self.rail_gadgets);
        if ui
            .add(button)
            .on_hover_hint(tr!("layouts-gadgets-the-gadgets-and-add"))
            .clicked()
        {
            self.rail_gadgets = !self.rail_gadgets;
            (self.rail_group, self.rail_favourites) = (None, false);
        }
    }

    /// The ribbon tab's groups, and the page their tools count as picked from.
    pub(super) fn ribbon_groups(&self) -> (Vec<gui::ToolGroup>, Page) {
        let t = &self.toolbar;
        match RIBBON_TABS[self.ribbon_tab] {
            "Model" => (t.modeling_groups.clone(), Page::Design),
            "Toolpaths" => (t.toolpath_groups.clone(), Page::Toolpaths),
            _ => (t.groups.clone(), Page::Design),
        }
    }

    /// Ribbon: tabs of tool groups across the top, each group's icons in two rows; the
    /// browser on the left and, while a form is open, the inspector on the right.
    pub(super) fn ribbon_layout(&mut self, ui: &mut egui::Ui) {
        let spacing = ui.spacing().clone();
        let edge = TOOL_ICON_ROWS * spacing.interact_size.y;
        egui::Panel::top("ribbon").show(ui, |ui| {
            cut_craft_bug_report::mark(ui, "Ribbon");
            let gadgets = self.gadget_tools();
            // Gadgets is the last tab, there once something is installed
            if gadgets.is_empty() && RIBBON_TABS[self.ribbon_tab] == GADGETS {
                self.ribbon_tab = 0;
            }
            ui.horizontal(|ui| {
                for (i, tab) in RIBBON_TABS.iter().enumerate() {
                    if *tab != GADGETS || !gadgets.is_empty() {
                        ui.selectable_value(&mut self.ribbon_tab, i, page_tab_name(tab));
                    }
                }
            });
            ui.separator();
            match RIBBON_TABS[self.ribbon_tab] {
                "Favourites" => {
                    self.ribbon_favourites(ui, edge);
                    return;
                }
                GADGETS => {
                    self.tool_sections(ui, &[(GADGETS, gadgets)], Page::Design, false);
                    return;
                }
                _ => {}
            }
            let (groups, page) = self.ribbon_groups();
            self.ribbon_rows(ui, &groups, page, edge);
        });
        side_panel(egui::Panel::left("browser"), ui.ctx()).show(ui, |ui| {
            // the hovered tool's guidance at the browser's foot, a fixed height so the
            // browser above never moves as it changes
            let row = ui.spacing().interact_size.y;
            egui::Panel::bottom("ribbon_tip")
                .resizable(false)
                .exact_size(RIBBON_TIP_ROWS * row)
                .show(ui, |ui| {
                    egui::ScrollArea::vertical()
                        .id_salt("ribbon tip")
                        .auto_shrink(false)
                        .show(ui, |ui| self.tip_contents(ui));
                });
            self.browser(ui);
        });
        if self.form.is_some() {
            side_panel(egui::Panel::right("inspector"), ui.ctx()).show(ui, |ui| self.inspector(ui));
        }
    }

    /// Ribbon ▸ a group's list ▸ a pin: `on:<id>` puts the tool on the Ribbon, `off:<id>`
    /// takes it off; kept in the options.
    pub(super) fn set_ribbon_pin(&mut self, action: &str) {
        let (on, id) = match action.split_once(':') {
            Some(("on", id)) => (true, id),
            Some(("off", id)) => (false, id),
            _ => return,
        };
        self.options.ribbon_pins.insert(id.to_owned(), on);
        self.save_options();
    }

    /// A Ribbon tab's groups, as many to a row as fit and the rest on further rows, in order
    /// and each group whole. Each group is its icons on a tight grid two high, as wide as
    /// its name under them; it shows as many of its tools as fill that width (two columns
    /// at least) until pinned or unpinned by hand. A group with none of its tools on this
    /// machine is left off.
    fn ribbon_rows(&mut self, ui: &mut egui::Ui, groups: &[gui::ToolGroup], page: Page, edge: f32) {
        let spacing = ui.spacing().clone();
        let font = egui::TextStyle::Body.resolve(ui.style());
        let room = ui.available_width();
        let cell = super::panels::tool_cell_size(&spacing, edge);
        let step = cell.x + spacing.item_spacing.x;
        // the line after each group, and the room either side of it
        let gap = 2.0 * spacing.item_spacing.x + 1.0;
        // a group with no tool this machine has (Laser without a laser) is left off
        let groups: Vec<gui::ToolGroup> = groups
            .iter()
            .copied()
            .filter(|group| group.tools.iter().any(|id| self.offered(id)))
            .collect();
        let plans: Vec<(Vec<&'static str>, f32)> = groups
            .iter()
            .map(|group| {
                let name = ui
                    .painter()
                    .layout_no_wrap(
                        group_label(group.name),
                        font.clone(),
                        egui::Color32::PLACEHOLDER,
                    )
                    .size()
                    .x
                    + 2.0 * spacing.button_padding.x;
                // compact: as many columns as fit under the name; every tool when asked
                let starter = if self.options.general.ribbon_icons
                    == crate::options::RibbonIcons::EveryTool
                {
                    usize::MAX
                } else {
                    2 * starter_columns(name, step)
                };
                let offered: Vec<_> = group
                    .tools
                    .iter()
                    .copied()
                    .filter(|id| self.offered(id))
                    .collect();
                let shown = on_ribbon(&offered, &self.options.ribbon_pins, starter);
                let across = shown.len().div_ceil(2);
                let icons = f32::from(u16::try_from(across).unwrap_or(u16::MAX)) * step
                    - spacing.item_spacing.x;
                (shown, icons.max(name))
            })
            .collect();
        let widths: Vec<f32> = plans.iter().map(|(_, w)| w + gap).collect();
        // two rows of icons, the same height in every group, so the names line up
        let icons_height = 2.0 * (cell.y + spacing.item_spacing.y);
        // never more than 40% of the window, so very large text never pushes the job
        // view away: further rows scroll
        let most = 0.4 * ui.ctx().content_rect().height();
        // inside a top panel the area otherwise starts short: size it to its rows
        let rows = f32::from(u16::try_from(wrap_rows(&widths, room).len()).unwrap_or(u16::MAX));
        let row = icons_height + spacing.interact_size.y + 3.0 * spacing.item_spacing.y + 1.0;
        egui::ScrollArea::vertical()
            .max_height(most)
            .min_scrolled_height((rows * row).min(most))
            .auto_shrink([false, true])
            .show(ui, |ui| {
                self.ribbon_wrapped(
                    ui,
                    &groups,
                    &plans,
                    &widths,
                    room,
                    (gap, icons_height),
                    page,
                    edge,
                );
            });
    }

    /// The rows of [`Self::ribbon_rows`]: each group whole, a line after each.
    #[allow(clippy::too_many_arguments)]
    fn ribbon_wrapped(
        &mut self,
        ui: &mut egui::Ui,
        groups: &[gui::ToolGroup],
        plans: &[(Vec<&'static str>, f32)],
        widths: &[f32],
        room: f32,
        (gap, icons_height): (f32, f32),
        page: Page,
        edge: f32,
    ) {
        let spacing = ui.spacing().clone();
        let line = ui.visuals().widgets.noninteractive.bg_stroke;
        for (n, row) in wrap_rows(widths, room).into_iter().enumerate() {
            if n > 0 {
                ui.separator();
            }
            let mut ends = vec![];
            let drawn = ui.horizontal_top(|ui| {
                for i in row {
                    let (shown, width) = &plans[i];
                    let group = ui.vertical(|ui| {
                        ui.set_width(*width);
                        self.ribbon_group(ui, groups[i], shown, page, edge, icons_height);
                    });
                    ends.push(group.response.rect.right() + gap / 2.0);
                    ui.add_space(gap - spacing.item_spacing.x);
                }
            });
            // a line after each group, as tall as the row and no taller
            let rect = drawn.response.rect;
            for x in ends {
                ui.painter().vline(x, rect.y_range(), line);
            }
        }
    }

    /// One Ribbon group: its pinned tools' icons two high, and under them its name, which
    /// lists every tool in it by icon and name, each with a pin that puts it on the Ribbon
    /// or takes it off.
    fn ribbon_group(
        &mut self,
        ui: &mut egui::Ui,
        group: gui::ToolGroup,
        shown: &[&'static str],
        page: Page,
        edge: f32,
        icons_height: f32,
    ) {
        let mut clicked = None;
        let mut chosen = None;
        let across = shown.len().div_ceil(2).max(1);
        let top = ui.cursor().min.y;
        for row in shown.chunks(across) {
            ui.horizontal(|ui| {
                for &id in row {
                    let response = self.tool_button(ui, id, edge);
                    if response.clicked() {
                        clicked = Some(id);
                    }
                    if let Some(action) = self.tool_menu_on(&response, id, false, false) {
                        chosen = Some(action);
                    }
                }
            });
        }
        let used = ui.cursor().min.y - top;
        ui.add_space((icons_height - used).max(0.0));
        let response = ui.menu_button(group_label(group.name), |ui| {
            let row = ui.spacing().interact_size.y;
            let mut pin = None;
            egui::Grid::new(("ribbon list", group.name))
                .num_columns(2)
                .show(ui, |ui| {
                    let listed: Vec<&'static str> = group
                        .tools
                        .iter()
                        .copied()
                        .filter(|id| self.offered(id))
                        .collect();
                    for id in listed {
                        if self.tool_rows(ui, &[id]) {
                            ui.close();
                        }
                        let pinned = shown.contains(&id);
                        if ui
                            .add(self.pin_button(pinned, row))
                            .on_hover_hint(tr!(if pinned {
                                "layouts-ribbon-unpin"
                            } else {
                                "layouts-ribbon-pin"
                            }))
                            .clicked()
                        {
                            pin = Some(if pinned { "off:" } else { "on:" }.to_owned() + id);
                        }
                        ui.end_row();
                    }
                });
            ui.separator();
            ui.weak(tr!("layouts-ribbon-pins-note"));
            if let Some(pin) = pin {
                self.invoke(ui.ctx(), &[gui::RIBBON_PIN, &pin].concat());
            }
        });
        response.response.on_hover_hint(tr!(
            "layouts-ribbon-group-list",
            group = group_name(group.name)
        ));
        if let Some(action) = chosen {
            self.invoke(ui.ctx(), &action);
        }
        if let Some(id) = clicked {
            self.pick(ui.ctx(), page, id);
        }
    }

    /// The Ribbon's Favourites tab: each page's favourites as a group of icons, in the order
    /// picked; a right-click moves one along them or takes it off.
    fn ribbon_favourites(&mut self, ui: &mut egui::Ui, edge: f32) {
        let pages = [
            (Page::Design, "Design Favourites"),
            (Page::Toolpaths, "Toolpath Favourites"),
        ];
        let lists: Vec<_> = pages
            .into_iter()
            .map(|(page, name)| {
                (
                    page,
                    name,
                    self.tool_model().favourites(page, &self.options.favourites),
                )
            })
            .filter(|(_, _, tools)| !tools.is_empty())
            .collect();
        if lists.is_empty() {
            ui.weak(NO_FAVOURITES);
            return;
        }
        let spacing = ui.spacing().clone();
        let cell = edge + 2.0 * spacing.button_padding.x + spacing.item_spacing.x;
        egui::ScrollArea::horizontal()
            .id_salt("ribbon favourites")
            .show(ui, |ui| {
                ui.horizontal_top(|ui| {
                    for (page, name, tools) in lists {
                        ui.vertical(|ui| {
                            let columns =
                                u16::try_from(tools.len().div_ceil(2).max(2)).unwrap_or(u16::MAX);
                            ui.set_max_width(f32::from(columns) * cell);
                            self.tool_sections(ui, &[(name, tools)], page, true);
                        });
                        ui.separator();
                    }
                });
            });
    }
}

/// What a favourites list says while it has none, and how to add one.
const NO_FAVOURITES: &str = "No favourites yet: right-click a tool ▸ Add to Favourites";

/// How many rows of text the Ribbon keeps for the tool tip at the browser's foot.
const RIBBON_TIP_ROWS: f32 = 6.0;

/// A Ribbon group's name under its icons, with the arrow that lists its tools.
fn group_label(name: &str) -> String {
    format!("{} ▾", group_name(name))
}

/// The rows a Ribbon tab's groups go in, by each group's width: as many to a row as fit in
/// `room`, in order, a group that does not fit starting the next row whole (one wider than
/// `room` has a row to itself).
pub(super) fn wrap_rows(widths: &[f32], room: f32) -> Vec<Vec<usize>> {
    let mut rows: Vec<Vec<usize>> = Vec::new();
    let mut used = 0.0;
    for (i, &width) in widths.iter().enumerate() {
        match rows.last_mut() {
            Some(row) if used + width <= room => row.push(i),
            _ => {
                rows.push(vec![i]);
                used = 0.0;
            }
        }
        used += width;
    }
    rows
}

/// How many columns of icons a Ribbon group shows until its tools are pinned or unpinned by
/// hand: as many as fit under its name (`name` wide, a column every `step`), two at least.
pub(super) fn starter_columns(name: f32, step: f32) -> usize {
    // whole columns only; a name narrower than two still gets two
    let fit = (name / step).floor();
    if fit >= 2.0 {
        // a width in columns, small and positive: no truncation
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let fit = fit as usize;
        fit
    } else {
        2
    }
}

/// The tools of a Ribbon group drawn as icons, in order: those pinned by hand, and of the
/// rest the group's first `starter` unless unpinned by hand (`pins`).
pub(super) fn on_ribbon(
    tools: &[&'static str],
    pins: &std::collections::BTreeMap<String, bool>,
    starter: usize,
) -> Vec<&'static str> {
    tools
        .iter()
        .enumerate()
        .filter(|&(i, id)| pins.get(*id).copied().unwrap_or(i < starter))
        .map(|(_, id)| *id)
        .collect()
}
