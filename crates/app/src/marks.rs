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

fn spotlit_id() -> egui::Id {
    egui::Id::unique("help-mark-spotlit")
}

/// Rings `name` wherever it is drawn, and scrolls it into view, until [`clear_spotlight`]:
/// a setting a hint sends the person to (Turn on advanced rotary mode…).
pub fn spotlight(ctx: &egui::Context, name: &str) {
    ctx.data_mut(|d| d.insert_temp(spotlit_id(), name.to_owned()));
}

/// Stops ringing what [`spotlight`] rang.
pub fn clear_spotlight(ctx: &egui::Context) {
    ctx.data_mut(|d| d.remove::<String>(spotlit_id()));
}

/// Whether `name` is the one [`spotlight`] rings.
pub fn spotlit(ctx: &egui::Context, name: &str) -> bool {
    ctx.data(|d| d.get_temp::<String>(spotlit_id()))
        .is_some_and(|w| w == name)
}

/// Notes that `name` was drawn at `rect` this frame, in `ui`; scrolls it into view when it is
/// [`want`]ed, and rings it when it is [`spotlight`]ed.
pub fn note(ui: &egui::Ui, name: &str, rect: egui::Rect) {
    let lit = spotlit(ui.ctx(), name);
    if lit {
        let stroke = egui::Stroke::new(2.0, ui.visuals().selection.stroke.color);
        ui.painter()
            .rect_stroke(rect.expand(3.0), 4.0, stroke, egui::StrokeKind::Outside);
    }
    if (keep(ui.ctx(), name, rect) || lit) && !ui.clip_rect().contains_rect(rect) {
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

/// Lets go of where `name` was last drawn, so [`get`] finds it again only once it is drawn
/// anew (the How-to suite, before each tour step).
#[cfg(test)]
pub fn forget(ctx: &egui::Context, name: &str) {
    ctx.data_mut(|d| d.remove::<egui::Rect>(id(name)));
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
