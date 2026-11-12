//! How far a long job has got, and whether the person waiting on it asked it to stop.
//!
//! The app runs a slow job (a 3D toolpath, a preview, recalculating every toolpath) on a
//! thread of its own with a [`Progress`] it shows as a bar, and installs it there with
//! [`Progress::track`]. Anything the job calls reports through the free functions here,
//! which find that thread's progress, so a command reports without the [`crate::Command`]
//! contract changing: [`report`] how much of its own work is done, [`stage`] what it is
//! doing, [`part`] to split its work between steps, and [`cancelled`] (or
//! [`stop_if_cancelled`]) to give up early once the person pressed Cancel. Off such a
//! thread (tests, quick commands run in place) every call does nothing.

use std::cell::RefCell;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

/// A job's progress, shared between the thread doing it and the window showing it.
#[derive(Clone, Debug, Default)]
pub struct Progress(Arc<Shared>);

#[derive(Debug, Default)]
struct Shared {
    /// The fraction done, as `f64` bits: for fractions from 0 to 1 the bits order as the
    /// numbers do, so the largest reported wins and the bar never runs backwards.
    done: AtomicU64,
    /// Whether anything reported a fraction: until then the bar only says it is busy.
    reported: AtomicBool,
    stage: Mutex<String>,
    cancelled: AtomicBool,
}

impl Progress {
    pub fn new() -> Self {
        Self::default()
    }

    /// How much is done, from 0 to 1; `None` while the job has not said.
    pub fn fraction(&self) -> Option<f64> {
        self.0
            .reported
            .load(Ordering::Relaxed)
            .then(|| f64::from_bits(self.0.done.load(Ordering::Relaxed)))
    }

    /// What the job says it is doing now (empty until it says).
    pub fn stage(&self) -> String {
        self.0.stage.lock().map(|s| s.clone()).unwrap_or_default()
    }

    /// Asks the job to stop: [`cancelled`] says so on its thread from now on.
    pub fn cancel(&self) {
        self.0.cancelled.store(true, Ordering::Relaxed);
    }

    pub fn is_cancelled(&self) -> bool {
        self.0.cancelled.load(Ordering::Relaxed)
    }

    /// Runs `work` with this as the thread's progress, so what it calls reports here.
    pub fn track<R>(&self, work: impl FnOnce() -> R) -> R {
        let outer = CURRENT.with(|c| {
            c.replace(Some(Span {
                progress: self.clone(),
                start: 0.0,
                width: 1.0,
            }))
        });
        let _restore = Restore(outer);
        work()
    }

    fn set(&self, fraction: f64) {
        let fraction = fraction.clamp(0.0, 1.0);
        self.0.done.fetch_max(fraction.to_bits(), Ordering::Relaxed);
        self.0.reported.store(true, Ordering::Relaxed);
    }

    fn set_stage(&self, stage: &str) {
        if let Ok(mut s) = self.0.stage.lock() {
            stage.clone_into(&mut s);
        }
    }
}

/// The part of the whole job the code running now stands for: reports inside it fill
/// `width` of the bar from `start`.
#[derive(Clone)]
struct Span {
    progress: Progress,
    start: f64,
    width: f64,
}

thread_local! {
    static CURRENT: RefCell<Option<Span>> = const { RefCell::new(None) };
}

/// Puts back the span that was current, however the work ended (a panic included).
struct Restore(Option<Span>);

impl Drop for Restore {
    fn drop(&mut self) {
        let outer = self.0.take();
        CURRENT.with(|c| *c.borrow_mut() = outer);
    }
}

fn current() -> Option<Span> {
    CURRENT.with(|c| c.borrow().clone())
}

/// `fraction` (0 to 1) of the work running now is done.
pub fn report(fraction: f64) {
    if let Some(span) = current() {
        span.progress
            .set(span.start + span.width * fraction.clamp(0.0, 1.0));
    }
}

/// `done` of `total` steps of the work running now are done.
pub fn report_steps(done: usize, total: usize) {
    if total > 0 {
        #[expect(clippy::cast_precision_loss, reason = "a fraction for a progress bar")]
        report(done as f64 / total as f64);
    }
}

/// Tells the person what the job is doing now ("Placing the model").
pub fn stage(text: &str) {
    if let Some(span) = current() {
        span.progress.set_stage(text);
        // in the log too, so a crash's log shows the step a long job was on
        log::info!("Step: {text}");
    }
}

