//! Detail that follows the frame rate: while the 3D view moves, it watches how long each
//! frame takes and draws the model coarser when frames come too slowly, finer again when
//! they come quickly, so turning the view stays smooth on a slow computer and sharp on a
//! fast one (not one of Aspire's).
//!
//! It is a pure function of the frame times it is given ([`Pacer::frame`]), so its tests
//! feed it made-up timings.

use crate::options::Detail;
use std::collections::VecDeque;

/// Frames judged together: their middle time (the median) is the view's pace, so one slow
/// frame, a pause or a hiccup, is not enough to change the detail.
const WINDOW: usize = 5;
/// Slower than the target by this much, and the detail drops.
const SLOW: f64 = 1.25;
/// Slower than the target by this much, and it drops two steps at once.
const VERY_SLOW: f64 = 2.5;
/// Quicker than the target by this much (a share of a frame's time), and it may rise.
const QUICK: f64 = 0.8;
/// Seconds of quick frames before the detail rises a step, at first.
const RISE_AFTER: f64 = 1.0;
/// The longest wait before trying a rise again, after rises kept proving too slow.
const RISE_AFTER_MOST: f64 = 16.0;
/// A rise that is too slow within this many seconds counts as a failed one.
const PROVE: f64 = 2.0;
/// Seconds after a change before frames count again: the first frames at a new detail
/// include making it.
const HOLD: f64 = 0.25;
/// A frame longer than this is a pause (nothing to draw, the window hidden, a held mouse),
/// not a slow frame.
const GAP: f64 = 1.0;

/// Chooses the detail the 3D view moves at, from the frame times it is fed.
#[derive(Clone, Debug)]
pub struct Pacer {
    /// The detail it moves at now: kept between motions, so the next starts where this
    /// computer last kept up.
    level: Detail,
    /// The last frames' times since the detail last changed, newest last.
    recent: VecDeque<f64>,
    /// The last frames' times whatever the detail, newest last: for the frames a second shown.
    shown: VecDeque<f64>,
    /// Seconds before frames count again, after a change.
    hold: f64,
    /// Seconds of quick frames in a row.
    quick_for: f64,
    /// Seconds of quick frames a rise waits for: doubled each time a rise proves too slow.
    rise_after: f64,
    /// Seconds since the last rise, while it may still prove too slow.
    since_rise: Option<f64>,
}

impl Default for Pacer {
    fn default() -> Self {
        Self {
            level: Detail::VeryHigh,
            recent: VecDeque::with_capacity(WINDOW),
            shown: VecDeque::with_capacity(WINDOW),
            hold: 0.0,
            quick_for: 0.0,
            rise_after: RISE_AFTER,
            since_rise: None,
        }
    }
}

impl Pacer {
    /// The detail to draw a frame at, given how long the last frame took (`seconds`),
    /// whether the view is moving, the detail when still (the finest it goes) and the frames
    /// a second wanted. Once still it is the detail when still.
    pub fn frame(&mut self, seconds: f64, moving: bool, still: Detail, target_fps: f64) -> Detail {
        self.level = self.level.min(still);
        if !moving {
            // a new motion is judged afresh, from the detail it ended at
            self.recent.clear();
            self.shown.clear();
            self.quick_for = 0.0;
            self.hold = 0.0;
            return still;
        }
        if !(seconds > 0.0 && seconds < GAP) {
            return self.level;
        }
        if let Some(since) = &mut self.since_rise {
            *since += seconds;
            if *since > PROVE {
                // the rise held: the next waits the usual time
                self.since_rise = None;
                self.rise_after = RISE_AFTER;
            }
        }
        if self.shown.len() == WINDOW {
            self.shown.pop_front();
        }
        self.shown.push_back(seconds);
        if self.hold > 0.0 {
            self.hold -= seconds;
            return self.level;
        }
        if self.recent.len() == WINDOW {
            self.recent.pop_front();
        }
        self.recent.push_back(seconds);
        let Some(pace) = middle(&self.recent) else {
            return self.level;
        };
        let budget = 1.0 / target_fps.max(1.0);
        if pace > budget * SLOW {
            let steps = if pace > budget * VERY_SLOW { 2 } else { 1 };
            let lower = self.level.lower(steps);
            if lower != self.level {
                if self.since_rise.take().is_some() {
                    // it rose too far: wait longer before trying again
                    self.rise_after = (self.rise_after * 2.0).min(RISE_AFTER_MOST);
                }
                self.change(lower);
            }
            self.quick_for = 0.0;
        } else if pace < budget * QUICK {
            self.quick_for += seconds;
            if self.quick_for >= self.rise_after && self.level < still {
                self.change(self.level.higher(1));
                self.since_rise = Some(0.0);
            }
        } else {
            self.quick_for = 0.0;
        }
        self.level
    }

