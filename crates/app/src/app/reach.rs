//! A toolpath form's reach line, under its Tool: can the bit reach everything? The same
//! check Calculate and Save Toolpaths make ([`cut_craft_core::reach`]), worked out from the
//! form's settings and the selected shapes before anything is calculated.

use cut_craft_core::reach::{self, Line, Missed, Side};
use cut_craft_core::toolpath::Tool;
use eframe::egui;
use serde_json::Value;

/// What the last reach line was worked out from, and what it found: shapes take a moment, so
/// they are only measured again when something here changes.
pub(super) struct ReachFound {
    key: (Value, Value, u64, cut_craft_core::selection::Selection),
    missed: Option<Missed>,
}

/// Which side of its shapes a form cuts, for the forms whose bit can be too wide for a spot
/// in them: Pocket inside; Profile inside or outside (not on the line).
pub(super) fn side(id: &str, params: &Value) -> Option<Side> {
    match (id, params["side"].as_str()) {
        ("pocket_toolpath", _) | ("profile_toolpath", Some("inside")) => Some(Side::Inside),
        ("profile_toolpath", Some("outside")) => Some(Side::Outside),
        _ => None,
    }
}

/// How deep form `id` cuts from the top surface, where its settings say: outright, or for a
/// Chamfer worked out from its start depth, width, bit and overcut.
pub(super) fn depth(id: &str, params: &Value) -> Option<f64> {
    if id == "chamfer_toolpath" {
        let p: cmd_chamfer_toolpath::Params = serde_json::from_value(params.clone()).ok()?;
        return Some(p.deepest()).filter(|d| d.is_finite() && *d > 0.0);
    }
    ["depth", "cut_depth"]
        .iter()
        .find_map(|k| params[k].as_f64())
        .filter(|d| *d > 0.0)
}

/// The reach line in its colour.
pub(super) fn show(ui: &mut egui::Ui, line: &Line) {
    let colour = match line {
        Line::Reaches(_) => ui.visuals().weak_text_color(),
        Line::Short(_) => ui.visuals().error_fg_color,
        Line::Unknown(_) => ui.visuals().warn_fg_color,
    };
    let shown = ui.colored_label(colour, line.text());
    if matches!(line, Line::Short(_)) {
        shown.on_hover_text(
            "Pick a smaller bit (or one with longer flutes) with Select…, or cut less deep. \
             Calculate still makes the toolpath; what the bit can't reach is left uncut",
        );
    }
}

impl super::CutCraftApp {
    /// The open form's reach line: `None` for a form without a tool, or before one is picked.
    pub(super) fn form_reach(&mut self) -> Option<Line> {
        let form = self.form.as_ref()?;
        let tool: Tool = serde_json::from_value(form.params.get("tool")?.clone()).ok()?;
        let missed = match side(form.id, &form.params) {
            Some(side) => {
                let key = (
                    Value::from(form.id),
                    serde_json::json!([form.params["tool"], form.params["side"]]),
                    self.history.revision(),
                    self.doc.selection.clone(),
                );
                if self.reach_found.as_ref().is_none_or(|f| f.key != key) {
                    let (_, shapes) = self.doc.selected_paths(|p| p.closed);
                    let missed = (!shapes.is_empty()).then(|| reach::flat(&shapes, &tool, side));
                    self.reach_found = Some(ReachFound { key, missed });
                }
                self.reach_found.as_ref().and_then(|f| f.missed)
            }
            None => None,
        };
        reach::form_line(
            &tool,
            depth(form.id, &form.params),
            missed.as_ref(),
            self.doc.job.units,
        )
    }
}
