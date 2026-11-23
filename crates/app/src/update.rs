//! Help ▸ Check for Updates and the check at startup: the window that offers a new version
//! ("Download, install and relaunch?"), shows the download, and once it is installed closes
//! Cut Craft (asking to save first) and opens the new version on the same job. The work is
//! [`cut_craft_updater`]'s, on a thread of its own so the window keeps drawing.

use cut_craft_i18n::tr;
use cut_craft_updater::{Found, Install, RELEASES_PAGE, Step, Version};
use eframe::egui;
use std::path::PathBuf;
use std::sync::mpsc::{Receiver, channel};

/// Where an update has got to.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Phase {
    Idle,
    Checking,
    /// A check's answer: up to date, or what there is.
    Found(Found),
    Working(Step),
    /// Installed: the program to start.
    Installed(PathBuf),
    Failed(String),
}

/// What the background thread says.
enum Msg {
    Checked(Result<Found, String>),
    Step(Step),
    Done(Result<PathBuf, String>),
}

/// What the window asks the app to do.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Asked {
    Nothing,
    /// A check finished: keep when, so the next start does not ask again too soon.
    Checked,
    /// Skip This Version: not offered at startup again.
    Skip(String),
    /// Close Cut Craft (asking to save as usual) and start this program: Relaunch Now, or
    /// Install Now's download finished.
    Restart(PathBuf),
}

pub struct Updates {
    pub phase: Phase,
    /// The window is showing.
    pub open: bool,
    /// The check was asked for (Help ▸ Check for Updates), so every answer is shown; the
    /// startup check only speaks up about a new version.
    manual: bool,
    /// Skip This Version's version, which the startup check keeps quiet about.
    skipped: Option<String>,
    /// The version found, for the window's words once it is installed.
    offered: Option<Version>,
    /// Install Now was pressed: relaunch as soon as the new version is installed.
    relaunch: bool,
    install: Install,
    current: Version,
    rx: Option<Receiver<Msg>>,
}

impl Default for Updates {
    fn default() -> Self {
        Self::new(
            cut_craft_updater::this_install(),
            cut_craft_updater::current_version(),
        )
    }
}

impl Updates {
    pub fn new(install: Install, current: Version) -> Self {
        Self {
            phase: Phase::Idle,
            open: false,
            manual: false,
            skipped: None,
            offered: None,
            relaunch: false,
            install,
            current,
            rx: None,
        }
    }

    /// Shows the window at `phase`, as the help's pictures do (`--screenshots`).
    pub fn show(&mut self, phase: Phase) {
        if let Phase::Found(Found::Release { release, .. }) = &phase {
            self.offered = Some(release.version.clone());
        }
        (self.phase, self.open) = (phase, true);
    }

    /// Whether the Updates window is showing.
    pub fn is_open(&self) -> bool {
        self.open
    }

    /// Whether the startup check runs for this copy (not when a package manager updates it).
    pub fn checks_at_startup(&self) -> bool {
        self.install.checks_at_startup()
    }

    fn busy(&self) -> bool {
        matches!(self.phase, Phase::Checking | Phase::Working(_))
    }

    /// Asks GitHub in the background; `manual` shows the window and every answer.
    pub fn check(&mut self, ctx: &egui::Context, manual: bool, skipped: Option<String>) {
        if manual {
            self.open = true;
        }
        // a check or download already going: the window shows it
        if self.busy() {
            self.manual |= manual;
            return;
        }
        (self.manual, self.skipped, self.phase) = (manual, skipped, Phase::Checking);
        let (tx, rx) = channel();
        self.rx = Some(rx);
        let (install, current, ctx) = (self.install.clone(), self.current.clone(), ctx.clone());
        std::thread::spawn(move || {
            let _ = tx.send(Msg::Checked(cut_craft_updater::check(&install, &current)));
            ctx.request_repaint();
        });
    }

    /// Install Now: downloads and installs what the check found, then relaunches.
    fn start_install(&mut self, ctx: &egui::Context) {
        let Phase::Found(found) = &self.phase else {
            return;
        };
        let found = found.clone();
        self.relaunch = true;
        self.phase = Phase::Working(Step::Downloading { done: 0, total: 0 });
        let (tx, rx) = channel();
        self.rx = Some(rx);
        let (install, ctx) = (self.install.clone(), ctx.clone());
        std::thread::spawn(move || {
            let step = |s| {
                let _ = tx.send(Msg::Step(s));
                ctx.request_repaint();
            };
            let done = cut_craft_updater::install(&install, &found, &step);
            let _ = tx.send(Msg::Done(done));
            ctx.request_repaint();
        });
    }

