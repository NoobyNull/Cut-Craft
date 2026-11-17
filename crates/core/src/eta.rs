//! How long a slow job has left, told the way a person waiting on it wants to hear it.
//!
//! Dividing the time taken by the share done jumps about: a job's steps run at different
//! speeds, and early on a few hundredths of a second of noise swing the answer by minutes.
//! [`Estimator`] steadies it instead:
//!
//! - the rate is a blend of the average over the last stretch of the job and a smoothed
//!   rate (an exponentially weighted moving average), so a slow or quick step moves it
//!   gradually and the guess follows a job whose pace changes part way;
//! - until enough is done to judge, it says it is still working that out, unless
//!   [`History`] knows how long this kind of job took last time, which it starts from and
//!   leaves behind as the job's own pace shows;
//! - the time shown counts down with the clock and only drifts towards a new guess, more
//!   slowly upwards than downwards, so it never leaps back up;
//! - a job that cannot say how far it has got is told how long it usually takes.
//!
//! [`Estimate::words`] turns the answer into "about 2 minutes left".

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::sync::{LazyLock, Mutex};

/// How quickly the smoothed rate follows the job's pace, in seconds.
const RATE_SECONDS: f64 = 6.0;
/// The window the recent average rate is taken over: this share of the time so far, but
/// never less than [`WINDOW_SECONDS`].
const WINDOW_SHARE: f64 = 0.3;
const WINDOW_SECONDS: f64 = 10.0;
/// Samples closer together than this say little about the pace.
const SAMPLE_SECONDS: f64 = 0.25;
/// How quickly the time shown follows a shorter guess, and a longer one, in seconds.
const DOWN_SECONDS: f64 = 4.0;
const UP_SECONDS: f64 = 10.0;
/// A longer guess within this share of the time shown is put down to noise.
const UP_SLACK: f64 = 0.15;
/// With no history, nothing is guessed before this share is done and this time has gone.
const FIRST_SHARE: f64 = 0.03;
const FIRST_SECONDS: f64 = 2.0;
/// The share done and time gone by which the job's own pace counts in full over history.
const TRUSTED_SHARE: f64 = 0.15;
const TRUSTED_SECONDS: f64 = 3.0;
/// Under this, the job is almost done.
const ALMOST_SECONDS: f64 = 3.0;

/// What to tell the person about the time left.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Estimate {
    /// Nothing to go on: the job does not say how far it is and has no history.
    Unknown,
    /// Too early to tell.
    Estimating,
    /// About this many seconds left.
    Left(f64),
    /// A few seconds at most.
    AlmostDone,
    /// The job does not say how far it is, but this kind usually takes this many seconds.
    Usually(f64),
    /// As [`Estimate::Usually`], but it has already run well past that.
    LongerThanUsual,
}

impl Estimate {
    /// The seconds left, when there is a number to give.
    pub fn seconds_left(self) -> Option<f64> {
        match self {
            Self::Left(s) => Some(s),
            Self::AlmostDone => Some(0.0),
            _ => None,
        }
    }

    /// "about 2 minutes left", "working out how long it will take…"; `None` when there is
    /// nothing to say.
    pub fn words(self) -> Option<String> {
        Some(match self {
            Self::Unknown => return None,
            Self::Estimating => "working out how long it will take…".to_owned(),
            Self::Left(s) => format!("about {} left", roughly(s)),
            Self::AlmostDone => "almost done".to_owned(),
            Self::Usually(s) => format!("usually takes about {}", roughly(s)),
            Self::LongerThanUsual => "taking longer than usual".to_owned(),
        })
    }
}

/// A time rounded as a person says it: "10 seconds", "1 minute", "25 minutes",
/// "1 hour 20 minutes". Short times to 5 seconds, then whole minutes, then 5 minutes.
pub fn roughly(seconds: f64) -> String {
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "a time, rounded to a few seconds"
    )]
    let whole = |s: f64, step: f64| ((s / step).round() * step).max(step) as u64;
    let plural = |n: u64, one: &str| {
        if n == 1 {
            format!("1 {one}")
        } else {
            format!("{n} {one}s")
        }
    };
    let s = seconds.max(0.0);
    if s < 57.5 {
        return plural(whole(s, 5.0), "second");
    }
    if s < 3570.0 {
        return plural(whole(s, 60.0) / 60, "minute");
    }
    let minutes = whole(s, 300.0) / 60;
    match (minutes / 60, minutes % 60) {
        (h, 0) => plural(h, "hour"),
        (h, m) => format!("{} {}", plural(h, "hour"), plural(m, "minute")),
    }
}

