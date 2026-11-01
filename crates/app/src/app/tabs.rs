//! Edit Tabs: a window over the Profile Toolpath form for its tabs. 2D or 3D, their size,
//! how many (or how far apart) and whether to keep off corners; Add Tabs puts them on the
//! selected vectors, where a click adds or takes one off and a drag moves one.

use super::CutCraftApp;
use crate::form;
use cut_craft_core::document::Units;
use cut_craft_core::entry::{Entry, TabPlacement, TabSettings, TabSpacing, Tabs, spread_tabs};
use cut_craft_core::toolpath::Tool;
use eframe::egui;

/// Where the window's open state is kept.
fn window_id() -> egui::Id {
    egui::Id::unique("edit_tabs")
}

/// The Edit Tabs… button, at the foot of a form's Tabs (Profile Toolpath's).
pub(super) fn group_end(ui: &mut egui::Ui, group: &str) {
    if group == "Tabs"
        && ui
            .button("Edit Tabs…")
            .on_hover_text("Place the tabs: how many, where, their size, 2D or 3D")
            .clicked()
    {
        ui.data_mut(|d| d.insert_temp(window_id(), true));
    }
}

impl CutCraftApp {
    /// The Edit Tabs window over the Profile Toolpath form, while open.
    pub(super) fn edit_tabs(&mut self, ui: &mut egui::Ui) {
        let id = window_id();
        let mut open = ui.data(|d| d.get_temp::<bool>(id)).unwrap_or(false);
        if open {
            let mut add = false;
            let units = self.doc.job.units;
            let Some(form) = self.form.as_mut() else {
                return;
            };
            let params = &mut form.params;
            let before = (params["tabs"].clone(), params["tab"].clone());
            let mut tabs: Tabs = serde_json::from_value(params["tabs"].clone()).unwrap_or_default();
            let mut tab: TabSettings =
                serde_json::from_value(params["tab"].clone()).unwrap_or_default();
            let placed = params["tab_points"].as_array().map_or(0, Vec::len);
            let tool: Tool = serde_json::from_value(params["tool"].clone()).unwrap_or_default();
            let depth = params["depth"].as_f64().unwrap_or_default();
            let mut clear = false;
            egui::Window::new("Edit Tabs")
                .open(&mut open)
                .resizable(false)
                .collapsible(false)
                .show(ui.ctx(), |ui| {
                    settings(ui, &mut tabs, &mut tab, &tool, depth, units);
                    ui.separator();
                    ui.horizontal(|ui| {
                        add = ui
                            .button("Add Tabs")
                            .on_hover_text("Put the tabs on the selected vectors, as set above")
                            .clicked();
                        clear = ui
                            .add_enabled(placed > 0, egui::Button::new("Delete All Tabs"))
                            .clicked();
                    });
                    ui.label(format!("{placed} tabs placed"));
                    ui.weak(
                        "In the 2D view: click a selected vector to add a tab, click a tab to \
                         take it off, drag a tab to move it",
                    );
                });
            // Add Tabs spreads them by the number or distance picked, then they are placed
            let mut spread = tab;
            if spread.spacing != TabSpacing::ConstantDistance {
                spread.spacing = TabSpacing::ConstantNumber; // as the window shows it
            }
            if add || clear {
                tab.spacing = TabSpacing::Placed;
                if tabs == Tabs::None {
                    tabs = Tabs::Flat;
                }
            }
            params["tabs"] = serde_json::to_value(tabs).expect("tabs serialise");
            params["tab"] = serde_json::to_value(tab).expect("tab settings serialise");
            form.dirty |= before != (params["tabs"].clone(), params["tab"].clone());
            if clear {
                params["tab_points"] = serde_json::json!([]);
                form.dirty = true;
            }
            if add {
                self.add_tabs(tabs, spread, tool.diameter);
            }
        }
        ui.data_mut(|d| d.insert_temp(id, open));
    }

