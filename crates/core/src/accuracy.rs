//! How exactly toolpaths are worked out, as Options ▸ Advanced sets it: the toolpath code reads
//! these where it once used fixed values, and the app sets them when Options loads or
//! changes, as it does the core limit. Each defaults to the value that was fixed, and each is
//! kept to a range where the result is still a safe cut.

use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};

/// How close curves are cut into straight pieces for offsets, pockets and arcs in G-code, mm.
pub const CURVE: f64 = crate::polygon::TOLERANCE;
/// How far a 3D roughing move may pass under the machining allowance before it is split, mm.
pub const ROUGHING_3D: f64 = 0.01;
/// How far a 3D finishing move may pass under the model before it is split, mm.
pub const FINISHING_3D: f64 = 0.002;
/// How far above the model what was left must stand to count as unmachined, mm.
pub const REST: f64 = 0.05;
/// The most sheets Add sheets as needed makes in one nesting.
pub const NEST_SHEETS: usize = 50;
/// The most filler parts one nesting places.
pub const NEST_FILLERS: usize = 2000;

/// The ranges Options offers; a value outside is kept to them.
pub const CURVE_RANGE: [f64; 2] = [0.001, 0.1];
pub const ROUGHING_3D_RANGE: [f64; 2] = [0.001, 0.1];
pub const FINISHING_3D_RANGE: [f64; 2] = [0.0005, 0.02];
pub const REST_RANGE: [f64; 2] = [0.005, 1.0];
pub const NEST_SHEETS_RANGE: [usize; 2] = [1, 1000];
pub const NEST_FILLERS_RANGE: [usize; 2] = [1, 100_000];

static CURVE_NOW: AtomicU64 = AtomicU64::new(CURVE.to_bits());
static ROUGHING_NOW: AtomicU64 = AtomicU64::new(ROUGHING_3D.to_bits());
static FINISHING_NOW: AtomicU64 = AtomicU64::new(FINISHING_3D.to_bits());
static REST_NOW: AtomicU64 = AtomicU64::new(REST.to_bits());
static SHEETS_NOW: AtomicUsize = AtomicUsize::new(NEST_SHEETS);
static FILLERS_NOW: AtomicUsize = AtomicUsize::new(NEST_FILLERS);

/// What Options ▸ Advanced says, all at once.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Accuracy {
    pub curve: f64,
    pub roughing_3d: f64,
    pub finishing_3d: f64,
    pub rest: f64,
    pub nest_sheets: usize,
    pub nest_fillers: usize,
}

impl Default for Accuracy {
    fn default() -> Self {
        Self {
            curve: CURVE,
            roughing_3d: ROUGHING_3D,
            finishing_3d: FINISHING_3D,
            rest: REST,
            nest_sheets: NEST_SHEETS,
            nest_fillers: NEST_FILLERS,
        }
    }
}

/// `value` kept to `range`; not a number gives `default`.
fn kept(value: f64, [lo, hi]: [f64; 2], default: f64) -> f64 {
    if value.is_nan() {
        default
    } else {
        value.clamp(lo, hi)
    }
}

/// Sets them all, each kept to its range.
pub fn set(a: Accuracy) {
    let store = |cell: &AtomicU64, v: f64| cell.store(v.to_bits(), Ordering::Relaxed);
    store(&CURVE_NOW, kept(a.curve, CURVE_RANGE, CURVE));
    store(
        &ROUGHING_NOW,
        kept(a.roughing_3d, ROUGHING_3D_RANGE, ROUGHING_3D),
    );
    store(
        &FINISHING_NOW,
        kept(a.finishing_3d, FINISHING_3D_RANGE, FINISHING_3D),
    );
    store(&REST_NOW, kept(a.rest, REST_RANGE, REST));
    let [lo, hi] = NEST_SHEETS_RANGE;
    SHEETS_NOW.store(a.nest_sheets.clamp(lo, hi), Ordering::Relaxed);
    let [lo, hi] = NEST_FILLERS_RANGE;
    FILLERS_NOW.store(a.nest_fillers.clamp(lo, hi), Ordering::Relaxed);
}

/// What is set now.
pub fn get() -> Accuracy {
    let load = |cell: &AtomicU64| f64::from_bits(cell.load(Ordering::Relaxed));
    Accuracy {
        curve: load(&CURVE_NOW),
        roughing_3d: load(&ROUGHING_NOW),
        finishing_3d: load(&FINISHING_NOW),
        rest: load(&REST_NOW),
        nest_sheets: SHEETS_NOW.load(Ordering::Relaxed),
        nest_fillers: FILLERS_NOW.load(Ordering::Relaxed),
    }
}

/// How close curves are cut into straight pieces for toolpaths, mm.
pub fn curve() -> f64 {
    get().curve
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn values_are_kept_to_their_ranges() {
        set(Accuracy {
            curve: 5.0,
            roughing_3d: 0.0,
            finishing_3d: f64::NAN,
            rest: 0.2,
            nest_sheets: 0,
            nest_fillers: 1_000_000,
        });
        assert_eq!(
            get(),
            Accuracy {
                curve: 0.1,
                roughing_3d: 0.001,
                finishing_3d: FINISHING_3D,
                rest: 0.2,
                nest_sheets: 1,
                nest_fillers: 100_000,
            }
        );
        set(Accuracy::default());
        assert_eq!(get(), Accuracy::default());
    }
}
