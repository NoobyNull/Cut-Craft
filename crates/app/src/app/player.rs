//! The preview player: a toolpath preview playing (or paused, or kept after it stopped)
//! into the material, a distance at a time. It knows where the tool is going and how fast,
//! how long a frame may spend cutting, and when the 3D view should show the cut again; the
//! app does what follows from it (the cut material becoming the preview, the view's
//! surfaces made again).

use super::{Material, view3d};
use cut_craft_core::playback::Playback;
use std::time::{Duration, Instant};

/// What a playing preview's GPU surface was made from: the playback's revision, the detail,
/// the job's revision, the wrap and whether the sides are drawn.
pub(super) type PlaybackGpuKey = (u64, usize, u64, Option<view3d::Wrap>, bool);
/// A frame's time for cutting a playing preview: the window still answers at once.
pub(super) const PLAYBACK_BUDGET: Duration = Duration::from_millis(30);
/// How often a playing preview's 3D view is made again from the cut so far: at full detail
/// that takes a while, so not every frame (the tool still moves every frame).
pub(super) const PLAYBACK_REDRAW: Duration = Duration::from_millis(100);

/// A frame's time for a jump (the Progress slider, Run to Retract): more than for playing,
/// so it gets there within a second or so; the window answers less often meanwhile.
pub(super) fn jump_budget(budget: Duration) -> Duration {
    budget * 4
}

pub(crate) struct Player {
    /// A preview playing (or paused): the tool cutting the material a distance at a time.
    pub(super) playback: Option<Playback>,
    pub(super) playing: bool,
    /// Where the tool is being moved to (Single Step, Run to Retract, the Progress slider),
    /// cut a little each frame until it gets there, or Stop.
    pub(super) goal: Option<f64>,
    /// A preview stopped or played to its end, kept so its controls stay to scrub it back
    /// and forth, till another preview, Reset Preview or another job.
    pub(super) kept: Option<Playback>,
    /// The job's history revision when the playing (or kept) preview was laid out: once
    /// the job changes, its moves are old, so it stops and its controls close.
    pub(super) revision: u64,
    /// A playing preview's material as the 3D view draws it, and the GPU's surface of it,
    /// kept by what they were made from (the playback's revision, the detail and the job's
    /// revision; for the GPU, its wrap and sides too) so a paused preview is not made again
    /// each frame, nor a moving one more than once a frame.
    pub(super) view: Option<((u64, usize, u64), Material)>,
    pub(super) gpu: Option<(PlaybackGpuKey, std::sync::Arc<view3d::gpu::Geometry>)>,
    /// How many times the playback's view has been made (for tests).
    pub(super) builds: usize,
    /// The playback's revision the 3D view shows, and when it was taken: while it plays
    /// it is taken again every [`Player::redraw`], and at once when it stops somewhere.
    pub(super) shown: Option<(u64, Instant)>,
    /// How often that is, [`PLAYBACK_REDRAW`] (longer in tests, whose frames are slow).
    pub(super) redraw: Duration,
    /// How long a frame may spend cutting a playing preview; the rest waits for the next
    /// frame, so a long toolpath plays slower instead of freezing the window.
    pub(super) budget: Duration,
    /// How many times faster than the machine the preview plays (1: the machine's speed).
    pub(super) speed: f64,
}

/// What a frame of the player asks of the window.
#[derive(Debug, Default, PartialEq)]
pub(super) struct Frame {
    /// The tool moved: draw again next frame.
    pub(super) repaint: bool,
    /// The 3D view is due to show the cut again after this long.
    pub(super) repaint_after: Option<Duration>,
    /// The preview has played to its end.
    pub(super) finished: bool,
}

impl Default for Player {
    fn default() -> Self {
        Self {
            playback: None,
            playing: false,
            goal: None,
            kept: None,
            revision: 0,
            view: None,
            gpu: None,
            builds: 0,
            shown: None,
            redraw: PLAYBACK_REDRAW,
            budget: PLAYBACK_BUDGET,
            speed: super::PLAYBACK_SPEED,
        }
    }
}

impl Player {
    /// `playback` starts playing, laid out at the job's `revision`; the one kept goes.
    pub(super) fn start(&mut self, playback: Playback, revision: u64) {
        (self.playback, self.kept, self.revision) = (Some(playback), None, revision);
        (self.playing, self.goal) = (true, None);
    }

    /// The playing preview, else the one kept after it stopped.
    pub(super) fn current(&self) -> Option<&Playback> {
        self.playback.as_ref().or(self.kept.as_ref())
    }

    /// The preview stops where it is (Stop, or the end of the path) and is kept, so its
    /// controls stay to scrub it again. What it has cut, if one was playing.
    pub(super) fn finish(&mut self) -> Option<Material> {
        (self.goal, self.shown, self.view, self.gpu) = (None, None, None, None);
        let playback = self.playback.take()?;
        let cut = playback.material().clone();
        (self.kept, self.playing) = (Some(playback), false);
        Some(cut)
    }

    /// The preview goes, playing or kept, and its controls close; how it plays (its speed,
    /// its budget) stays for the next.
    pub(super) fn close(&mut self) {
        (self.playback, self.kept, self.playing) = (None, None, false);
        (self.goal, self.shown, self.view, self.gpu) = (None, None, None, None);
    }

    /// The kept preview taken up again, to play or scrub it. Whether there was one.
    pub(super) fn resume(&mut self) -> bool {
        let Some(kept) = self.kept.take() else {
            return false;
        };
        self.playback = Some(kept);
        true
    }

    /// Once the job is at another `revision` than the preview was laid out at, its moves
    /// are old: a preview there (playing or kept) is stale.
    pub(super) fn stale(&self, revision: u64) -> bool {
        revision != self.revision && (self.playback.is_some() || self.kept.is_some())
    }

    /// A frame `dt` seconds after the last, starting `now`: the playing preview's tool moved
    /// on at its speed, or toward its goal, cutting for no longer than the frame's budget,
    /// and whether the 3D view shows the cut again yet.
    pub(super) fn advance(&mut self, dt: f64, now: Instant) -> Frame {
        let mut frame = Frame::default();
        let Some(playback) = self.playback.as_mut() else {
            self.goal = None;
            return frame;
        };
        // a slow frame does not send the tool further on the next
        let dt = dt.min(0.1);
        let goal = (self.goal).or_else(|| self.playing.then(|| playback.at() + self.speed * dt));
        if let Some(goal) = goal {
            let budget = if self.goal.is_some() {
                jump_budget(self.budget)
            } else {
                self.budget
            };
            let until = now + budget;
            if playback.seek_until(goal, || Instant::now() >= until) {
                self.goal = None;
            }
            frame.repaint = true;
        }
        // the view shows where it has stopped at once; on the way, every so often
        let (revision, now) = (playback.revision(), Instant::now());
        let settled = self.goal.is_none() && !self.playing;
        match self.shown {
            Some((shown, _)) if shown == revision => {}
            Some((_, when)) if !settled && now < when + self.redraw => {
                frame.repaint_after = Some(when + self.redraw - now);
            }
            _ => self.shown = Some((revision, now)),
        }
        frame.finished = playback.finished();
        frame
    }
}

#[cfg(test)]
mod tests;
