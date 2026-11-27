//! The view cube's steps (quarter turns about the line of sight, nudges to the next face)
//! and turning smoothly from one view to another.

use super::{DVec3, Orbit, Rotation};
use crate::view3d::cube::{FACES, Nudge};
use cut_craft_core::glam::{DMat3, DQuat};

/// How near (the cosine of the angle) the line of sight must be to a face's to count as
/// looking straight at it.
const HEAD_ON: f64 = 1.0 - 1e-9;

/// How long a smooth turn to a new view takes, seconds: quick enough not to wait for, slow
/// enough to see which way the job turned.
pub const TURN_SECONDS: f64 = 0.25;

impl Orbit {
    /// Whether the view looks straight at one of the job's faces (Top, Front, ...).
    pub fn head_on(self) -> bool {
        self.towards_viewer().abs().max_element() >= HEAD_ON
    }

    /// Turned a quarter turn about the line of sight, clockwise on the screen or anticlockwise.
    #[must_use]
    pub fn quarter_turned(self, clockwise: bool) -> Self {
        let quarter = std::f64::consts::FRAC_PI_2;
        self.turned(DVec3::Z * if clockwise { -quarter } else { quarter })
            .squared()
    }

    /// Turned the short way so the face beyond `side` of the screen (the one
    /// [`Orbit::face_beyond`] names) faces the viewer: a quarter turn from a face straight on.
    #[must_use]
    pub fn nudged(self, side: Nudge) -> Self {
        let Some(face) = self.face_toward(side) else {
            return self;
        };
        let m = self.matrix();
        let arc = DQuat::from_rotation_arc((m * face.0).normalize(), DVec3::Z);
        Self::from_matrix(DMat3::from_quat(arc) * m).squared()
    }

    /// The name key of the face beyond `side` of the screen, the one its nudge brings round.
    pub fn face_beyond(self, side: Nudge) -> &'static str {
        self.face_toward(side).map_or("", |f| f.3)
    }

    /// The face whose outward direction is nearest the screen's `side`.
    fn face_toward(self, side: Nudge) -> Option<&'static (DVec3, DVec3, DVec3, &'static str)> {
        let out = self.to_world(side.screen());
        FACES
            .iter()
            .max_by(|a, b| a.0.dot(out).total_cmp(&b.0.dot(out)))
    }

    /// Near a view square to the job (within a hair), exactly square: a step from the view
    /// cube ends on a face straight on, so the next step is offered too.
    #[must_use]
    fn squared(self) -> Self {
        let m = self.matrix();
        let snap = |v: DVec3| {
            let r = v.round();
            if (v - r).abs().max_element() < 1e-6 {
                r
            } else {
                v
            }
        };
        let m = DMat3::from_cols(snap(m.x_axis), snap(m.y_axis), snap(m.z_axis));
        Self::from_matrix(m)
    }

    /// The view `t` of the way (0 to 1) from this one to `to`, turning the short way.
    #[must_use]
    pub fn towards(self, to: Self, t: f64) -> Self {
        let (a, b) = (
            DQuat::from_mat3(&self.matrix()),
            DQuat::from_mat3(&to.matrix()),
        );
        Self::from_matrix(DMat3::from_quat(a.slerp(b, t.clamp(0.0, 1.0))))
    }

    /// Upright as `rotation` keeps it: a turntable keeps the job's up up, so a view the cube
    /// turned on its side is turned back first.
    #[must_use]
    pub fn upright(self, rotation: Rotation) -> Self {
        match rotation {
            Rotation::Turntable => Self { roll: 0.0, ..self },
            Rotation::Free => self,
        }
    }
}

/// A smooth turn from one view to another under way.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Turning {
    from: Orbit,
    /// Where it ends; a drag while it turns moves this.
    pub to: Orbit,
    /// When it began (input time, seconds).
    start: f64,
}

impl Turning {
    pub fn new(from: Orbit, to: Orbit, now: f64) -> Self {
        Self {
            from,
            to,
            start: now,
        }
    }

    /// The view at `now`, eased in and out, and whether the turn is over.
    pub fn at(&self, now: f64) -> (Orbit, bool) {
        let t = ((now - self.start) / TURN_SECONDS).clamp(0.0, 1.0);
        if t >= 1.0 {
            return (self.to, true);
        }
        let eased = t * t * (3.0 - 2.0 * t);
        (self.from.towards(self.to, eased), false)
    }
}
