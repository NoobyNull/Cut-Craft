//! A slow job (calculating a toolpath, a preview, recalculating them all) run on a thread
//! of its own, so the window keeps drawing a progress bar with a Cancel button instead of
//! freezing. The job reports through [`cut_craft_core::progress`]; what it makes is handed
//! back to the window's state once it is done.
//!
//! A job that is over quickly never shows the bar: the window waits a moment for it first
//! ([`Busy::wait`]) and goes on as if it had run in place.

use cut_craft_core::eta::{self, Estimate, Estimator};
use cut_craft_core::progress::Progress;
use eframe::egui;
use serde_json::{Value, json};
use std::cell::RefCell;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

/// How long the window waits for a job before showing its progress: about what a person
/// notices as a pause.
pub const SHOW_AFTER: Duration = Duration::from_millis(250);

/// A job running for state `A` (the app): how far it is, and what to do with its answer.
pub struct Busy<A> {
    /// What it is doing, as the window's heading says ("Calculating 3D Roughing").
    what: String,
    progress: Progress,
    started: Instant,
    /// Works out the time left from how the job is going and how long its kind took last
    /// time ([`eta`]).
    estimator: RefCell<Estimator>,
    done: Arc<AtomicBool>,
    /// Waits for the answer and hands it to the state.
    finish: Box<dyn FnOnce(&mut A)>,
}

/// Says the job is done however its thread ends, a panic included.
struct Done(Arc<AtomicBool>, Option<egui::Context>);

impl Drop for Done {
    fn drop(&mut self) {
        self.0.store(true, Ordering::Release);
        if let Some(ctx) = &self.1 {
            ctx.request_repaint();
        }
    }
}

/// Whether a job's answer is one worth timing for [`eta`]'s history: a calculation that
/// was refused (an `Err`) took no real time, so it would throw the next guess off.
pub trait Outcome {
    fn succeeded(&self) -> bool {
        true
    }
}

impl<T, E> Outcome for Result<T, E> {
    fn succeeded(&self) -> bool {
        self.is_ok()
    }
}

// answers that are always worth timing: a preview's material, the checks' findings
impl Outcome for () {}
impl Outcome for u32 {}
impl Outcome for cut_craft_core::simulation::Material {}
impl Outcome for cut_craft_core::playback::Playback {}
impl<T> Outcome for Vec<T> {}
impl<K, V> Outcome for std::collections::BTreeMap<K, V> {}
impl<A, B> Outcome for (A, B) {}
impl<A, B, C> Outcome for (A, B, C) {}

