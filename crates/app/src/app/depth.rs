//! A toolpath set to cut deeper than the material is thick: Calculate stops to say the bit
//! will go into the machine bed, OK makes it as set and Cancel goes back with the cut depth
//! put at the material's thickness.

use super::CutCraftApp;
use cut_craft_bug_report::hint::HoverHint;
use cut_craft_core::document::Units;
use eframe::egui;
use serde_json::Value;

/// The forms whose `depth` is how far below the material's surface they cut.
const CHECKED: [&str; 1] = ["profile_toolpath"];

/// How far past the material's bottom a cut must go to be warned of, mm: a cut to exactly
/// the thickness (a clean cut right through) is not.
const PAST: f64 = 1e-3;

/// A Calculate held back for the warning: the form, its settings and the depth they cut to.
#[derive(Clone, Debug)]
pub(super) struct DeepCut {
    pub id: &'static str,
    pub params: Value,
    pub depth: f64,
}

/// How deep form `id`'s `params` cut, when that is past the bottom of material `thickness`
/// thick. A toolpath dropped onto the 3D model never goes below the material, so is not.
pub(super) fn too_deep(id: &str, params: &Value, thickness: f64) -> Option<f64> {
    if !CHECKED.contains(&id) || thickness <= 0.0 || params["project_onto_model"] == true {
        return None;
    }
    let depth = if params["through"] == true {
        thickness
    } else {
        params["depth"].as_f64()?
    };
    (depth > thickness + PAST).then_some(depth)
}

/// The warning's words: how deep, and how thick the material is.
pub(super) fn warning(depth: f64, thickness: f64, units: Units) -> String {
    let (per, unit) = crate::form::shown_in("mm", units).unwrap_or((1.0, "mm"));
    let shown = |mm: f64| format!("{} {unit}", super::trimmed(mm / per));
    format!(
        "This toolpath is set to cut {} deep, {} past the bottom of the {} thick material, \
         so the bit will cut into the machine bed or spoilboard.",
        shown(depth),
        shown(depth - thickness),
        shown(thickness)
    )
}

impl CutCraftApp {
    /// The warning over everything while a Calculate waits on it.
    pub(super) fn deep_cut_warning(&mut self, ctx: &egui::Context) {
        let Some(held) = self.deep_cut.clone() else {
            return;
        };
        let thickness = self.doc.job.thickness;
        let (mut ok, mut cancel) = (false, false);
        egui::Modal::new(egui::Id::unique("deep_cut")).show(ctx, |ui| {
            ui.set_max_width(ui.spacing().interact_size.x * 6.0);
            ui.heading("Warning: deeper than the material");
            ui.label(warning(held.depth, thickness, self.doc.job.units));
            ui.label(
                "OK makes the toolpath as it is set. Cancel goes back to the form, with the cut \
                 depth set to the material's thickness.",
            );
            ui.separator();
            ui.horizontal(|ui| {
                ok = ui
                    .button("OK")
                    .on_hover_hint("Cut this deep anyway: the settings are not changed")
                    .clicked();
                cancel = ui
                    .button("Cancel")
                    .on_hover_hint("Back to the form, cutting no deeper than the material")
                    .clicked();
            });
        });
        if !(ok || cancel) {
            return;
        }
        self.deep_cut = None;
        if ok {
            self.calculate(held.id, &held.params);
        } else if let Some(form) = self.form.as_mut().filter(|f| f.id == held.id) {
            form.params["depth"] = thickness.into();
            form.params["through"] = false.into();
            form.dirty = true;
            self.status
                .set_message("Cut depth set to the material's thickness");
        }
    }

    /// A form's Calculate (or Apply): held back for the warning when it cuts past the
    /// material, else made.
    pub(super) fn calculate_checked(&mut self, id: &'static str, params: &Value) {
        if let Some(depth) = too_deep(id, params, self.doc.job.thickness) {
            self.deep_cut = Some(DeepCut {
                id,
                params: params.clone(),
                depth,
            });
            return;
        }
        self.calculate(id, params);
    }

    /// The form's settings made, what goes wrong put on the form.
    fn calculate(&mut self, id: &str, params: &Value) {
        // a job that goes on in the background puts what goes wrong on the form
        if let Some(open) = &mut self.form {
            open.error = None;
        }
        let result = self.apply_form(id, params);
        if let (Err(why), Some(open)) = (result, &mut self.form) {
            open.error = Some(why);
        }
    }
}
