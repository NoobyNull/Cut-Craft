//! Right-click ▸ Report a Bug Here…: a GitHub issue saying what was wrong, in the reporter's
//! words, with where it was (the named parts of the window under the pointer and the source
//! line that draws each), the app's state, a screenshot of the window and any pictures added.
//!
//! Code marks what it draws with [`mark`]; the app calls [`Reporter::frame`] once a frame,
//! after drawing. Issues are made through the GitHub CLI (`gh`, already signed in); a report
//! that cannot be sent is kept on disk instead.

mod github;

use eframe::egui;
use std::panic::Location;
use std::path::PathBuf;
use std::sync::{Arc, Mutex, mpsc};

pub use github::{Report, compose, issue_url, open_with_platform, png, submit};

/// The report form's share of the screen.
const FORM_SHARE: f32 = 0.5;
/// The screenshot's thumbnail, as a share of the form's width.
const THUMBNAIL_SHARE: f32 = 0.5;
/// Lines of the description box.
const TEXT_ROWS: usize = 6;

/// A named part of the window and the source line that drew it.
#[derive(Clone, Debug, PartialEq)]
pub struct Region {
    pub name: String,
    pub rect: egui::Rect,
    pub at: &'static Location<'static>,
}

/// This frame's regions, outermost first, and whether a right-click menu took the click.
#[derive(Clone, Default)]
struct Frame {
    number: u64,
    regions: Vec<Region>,
    claimed: bool,
    asked: bool,
}

fn frame_id() -> egui::Id {
    egui::Id::unique("bug_report_frame")
}

/// How tall a dialog opens, as a share of the main window's height.
pub const DIALOG_HEIGHT_SHARE: f32 = 0.8;

/// A dialog in a window of its own beside the main one (the Tool Database, Machine
/// Configuration, Report a Bug...), closed with the desktop's own button: `open` goes false
/// then. Its content scrolls when it is taller than the window. Where no second window can
/// open (the web, tests), an egui window inside the main one instead.
pub fn dialog<R>(
    ctx: &egui::Context,
    id: egui::Id,
    title: &str,
    size: egui::Vec2,
    open: &mut bool,
    add: impl FnOnce(&mut egui::Ui) -> R,
) -> Option<R> {
    dialog_in(ctx, id, title, size, open, true, add)
}

/// [`dialog`] for content that lays itself out with panels, which a scroll area would break.
pub fn dialog_with_panels<R>(
    ctx: &egui::Context,
    id: egui::Id,
    title: &str,
    size: egui::Vec2,
    open: &mut bool,
    add: impl FnOnce(&mut egui::Ui) -> R,
) -> Option<R> {
    dialog_in(ctx, id, title, size, open, false, add)
}

fn dialog_in<R>(
    ctx: &egui::Context,
    id: egui::Id,
    title: &str,
    size: egui::Vec2,
    open: &mut bool,
    scroll: bool,
    add: impl FnOnce(&mut egui::Ui) -> R,
) -> Option<R> {
    if !*open {
        return None;
    }
    if ctx.embed_viewports() {
        return egui::Window::new(title)
            .id(id)
            .open(open)
            .default_size(size)
            .collapsible(false)
            .show(ctx, add)
            .and_then(|shown| shown.inner);
    }
    let mut add = Some(add);
    ctx.show_viewport_immediate(
        egui::ViewportId::from_hash_of(id),
        egui::ViewportBuilder::default()
            .with_title(title)
            .with_inner_size(size),
        |ui, _| {
            let out = egui::CentralPanel::default()
                .show(ui, |ui| {
                    let add = add.take()?;
                    Some(if scroll {
                        egui::ScrollArea::vertical()
                            .auto_shrink(false)
                            .show(ui, add)
                            .inner
                    } else {
                        add(ui)
                    })
                })
                .inner;
            if ui.ctx().input(|i| i.viewport().close_requested()) {
                *open = false;
            }
            out
        },
    )
}

