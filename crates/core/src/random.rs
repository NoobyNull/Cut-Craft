//! A small seeded random generator, so patterns recalculate the same.

/// A small seeded generator (splitmix64), so a toolpath recalculates to the same pattern.
pub struct Random(pub u64);

impl Random {
    /// Uniform in [0, 1).
    pub fn unit(&mut self) -> f64 {
        self.0 = self.0.wrapping_add(0x9e37_79b9_7f4a_7c15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        #[expect(clippy::cast_precision_loss, reason = "53 random bits")]
        let unit = (z >> 11) as f64 / (1u64 << 53) as f64;
        unit
    }

    /// Uniform between `percent` % of `max` and `max`.
    pub fn between(&mut self, percent: f64, max: f64) -> f64 {
        let min = max * percent / 100.0;
        min + (max - min) * self.unit()
    }
}
