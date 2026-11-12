//! Right-click ▸ Report a Bug Here…: a GitHub issue saying what was wrong, in the reporter's
//! words, with where it was (the named parts of the window under the pointer and the source
//! line that draws each, and the exact widget clicked), the app's state, the log, a screenshot
//! of the window and any pictures added. After a crash the next start offers its crash log.
//!
//! Reports are public, so nothing personal goes without asking: the screenshot and the log
//! are each sent only when ticked, the form shows exactly what each says, and the log and
//! state have the person's home folder and user name taken out ([`privacy::scrub`]).
//!
//! Code marks what it draws with [`mark`]; the app calls [`Reporter::frame`] once a frame,
//! after drawing. Issues are made through the GitHub CLI (`gh`, already signed in); a report
//! that cannot be sent is kept on disk instead.

mod github;
pub mod hint;
pub mod logs;
pub mod privacy;
pub mod relay;
mod spot;

use eframe::egui;
use std::panic::Location;
use std::path::PathBuf;
use std::sync::{Arc, Mutex, mpsc};

pub use github::{Report, compose, issue_url, open_with_platform, png, submit};
pub use spot::widget_at;

/// The report form's share of the screen.
const FORM_SHARE: f32 = 0.5;
/// The screenshot's thumbnail, as a share of the form's width.
const THUMBNAIL_SHARE: f32 = 0.5;
/// Lines of the description box.
const TEXT_ROWS: usize = 6;
/// Lines of the log shown in the form before it scrolls.
const LOG_ROWS: f32 = 8.0;
/// How faint the screenshot shows while it is not to be sent.
const UNSENT_ALPHA: u8 = 90;
/// What the form says about who sees a report.
pub const PUBLIC_NOTE: &str = "Reports are public on GitHub, so anyone can read them. Tick only \
    what you are happy to share.";

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

