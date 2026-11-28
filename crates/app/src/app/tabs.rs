//! Edit Tabs: a window over the Profile Toolpath form for its tabs. 2D or 3D, their size,
//! how many (or how far apart) and whether to keep off corners; Add Tabs puts them on the
//! selected vectors, where a click adds or takes one off and a drag moves one.

use super::CutCraftApp;
use crate::form;
use cut_craft_bug_report::hint::HoverHint;
use cut_craft_bug_report::number::Number;
use cut_craft_core::document::Units;
use cut_craft_core::entry::{Entry, TabPlacement, TabSettings, TabSpacing, Tabs, spread_tabs};
use cut_craft_core::toolpath::Tool;
use cut_craft_i18n::tr;
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
    /// The active machine's bed size, mm (0 when not known), for flattening the spoilboard.
    pub bed: [f64; 2],
    /// The job's Clearing Settings, said in the Cutting window.
    pub clearing: cut_craft_core::entry::ClearingSettings,
    /// With tabs placed by hand, how many of the shapes cut out got no tab.
    pub untabbed: usize,
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

/// Whether a Profile Toolpath form's `params` cut right through `stock`.
fn through(params: &serde_json::Value, stock: Stock) -> bool {
    let depth = if params["through"] == true {
        stock.thickness
    } else {
        params["depth"].as_f64().unwrap_or_default()
    };
    stock.thickness > 0.0 && depth + THROUGH >= stock.thickness
}

/// A toolpath made with Cut right through (no longer on the form) read as a depth of the
/// material's thickness, to change as any other; whether it was. Done whichever tab of the
/// form is shown.
pub(super) fn through_as_depth(params: &mut serde_json::Value, stock: Stock) -> bool {
    if params["through"] != true {
        return false;
    }
    params["depth"] = stock.thickness.into();
    params["through"] = false.into();
    true
}

