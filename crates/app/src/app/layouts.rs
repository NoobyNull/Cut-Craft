//! The Rail and Ribbon window layouts, and the Find Tool window and inspector every layout
//! can use (`docs/superpowers/specs/2026-10-27-window-layout-setting-design.md`).

use super::{
    Align, CutCraftApp, INSPECTOR_TABS, Layout, Page, RIBBON_TABS, TOOL_ICON_ROWS, egui, gui,
    side_panel,
};
use crate::options::WindowLayout;

/// View ▸ Window Layout's items and the layouts they pick.
const LAYOUTS: [(&str, WindowLayout); 3] = [
    ("layout_classic", WindowLayout::Classic),
    ("layout_rail", WindowLayout::Rail),
    ("layout_ribbon", WindowLayout::Ribbon),
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

    /// Every tool group: the drawing groups, then the modeling and toolpath ones.
    pub(super) fn all_groups(&self) -> Vec<gui::ToolGroup> {
        let t = &self.toolbar;
        t.groups
            .iter()
            .chain(&t.modeling_groups)
            .chain(&t.toolpath_groups)
            .copied()
            .collect()
    }

    /// A tool's name: the command's label, or its id in words.
    pub(super) fn tool_label(&self, id: &str) -> String {
        self.registry
            .get(id)
            .map_or_else(|| gui::title_case(id), |c| c.meta().label.to_owned())
    }

    /// The available tools whose names contain `text` (any case), in group order.
    pub(super) fn search_matches(&self, text: &str) -> Vec<(&'static str, String)> {
        let text = text.trim().to_lowercase();
        self.all_groups()
            .iter()
            .flat_map(|g| g.tools.iter().copied())
            .filter(|id| self.available(id))
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
        egui::Window::new("Find Tool")
            .id(egui::Id::unique("find_tool"))
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_TOP, egui::vec2(0.0, 2.0 * row))
            .open(&mut open)
            .show(ctx, |ui| {
                cut_craft_bug_report::mark(ui, "Find Tool");
                let edit = ui
                    .add(egui::TextEdit::singleline(&mut text).hint_text("Part of a tool's name"));
                // the field takes the keyboard when the window opens
                if ui.memory(egui::Memory::focused).is_none() {
                    edit.request_focus();
                }
                if edit.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                    picked = matches.first().map(|(id, _)| *id);
                }
                ui.separator();
                if matches.is_empty() {
                    ui.weak("No tool has that in its name");
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
        // Escape closes it from `shortcuts`, with whatever else is open
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
            // the form draws its own Apply and Close
            egui::ScrollArea::vertical().show(ui, |ui| self.command_form(ui));
            return;
        }
        self.browser(ui);
    }

    /// Sheets, layers, components, clipart and the toolpath list under a row of tabs.
    pub(super) fn browser(&mut self, ui: &mut egui::Ui) {
        cut_craft_bug_report::mark(ui, "Browser");
        ui.horizontal_wrapped(|ui| {
            for (i, tab) in INSPECTOR_TABS.iter().enumerate() {
                ui.selectable_value(&mut self.inspector_tab, i, *tab);
            }
        });
        ui.separator();
        match INSPECTOR_TABS[self.inspector_tab] {
            "Toolpaths" => self.toolpath_list(ui),
            tab => {
                egui::ScrollArea::vertical().show(ui, |ui| match tab {
                    "Sheets" => self.sheets_panel(ui),
                    "Layers" => self.layers_panel(ui),
                    "Components" => self.component_list(ui),
                    _ => self.clipart_panel(ui),
                });
            }
        }
    }

    /// A group's tools by icon and name, one per row; whether one was picked.
    pub(super) fn tool_rows(&mut self, ui: &mut egui::Ui, group: gui::ToolGroup) -> bool {
        let edge = ui.spacing().interact_size.y;
        let mut clicked = None;
        for &id in group.tools {
            let label = self.tool_label(id);
            let explicit = self.registry.get(id).map_or("", |c| c.meta().icon);
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
            let response = ui
                .add_enabled(self.available(id), button)
                .on_disabled_hover_text(format!("{label}: not implemented yet"));
            if response.clicked() {
                clicked = Some(id);
            }
        }
        if let Some(id) = clicked {
            self.pick(ui.ctx(), self.page_of(id), id);
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
        let groups = self.all_groups();
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
                                let explicit = self.registry.get(id).map_or("", |c| c.meta().icon);
                                self.icons.find(explicit, id)
                            });
                            let button = match icon {
                                Some(path) => egui::Button::image(
                                    self.icons
                                        .image(path)
                                        .fit_to_exact_size(egui::vec2(edge, edge)),
                                ),
                                None => egui::Button::new(group.name)
                                    .wrap_mode(egui::TextWrapMode::Truncate),
                            }
                            .selected(self.rail_group == Some(i));
                            if ui.add(button).on_hover_text(group.name).clicked() {
                                self.rail_group = (self.rail_group != Some(i)).then_some(i);
                            }
                        }
                    });
            });
        if let Some(group) = self.rail_group.and_then(|i| groups.get(i).copied()) {
            side_panel(egui::Panel::left("rail_flyout"), ui.ctx()).show(ui, |ui| {
                cut_craft_bug_report::mark(ui, "Rail flyout");
                ui.horizontal(|ui| {
                    ui.strong(group.name);
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        ui.toggle_value(&mut self.rail_pinned, "📌")
                            .on_hover_text("Keep this list open after picking a tool");
                    });
                });
                ui.separator();
                let picked = egui::ScrollArea::vertical()
                    .show(ui, |ui| self.tool_rows(ui, group))
                    .inner;
                if picked && !self.rail_pinned {
                    self.rail_group = None;
                }
            });
        }
        side_panel(egui::Panel::right("inspector"), ui.ctx()).show(ui, |ui| self.inspector(ui));
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
            ui.horizontal(|ui| {
                for (i, tab) in RIBBON_TABS.iter().enumerate() {
                    ui.selectable_value(&mut self.ribbon_tab, i, *tab);
                }
            });
            ui.separator();
            let (groups, page) = self.ribbon_groups();
            egui::ScrollArea::horizontal().show(ui, |ui| {
                ui.horizontal_top(|ui| {
                    for group in &groups {
                        ui.vertical(|ui| {
                            // two rows of icons: half the tools across, rounded up, but
                            // never narrower than the group's name
                            let columns =
                                u16::try_from(group.tools.len().div_ceil(2)).unwrap_or(u16::MAX);
                            let cell =
                                edge + 2.0 * spacing.button_padding.x + spacing.item_spacing.x;
                            let name = ui
                                .painter()
                                .layout_no_wrap(
                                    group.name.to_owned(),
                                    egui::TextStyle::Body.resolve(ui.style()),
                                    egui::Color32::PLACEHOLDER,
                                )
                                .size()
                                .x;
                            ui.set_max_width((f32::from(columns) * cell).max(name + cell));
                            self.tool_groups(ui, std::slice::from_ref(group), page);
                        });
                        ui.separator();
                    }
                });
            });
        });
        side_panel(egui::Panel::left("browser"), ui.ctx()).show(ui, |ui| self.browser(ui));
        if self.form.is_some() {
            side_panel(egui::Panel::right("inspector"), ui.ctx()).show(ui, |ui| self.inspector(ui));
        }
    }
}
