//! The Announcements window: at start, the announcements meant for today not shown yet today
//! (in place of that start's Did You Know? tip), and from Help ▸ Announcements every current
//! one. What they are and how they are read is in [`crate::announcements`].

use super::{CutCraftApp, egui};
use crate::announcements::{self as notes, Note};
use chrono::NaiveDate;
use cut_craft_i18n::tr;
use std::sync::mpsc::{Receiver, channel};
use std::time::{Duration, Instant};

/// How long the startup tip waits for GitHub's announcements before it shows anyway.
const TIP_WAITS: Duration = Duration::from_secs(4);

/// Where the program's repository is, for Support us on GitHub.
const REPOSITORY: &str = "https://github.com/Power-Whittle/PowerWhittle";

/// The Announcements window and the fetch from GitHub.
pub(super) struct Announcements {
    /// The notes the window shows, while it is open.
    pub open: Option<Vec<Note>>,
    /// The window was opened from the Help menu (it shows every current note, and what the
    /// fetch is doing).
    from_menu: bool,
    /// The notes read from GitHub, once it answered.
    fetched: Option<Vec<Note>>,
    /// The fetch going on.
    rx: Option<Receiver<Result<String, String>>>,
    /// The last fetch could not reach GitHub.
    failed: bool,
    /// The startup tip, held back while GitHub is asked, and until when.
    waiting_tip: Option<(usize, Instant)>,
    /// The fetch going on is the one at start: what it finds due opens.
    at_startup: bool,
    /// What asks GitHub (a stand-in in tests).
    pub fetch: fn() -> Result<String, String>,
    /// Today, for tests; the computer's own day when not set.
    pub today: Option<NaiveDate>,
    cache: egui_commonmark::CommonMarkCache,
}

impl Default for Announcements {
    fn default() -> Self {
        Self {
            open: None,
            from_menu: false,
            fetched: None,
            rx: None,
            failed: false,
            waiting_tip: None,
            at_startup: false,
            fetch: cut_craft_updater::announcements,
            today: None,
            cache: egui_commonmark::CommonMarkCache::default(),
        }
    }
}

impl Announcements {
    fn today(&self) -> NaiveDate {
        self.today.unwrap_or_else(notes::today)
    }

    /// Whether GitHub is being asked.
    pub fn asking(&self) -> bool {
        self.rx.is_some()
    }

    /// Every announcement meant for today: the bundled ones, then GitHub's.
    fn current(&self) -> Vec<Note> {
        let mut all = notes::bundled();
        all.extend(self.fetched.iter().flatten().cloned());
        notes::current(&all, self.today(), &cut_craft_i18n::language())
    }

    /// Asks GitHub in the background, unless already asking.
    fn start_fetch(&mut self, ctx: &egui::Context) {
        if self.rx.is_some() {
            return;
        }
        let (tx, rx) = channel();
        self.rx = Some(rx);
        let (fetch, ctx) = (self.fetch, ctx.clone());
        std::thread::spawn(move || {
            let _ = tx.send(fetch());
            ctx.request_repaint();
        });
    }

    /// Takes in GitHub's answer: whether one came.
    fn poll(&mut self) -> bool {
        let Some(answer) = self.rx.as_ref().and_then(|rx| rx.try_recv().ok()) else {
            return false;
        };
        self.rx = None;
        match answer {
            Ok(text) => {
                let (found, problems) = notes::parse(&text);
                for p in problems {
                    log::warn!("announcements.md: {p}");
                }
                self.fetched = Some(found);
                self.failed = false;
            }
            Err(e) => {
                log::info!("no announcements from GitHub: {e}");
                self.failed = true;
            }
        }
        true
    }
}

impl CutCraftApp {
    /// At start: the announcements due today, when any are and they are not turned off, in
    /// place of the tip. Whether the window opened.
    pub(super) fn announcements_at_startup(&mut self) -> bool {
        if !self.options.updates.show_announcements
            || self.first_run.is_some()
            || self.layout_question.is_some()
        {
            return false;
        }
        self.open_due_announcements()
    }

    /// Opens the window on the announcements due now, marking them shown today. Whether any
    /// were.
    fn open_due_announcements(&mut self) -> bool {
        let a = &self.announcements;
        let (today, current) = (a.today(), a.current());
        let seen = &self.options.announcements;
        let due: Vec<Note> = current
            .iter()
            .filter(|n| seen.is_due(n, today))
            .cloned()
            .collect();
        if due.is_empty() {
            return false;
        }
        self.options.announcements.shown_on(&due, today, &current);
        self.save_options();
        // added to those already open at start (the bundled ones, before GitHub answered)
        match &mut self.announcements.open {
            Some(open) if !self.announcements.from_menu => {
                let new: Vec<Note> = due
                    .into_iter()
                    .filter(|d| !open.iter().any(|o| o.id == d.id))
                    .collect();
                open.extend(new);
            }
            _ => {
                self.announcements.open = Some(due);
                self.announcements.from_menu = false;
            }
        }
        true
    }

