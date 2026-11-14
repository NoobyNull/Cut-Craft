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

/// Notes that `name` was drawn at `rect` this frame.
pub fn note(ctx: &egui::Context, name: &str, rect: egui::Rect) {
    if on(ctx) {
        ctx.data_mut(|d| d.insert_temp(id(name), rect));
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
        note(&ctx, "tool:pocket_toolpath", rect);
        assert_eq!(get(&ctx, "tool:pocket_toolpath"), None, "off: nothing kept");
        switch_on(&ctx);
        note(&ctx, "tool:pocket_toolpath", rect);
        assert_eq!(get(&ctx, "tool:pocket_toolpath"), Some(rect));
        assert_eq!(get(&ctx, "tool:profile_toolpath"), None);
    }
}
