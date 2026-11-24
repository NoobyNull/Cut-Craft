//! The status bar's word for how briskly a calculation is going, while one runs: Crawl, Walk,
//! Run or Sprint ([`cut_craft_core::work::Pace`]), each with its own picture and colour, so
//! the reason a calculation is slow is never a mystery. Click it for Options ▸ Performance.

use super::CutCraftApp;
use cut_craft_bug_report::hint::HoverHint;
use cut_craft_core::work::{self, Gauge, Pace};
use cut_craft_i18n::tr;
use eframe::egui;

/// A pace's picture (drawn white, tinted with [`colour`]) and its words' key.
fn look(pace: Pace) -> (&'static str, &'static str) {
    match pace {
        Pace::Crawl => ("view/pace_crawl.png", "panels-pace-crawl"),
        Pace::Walk => ("view/pace_walk.png", "panels-pace-walk"),
        Pace::Run => ("view/pace_run.png", "panels-pace-run"),
        Pace::Sprint => ("view/pace_sprint.png", "panels-pace-sprint"),
    }
}

/// A pace's colour: orange, amber, green, blue; darker on a light theme, to read on it.
pub(super) fn colour(pace: Pace, dark: bool) -> egui::Color32 {
    let (light, on_dark) = match pace {
        Pace::Crawl => ([0xc2, 0x41, 0x0c], [0xf0, 0x91, 0x3a]),
        Pace::Walk => ([0x8f, 0x74, 0x00], [0xe6, 0xc8, 0x4a]),
        Pace::Run => ([0x2e, 0x9a, 0x3e], [0x5f, 0xc4, 0x6c]),
        Pace::Sprint => ([0x2f, 0x6f, 0xd6], [0x6a, 0xa0, 0xf0]),
    };
    let [r, g, b] = if dark { on_dark } else { light };
    egui::Color32::from_rgb(r, g, b)
}

impl CutCraftApp {
    /// Measures the pace once a frame and, while a calculation runs, draws it in the status
    /// bar; whether it drew anything.
    pub(super) fn pace_indicator(&mut self, ui: &mut egui::Ui) -> bool {
        let running = work::running();
        let limit = work::limit();
        self.gauge.tick(
            std::time::Instant::now(),
            work::process_time(),
            running,
            limit,
        );
        if running > 0 {
            // measured again once a second, even while nothing else asks for a frame
            ui.ctx().request_repaint_after(Gauge::EVERY);
        }
        let Some(pace) = self.gauge.pace() else {
            return false;
        };
        let (picture, key) = look(pace);
        let colour = colour(pace, ui.visuals().dark_mode);
        let all = work::all_cores();
        let hint = tr!(
            &format!("{key}.tooltip"),
            cores = limit.cores_of(all),
            all = all,
            share = limit.share
        );
        let edge = ui.text_style_height(&egui::TextStyle::Body);
        let response = ui
            .add(
                egui::Button::image_and_text(
                    self.icons
                        .image(picture)
                        .tint(colour)
                        .fit_to_exact_size(egui::Vec2::splat(edge)),
                    egui::RichText::new(tr!(key)).color(colour).strong(),
                )
                .frame(false),
            )
            .on_hover_cursor(egui::CursorIcon::PointingHand)
            .on_hover_hint(hint);
        crate::marks::note(ui, "status:pace", response.rect);
        if response.clicked() {
            self.options_window = true;
            self.options_goto = Some(crate::options::PAGE);
        }
        true
    }
}
