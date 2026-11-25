//! Changes a settings window keeps until Apply: the note saying some are waiting, the
//! Close / Cancel / Apply row, and the question asked when the window is closed with
//! changes not applied, shared so every settings window says it the same way.

use eframe::egui;

/// Shown at the foot of a window while there are changes Apply hasn't used yet.
pub const NOT_APPLIED: &str = "● Changes not applied yet";
/// Shown in the same place once everything is applied.
pub const ALL_APPLIED: &str = "No changes waiting";
/// The question asked when a window is closed with changes not applied.
pub const ASK: &str = "Apply your changes?";

/// A button along the foot of a settings window.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Button {
    /// Use the changes and keep the window open.
    Apply,
    /// Throw the changes away and close.
    Cancel,
    /// Close, asking first if there are changes not applied.
    Close,
}

/// The answer to [`ASK`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Answer {
    ApplyAndClose,
    Discard,
    KeepEditing,
}

/// [`NOT_APPLIED`] while `dirty`, else [`ALL_APPLIED`], in the same place.
pub fn note(ui: &mut egui::Ui, dirty: bool) {
    if dirty {
        ui.colored_label(ui.visuals().warn_fg_color, NOT_APPLIED)
            .on_hover_text(cut_craft_i18n::t!(
                "Your changes take effect when you press Apply. Cancel throws them away."
            ));
    } else {
        ui.weak(cut_craft_i18n::t!(ALL_APPLIED));
    }
}

/// Apply, greyed out until something changes.
pub fn apply_button(ui: &mut egui::Ui, dirty: bool) -> bool {
    ui.add_enabled(dirty, egui::Button::new(cut_craft_i18n::t!("Apply")))
        .on_hover_text(cut_craft_i18n::t!(
            "Use the changes now and keep this window open"
        ))
        .on_disabled_hover_text(cut_craft_i18n::t!("Nothing has changed yet"))
        .clicked()
}

/// The foot of a settings window: the [`note`], then Close, Cancel and Apply, always in
/// the same places. What was pressed, if anything.
pub fn footer(ui: &mut egui::Ui, dirty: bool) -> Option<Button> {
    let mut pressed = None;
    ui.horizontal(|ui| {
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            // right to left: Apply sits at the far right, as on most settings windows
            if apply_button(ui, dirty) {
                pressed = Some(Button::Apply);
            }
            if ui
                .button(cut_craft_i18n::t!("Cancel"))
                .on_hover_text(cut_craft_i18n::t!(
                    "Throw away the changes not applied yet and close"
                ))
                .clicked()
            {
                pressed = Some(Button::Cancel);
            }
            if ui
                .button(cut_craft_i18n::t!("Close"))
                .on_hover_text(cut_craft_i18n::t!(
                    "Close this window; if some changes aren't applied yet, it asks first"
                ))
                .clicked()
            {
                pressed = Some(Button::Close);
            }
            ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                note(ui, dirty);
            });
        });
    });
    pressed
}

/// Inside a window's drawing: while `dirty`, its own close box asks first rather than
/// closing it. Whether the close box was pressed.
pub fn hold_close(ui: &egui::Ui, dirty: bool) -> bool {
    let asked = ui.ctx().input(|i| i.viewport().close_requested());
    if asked && dirty {
        ui.ctx()
            .send_viewport_cmd(egui::ViewportCommand::CancelClose);
    }
    asked
}

/// The question asked when a window (`salt` tells them apart) is closed with changes not
/// applied. The answer, once given; Esc or a click beside it is Keep Editing.
pub fn question(ctx: &egui::Context, salt: &str) -> Option<Answer> {
    let mut answer = None;
    let shown = egui::Modal::new(egui::Id::unique(salt)).show(ctx, |ui| {
        ui.set_width(ui.spacing().interact_size.x * 9.0);
        ui.heading(cut_craft_i18n::t!(ASK));
        ui.label(cut_craft_i18n::t!(
            "You changed some settings but haven't pressed Apply."
        ));
        ui.separator();
        ui.horizontal(|ui| {
            if ui
                .button(cut_craft_i18n::t!("Apply and Close"))
                .on_hover_text(cut_craft_i18n::t!("Use the changes, keep them, and close"))
                .clicked()
            {
                answer = Some(Answer::ApplyAndClose);
            }
            if ui
                .button(cut_craft_i18n::t!("Discard Changes"))
                .on_hover_text(cut_craft_i18n::t!(
                    "Throw the changes away and close: the settings stay as they were"
                ))
                .clicked()
            {
                answer = Some(Answer::Discard);
            }
            if ui
                .button(cut_craft_i18n::t!("Keep Editing"))
                .on_hover_text(cut_craft_i18n::t!(
                    "Go back to the settings without closing"
                ))
                .clicked()
            {
                answer = Some(Answer::KeepEditing);
            }
        });
    });
    if answer.is_none() && shown.should_close() {
        answer = Some(Answer::KeepEditing);
    }
    answer
}