/// Where a window picture lands once egui has drawn it (a frame or more later).
pub type Shot = Arc<Mutex<Option<Arc<egui::ColorImage>>>>;

/// Asks egui for a picture of the window as it is drawn this frame; it lands in `shot`.
pub fn take_screenshot(ctx: &egui::Context, shot: &Shot) {
    let landing = shot.clone();
    ctx.send_viewport_cmd(egui::ViewportCommand::Screenshot(
        egui::ScreenshotCallback::new(move |image| {
            if let Ok(mut taken) = landing.lock() {
                *taken = Some(image);
            }
        }),
    ));
}

/// The picture `take_screenshot` asked for, once it has landed.
pub fn taken(shot: &Shot) -> Option<Arc<egui::ColorImage>> {
    shot.lock().ok()?.take()
}

fn with_frame<R>(ctx: &egui::Context, f: impl FnOnce(&mut Frame) -> R) -> R {
    let number = ctx.cumulative_pass_nr();
    ctx.data_mut(|d| {
        let frame = d.get_temp_mut_or_default::<Frame>(frame_id());
        if frame.number != number {
            *frame = Frame {
                number,
                ..Frame::default()
            };
        }
        f(frame)
    })
}

/// Names the space `ui` has to draw in, for bug reports: the caller's source line is kept.
#[track_caller]
pub fn mark(ui: &egui::Ui, name: &str) {
    mark_rect(ui.ctx(), ui.max_rect(), name);
}

/// Names `rect`, for bug reports, as [`mark`].
#[track_caller]
pub fn mark_rect(ctx: &egui::Context, rect: egui::Rect, name: &str) {
    let at = Location::caller();
    with_frame(ctx, |f| {
        f.regions.push(Region {
            name: name.to_owned(),
            rect,
            at,
        });
    });
}

/// Another right-click menu took this frame's right-click; it offers [`open_here`] itself.
pub fn claim(ctx: &egui::Context) {
    with_frame(ctx, |f| f.claimed = true);
}

/// Opens the report form for the last right-click (from another right-click menu's item).
pub fn open_here(ctx: &egui::Context) {
    with_frame(ctx, |f| f.asked = true);
}

/// The regions holding `at`, outermost first (in the order they were drawn).
pub fn regions_at(regions: &[Region], at: egui::Pos2) -> Vec<Region> {
    regions
        .iter()
        .filter(|r| r.rect.contains(at))
        .cloned()
        .collect()
}

/// A report being written.
struct Draft {
    title: String,
    text: String,
    at: egui::Pos2,
    regions: Vec<Region>,
    state: String,
    screenshot: Option<Arc<egui::ColorImage>>,
    send_screenshot: bool,
    pictures: Vec<PathBuf>,
    error: Option<String>,
}

impl Draft {
    /// What is sent.
    fn report(&self) -> Report {
        Report {
            title: self.title.trim().to_owned(),
            text: self.text.clone(),
            at: (self.at.x, self.at.y),
            regions: self
                .regions
                .iter()
                .map(|r| (r.name.clone(), format!("{}:{}", r.at.file(), r.at.line())))
                .collect(),
            state: self.state.clone(),
            screenshot: self
                .send_screenshot
                .then(|| self.screenshot.as_deref().and_then(|i| github::png(i).ok()))
                .flatten(),
            pictures: self.pictures.clone(),
        }
    }
}

/// Where reports go and the right-click being reported on.
pub struct Reporter {
    /// The GitHub repository issues are made in (`owner/name`).
    pub repo: String,
    /// Where a report that cannot be sent is kept.
    pub keep_dir: Option<PathBuf>,
    /// Without the GitHub CLI, the browser opens a new-issue page with the report filled in
    /// (off in tests).
    pub browser: bool,
    /// The last right-click: where, and the regions there then.
    clicked: Option<(egui::Pos2, Vec<Region>)>,
    /// Shows the small Report a Bug menu at the right-click.
    menu: bool,
    /// The window as it was at the last right-click, and where egui delivers it.
    screenshot: Option<Arc<egui::ColorImage>>,
    shot: Shot,
    draft: Option<Draft>,
    sending: Option<mpsc::Receiver<Result<String, String>>>,
}

