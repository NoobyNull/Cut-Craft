//! The app's side of updating: the startup check, the Updates window's requests, and
//! starting the new version once the window has closed.

use super::CutCraftApp;
use super::job_files::HandOff;
use crate::update::Asked;
use eframe::egui;

/// Seconds since 1970, by this computer's clock.
fn now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs())
}

impl CutCraftApp {
    /// Every start, unless Options ▸ Updates says not to or a package manager looks after
    /// this copy: asks GitHub in the background, and says something only when there is a
    /// new version.
    pub(super) fn check_for_updates_at_startup(&mut self, ctx: &egui::Context) {
        let o = &self.options.updates;
        if o.check_at_startup && self.updates.checks_at_startup() {
            self.updates.check(ctx, false, o.skipped_version.clone());
        }
    }

    pub(super) fn updates_window(&mut self, ctx: &egui::Context) {
        match self.updates.window(ctx) {
            Asked::Nothing => {}
            Asked::Checked => {
                self.options.updates.last_check = Some(now());
                self.save_options();
            }
            Asked::Skip(version) => {
                self.options.updates.skipped_version = Some(version);
                self.save_options();
            }
            Asked::Restart(program) => {
                // closing asks to save unsaved work first, then starts the new version
                self.restart_after_close = Some(program);
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            }
        }
    }

    /// The window was asked to close: whether it closes. After Install Now or Relaunch Now
    /// unsaved work is kept for the new version without asking (else "Save changes?" is
    /// asked, as for any close); once kept, a repeated request closes without asking again.
    pub(super) fn close_requested(&mut self) -> bool {
        if self.work_kept_for_update {
            // handed to the new version: nothing to ask, and nothing to tidy away
            return true;
        }
        if self.restart_after_close.is_some() {
            match self.hand_off_for_update() {
                HandOff::Kept => {
                    self.restart_into_update();
                    return true;
                }
                HandOff::NothingUnsaved => {
                    self.clear_recovery();
                    self.restart_into_update();
                    return true;
                }
                HandOff::NotKept => {}
            }
        }
        if self.confirm_discard() {
            // asked once: a repeated close request must not ask again
            self.session.discard_changes(&self.history);
            self.clear_recovery();
            self.restart_into_update();
            true
        } else {
            // Cancel at "Save changes?" cancels the relaunch after an update too
            self.restart_after_close = None;
            false
        }
    }

    /// Closing went ahead after Install Now or Relaunch Now: starts the new version, on the
    /// job that was open (as saved, if the save question was answered).
    pub(super) fn restart_into_update(&mut self) {
        if let Some(program) = self.restart_after_close.take() {
            let job = self.job_to_reopen();
            if let Err(e) = crate::update::restart(&program, job.as_deref()) {
                log::warn!("{e}");
                // the new version did not start: the kept work is recovered next start as
                // work left behind, not as an update's
                if self.work_kept_for_update
                    && let Some(copy) = &self.recovery_path
                {
                    let note = crate::files::RecoveryNote {
                        path: self.session.path.clone(),
                        after_update: false,
                    };
                    let _ = crate::files::write_recovery(
                        copy,
                        &cut_craft_core::file::save_job(&self.doc),
                        &note,
                    );
                }
                let _ = rfd::MessageDialog::new()
                    .set_title(super::APP_NAME)
                    .set_description(e)
                    .show();
            }
        }
    }

    /// The job file the new version opens: the open job's, when it has one on disk.
    pub(super) fn job_to_reopen(&self) -> Option<std::path::PathBuf> {
        // the kept copy opens instead, unsaved
        if matches!(self.screen, super::Screen::Start) || self.work_kept_for_update {
            return None;
        }
        self.session.path.clone().filter(|p| p.is_file())
    }
}
