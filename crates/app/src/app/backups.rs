//! Backing up the settings ([`cut_craft_backup`]): Help ▸ Back Up Settings, the backups made
//! by themselves before big changes and every week, and their list on Options ▸ Backups.

use super::{CutCraftApp, egui, files};
use cut_craft_backup::{Automatic, Choice, Kept, Part, Reason, now};
use cut_craft_bug_report::hint::HoverHint;
use cut_craft_i18n::tr;
use std::path::PathBuf;

/// The backups' state while PowerWhittle runs.
#[derive(Default)]
pub(super) struct Backups {
    /// The settings folder backed up; `None` in tests, which back up nothing unless they say
    /// where.
    pub config: Option<PathBuf>,
    /// Help ▸ Back Up Settings' window while it is open: the parts ticked.
    window: Option<Choice>,
    /// The backups Options ▸ Backups lists, read when the page is opened.
    listed: Option<Vec<Kept>>,
}

/// What was pressed on Options ▸ Backups.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Pressed {
    BackUpNow,
    OpenFolder,
}

/// A backup's date and time, in this computer's time zone.
fn when(seconds: u64) -> String {
    i64::try_from(seconds)
        .ok()
        .and_then(|s| chrono::DateTime::from_timestamp(s, 0))
        .unwrap_or_default()
        .with_timezone(&chrono::Local)
        .format("%Y-%m-%d %H:%M")
        .to_string()
}

impl CutCraftApp {
    /// The automatic backups, keeping as many as Options ▸ Backups says; none in tests.
    pub(super) fn automatic_backups(&self) -> Option<Automatic> {
        let config = self.backups.config.clone()?;
        Some(Automatic::new(
            config,
            self.options.backups.kept(),
            env!("CARGO_PKG_VERSION"),
        ))
    }

    /// A backup made before a big change. One that fails says why in the status bar and the
    /// change goes ahead, as it would have before there were backups.
    pub(super) fn back_up_before(&mut self, reason: Reason) {
        let Some(auto) = self.automatic_backups() else {
            return;
        };
        match auto.make(reason, now()) {
            Ok(made) => {
                if made.is_some() {
                    self.backups.listed = None;
                }
            }
            Err(e) => self
                .status
                .set_message(&tr!("backup-automatic-failed", why = e.to_string())),
        }
    }

    /// At startup: the weekly backup, when Options ▸ Backups wants it and a week has gone by
    /// since the newest. Made in the background, so the window opens straight away.
    pub(super) fn weekly_backup_at_startup(&self) {
        let Some(auto) = self.automatic_backups() else {
            return;
        };
        let now = now();
        if self.options.backups.weekly && auto.weekly_due(now) {
            std::thread::spawn(move || {
                if let Err(e) = auto.make(Reason::Weekly, now) {
                    log::warn!("weekly backup: {e}");
                }
            });
        }
    }

    /// Help ▸ Back Up Settings: the window to choose what goes in.
    pub(super) fn open_back_up_window(&mut self) {
        self.backups.window = Some(Choice::default());
    }

    /// Help ▸ Back Up Settings' window, while it is open: a tick box for each part, then Save
    /// Backup… asks where.
    pub(super) fn back_up_window(&mut self, ctx: &egui::Context) {
        let Some(choice) = self.backups.window.as_mut() else {
            return;
        };
        let mut open = true;
        let id = egui::Id::unique("back_up_settings");
        let size = egui::vec2(460.0, 420.0);
        let title = tr!("backup-window-title");
        let pressed = cut_craft_bug_report::dialog(ctx, id, &title, size, &mut open, |ui| {
            choice_ui(ui, choice)
        })
        .flatten();
        match pressed {
            Some(true) => {
                if let Some(choice) = self.backups.window.take() {
                    self.save_backup(&choice);
                }
            }
            Some(false) => self.backups.window = None,
            None if !open => self.backups.window = None,
            None => {}
        }
    }

    /// Save Backup…: asks where, then writes the backup there.
    fn save_backup(&mut self, choice: &Choice) {
        let Some(config) = self.backups.config.clone() else {
            return;
        };
        let name = tr!("backup-file-name", date = when(now()).replace(':', "-"));
        let ext = cut_craft_backup::EXTENSION;
        let dialog = super::dialogs::filtered_dialog(
            crate::file_filters::Dialog::SaveBackup,
            &self.registry,
        )
        .set_file_name(format!("{name}.{ext}"));
        let Some(path) = dialog
            .save_file()
            .and_then(|p| super::dialogs::saving_path(&p, &[ext]))
        else {
            return;
        };
        let written = cut_craft_backup::write(
            &config,
            choice,
            Reason::ByHand,
            env!("CARGO_PKG_VERSION"),
            now(),
            &path,
        );
        self.status.set_message(&match written {
            Ok(_) => tr!("backup-saved", file = path.display().to_string()),
            Err(e) => tr!("backup-not-saved", why = e.to_string()),
        });
        self.backups.listed = None;
    }