    /// As PowerWhittle starts, unless turned off: asks GitHub for its announcements, holding the
    /// startup tip back a few seconds in case one is due.
    pub(super) fn fetch_announcements_at_startup(&mut self, ctx: &egui::Context) {
        if !self.options.updates.show_announcements
            || self.first_run.is_some()
            || self.layout_question.is_some()
        {
            return;
        }
        if self.announcements.open.is_none()
            && let Some(tip) = self.tip.take()
        {
            self.announcements.waiting_tip = Some((tip, Instant::now() + TIP_WAITS));
        }
        self.announcements.at_startup = true;
        self.announcements.start_fetch(ctx);
    }

    /// Help ▸ Announcements: every current announcement, asking GitHub again when it has not
    /// answered yet (not while announcements are turned off: then only the bundled ones).
    pub(super) fn open_announcements(&mut self, ctx: &egui::Context) {
        if self.options.updates.show_announcements && self.announcements.fetched.is_none() {
            self.announcements.start_fetch(ctx);
        }
        self.announcements.open = Some(self.announcements.current());
        self.announcements.from_menu = true;
    }

    /// GitHub's answer, the held-back tip and the window, each frame.
    pub(super) fn announcements_window(&mut self, ctx: &egui::Context) {
        if self.announcements.poll() {
            let at_startup = std::mem::take(&mut self.announcements.at_startup);
            if self.announcements.from_menu && self.announcements.open.is_some() {
                self.announcements.open = Some(self.announcements.current());
            } else if at_startup && self.tip.is_none() && self.announcements_at_startup() {
                // shown instead of the tip
                self.announcements.waiting_tip = None;
            }
        }
        if let Some((tip, until)) = self.announcements.waiting_tip {
            let now = Instant::now();
            if self.announcements.rx.is_none() || now >= until {
                self.announcements.waiting_tip = None;
                // the tip is this start's window: GitHub answering later opens nothing
                self.announcements.at_startup = false;
                if self.announcements.open.is_none() && self.tip.is_none() {
                    self.tip = Some(tip);
                }
            } else {
                ctx.request_repaint_after(until - now);
            }
        }
        self.draw_announcements(ctx);
    }

    fn draw_announcements(&mut self, ctx: &egui::Context) {
        let Some(shown) = self.announcements.open.clone() else {
            return;
        };
        let mut open = true;
        let mut close = false;
        let mut seen = self.options.announcements.clone();
        let mut show = self.options.updates.show_announcements;
        let looking = self.announcements.asking();
        let (from_menu, failed) = (self.announcements.from_menu, self.announcements.failed);
        let size = egui::vec2(28.0, 22.0) * ctx.global_style().spacing.interact_size.y;
        let cache = &mut self.announcements.cache;
        let id = egui::Id::unique("announcements");
        let title = tr!("announcements-title");
        cut_craft_bug_report::dialog(ctx, id, &title, size, &mut open, |ui| {
            egui::ScrollArea::vertical()
                .max_height(18.0 * ui.spacing().interact_size.y)
                .auto_shrink([false, true])
                .show(ui, |ui| {
                    if shown.is_empty() {
                        ui.label(tr!("announcements-none"));
                    }
                    for (n, note) in shown.iter().enumerate() {
                        if n > 0 {
                            ui.separator();
                        }
                        ui.heading(&note.title);
                        egui_commonmark::CommonMarkViewer::new().show(ui, cache, &note.body);
                        if note.support {
                            support_row(ui);
                        }
                        let mut off = seen.dismissed.contains(&note.id);
                        if ui
                            .checkbox(&mut off, tr!("announcements-dont-show-again"))
                            .changed()
                        {
                            seen.set_dismissed(&note.id, off);
                        }
                    }
                });
            if from_menu && looking {
                ui.weak(tr!("announcements-looking"));
            } else if from_menu && failed {
                ui.weak(tr!("announcements-offline"));
            }
            ui.separator();
            ui.horizontal(|ui| {
                ui.checkbox(&mut show, tr!("announcements-show-at-start"));
                if ui.button(tr!("announcements-close")).clicked() {
                    close = true;
                }
            });
        });
        let mut changed = false;
        if seen != self.options.announcements {
            self.options.announcements = seen;
            changed = true;
        }
        if show != self.options.updates.show_announcements {
            self.options.updates.show_announcements = show;
            changed = true;
        }
        if changed {
            self.save_options();
        }
        if !open || close {
            self.announcements.open = None;
        }
    }
}

/// Support us on GitHub: what helps, and a button to the repository.
fn support_row(ui: &mut egui::Ui) {
    ui.add_space(ui.spacing().item_spacing.y);
    ui.horizontal_wrapped(|ui| {
        ui.label(tr!("announcements-support-text"));
        if ui.button(tr!("announcements-support-button")).clicked() {
            ui.ctx().open_url(egui::OpenUrl::new_tab(REPOSITORY));
        }
    });
}
