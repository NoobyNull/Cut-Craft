//! Edit ▸ Options ▸ Icons: the flat icons, built in, or one of the icon sets that icon themes
//! installed from Gadgets ▸ Add-ons bring. Whatever a set has no picture for keeps its flat
//! one. Like every Options page, the choice is used once Apply is pressed.

use crate::icon_themes::IconSet;
use cut_craft_bug_report::hint::HoverHint;
use cut_craft_i18n::tr;
use eframe::egui;

/// The page: `chosen` is the set picked (its key; empty for the flat icons). The set picked
/// now, when another was clicked.
pub(super) fn show(ui: &mut egui::Ui, sets: &[IconSet], chosen: &str) -> Option<String> {
    let mut picked = chosen.to_owned();
    ui.radio_value(&mut picked, String::new(), tr!("icons-flat"));
    for set in sets {
        ui.horizontal(|ui| {
            ui.radio_value(&mut picked, set.key.clone(), &set.name);
            if !set.signed {
                ui.colored_label(ui.visuals().warn_fg_color, tr!("language-not-official"))
                    .on_hover_hint(tr!("language-not-official.tooltip"));
            }
        });
    }
    if sets.is_empty() {
        ui.weak(tr!("icons-none-installed"));
    }
    ui.separator();
    ui.weak(tr!("icons-fallback"));
    (picked != chosen).then_some(picked)
}
