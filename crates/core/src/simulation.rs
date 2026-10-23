//! Cutting simulation: what the material looks like after the toolpaths, for the preview.

use crate::document::Job;
use crate::geometry::Point;
use crate::toolpath::{Tool, ToolShape, Toolpath};
use glam::{DVec2, DVec3};
use std::ops::Range;

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

    /// Indices of the cells from `lo` to `hi` mm along a side `count` cells long.
    fn cells(&self, lo: f64, hi: f64, count: usize) -> Range<usize> {
        ((lo / self.cell).floor().max(0.0) as usize)
            ..((hi / self.cell).ceil().max(0.0) as usize).min(count)
    }

    /// How low the tool's tip can go over `at` before its cutting edge touches the surface
    /// at any cell centre it covers ("drop cutter").
    pub fn drop_tool(&self, tool: &Tool, at: Point) -> f64 {
        let radius = tool.diameter / 2.0;
        let mut tip = f64::NEG_INFINITY;
        for row in self.cells(at.y - radius, at.y + radius, self.rows) {
            for column in self.cells(at.x - radius, at.x + radius, self.columns) {
                if let Some(rise) = tool.profile(self.centre(column, row).distance(at)) {
                    tip = tip.max(self.heights[row * self.columns + column] - rise);
                }
            }
        }
        // a tool narrower than a cell may cover no centre: rest on the cell under it
        if tip.is_finite() {
            tip
        } else {
            self.at(at).unwrap_or(0.0)
        }
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
        if a.z >= 0.0 && b.z >= 0.0 {
            return;
        }
        let (columns, rows) = self.swept(a, b, tool);
        for row in rows {
            for column in columns.clone() {
                if let Some(z) = lowest_edge(tool, a, b, self.centre(column, row)) {
                    let height = &mut self.heights[row * self.columns + column];
                    *height = height.min(z.max(self.bottom));
                }
            }
        }
    }

    /// How far below the surface the tool would cut going straight from `a` to `b`, at the
    /// deepest cell centre it passes over; zero or less when it clears everything.
    pub fn overcut(&self, tool: &Tool, a: DVec3, b: DVec3) -> f64 {
        let (columns, rows) = self.swept(a, b, tool);
        rows.flat_map(|row| columns.clone().map(move |column| (column, row)))
            .filter_map(|(column, row)| {
                let z = lowest_edge(tool, a, b, self.centre(column, row))?;
                Some(self.heights[row * self.columns + column] - z)
            })
            .fold(f64::NEG_INFINITY, f64::max)
    }

    /// The columns and rows under the tool's footprint anywhere from `a` to `b`.
    fn swept(&self, a: DVec3, b: DVec3, tool: &Tool) -> (Range<usize>, Range<usize>) {
        let radius = tool.diameter / 2.0;
        let (lo, hi) = (
            a.truncate().min(b.truncate()) - radius,
            a.truncate().max(b.truncate()) + radius,
        );
        (
            self.cells(lo.x, hi.x, self.columns),
            self.cells(lo.y, hi.y, self.rows),
        )
    }
}

/// The lowest the cutting edge of `tool` reaches over `centre` while its tip moves straight
/// from `a` to `b`; `None` if the tool never covers it.
fn lowest_edge(tool: &Tool, a: DVec3, b: DVec3, centre: Point) -> Option<f64> {
    let radius = tool.diameter / 2.0;
    let (start, along) = (a.truncate(), (b - a).truncate());
    let (t0, t1) = covered(start - centre, along, radius)?;
    let edge = |t: f64| {
        let tip = a.lerp(b, t);
        let r = tip.truncate().distance(centre).min(radius);
        tip.z + tool.profile(r).unwrap_or(0.0)
    };
    Some(match tool.shape {
        // z is linear in t, so a flat end is lowest at one end of the covered span
        ToolShape::EndMill => a.lerp(b, t0).z.min(a.lerp(b, t1).z),
        // a level move is lowest where it passes closest
        _ if (b.z - a.z).abs() < 1e-9 && along != DVec2::ZERO => {
            edge((-(start - centre).dot(along) / along.length_squared()).clamp(t0, t1))
        }
        _ => lowest(t0, t1, edge),
    })
}

/// The least of a convex `f` over `[lo, hi]`, by golden-section search; the ends are checked
/// exactly.
fn lowest(lo: f64, hi: f64, f: impl Fn(f64) -> f64) -> f64 {
    const SHRINK: f64 = 0.618_033_988_749_894_9; // 1 / golden ratio
    // the bracket, and two probes inside it with their values
    let (mut start, mut end) = (lo, hi);
    let mut left = (end - SHRINK * (end - start), 0.0);
    let mut right = (start + SHRINK * (end - start), 0.0);
    (left.1, right.1) = (f(left.0), f(right.0));
    for _ in 0..40 {
        if left.1 < right.1 {
            (end, right) = (right.0, left);
            left.0 = end - SHRINK * (end - start);
            left.1 = f(left.0);
        } else {
            (start, left) = (left.0, right);
            right.0 = start + SHRINK * (end - start);
            right.1 = f(right.0);
        }
    }
    f(lo).min(f(hi)).min(left.1).min(right.1)
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