/// The words over Add Tabs when a profile would cut its piece loose.
pub(super) fn loose() -> String {
    tr!("tabs-loose")
}

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
    let size = tr!(
        if from_tool {
            "tabs-summary-size-from-tool"
        } else {
            "tabs-summary-size"
        },
        length = shown(tab.length, units),
        thickness = shown(tab.thickness, units)
    );
    let spread = match tab.spacing {
        TabSpacing::Automatic => tr!(
            "tabs-summary-automatic",
            distance = shown(tab.distance, units)
        ),
        TabSpacing::ConstantNumber => tr!("tabs-summary-number", number = tab.number),
        TabSpacing::ConstantDistance => tr!(
            "tabs-summary-distance",
            distance = shown(tab.distance, units),
            minimum = tab.minimum,
            maximum = tab.maximum
        ),
        TabSpacing::Placed => {
            return Some(tr!("tabs-summary-placed", size = size, placed = placed));
        }
    };
    let place = match tab.placement {
        TabPlacement::Evenly => "tabs-summary-evenly",
        TabPlacement::AtStart => "tabs-summary-at-start",
        TabPlacement::AvoidCorners => "tabs-summary-avoid-corners",
    };
    Some(tr!(place, size = size, spread = spread))
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
    if group == form::CUTTING_DEPTHS {
        changed |= through_as_depth(params, stock);
    }
    if group == "Tabs" {
        if stock.untabbed > 0 && !cuts_loose(params, stock) && through(params, stock) {
            ui.colored_label(
                ui.visuals().warn_fg_color,
                tr!("tabs-untabbed", count = stock.untabbed),
            );
        }
        if cuts_loose(params, stock) {
            ui.colored_label(ui.visuals().warn_fg_color, loose());
            if ui
                .button(tr!("tabs-add"))
                .on_hover_hint(tr!("tabs-add.tooltip"))
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
            .button(tr!("tabs-edit"))
            .on_hover_hint(tr!("tabs-edit.tooltip"))
            .clicked()
        {
            ui.data_mut(|d| d.insert_temp(window_id(), true));
        }
    }
    changed |= super::cutting::group_end(ui, group, params, stock.units, &stock.clearing);
    changed |= super::entry::group_end(ui, group, params, stock.units);
    changed | super::surfacing::group_end(ui, group, params, stock)
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
            egui::Window::new(tr!("tabs-window-title"))
                .id(egui::Id::unique("edit_tabs_window"))
                .open(&mut open)
                .resizable(false)
                .collapsible(false)
                .show(ui.ctx(), |ui| {
                    settings(ui, &mut tabs, &mut tab, &tool, depth, units);
                    ui.separator();
                    ui.horizontal(|ui| {
                        add = ui
                            .button(tr!("tabs-add"))
                            .on_hover_hint(tr!("tabs-add.tooltip-window"))
                            .clicked();
                        clear = ui
                            .add_enabled(placed > 0, egui::Button::new(tr!("tabs-delete-all")))
                            .on_hover_hint(tr!("tabs-delete-all.tooltip"))
                            .clicked();
                    });
                    ui.label(tr!("tabs-placed", count = placed));
                    ui.weak(tr!("tabs-how-to-place"));
                });
            // Add Tabs spreads them as picked (by number when they were placed already),
            // then they are placed, to move by hand
            let mut spread = tab;
            if spread.spacing == TabSpacing::Placed {
                spread.spacing = TabSpacing::ConstantNumber;
            }
            // tabs added are placed, to move by hand, and turn tabs on; with nothing selected
            // none are added and the settings stay as they were
            let on = if tabs == Tabs::None { Tabs::Flat } else { tabs };
            if add && self.add_tabs(on, spread, tool.diameter) {
                tab.spacing = TabSpacing::Placed;
                tabs = on;
            }
            let Some(form) = self.form.as_mut() else {
                return;
            };
            let params = &mut form.params;
            params["tabs"] = serde_json::to_value(tabs).expect("tabs serialise");
            params["tab"] = serde_json::to_value(tab).expect("tab settings serialise");
            form.dirty |= before != (params["tabs"].clone(), params["tab"].clone());
            if clear {
                // still placed by hand, on or off as they were: say how to place more
                params["tab_points"] = serde_json::json!([]);
                form.dirty = true;
                self.status.set_message(&tr!("tabs-all-deleted"));
            }
        }
        ui.data_mut(|d| d.insert_temp(id, open));
    }

    /// Tabs spread on each selected vector by `tab`'s number or distance (by number when it
    /// was set to be placed by hand) and placement, in place of those placed before. Whether
    /// any were added (none with nothing selected).
    fn add_tabs(&mut self, tabs: Tabs, tab: TabSettings, tool_diameter: f64) -> bool {
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
            self.status.set_message(&tr!("tabs-select-first"));
            return false;
        }
        self.status
            .set_message(&tr!("tabs-added", count = points.len()));
        if let Some(form) = self.form.as_mut() {
            form.params["tab_points"] = serde_json::json!(points);
            form.dirty = true;
        }
        true
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
        ui.radio_value(tabs, Tabs::Flat, tr!("tabs-flat"))
            .on_hover_hint(tr!("tabs-flat.tooltip"));
        ui.radio_value(tabs, Tabs::Triangular, tr!("tabs-triangular"))
            .on_hover_hint(tr!("tabs-triangular.tooltip"));
    });
    ui.checkbox(&mut tab.size_from_tool, tr!("tabs-size-from-tool"))
        .on_hover_hint(tr!("tabs-size-from-tool.tooltip"));
    let sized = tab.sized(tool.pass_depth, depth);
    egui::Grid::new("tab_size").num_columns(2).show(ui, |ui| {
        ui.label(tr!("tabs-length"));
        if tab.size_from_tool {
            ui.label(shown(sized.length, units));
        } else {
            length(ui, &tr!("tabs-length"), &mut tab.length, units);
        }
        ui.end_row();
        ui.label(tr!("tabs-thickness"));
        if tab.size_from_tool {
            ui.label(shown(sized.thickness, units));
        } else {
            length(ui, &tr!("tabs-thickness"), &mut tab.thickness, units);
        }
        ui.end_row();
    });
    ui.separator();
    ui.label(tr!("tabs-how-many"));
    ui.radio_value(
        &mut tab.spacing,
        TabSpacing::Automatic,
        tr!("tabs-automatic"),
    )
    .on_hover_hint(tr!("tabs-automatic.tooltip"));
    egui::Grid::new("tab_spread").num_columns(2).show(ui, |ui| {
        ui.radio_value(
            &mut tab.spacing,
            TabSpacing::ConstantNumber,
            tr!("tabs-number"),
        )
        .on_hover_hint(tr!("tabs-number.tooltip"));
        ui.add(
            Number::new(&mut tab.number)
                .range(1..=100)
                .name(tr!("tabs-number")),
        );
        ui.end_row();
        ui.radio_value(
            &mut tab.spacing,
            TabSpacing::ConstantDistance,
            tr!("tabs-distance-apart"),
        )
        .on_hover_hint(tr!("tabs-distance-apart.tooltip"));
        length(ui, &tr!("tabs-distance-apart"), &mut tab.distance, units);
        ui.end_row();
        if tab.spacing == TabSpacing::ConstantDistance {
            indented(ui, tr!("tabs-at-least"));
            ui.add(
                Number::new(&mut tab.minimum)
                    .range(1..=tab.maximum.max(1))
                    .name(tr!("tabs-at-least")),
            );
            ui.end_row();
            indented(ui, tr!("tabs-at-most"));
            ui.add(
                Number::new(&mut tab.maximum)
                    .range(tab.minimum.max(1)..=1000)
                    .name(tr!("tabs-at-most")),
            );
            ui.end_row();
        }
    });
    ui.radio_value(
        &mut tab.spacing,
        TabSpacing::Placed,
        tr!("tabs-where-clicked"),
    )
    .on_hover_hint(tr!("tabs-where-clicked.tooltip"));
    ui.horizontal(|ui| {
        ui.radio_value(&mut tab.placement, TabPlacement::Evenly, tr!("tabs-evenly"));
        ui.radio_value(
            &mut tab.placement,
            TabPlacement::AtStart,
            tr!("tabs-first-at-start"),
        );
        ui.radio_value(
            &mut tab.placement,
            TabPlacement::AvoidCorners,
            tr!("tabs-not-on-corners"),
        )
        .on_hover_hint(tr!("tabs-not-on-corners.tooltip"));
    });
}

/// A label set in under the setting above it, as At least and At most under Distance apart.
fn indented(ui: &mut egui::Ui, text: String) {
    ui.horizontal(|ui| {
        ui.add_space(ui.spacing().indent);
        ui.label(text);
    });
}

/// `mm` in the job's units.
fn shown(mm: f64, units: Units) -> String {
    let (per, unit) = form::shown_in("mm", units).unwrap_or((1.0, "mm"));
    format!("{} {unit}", super::trimmed(mm / per))
}

/// A length edited in the job's units, kept in mm.
/// `name` is what a screen reader calls the box: its label's words.
pub(super) fn length(ui: &mut egui::Ui, name: &str, mm: &mut f64, units: Units) {
    let (per, unit) = form::shown_in("mm", units).unwrap_or((1.0, "mm"));
    let mut value = *mm / per;
    if ui
        .add(
            Number::new(&mut value)
                .range(0.0..=f64::MAX)
                .speed(0.1 / per)
                .suffix(format!(" {unit}"))
                .name(name),
        )
        .changed()
    {
        *mm = value * per;
    }
}
