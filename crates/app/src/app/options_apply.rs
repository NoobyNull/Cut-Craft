//! Edit ▸ Options' Apply, Cancel and Close: changes wait until Apply, Cancel throws them
//! away, and Close with changes not applied asks which first, so nothing is lost or kept
//! by surprise.

use super::CutCraftApp;
use crate::options::Options;
use eframe::egui;

#[cfg(test)]
pub(super) use cut_craft_bug_report::unapplied::{ALL_APPLIED, ASK, NOT_APPLIED};
pub(super) use cut_craft_bug_report::unapplied::{Answer, Button};

impl CutCraftApp {
    /// The options as the Options window shows them: with any changes not applied yet.
    pub(super) fn edited_options(&self) -> &Options {
        self.options_editor.shown(&self.options)
    }

    /// `edited` is what the Options window now shows: kept until Apply, unless it is the
    /// options as they already are.
    pub(super) fn options_edited(&mut self, edited: Options) {
        self.options_editor.edited(&self.options, edited);
    }

    /// There are changes in the Options window that Apply hasn't used yet.
    pub(super) fn options_dirty(&self) -> bool {
        self.options_editor.dirty()
    }

    /// Uses the changes waiting in the Options window, and keeps them.
    fn apply_options(&mut self, ctx: &egui::Context) {
        if let Some(edited) = self.options_editor.take_changes() {
            self.options_changed(ctx, edited);
        }
    }

    fn close_options(&mut self) {
        self.options_editor.close();
        self.capture = None;
    }

    /// One of the buttons at the foot of the Options window was pressed (or its own close
    /// box, as Close).
    pub(super) fn options_button(&mut self, ctx: &egui::Context, button: Button) {
        match button {
            Button::Apply => self.apply_options(ctx),
            Button::Close if self.options_dirty() => self.options_editor.closing = true,
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
            Answer::KeepEditing => self.options_editor.closing = false,
        }
    }
}

/// The foot of the Options window: whether there are changes waiting, then Close, Cancel
/// and Apply, always in the same places. What was pressed, if anything.
pub(super) fn footer(ui: &mut egui::Ui, dirty: bool) -> Option<Button> {
    cut_craft_bug_report::unapplied::footer(ui, dirty)
}

/// Close's question, over the Options window. The answer, once given.
pub(super) fn question(ctx: &egui::Context) -> Option<Answer> {
    cut_craft_bug_report::unapplied::question(ctx, "options_apply_question")
}