/// Runs `work` as step `index` of `count` equal steps of the work running now: its own
/// reports fill only that step's share, and the step counts as done when it returns.
pub fn part<R>(index: usize, count: usize, work: impl FnOnce() -> R) -> R {
    let Some(span) = current().filter(|_| count > 0) else {
        return work();
    };
    #[expect(clippy::cast_precision_loss, reason = "a fraction for a progress bar")]
    let (index, count) = (index as f64, count as f64);
    let width = span.width / count;
    let inner = Span {
        progress: span.progress.clone(),
        start: span.start + width * index,
        width,
    };
    let outer = CURRENT.with(|c| c.replace(Some(inner)));
    let _restore = Restore(outer);
    let answer = work();
    span.progress.set(span.start + width * (index + 1.0));
    answer
}

/// Whether the person asked the job running on this thread to stop.
pub fn cancelled() -> bool {
    current().is_some_and(|s| s.progress.is_cancelled())
}

/// The error a command gives up with once the person asked it to stop; nobody sees it,
/// as the app has already let the job go.
pub fn stop_if_cancelled() -> Result<(), crate::CommandError> {
    if cancelled() {
        Err(crate::CommandError::Failed("stopped".into()))
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nothing_is_reported_off_a_tracked_thread() {
        report(0.5);
        stage("Busy");
        assert!(!cancelled());
        assert!(stop_if_cancelled().is_ok());
        assert_eq!(part(1, 2, || 7), 7);
    }

    #[test]
    fn a_tracked_job_reports_its_fraction_and_stage() {
        let progress = Progress::new();
        assert_eq!(progress.fraction(), None, "busy, but not saying how far");
        progress.track(|| {
            stage("Dropping the tool");
            report(0.25);
        });
        assert_eq!(progress.fraction(), Some(0.25));
        assert_eq!(progress.stage(), "Dropping the tool");
        report(0.9);
        assert_eq!(
            progress.fraction(),
            Some(0.25),
            "outside track the thread reports nowhere"
        );
    }

    #[test]
    fn the_bar_never_runs_backwards_or_past_the_end() {
        let progress = Progress::new();
        progress.track(|| {
            report(0.6);
            report(0.3);
        });
        assert_eq!(progress.fraction(), Some(0.6));
        progress.track(|| report(7.0));
        assert_eq!(progress.fraction(), Some(1.0));
    }

    #[test]
    fn steps_count_towards_the_fraction() {
        let progress = Progress::new();
        progress.track(|| report_steps(3, 4));
        assert_eq!(progress.fraction(), Some(0.75));
        progress.track(|| report_steps(1, 0));
        assert_eq!(progress.fraction(), Some(0.75), "no steps: nothing to say");
    }

    #[test]
    fn parts_split_the_bar_and_nest() {
        let progress = Progress::new();
        progress.track(|| {
            part(1, 4, || {
                report(0.5);
            });
        });
        // step 1 of 4 is 0.25..0.5; done, it fills its share
        assert_eq!(progress.fraction(), Some(0.5));
        let nested = Progress::new();
        let inside = nested.track(|| {
            part(1, 2, || {
                part(1, 2, || {
                    report(0.0);
                    nested.fraction()
                })
            })
        });
        assert_eq!(inside, Some(0.75), "the last quarter is under way");
        assert_eq!(
            nested.fraction(),
            Some(1.0),
            "and done once its parts return"
        );
        let early = Progress::new();
        early.track(|| part(2, 4, || report(0.5)));
        assert_eq!(early.fraction(), Some(0.75));
    }

    #[test]
    fn cancel_reaches_the_job_on_its_own_thread() {
        let progress = Progress::new();
        let job = progress.clone();
        let (go, wait) = std::sync::mpsc::channel();
        let worker = std::thread::spawn(move || {
            job.track(|| {
                wait.recv().unwrap();
                stop_if_cancelled()
            })
        });
        progress.cancel();
        go.send(()).unwrap();
        assert!(worker.join().unwrap().is_err(), "the job saw Cancel");
        assert!(progress.is_cancelled());
    }

    #[test]
    fn the_thread_forgets_the_progress_after_a_panic() {
        let progress = Progress::new();
        let caught = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            progress.track(|| panic!("a job that fails"));
        }));
        assert!(caught.is_err());
        progress.cancel();
        assert!(!cancelled(), "nothing is tracked here any more");
    }
}
