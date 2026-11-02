//! Help ▸ Check for Updates and the check at startup: the window that offers a new version,
//! shows the download, and asks to restart. The work is [`cut_craft_updater`]'s, on a thread
//! of its own so the window keeps drawing.

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
    /// Close Cut Craft (asking to save as usual) and start this program.
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
            install,
            current,
            rx: None,
        }
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

    /// Update Now: downloads and installs what the check found.
    fn start_install(&mut self, ctx: &egui::Context) {
        let Phase::Found(found) = &self.phase else {
            return;
        };
        let found = found.clone();
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

    /// Takes in what the background thread has said; whether a check finished.
    fn poll(&mut self) -> bool {
        let Some(rx) = &self.rx else { return false };
        let msgs: Vec<Msg> = rx.try_iter().collect();
        let mut checked = false;
        for msg in msgs {
            match msg {
                Msg::Checked(answer) => {
                    checked = answer.is_ok();
                    self.answered(answer);
                }
                Msg::Step(step) => self.phase = Phase::Working(step),
                Msg::Done(done) => {
                    self.phase = match done {
                        Ok(program) => Phase::Installed(program),
                        Err(e) => {
                            log::warn!("the update failed: {e}");
                            Phase::Failed(e)
                        }
                    };
                    self.open = true;
                }
            }
        }
        checked
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
        let checked = self.poll();
        if !self.open {
            return if checked {
                Asked::Checked
            } else {
                Asked::Nothing
            };
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
        let mut asked = if checked {
            Asked::Checked
        } else {
            Asked::Nothing
        };
        let close = match pressed {
            Press::Nothing => false,
            Press::Close => true,
            Press::UpdateNow => {
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
                    ui.label("Asking GitHub for the newest version...");
                });
                Press::Nothing
            }
            Phase::Found(Found::UpToDate) => {
                ui.heading("Cut Craft is up to date");
                ui.label(format!("Version {} is the newest version.", self.current));
                ui.add_space(8.0);
                pressed(ui.button("Close").clicked(), Press::Close)
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
                ui.weak(
                    "You can close this window and keep working; it opens again when the \
                     update is ready.",
                );
                Press::Nothing
            }
            Phase::Installed(program) => {
                let version = self
                    .offered
                    .as_ref()
                    .map_or_else(String::new, |v| format!(" {v}"));
                ui.heading(format!("Cut Craft{version} is installed"));
                ui.label(
                    "Restart Cut Craft to start using it. If your job has changes that are not \
                     saved, you will be asked whether to save them first.",
                );
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    let now = ui
                        .button(egui::RichText::new("Restart Now").strong())
                        .clicked();
                    let later = ui
                        .button("Restart Later")
                        .on_hover_text("The new version starts the next time you open Cut Craft")
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
                ui.heading("The update did not finish");
                ui.label(e);
                ui.label("This copy of Cut Craft has not been changed.");
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    let again = ui.button("Try Again").clicked();
                    if ui.button("Open the Download Page").clicked() {
                        ui.ctx().open_url(egui::OpenUrl::new_tab(RELEASES_PAGE));
                    }
                    let close = ui.button("Close").clicked();
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
    UpdateNow,
    TryAgain,
    Skip(String),
    Restart(PathBuf),
}

fn pressed(clicked: bool, press: Press) -> Press {
    if clicked { press } else { Press::Nothing }
}

/// A newer release: what changed, and Update Now (or the download page, when it cannot
/// install itself here), Later and Skip This Version.
fn offer(
    ui: &mut egui::Ui,
    release: &cut_craft_updater::Release,
    ready: &Result<(), String>,
    current: &Version,
) -> Press {
    ui.heading(format!("Cut Craft {} is available", release.version));
    ui.label(format!("You have version {current}."));
    ui.add_space(4.0);
    if !release.notes.is_empty() {
        egui::CollapsingHeader::new("What's new")
            .default_open(true)
            .show(ui, |ui| {
                egui::ScrollArea::vertical()
                    .max_height(140.0)
                    .show(ui, |ui| ui.label(&release.notes));
            });
    }
    if !release.page.is_empty() {
        ui.hyperlink_to("See this version on GitHub", &release.page);
    }
    ui.add_space(8.0);
    ui.label(match ready {
        Ok(()) => {
            "Update Now downloads it from GitHub, checks the download is complete and \
             unchanged, and installs it. Cut Craft asks before restarting, and your open job \
             is not touched."
        }
        Err(why) => why.as_str(),
    });
    ui.add_space(8.0);
    let press = ui
        .horizontal(|ui| {
            let mut press = Press::Nothing;
            if ready.is_ok() {
                if ui
                    .button(egui::RichText::new("Update Now").strong())
                    .clicked()
                {
                    press = Press::UpdateNow;
                }
            } else if ui.button("Open the Download Page").clicked() {
                let page = if release.page.is_empty() {
                    RELEASES_PAGE
                } else {
                    &release.page
                };
                ui.ctx().open_url(egui::OpenUrl::new_tab(page));
            }
            if ui.button("Later").clicked() {
                press = Press::Close;
            }
            if ui
                .button("Skip This Version")
                .on_hover_text(
                    "Not offered again when Cut Craft starts; Help ▸ Check for Updates still \
                     finds it",
                )
                .clicked()
            {
                press = Press::Skip(release.version.to_string());
            }
            press
        })
        .inner;
    ui.add_space(4.0);
    ui.weak("Automatic checks can be turned off in Edit ▸ Options ▸ Updates.");
    press
}

/// What an update step says, and how far along it is when that is known.
fn progress(step: Step) -> (String, Option<f32>) {
    #[expect(clippy::cast_precision_loss, reason = "megabytes to one decimal place")]
    let mb = |bytes: u64| bytes as f64 / 1_000_000.0;
    match step {
        Step::Downloading { done, total } if total > 0 => (
            format!("Downloading... {:.1} of {:.1} MB", mb(done), mb(total)),
            #[expect(clippy::cast_possible_truncation, reason = "a fraction")]
            Some((mb(done) / mb(total)).min(1.0) as f32),
        ),
        Step::Downloading { .. } => ("Downloading...".to_owned(), None),
        Step::Checking => (
            "Checking the download is complete and unchanged...".to_owned(),
            None,
        ),
        Step::Installing => ("Installing...".to_owned(), None),
    }
}

/// Starts the new version; called once the user has said Restart Now and closing went ahead.
pub fn restart(program: &std::path::Path) -> Result<(), String> {
    cut_craft_updater::restart(program).map_err(|e| {
        format!(
            "Cut Craft could not start the new version ({e}). Open it again yourself: it is \
             installed."
        )
    })
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
        assert!(progress(Step::Checking).0.contains("unchanged"));
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
