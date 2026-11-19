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
        }
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
