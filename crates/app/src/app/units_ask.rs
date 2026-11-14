//! A drawing that doesn't say what units it was made in (an R12 DXF): before it comes in, a
//! pop-up asks whether it is in inches or millimetres, both sizes shown and the importer's
//! guess picked. Either answer zooms to it; Zoom and Set Size also opens Set Size on it,
//! ready to type the size wanted. Cancel brings nothing in.

use super::CutCraftApp;
use cut_craft_bug_report::hint::HoverHint;
use cut_craft_core::document::Units;
use cut_craft_core::geometry::Point;
use cut_craft_core::glam::{DAffine2, DVec2};
use cut_craft_core::{Change, CommandError, Edit, UnitGuess};
use eframe::egui;
use fmt_import_dxf::{Fit, fit, other_units};

/// An import held for the question.
pub(super) struct UnitsAsk {
    change: Change,
    /// What the status bar says once it is in.
    done: String,
    guess: UnitGuess,
    /// What it is scaled about if other units are chosen: where the import put the
    /// drawing's origin.
    pivot: Point,
    /// The units picked so far.
    chosen: Units,
}

/// `change`'s new vectors scaled by `factor` about `pivot`.
fn rescaled(mut change: Change, factor: f64, pivot: Point) -> Change {
    let t = DAffine2::from_translation(pivot)
        * DAffine2::from_scale(DVec2::splat(factor))
        * DAffine2::from_translation(-pivot);
    for edit in &mut change.edits {
        if let Edit::Add { path, .. } = edit {
            *path = path.transformed(&t);
        }
    }
    change
}

impl CutCraftApp {
    /// An import's result: held for the units question when the importer had to guess
    /// them, recorded otherwise. `pivot` is where the drawing's origin landed.
    pub(super) fn record_import(
        &mut self,
        change: Result<Change, CommandError>,
        done: &str,
        pivot: Point,
    ) {
        match change {
            Ok(mut change) if change.unit_guess.is_some() => {
                let guess = change.unit_guess.take().expect("just checked");
                self.units_ask = Some(UnitsAsk {
                    change,
                    done: done.to_owned(),
                    guess,
                    pivot,
                    chosen: guess.taken,
                });
            }
            change => self.record(change, done),
        }
    }

    /// The question, over everything, while an import waits on it.
    pub(super) fn units_question(&mut self, ctx: &egui::Context) {
        let Some(ask) = self.units_ask.as_mut() else {
            return;
        };
        let job = self.doc.job;
        let size = ask.guess.size;
        let (mut zoom, mut set_size, mut cancel) = (false, false, false);
        egui::Modal::new(egui::Id::unique("units_ask")).show(ctx, |ui| {
            ui.set_width(ui.spacing().interact_size.x * 11.0);
            ui.heading("Which units is this drawing in?");
            ui.label("This drawing doesn't say what units it was made in");
            ui.add_space(ui.spacing().item_spacing.y);
            // the guess first
            for units in [ask.guess.taken, other_units(ask.guess.taken)] {
                // no hover hint: it would cover the other choice, and the text says it all
                let name = match units {
                    Units::Inch => "Inches",
                    Units::Mm => "Millimetres",
                };
                let how = match fit(size, units, &job) {
                    Fit::Fits => "fits the material",
                    Fit::TooSmall => "too small to see",
                    Fit::TooBig => "too big for the material",
                };
                let text = format!(
                    "{name}: {} by {} ({how})",
                    units.length(size.x * units.per(), 1),
                    units.length(size.y * units.per(), 1)
                );
                ui.radio_value(&mut ask.chosen, units, text);
            }
            ui.separator();
            ui.horizontal(|ui| {
                zoom = ui
                    .button("Zoom to It")
                    .on_hover_hint("Bring it in at this size and zoom the view to it")
                    .clicked();
                set_size = ui
                    .button("Zoom and Set Size…")
                    .on_hover_hint("Bring it in, zoom to it and open Set Size to type its size")
                    .clicked();
                cancel = ui
                    .button("Cancel")
                    .on_hover_hint("Don't bring the drawing in")
                    .clicked();
            });
        });
        if !(zoom || set_size || cancel) {
            return;
        }
        let Some(ask) = self.units_ask.take() else {
            return;
        };
        if cancel {
            self.status.set_message("Import cancelled");
            return;
        }
        let factor = ask.chosen.per() / ask.guess.taken.per();
        let change = rescaled(ask.change, factor, ask.pivot);
        self.record(Ok(change), &ask.done);
        self.invoke(ctx, "zoom_selected");
        if set_size {
            self.invoke(ctx, "set_size");
        }
    }
}
