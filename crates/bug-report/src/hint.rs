//! The status bar's hint: what the thing under the pointer does, or what to click next. Any
//! window sets it while drawing (a hovered tooltip through [`HoverHint`], a tool's next step
//! through [`hint`]); the status bar shows it in place of the last message while it is fresh.
//! Kept here, the one egui crate every window shares.

use eframe::egui;

/// Where the hint is kept: its text and the pass that set it.
fn id() -> egui::Id {
    egui::Id::unique("status_hint")
}

/// Sets this frame's hint.
pub fn hint(ctx: &egui::Context, text: &str) {
    let pass = ctx.cumulative_pass_nr();
    ctx.data_mut(|d| d.insert_temp(id(), (pass, text.to_owned())));
}

/// The hint set this frame or the last (the status bar may be drawn before what sets it).
pub fn current(ctx: &egui::Context) -> Option<String> {
    let pass = ctx.cumulative_pass_nr();
    ctx.data(|d| d.get_temp::<(u64, String)>(id()))
        .filter(|(set, _)| pass.saturating_sub(*set) <= 1)
        .map(|(_, text)| text)
}

/// A tooltip that is the status bar's hint too while hovered.
pub trait HoverHint {
    #[expect(
        clippy::return_self_not_must_use,
        reason = "as egui's on_hover_text, the response is often not wanted"
    )]
    fn on_hover_hint(self, text: impl Into<egui::WidgetText>) -> Self;
}

impl HoverHint for egui::Response {
    fn on_hover_hint(self, text: impl Into<egui::WidgetText>) -> Self {
        let text = text.into();
        if self.hovered() {
            hint(&self.ctx, text.text());
        }
        self.on_hover_text(text)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_hovered_tooltip_is_the_hint_until_the_frame_after_next() {
        let ctx = egui::Context::default();
        let mut input = egui::RawInput::default();
        input
            .events
            .push(egui::Event::PointerMoved(egui::pos2(10.0, 10.0)));
        let run = |ctx: &egui::Context, input: egui::RawInput, hover: bool| {
            // no renderer takes a headless frame's texture changes
            ctx.run_ui(input, |ui| {
                egui::CentralPanel::default().show(ui, |ui| {
                    let r = ui.button("Go");
                    if hover {
                        let _ = r.on_hover_hint("Goes");
                    }
                });
            })
            .textures_delta
            .clear();
        };
        run(&ctx, input.clone(), true);
        run(&ctx, input, true);
        assert_eq!(current(&ctx).as_deref(), Some("Goes"));
        run(&ctx, egui::RawInput::default(), false);
        run(&ctx, egui::RawInput::default(), false);
        assert_eq!(current(&ctx), None, "gone once nothing sets it");
    }
}