    fn change(&mut self, to: Detail) {
        self.level = to;
        self.recent.clear();
        self.quick_for = 0.0;
        self.hold = HOLD;
    }

    /// Frames a second while moving, from the last frames' middle time; none until enough
    /// frames have been seen.
    pub fn fps(&self) -> Option<f64> {
        middle(&self.shown).map(|seconds| 1.0 / seconds)
    }

    /// The detail it moves at now.
    #[cfg(test)]
    pub fn level(&self) -> Detail {
        self.level
    }
}

/// The middle of the frames' times, once there are enough of them.
fn middle(frames: &VecDeque<f64>) -> Option<f64> {
    if frames.len() < WINDOW {
        return None;
    }
    let mut times: Vec<f64> = frames.iter().copied().collect();
    times.sort_by(f64::total_cmp);
    Some(times[WINDOW / 2])
}

#[cfg(test)]
mod tests {
    use super::*;

    const AT_30: f64 = 30.0;

    /// Feeds `count` frames of `seconds` each, moving; the detail after the last.
    fn run(p: &mut Pacer, count: usize, seconds: f64, still: Detail) -> Detail {
        let mut level = p.level();
        for _ in 0..count {
            level = p.frame(seconds, true, still, AT_30);
        }
        level
    }

    #[test]
    fn quick_frames_keep_the_detail_when_still() {
        let mut p = Pacer::default();
        assert_eq!(
            run(&mut p, 200, 1.0 / 60.0, Detail::VeryHigh),
            Detail::VeryHigh
        );
    }

    #[test]
    fn slow_frames_drop_a_step_within_a_few_frames() {
        let mut p = Pacer::default();
        // 40 ms a frame: 25 a second against 30 wanted
        let level = run(&mut p, WINDOW, 0.045, Detail::VeryHigh);
        assert_eq!(level, Detail::High, "dropped one step");
    }

    #[test]
    fn very_slow_frames_drop_two_steps_at_once() {
        let mut p = Pacer::default();
        let level = run(&mut p, WINDOW, 0.2, Detail::VeryHigh);
        assert_eq!(level, Detail::Medium);
    }

    #[test]
    fn it_settles_on_the_finest_detail_that_keeps_up() {
        let mut p = Pacer::default();
        // what each detail takes a frame on this made-up computer: Medium keeps up with 30
        // a second, High does not
        let took = |d: Detail| match d {
            Detail::VeryHigh => 0.07,
            Detail::High => 0.045,
            Detail::Medium => 0.02,
            Detail::Low => 0.01,
        };
        let mut at_medium = 0;
        for _ in 0..(60 * 30) {
            if p.frame(took(p.level()), true, Detail::VeryHigh, AT_30) == Detail::Medium {
                at_medium += 1;
            }
        }
        // the rest are the first few frames and the ever rarer tries at High
        assert!(at_medium > 60 * 30 * 9 / 10, "{at_medium} frames at Medium");
        assert_ne!(p.level(), Detail::Low, "never needed Low");
    }

    #[test]
    fn one_slow_frame_among_quick_ones_changes_nothing() {
        let mut p = Pacer::default();
        for i in 0..100 {
            let seconds = if i % 10 == 0 { 0.5 } else { 1.0 / 60.0 };
            assert_eq!(
                p.frame(seconds, true, Detail::VeryHigh, AT_30),
                Detail::VeryHigh
            );
        }
    }

    #[test]
    fn pauses_are_not_slow_frames() {
        let mut p = Pacer::default();
        // a mouse held still mid-drag: no frames for seconds at a time
        assert_eq!(run(&mut p, 20, 3.0, Detail::VeryHigh), Detail::VeryHigh);
    }

