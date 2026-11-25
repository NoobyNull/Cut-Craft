//! Part of the rotary carving engine.

use cut_craft_core::geometry::Point;
use cut_craft_core::simulation::Material;
use cut_craft_core::toolpath::{Tool, ToolShape};

/// How far above where a stretch starts the bit comes down to at the rapid rate, when
/// nothing is in the way, before it plunges, mm.
pub const APPROACH: f64 = 1.0;

/// How deep the registration mark goes, mm.
pub const MARK_DEPTH: f64 = 0.5;

/// Why a registration mark asked for is not cut.
pub const NO_ROOM_FOR_MARK: &str = "No room for the registration mark past End without cutting \
    into the part, so it is left out: pencil a line along the material at A=0 before you \
    start, or bring End in from the end of the material.";

/// The model as radii round the axis, on a grid of (along, round) cells: what the drop
/// works from.
pub struct Turned {
    /// Cells along the axis, and round it.
    pub along: usize,
    pub round: usize,
    /// Cell sizes along and round (at the surface), mm.
    pub along_cell: f64,
    pub round_cell: f64,
    /// The material's radius.
    pub radius: f64,
    /// Radius of each cell, row by row round the axis.
    pub radii: Vec<f64>,
    /// The smallest radius anywhere, and the largest.
    pub least: f64,
    pub most: f64,
}

impl Turned {
    /// `surface` (heights below the material's top, unrolled) as radii, on cells no larger
    /// than `cell`, each the highest of the surface under it so nothing is missed. Along and
    /// round count from the material's corner, `origin` in the job's X and Y.
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        clippy::cast_precision_loss,
        reason = "cell counts are small and positive"
    )]
    pub fn of(
        surface: Option<&Material>,
        radius: f64,
        along_x: bool,
        size: (f64, f64),
        cell: f64,
        origin: Point,
    ) -> Self {
        let (length, circumference) = size;
        let along = ((length / cell).ceil() as usize).max(1);
        let round = ((circumference / cell).ceil() as usize).max(3);
        let (along_cell, round_cell) = (length / along as f64, circumference / round as f64);
        // a few samples across each cell, as fine as the surface's own
        let samples = surface.map_or(1, |m| ((cell / m.cell).ceil() as usize).clamp(1, 4));
        let mut radii = Vec::with_capacity(along * round);
        for j in 0..round {
            for i in 0..along {
                let mut highest = f64::NEG_INFINITY;
                for si in 0..samples {
                    for sj in 0..samples {
                        let a = (i as f64 + (si as f64 + 0.5) / samples as f64) * along_cell;
                        let w = (j as f64 + (sj as f64 + 0.5) / samples as f64) * round_cell;
                        let at = if along_x {
                            Point::new(origin.x + a, origin.y + w)
                        } else {
                            Point::new(origin.x + w, origin.y + a)
                        };
                        if let Some(h) = surface.and_then(|m| m.at(at)).filter(|h| h.is_finite()) {
                            highest = highest.max(h);
                        }
                    }
                }
                // nothing of the model there: down to the axis
                radii.push((radius + highest).max(0.0));
            }
        }
        let least = radii.iter().copied().fold(f64::INFINITY, f64::min);
        let most = radii.iter().copied().fold(0.0, f64::max);
        Self {
            along,
            round,
            along_cell,
            round_cell,
            radius,
            radii,
            least,
            most,
        }
    }

    /// The highest the tip of `tool` can sit, held `across` mm to the side of the axis, with A
    /// turned to `round` (mm round the surface) and `along` the part, without touching the
    /// model grown by `allowance`: its height above the axis.
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_possible_wrap,
        clippy::cast_sign_loss,
        clippy::cast_precision_loss,
        reason = "cell indices are small"
    )]
    pub fn drop(&self, tool: &Tool, along: f64, round: f64, across: f64, allowance: f64) -> f64 {
        use std::f64::consts::FRAC_PI_2;
        let rb = tool.diameter / 2.0;
        let ball = tool.shape == ToolShape::BallNose;
        let theta = round / self.radius;
        // how far round the part a point can be and still be under the bit: furthest round
        // where the model comes nearest the axis, or furthest out for the far edge
        let (nearest, furthest) = (
            (self.least + allowance).max(1e-6),
            (self.most + allowance).max(1e-6),
        );
        let angle = |s: f64| s.clamp(-1.0, 1.0).asin();
        let low = if across - rb > 0.0 { furthest } else { nearest };
        let high = if across + rb > 0.0 { nearest } else { furthest };
        let (from, to) = (angle((across - rb) / low), angle((across + rb) / high));
        let rows = |phi: f64| (theta + phi) * self.radius / self.round_cell;
        let first = rows(from).floor() as i64 - 1;
        let last = (rows(to).ceil() as i64 + 1).min(first + self.round as i64 - 1);
        let cols = (rb / self.along_cell).ceil() as i64 + 1;
        let first_col = (along / self.along_cell).floor() as i64;
        let mut tip: f64 = 0.0;
        for col in first_col - cols..=first_col + cols {
            if col < 0 || col >= self.along as i64 {
                continue;
            }
            let dx = (col as f64 + 0.5) * self.along_cell - along;
            if dx.abs() > rb {
                continue;
            }
            for row in first..=last {
                let phi = (row as f64 + 0.5) * self.round_cell / self.radius - theta;
                if phi.abs() >= FRAC_PI_2 {
                    continue;
                }
                let at = row.rem_euclid(self.round as i64) as usize;
                let r = self.radii[at * self.along + col as usize] + allowance;
                let side = r * phi.sin() - across;
                let gap = rb * rb - dx * dx - side * side;
                if gap < 0.0 {
                    continue;
                }
                let here = if ball {
                    // the ball's centre, rb out from the tip, stays rb from the point
                    r * phi.cos() + gap.sqrt() - rb
                } else {
                    // a flat bottom rests on the point
                    r * phi.cos()
                };
                tip = tip.max(here);
            }
        }
        tip
    }
}
