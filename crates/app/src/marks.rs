//! Where things were drawn on screen, by name, so the help pictures (`--help-pictures`) can
//! ring the button or setting a step is about, whatever the window's layout. Off, and free,
//! unless the help pictures are being taken.
//!
//! Names: `tool:<tool id>` for a tool's button, `field:<setting key>` for a form's setting
//! (its name and its box together), `button:<words>` for a form's own buttons.

use eframe::egui;

fn on_id() -> egui::Id {
    egui::Id::unique("help-marks-on")
}

fn id(name: &str) -> egui::Id {
    egui::Id::unique(("help-mark", name))
}

/// Whether places are being noted.
pub fn on(ctx: &egui::Context) -> bool {
    ctx.data(|d| d.get_temp::<bool>(on_id()).unwrap_or(false))
}

/// Starts noting places (the help pictures' run only).
pub fn switch_on(ctx: &egui::Context) {
    ctx.data_mut(|d| d.insert_temp(on_id(), true));
}

fn wanted_id() -> egui::Id {
    egui::Id::unique("help-mark-wanted")
}

/// Asks for `name` to be scrolled into view wherever it is drawn (a setting low on a long
/// form), until another is wanted.
pub fn want(ctx: &egui::Context, name: &str) {
    ctx.data_mut(|d| d.insert_temp(wanted_id(), name.to_owned()));
}

fn wanted(ctx: &egui::Context, name: &str) -> bool {
    ctx.data(|d| d.get_temp::<String>(wanted_id()))
        .is_some_and(|w| w == name)
}

/// Notes that `name` was drawn at `rect` this frame, in `ui`; scrolls it into view when it is
/// [`want`]ed.
pub fn note(ui: &egui::Ui, name: &str, rect: egui::Rect) {
    if keep(ui.ctx(), name, rect) && !ui.clip_rect().contains_rect(rect) {
        ui.scroll_to_rect(rect, Some(egui::Align::Center));
    }
}

/// Keeps where `name` was drawn, while noting is on; whether it is the one wanted.
fn keep(ctx: &egui::Context, name: &str, rect: egui::Rect) -> bool {
    on(ctx) && {
        ctx.data_mut(|d| d.insert_temp(id(name), rect));
        wanted(ctx, name)
    }
}

/// Where `name` was last drawn.
pub fn get(ctx: &egui::Context, name: &str) -> Option<egui::Rect> {
    ctx.data(|d| d.get_temp::<egui::Rect>(id(name)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn places_are_noted_only_once_switched_on() {
        let ctx = egui::Context::default();
        let rect = egui::Rect::from_min_size(egui::pos2(10.0, 20.0), egui::vec2(30.0, 40.0));
        assert!(!keep(&ctx, "tool:pocket_toolpath", rect));
        assert_eq!(get(&ctx, "tool:pocket_toolpath"), None, "off: nothing kept");
        switch_on(&ctx);
        assert!(!keep(&ctx, "tool:pocket_toolpath", rect), "not wanted");
        assert_eq!(get(&ctx, "tool:pocket_toolpath"), Some(rect));
        want(&ctx, "tool:pocket_toolpath");
        assert!(
            keep(&ctx, "tool:pocket_toolpath", rect),
            "wanted: scrolled to"
        );
        assert_eq!(get(&ctx, "tool:profile_toolpath"), None);
    }
}
