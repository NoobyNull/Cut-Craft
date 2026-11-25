//! What the Options window keeps while it is open: whether it is open, the page shown,
//! the changes not applied yet, and Close's question. The app's options themselves stay
//! with the app; this holds only the window's own state, so every way of opening it, and
//! every way of closing it, goes through the same few steps.

use crate::options::Options;
use serde_json::Value;

#[derive(Default)]
pub(super) struct OptionsEditor {
    /// The window is open.
    pub(super) open: bool,
    /// Changes made and not applied yet; `None` when it shows the options as they are.
    /// They take effect only on Apply.
    draft: Option<Options>,
    /// Job Defaults' job settings as being edited, and why the job they make was refused,
    /// if it was (kept as typed until it is put right).
    pub(super) job_defaults_form: Option<(Value, Option<String>)>,
    /// Close was pressed with changes not applied: the window asks Apply or Discard.
    pub(super) closing: bool,
    /// The category shown, by its place in the list.
    pub(super) page: usize,
    /// Show advanced settings is ticked: the Advanced category lists. Off at every start,
    /// so reaching them is always a deliberate step.
    pub(super) show_advanced: bool,
    /// The category to show when the window next draws, by its key.
    goto: Option<&'static str>,
}

impl OptionsEditor {
    /// Opens the window at the category with this key.
    pub(super) fn open_at(&mut self, key: &'static str) {
        self.goto = Some(key);
        self.open = true;
    }

    /// The category the window was opened at, once: the window then shows it.
    pub(super) fn take_goto(&mut self) -> Option<&'static str> {
        self.goto.take()
    }

    /// The options as the window shows them: `options` with any changes not applied yet.
    pub(super) fn shown<'a>(&'a self, options: &'a Options) -> &'a Options {
        self.draft.as_ref().unwrap_or(options)
    }

    /// `edited` is what the window now shows: kept until Apply, unless it is `options`
    /// as they already are.
    pub(super) fn edited(&mut self, options: &Options, edited: Options) {
        self.draft = (edited != *options).then_some(edited);
    }

    /// There are changes that Apply hasn't used yet.
    pub(super) fn dirty(&self) -> bool {
        self.draft.is_some()
    }

    /// The changes waiting, for Apply to use; none wait after.
    pub(super) fn take_changes(&mut self) -> Option<Options> {
        self.draft.take()
    }

    /// The changes waiting, if any, to change along with the options themselves.
    pub(super) fn changes_mut(&mut self) -> Option<&mut Options> {
        self.draft.as_mut()
    }

    /// Closes the window, throwing away what was not applied.
    pub(super) fn close(&mut self) {
        self.draft = None;
        self.job_defaults_form = None;
        self.closing = false;
        self.open = false;
    }
}

#[cfg(test)]
mod tests;
