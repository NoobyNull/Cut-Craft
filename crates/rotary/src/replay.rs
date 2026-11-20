//! A rotary file played back into the material, move by move, as the machine will run it:
//! the material turning with A and the bit moving along, across and up and down, each move
//! taking as long as the file's feed says. Going back starts again from a saved copy of the
//! material, so moving the position back and forth always shows what the file has cut by
//! then, and a move cut a frame at a time cuts just what it cuts all at once.

use crate::cutter::Cutter;
use crate::frame::Pose;
use crate::gcode::Motion;
use crate::voxels::Voxels;

/// How fast the machine moves where the file does not say: its rapids, along a straight
/// line (mm/min) and turning A (degrees/min).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Speeds {
    pub rapid: f64,
    pub turn: f64,
}

/// One straight move of the file.
#[derive(Clone, Copy, Debug)]
struct Step {
    cutter: usize,
    from: Pose,
    to: Pose,
}

/// The most copies of the material kept along the way, so going back starts near where it
/// is going.
const MOST_SAVES: usize = 40;

static REVISIONS: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);

fn next_revision() -> u64 {
    REVISIONS.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
}

pub struct Replay {
    cutters: Vec<Cutter>,
    steps: Vec<Step>,
    /// The machine's time (seconds) at the end of each step.
    ends: Vec<f64>,
    start: Voxels,
    material: Voxels,
    /// The steps cut whole, and how many of the next one's samples ([`Voxels::samples`]).
    done: usize,
    sampled: usize,
    at: f64,
    /// The material after so many whole steps, in order.
    saves: Vec<(usize, Voxels)>,
    save_every: f64,
    revision: u64,
}

impl Replay {
    /// The moves of each part of a file (`parts`, each cut by its own bit, in order) played
    /// into `start`. A move with no feed of its own goes at `speeds`.
    #[must_use]
    pub fn new(start: Voxels, parts: Vec<(Cutter, Vec<Motion>)>, speeds: Speeds) -> Self {
        let (mut cutters, mut steps, mut ends) = (vec![], vec![], vec![]);
        let mut time = 0.0;
        for (cutter, moves) in parts {
            cutters.push(cutter);
            for pair in moves.windows(2) {
                let (from, to) = (pair[0].to, pair[1].to);
                let minutes = pair[1].minutes.unwrap_or_else(|| {
                    let line = (to.along - from.along)
                        .hypot(to.across - from.across)
                        .hypot(to.height - from.height);
                    let degrees = (to.turn - from.turn).abs().to_degrees();
                    (line / speeds.rapid.max(1e-9)).max(degrees / speeds.turn.max(1e-9))
                });
                time += minutes * 60.0;
                steps.push(Step {
                    cutter: cutters.len() - 1,
                    from,
                    to,
                });
                ends.push(time);
            }
        }
        #[expect(clippy::cast_precision_loss, reason = "a handful of saves")]
        let save_every = time / (MOST_SAVES + 1) as f64;
        Self {
            cutters,
            steps,
            ends,
            material: start.clone(),
            start,
            done: 0,
            sampled: 0,
            at: 0.0,
            saves: vec![],
            save_every,
            revision: next_revision(),
        }
    }

    /// How long the whole file takes, seconds.
    #[must_use]
    pub fn length(&self) -> f64 {
        self.ends.last().copied().unwrap_or(0.0)
    }

    /// Where it has got to, seconds.
    #[must_use]
    pub fn at(&self) -> f64 {
        self.at
    }

    #[must_use]
    pub fn finished(&self) -> bool {
        self.done >= self.steps.len()
    }

    /// The material as cut so far, in the part's frame.
    #[must_use]
    pub fn material(&self) -> &Voxels {
        &self.material
    }

    /// Changes whenever the material does.
    #[must_use]
    pub fn revision(&self) -> u64 {
        self.revision
    }

    /// Where the bit is now and which it is; `None` for a file with no moves.
    #[must_use]
    pub fn bit(&self) -> Option<(Pose, &Cutter)> {
        let last = self.steps.last()?;
        let pose = match self.steps.get(self.done) {
            Some(step) => step.from.lerp(step.to, self.fraction(self.done, self.at)),
            None => last.to,
        };
        let step = self.steps.get(self.done).unwrap_or(last);
        Some((pose, &self.cutters[step.cutter]))
    }

    /// How far through step `i` time `t` is, 0 to 1.
    fn fraction(&self, i: usize, t: f64) -> f64 {
        let begin = if i == 0 { 0.0 } else { self.ends[i - 1] };
        let long = self.ends[i] - begin;
        if long <= 0.0 {
            1.0
        } else {
            ((t - begin) / long).clamp(0.0, 1.0)
        }
    }