/// Follows one job's progress and says how long it has left; fed once a frame or so with
/// the time it has run and the share done.
#[derive(Clone, Debug, Default)]
pub struct Estimator {
    /// How long this kind of job took last time, in seconds.
    usual: Option<f64>,
    /// The samples kept, oldest first: time and share done.
    samples: std::collections::VecDeque<(f64, f64)>,
    /// The smoothed recent rate, share per second.
    rate: Option<f64>,
    /// The time left last shown, and when.
    shown: Option<(f64, f64)>,
}

impl Estimator {
    /// For a job of a kind that took `usual` seconds last time, when known.
    pub fn new(usual: Option<f64>) -> Self {
        Self {
            usual: usual.filter(|s| s.is_finite() && *s > 0.0),
            ..Self::default()
        }
    }

    /// The time left after `elapsed` seconds with `fraction` (0 to 1) done, or `None` when
    /// the job does not say how far it is.
    pub fn update(&mut self, elapsed: f64, fraction: Option<f64>) -> Estimate {
        let Some(f) = fraction.map(|f| f.clamp(0.0, 1.0)) else {
            return match self.usual {
                None => Estimate::Unknown,
                Some(usual) if elapsed > usual * 1.25 + 2.0 => Estimate::LongerThanUsual,
                Some(usual) => Estimate::Usually(usual),
            };
        };
        if f >= 0.999 {
            return Estimate::AlmostDone;
        }
        self.sample(elapsed, f);
        let Some(guess) = self.guess(elapsed, f) else {
            return Estimate::Estimating;
        };
        let left = self.steadied(elapsed, guess);
        if left < ALMOST_SECONDS {
            Estimate::AlmostDone
        } else {
            Estimate::Left(left)
        }
    }

    /// Keeps the sample and moves the recent rate towards the pace since the last one.
    fn sample(&mut self, t: f64, f: f64) {
        match self.samples.back().copied() {
            None => {
                self.samples.push_back((t, f));
                if f > 0.0 && t > 0.0 {
                    self.rate = Some(f / t);
                }
            }
            Some((lt, lf)) if t - lt >= SAMPLE_SECONDS => {
                let dt = t - lt;
                let now = ((f - lf) / dt).max(0.0);
                let weight = 1.0 - (-dt / RATE_SECONDS).exp();
                // it starts once something is done, from the average so far
                self.rate = match self.rate {
                    Some(r) => Some(r + weight * (now - r)),
                    None => (f > 0.0).then(|| f / t),
                };
                self.samples.push_back((t, f));
                // the oldest go once a later one still covers the window
                let window = (t * WINDOW_SHARE).max(WINDOW_SECONDS);
                while self.samples.get(1).is_some_and(|&(st, _)| t - st >= window) {
                    self.samples.pop_front();
                }
            }
            Some(_) => {}
        }
    }

    /// The time left from the pace so far, leaning on history until there is enough of it.
    fn guess(&self, t: f64, f: f64) -> Option<f64> {
        let average = match self.samples.front() {
            Some(&(st, sf)) if t - st >= SAMPLE_SECONDS => (f - sf) / (t - st),
            _ if t > 0.0 => f / t,
            _ => 0.0,
        };
        let rate = 0.5 * average + 0.5 * self.rate.unwrap_or(average);
        let own = (rate > 0.0).then(|| (1.0 - f) / rate);
        let usual = self.usual.map(|u| u - t).filter(|left| *left > 0.0);
        let trust = (f / TRUSTED_SHARE).min(1.0) * (t / TRUSTED_SECONDS).min(1.0);
        match (own, usual) {
            (Some(own), Some(usual)) => Some(trust * own + (1.0 - trust) * usual),
            (None, Some(usual)) => Some(usual),
            (Some(own), None) if f >= FIRST_SHARE && t >= FIRST_SECONDS => Some(own),
            _ => None,
        }
    }

