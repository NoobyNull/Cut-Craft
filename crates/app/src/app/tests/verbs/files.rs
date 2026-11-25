//! Files: saving, opening, importing, recovery copies, updating and closing.

use super::super::*;
use crate::clipart::ToLevel;
use cut_craft_core::geometry::Point;

impl Harness {
    // ---- files ----

    /// File ▸ Save As to `path`.
    pub(in super::super) fn save_to(&mut self, path: &std::path::Path) {
        self.app.save_to(path);
    }

    /// File ▸ Open of `path`: whether it opened.
    pub(in super::super) fn open(&mut self, path: &std::path::Path) -> bool {
        self.app.open_path(path)
    }

    /// Clipart dropped at `at` on the view (or imported), onto `level` if given.
    pub(in super::super) fn drop_clipart(
        &mut self,
        path: &std::path::Path,
        at: Option<Point>,
        level: Option<ToLevel>,
    ) {
        self.app.import_clipart(path, at, level);
    }

    // ---- updating, recovery and closing ----

    /// Whether the Updates window is open.
    pub(in super::super) fn updates_window_open(&self) -> bool {
        self.app.updates.is_open()
    }

    /// Starts this run as run `run` with its settings in `settings`: where its recovery copy
    /// goes.
    pub(in super::super) fn given_settings_folder(
        &mut self,
        settings: &std::path::Path,
        run: &str,
    ) {
        self.app.claim_recovery(settings, run);
    }

    /// No folder for recovery copies (the settings folder could not be made).
    pub(in super::super) fn given_no_recovery_folder(&mut self) {
        self.app.recovery_path = None;
    }

    /// Opens the work an earlier run left, as starting does: whether there was any.
    pub(in super::super) fn open_recovered_work(&mut self) -> bool {
        self.app.open_recovery()
    }

    /// Install Now, as far as handing the open work over to the new version.
    pub(in super::super) fn hand_off_for_update(&mut self) -> crate::app::job_files::HandOff {
        self.app.hand_off_for_update()
    }

    /// The file the new version opens after an update, if it opens one.
    pub(in super::super) fn job_to_reopen(&self) -> Option<std::path::PathBuf> {
        self.app.job_to_reopen()
    }

    /// The window's close button: whether it closes.
    pub(in super::super) fn close_window(&mut self) -> bool {
        self.app.close_requested()
    }

    /// The job's file, if it has one (None: Untitled).
    pub(in super::super) fn job_path(&self) -> Option<&std::path::Path> {
        self.app.session.path.as_deref()
    }

    /// Whether the job has changes not yet saved.
    pub(in super::super) fn has_unsaved_changes(&self) -> bool {
        self.app.session.is_dirty(&self.app.history)
    }

    /// The status bar's message.
    pub(in super::super) fn status_message(&self) -> &str {
        &self.app.status.message
    }

    /// Whether a recovery copy is being written now.
    pub(in super::super) fn writing_recovery_copy(&self) -> bool {
        self.app.autosave.is_some()
    }

    /// Waits for the recovery copy being written to be done.
    pub(in super::super) fn finish_recovery_copy(&mut self) {
        let (_, writing) = self.app.autosave.take().expect("a copy is being written");
        writing.join().unwrap().unwrap();
    }
}