impl<A: 'static> Busy<A> {
    /// Starts `work` on its own thread, `then` to be given its answer (or why there is none)
    /// on the window's; `ctx` is woken when it is done. `kind` names the sort of job for
    /// [`eta`]'s history ("calculate:pocket-toolpath"): its time left starts from how long
    /// the last one took, and a job that succeeds ([`Outcome`]) without being cancelled is
    /// remembered under it, timed as its thread finishes.
    pub fn start<T: Outcome + Send + 'static>(
        what: &str,
        kind: &str,
        ctx: Option<&egui::Context>,
        work: impl FnOnce() -> T + Send + 'static,
        then: impl FnOnce(&mut A, Result<T, String>) + 'static,
    ) -> Self {
        let progress = Progress::new();
        let started = Instant::now();
        let done = Arc::new(AtomicBool::new(false));
        let (tracked, flag, wake) = (progress.clone(), Arc::clone(&done), ctx.cloned());
        let timed = kind.to_owned();
        let spawned = std::thread::Builder::new()
            .name("cut-craft job".into())
            .spawn(move || {
                let _done = Done(flag, wake);
                let answer = tracked.track(work);
                if answer.succeeded() && !tracked.is_cancelled() {
                    eta::record(&timed, started.elapsed().as_secs_f64());
                }
                answer
            });
        // logged as started now and as finished or failed once its answer is used; a crash
        // meanwhile names it and the step it was on
        let at = progress.clone();
        let doing = cut_craft_bug_report::logs::doing_with(what, move || at.stage());
        let estimator = RefCell::new(Estimator::new(eta::usual(kind)));
        let finish: Box<dyn FnOnce(&mut A)> = if let Ok(handle) = spawned {
            Box::new(move |state| {
                let answer = handle.join().map_err(|panic| failed(&*panic));
                match &answer {
                    Ok(_) => doing.finished(""),
                    Err(why) => doing.failed(why),
                }
                then(state, answer);
            })
        } else {
            // no thread to be had (the system is out of them): the job cannot run
            done.store(true, Ordering::Release);
            Box::new(move |state| then(state, Err("there was no thread to run it on".into())))
        };
        Self {
            what: what.to_owned(),
            progress,
            started,
            estimator,
            done,
            finish,
        }
    }

    pub fn what(&self) -> &str {
        &self.what
    }

    pub fn is_done(&self) -> bool {
        self.done.load(Ordering::Acquire)
    }

    /// Waits up to `budget` for the job; whether it is done.
    pub fn wait(&self, budget: Duration) -> bool {
        let until = Instant::now() + budget;
        while !self.is_done() {
            if Instant::now() >= until {
                return false;
            }
            std::thread::sleep(Duration::from_millis(1));
        }
        true
    }

    /// Hands the answer to `state`, waiting for it if the job is still running.
    pub fn finish(self, state: &mut A) {
        (self.finish)(state);
    }

    /// The step it is on now.
    pub fn stage(&self) -> String {
        self.progress.stage()
    }

    /// How long the job has left, as it looks now.
    pub fn estimate(&self) -> Estimate {
        let elapsed = self.started.elapsed().as_secs_f64();
        self.estimator
            .borrow_mut()
            .update(elapsed, self.progress.fraction())
    }

    /// The job as the Web API's `GET progress` (and `GET status`) tells it: what it is
    /// doing, the step, how far it has got, how long it has run, and the time left worked
    /// out as the progress window's.
    pub fn status(&self) -> Value {
        let estimate = self.estimate();
        let round = |s: f64| (s * 10.0).round() / 10.0;
        json!({
            "busy": true,
            "doing": self.what,
            "step": self.progress.stage(),
            "fraction": self.progress.fraction(),
            "seconds": round(self.started.elapsed().as_secs_f64()),
            "seconds_left": estimate.seconds_left().map(round),
            "time_left": estimate.words(),
        })
    }

    /// Asks the job to stop. The window lets it go at once; what it makes is thrown away.
    pub fn cancel(self) {
        self.progress.cancel();
    }

    /// The window over the app while the job runs: what it is doing, how far it has got,
    /// the time it has taken and (once it can tell) about how long is left, and Cancel.
    /// Whether Cancel was pressed.
    pub fn window(&self, ctx: &egui::Context) -> bool {
        let elapsed = self.started.elapsed();
        let fraction = self.progress.fraction();
        let estimate = self.estimate();
        let stage = self.progress.stage();
        let note = self.progress.note();
        let mut cancel = false;
        egui::Modal::new(egui::Id::unique("busy_job")).show(ctx, |ui| {
            cut_craft_bug_report::mark(ui, "Progress window");
            ui.set_width(ui.spacing().interact_size.x * 8.0);
            ui.heading(format!("{}…", self.what));
            // what it is doing and its word on that, each in room of its own that never
            // grows or shrinks, so the window holds still as the steps change
            fixed_lines(ui, egui::RichText::new(stage), 2);
            fixed_lines(ui, egui::RichText::new(note).weak().small(), 2);
            match fraction {
                Some(f) => {
                    #[expect(clippy::cast_possible_truncation, reason = "a bar's fill")]
                    let bar = egui::ProgressBar::new(f as f32).show_percentage();
                    ui.add(bar.animate(true));
                }
                None => {
                    ui.horizontal(|ui| {
                        ui.spinner();
                        ui.label("Working…");
                    });
                }
            }
            ui.weak(time_line(elapsed, estimate));
            ui.add_space(ui.spacing().item_spacing.y);
            cancel = ui
                .button("Cancel")
                .on_hover_text("Stop it (Esc): nothing in the job changes")
                .clicked();
        });
        // Escape is Cancel, as in any other window
        cancel |= ctx.input_mut(|i| {
            i.consume_shortcut(&egui::KeyboardShortcut::new(
                egui::Modifiers::NONE,
                egui::Key::Escape,
            ))
        });
        ctx.request_repaint_after(Duration::from_millis(100));
        cancel
    }
}

