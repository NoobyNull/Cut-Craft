//! Edit ▸ Options ▸ Language: English, built in, or one of the language packs installed from
//! Gadgets ▸ Add-ons, each with how much of Cut Craft it covers and whether it is official.
//! Like every Options page, the choice is used once Apply is pressed.

use crate::languages::{Pack, TEST_LANGUAGE};
use cut_craft_bug_report::hint::HoverHint;
use cut_craft_i18n::tr;
use eframe::egui;

/// The page: `chosen` is the pack picked (its id; empty for English). The pack picked now,
/// when another was clicked. The test language is offered with the advanced settings shown.
pub(super) fn show(
    ui: &mut egui::Ui,
    packs: &[(Pack, u32)],
    chosen: &str,
    advanced: bool,
) -> Option<String> {
    let mut picked = chosen.to_owned();
    ui.radio_value(&mut picked, String::new(), tr!("language-english"));
    for (pack, percent) in packs {
        ui.horizontal(|ui| {
            ui.radio_value(&mut picked, pack.id.clone(), &pack.name);
            ui.weak(tr!("language-coverage", percent = *percent));
            if !pack.signed {
                ui.colored_label(ui.visuals().warn_fg_color, tr!("language-not-official"))
                    .on_hover_hint(tr!("language-not-official.tooltip"));
            }
        });
    }
    if packs.is_empty() {
        ui.weak(tr!("language-none-installed"));
    }
    if advanced {
        ui.radio_value(&mut picked, TEST_LANGUAGE.to_owned(), tr!("language-test"))
            .on_hover_hint(tr!("language-test.tooltip"));
    }
    ui.separator();
    ui.weak(tr!("language-fallback"));
    (picked != chosen).then_some(picked)
}