/// What the app gives a report: its state, and a picker for pictures to add.
pub struct Host<'a> {
    pub state: &'a dyn Fn() -> String,
    pub pick_pictures: &'a dyn Fn() -> Vec<PathBuf>,
}

impl Reporter {
    pub fn new(repo: &str, keep_dir: Option<PathBuf>) -> Self {
        Self {
            repo: repo.to_owned(),
            keep_dir,
            browser: true,
            clicked: None,
            menu: false,
            screenshot: None,
            shot: Shot::default(),
            draft: None,
            sending: None,
        }
    }

    /// Whether the report form is open.
    pub fn is_open(&self) -> bool {
        self.draft.is_some()
    }

    /// Once a frame, after everything is drawn: notices right-clicks, shows the menu and the
    /// form, and sends. A message for the status bar when a report was sent or failed.
    pub fn frame(&mut self, ctx: &egui::Context, host: &Host) -> Option<String> {
        let (regions, claimed, asked) =
            with_frame(ctx, |f| (f.regions.clone(), f.claimed, f.asked));
        let clicked = ctx.input(|i| {
            i.pointer
                .secondary_clicked()
                .then(|| i.pointer.interact_pos())
                .flatten()
        });
        if let Some(at) = clicked {
            self.clicked = Some((at, regions_at(&regions, at)));
            self.menu = !claimed;
            // the window as it is now, before any menu is drawn over it
            take_screenshot(ctx, &self.shot);
        }
        if let Some(image) = taken(&self.shot) {
            self.screenshot = Some(image);
        }
        if self.menu && clicked.is_none() && self.small_menu(ctx) {
            self.open(host);
        }
        if asked {
            self.open(host);
        }
        self.form(ctx, host);
        self.sent(ctx)
    }

    fn open(&mut self, host: &Host) {
        let (at, regions) = self.clicked.clone().unwrap_or_default();
        self.menu = false;
        self.draft = Some(Draft {
            title: String::new(),
            text: String::new(),
            at,
            regions,
            state: (host.state)(),
            screenshot: self.screenshot.clone(),
            send_screenshot: self.screenshot.is_some(),
            pictures: vec![],
            error: None,
        });
    }

    /// The one-item menu where nothing else has a right-click menu; whether it was chosen.
    fn small_menu(&mut self, ctx: &egui::Context) -> bool {
        let Some((at, _)) = &self.clicked else {
            return false;
        };
        let mut chosen = false;
        let area = egui::Area::new(egui::Id::unique("bug_report_menu"))
            .order(egui::Order::Foreground)
            .fixed_pos(*at)
            // kept inside the window, as a right-click near an edge would put it outside
            .constrain(true)
            .show(ctx, |ui| {
                egui::Frame::menu(ui.style()).show(ui, |ui| {
                    chosen = ui.button("Report a Bug Here…").clicked();
                });
            });
        let away = ctx.input(|i| {
            let outside = i.pointer.any_pressed()
                && i.pointer
                    .interact_pos()
                    .is_some_and(|p| !area.response.rect.contains(p));
            outside || i.key_pressed(egui::Key::Escape)
        });
        if chosen || away {
            self.menu = false;
        }
        chosen
    }

