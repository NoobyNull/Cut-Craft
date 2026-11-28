//! Edit ▸ Options ▸ Language: English, built in, or one of the language packs installed from
//! Gadgets ▸ Add-ons, each with how much of PowerWhittle it covers, who translated it and
//! whether it is official (official packs are community supported: wording goes to their
//! translators).
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
            ui.weak(crate::languages::support_lines(pack, Some(*percent)).join(" · "));
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
    // the pack picked is written and kept up by volunteers: wording goes to them
    if let Some((pack, _)) = packs.iter().find(|(p, _)| p.id == picked) {
        ui.horizontal_wrapped(|ui| {
            ui.weak(tr!("language-volunteers", name = pack.name.as_str()));
            ui.hyperlink_to(
                tr!("language-report-a-word"),
                crate::languages::wording_issues(pack),
            );
        });
    }
    (picked != chosen).then_some(picked)
}
