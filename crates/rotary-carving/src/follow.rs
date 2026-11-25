//! Following the surface: on each line of a finish, A and how far across the axis the bit is
//! held change together at every point, so the bit meets the model square on. A flat face is
//! cut with A still and the bit moving across it; a steep side is cut by the side of the
//! ball facing it, not its slow tip. Four axes can't tilt the bit, so it is turned to where
//! the surface faces up instead.

use crate::{Way, Work};

/// The furthest a point is turned away from straight under the bit, radians (60°): past it
/// the bit would be held so far off the axis that the holder comes down beside the part.
const MOST_TILT: f64 = std::f64::consts::FRAC_PI_3;

/// One point of a finish's line: where A turns to (as `round`, mm round the surface), how far
/// across the axis the bit is held, and how low it goes there (from the axis).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Posed {
    pub along: f64,
    pub round: f64,
    pub across: f64,
    pub tip: f64,
}

impl Work<'_> {
    /// The points of `line` (along, round the surface) following the surface: each turned so
    /// the model faces straight up at it, no further than [`MOST_TILT`], with the bit held
    /// across the axis to stay over it.
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        clippy::cast_precision_loss,
        reason = "a window of a few points"
    )]
    pub fn following(&self, line: &[(f64, f64)], way: Way) -> Vec<Posed> {
        let radius = self.turned.radius;
        let cell = self.turned.round_cell;
        let over = |along: f64, round: f64| self.tip(along, round, 0.0).min(self.top);
        let heights: Vec<f64> = line.iter().map(|&(a, r)| over(a, r)).collect();
        let n = line.len();
        // how steeply the surface rises round the part at each point, from either side of it
        let rises: Vec<f64> = match way {
            // a ring's last point is its first again: its neighbours wrap round
            Way::RoundThePart if n > 2 => (0..n)
                .map(|j| {
                    let before = if j == 0 { n - 2 } else { j - 1 };
                    let after = if j + 1 >= n { 1 } else { j + 1 };
                    (heights[after] - heights[before]) / (2.0 * cell)
                })
                .collect(),
            _ => line
                .iter()
                .map(|&(a, r)| (over(a, r + cell) - over(a, r - cell)) / (2.0 * cell))
                .collect(),
        };
        // how far each point is turned: the surface's slope across it, as an angle from
        // straight up through the axis
        let tilts: Vec<f64> = rises
            .iter()
            .zip(&heights)
            .map(|(&rise, &h)| (rise * radius).atan2(h.max(1e-9)))
            .collect();
        // evened out over the ball's width, so A and the bit don't jitter with the model's
        // cells
        let half = ((self.tool.diameter / 2.0 / cell).round() as usize).max(1);
        let tilts: Vec<f64> = (0..n)
            .map(|j| {
                let near = &tilts[j.saturating_sub(half)..=(j + half).min(n - 1)];
                (near.iter().sum::<f64>() / near.len() as f64).clamp(-MOST_TILT, MOST_TILT)
            })
            .collect();
        line.iter()
            .zip(heights.iter().zip(&tilts))
            .map(|(&(along, round), (&h, &tilt))| {
                let across = h * tilt.sin();
                let round = round - radius * tilt;
                Posed {
                    along,
                    round,
                    across,
                    tip: self.tip(along, round, across).min(self.top),
                }
            })
            .collect()
    }
}