    fn form(&mut self, ctx: &egui::Context, host: &Host) {
        let Some(draft) = &mut self.draft else {
            return;
        };
        let sending = self.sending.is_some();
        let (mut open, mut submit_now, mut cancel) = (true, false, false);
        let size = ctx.content_rect().size() * egui::vec2(FORM_SHARE, DIALOG_HEIGHT_SHARE);
        let id = egui::Id::unique("report_a_bug");
        dialog(ctx, id, "Report a Bug", size, &mut open, |ui| {
            ui.label("Title");
            ui.add(egui::TextEdit::singleline(&mut draft.title).desired_width(f32::INFINITY));
            ui.label("What went wrong, and what you expected");
            ui.add(
                egui::TextEdit::multiline(&mut draft.text)
                    .desired_rows(TEXT_ROWS)
                    .desired_width(f32::INFINITY),
            );
            ui.collapsing("Where (sent with the report)", |ui| {
                if draft.regions.is_empty() {
                    ui.weak("No named part of the window here");
                }
                for r in &draft.regions {
                    ui.label(format!("{} — {}:{}", r.name, r.at.file(), r.at.line()));
                }
                ui.weak(&draft.state);
            });
            if let Some(image) = &draft.screenshot {
                ui.checkbox(
                    &mut draft.send_screenshot,
                    "Send a screenshot of the window",
                );
                if draft.send_screenshot {
                    let texture = ui.ctx().load_texture(
                        "bug_report_screenshot",
                        Arc::clone(image),
                        egui::TextureOptions::LINEAR,
                    );
                    let shown = ui.available_width() * THUMBNAIL_SHARE;
                    let size = texture.size_vec2() * (shown / texture.size_vec2().x);
                    ui.image((texture.id(), size));
                }
            }
            ui.horizontal(|ui| {
                if ui.button("Add Pictures…").clicked() {
                    draft.pictures.extend((host.pick_pictures)());
                }
                let mut remove = None;
                for (i, p) in draft.pictures.iter().enumerate() {
                    let name = p
                        .file_name()
                        .map_or_else(String::new, |n| n.to_string_lossy().into_owned());
                    if ui
                        .small_button(format!("{name} ✕"))
                        .on_hover_text("Leave it out")
                        .clicked()
                    {
                        remove = Some(i);
                    }
                }
                if let Some(i) = remove {
                    draft.pictures.remove(i);
                }
            });
            if let Some(e) = &draft.error {
                ui.colored_label(ui.visuals().error_fg_color, e);
            }
            ui.separator();
            ui.horizontal(|ui| {
                let ready = !draft.title.trim().is_empty() && !sending;
                submit_now = ui
                    .add_enabled(ready, egui::Button::new("Submit to GitHub"))
                    .on_disabled_hover_text("Give it a title")
                    .clicked();
                cancel = ui.button("Cancel").clicked();
                if sending {
                    ui.spinner();
                    ui.label("Sending…");
                }
            });
        });
        if !open || cancel {
            self.draft = None;
            return;
        }
        if submit_now {
            let report = draft.report();
            self.send(ctx, report);
        }
    }

    /// Sends `report` on a thread of its own; a report that cannot go is kept on disk, and
    /// without the GitHub CLI the browser gets it as a new issue to finish by hand.
    fn send(&mut self, ctx: &egui::Context, report: Report) {
        let (repo, keep, browser) = (self.repo.clone(), self.keep_dir.clone(), self.browser);
        let (tx, rx) = mpsc::channel();
        let ctx = ctx.clone();
        std::thread::spawn(move || {
            let result = submit(&report, &repo, &github::run_gh).map_err(|e| {
                let kept = match keep.map(|dir| github::keep(&report, &dir)) {
                    Some(Ok(path)) => format!("{e}; the report is kept in {}", path.display()),
                    _ => e,
                };
                if browser && !github::gh_installed() {
                    let url = github::issue_url(&repo, &report);
                    if github::open_with_platform(url.as_ref()).is_ok() {
                        return format!(
                            "{kept}. The GitHub CLI (gh) is not installed, so your browser has \
                             the report as a new issue: add the pictures there and submit it"
                        );
                    }
                }
                kept
            });
            let _ = tx.send(result);
            ctx.request_repaint();
        });
        self.sending = Some(rx);
    }

