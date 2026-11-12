//! What Options says about importing, for importers: they are handed only a file's bytes,
//! so the app sets these when Options loads or changes, as it does the core limit.

use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};

/// How many pixels make an inch of an imported picture unless Options says otherwise: the
/// screen's usual 96.
pub const PIXELS_PER_INCH: f64 = 96.0;

/// Points per knot span a DXF spline is turned into unless Options says otherwise.
pub const SPLINE_SAMPLES: u32 = 32;

/// The pixels-per-inch and spline-samples ranges Options offers; values outside are kept
/// to them.
pub const PIXELS_PER_INCH_RANGE: std::ops::RangeInclusive<f64> = 10.0..=1200.0;
pub const SPLINE_SAMPLES_RANGE: std::ops::RangeInclusive<u32> = 4..=256;

static PICTURE_PPI: AtomicU64 = AtomicU64::new(PIXELS_PER_INCH.to_bits());
static SAMPLES: AtomicU32 = AtomicU32::new(SPLINE_SAMPLES);

/// Sets the size imported pictures come in at, in pixels per inch.
pub fn set_pixels_per_inch(ppi: f64) {
    PICTURE_PPI.store(ppi_in_range(ppi).to_bits(), Ordering::Relaxed);
}

/// The size imported pictures come in at, in pixels per inch.
pub fn pixels_per_inch() -> f64 {
    f64::from_bits(PICTURE_PPI.load(Ordering::Relaxed))
}

/// Sets how many points each knot span of an imported DXF spline becomes.
pub fn set_spline_samples(samples: u32) {
    SAMPLES.store(samples_in_range(samples), Ordering::Relaxed);
}

/// How many points each knot span of an imported DXF spline becomes.
pub fn spline_samples() -> u32 {
    SAMPLES.load(Ordering::Relaxed)
}

/// `ppi` kept to the range Options offers; not a number gives the standard 96.
pub fn ppi_in_range(ppi: f64) -> f64 {
    if ppi.is_nan() {
        PIXELS_PER_INCH
    } else {
        ppi.clamp(*PIXELS_PER_INCH_RANGE.start(), *PIXELS_PER_INCH_RANGE.end())
    }
}

/// `samples` kept to the range Options offers.
pub fn samples_in_range(samples: u32) -> u32 {
    samples.clamp(*SPLINE_SAMPLES_RANGE.start(), *SPLINE_SAMPLES_RANGE.end())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn settings_are_kept_to_their_ranges() {
        assert_eq!(ppi_in_range(300.0), 300.0);
        assert_eq!(ppi_in_range(0.0), 10.0);
        assert_eq!(ppi_in_range(1e9), 1200.0);
        assert_eq!(ppi_in_range(f64::NAN), PIXELS_PER_INCH);
        assert_eq!(samples_in_range(0), 4);
        assert_eq!(samples_in_range(64), 64);
        assert_eq!(samples_in_range(10_000), 256);
    }
}