    /// Takes in what the background thread has said: a check finished, or the new version
    /// was installed and Install Now asked for a relaunch.
    fn poll(&mut self) -> Asked {
        let Some(rx) = &self.rx else {
            return Asked::Nothing;
        };
        let msgs: Vec<Msg> = rx.try_iter().collect();
        let mut asked = Asked::Nothing;
        for msg in msgs {
            match msg {
                Msg::Checked(answer) => {
                    if answer.is_ok() {
                        asked = Asked::Checked;
                    }
                    self.answered(answer);
                }
                Msg::Step(step) => self.phase = Phase::Working(step),
                Msg::Done(done) => {
                    self.phase = match done {
                        Ok(program) => {
                            if std::mem::take(&mut self.relaunch) {
                                asked = Asked::Restart(program.clone());
                            }
                            Phase::Installed(program)
                        }
                        Err(e) => {
                            self.relaunch = false;
                            log::warn!("the update failed: {e}");
                            Phase::Failed(e)
                        }
                    };
                    // shown if the relaunch is called off (Cancel at "Save changes?")
                    self.open = true;
                }
            }
        }
        asked
    }

    /// The check's answer: shown when asked for, or when the startup check found a version
    /// that was not skipped; otherwise only logged.
    fn answered(&mut self, answer: Result<Found, String>) {
        let show = match &answer {
            _ if self.manual => true,
            Ok(Found::Release { release, .. }) => {
                self.skipped.as_deref() != Some(release.version.to_string().as_str())
            }
            Ok(Found::UpToDate) => false,
            Err(e) => {
                log::info!("the update check did not finish: {e}");
                false
            }
        };
        if let Ok(Found::Release { release, .. }) = &answer {
            self.offered = Some(release.version.clone());
        }
        self.phase = match answer {
            Ok(found) if show => Phase::Found(found),
            Err(e) if show => Phase::Failed(e),
            // quiet: nothing is left for the window to show later
            _ => Phase::Idle,
        };
        self.open |= show;
    }

    /// The window, when open; what it asks the app to do.
    pub fn window(&mut self, ctx: &egui::Context) -> Asked {
        let polled = self.poll();
        if !self.open || matches!(polled, Asked::Restart(_)) {
            return polled;
        }
        let mut open = true;
        let id = egui::Id::unique("updates_window");
        let size = egui::vec2(460.0, 320.0);
        let pressed =
            cut_craft_bug_report::dialog(ctx, id, "Cut Craft Updates", size, &mut open, |ui| {
                ui.set_max_width(440.0);
                self.body(ui)
            })
            .unwrap_or(Press::Nothing);
        let mut asked = polled;
        let close = match pressed {
            Press::Nothing => false,
            Press::Close => true,
            Press::InstallNow => {
                self.start_install(ctx);
                false
            }
            Press::TryAgain => {
                let skipped = self.skipped.clone();
                self.check(ctx, true, skipped);
                false
            }
            Press::Skip(version) => {
                asked = Asked::Skip(version);
                true
            }
            Press::Restart(program) => {
                asked = Asked::Restart(program);
                false
            }
        };
        if close || !open {
            self.open = false;
            // a finished answer is not shown again; a download keeps going
            if !self.busy() && !matches!(self.phase, Phase::Installed(_)) {
                self.phase = Phase::Idle;
            }
        }
        asked
    }

