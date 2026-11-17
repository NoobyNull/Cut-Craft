//! Edit ▸ Options' Apply, Cancel and Close: changes wait until Apply, Cancel throws them
//! away, and Close with changes not applied asks which first, so nothing is lost or kept
//! by surprise.

use super::CutCraftApp;
use crate::options::Options;
use eframe::egui;

/// Shown at the foot of the window while there are changes Apply hasn't used yet.
pub(super) const NOT_APPLIED: &str = "● Changes not applied yet";
/// Shown in the same place once everything is applied.
pub(super) const ALL_APPLIED: &str = "No changes waiting";
/// The question Close asks when there are changes not applied.
pub(super) const ASK: &str = "Apply your changes?";

/// A button at the foot of the Options window.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Button {
    /// Use the changes and keep the window open.
    Apply,
    /// Throw the changes away and close.
    Cancel,
    /// Close, asking first if there are changes not applied.
    Close,
}

/// The answer to [`ASK`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Answer {
    ApplyAndClose,
    Discard,
    KeepEditing,
}

impl CutCraftApp {
    /// The options as the Options window shows them: with any changes not applied yet.
    pub(super) fn edited_options(&self) -> &Options {
        self.options_draft.as_ref().unwrap_or(&self.options)
    }

    /// `edited` is what the Options window now shows: kept until Apply, unless it is the
    /// options as they already are.
    pub(super) fn options_edited(&mut self, edited: Options) {
        self.options_draft = (edited != self.options).then_some(edited);
    }

    /// There are changes in the Options window that Apply hasn't used yet.
    pub(super) fn options_dirty(&self) -> bool {
        self.options_draft.is_some()
    }

    /// Uses the changes waiting in the Options window, and keeps them.
    fn apply_options(&mut self, ctx: &egui::Context) {
        if let Some(edited) = self.options_draft.take() {
            self.options_changed(ctx, edited);
        }
    }

    fn close_options(&mut self) {
        self.options_draft = None;
        self.options_closing = false;
        self.options_window = false;
        self.capture = None;
    }

    /// One of the buttons at the foot of the Options window was pressed (or its own close
    /// box, as Close).
    pub(super) fn options_button(&mut self, ctx: &egui::Context, button: Button) {
        match button {
            Button::Apply => self.apply_options(ctx),
            Button::Close if self.options_dirty() => self.options_closing = true,
            Button::Cancel | Button::Close => self.close_options(),
        }
    }

    /// The answer to Close's question.
    pub(super) fn options_answer(&mut self, ctx: &egui::Context, answer: Answer) {
        match answer {
            Answer::ApplyAndClose => {
                self.apply_options(ctx);
                self.close_options();
            }
            Answer::Discard => self.close_options(),
            Answer::KeepEditing => self.options_closing = false,
        }
    }
}

/// The foot of the Options window: whether there are changes waiting, then Close, Cancel
/// and Apply, always in the same places. What was pressed, if anything.
pub(super) fn footer(ui: &mut egui::Ui, dirty: bool) -> Option<Button> {
    let mut pressed = None;
    ui.horizontal(|ui| {
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            // right to left: Apply sits at the far right, as on most settings windows
            if ui
                .add_enabled(dirty, egui::Button::new("Apply"))
                .on_hover_text("Use the changes now and keep this window open")
                .on_disabled_hover_text("Nothing has changed yet")
                .clicked()
            {
                pressed = Some(Button::Apply);
            }
            if ui
                .button("Cancel")
                .on_hover_text("Throw away the changes not applied yet and close")
                .clicked()
            {
                pressed = Some(Button::Cancel);
            }
            if ui
                .button("Close")
                .on_hover_text(
                    "Close this window; if some changes aren't applied yet, it asks first",
                )
                .clicked()
            {
                pressed = Some(Button::Close);
            }
            ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                if dirty {
                    ui.colored_label(ui.visuals().warn_fg_color, NOT_APPLIED)
                        .on_hover_text(
                            "Your changes take effect when you press Apply. Cancel throws \
                             them away.",
                        );
                } else {
                    ui.weak(ALL_APPLIED);
                }
            });
        });
    });
    pressed
}

/// Close's question, over the Options window. The answer, once given.
pub(super) fn question(ctx: &egui::Context) -> Option<Answer> {
    let mut answer = None;
    let shown = egui::Modal::new(egui::Id::unique("options_apply_question")).show(ctx, |ui| {
        ui.set_width(ui.spacing().interact_size.x * 9.0);
        ui.heading(ASK);
        ui.label("You changed some settings but haven't pressed Apply.");
        ui.separator();
        ui.horizontal(|ui| {
            if ui
                .button("Apply and Close")
                .on_hover_text("Use the changes, keep them, and close")
                .clicked()
            {
                answer = Some(Answer::ApplyAndClose);
            }
            if ui
                .button("Discard Changes")
                .on_hover_text("Throw the changes away and close: the settings stay as they were")
                .clicked()
            {
                answer = Some(Answer::Discard);
            }
            if ui
                .button("Keep Editing")
                .on_hover_text("Go back to the settings without closing")
                .clicked()
            {
                answer = Some(Answer::KeepEditing);
            }
        });
    });
    // Esc, or a click beside it: back to the settings, nothing lost
    if answer.is_none() && shown.should_close() {
        answer = Some(Answer::KeepEditing);
    }
    answer
}