    /// The backups Options ▸ Backups lists, newest first: read when the page is opened, and
    /// again after a backup is made.
    pub(super) fn backups_listed(&mut self) -> Vec<Kept> {
        if self.backups.listed.is_none() {
            self.backups.listed = Some(self.automatic_backups().map_or_default(|a| a.kept()));
        }
        self.backups.listed.clone().unwrap_or_default()
    }

    /// Options ▸ Backups' list is read again the next time the page shows.
    pub(super) fn forget_backups_listed(&mut self) {
        self.backups.listed = None;
    }

    pub(super) fn backups_pressed(&mut self, pressed: Pressed) {
        match pressed {
            Pressed::BackUpNow => self.open_back_up_window(),
            Pressed::OpenFolder => {
                let opened = self
                    .automatic_backups()
                    .ok_or_else(|| tr!("backup-no-settings-folder"))
                    .and_then(|auto| {
                        let folder = auto.folder();
                        std::fs::create_dir_all(&folder).map_err(|e| e.to_string())?;
                        super::dialogs::on_the_desktop();
                        files::show_folder(&folder).map_err(|e| e.to_string())
                    });
                if let Err(e) = opened {
                    self.status
                        .set_message(&tr!("backup-folder-not-opened", why = e));
                }
            }
        }
    }
}

/// The parts to tick and the secrets box: `Some(true)` for Save Backup…, `Some(false)` for
/// Cancel.
fn choice_ui(ui: &mut egui::Ui, choice: &mut Choice) -> Option<bool> {
    ui.label(tr!("backup-window-intro"));
    ui.add_space(4.0);
    for part in Part::ALL {
        let mut ticked = choice.parts.contains(&part);
        if ui
            .checkbox(&mut ticked, tr!(part.key()))
            .on_hover_hint(tr!(&format!("{}.tooltip", part.key())))
            .changed()
        {
            if ticked {
                choice.parts.insert(part);
            } else {
                choice.parts.remove(&part);
            }
        }
    }
    ui.separator();
    ui.checkbox(&mut choice.secrets, tr!("backup-include-secrets"))
        .on_hover_hint(tr!("backup-include-secrets.tooltip"));
    if choice.secrets {
        ui.colored_label(ui.visuals().warn_fg_color, tr!("backup-secrets-warning"));
    }
    ui.add_space(8.0);
    let mut pressed = None;
    ui.horizontal(|ui| {
        let save = ui
            .add_enabled(
                !choice.parts.is_empty(),
                egui::Button::new(tr!("backup-save")),
            )
            .on_hover_hint(tr!("backup-save.tooltip"));
        if save.clicked() {
            pressed = Some(true);
        }
        if ui.button(tr!("backup-cancel")).clicked() {
            pressed = Some(false);
        }
    });
    pressed
}

/// Under Options ▸ Backups' settings: Back Up Now…, Open Backups Folder, and the backups
/// kept.
pub(super) fn list_ui(ui: &mut egui::Ui, listed: &[Kept]) -> Option<Pressed> {
    let mut pressed = None;
    ui.add_space(8.0);
    ui.horizontal(|ui| {
        if ui
            .button(tr!("backup-back-up-now"))
            .on_hover_hint(tr!("backup-back-up-now.tooltip"))
            .clicked()
        {
            pressed = Some(Pressed::BackUpNow);
        }
        if ui
            .button(tr!("backup-open-folder"))
            .on_hover_hint(tr!("backup-open-folder.tooltip"))
            .clicked()
        {
            pressed = Some(Pressed::OpenFolder);
        }
    });
    ui.add_space(4.0);
    ui.strong(tr!("backup-kept-heading"));
    if listed.is_empty() {
        ui.label(tr!("backup-none-yet"));
    }
    egui::Grid::new("backups_kept")
        .striped(true)
        .show(ui, |ui| {
            for kept in listed {
                ui.label(when(kept.manifest.created));
                ui.label(tr!(kept.manifest.reason.key()));
                #[expect(
                    clippy::cast_precision_loss,
                    reason = "a file size shown to one decimal"
                )]
                let megabytes = kept.size as f64 / 1_000_000.0;
                ui.label(tr!("backup-size", megabytes = format!("{megabytes:.1}")));
                ui.end_row();
            }
        });
    pressed
}

#[cfg(test)]
mod tests {
    use cut_craft_backup::{Part, Reason};

    #[test]
    fn every_part_and_reason_has_words() {
        let reasons = [
            Reason::ByHand,
            Reason::BeforeUpdate,
            Reason::BeforeLayoutChange,
            Reason::BeforeSetup,
            Reason::BeforeLibraryChange,
            Reason::Weekly,
        ];
        let keys = Part::ALL
            .iter()
            .flat_map(|p| [p.key().to_owned(), format!("{}.tooltip", p.key())])
            .chain(reasons.iter().map(|r| r.key().to_owned()));
        for key in keys {
            assert!(
                cut_craft_i18n::english().has(&key),
                "{key} has no English words"
            );
        }
    }
}
