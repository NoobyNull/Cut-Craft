//! The Ribbon interface (Options ▸ Appearance ▸ Interface style), as Fusion lays its tools
//! out: tabs of tool groups across the top, each group's icons over its name, which drops
//! down its tools by name; a browser of the job's sheets, layers, components, clipart and
//! toolpaths on the left; the open tool's form on the right.

use super::{CutCraftApp, DESIGN_PANEL_WIDTHS, TOOL_ICON_ROWS, egui, gui};

/// The ribbon's tabs.
pub(super) const RIBBON_TABS: [&str; 3] = ["Design", "Model", "Toolpaths"];
/// The browser's tabs.
pub(super) const BROWSER_TABS: [&str; 5] =
    ["Sheets", "Layers", "Components", "Clipart", "Toolpaths"];

impl CutCraftApp {
    /// The job's ribbon, browser and form around the views.
    pub(super) fn ribbon_pages(&mut self, ui: &mut egui::Ui) {
        let width = DESIGN_PANEL_WIDTHS * ui.global_style().spacing.interact_size.x;
        egui::Panel::top("ribbon").show(ui, |ui| self.ribbon(ui));
        egui::Panel::left("browser")
            .default_size(width)
            .show(ui, |ui| self.browser(ui));
        if self.form.is_some() {
            egui::Panel::right("ribbon_form")
                .default_size(width)
                .show(ui, |ui| {
                    egui::ScrollArea::vertical().show(ui, |ui| self.command_form(ui));
                });
        }
    }

    /// The ribbon's groups by tab.
    fn ribbon_groups(&self, tab: &str) -> Vec<gui::ToolGroup> {
        let toolbar = &self.toolbar;
        match tab {
            "Design" => toolbar.groups.clone(),
            "Model" => toolbar.modeling_groups.clone(),
            _ => toolbar.toolpath_groups.clone(),
        }
    }

    fn ribbon(&mut self, ui: &mut egui::Ui) {
        cut_craft_bug_report::mark(ui, "Ribbon");
        let tab = RIBBON_TABS[self.ribbon_tab.min(RIBBON_TABS.len() - 1)];
        ui.horizontal(|ui| {
            for (i, name) in RIBBON_TABS.iter().enumerate() {
                if ui.selectable_label(i == self.ribbon_tab, *name).clicked() {
                    self.ribbon_tab = i;
                }
            }
        });
        let groups = self.ribbon_groups(tab);
        let edge = TOOL_ICON_ROWS * ui.spacing().interact_size.y;
        let mut clicked = None;
        egui::ScrollArea::horizontal().show(ui, |ui| {
            ui.horizontal(|ui| {
                for group in &groups {
                    ui.vertical(|ui| {
                        ui.horizontal(|ui| {
                            for &id in group.tools {
                                let (label, button) = self.tool_button(id, edge);
                                let response = ui
                                    .add_enabled(self.available(id), button)
                                    .on_hover_text(&label)
                                    .on_disabled_hover_text(format!(
                                        "{label}: not implemented yet"
                                    ));
                                if response.clicked() {
                                    clicked = Some(id);
                                }
                            }
                        });
                        // the group's name drops down its tools by name, as Fusion's do
                        ui.menu_button(format!("{} ▾", group.name), |ui| {
                            for &id in group.tools {
                                let (label, _) = self.tool_button(id, edge);
                                if ui
                                    .add_enabled(self.available(id), egui::Button::new(label))
                                    .clicked()
                                {
                                    clicked = Some(id);
                                }
                            }
                        });
                    });
                    ui.separator();
                }
            });
        });
        if let Some(id) = clicked {
            self.invoke(ui.ctx(), id);
        }
    }

    /// A tool's name and its icon button, picked out while it is the tool in hand.
    fn tool_button(&self, id: &str, edge: f32) -> (String, egui::Button<'static>) {
        let cmd = self.registry.get(id);
        let label = cmd.map_or_else(|| gui::title_case(id), |c| c.meta().label.to_owned());
        let explicit = cmd.map_or("", |c| c.meta().icon);
        let button = self
            .icon_button(explicit, id, &label, edge)
            .selected(self.status.current_tool == label);
        (label, button)
    }

    fn browser(&mut self, ui: &mut egui::Ui) {
        cut_craft_bug_report::mark(ui, "Browser");
        ui.horizontal_wrapped(|ui| {
            for (i, name) in BROWSER_TABS.iter().enumerate() {
                if ui.selectable_label(i == self.browser_tab, *name).clicked() {
                    self.browser_tab = i;
                }
            }
        });
        ui.separator();
        match BROWSER_TABS[self.browser_tab.min(BROWSER_TABS.len() - 1)] {
            "Sheets" => self.sheets_panel(ui),
            "Layers" => self.layers_panel(ui),
            "Components" => self.component_list(ui),
            "Clipart" => self.clipart_panel(ui),
            _ => self.toolpath_list(ui),
        }
    }
}