    #[test]
    fn it_never_goes_below_low() {
        let mut p = Pacer::default();
        assert_eq!(run(&mut p, 500, 0.5, Detail::VeryHigh), Detail::Low);
    }

    #[test]
    fn it_rises_again_only_after_a_second_of_quick_frames() {
        let mut p = Pacer::default();
        run(&mut p, WINDOW, 0.045, Detail::VeryHigh);
        assert_eq!(p.level(), Detail::High);
        // half a second of quick frames is not enough
        assert_eq!(run(&mut p, 30, 1.0 / 60.0, Detail::VeryHigh), Detail::High);
        assert_eq!(
            run(&mut p, 60, 1.0 / 60.0, Detail::VeryHigh),
            Detail::VeryHigh
        );
    }

    #[test]
    fn frames_near_the_target_hold_the_detail() {
        // between quick and slow (here 30 a second against 30 wanted) nothing moves either
        // way: the gap is what keeps it from flickering
        let mut p = Pacer::default();
        run(&mut p, WINDOW, 0.045, Detail::VeryHigh);
        assert_eq!(run(&mut p, 600, 1.0 / 30.0, Detail::VeryHigh), Detail::High);
    }

    #[test]
    fn a_rise_that_proves_too_slow_waits_longer_next_time() {
        // High keeps 60 a second, Very High only 20: it tries Very High, finds it too
        // slow, and tries again ever more rarely
        let mut p = Pacer::default();
        let mut changes = 0;
        let mut last = p.level();
        for _ in 0..(60 * 60) {
            let seconds = if p.level() == Detail::VeryHigh {
                0.05
            } else {
                1.0 / 60.0
            };
            let level = p.frame(seconds, true, Detail::VeryHigh, AT_30);
            if level != last {
                changes += 1;
                last = level;
            }
        }
        // a minute of turning: without backing off that would be ~50 changes
        assert!(changes <= 16, "{changes} changes in a minute");
        assert_eq!(p.rise_after, RISE_AFTER_MOST);
    }

    #[test]
    fn a_rise_that_holds_resets_the_wait() {
        let mut p = Pacer::default();
        run(&mut p, WINDOW, 0.045, Detail::VeryHigh);
        p.rise_after = 8.0;
        // eight seconds of quick frames, then quick ones at the new level too
        run(&mut p, 60 * 9, 1.0 / 60.0, Detail::VeryHigh);
        assert_eq!(p.level(), Detail::VeryHigh);
        run(&mut p, 60 * 3, 1.0 / 60.0, Detail::VeryHigh);
        assert!((p.rise_after - RISE_AFTER).abs() < 1e-9);
    }

    #[test]
    fn still_draws_at_the_detail_when_still_and_remembers_the_moving_one() {
        let mut p = Pacer::default();
        run(&mut p, WINDOW, 0.045, Detail::VeryHigh);
        assert_eq!(
            p.frame(1.0 / 60.0, false, Detail::VeryHigh, AT_30),
            Detail::VeryHigh
        );
        // the next motion starts where this computer last kept up
        assert_eq!(
            p.frame(1.0 / 60.0, true, Detail::VeryHigh, AT_30),
            Detail::High
        );
    }

    #[test]
    fn it_never_goes_finer_than_the_detail_when_still() {
        let mut p = Pacer::default();
        assert_eq!(
            run(&mut p, 600, 1.0 / 120.0, Detail::Medium),
            Detail::Medium
        );
        assert_eq!(p.level(), Detail::Medium);
    }

    #[test]
    fn a_higher_target_drops_sooner() {
        // 25 ms a frame keeps up with 30 a second but not with 60
        let mut p = Pacer::default();
        for _ in 0..WINDOW {
            p.frame(0.025, true, Detail::VeryHigh, 60.0);
        }
        assert_eq!(p.level(), Detail::High);
        let mut q = Pacer::default();
        assert_eq!(run(&mut q, 50, 0.025, Detail::VeryHigh), Detail::VeryHigh);
    }

    #[test]
    fn it_reports_frames_a_second_once_it_has_seen_enough() {
        let mut p = Pacer::default();
        assert_eq!(p.fps(), None);
        run(&mut p, WINDOW, 0.02, Detail::VeryHigh);
        assert!((p.fps().expect("enough frames") - 50.0).abs() < 1e-6);
    }
}