    /// A finished send: the issue's address, or why it failed (the form stays open then).
    fn sent(&mut self, ctx: &egui::Context) -> Option<String> {
        let result = self.sending.as_ref()?.try_recv().ok()?;
        self.sending = None;
        match result {
            Ok(url) => {
                self.draft = None;
                ctx.copy_text(url.clone());
                Some(format!("Bug reported: {url} (address copied)"))
            }
            Err(e) => {
                if let Some(d) = &mut self.draft {
                    d.error = Some(e.clone());
                }
                Some(format!("Bug report not sent: {e}"))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(ctx: &egui::Context, events: Vec<egui::Event>, draw: impl FnMut(&egui::Context)) {
        let input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(800.0, 600.0),
            )),
            events,
            ..Default::default()
        };
        // no renderer takes a headless frame's texture changes
        ctx.run_pass(input, draw).textures_delta.clear();
    }

    #[test]
    fn a_closed_dialog_shows_nothing_in_a_window_of_its_own() {
        let ctx = egui::Context::default();
        ctx.set_embed_viewports(false);
        let mut drawn = false;
        let mut open = false;
        run(&ctx, vec![], |ctx| {
            let id = egui::Id::unique("closed");
            dialog(
                ctx,
                id,
                "Closed",
                egui::vec2(200.0, 100.0),
                &mut open,
                |_| {
                    drawn = true;
                },
            );
        });
        assert!(!drawn, "a closed dialog was drawn");
    }

    fn right_click(at: egui::Pos2) -> Vec<egui::Event> {
        [true, false]
            .into_iter()
            .map(|pressed| egui::Event::PointerButton {
                pos: at,
                button: egui::PointerButton::Secondary,
                pressed,
                modifiers: egui::Modifiers::NONE,
            })
            .collect()
    }

    #[test]
    fn regions_under_the_pointer_come_outermost_first_with_their_source_lines() {
        let ctx = egui::Context::default();
        let mut found = vec![];
        run(&ctx, vec![], |ctx| {
            ctx.root_ui(|ui| {
                egui::CentralPanel::default().show(ui, |ui| {
                    mark(ui, "Whole");
                    mark_rect(
                        ctx,
                        egui::Rect::from_min_size(egui::pos2(10.0, 10.0), egui::vec2(50.0, 50.0)),
                        "Box",
                    );
                    mark_rect(
                        ctx,
                        egui::Rect::from_min_size(egui::pos2(500.0, 10.0), egui::vec2(50.0, 50.0)),
                        "Elsewhere",
                    );
                });
            });
            found = with_frame(ctx, |f| regions_at(&f.regions, egui::pos2(20.0, 20.0)));
        });
        let names: Vec<&str> = found.iter().map(|r| r.name.as_str()).collect();
        assert_eq!(names, ["Whole", "Box"]);
        assert!(
            found[1].at.file().ends_with("lib.rs"),
            "{}",
            found[1].at.file()
        );
    }

    #[test]
    fn a_right_click_offers_the_report_unless_another_menu_took_it() {
        let ctx = egui::Context::default();
        let mut reporter = Reporter::new("me/app", None);
        let host = Host {
            state: &|| "state".into(),
            pick_pictures: &Vec::new,
        };
        let at = egui::pos2(100.0, 100.0);
        let frame = |events, claim_it: bool, reporter: &mut Reporter| {
            run(&ctx, events, |ctx| {
                ctx.root_ui(|ui| egui::CentralPanel::default().show(ui, |ui| mark(ui, "Canvas")));
                if claim_it {
                    claim(ctx);
                }
                reporter.frame(ctx, &host);
            });
        };
        frame(right_click(at), true, &mut reporter);
        frame(vec![], false, &mut reporter);
        assert!(!reporter.menu, "the other menu offers it");
        frame(right_click(at), false, &mut reporter);
        assert!(reporter.menu);
        frame(vec![], false, &mut reporter);
        reporter.open(&host);
        let draft = reporter.draft.as_ref().unwrap();
        assert_eq!(draft.regions[0].name, "Canvas");
        assert_eq!(draft.state, "state");
        assert!(reporter.is_open());
    }
}
