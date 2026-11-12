//! Edit Tabs: a window over the Profile Toolpath form for its tabs. 2D or 3D, their size,
//! how many (or how far apart) and whether to keep off corners; Add Tabs puts them on the
//! selected vectors, where a click adds or takes one off and a drag moves one.

use super::CutCraftApp;
use crate::form;
use cut_craft_bug_report::hint::HoverHint;
use cut_craft_core::document::Units;
use cut_craft_core::entry::{Entry, TabPlacement, TabSettings, TabSpacing, Tabs, spread_tabs};
use cut_craft_core::toolpath::Tool;
use eframe::egui;

/// Where the window's open state is kept.
fn window_id() -> egui::Id {
    egui::Id::unique("edit_tabs")
}

/// What a form's group feet need to know of the job: the material and what is selected.
#[derive(Clone, Copy, Debug)]
pub(super) struct Stock {
    /// The material's thickness, mm.
    pub thickness: f64,
    /// Whether a closed shape is among the vectors the toolpath cuts.
    pub closed: bool,
    /// The job's units, for the lengths in the Tabs summary.
    pub units: Units,
}

/// How close to the material's bottom a cut must reach to count as right through, mm.
const THROUGH: f64 = 1e-3;

/// Whether a Profile Toolpath form's `params` cut the piece right out of `stock` with nothing
/// to hold it: right through, round a closed shape, with no tabs.
pub(super) fn cuts_loose(params: &serde_json::Value, stock: Stock) -> bool {
    let Some(side) = params.get("side").and_then(serde_json::Value::as_str) else {
        return false;
    };
    let depth = if params["through"] == true {
        stock.thickness
    } else {
        params["depth"].as_f64().unwrap_or_default()
    };
    let through = stock.thickness > 0.0 && depth + THROUGH >= stock.thickness;
    let closed = side != "on" || stock.closed;
    through && closed && !cut_craft_core::checks::held_by_tabs(params)
}

/// The words over Add Tabs when a profile would cut its piece loose.
pub(super) const LOOSE: &str = "This cuts the piece right out with nothing holding it, so it \
     can move into the cutter or be thrown. Add tabs, or hold it down another way (tape, \
     screws, a vacuum table).";

/// A Profile Toolpath form's tabs in one line, for the form (their settings are in Edit
/// Tabs): their size, and how many go where. None without tabs.
fn form_summary(params: &serde_json::Value, units: Units) -> Option<String> {
    let tabs: Tabs = serde_json::from_value(params["tabs"].clone()).unwrap_or_default();
    let tab: TabSettings = serde_json::from_value(params["tab"].clone()).unwrap_or_default();
    let tool: Tool = serde_json::from_value(params["tool"].clone()).unwrap_or_default();
    let depth = params["depth"].as_f64().unwrap_or_default();
    let placed = params["tab_points"].as_array().map_or(0, Vec::len);
    summary(
        tabs,
        tab.sized(tool.pass_depth, depth),
        tab.size_from_tool,
        placed,
        units,
    )
}

/// `tab` (sized already, from the tool when `from_tool`) in words: how big, how many and
/// where. None for no tabs.
pub(super) fn summary(
    tabs: Tabs,
    tab: TabSettings,
    from_tool: bool,
    placed: usize,
    units: Units,
) -> Option<String> {
    if tabs == Tabs::None {
        return None;
    }
    let size = format!(
        "{} long, {} thick{}",
        shown(tab.length, units),
        shown(tab.thickness, units),
        if from_tool { " (from the tool)" } else { "" }
    );
    let spread = match tab.spacing {
        TabSpacing::Automatic => format!(
            "about one every {}, at least 3 a shape",
            shown(tab.distance, units)
        ),
        TabSpacing::ConstantNumber => format!("{} a shape", tab.number),
        TabSpacing::ConstantDistance => format!(
            "one every {} ({} to {} a shape)",
            shown(tab.distance, units),
            tab.minimum,
            tab.maximum
        ),
        TabSpacing::Placed => return Some(format!("{size}, {placed} placed by hand")),
    };
    let place = match tab.placement {
        TabPlacement::Evenly => "",
        TabPlacement::AtStart => ", the first at the start",
        TabPlacement::AvoidCorners => ", off the corners",
    };
    Some(format!("{size}, {spread}{place}"))
}

