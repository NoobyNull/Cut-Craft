//! The machine's frame and the part's, which turns with A.

use cut_craft_core::glam::DVec3;

/// Where the machine holds the bit: `along` the axis, `across` it (from the axis), the
/// tip's `height` above the axis, and how far A has `turn`ed (radians). Turning A by a
/// positive amount brings the part's surface further round (larger angles from its `z` towards
/// its `y`) under the bit, as the unrolled sheet's round direction does.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Pose {
    pub along: f64,
    pub across: f64,
    pub height: f64,
    pub turn: f64,
}

impl Pose {
    #[must_use]
    pub fn new(along: f64, across: f64, height: f64, turn: f64) -> Self {
        Self {
            along,
            across,
            height,
            turn,
        }
    }

    /// Part of the way, `t` (0 to 1), to `to`: every axis moved in step, as a controller
    /// moves them in one straight move.
    #[must_use]
    pub fn lerp(self, to: Self, t: f64) -> Self {
        let mix = |a: f64, b: f64| a + (b - a) * t;
        Self {
            along: mix(self.along, to.along),
            across: mix(self.across, to.across),
            height: mix(self.height, to.height),
            turn: mix(self.turn, to.turn),
        }
    }

    /// The tip, in the part's frame.
    #[must_use]
    pub fn tip(self) -> DVec3 {
        to_part(DVec3::new(self.along, self.across, self.height), self.turn)
    }
}

/// A point of the part (`x` along, `y`, `z`) as it is in the machine's frame with A turned
/// `turn`: (along, across, height).
#[must_use]
pub fn to_machine(p: DVec3, turn: f64) -> DVec3 {
    let (s, c) = turn.sin_cos();
    DVec3::new(p.x, p.y * c - p.z * s, p.z * c + p.y * s)
}

/// What [`to_machine`] undoes: a point in the machine's frame (along, across, height) as it
/// is in the part's frame with A turned `turn`.
#[must_use]
pub fn to_part(m: DVec3, turn: f64) -> DVec3 {
    let (s, c) = turn.sin_cos();
    DVec3::new(m.x, m.y * c + m.z * s, m.z * c - m.y * s)
}

/// How far round the part `p` is: its angle from the part's `z` towards its `y`, radians in
/// (-π, π]. On the unrolled sheet it is `radius` times this round from A=0.
#[must_use]
pub fn angle(p: DVec3) -> f64 {
    p.y.atan2(p.z)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn turning_into_the_part_and_back_changes_nothing() {
        for (p, turn) in [
            (DVec3::new(1.0, 2.0, 3.0), 0.3),
            (DVec3::new(-5.0, -7.0, 0.5), -2.9),
            (DVec3::new(0.0, 0.0, 9.0), 7.0),
        ] {
            assert!(to_part(to_machine(p, turn), turn).distance(p) < 1e-12);
            assert!(to_machine(to_part(p, turn), turn).distance(p) < 1e-12);
        }
    }

    #[test]
    fn turning_a_brings_the_surface_further_round_under_the_bit() {
        // the point a quarter turn round from the top comes up under the bit at A = 90°
        let quarter = std::f64::consts::FRAC_PI_2;
        let p = DVec3::new(0.0, 10.0, 0.0);
        assert!((angle(p) - quarter).abs() < 1e-12);
        assert!(to_machine(p, quarter).distance(DVec3::new(0.0, 0.0, 10.0)) < 1e-12);
        // and the bit's tip over the axis at A = 90° is there
        let tip = Pose::new(0.0, 0.0, 10.0, quarter).tip();
        assert!(tip.distance(p) < 1e-12);
    }

    #[test]
    fn the_part_frame_matches_the_unrolled_sheets_off_axis_rule() {
        use cut_craft_core::rotary_axis::tip_of;
        // a bit held 4 across, 26 up, A turned to 30 mm round a 30 radius part
        let (radius, round, across, height) = (30.0, 30.0, 4.0, 26.0);
        let tip = Pose::new(0.0, across, height, round / radius).tip();
        let (w, z) = tip_of(radius, round, height - radius, across);
        assert!((radius * angle(tip) - w).abs() < 1e-9);
        assert!((tip.y.hypot(tip.z) - radius - z).abs() < 1e-9);
    }
}