    /// What to show: the last time shown counted down with the clock, drawn towards
    /// `guess` (gently when that is longer).
    fn steadied(&mut self, t: f64, guess: f64) -> f64 {
        let left = match self.shown {
            None => guess,
            Some((at, shown)) => {
                let dt = (t - at).max(0.0);
                let counted = (shown - dt).max(0.0);
                if guess < counted {
                    counted + (1.0 - (-dt / DOWN_SECONDS).exp()) * (guess - counted)
                } else if guess > counted * (1.0 + UP_SLACK) + 1.0 {
                    let towards = guess / (1.0 + UP_SLACK);
                    counted + (1.0 - (-dt / UP_SECONDS).exp()) * (towards - counted)
                } else {
                    counted
                }
            }
        };
        self.shown = Some((t, left));
        left
    }
}

/// How long each kind of job took last time, to start the next one's guess from and to
/// weigh steps of different kinds ([`crate::progress::part_weighted`]).
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct History {
    seconds: BTreeMap<String, f64>,
}

/// How much the latest run counts against those before it.
const LATEST_WEIGHT: f64 = 0.5;

impl History {
    /// How long a job of `kind` usually takes, in seconds.
    pub fn usual(&self, kind: &str) -> Option<f64> {
        self.seconds.get(kind).copied()
    }

    /// A job of `kind` took `seconds`: blended with earlier runs on a log scale, so one
    /// unusually big or small job does not throw the next guess far off.
    pub fn record(&mut self, kind: &str, seconds: f64) {
        if !seconds.is_finite() || seconds <= 0.0 || kind.is_empty() {
            return;
        }
        let blended = match self.seconds.get(kind) {
            Some(&old) => (old.ln() * (1.0 - LATEST_WEIGHT) + seconds.ln() * LATEST_WEIGHT).exp(),
            None => seconds,
        };
        self.seconds.insert(kind.to_owned(), blended);
    }

    /// The history kept in `path` (empty if there is none or it will not read).
    pub fn load(path: &std::path::Path) -> Self {
        std::fs::read_to_string(path)
            .ok()
            .and_then(|text| serde_json::from_str(&text).ok())
            .unwrap_or_default()
    }

    /// Writes the history to `path`.
    pub fn save(&self, path: &std::path::Path) -> std::io::Result<()> {
        let text = serde_json::to_string_pretty(self).map_err(std::io::Error::other)?;
        std::fs::write(path, text)
    }
}

static HISTORY: LazyLock<Mutex<History>> = LazyLock::new(Mutex::default);

/// How long a job of `kind` usually takes, from this run's shared history.
pub fn usual(kind: &str) -> Option<f64> {
    HISTORY.lock().ok().and_then(|h| h.usual(kind))
}

/// A job of `kind` took `seconds`, in this run's shared history.
pub fn record(kind: &str, seconds: f64) {
    if let Ok(mut h) = HISTORY.lock() {
        h.record(kind, seconds);
    }
}

/// The shared history as it stands, to save.
pub fn history() -> History {
    HISTORY.lock().map_or_default(|h| h.clone())
}

/// Replaces the shared history (with what was saved last time).
pub fn set_history(history: History) {
    if let Ok(mut h) = HISTORY.lock() {
        *h = history;
    }
}

/// Each step's share of a progress bar ([`crate::progress::part_weighted`]) from how long
/// its kind usually takes; a step of no kind, or one not timed yet, counts as the average
/// of those that are (all equal when none are).
pub fn weights(kinds: &[Option<String>]) -> Vec<f64> {
    let known: Vec<Option<f64>> = kinds.iter().map(|k| k.as_deref().and_then(usual)).collect();
    let timed: Vec<f64> = known.iter().flatten().copied().collect();
    #[expect(clippy::cast_precision_loss, reason = "an average of a few times")]
    let average = if timed.is_empty() {
        1.0
    } else {
        timed.iter().sum::<f64>() / timed.len() as f64
    };
    known.into_iter().map(|k| k.unwrap_or(average)).collect()
}

