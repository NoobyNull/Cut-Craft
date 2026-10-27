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
    /// The lower-left corner of the grid, in job coordinates.
    pub origin: Point,
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
            origin: job.area().min,
            columns,
            rows,
            cell,
            bottom: -job.thickness,
            heights: vec![0.0; columns * rows],
        }
    }

    /// Middle of a cell, job mm.
    pub fn centre(&self, column: usize, row: usize) -> Point {
        self.origin + Point::new(column as f64 + 0.5, row as f64 + 0.5) * self.cell
    }

    /// Height of the surface over `p`, or `None` off the material.
    pub fn at(&self, p: Point) -> Option<f64> {
        let (column, row) = (
            (p.x - self.origin.x) / self.cell,
            (p.y - self.origin.y) / self.cell,
        );
        if column < 0.0 || row < 0.0 {
            return None;
        }
        let (column, row) = (column as usize, row as usize);
        (column < self.columns && row < self.rows)
            .then(|| self.heights[row * self.columns + column])
    }

    /// The columns covering x from `lo` to `hi`.
    fn columns_between(&self, lo: f64, hi: f64) -> Range<usize> {
        self.span(lo - self.origin.x, hi - self.origin.x, self.columns)
    }

    /// The rows covering y from `lo` to `hi`.
    fn rows_between(&self, lo: f64, hi: f64) -> Range<usize> {
        self.span(lo - self.origin.y, hi - self.origin.y, self.rows)
    }

    /// Indices of the cells from `lo` to `hi` mm along a side `count` cells long, from its start.
    fn span(&self, lo: f64, hi: f64, count: usize) -> Range<usize> {
        ((lo / self.cell).floor().max(0.0) as usize)
            ..((hi / self.cell).ceil().max(0.0) as usize).min(count)
    }

    /// How low the tool's tip can go over `at` before its cutting edge touches the surface
    /// at any cell centre it covers ("drop cutter").
    pub fn drop_tool(&self, tool: &Tool, at: Point) -> f64 {
        self.drop_tool_on(tool, at, &self.peaks())
    }

    /// [`Self::drop_tool`] with this grid's [`Self::peaks`] worked out once: a taper is only
    /// searched in the tiles it could touch, which keeps a long taper quick.
    pub fn drop_tool_on(&self, tool: &Tool, at: Point, peaks: &Tiles) -> f64 {
        let near = (tool.diameter / 2.0).min(tool.reach());
        let mut tip = f64::NEG_INFINITY;
        let rest_on = |columns: Range<usize>, rows: Range<usize>, tip: &mut f64| {
            for row in rows {
                for column in columns.clone() {
                    if let Some(rise) = tool.profile(self.centre(column, row).distance(at)) {
                        *tip = tip.max(self.heights[row * self.columns + column] - rise);
                    }
                }
            }
        };
        rest_on(
            self.columns_between(at.x - near, at.x + near),
            self.rows_between(at.y - near, at.y + near),
            &mut tip,
        );
        if tool.reach() > near {
            self.tiles_near(peaks, at, at, tool.reach(), |tile, gap, columns, rows| {
                if tool
                    .profile(gap)
                    .is_some_and(|rise| peaks.values[tile] - rise > tip)
                {
                    rest_on(columns, rows, &mut tip);
                }
            });
        }
        // a tool narrower than a cell may cover no centre: rest on the cell under it
        if tip.is_finite() {
            tip
        } else {
            self.at(at).unwrap_or(0.0)
        }
    }

    /// The highest cell in each tile of the grid.
    pub fn peaks(&self) -> Tiles {
        Tiles::of(
            &self.heights,
            self.columns,
            self.rows,
            f64::NEG_INFINITY,
            f64::max,
        )
    }

    /// Calls `visit` with each tile that may have cells within `radius` of the line from
    /// `a` to `b`: its index, how near to the line its cells can be, and its cells.
    fn tiles_near(
        &self,
        tiles: &Tiles,
        a: Point,
        b: Point,
        radius: f64,
        mut visit: impl FnMut(usize, f64, Range<usize>, Range<usize>),
    ) {
        let n = tiles.size;
        let (lo, hi) = (
            a.min(b) - Point::splat(radius),
            a.max(b) + Point::splat(radius),
        );
        let (columns, rows) = (
            self.columns_between(lo.x, hi.x),
            self.rows_between(lo.y, hi.y),
        );
        for ty in rows.start / n..rows.end.div_ceil(n) {
            for tx in columns.start / n..columns.end.div_ceil(n) {
                let cells_x = tx * n..((tx + 1) * n).min(self.columns);
                let cells_y = ty * n..((ty + 1) * n).min(self.rows);
                let size = Point::new(cells_x.len() as f64, cells_y.len() as f64) * self.cell;
                let corner = self.origin
                    + Point::new(cells_x.start as f64, cells_y.start as f64) * self.cell;
                let centre = corner + size / 2.0;
                let gap = (distance_to_line(centre, a, b) - size.length() / 2.0).max(0.0);
                if gap <= radius {
                    visit(ty * tiles.columns + tx, gap, cells_x, cells_y);
                }
            }
        }
    }

    /// The surface `tool` leaves after finishing everywhere over this one, from every cell
    /// centre as low as it goes: what its cutting edge cannot reach stays.
    #[must_use]
    pub fn left_by(&self, tool: &Tool) -> Self {
        let peaks = self.peaks();
        let tips: Vec<f64> = (0..self.rows)
            .flat_map(|row| (0..self.columns).map(move |column| (column, row)))
            .map(|(column, row)| self.drop_tool_on(tool, self.centre(column, row), &peaks))
            .collect();
        let floors = Tiles::of(&tips, self.columns, self.rows, f64::INFINITY, f64::min);
        let near = (tool.diameter / 2.0).min(tool.reach());
        let mut left = self.clone();
        for row in 0..self.rows {
            for column in 0..self.columns {
                let at = self.centre(column, row);
                let mut lowest = f64::INFINITY;
                let edge_over = |columns: Range<usize>, rows: Range<usize>, lowest: &mut f64| {
                    for r in rows {
                        for c in columns.clone() {
                            if let Some(rise) = tool.profile(self.centre(c, r).distance(at)) {
                                *lowest = lowest.min(tips[r * self.columns + c] + rise);
                            }
                        }
                    }
                };
                edge_over(
                    self.columns_between(at.x - near, at.x + near),
                    self.rows_between(at.y - near, at.y + near),
                    &mut lowest,
                );
                // the taper only matters in tiles where it could come lower than that
                if tool.reach() > near {
                    self.tiles_near(&floors, at, at, tool.reach(), |tile, gap, columns, rows| {
                        if tool
                            .profile(gap)
                            .is_some_and(|rise| floors.values[tile] + rise < lowest)
                        {
                            edge_over(columns, rows, &mut lowest);
                        }
                    });
                }
                let height = &mut left.heights[row * self.columns + column];
                if lowest.is_finite() {
                    *height = lowest.max(*height);
                }
            }
        }
        left
    }

    /// Removes what the tool following `toolpath` cuts away.
    pub fn cut(&mut self, toolpath: &Toolpath) {
        for pair in toolpath.moves.windows(2) {
            self.sweep(pair[0].to, pair[1].to, &toolpath.tool);
        }
    }

    /// Lowers each cell the tool passes over from `a` to `b` to the lowest its cutting edge
    /// reaches there, at the cell centre.
    pub fn sweep(&mut self, a: DVec3, b: DVec3, tool: &Tool) {
        if a.z >= 0.0 && b.z >= 0.0 {
            return;
        }
        // the material is nowhere above its top, 0
        let (columns, rows) = self.swept(a, b, tool.reach_below(-a.z.min(b.z)));
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
        self.overcut_on(tool, a, b, &self.peaks())
    }

    /// [`Self::overcut`] with this grid's [`Self::peaks`] worked out once.
    pub fn overcut_on(&self, tool: &Tool, a: DVec3, b: DVec3, peaks: &Tiles) -> f64 {
        let mut worst = f64::NEG_INFINITY;
        let (low, from, to) = (a.z.min(b.z), a.truncate(), b.truncate());
        let dig = |columns: Range<usize>, rows: Range<usize>, worst: &mut f64| {
            for row in rows {
                for column in columns.clone() {
                    let (centre, height) = (
                        self.centre(column, row),
                        self.heights[row * self.columns + column],
                    );
                    // the edge comes no lower than the lowest end at the nearest the line
                    // passes: past that, the exact search cannot find worse
                    let nearest = distance_to_line(centre, from, to);
                    if !tool
                        .profile(nearest)
                        .is_some_and(|rise| height - (low + rise) > *worst)
                    {
                        continue;
                    }
                    if let Some(z) = lowest_edge(tool, a, b, centre) {
                        *worst = worst.max(height - z);
                    }
                }
            }
        };
        let near = (tool.diameter / 2.0).min(tool.reach());
        let (columns, rows) = self.swept(a, b, near);
        dig(columns, rows, &mut worst);
        if tool.reach() > near {
            self.tiles_near(peaks, from, to, tool.reach(), |tile, gap, columns, rows| {
                if tool
                    .profile(gap)
                    .is_some_and(|rise| peaks.values[tile] - (low + rise) > worst)
                {
                    dig(columns, rows, &mut worst);
                }
            });
        }
        worst
    }

    /// The columns and rows within `radius` of the tool's axis anywhere from `a` to `b`.
    fn swept(&self, a: DVec3, b: DVec3, radius: f64) -> (Range<usize>, Range<usize>) {
        let (lo, hi) = (
            a.truncate().min(b.truncate()) - radius,
            a.truncate().max(b.truncate()) + radius,
        );
        (
            self.columns_between(lo.x, hi.x),
            self.rows_between(lo.y, hi.y),
        )
    }
}

