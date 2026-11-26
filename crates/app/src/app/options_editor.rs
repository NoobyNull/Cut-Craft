//! What the Options window keeps while it is open: whether it is open, the page shown,
//! the changes not applied yet, and Close's question. The app's options themselves stay
//! with the app; this holds only the window's own state, so every way of opening it, and
//! every way of closing it, goes through the same few steps.

use super::unapplied::Unapplied;
use crate::options::Options;
use serde_json::Value;

#[derive(Default)]
pub(super) struct OptionsEditor {
    /// Whether it is open, the changes not applied yet, and Close's question.
    pub(super) window: Unapplied<Options>,
    /// Job Defaults' job settings as being edited, and why the job they make was refused,
    /// if it was (kept as typed until it is put right).
    pub(super) job_defaults_form: Option<(Value, Option<String>)>,
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
        self.window.open = true;
    }

    /// The category the window was opened at, once: the window then shows it.
    pub(super) fn take_goto(&mut self) -> Option<&'static str> {
        self.goto.take()
    }

    /// Closes the window, throwing away what was not applied (Job Defaults' form too).
    pub(super) fn close(&mut self) {
        self.window.close();
        self.job_defaults_form = None;
    }
}

#[cfg(test)]
mod tests;