/// The history kind for calculating a toolpath with command `id`, however it was started
/// (its form, Edit Toolpath or Recalculate).
pub fn calculating(id: &str) -> String {
    format!("calculate:{id}")
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Feeds `est` a job running at `rate` share per second, every 0.1 s up to `until`,
    /// from `from`; the last estimate.
    fn run(est: &mut Estimator, from: f64, until: f64, rate: impl Fn(f64) -> f64) -> Estimate {
        let mut t = from;
        let mut last = Estimate::Unknown;
        while t <= until + 1e-9 {
            last = est.update(t, Some(rate(t)));
            t += 0.1;
        }
        last
    }

    fn left(e: Estimate) -> f64 {
        e.seconds_left()
            .unwrap_or_else(|| panic!("no time left in {e:?}"))
    }

    #[test]
    fn times_are_rounded_as_a_person_says_them() {
        assert_eq!(roughly(3.0), "5 seconds");
        assert_eq!(roughly(33.0), "35 seconds");
        assert_eq!(roughly(58.0), "1 minute");
        assert_eq!(roughly(100.0), "2 minutes");
        assert_eq!(roughly(25.0 * 60.0 + 10.0), "25 minutes");
        assert_eq!(roughly(3600.0), "1 hour");
        assert_eq!(roughly(80.0 * 60.0 + 70.0), "1 hour 20 minutes");
        assert_eq!(roughly(2.0 * 3600.0 + 61.0), "2 hours");
    }

    #[test]
    fn the_words_say_what_a_novice_needs() {
        assert_eq!(
            Estimate::Left(120.0).words().unwrap(),
            "about 2 minutes left"
        );
        assert_eq!(Estimate::AlmostDone.words().unwrap(), "almost done");
        assert_eq!(
            Estimate::Usually(40.0).words().unwrap(),
            "usually takes about 40 seconds"
        );
        assert!(
            Estimate::Estimating
                .words()
                .unwrap()
                .contains("working out")
        );
        assert_eq!(Estimate::Unknown.words(), None);
    }

    #[test]
    fn it_says_it_is_estimating_until_enough_is_done() {
        let mut est = Estimator::new(None);
        assert_eq!(est.update(0.5, Some(0.0)), Estimate::Estimating);
        assert_eq!(est.update(1.0, Some(0.01)), Estimate::Estimating);
        // 2 s in, 1% done: still too little
        assert_eq!(est.update(2.0, Some(0.01)), Estimate::Estimating);
    }

    #[test]
    fn a_steady_job_is_told_the_right_time() {
        let mut est = Estimator::new(None);
        // 1% a second: 100 s in all
        let e = run(&mut est, 0.1, 20.0, |t| t / 100.0);
        assert!((left(e) - 80.0).abs() < 4.0, "{e:?}");
        let e = run(&mut est, 20.1, 60.0, |t| t / 100.0);
        assert!((left(e) - 40.0).abs() < 3.0, "{e:?}");
    }

    #[test]
    fn a_slower_second_half_is_followed_without_a_leap() {
        // 2% a second to half way (25 s), then 0.5% a second: 125 s in all
        let pace = |t: f64| {
            if t <= 25.0 {
                t * 0.02
            } else {
                0.5 + (t - 25.0) * 0.005
            }
        };
        let mut est = Estimator::new(None);
        let mut shown = left(run(&mut est, 0.1, 25.0, pace));
        let mut t = 25.1;
        while t < 80.0 {
            let now = left(est.update(t, Some(pace(t))));
            // never more than a few seconds up in a tenth of a second
            assert!(now - shown < 3.0, "jumped from {shown} to {now} at {t}");
            shown = now;
            t += 0.1;
        }
        // by now it has caught up with the slower pace: 45 s really left
        assert!((shown - 45.0).abs() < 8.0, "{shown}");
    }

    #[test]
    fn a_noisy_job_does_not_swing_wildly() {
        // the share done comes in lumps: 5% every 5 s
        let lumpy = |t: f64| (t / 5.0).floor() * 0.05;
        let mut est = Estimator::new(None);
        run(&mut est, 0.1, 10.0, lumpy);
        let mut low = f64::INFINITY;
        let mut high: f64 = 0.0;
        let mut t = 10.1;
        while t < 50.0 {
            let e = est.update(t, Some(lumpy(t)));
            // compare with the true time left, 100 s in all
            let off = left(e) - (100.0 - t);
            low = low.min(off);
            high = high.max(off);
            t += 0.1;
        }
        assert!(high - low < 25.0, "swung between {low} and {high}");
    }

    #[test]
    fn the_time_shown_counts_down_with_the_clock() {
        let mut est = Estimator::new(None);
        let a = left(run(&mut est, 0.1, 30.0, |t| t / 100.0));
        // stuck for a second (one long step): still counting down, not up
        let b = left(est.update(31.0, Some(0.30)));
        assert!(b <= a, "{a} then {b}");
    }

    #[test]
    fn history_starts_the_guess_and_the_job_takes_over() {
        let mut est = Estimator::new(Some(60.0));
        // at once: last time's 60 s
        let first = left(est.update(0.5, Some(0.0)));
        assert!((first - 59.5).abs() < 0.1, "{first}");
        // this one is quicker, 1 s for 5%: 20 s in all
        let e = run(&mut est, 0.6, 6.0, |t| t * 0.05);
        assert!(left(e) < 30.0, "{e:?}");
        let e = run(&mut est, 6.1, 12.0, |t| t * 0.05);
        assert!((left(e) - 8.0).abs() < 3.0, "{e:?}");
    }

    #[test]
    fn a_job_that_says_nothing_is_told_its_usual_time() {
        let mut est = Estimator::new(Some(40.0));
        assert_eq!(est.update(3.0, None), Estimate::Usually(40.0));
        assert_eq!(est.update(70.0, None), Estimate::LongerThanUsual);
        assert_eq!(Estimator::new(None).update(3.0, None), Estimate::Unknown);
    }

    #[test]
    fn the_end_is_almost_done() {
        let mut est = Estimator::new(None);
        run(&mut est, 0.1, 9.9, |t| t / 10.0);
        assert_eq!(est.update(10.0, Some(1.0)), Estimate::AlmostDone);
    }

    #[test]
    fn history_blends_runs_and_survives_a_save() {
        let mut h = History::default();
        h.record("calculate:pocket", 10.0);
        assert_eq!(h.usual("calculate:pocket"), Some(10.0));
        h.record("calculate:pocket", 40.0);
        // the geometric middle: one big job does not throw it far
        assert!((h.usual("calculate:pocket").unwrap() - 20.0).abs() < 1e-9);
        h.record("calculate:pocket", f64::NAN);
        h.record("", 3.0);
        assert!((h.usual("calculate:pocket").unwrap() - 20.0).abs() < 1e-9);
        let dir = std::env::temp_dir().join(format!("cut-craft-eta-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("job-times.json");
        h.save(&path).unwrap();
        assert_eq!(History::load(&path), h);
        assert_eq!(History::load(&dir.join("missing.json")), History::default());
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn steps_weigh_what_their_kind_took_last_time() {
        let (slow, quick) = ("test:weights-slow", "test:weights-quick");
        record(slow, 30.0);
        record(quick, 10.0);
        let kinds = [Some(slow.to_owned()), Some(quick.to_owned()), None];
        assert_eq!(weights(&kinds), [30.0, 10.0, 20.0], "unknown: the average");
        assert_eq!(
            weights(&[None, Some("test:weights-never".to_owned())]),
            [1.0, 1.0]
        );
    }

    #[test]
    fn the_shared_history_is_kept_by_kind() {
        let kind = "test:the_shared_history_is_kept_by_kind";
        assert_eq!(usual(kind), None);
        record(kind, 12.0);
        assert_eq!(usual(kind), Some(12.0));
        assert_eq!(history().usual(kind), Some(12.0));
    }
}