/// The highest (or lowest) value in each square tile of a grid, so a search can pass over
/// tiles nothing in which could matter.
#[derive(Clone, Debug, PartialEq)]
pub struct Tiles {
    /// Cells along a tile's side.
    size: usize,
    /// Tiles across.
    columns: usize,
    values: Vec<f64>,
}

impl Tiles {
    // ponytail: 8-cell tiles, one level; a deeper pyramid if long tapers still crawl
    const SIZE: usize = 8;

    /// `pick` (max or min) of each tile's cells of a `columns` by `rows` grid, from `start`.
    fn of(
        values: &[f64],
        columns: usize,
        rows: usize,
        start: f64,
        pick: fn(f64, f64) -> f64,
    ) -> Self {
        let n = Self::SIZE;
        let across = columns.div_ceil(n);
        let mut tiles = vec![start; across * rows.div_ceil(n)];
        for row in 0..rows {
            for column in 0..columns {
                let t = &mut tiles[row / n * across + column / n];
                *t = pick(*t, values[row * columns + column]);
            }
        }
        Self {
            size: n,
            columns: across,
            values: tiles,
        }
    }
}

/// How far `p` is from the line from `a` to `b`.
fn distance_to_line(p: Point, a: Point, b: Point) -> f64 {
    let along = b - a;
    let t = if along == Point::ZERO {
        0.0
    } else {
        ((p - a).dot(along) / along.length_squared()).clamp(0.0, 1.0)
    };
    p.distance(a + along * t)
}

