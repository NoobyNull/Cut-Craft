//! How the 3D view is moving: when it last turned, panned or zoomed, Auto-rotate's spin,
//! and the detail it draws at while it moves (Options ▸ Adjust detail to keep the view
//! smooth), judged once a frame from how quickly frames come.

use crate::frame_rate::Pacer;
use crate::options::Detail;

/// What the frame's detail is judged from.
pub(super) struct Frame {
    /// The egui pass (`egui::Context::cumulative_pass_nr`): judged once each.
    pub(super) pass: u64,
    /// The input clock now, seconds.
    pub(super) now: f64,
    /// How long the last frame took, seconds.
    pub(super) seconds: f64,
    /// Something else moves the picture: a preview playing, a handle dragged.
    pub(super) busy: bool,
    /// Seconds the view counts as moving after it last moved.
    pub(super) settle: f64,
    /// The detail when still, the finest it goes.
    pub(super) still: Detail,
    /// Frames a second wanted.
    pub(super) fps: f64,
}

pub(super) struct Motion3d {
    /// When the view last moved (input time, seconds).
    moved: f64,
    /// While Auto-rotate spins the view: when it last turned it (input time, seconds).
    spinning: Option<f64>,
    /// Picks the detail while it moves from how quickly frames come.
    pacer: Pacer,
    /// The detail drawn at this frame, and whether the view is moving.
    detail: (Detail, bool),
    /// The pass that was judged.
    judged: u64,
}

impl Default for Motion3d {
    fn default() -> Self {
        Self {
            moved: 0.0,
            spinning: None,
            pacer: Pacer::default(),
            detail: (Detail::default(), false),
            judged: u64::MAX,
        }
    }
}

impl Motion3d {
    /// The view turned, panned or zoomed at `now`.
    pub(super) fn moved(&mut self, now: f64) {
        self.moved = now;
    }

    /// Seconds since the view last moved.
    pub(super) fn since_moved(&self, now: f64) -> f64 {
        now - self.moved
    }

    /// Whether Auto-rotate is spinning the view.
    pub(super) fn spinning(&self) -> bool {
        self.spinning.is_some()
    }

    /// When Auto-rotate last turned the view, while it spins (kept and put back by the web
    /// API's pictures, which hold the view still).
    pub(super) fn spin(&self) -> Option<f64> {
        self.spinning
    }

    /// Auto-rotate spins from `since` (input time), or stops (`None`).
    pub(super) fn set_spin(&mut self, since: Option<f64>) {
        self.spinning = since;
    }

    /// Auto-rotate stops: a drag, the view cube or a 3D mouse took the view.
    pub(super) fn stop_spin(&mut self) {
        self.spinning = None;
    }

    /// While Auto-rotate spins, the seconds since it last turned the view, which it turns
    /// again now.
    pub(super) fn spun(&mut self, now: f64) -> Option<f64> {
        let last = self.spinning?;
        self.spinning = Some(now);
        Some(now - last)
    }

    /// Judges `frame`'s detail, once a pass: the view counts as moving for the settle time
    /// after it last moved, or while `busy`. Returns how long to wait before asking for a
    /// frame to draw finely, while it moves at less than the detail when still.
    pub(super) fn judge(&mut self, frame: &Frame) -> Option<f64> {
        if self.judged == frame.pass {
            return None;
        }
        self.judged = frame.pass;
        let since = frame.now - self.moved;
        let moving = since < frame.settle || frame.busy;
        let level = self
            .pacer
            .frame(frame.seconds, moving, frame.still, frame.fps);
        self.detail = (level, moving);
        (moving && level < frame.still).then(|| (frame.settle - since).max(0.0) + 0.01)
    }

    /// The detail drawn at this frame, and whether the view is moving.
    pub(super) fn detail(&self) -> (Detail, bool) {
        self.detail
    }

    /// Frames a second while the view moves; none while still or until enough are seen.
    pub(super) fn fps(&self) -> Option<f64> {
        self.pacer.fps().filter(|_| self.detail.1)
    }
}

#[cfg(test)]
mod tests;
