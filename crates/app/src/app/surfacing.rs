//! The Surfacing Toolpath form's extras: the machine's bed size carried into its settings,
//! the red warning when it flattens the spoilboard, and its Cutting window (which way the
//! bit runs, back and forth or one way).

use super::tabs::Stock;
use cut_craft_bug_report::hint::HoverHint;
use eframe::egui;
use serde_json::Value;

use cmd_surfacing_toolpath::AREA_GROUP;

/// The form's group its pattern is in.
const PATTERN_GROUP: &str = "Pattern";

/// What the red warning says when the spoilboard is chosen.
pub(super) const SPOILBOARD_WARNING: &str = "This cuts your spoilboard, not your material. \
    Take everything off the bed first. Zero X and Y at the spoilboard's front-left corner, and \
    Z on top of the spoilboard.";

/// Said under the tool when a Surfacing Toolpath cuts with a bit that isn't a surfacing bit.
pub(super) const NOT_A_SURFACING_BIT: &str = "Not a surfacing bit: it works, but a wide \
    surfacing bit (Tool Type Surfacing Bit) flattens in fewer rows";

/// Whether a form's `params` flatten with `tool` that isn't a surfacing bit.
pub(super) fn wants_surfacing_bit(params: &Value, tool: &cut_craft_core::toolpath::Tool) -> bool {
    is_surfacing(params) && tool.shape != cut_craft_core::toolpath::ToolShape::SurfacingBit
}

/// Whether `params` are a Surfacing Toolpath form's.
fn is_surfacing(params: &Value) -> bool {
    params.get("area").is_some() && params.get("pattern").is_some()
}

/// The machine's bed size put in the settings, and the spoilboard's size filled in from it
/// while it is not set. Whether anything changed.
pub(super) fn fill_bed(params: &mut Value, bed: [f64; 2]) -> bool {
    let mut changed = false;
    let bed_value = serde_json::json!(bed);
    if params["bed"] != bed_value {
        params["bed"] = bed_value;
        changed = true;
    }
    let unset = |key: &str| params[key].as_f64().unwrap_or(0.0) <= 0.0;
    if bed[0] > 0.0 && bed[1] > 0.0 && unset("spoilboard_width") && unset("spoilboard_height") {
        params["spoilboard_width"] = bed[0].into();
        params["spoilboard_height"] = bed[1].into();
        changed = true;
    }
    changed
}

/// What the Cutting choices make, in a line.
pub(super) fn cutting_said(params: &Value) -> String {
    let climb = params["direction"] != "conventional";
    let cut = if climb { "climb" } else { "conventional" };
    if params["pattern"] == "spiral" {
        let way = if climb { "clockwise" } else { "anticlockwise" };
        return format!("Rings from the outside in, going {way} ({cut})");
    }
    if params["rows"] == "one_way" {
        format!("Every row one way ({cut}), lifting back between rows")
    } else {
        format!("Back and forth, staying down; the first row {cut}")
    }
}

/// The foot of the form's groups: the bed and spoilboard warning under What to Flatten,
/// Cutting… under Pattern. Whether the settings changed.
pub(super) fn group_end(ui: &mut egui::Ui, group: &str, params: &mut Value, stock: Stock) -> bool {
    if !is_surfacing(params) {
        return false;
    }
    match group {
        AREA_GROUP => area_end(ui, params, stock),
        PATTERN_GROUP => pattern_end(ui, params),
        _ => false,
    }
}

fn area_end(ui: &mut egui::Ui, params: &mut Value, stock: Stock) -> bool {
    let changed = fill_bed(params, stock.bed);
    match params["area"].as_str().unwrap_or_default() {
        "spoilboard" => {
            let red = ui.visuals().error_fg_color;
            egui::Frame::group(ui.style())
                .stroke(egui::Stroke::new(1.5, red))
                .show(ui, |ui| {
                    ui.colored_label(red, format!("⚠ {SPOILBOARD_WARNING}"));
                });
            if stock.bed[0] <= 0.0 {
                ui.weak(
                    "Your machine's bed size isn't set: type the spoilboard's size, or set the \
                     bed size in Machine ▸ Machines",
                );
            }
        }
        "selected_vectors" if !stock.closed => {
            ui.colored_label(
                ui.visuals().warn_fg_color,
                "Select the closed outline of what to flatten",
            );
        }
        _ => {}
    }
    changed
}

fn pattern_end(ui: &mut egui::Ui, params: &mut Value) -> bool {
    let id = egui::Id::unique("surfacing_cutting");
    let mut open = ui.data(|d| d.get_temp::<bool>(id)).unwrap_or(false);
    ui.label(cutting_said(params));
    ui.weak("Goes down beside the work and feeds in from the side: it never plunges");
    if ui
        .button("Cutting…")
        .on_hover_hint("Which way the bit runs, and back and forth or one way")
        .clicked()
    {
        open = true;
    }
    let before = params.clone();
    if open {
        egui::Window::new("Cutting")
            .open(&mut open)
            .resizable(false)
            .collapsible(false)
            .show(ui.ctx(), |ui| window(ui, params));
    }
    ui.data_mut(|d| d.insert_temp(id, open));
    before != *params
}

/// The window's inside: the choices, then what they make.
fn window(ui: &mut egui::Ui, params: &mut Value) {
    egui::Grid::new("surfacing_cutting")
        .num_columns(2)
        .show(ui, |ui| {
            super::cutting::choose(
                ui,
                params,
                "direction",
                "Direction",
                &[
                    (
                        "climb",
                        "Climb",
                        "The cutter's edge meets the material head on: usually the smoother \
                         finish",
                    ),
                    ("conventional", "Conventional", "The other way round"),
                ],
            );
            if params["pattern"] != "spiral" {
                super::cutting::choose(
                    ui,
                    params,
                    "rows",
                    "Rows",
                    &[
                        (
                            "back_and_forth",
                            "Back and forth",
                            "Each row back the way the last came, staying down: the quickest",
                        ),
                        (
                            "one_way",
                            "One way",
                            "Every row the direction chosen, lifting back between rows: the \
                             most even finish",
                        ),
                    ],
                );
            }
        });
    ui.label(cutting_said(params));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_bed_fills_an_unset_spoilboard_but_never_one_typed_in() {
        let mut p = serde_json::json!({"area": "spoilboard", "spoilboard_width": 0.0,
            "spoilboard_height": 0.0, "bed": [0.0, 0.0]});
        assert!(fill_bed(&mut p, [1220.0, 610.0]));
        assert_eq!(p["spoilboard_width"], 1220.0);
        assert_eq!(p["spoilboard_height"], 610.0);
        assert_eq!(p["bed"], serde_json::json!([1220.0, 610.0]));
        assert!(!fill_bed(&mut p, [1220.0, 610.0]), "nothing new");
        p["spoilboard_width"] = 900.0.into();
        fill_bed(&mut p, [1300.0, 700.0]);
        assert_eq!(p["spoilboard_width"], 900.0, "typed in, kept");
    }

    #[test]
    fn the_cutting_line_says_what_the_choices_make() {
        let p = |pattern: &str, rows: &str, direction: &str| {
            cutting_said(&serde_json::json!({"pattern": pattern, "rows": rows,
                "direction": direction}))
        };
        assert!(p("along_x", "back_and_forth", "climb").starts_with("Back and forth"));
        assert!(p("along_y", "one_way", "conventional").contains("one way (conventional)"));
        assert!(p("spiral", "one_way", "climb").contains("clockwise (climb)"));
    }
}
