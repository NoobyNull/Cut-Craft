//! Cutting simulation: what the material looks like after the toolpaths, for the preview.

use crate::document::Job;
use crate::geometry::Point;
use crate::toolpath::{Tool, ToolShape, Toolpath};
use glam::{DVec2, DVec3};

/// The material seen from above: a grid of square cells, each holding the height of the
/// surface over it (0 is the top of the material, `bottom` its underside).
#[derive(Clone, Debug, PartialEq)]
pub struct Material {
    pub columns: usize,
    pub rows: usize,
    /// Edge of one cell, mm.
    pub cell: f64,
    pub bottom: f64,
    /// Row by row from y = 0.
    pub heights: Vec<f64>,
}

#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_precision_loss,
    reason = "cell counts and indices are small and non-negative"
)]
impl Material {
    /// Uncut material covering the job, with `cells_across` square cells along its longer side.
    pub fn new(job: &Job, cells_across: usize) -> Self {
        let cell = job.width.max(job.height) / cells_across as f64;
        let count = |length: f64| ((length / cell).round() as usize).max(1);
        let (columns, rows) = (count(job.width), count(job.height));
        Self {
            columns,
            rows,
            cell,
            bottom: -job.thickness,
            heights: vec![0.0; columns * rows],
        }
    }

    /// Middle of a cell, job mm.
    pub fn centre(&self, column: usize, row: usize) -> Point {
        Point::new(column as f64 + 0.5, row as f64 + 0.5) * self.cell
    }

    /// Height of the surface over `p`, or `None` off the material.
    pub fn at(&self, p: Point) -> Option<f64> {
        let (column, row) = (p.x / self.cell, p.y / self.cell);
        if column < 0.0 || row < 0.0 {
            return None;
        }
        let (column, row) = (column as usize, row as usize);
        (column < self.columns && row < self.rows)
            .then(|| self.heights[row * self.columns + column])
    }

    /// Removes what the tool following `toolpath` cuts away.
    pub fn cut(&mut self, toolpath: &Toolpath) {
        for pair in toolpath.moves.windows(2) {
            self.sweep(pair[0].to, pair[1].to, &toolpath.tool);
        }
    }

    /// Lowers each cell the tool passes over from `a` to `b` to the lowest its cutting edge
    /// reaches there, at the cell centre.
    fn sweep(&mut self, a: DVec3, b: DVec3, tool: &Tool) {
        let radius = tool.diameter / 2.0;
        if a.z >= 0.0 && b.z >= 0.0 {
            return;
        }
        let (start, along) = (a.truncate(), (b - a).truncate());
        let (lo, hi) = (
            start.min(start + along) - radius,
            start.max(start + along) + radius,
        );
        let cells = |lo: f64, hi: f64, count: usize| {
            ((lo / self.cell).floor().max(0.0) as usize)
                ..((hi / self.cell).ceil().max(0.0) as usize).min(count)
        };
        for row in cells(lo.y, hi.y, self.rows) {
            for column in cells(lo.x, hi.x, self.columns) {
                let centre = self.centre(column, row);
                let Some((t0, t1)) = covered(start - centre, along, radius) else {
                    continue;
                };
                let z = match tool.shape {
                    // z is linear in t, so a flat end is lowest at one end of the covered span
                    ToolShape::EndMill => a.lerp(b, t0).z.min(a.lerp(b, t1).z),
                    _ => lowest(t0, t1, |t| {
                        let tip = a.lerp(b, t);
                        let r = tip.truncate().distance(centre).min(radius);
                        tip.z + tool.profile(r).unwrap_or(0.0)
                    }),
                }
                .max(self.bottom);
                let height = &mut self.heights[row * self.columns + column];
                *height = height.min(z);
            }
        }
    }
}

/// The least of a convex `f` over `[lo, hi]`, by ternary search; the ends are checked exactly.
fn lowest(lo: f64, hi: f64, f: impl Fn(f64) -> f64) -> f64 {
    let (mut a, mut b) = (lo, hi);
    for _ in 0..60 {
        let (third, two_thirds) = (a + (b - a) / 3.0, b - (b - a) / 3.0);
        if f(third) < f(two_thirds) {
            b = two_thirds;
        } else {
            a = third;
        }
    }
    f(lo).min(f(hi)).min(f(f64::midpoint(a, b)))
}

/// The span of t in [0, 1] where `|from + t * along| <= radius`.
fn covered(from: DVec2, along: DVec2, radius: f64) -> Option<(f64, f64)> {
    let (a, half_b, c) = (
        along.length_squared(),
        from.dot(along),
        from.length_squared() - radius * radius,
    );
    if a == 0.0 {
        return (c <= 0.0).then_some((0.0, 1.0));
    }
    let discriminant = half_b * half_b - a * c;
    if discriminant < 0.0 {
        return None;
    }
    let root = discriminant.sqrt();
    let (t0, t1) = (
        ((-half_b - root) / a).max(0.0),
        ((-half_b + root) / a).min(1.0),
    );
    (t0 <= t1).then_some((t0, t1))
}