/// `text` wrapped in room `lines` lines high across the window, whatever it says (blank
/// included): longer is cut short with "…".
fn fixed_lines(ui: &mut egui::Ui, text: egui::RichText, lines: usize) {
    let width = ui.available_width();
    #[expect(clippy::cast_precision_loss, reason = "a few lines")]
    let height = ui.text_style_height(&egui::TextStyle::Body) * lines as f32;
    ui.allocate_ui_with_layout(
        egui::vec2(width, height),
        egui::Layout::top_down(egui::Align::Min),
        |ui| {
            ui.set_min_size(egui::vec2(width, height));
            ui.set_max_size(egui::vec2(width, height));
            let wrapping = egui::text::TextWrapping {
                max_width: width,
                max_rows: lines,
                ..Default::default()
            };
            let galley = egui::WidgetText::from(text).into_galley_impl(
                ui.ctx(),
                ui.style(),
                wrapping,
                egui::FontSelection::Default,
                egui::Align::Min,
            );
            ui.label(galley);
        },
    );
}

/// What a person is told when a job panicked, from what the panic said.
fn failed(panic: &(dyn std::any::Any + Send)) -> String {
    let why = panic
        .downcast_ref::<&str>()
        .map(|s| (*s).to_owned())
        .or_else(|| panic.downcast_ref::<String>().cloned())
        .unwrap_or_else(|| "it stopped unexpectedly".into());
    format!("Something went wrong: {why}. Please report it (Help ▸ Report a Problem)")
}

/// "12 s so far, about 30 seconds left": the time taken, and what [`eta`] makes of the
/// time left when it has anything to say.
pub fn time_line(elapsed: Duration, estimate: Estimate) -> String {
    let so_far = format!("{} so far", seconds(elapsed.as_secs_f64()));
    match estimate.words() {
        Some(left) => format!("{so_far}, {left}"),
        None => so_far,
    }
}