    /// Moves to `t` seconds, cutting what the file cuts by then.
    pub fn seek(&mut self, t: f64) {
        self.seek_until(t, || false);
    }

    /// Moves toward `goal` seconds, a whole move at a time, until it gets there or
    /// `out_of_time` says to stop (asked after each move, so it always gets somewhere).
    /// Whether it got there.
    pub fn seek_until(&mut self, goal: f64, mut out_of_time: impl FnMut() -> bool) -> bool {
        let goal = goal.clamp(0.0, self.length());
        if goal < self.at {
            self.go_back_to(goal);
        }
        let before = (self.done, self.sampled);
        while self.done < self.steps.len() && self.ends[self.done] <= goal {
            let step = self.steps[self.done];
            let cutter = &self.cutters[step.cutter];
            let all = self.material.samples(step.from, step.to);
            if self.sampled <= all {
                let which = self.sampled..=all;
                self.material.cut_samples(cutter, step.from, step.to, which);
            }
            (self.done, self.sampled) = (self.done + 1, 0);
            self.at = self.ends[self.done - 1];
            self.save();
            if out_of_time() {
                self.changed(before);
                return self.at >= goal;
            }
        }
        // part way through the next move: its samples up to where the bit is
        if let Some(step) = self.steps.get(self.done).copied() {
            let all = self.material.samples(step.from, step.to);
            #[expect(
                clippy::cast_possible_truncation,
                clippy::cast_sign_loss,
                clippy::cast_precision_loss,
                reason = "a sample count"
            )]
            let upto = (self.fraction(self.done, goal) * all as f64).floor() as usize;
            if upto >= self.sampled {
                let cutter = &self.cutters[step.cutter];
                let which = self.sampled..=upto;
                self.material.cut_samples(cutter, step.from, step.to, which);
                self.sampled = upto + 1;
            }
        }
        self.at = goal;
        self.changed(before);
        true
    }

    fn changed(&mut self, before: (usize, usize)) {
        if (self.done, self.sampled) != before {
            self.revision = next_revision();
        }
    }

    /// Back to the last saved material at or before `t`, or the start.
    fn go_back_to(&mut self, t: f64) {
        let kept = self
            .saves
            .iter()
            .rev()
            .find(|(done, _)| self.ends[done - 1] <= t);
        (self.material, self.done) = match kept {
            Some((done, material)) => (material.clone(), *done),
            None => (self.start.clone(), 0),
        };
        self.sampled = 0;
        self.at = if self.done == 0 {
            0.0
        } else {
            self.ends[self.done - 1]
        };
        self.revision = next_revision();
    }

    /// Keeps a copy of the material every so often on the way forward.
    #[expect(clippy::cast_precision_loss, reason = "a handful of saves")]
    fn save(&mut self) {
        let last = self.saves.last().map_or(0, |(done, _)| *done);
        if self.done > last
            && self.saves.len() < MOST_SAVES
            && self.at >= (self.saves.len() + 1) as f64 * self.save_every
        {
            self.saves.push((self.done, self.material.clone()));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::stock::{Section, Stock};
    use cut_craft_core::toolpath::{Tool, ToolShape};

    fn cutter(diameter: f64) -> Cutter {
        Cutter::new(&Tool {
            shape: ToolShape::BallNose,
            diameter,
            flute_length: 30.0,
            ..Tool::default()
        })
    }

    fn at(along: f64, height: f64, degrees: f64, minutes: Option<f64>) -> Motion {
        Motion {
            rapid: minutes.is_none(),
            to: Pose::new(along, 0.0, height, degrees.to_radians()),
            minutes,
        }
    }

    /// A rough pass winding round and along, then a finish with a smaller bit.
    fn file() -> Vec<(Cutter, Vec<Motion>)> {
        let mut rough = vec![at(0.0, 30.0, 0.0, None), at(0.0, 12.0, 0.0, Some(0.05))];
        for i in 1..=12 {
            let i = f64::from(i);
            rough.push(at(i * 4.0, 12.0, i * 120.0, Some(0.1)));
        }
        rough.push(at(48.0, 30.0, 1440.0, None));
        let finish = vec![
            at(5.0, 30.0, 0.0, None),
            at(5.0, 9.0, 0.0, Some(0.02)),
            at(45.0, 9.0, 720.0, Some(0.5)),
            at(45.0, 30.0, 720.0, None),
        ];
        vec![(cutter(6.0), rough), (cutter(3.0), finish)]
    }

    fn start() -> Voxels {
        Voxels::of(&Stock::new(Section::Round { diameter: 30.0 }, 50.0), 0.8)
    }

    const SPEEDS: Speeds = Speeds {
        rapid: 3000.0,
        turn: 3600.0,
    };

    fn straight_through() -> Voxels {
        let mut material = start();
        for (cutter, moves) in file() {
            for pair in moves.windows(2) {
                material.cut(&cutter, pair[0].to, pair[1].to);
            }
        }
        material
    }

    #[test]
    fn played_through_it_cuts_just_what_each_move_cut_in_turn_does() {
        let mut replay = Replay::new(start(), file(), SPEEDS);
        assert!(!replay.finished());
        replay.seek(replay.length());
        assert!(replay.finished());
        assert_eq!(replay.material(), &straight_through());
        assert!(replay.material().count() < start().count());
    }

    #[test]
    fn played_a_frame_at_a_time_it_cuts_exactly_what_it_cuts_all_at_once() {
        let mut replay = Replay::new(start(), file(), SPEEDS);
        let mut t = 0.0;
        while !replay.finished() {
            t += 0.37;
            replay.seek(t);
        }
        assert_eq!(replay.material(), &straight_through());
    }

    #[test]
    fn going_back_shows_what_was_cut_by_then_and_going_on_again_ends_the_same() {
        let mut replay = Replay::new(start(), file(), SPEEDS);
        let half = replay.length() / 2.0;
        replay.seek(half);
        let at_half = replay.material().clone();
        replay.seek(replay.length());
        replay.seek(half);
        assert_eq!(replay.material(), &at_half);
        replay.seek(0.0);
        assert_eq!(replay.material(), &start());
        replay.seek(replay.length() * 0.8);
        replay.seek(replay.length() * 0.3);
        replay.seek(replay.length());
        assert_eq!(replay.material(), &straight_through());
    }

    #[test]
    fn each_move_takes_as_long_as_the_file_says_and_rapids_as_the_machine_goes() {
        let replay = Replay::new(start(), file(), SPEEDS);
        // the rough pass: 0.05 + 12 × 0.1 minutes cutting, then a rapid up 18 mm (at 3000
        // mm/min) turning 0° (it is already at 1440°); the finish: a rapid of 5 along and
        // turning back 1440° (at 3600°/min), 0.02 + 0.5 cutting, and 21 mm up
        let rough = 0.05 + 1.2 + 18.0 / 3000.0;
        let finish = 0.02 + 0.5 + 21.0 / 3000.0;
        let want = (rough + finish) * 60.0;
        assert!((replay.length() - want).abs() < 1e-9, "{}", replay.length());
    }

    #[test]
    fn the_bit_is_where_the_file_has_it_and_changes_with_the_tool() {
        let mut replay = Replay::new(start(), file(), SPEEDS);
        // half way through the first cut along and round: from 0 along at 0°, to 4 along at
        // 120°, after the 3 s plunge
        replay.seek(3.0 + 3.0);
        let (pose, bit) = replay.bit().unwrap();
        assert!((pose.along - 2.0).abs() < 1e-9 && (pose.turn - 60f64.to_radians()).abs() < 1e-9);
        assert!((bit.tool().diameter - 6.0).abs() < 1e-12);
        replay.seek(replay.length());
        let (pose, bit) = replay.bit().unwrap();
        assert!((bit.tool().diameter - 3.0).abs() < 1e-12);
        assert_eq!(pose, Pose::new(45.0, 0.0, 30.0, 720f64.to_radians()));
    }

    #[test]
    fn short_of_time_it_still_gets_on_and_says_it_has_not_got_there() {
        let mut replay = Replay::new(start(), file(), SPEEDS);
        let revision = replay.revision();
        assert!(!replay.seek_until(replay.length(), || true));
        assert!(replay.at() > 0.0 && !replay.finished());
        assert_ne!(replay.revision(), revision);
        while !replay.seek_until(replay.length(), || true) {}
        assert_eq!(replay.material(), &straight_through());
    }

    #[test]
    fn a_file_with_no_moves_has_nothing_to_play() {
        let mut replay = Replay::new(start(), vec![(cutter(6.0), vec![])], SPEEDS);
        assert_eq!(replay.length(), 0.0);
        assert!(replay.bit().is_none() && replay.finished());
        replay.seek(5.0);
        assert_eq!(replay.material(), &start());
    }
}