/// `response`'s right-click menu, with Report a Bug Here… at its foot: the right-click is
/// [`claim`]ed, so the small menu doesn't open over this one as well.
pub fn context_menu(response: &egui::Response, add_contents: impl FnOnce(&mut egui::Ui)) {
    if response.secondary_clicked() {
        claim(&response.ctx);
    }
    response.context_menu(|ui| {
        add_contents(ui);
        ui.separator();
        if ui.button("Report a Bug Here…").clicked() {
            open_here(ui.ctx());
            ui.close();
        }
    });
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
    /// The widget clicked, from the accessibility tree.
    widget: Option<String>,
    state: String,
    /// The log, or the crash log, as it would be sent: scrubbed already.
    log: String,
    send_log: bool,
    screenshot: Option<Arc<egui::ColorImage>>,
    send_screenshot: bool,
    pictures: Vec<PathBuf>,
    error: Option<String>,
    /// A report of the last run's crash, its log the crash log.
    crash: bool,
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
            widget: self.widget.clone(),
            state: self.state.clone(),
            log: if self.send_log {
                self.log.clone()
            } else {
                String::new()
            },
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
    /// The widget under the last right-click.
    spotted: Option<String>,
    /// Whether the plugin that finds it is in.
    spotting: bool,
    /// A crash log the last run left, to offer as a report, and whether its log starts
    /// ticked (the person said yes to sending crash logs).
    crash: Option<(String, bool)>,
    /// Cut Craft's crash-report service (its `/v1/crash` address), once there is one: a
    /// crash's log goes there by itself for people who said yes.
    pub crash_service: Option<String>,
    /// How a crash's log is posted to the service (replaced in tests).
    pub post: fn(&str, &str) -> Result<(), relay::Failure>,
    /// How hard the proof of work sent with a crash's log is ([`relay::WORK_BITS`]; less in
    /// tests, which have no service to satisfy).
    pub work_bits: u32,
    /// Where a crash log that could not be sent for now is put back, to be sent at the next
    /// start (the folder `crash.log` is kept in).
    pub retry_dir: Option<PathBuf>,
    /// A crash log on its way to the service, and how that went.
    crash_sending: Option<(String, mpsc::Receiver<Result<(), relay::Failure>>)>,
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
            spotted: None,
            spotting: false,
            crash: None,
            crash_service: None,
            post: relay::post_https,
            work_bits: relay::WORK_BITS,
            retry_dir: None,
            crash_sending: None,
        }
    }

    /// Sends `crash` (the last run's crash log) to the crash-report service on a thread of
    /// its own, as install `install`, for someone who said yes to sending crash logs. With
    /// no service, or should it fail, the report form opens with the log ticked instead.
    pub fn send_crash(&mut self, ctx: &egui::Context, crash: String, install: &str) {
        let Some(url) = self.crash_service.clone() else {
            self.report_crash(crash, true);
            return;
        };
        let (tx, rx) = mpsc::channel();
        let (post, bits, text, install, ctx) = (
            self.post,
            self.work_bits,
            crash.clone(),
            install.to_owned(),
            ctx.clone(),
        );
        // the proof of work takes a moment: never on the frame
        std::thread::spawn(move || {
            let _ = tx.send(relay::send(&url, &text, &install, bits, &post));
            ctx.request_repaint();
        });
        self.crash_sending = Some((crash, rx));
    }

    /// A crash log sent to the service: thanks. One that could not go for now is put back
    /// to try at the next start; one the service refused opens the form.
    fn crash_sent(&mut self) -> Option<String> {
        let (_, rx) = self.crash_sending.as_ref()?;
        let result = rx.try_recv().ok()?;
        let (crash, _) = self.crash_sending.take()?;
        match result {
            Ok(()) => Some(
                "Cut Craft closed with an error last time. Its crash report was sent: thank \
                 you, it helps fix Cut Craft."
                    .into(),
            ),
            Err(failed) => {
                log::warn!("{}", failed.why);
                if failed.retry
                    && let Some(dir) = &self.retry_dir
                    && logs::put_back_crash(dir, &crash).is_ok()
                {
                    return Some(
                        "Cut Craft closed with an error last time. Its crash report could not \
                         be sent just now; Cut Craft will try again the next time it starts."
                            .into(),
                    );
                }
                self.crash = Some((crash, true));
                None
            }
        }
    }

    /// Offers `crash` (what the last run's panic left) as a report, from the next frame; its
    /// log ticked to send when `send_log` (the person agreed to send crash logs), else
    /// left for them to tick.
    pub fn report_crash(&mut self, crash: String, send_log: bool) {
        self.crash = Some((crash, send_log));
    }

    /// Whether the report form is open.
    pub fn is_open(&self) -> bool {
        self.draft.is_some()
    }

    /// Once a frame, after everything is drawn: notices right-clicks, shows the menu and the
    /// form, and sends. A message for the status bar when a report was sent or failed.
    pub fn frame(&mut self, ctx: &egui::Context, host: &Host) -> Option<String> {
        if !self.spotting {
            ctx.add_plugin(spot::Spotter);
            self.spotting = true;
        }
        if let Some(found) = ctx.data_mut(|d| d.remove_temp::<Option<String>>(spot::id())) {
            self.spotted = found;
        }
        let thanks = self.crash_sent();
        if let Some((crash, send_log)) = self.crash.take() {
            self.open_crash(&crash, send_log, host);
        }
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
            self.spotted = None;
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
        self.sent(ctx).or(thanks)
    }

    fn open(&mut self, host: &Host) {
        let (at, regions) = self.clicked.clone().unwrap_or_default();
        self.menu = false;
        // asked each time: neither goes unless ticked
        self.draft = Some(Draft {
            title: String::new(),
            text: String::new(),
            at,
            regions,
            widget: self.spotted.clone(),
            state: privacy::scrub(&(host.state)()),
            log: privacy::scrub(&logs::recent()),
            send_log: false,
            screenshot: self.screenshot.clone(),
            send_screenshot: false,
            pictures: vec![],
            error: None,
            crash: false,
        });
    }

    /// The form for a crash: titled in plain words, the crash log to send when allowed.
    fn open_crash(&mut self, crash: &str, send_log: bool, host: &Host) {
        self.draft = Some(Draft {
            title: privacy::crash_title(crash),
            text: "Cut Craft closed with an error the last time it ran.\n\nWhat were you doing?\n"
                .into(),
            at: egui::Pos2::ZERO,
            regions: vec![],
            widget: None,
            state: privacy::scrub(&(host.state)()),
            log: privacy::scrub(crash),
            send_log,
            screenshot: None,
            send_screenshot: false,
            pictures: vec![],
            error: None,
            crash: true,
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
            what_is_sent(ui, draft);
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

/// The form's question: may the screenshot and the log go too? Each starts unticked
/// (a crash's log ticked when crash logs were allowed), each shows exactly what it is, and
/// what always goes is shown as well.
fn what_is_sent(ui: &mut egui::Ui, draft: &mut Draft) {
    ui.separator();
    ui.strong("May Cut Craft send these with your report?");
    ui.weak(PUBLIC_NOTE);
    if let Some(image) = &draft.screenshot {
        ui.checkbox(
            &mut draft.send_screenshot,
            "A picture of the Cut Craft window",
        );
        ui.weak("It shows your design as it is on screen.");
        let texture = ui.ctx().load_texture(
            "bug_report_screenshot",
            Arc::clone(image),
            egui::TextureOptions::LINEAR,
        );
        let shown = ui.available_width() * THUMBNAIL_SHARE;
        let size = texture.size_vec2() * (shown / texture.size_vec2().x);
        let tint = if draft.send_screenshot {
            egui::Color32::WHITE
        } else {
            // not ticked: shown faded, so it reads as left out
            egui::Color32::from_white_alpha(UNSENT_ALPHA)
        };
        ui.add(egui::Image::new((texture.id(), size)).tint(tint));
    }
    if !draft.log.is_empty() {
        let label = if draft.crash {
            "The crash log"
        } else {
            "Cut Craft's log"
        };
        ui.checkbox(&mut draft.send_log, label);
        ui.weak(
            "What Cut Craft was doing: the tools used, what each did and how it ended. Your \
             user name and home folder are left out.",
        );
        ui.collapsing("See the log", |ui| {
            egui::ScrollArea::vertical()
                .max_height(ui.text_style_height(&egui::TextStyle::Monospace) * LOG_ROWS)
                .stick_to_bottom(true)
                .show(ui, |ui| ui.monospace(&draft.log));
        });
    }
    ui.collapsing("Always sent: the version and where you clicked", |ui| {
        ui.weak("No designs or files: only these lines.");
        if let Some(widget) = &draft.widget {
            ui.label(format!("Clicked: {widget}"));
        }
        if draft.regions.is_empty() && !draft.crash {
            ui.weak("No named part of the window here");
        }
        for r in &draft.regions {
            ui.label(format!("{} — {}:{}", r.name, r.at.file(), r.at.line()));
        }
        ui.monospace(&draft.state);
    });
    ui.separator();
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

    /// A right-click menu made with [`context_menu`] has Report a Bug Here… at its foot, and
    /// takes the right-click, so the small menu doesn't open over it as well.
    #[test]
    fn a_menu_of_its_own_ends_with_the_report() {
        let ctx = egui::Context::default();
        let mut reporter = Reporter::new("me/app", None);
        let host = Host {
            state: &|| "state".into(),
            pick_pictures: &Vec::new,
        };
        let at = egui::pos2(100.0, 100.0);
        let mut texts = vec![];
        let mut frame = |events, reporter: &mut Reporter| {
            let input = egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(800.0, 600.0),
                )),
                events,
                ..Default::default()
            };
            let mut output = ctx.run_pass(input, |ctx| {
                ctx.root_ui(|ui| {
                    egui::CentralPanel::default().show(ui, |ui| {
                        mark(ui, "Tiles");
                        let tile = ui.add_sized([400.0, 400.0], egui::Button::new("Tile"));
                        context_menu(&tile, |ui| {
                            ui.label("Mine");
                        });
                    });
                });
                reporter.frame(ctx, &host);
            });
            output.textures_delta.clear();
            texts = output
                .shapes
                .iter()
                .filter_map(|c| match &c.shape {
                    egui::Shape::Text(t) => Some(t.galley.text().to_owned()),
                    _ => None,
                })
                .collect::<Vec<_>>();
        };
        frame(vec![egui::Event::PointerMoved(at)], &mut reporter);
        frame(right_click(at), &mut reporter);
        // a new menu is drawn unseen for a frame while egui sizes it
        frame(vec![], &mut reporter);
        frame(vec![], &mut reporter);
        assert!(!reporter.menu, "the small menu stays shut");
        assert!(texts.iter().any(|t| t == "Mine"), "{texts:?}");
        let reports = texts.iter().filter(|t| *t == "Report a Bug Here…").count();
        assert_eq!(reports, 1, "{texts:?}");
    }

    #[test]
    fn nothing_personal_goes_unless_ticked() {
        let mut reporter = Reporter::new("me/app", None);
        reporter.screenshot = Some(Arc::new(egui::ColorImage::filled(
            [2, 1],
            egui::Color32::RED,
        )));
        let host = Host {
            state: &|| "Status: Opened /home/sam/Jobs/sign.ccjob".into(),
            pick_pictures: &Vec::new,
        };
        reporter.open(&host);
        let draft = reporter.draft.as_mut().unwrap();
        draft.log = "Saved /home/sam/a.nc".into();
        let report = draft.report();
        assert!(
            report.screenshot.is_none(),
            "the screenshot waits to be ticked"
        );
        assert!(report.log.is_empty(), "the log waits to be ticked");
        assert_eq!(report.state, "Status: Opened ~/Jobs/sign.ccjob");
        draft.send_screenshot = true;
        draft.send_log = true;
        let report = draft.report();
        assert!(report.screenshot.is_some());
        assert_ne!(report.log, "");
    }

    #[test]
    fn a_crash_is_titled_plainly_and_its_log_ticked_only_when_allowed() {
        let host = Host {
            state: &|| "state".into(),
            pick_pictures: &Vec::new,
        };
        let crash = "Cut Craft 0.6.3 on linux crashed: panicked at src/a.rs:1:1:\nboom\n\n\
                     Backtrace:\n  at /home/sam/src/a.rs\n";
        for allowed in [true, false] {
            let mut reporter = Reporter::new("me/app", None);
            reporter.open_crash(crash, allowed, &host);
            let draft = reporter.draft.as_ref().unwrap();
            assert_eq!(draft.title, "Crash: boom");
            assert_eq!(draft.send_log, allowed);
            assert!(draft.log.contains("at ~/src/a.rs"), "{}", draft.log);
        }
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
