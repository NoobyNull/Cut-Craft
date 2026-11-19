//! The app's side of updating: the startup check, the Updates window's requests, and
//! starting the new version once the window has closed.

use super::CutCraftApp;
use crate::update::Asked;
use eframe::egui;

/// Seconds since 1970, by this computer's clock.
fn now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs())
}

impl CutCraftApp {
    /// About once a day, unless Options ▸ Updates says not to or a package manager looks
    /// after this copy: asks GitHub in the background,
    /// and says something only when there is a new version. Also asks for a new message from
    /// the Cut Craft team ([`crate::announcements`]), unless Options ▸ Updates says not to.
    pub(super) fn check_for_updates_at_startup(&mut self, ctx: &egui::Context) {
        let o = &self.options.updates;
        if o.check_at_startup
            && self.updates.checks_at_startup()
            && cut_craft_updater::due(o.last_check, now())
        {
            self.updates.check(ctx, false, o.skipped_version.clone());
        }
        // and, unless switched off, whether the team has a new message
        if self.options.updates.team_messages {
            self.ask_for_team_messages(ctx, false);
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

    /// Closing went ahead after Install Now or Relaunch Now: starts the new version, on the
    /// job that was open (as saved, if the save question was answered).
    pub(super) fn restart_into_update(&mut self) {
        if let Some(program) = self.restart_after_close.take() {
            let job = self.job_to_reopen();
            if let Err(e) = crate::update::restart(&program, job.as_deref()) {
                log::warn!("{e}");
                let _ = rfd::MessageDialog::new()
                    .set_title(super::APP_NAME)
                    .set_description(e)
                    .show();
            }
        }
    }

    /// The job file the new version opens: the open job's, when it has one on disk.
    pub(super) fn job_to_reopen(&self) -> Option<std::path::PathBuf> {
        if matches!(self.screen, super::Screen::Start) {
            return None;
        }
        self.session.path.clone().filter(|p| p.is_file())
    }
}

impl CutCraftApp {
    /// Asks the Cut Craft team's service for a message: at startup (`manual` false) a new one
    /// opens by itself; from the Help menu every answer is shown.
    pub(super) fn ask_for_team_messages(&mut self, ctx: &egui::Context, manual: bool) {
        let o = &self.options.updates;
        self.announcements
            .ask(ctx, manual, &o.seen_messages, o.votes.clone());
    }

    pub(super) fn team_messages_window(&mut self, ctx: &egui::Context) {
        use crate::announcements::Asked;
        let bug_reports = &mut self.options.bug_reports;
        let mut made_id = false;
        let asked = self.announcements.window(ctx, &mut || {
            bug_reports
                .install_id
                .get_or_insert_with(|| {
                    made_id = true;
                    cut_craft_bug_report::relay::new_install_id()
                })
                .clone()
        });
        let o = &mut self.options.updates;
        match asked {
            // a new install number is kept straight away, so a later vote replaces this one
            Asked::Nothing if made_id => {}
            Asked::Nothing => return,
            Asked::Seen(id) => {
                if !o.seen_messages.contains(&id) {
                    o.seen_messages.push(id);
                }
            }
            Asked::Voted(id, choice) => {
                o.votes.retain(|(p, _)| *p != id);
                o.votes.push((id, choice));
            }
            Asked::TurnOff => o.team_messages = false,
        }
        self.save_options();
    }
}
