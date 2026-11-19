//! A machine's rotary (4th axis, A): where it lies on the bed, which way it turns, how fast,
//! and how much room its chuck and tailstock take. Synchronized XYZA cutting needs it: once Y
//! moves while A turns, the toolpaths must know exactly where the turning axis is.

use crate::document::Along;
use serde::{Deserialize, Serialize};

/// A machine's rotary, as set in Machine Configuration.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct RotaryAxis {
    /// Which way the rotary lies on the bed: along X (most hobby kits) or along Y.
    pub along: Along,
    /// The centre line has been found on the machine (step by step in the help) and the
    /// offsets below measured; until then the checks before cutting ask about it.
    pub centre_set: bool,
    /// Across the bed from work zero to the turning axis, mm: Y for a rotary along X, X for
    /// one along Y. 0 when work zero is set on the axis, as is usual. (Its height is the
    /// job's: Job Setup's Z zero, on the axis or on the cylinder's top.)
    pub centre_across: f64,
    /// +A turns clockwise, seen from the chuck end.
    pub clockwise: bool,
    /// The fastest A turns, degrees a minute: for cutting times, and to keep inverse-time
    /// feeds within it.
    pub max_rate: f64,
    /// How far the chuck reaches along the axis from the job's start, mm: kept out of.
    pub chuck: f64,
    /// How far the tailstock reaches back along the axis from the job's end, mm.
    pub tailstock: f64,
    /// Whether A may keep turning one way for as long as the job likes.
    pub turning: Turning,
    /// With [`Turning::Unwind`]: how many whole turns A may go either way from where it
    /// started (a cable or hose on the chuck, or a limit switch, decides it).
    pub turns: f64,
    /// With [`Turning::Unwind`]: what the file does at the limit.
    pub at_limit: AtLimit,
    /// How far A turns before it moves the material when it changes direction, degrees: the
    /// file turns that much further on each change of direction. 0 for none.
    pub backlash: f64,
}

/// How far A may turn one way.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Turning {
    /// As far as it likes, like a lathe: A never turns back unless the job does.
    #[default]
    Free,
    /// No more than a set number of whole turns either way from where it started.
    Unwind,
}

/// What a file does when the next pass would take A past its turns.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AtLimit {
    /// Cut the next pass the other way round, so A comes back as it cuts: no turning spent
    /// on spinning back. A pass that can't be cut backwards (one that ramps in) spins back.
    #[default]
    CutBothWays,
    /// Lift the bit clear and spin A back whole turns, nearer where it started. Spinning
    /// back to the start also shows up any steps A has lost.
    SpinBack,
}

impl Default for RotaryAxis {
    fn default() -> Self {
        Self {
            along: Along::X,
            centre_set: false,
            centre_across: 0.0,
            clockwise: true,
            max_rate: 3600.0,
            chuck: 40.0,
            tailstock: 40.0,
            turning: Turning::Free,
            turns: 2.0,
            at_limit: AtLimit::CutBothWays,
            backlash: 0.0,
        }
    }
}

impl RotaryAxis {
    /// How far A may turn either way from where it started, degrees; `None` when it is free.
    #[must_use]
    pub fn limit(&self) -> Option<f64> {
        (self.turning == Turning::Unwind).then(|| self.turns.max(0.5) * 360.0)
    }
}

impl Along {
    /// Its name in a sentence: "X" or "Y".
    #[must_use]
    pub fn letter(self) -> &'static str {
        match self {
            Self::X => "X",
            Self::Y => "Y",
        }
    }
}

/// Where the tip of a bit held `across` mm off the turning axis cuts, on the unrolled
/// material: from A (as flat `round`, mm round the surface) and the bit's height `z` (from
/// the material's top, straight up from the axis) to the tip's own `(round, z)`, its depth
/// measured towards the axis. A material of `radius`.
#[must_use]
pub fn tip_of(radius: f64, round: f64, z: f64, across: f64) -> (f64, f64) {
    let height = radius + z;
    let out = height.hypot(across);
    (round + radius * across.atan2(height), out - radius)
}

/// What [`tip_of`] undoes: where A turns to (as flat `round`) and how high the bit is (from
/// the material's top) for its tip, held `across` off the axis, to cut at `(round, z)` on the
/// unrolled material.
#[must_use]
pub fn held_at(radius: f64, round: f64, z: f64, across: f64) -> (f64, f64) {
    let out = (radius + z).max(across.abs());
    let height = (out * out - across * across).sqrt();
    (round - radius * across.atan2(height), height - radius)
}

impl RotaryAxis {
    /// Which way along the machine's across axis (Y for a rotary along X, X for one along Y)
    /// a positive `across` is: +1 or -1. +A turning clockwise from the chuck end carries the
    /// top of the material towards -Y on a rotary along X, and towards +X on one along Y.
    #[must_use]
    pub fn across_sign(&self, along: Along) -> f64 {
        let sign = if along == Along::X { 1.0 } else { -1.0 };
        if self.clockwise { sign } else { -sign }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_bit_held_off_the_axis_cuts_where_its_tip_reaches_and_back() {
        // straight over the axis nothing changes
        assert_eq!(tip_of(30.0, 12.0, -4.0, 0.0), (12.0, -4.0));
        // held 10 off a 30 radius, 26 up: its tip is 10 round and sqrt(26²+10²) out
        let (round, z) = tip_of(30.0, 0.0, -4.0, 10.0);
        assert!((z - (26f64.hypot(10.0) - 30.0)).abs() < 1e-9);
        assert!((round - 30.0 * 10f64.atan2(26.0)).abs() < 1e-9);
        for (r, a, z, d) in [(30.0, 5.0, -4.0, 10.0), (20.0, -3.0, 2.0, -7.0)] {
            let (w, t) = tip_of(r, a, z, d);
            let (back, height) = held_at(r, w, t, d);
            assert!((back - a).abs() < 1e-9 && (height - z).abs() < 1e-9);
        }
    }

    #[test]
    fn positive_across_is_the_way_the_top_turns_from_minus_a() {
        let mut axis = RotaryAxis::default();
        assert_eq!(axis.across_sign(Along::X), 1.0);
        assert_eq!(axis.across_sign(Along::Y), -1.0);
        axis.clockwise = false;
        assert_eq!(axis.across_sign(Along::X), -1.0);
    }
}
