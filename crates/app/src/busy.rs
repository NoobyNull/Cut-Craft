//! A slow job (calculating a toolpath, a preview, recalculating them all) run on a thread
//! of its own, so the window keeps drawing a progress bar with a Cancel button instead of
//! freezing. The job reports through [`cut_craft_core::progress`]; what it makes is handed
//! back to the window's state once it is done.
//!
//! A job that is over quickly never shows the bar: the window waits a moment for it first
//! ([`Busy::wait`]) and goes on as if it had run in place.

use cut_craft_core::progress::Progress;
use eframe::egui;
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

impl<A: 'static> Busy<A> {
    /// Starts `work` on its own thread, `then` to be given its answer (or why there is none)
    /// on the window's; `ctx` is woken when it is done.
    pub fn start<T: Send + 'static>(
        what: &str,
        ctx: Option<&egui::Context>,
        work: impl FnOnce() -> T + Send + 'static,
        then: impl FnOnce(&mut A, Result<T, String>) + 'static,
    ) -> Self {
        let progress = Progress::new();
        let done = Arc::new(AtomicBool::new(false));
        let (tracked, flag, wake) = (progress.clone(), Arc::clone(&done), ctx.cloned());
        let spawned = std::thread::Builder::new()
            .name("cut-craft job".into())
            .spawn(move || {
                let _done = Done(flag, wake);
                tracked.track(work)
            });
        let finish: Box<dyn FnOnce(&mut A)> = if let Ok(handle) = spawned {
            Box::new(move |state| then(state, handle.join().map_err(|panic| failed(&*panic))))
        } else {
            // no thread to be had (the system is out of them): the job cannot run
            done.store(true, Ordering::Release);
            Box::new(move |state| then(state, Err("there was no thread to run it on".into())))
        };
        Self {
            what: what.to_owned(),
            progress,
            started: Instant::now(),
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
        let stage = self.progress.stage();
        let mut cancel = false;
        egui::Modal::new(egui::Id::unique("busy_job")).show(ctx, |ui| {
            cut_craft_bug_report::mark(ui, "Progress window");
            ui.set_width(ui.spacing().interact_size.x * 8.0);
            ui.heading(format!("{}…", self.what));
            if !stage.is_empty() {
                ui.label(&stage);
            }
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
            ui.weak(time_line(elapsed, fraction));
            ui.add_space(ui.spacing().item_spacing.y);
            cancel = ui
                .button("Cancel")
                .on_hover_text("Stop it: nothing in the job changes")
                .clicked();
        });
        ctx.request_repaint_after(Duration::from_millis(100));
        cancel
    }
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

/// "12 s so far, about 30 s left": the time left only once enough is done to judge it.
pub fn time_line(elapsed: Duration, fraction: Option<f64>) -> String {
    let so_far = format!("{} so far", seconds(elapsed.as_secs_f64()));
    match fraction.filter(|f| (0.05..1.0).contains(f)) {
        Some(f) => {
            let left = elapsed.as_secs_f64() * (1.0 - f) / f;
            format!("{so_far}, about {} left", seconds(left))
        }
        None => so_far,
    }
}

/// A time as a person reads it: "8 s", "2 min 5 s".
fn seconds(s: f64) -> String {
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "a short time, rounded to whole seconds"
    )]
    let s = s.max(0.0).round() as u64;
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

    #[derive(Default)]
    struct State {
        answer: Option<Result<u32, String>>,
    }

    fn keep(state: &mut State, answer: Result<u32, String>) {
        state.answer = Some(answer);
    }

    #[test]
    fn the_answer_reaches_the_state_once_finished() {
        let busy = Busy::start("Adding", None, || 2 + 2, keep);
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
        let busy = Busy::start("Failing", None, || -> u32 { panic!("no model") }, keep);
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
    fn the_time_line_guesses_what_is_left_once_it_can() {
        let secs = Duration::from_secs;
        assert_eq!(time_line(secs(3), None), "3 s so far");
        assert_eq!(
            time_line(secs(3), Some(0.01)),
            "3 s so far",
            "too early to tell"
        );
        assert_eq!(
            time_line(secs(10), Some(0.25)),
            "10 s so far, about 30 s left"
        );
        assert_eq!(
            time_line(secs(125), Some(0.5)),
            "2 min 5 s so far, about 2 min 5 s left"
        );
        assert_eq!(time_line(secs(120), Some(1.0)), "2 min so far");
    }
}