/// A time as a person reads it: "8 s", "2 min 5 s", "3 h 20 min".
pub fn seconds(s: f64) -> String {
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "a time, rounded to whole seconds"
    )]
    let s = s.max(0.0).round() as u64;
    if s >= 3600 {
        return match (s / 3600, s % 3600 / 60) {
            (h, 0) => format!("{h} h"),
            (h, m) => format!("{h} h {m} min"),
        };
    }
    match (s / 60, s % 60) {
        (0, s) => format!("{s} s"),
        (m, 0) => format!("{m} min"),
        (m, s) => format!("{m} min {s} s"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cut_craft_core::progress;

    #[test]
    fn times_read_as_a_person_says_them() {
        assert_eq!(seconds(8.2), "8 s");
        assert_eq!(seconds(125.0), "2 min 5 s");
        assert_eq!(seconds(12_000.0), "3 h 20 min");
        assert_eq!(seconds(7200.0), "2 h");
    }

    #[derive(Default)]
    struct State {
        answer: Option<Result<u32, String>>,
    }

    fn keep(state: &mut State, answer: Result<u32, String>) {
        state.answer = Some(answer);
    }

    #[test]
    fn the_answer_reaches_the_state_once_finished() {
        let busy = Busy::start("Adding", "test:adding", None, || 2 + 2, keep);
        assert!(busy.wait(Duration::from_secs(10)));
        let mut state = State::default();
        busy.finish(&mut state);
        assert_eq!(state.answer, Some(Ok(4)));
    }

    #[test]
    fn a_slow_job_is_not_waited_for_past_the_budget() {
        let (go, wait) = std::sync::mpsc::channel::<()>();
        let busy = Busy::start(
            "Waiting",
            "test:waiting",
            None,
            move || {
                wait.recv().ok();
                1
            },
            keep,
        );
        assert!(!busy.wait(Duration::from_millis(20)));
        assert!(!busy.is_done());
        go.send(()).unwrap();
        assert!(busy.wait(Duration::from_secs(10)));
    }

    #[test]
    fn a_job_that_panics_says_so_instead_of_taking_the_app_down() {
        let busy = Busy::start(
            "Failing",
            "test:failing",
            None,
            || -> u32 { panic!("no model") },
            keep,
        );
        assert!(busy.wait(Duration::from_secs(10)), "a panic still ends it");
        let mut state = State::default();
        busy.finish(&mut state);
        let why = state.answer.unwrap().unwrap_err();
        assert!(
            why.contains("no model") && why.contains("Report a Problem"),
            "{why}"
        );
    }

    #[test]
    fn the_job_reports_progress_and_sees_cancel() {
        let (go, wait) = std::sync::mpsc::channel::<()>();
        let (seen, saw) = std::sync::mpsc::channel::<bool>();
        let busy = Busy::start(
            "Counting",
            "test:counting",
            None,
            move || {
                progress::stage("Halfway");
                progress::report(0.5);
                wait.recv().ok();
                seen.send(progress::cancelled()).ok();
                0
            },
            keep,
        );
        while busy.progress.fraction().is_none() {
            std::thread::yield_now();
        }
        assert_eq!(busy.progress.fraction(), Some(0.5));
        assert_eq!(busy.progress.stage(), "Halfway");
        busy.cancel();
        go.send(()).unwrap();
        assert!(saw.recv().unwrap(), "the job was told to stop");
    }

    #[test]
    fn the_window_shows_what_is_happening_how_far_and_cancel() {
        let ctx = egui::Context::default();
        let (go, wait) = std::sync::mpsc::channel::<()>();
        let busy: Busy<State> = Busy::start(
            "Calculating 3D Roughing",
            "test:window",
            Some(&ctx),
            move || {
                progress::report(0.25);
                wait.recv().ok();
                1
            },
            keep,
        );
        while busy.progress.fraction().is_none() {
            std::thread::yield_now();
        }
        let mut texts = vec![];
        let mut output = None;
        // a window is laid out on its first frame and drawn from the second
        for _ in 0..2 {
            let mut frame = ctx.run_ui(egui::RawInput::default(), |ui| {
                assert!(!busy.window(ui.ctx()));
            });
            frame.textures_delta.clear();
            output = Some(frame);
        }
        for shape in output.unwrap().shapes {
            if let egui::epaint::Shape::Text(t) = shape.shape {
                texts.push(t.galley.text().to_owned());
            }
        }
        assert!(
            texts.iter().any(|t| t == "Calculating 3D Roughing…"),
            "{texts:?}"
        );
        assert!(texts.iter().any(|t| t == "25%"), "{texts:?}");
        assert!(texts.iter().any(|t| t == "Cancel"), "{texts:?}");
        go.send(()).unwrap();
    }

    #[test]
    fn the_window_holds_still_whatever_the_step_says() {
        let ctx = egui::Context::default();
        let (go, wait) = std::sync::mpsc::channel::<()>();
        let (said, heard) = std::sync::mpsc::channel::<()>();
        let busy: Busy<State> = Busy::start(
            "Calculating Pocket",
            "test:holds-still",
            Some(&ctx),
            move || {
                progress::report(0.1);
                wait.recv().ok();
                progress::stage("Adaptive: clearing the wider areas, a long way round");
                progress::note(&"Some areas take longer than others. ".repeat(10));
                said.send(()).ok();
                wait.recv().ok();
                1
            },
            keep,
        );
        while busy.progress.fraction().is_none() {
            std::thread::yield_now();
        }
        // where the Cancel button's word is drawn
        let cancel_at = |ctx: &egui::Context| {
            let mut at = None;
            for _ in 0..2 {
                let mut frame = ctx.run_ui(egui::RawInput::default(), |ui| {
                    busy.window(ui.ctx());
                });
                frame.textures_delta.clear();
                for shape in frame.shapes {
                    if let egui::epaint::Shape::Text(t) = shape.shape
                        && t.galley.text() == "Cancel"
                    {
                        at = Some(t.pos);
                    }
                }
            }
            at.expect("a Cancel button")
        };
        let before = cancel_at(&ctx);
        go.send(()).unwrap();
        heard.recv().unwrap();
        let after = cancel_at(&ctx);
        assert_eq!(before, after, "the window moved when the step changed");
        go.send(()).unwrap();
    }

    #[test]
    fn the_time_line_says_the_time_taken_and_what_is_left() {
        let secs = Duration::from_secs;
        assert_eq!(time_line(secs(3), Estimate::Unknown), "3 s so far");
        assert_eq!(
            time_line(secs(3), Estimate::Estimating),
            "3 s so far, working out how long it will take…"
        );
        assert_eq!(
            time_line(secs(10), Estimate::Left(31.0)),
            "10 s so far, about 30 seconds left"
        );
        assert_eq!(
            time_line(secs(125), Estimate::Left(125.0)),
            "2 min 5 s so far, about 2 minutes left"
        );
        assert_eq!(
            time_line(secs(20), Estimate::Usually(45.0)),
            "20 s so far, usually takes about 45 seconds"
        );
    }

    #[test]
    fn a_finished_job_is_remembered_and_starts_the_next_ones_guess() {
        let kind = "test:remembered";
        let busy = Busy::start(
            "Remembering",
            kind,
            None,
            || std::thread::sleep(Duration::from_millis(30)),
            |_: &mut State, _| {},
        );
        assert!(busy.wait(Duration::from_secs(10)));
        busy.finish(&mut State::default());
        let took = eta::usual(kind).expect("remembered");
        assert!(took >= 0.03, "{took}");
        // the next one, saying nothing of how far it is, is told how long it usually takes
        let (go, wait) = std::sync::mpsc::channel::<()>();
        let next: Busy<State> = Busy::start(
            "Remembering",
            kind,
            None,
            move || {
                wait.recv().ok();
            },
            |_, _| {},
        );
        assert_eq!(next.estimate(), Estimate::Usually(took));
        let status = next.status();
        assert_eq!(status["doing"], "Remembering");
        assert!(
            status["time_left"]
                .as_str()
                .unwrap()
                .starts_with("usually takes")
        );
        assert_eq!(status["seconds_left"], Value::Null);
        go.send(()).unwrap();
    }

    #[test]
    fn a_refused_or_cancelled_job_is_not_remembered() {
        let kind = "test:refused";
        let busy = Busy::start(
            "Refusing",
            kind,
            None,
            || -> Result<u32, String> { Err("select a vector".into()) },
            |_: &mut State, _| {},
        );
        assert!(busy.wait(Duration::from_secs(10)));
        busy.finish(&mut State::default());
        assert_eq!(eta::usual(kind), None, "refused: not timed");
        let kind = "test:cancelled";
        let (go, wait) = std::sync::mpsc::channel::<()>();
        let (seen, saw) = std::sync::mpsc::channel::<()>();
        let busy: Busy<State> = Busy::start(
            "Cancelling",
            kind,
            None,
            move || {
                wait.recv().ok();
                seen.send(()).ok();
            },
            |_, _| {},
        );
        busy.cancel();
        go.send(()).unwrap();
        saw.recv().unwrap();
        std::thread::sleep(Duration::from_millis(50));
        assert_eq!(eta::usual(kind), None, "cancelled: not timed");
    }

    #[test]
    fn a_failed_job_is_not_remembered() {
        let kind = "test:not-remembered";
        let busy = Busy::start("Failing", kind, None, || -> u32 { panic!("no") }, keep);
        assert!(busy.wait(Duration::from_secs(10)));
        busy.finish(&mut State::default());
        assert_eq!(eta::usual(kind), None);
    }
}