    /// The window's words and buttons for where the update has got to.
    fn body(&self, ui: &mut egui::Ui) -> Press {
        match &self.phase {
            Phase::Idle => Press::Close,
            Phase::Checking => {
                ui.horizontal(|ui| {
                    ui.spinner();
                    ui.label(tr!("update-asking-github-for-the-newest"));
                });
                Press::Nothing
            }
            Phase::Found(Found::UpToDate) => {
                ui.heading(tr!("update-cut-craft-is-up-to"));
                ui.label(tr!("update-newest", version = self.current.to_string()));
                ui.add_space(8.0);
                pressed(ui.button(tr!("update-close")).clicked(), Press::Close)
            }
            Phase::Found(Found::Release { release, ready }) => {
                offer(ui, release, ready, &self.current)
            }
            Phase::Working(step) => {
                let (words, fraction) = progress(*step);
                ui.label(words);
                let bar = egui::ProgressBar::new(fraction.unwrap_or(0.0)).show_percentage();
                ui.add(if fraction.is_some() {
                    bar
                } else {
                    bar.animate(true)
                });
                ui.add_space(4.0);
                ui.weak(tr!("update-you-can-close-this-window"));
                Press::Nothing
            }
            Phase::Installed(program) => {
                ui.heading(self.offered.as_ref().map_or_else(
                    || tr!("update-installed"),
                    |v| tr!("update-version-installed", version = v.to_string()),
                ));
                ui.label(tr!("update-relaunch-cut-craft-to-start"));
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    let now = ui
                        .button(egui::RichText::new(tr!("update-relaunch-now")).strong())
                        .clicked();
                    let later = ui
                        .button(tr!("update-relaunch-later"))
                        .on_hover_text(tr!("update-the-new-version-starts-the"))
                        .clicked();
                    if now {
                        Press::Restart(program.clone())
                    } else {
                        pressed(later, Press::Close)
                    }
                })
                .inner
            }
            Phase::Failed(e) => {
                ui.heading(tr!("update-the-update-did-not-finish"));
                ui.label(e);
                ui.label(tr!("update-this-copy-of-cut-craft"));
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    let again = ui.button(tr!("update-try-again")).clicked();
                    if ui.button(tr!("update-open-the-download-page")).clicked() {
                        ui.ctx().open_url(egui::OpenUrl::new_tab(RELEASES_PAGE));
                    }
                    let close = ui.button(tr!("update-close")).clicked();
                    if again {
                        Press::TryAgain
                    } else {
                        pressed(close, Press::Close)
                    }
                })
                .inner
            }
        }
    }
}

/// A button pressed in the window.
enum Press {
    Nothing,
    Close,
    InstallNow,
    TryAgain,
    Skip(String),
    Restart(PathBuf),
}

fn pressed(clicked: bool, press: Press) -> Press {
    if clicked { press } else { Press::Nothing }
}

/// A newer release: "Download, install and relaunch?" with what changed, and Install Now
/// (or, when this copy cannot replace itself, how to update it, with its commands to copy),
/// Later and Skip This Version.
fn offer(
    ui: &mut egui::Ui,
    release: &cut_craft_updater::Release,
    ready: &Result<(), String>,
    current: &Version,
) -> Press {
    ui.heading(offer_heading(&release.version));
    ui.label(tr!("update-you-have", version = current.to_string()));
    ui.add_space(4.0);
    if !release.notes.is_empty() {
        egui::CollapsingHeader::new(tr!("update-what-s-new"))
            .id_salt("update_whats_new")
            .default_open(true)
            .show(ui, |ui| {
                egui::ScrollArea::vertical()
                    .max_height(140.0)
                    .show(ui, |ui| ui.label(&release.notes));
            });
    }
    if !release.page.is_empty() {
        ui.hyperlink_to(tr!("update-see-this-version-on-github"), &release.page);
    }
    ui.add_space(8.0);
    match ready {
        Ok(()) => {
            ui.label(egui::RichText::new(tr!("update-download-install-and-relaunch")).strong());
            ui.label(tr!("update-install-now-downloads-it-from"));
        }
        Err(why) => {
            ui.label(egui::RichText::new(tr!("update-update-this-copy-yourself")).strong());
            // the commands are shown below, each with Copy
            ui.label(why.replace('`', ""));
            for command in commands_in(why) {
                ui.horizontal(|ui| {
                    ui.add(egui::Label::new(egui::RichText::new(command).monospace()).wrap());
                    if ui
                        .small_button(tr!("update-copy"))
                        .on_hover_text(tr!("update-copy-this-command-to-paste"))
                        .clicked()
                    {
                        ui.ctx().copy_text(command.to_owned());
                    }
                });
            }
        }
    }
    ui.add_space(8.0);
    let press = ui
        .horizontal(|ui| {
            let mut press = Press::Nothing;
            if ready.is_ok() {
                if ui
                    .button(egui::RichText::new(tr!("update-install-now")).strong())
                    .on_hover_text(tr!("update-download-install-and-relaunch-cut"))
                    .clicked()
                {
                    press = Press::InstallNow;
                }
            } else if ui.button(tr!("update-open-the-download-page")).clicked() {
                let page = if release.page.is_empty() {
                    RELEASES_PAGE
                } else {
                    &release.page
                };
                ui.ctx().open_url(egui::OpenUrl::new_tab(page));
            }
            if ui
                .button(tr!("update-later"))
                .on_hover_text(tr!("update-ask-again-the-next-time"))
                .clicked()
            {
                press = Press::Close;
            }
            if ui
                .button(tr!("update-skip-this-version"))
                .on_hover_text(tr!("update-not-offered-again-when-cut"))
                .clicked()
            {
                press = Press::Skip(release.version.to_string());
            }
            press
        })
        .inner;
    ui.add_space(4.0);
    ui.weak(tr!("update-automatic-checks-can-be-turned"));
    press
}