/// The Edit Tabs… button, at the foot of a form's Tabs (Profile Toolpath's), with a warning
/// and Add Tabs when the cut would set the piece loose; a cut made right through with the
/// old tick turned into the material's thickness under Cutting Depths; then what else a group's foot holds (Edit
/// Loops…). Whether the form's settings changed.
pub(super) fn group_end(
    ui: &mut egui::Ui,
    group: &str,
    params: &mut serde_json::Value,
    stock: Stock,
) -> bool {
    let mut changed = false;
    // a toolpath made with Cut right through (no longer on the form) reads as a depth of
    // the material's thickness, to change as any other
    if group == form::CUTTING_DEPTHS && params["through"] == true {
        params["depth"] = stock.thickness.into();
        params["through"] = false.into();
        changed = true;
    }
    if group == "Tabs" {
        if cuts_loose(params, stock) {
            ui.colored_label(ui.visuals().warn_fg_color, LOOSE);
            if ui
                .button("Add Tabs")
                .on_hover_hint("Leave small bridges of material that hold the piece in place")
                .clicked()
            {
                params["tabs"] = "flat".into();
                changed = true;
            }
        }
        if let Some(line) = form_summary(params, stock.units) {
            ui.label(line);
        }
        if ui
            .button("Edit Tabs…")
            .on_hover_hint("Place the tabs: how many, where, their size, 2D or 3D")
            .clicked()
        {
            ui.data_mut(|d| d.insert_temp(window_id(), true));
        }
    }
    changed | super::cutting::group_end(ui, group, params, stock.units)
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
                            .on_hover_hint(
                                "Put the tabs on the selected vectors, as set above, to move \
                                 by hand",
                            )
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
            // Add Tabs spreads them as picked (by number when they were placed already),
            // then they are placed, to move by hand
            let mut spread = tab;
            if spread.spacing == TabSpacing::Placed {
                spread.spacing = TabSpacing::ConstantNumber;
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
            .on_hover_hint("Up at the start of each tab, across it flat, down at its end");
        ui.radio_value(tabs, Tabs::Triangular, "3D Tabs")
            .on_hover_hint("Up to the tab's thickness at its middle and down the other side");
    });
    ui.checkbox(&mut tab.size_from_tool, "Size from the tool")
        .on_hover_hint("As thick as the pass depth (at most half the cut), twice as long");
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
    ui.label("How many, and where");
    ui.radio_value(&mut tab.spacing, TabSpacing::Automatic, "Automatic")
        .on_hover_hint(
            "About one every distance below, at least 3 on a shape, and never more than a \
             third of it in tabs: a safe start",
        );
    egui::Grid::new("tab_spread").num_columns(2).show(ui, |ui| {
        ui.radio_value(&mut tab.spacing, TabSpacing::ConstantNumber, "Number")
            .on_hover_hint("This many on every shape, spread along it");
        ui.add(egui::DragValue::new(&mut tab.number).range(1..=100));
        ui.end_row();
        ui.radio_value(
            &mut tab.spacing,
            TabSpacing::ConstantDistance,
            "Distance apart",
        )
        .on_hover_hint("One about every this far along each shape");
        length(ui, &mut tab.distance, units);
        ui.end_row();
        if tab.spacing == TabSpacing::ConstantDistance {
            ui.label("    At least");
            ui.add(egui::DragValue::new(&mut tab.minimum).range(1..=tab.maximum.max(1)));
            ui.end_row();
            ui.label("    At most");
            ui.add(egui::DragValue::new(&mut tab.maximum).range(tab.minimum.max(1)..=1000));
            ui.end_row();
        }
    });
    ui.radio_value(&mut tab.spacing, TabSpacing::Placed, "Where clicked")
        .on_hover_hint("Placed by hand: Add Tabs below, then click, drag or take them off");
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
        .on_hover_hint("Moved along a little to keep them off corners and curves");
    });
}

/// `mm` in the job's units.
fn shown(mm: f64, units: Units) -> String {
    let (per, unit) = form::shown_in("mm", units).unwrap_or((1.0, "mm"));
    format!("{} {unit}", super::trimmed(mm / per))
}

/// A length edited in the job's units, kept in mm.
pub(super) fn length(ui: &mut egui::Ui, mm: &mut f64, units: Units) {
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