/// The lowest the cutting edge of `tool` reaches over `centre` while its tip moves straight
/// from `a` to `b`; `None` if the tool never covers it.
fn lowest_edge(tool: &Tool, a: DVec3, b: DVec3, centre: Point) -> Option<f64> {
    let radius = tool.reach();
    let (start, along) = (a.truncate(), (b - a).truncate());
    let (t0, t1) = covered(start - centre, along, radius)?;
    let edge = |t: f64| {
        let tip = a.lerp(b, t);
        let r = tip.truncate().distance(centre).min(radius);
        tip.z + tool.profile(r).unwrap_or(0.0)
    };
    Some(match tool.shape {
        // z is linear in t, so a flat end (untapered) is lowest at one end of the covered span
        ToolShape::EndMill if tool.reach() <= tool.diameter / 2.0 => {
            a.lerp(b, t0).z.min(a.lerp(b, t1).z)
        }
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

/// How far above the model what was left must stand to count as unmachined.
// ponytail: one fixed tolerance; a setting if finishes need finer or coarser rest areas
const REST_TOLERANCE: f64 = 0.05;

/// Rest machining's areas on `model`'s grid: where the surface `left` (on any grid) stands
/// above the model, leaving out areas smaller across than `minimum_detail`, and grown by
/// `offset`. True for a point inside them.
#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_precision_loss,
    clippy::cast_possible_wrap,
    reason = "cell counts and indices are small and non-negative"
)]
pub fn rest_areas(
    model: &Material,
    left: &Material,
    minimum_detail: f64,
    offset: f64,
) -> impl Fn(Point) -> bool + use<> {
    let (columns, rows) = (model.columns, model.rows);
    let height = |c: usize, r: usize| model.heights[r * columns + c];
    // how steep the model is: tool positions only at cell centres make `left` up to about
    // slope x cell too high there, which is not material left behind
    let slope = |c: usize, r: usize| {
        let along = |lo: usize, hi: usize, h: &dyn Fn(usize) -> f64| {
            if hi > lo {
                (h(hi) - h(lo)) / ((hi - lo) as f64 * model.cell)
            } else {
                0.0
            }
        };
        let dx = along(c.saturating_sub(1), (c + 1).min(columns - 1), &|c| {
            height(c, r)
        });
        let dy = along(r.saturating_sub(1), (r + 1).min(rows - 1), &|r| {
            height(c, r)
        });
        dx.hypot(dy)
    };
    let unmachined: Vec<bool> = (0..rows * columns)
        .map(|i| {
            let (c, r) = (i % columns, i / columns);
            let here = height(c, r);
            let allowed = REST_TOLERANCE + slope(c, r) * model.cell;
            left.at(model.centre(c, r)).unwrap_or(here) - here > allowed
        })
        .collect();
    // keep the areas at least the minimum detail across
    let mut kept = vec![false; rows * columns];
    let mut seen = vec![false; rows * columns];
    for start in 0..rows * columns {
        if !unmachined[start] || seen[start] {
            continue;
        }
        seen[start] = true;
        let (mut area, mut next) = (vec![], vec![start]);
        while let Some(i) = next.pop() {
            area.push(i);
            let (c, r) = (i % columns, i / columns);
            let neighbours = [
                (c > 0).then(|| i - 1),
                (c + 1 < columns).then_some(i + 1),
                (r > 0).then(|| i - columns),
                (r + 1 < rows).then_some(i + columns),
            ];
            for n in neighbours.into_iter().flatten() {
                if unmachined[n] && !seen[n] {
                    seen[n] = true;
                    next.push(n);
                }
            }
        }
        let across = span_in(&area, |i| i % columns).max(span_in(&area, |i| i / columns));
        if across as f64 * model.cell >= minimum_detail {
            for i in area {
                kept[i] = true;
            }
        }
    }
    // grown by the offset
    let reach = (offset / model.cell).floor() as isize;
    let mut grown = kept.clone();
    for i in (0..rows * columns).filter(|&i| kept[i]) {
        let (c, r) = ((i % columns) as isize, (i / columns) as isize);
        for dr in -reach..=reach {
            for dc in -reach..=reach {
                let (cc, rr) = (c + dc, r + dr);
                if dc * dc + dr * dr <= reach * reach
                    && (0..columns as isize).contains(&cc)
                    && (0..rows as isize).contains(&rr)
                {
                    grown[rr as usize * columns + cc as usize] = true;
                }
            }
        }
    }
    let (origin, cell) = (model.origin, model.cell);
    move |p: Point| {
        let (x, y) = ((p.x - origin.x) / cell, (p.y - origin.y) / cell);
        x >= 0.0
            && y >= 0.0
            && (x as usize) < columns
            && (y as usize) < rows
            && grown[y as usize * columns + x as usize]
    }
}

/// How many distinct values `of` takes over `area`'s cells, end to end.
fn span_in(area: &[usize], of: impl Fn(usize) -> usize) -> usize {
    let lo = area.iter().map(|&i| of(i)).min().unwrap_or(0);
    let hi = area.iter().map(|&i| of(i)).max().unwrap_or(0);
    hi - lo + 1
}