/// The offer's heading.
fn offer_heading(version: &Version) -> String {
    tr!("update-new-version", version = version.to_string())
}

/// The commands in a how-to-update message: each part written in `backticks`, to copy.
fn commands_in(why: &str) -> Vec<&str> {
    why.split('`')
        .skip(1)
        .step_by(2)
        .map(str::trim)
        .filter(|c| !c.is_empty())
        .collect()
}

/// What an update step says, and how far along it is when that is known.
fn progress(step: Step) -> (String, Option<f32>) {
    #[expect(clippy::cast_precision_loss, reason = "megabytes to one decimal place")]
    let mb = |bytes: u64| bytes as f64 / 1_000_000.0;
    match step {
        Step::Downloading { done, total } if total > 0 => (
            tr!(
                "update-downloading-of",
                done = format!("{:.1}", mb(done)),
                total = format!("{:.1}", mb(total))
            ),
            #[expect(clippy::cast_possible_truncation, reason = "a fraction")]
            Some((mb(done) / mb(total)).min(1.0) as f32),
        ),
        Step::Downloading { .. } => (tr!("update-downloading"), None),
        Step::Checking => (tr!("update-checking-signed"), None),
        Step::Installing => (tr!("update-installing"), None),
    }
}

/// Starts the new version on the job `open`, if one was open; called once closing went
/// ahead after Install Now or Relaunch Now.
pub fn restart(program: &std::path::Path, open: Option<&std::path::Path>) -> Result<(), String> {
    cut_craft_updater::restart(program, open)
        .map_err(|e| tr!("update-could-not-restart", error = e.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use cut_craft_updater::{Asset, Release};

    fn updates() -> Updates {
        Updates::new(
            Install::Elsewhere { why: "test".into() },
            Version::new(0, 4, 3),
        )
    }

    fn found(version: &str) -> Found {
        Found::Release {
            release: Release {
                tag: format!("v{version}"),
                version: Version::parse(version).unwrap(),
                notes: String::new(),
                page: String::new(),
                assets: Vec::<Asset>::new(),
            },
            ready: Ok(()),
        }
    }

    #[test]
    fn a_package_managers_copy_does_not_check_at_startup() {
        let packaged = Updates::new(
            Install::Packaged { why: "apt".into() },
            Version::new(0, 6, 5),
        );
        assert!(!packaged.checks_at_startup());
        assert!(updates().checks_at_startup());
    }

    #[test]
    fn the_startup_check_only_speaks_up_about_a_new_version() {
        let mut u = updates();
        u.answered(Ok(Found::UpToDate));
        assert!(!u.open);
        assert_eq!(u.phase, Phase::Idle);
        u.answered(Err("offline".into()));
        assert!(!u.open, "no network at startup is not worth a window");
        u.answered(Ok(found("0.5.0")));
        assert!(u.open);
        assert!(matches!(u.phase, Phase::Found(Found::Release { .. })));
    }

    #[test]
    fn a_skipped_version_is_not_offered_at_startup() {
        let mut u = updates();
        u.skipped = Some("0.5.0".into());
        u.answered(Ok(found("0.5.0")));
        assert!(!u.open);
        u.answered(Ok(found("0.5.1")));
        assert!(u.open, "a later version is offered");
    }

    #[test]
    fn checking_by_hand_shows_every_answer() {
        let mut u = updates();
        u.manual = true;
        u.skipped = Some("0.5.0".into());
        u.answered(Ok(found("0.5.0")));
        assert!(
            u.open,
            "Help ▸ Check for Updates still finds a skipped version"
        );
        let mut u = updates();
        u.manual = true;
        u.answered(Ok(Found::UpToDate));
        assert_eq!(u.phase, Phase::Found(Found::UpToDate));
        let mut u = updates();
        u.manual = true;
        u.answered(Err("Could not reach GitHub".into()));
        assert_eq!(u.phase, Phase::Failed("Could not reach GitHub".into()));
    }

    #[test]
    fn progress_says_how_far_the_download_is() {
        let (words, fraction) = progress(Step::Downloading {
            done: 7_340_000,
            total: 14_680_000,
        });
        assert_eq!(words, "Downloading... 7.3 of 14.7 MB");
        assert_eq!(fraction, Some(0.5));
        assert_eq!(progress(Step::Downloading { done: 5, total: 0 }).1, None);
        assert!(progress(Step::Checking).0.contains("signed"));
    }

    /// Draws the window once, as a frame would; what it asked for.
    fn draw(u: &mut Updates) -> Asked {
        let ctx = egui::Context::default();
        let mut asked = Asked::Nothing;
        let mut out = ctx.run_ui(egui::RawInput::default(), |ui| asked = u.window(ui.ctx()));
        out.textures_delta.clear();
        asked
    }

    #[test]
    fn every_stage_of_an_update_draws() {
        let mut u = updates();
        u.open = true;
        for phase in [
            Phase::Checking,
            Phase::Found(Found::UpToDate),
            Phase::Found(found("0.5.0")),
            Phase::Found(Found::Release {
                release: match found("0.5.0") {
                    Found::Release { release, .. } => release,
                    Found::UpToDate => unreachable!(),
                },
                ready: Err(Install::flatpak_steps(&Version::new(0, 5, 0))),
            }),
            Phase::Found(Found::Release {
                release: Release {
                    notes: "New: automatic updates".into(),
                    ..match found("0.5.0") {
                        Found::Release { release, .. } => release,
                        Found::UpToDate => unreachable!(),
                    }
                },
                ready: Err("download it by hand".into()),
            }),
            Phase::Working(Step::Downloading { done: 1, total: 2 }),
            Phase::Working(Step::Installing),
            Phase::Installed("/x/cut-craft".into()),
            Phase::Failed("no network".into()),
        ] {
            u.phase = phase.clone();
            assert_eq!(draw(&mut u), Asked::Nothing, "{phase:?}");
            assert!(u.open, "{phase:?} stays open until a button is pressed");
        }
    }

    #[test]
    fn the_offer_asks_to_download_install_and_relaunch() {
        assert_eq!(
            offer_heading(&Version::new(0, 6, 14)),
            "A new version (0.6.14) is available"
        );
    }

    #[test]
    fn install_now_relaunches_once_the_new_version_is_installed() {
        let mut u = updates();
        let (tx, rx) = channel();
        u.rx = Some(rx);
        u.relaunch = true;
        u.phase = Phase::Working(Step::Installing);
        tx.send(Msg::Done(Ok("/a/CutCraft-0.6.14-x86_64.AppImage".into())))
            .unwrap();
        assert_eq!(
            draw(&mut u),
            Asked::Restart("/a/CutCraft-0.6.14-x86_64.AppImage".into()),
            "no second click: closing (and the save question) follows straight on"
        );
        // if closing is called off, the window offers Relaunch Now and Later
        assert!(u.open);
        assert!(matches!(u.phase, Phase::Installed(_)));
        assert_eq!(draw(&mut u), Asked::Nothing, "asked once only");
    }

    #[test]
    fn a_failed_install_does_not_relaunch() {
        let mut u = updates();
        let (tx, rx) = channel();
        u.rx = Some(rx);
        u.relaunch = true;
        tx.send(Msg::Done(Err("the signature does not match".into())))
            .unwrap();
        assert_eq!(draw(&mut u), Asked::Nothing);
        assert_eq!(
            u.phase,
            Phase::Failed("the signature does not match".into())
        );
        assert!(!u.relaunch);
    }

    #[test]
    fn how_to_update_by_hand_lists_its_commands_to_copy() {
        let flatpak = Install::flatpak_steps(&Version::new(0, 6, 14));
        assert_eq!(
            commands_in(&flatpak),
            ["flatpak install --user cut-craft-0.6.14-x86_64.flatpak"]
        );
        let apt = "Update it there: for example `sudo apt update && sudo apt upgrade`, \
                   `sudo dnf upgrade`, or `paru -Syu`.";
        assert_eq!(
            commands_in(apt),
            [
                "sudo apt update && sudo apt upgrade",
                "sudo dnf upgrade",
                "paru -Syu"
            ]
        );
        assert_eq!(
            commands_in("Download it from the release page."),
            Vec::<&str>::new()
        );
    }

    #[test]
    fn a_closed_window_still_reports_a_finished_check() {
        let mut u = updates();
        let (tx, rx) = channel();
        u.rx = Some(rx);
        u.phase = Phase::Checking;
        tx.send(Msg::Checked(Ok(Found::UpToDate))).unwrap();
        assert_eq!(draw(&mut u), Asked::Checked);
        assert!(!u.open);
    }
}