    /// Tabs spread on each selected vector by `tab`'s number or distance (by number when it
    /// was set to be placed by hand) and placement, in place of those placed before.
    fn add_tabs(&mut self, tabs: Tabs, tab: TabSettings, tool_diameter: f64) {
        let entry = Entry {
            tabs,
            tab,
            tool_diameter,
            ..Entry::default()
        };
        let points: Vec<_> = self
            .doc
            .selection
            .iter()
            .filter_map(|id| self.doc.vectors.get(id))
            .flat_map(|path| {
                let mut flat = path.flatten(cut_craft_core::polygon::TOLERANCE);
                if path.closed && flat.len() > 1 && flat.first() == flat.last() {
                    flat.pop();
                }
                spread_tabs(&flat, path.closed, &entry)
            })
            .collect();
        if points.is_empty() {
            self.status
                .set_message("select the vectors to put the tabs on");
            return;
        }
        self.status
            .set_message(&format!("{} tabs added", points.len()));
        if let Some(form) = self.form.as_mut() {
            form.params["tab_points"] = serde_json::json!(points);
            form.dirty = true;
        }
    }
}

/// 2D or 3D, the size (from the tool, or set), and how the tabs spread.
fn settings(
    ui: &mut egui::Ui,
    tabs: &mut Tabs,
    tab: &mut TabSettings,
    tool: &Tool,
    depth: f64,
    units: Units,
) {
    ui.horizontal(|ui| {
        ui.radio_value(tabs, Tabs::Flat, "2D Tabs")
            .on_hover_text("Up at the start of each tab, across it flat, down at its end");
        ui.radio_value(tabs, Tabs::Triangular, "3D Tabs")
            .on_hover_text("Up to the tab's thickness at its middle and down the other side");
    });
    ui.checkbox(&mut tab.size_from_tool, "Size from the tool")
        .on_hover_text("As thick as the pass depth (at most half the cut), twice as long");
    let sized = tab.sized(tool.pass_depth, depth);
    egui::Grid::new("tab_size").num_columns(2).show(ui, |ui| {
        ui.label("Length (width along the path)");
        if tab.size_from_tool {
            ui.label(shown(sized.length, units));
        } else {
            length(ui, &mut tab.length, units);
        }
        ui.end_row();
        ui.label("Thickness (height)");
        if tab.size_from_tool {
            ui.label(shown(sized.thickness, units));
        } else {
            length(ui, &mut tab.thickness, units);
        }
        ui.end_row();
    });
    ui.separator();
    let by_distance = tab.spacing == TabSpacing::ConstantDistance;
    egui::Grid::new("tab_spread").num_columns(2).show(ui, |ui| {
        if ui.radio(!by_distance, "Number").clicked() {
            tab.spacing = TabSpacing::ConstantNumber;
        }
        ui.add(egui::DragValue::new(&mut tab.number).range(1..=100));
        ui.end_row();
        if ui.radio(by_distance, "Distance apart").clicked() {
            tab.spacing = TabSpacing::ConstantDistance;
        }
        length(ui, &mut tab.distance, units);
        ui.end_row();
    });
    ui.horizontal(|ui| {
        ui.radio_value(&mut tab.placement, TabPlacement::Evenly, "Evenly");
        ui.radio_value(
            &mut tab.placement,
            TabPlacement::AtStart,
            "First at the start",
        );
        ui.radio_value(
            &mut tab.placement,
            TabPlacement::AvoidCorners,
            "Not on corners",
        )
        .on_hover_text("Moved along a little to keep them off corners and curves");
    });
}

/// `mm` in the job's units.
fn shown(mm: f64, units: Units) -> String {
    let (per, unit) = form::shown_in("mm", units).unwrap_or((1.0, "mm"));
    format!("{} {unit}", super::trimmed(mm / per))
}

/// A length edited in the job's units, kept in mm.
fn length(ui: &mut egui::Ui, mm: &mut f64, units: Units) {
    let (per, unit) = form::shown_in("mm", units).unwrap_or((1.0, "mm"));
    let mut value = *mm / per;
    if ui
        .add(
            egui::DragValue::new(&mut value)
                .range(0.0..=f64::MAX)
                .speed(0.1 / per)
                .suffix(format!(" {unit}")),
        )
        .changed()
    {
        *mm = value * per;
    }
}
