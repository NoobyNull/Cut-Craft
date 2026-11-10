//! The ornament's pieces raised into a height grid: each piece rounded by how far each point
//! is from its edge and where it lies along its middle line, the later parts (leaves over
//! bands, eyes and hearts over those) resting on what is under them, then the whole closed
//! with the finishing bit's ball so nothing is finer than the bit can carve.

use cmd_vector_texture::{Part, Piece};
use cut_craft_core::geometry::{Point, Rect};

/// A height grid: row by row from the bottom, `NaN` where nothing is raised.
#[derive(Clone, Debug)]
pub struct Grid {
    pub origin: Point,
    pub cell: f64,
    pub columns: usize,
    pub rows: usize,
    pub heights: Vec<f64>,
}

impl Grid {
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "cell counts are small"
    )]
    fn new(area: Rect, cell: f64) -> Self {
        let count = |length: f64| ((length / cell).ceil() as usize).max(1);
        let (columns, rows) = (count(area.size().x), count(area.size().y));
        Self {
            origin: area.min,
            cell,
            columns,
            rows,
            heights: vec![f64::NAN; columns * rows],
        }
    }

    /// The centre of the cell in `column` and `row`.
    #[expect(clippy::cast_precision_loss, reason = "cell counts are small")]
    pub fn centre(&self, column: usize, row: usize) -> Point {
        self.origin + Point::new(column as f64 + 0.5, row as f64 + 0.5) * self.cell
    }

    /// The highest raised cell, if any.
    pub fn top(&self) -> Option<f64> {
        self.heights
            .iter()
            .copied()
            .filter(|h| !h.is_nan())
            .reduce(f64::max)
    }
}

/// How the pieces are rounded.
#[derive(Clone, Copy, Debug)]
pub struct Shaping {
    /// The highest point, mm.
    pub height: f64,
    /// The finishing bit's radius (a ball-nose), mm: nothing in the relief is finer.
    pub bit_radius: f64,
    /// Bands with a groove down their middle, and leaves with a deep midrib (baroque).
    pub fluted: bool,
}

/// Which pieces go down first: bands, then the leaves over them, then flowers, then eyes
/// and hearts on top.
fn layer(part: Part) -> u8 {
    match part {
        Part::Band => 0,
        Part::Leaf => 1,
        Part::Petals => 2,
        Part::Eye => 3,
    }
}

/// How much of what is under a piece it rests on: it sits on the blurred shape below, so a
/// leaf laid over a band swells where the band runs under it.
const REST: f64 = 0.6;

/// The pieces raised over `area` in cells of `cell` mm, the highest point `shaping.height`,
/// carvable with the finishing bit.
#[must_use]
pub fn raise(pieces: &[Piece], area: Rect, cell: f64, shaping: Shaping) -> Grid {
    let mut grid = Grid::new(area, cell);
    let mut order: Vec<&Piece> = pieces.iter().collect();
    order.sort_by_key(|p| layer(p.part));
    // each piece's own shape on its own, spread over the processor's cores
    let threads = std::thread::available_parallelism().map_or(1, std::num::NonZero::get);
    let shaped: Vec<Option<Shaped>> = std::thread::scope(|scope| {
        let grid = &grid;
        let order = &order;
        let workers: Vec<_> = (0..threads)
            .map(|k| {
                scope.spawn(move || {
                    (k..order.len())
                        .step_by(threads)
                        .map(|i| (i, shape(grid, order[i], shaping)))
                        .collect::<Vec<_>>()
                })
            })
            .collect();
        let mut all: Vec<Option<Shaped>> = (0..order.len()).map(|_| None).collect();
        for worker in workers {
            for (i, made) in worker.join().expect("a shaping thread panicked") {
                all[i] = made;
            }
        }
        all
    });
    // then laid in order, each resting on what is under it
    let mut under = vec![0.0; grid.heights.len()];
    let mut current = None;
    for (piece, made) in order.iter().zip(shaped) {
        if current != Some(layer(piece.part)) {
            current = Some(layer(piece.part));
            // what this layer rests on: everything so far, softened by about a bit across
            under = blur(&grid, shaping.bit_radius * 2.0);
        }
        if let Some(made) = made {
            lay(&mut grid, &under, &made);
        }
    }
    soften(&mut grid);
    if let Some(top) = grid.top().filter(|&t| t > 0.0) {
        let scale = shaping.height / top;
        for h in &mut grid.heights {
            *h *= scale;
        }
    }
    close(&mut grid, shaping.bit_radius);
    grid
}

/// A piece's own heights over its box of cells (from `start`, `size` across), `NaN`
/// outside it.
struct Shaped {
    start: (usize, usize),
    size: (usize, usize),
    heights: Vec<f64>,
}

/// One piece's own heights, before it rests on anything.
#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "cell indices are small"
)]
fn shape(grid: &Grid, piece: &Piece, shaping: Shaping) -> Option<Shaped> {
    let first = piece.outline.first()?;
    let bounds = piece
        .outline
        .iter()
        .fold(Rect::point(*first), |r, &p| r.include(p));
    let cell = grid.cell;
    // the piece's cells, one spare all round so its edge is outside
    let to_cell = |v: f64, o: f64| ((v - o) / cell - 0.5).floor() as i64;
    let c0 = (to_cell(bounds.min.x, grid.origin.x) - 1).max(0) as usize;
    let r0 = (to_cell(bounds.min.y, grid.origin.y) - 1).max(0) as usize;
    let c1 = ((to_cell(bounds.max.x, grid.origin.x) + 2).max(0) as usize).min(grid.columns);
    let r1 = ((to_cell(bounds.max.y, grid.origin.y) + 2).max(0) as usize).min(grid.rows);
    if c0 >= c1 || r0 >= r1 {
        return None;
    }
    let (w, h) = (c1 - c0, r1 - r0);
    // corners cut, so the distance from the edge has no creases running in from them
    let outline = resample(&rounded_ring(&piece.outline), cell);
    let inside = mask(&outline, grid, (c0, r0), (w, h));
    // distance from each inside cell to the outline, mm: roughly by cells, then exactly
    // to the outline's segments near that
    let rough = distances(&inside, w, h);
    let near = Segments::new(&outline, cell * 8.0);
    // where each cell lies along the middle line, and how far from it
    let spine = Line::new(
        resample(&rounded(&piece.spine), cell * 2.0),
        (cell * 8.0).max(bounds.size().length() / 16.0),
    );
    let top = shaping.height * piece.rise;
    let heights = (0..w * h)
        .map(|i| {
            if !inside[i] {
                return f64::NAN;
            }
            let at = grid.centre(c0 + i % w, r0 + i / w);
            let edge = near.distance(at, (rough[i].sqrt() + 1.5) * cell);
            let (t, off) = spine.nearest(at);
            // 0 at the edge, 1 on the middle line: smooth however the width changes
            let u = edge / (edge + off).max(1e-9);
            top * profile(piece.part, t, u, shaping.fluted)
        })
        .collect();
    Some(Shaped {
        start: (c0, r0),
        size: (w, h),
        heights,
    })
}

/// A shaped piece laid onto the grid, resting on `under`: never lower than what it covers.
fn lay(grid: &mut Grid, under: &[f64], piece: &Shaped) {
    let ((c0, r0), (w, h)) = (piece.start, piece.size);
    for r in 0..h {
        for c in 0..w {
            let own = piece.heights[r * w + c];
            if own.is_nan() {
                continue;
            }
            let at = (r0 + r) * grid.columns + c0 + c;
            let below = grid.heights[at];
            let raised = REST * under[at] + own;
            grid.heights[at] = if below.is_nan() {
                raised
            } else {
                below.max(raised)
            };
        }
    }
}

/// A piece's height as a share of its top: `t` how far along its middle (0 at its foot), `u`
/// how far in from its edge (0 on it, 1 on the middle line).
fn profile(part: Part, t: f64, u: f64, fluted: bool) -> f64 {
    let round = (1.0 - (1.0 - u).powi(2)).sqrt();
    match part {
        Part::Band => {
            // rising as it rolls into the volute, a groove down the middle of a baroque one
            let along = 0.7 + 0.3 * smooth(0.0, 1.0, t);
            let flute = if fluted {
                0.22 * (-((1.0 - u) / 0.3).powi(2)).exp() * smooth(0.0, 0.15, t)
            } else {
                0.0
            };
            along * (0.3 + 0.7 * round - flute)
        }
        Part::Leaf => {
            // lifting from the band to curl over near the tip, then rolling down to it
            let along = 0.5 + 0.5 * (std::f64::consts::FRAC_PI_2 * (t / 0.7).min(1.0)).sin()
                - 0.3 * smooth(0.8, 1.0, t);
            let across = if fluted {
                // acanthus: a rolled rim following every lobe, a scooped channel inside it
                // and a raised midrib, so the lobes read as carved hollows
                let rim = smooth(0.0, 0.3, u);
                let channel = 0.4 * (-((u - 0.62) / 0.17).powi(2)).exp();
                let rib = 0.2 * smooth(0.85, 1.0, u);
                rim - channel + rib
            } else {
                // western: domed, a vein cut down the middle
                u.powf(0.6) - 0.15 * (-((1.0 - u) / 0.18).powi(2)).exp() * smooth(0.05, 0.3, t)
            };
            along * (0.25 + 0.75 * across)
        }
        Part::Eye => 0.55 + 0.45 * round,
        Part::Petals => 0.3 + 0.7 * u.powf(0.5),
    }
}

/// 0 below `a`, 1 above `b`, smoothly between.
fn smooth(a: f64, b: f64, x: f64) -> f64 {
    let t = ((x - a) / (b - a)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// Which of the `size` cells from `start` have their centres inside `outline`.
#[expect(clippy::cast_precision_loss, reason = "cell indices are small")]
fn mask(outline: &[Point], grid: &Grid, start: (usize, usize), size: (usize, usize)) -> Vec<bool> {
    let (w, h) = size;
    let mut inside = vec![false; w * h];
    let mut crossings = Vec::new();
    for r in 0..h {
        let y = grid.centre(0, start.1 + r).y;
        crossings.clear();
        for (i, &a) in outline.iter().enumerate() {
            let b = outline[(i + 1) % outline.len()];
            if (a.y > y) != (b.y > y) {
                crossings.push(a.x + (y - a.y) / (b.y - a.y) * (b.x - a.x));
            }
        }
        crossings.sort_by(f64::total_cmp);
        for pair in crossings.as_chunks::<2>().0 {
            for c in 0..w {
                let x = grid.origin.x + ((start.0 + c) as f64 + 0.5) * grid.cell;
                if x >= pair[0] && x < pair[1] {
                    inside[r * w + c] = true;
                }
            }
        }
    }
    inside
}

/// Squared distance, in cells, from each inside cell to the nearest outside one (exact,
/// Felzenszwalb and Huttenlocher's two passes).
fn distances(inside: &[bool], w: usize, h: usize) -> Vec<f64> {
    const FAR: f64 = 1e20;
    let mut d: Vec<f64> = inside.iter().map(|&i| if i { FAR } else { 0.0 }).collect();
    let mut line = Vec::new();
    for c in 0..w {
        line.clear();
        line.extend((0..h).map(|r| d[r * w + c]));
        let out = transform(&line);
        for (r, v) in out.into_iter().enumerate() {
            d[r * w + c] = v;
        }
    }
    for r in 0..h {
        let out = transform(&d[r * w..(r + 1) * w]);
        d[r * w..(r + 1) * w].copy_from_slice(&out);
    }
    d
}

/// One pass of the squared distance transform over `f`.
#[expect(
    clippy::cast_precision_loss,
    clippy::many_single_char_names,
    reason = "cell indices are small; the names are the paper's"
)]
fn transform(f: &[f64]) -> Vec<f64> {
    let n = f.len();
    let mut out = vec![0.0; n];
    let mut v = vec![0usize; n];
    let mut z = vec![0.0f64; n + 1];
    let mut k = 0;
    z[0] = f64::NEG_INFINITY;
    z[1] = f64::INFINITY;
    let at = |q: usize| q as f64;
    for q in 1..n {
        let mut s;
        loop {
            let p = v[k];
            s = ((f[q] + at(q) * at(q)) - (f[p] + at(p) * at(p))) / (2.0 * (at(q) - at(p)));
            // z[0] is minus infinity, so this stops at the first parabola
            if s <= z[k] {
                k -= 1;
            } else {
                break;
            }
        }
        k += 1;
        v[k] = q;
        z[k] = s;
        z[k + 1] = f64::INFINITY;
    }
    k = 0;
    for (q, o) in out.iter_mut().enumerate() {
        while z[k + 1] < at(q) {
            k += 1;
        }
        let p = v[k];
        *o = (at(q) - at(p)).powi(2) + f[p];
    }
    out
}

/// A piece's middle line, its segments filed by the square they lie over.
struct Line {
    points: Vec<Point>,
    size: f64,
    squares: std::collections::HashMap<(i64, i64), Vec<usize>>,
}

impl Line {
    #[expect(clippy::cast_possible_truncation, reason = "grid squares")]
    fn new(points: Vec<Point>, size: f64) -> Self {
        let mut squares: std::collections::HashMap<(i64, i64), Vec<usize>> =
            std::collections::HashMap::new();
        let key = |v: f64| (v / size).floor() as i64;
        for (k, w) in points.windows(2).enumerate() {
            let (a, b) = (w[0], w[1]);
            for x in key(a.x.min(b.x))..=key(a.x.max(b.x)) {
                for y in key(a.y.min(b.y))..=key(a.y.max(b.y)) {
                    squares.entry((x, y)).or_default().push(k);
                }
            }
        }
        Self {
            points,
            size,
            squares,
        }
    }

    /// How far along the line the point nearest `at` is (0 at its start, 1 at its end, by
    /// segments), and how far `at` is from it: the squares searched ring by ring outwards
    /// until no nearer segment can be further out.
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_precision_loss,
        reason = "grid squares and a few thousand points"
    )]
    fn nearest(&self, at: Point) -> (f64, f64) {
        match self.points.as_slice() {
            [] => return (0.0, 0.0),
            [only] => return (0.0, only.distance(at)),
            _ => {}
        }
        let (cx, cy) = (
            (at.x / self.size).floor() as i64,
            (at.y / self.size).floor() as i64,
        );
        let last = (self.points.len() - 1) as f64;
        let mut best = (0.0, f64::INFINITY);
        for ring in 0..10_000_i64 {
            for x in cx - ring..=cx + ring {
                for y in cy - ring..=cy + ring {
                    if (x - cx).abs() != ring && (y - cy).abs() != ring {
                        continue;
                    }
                    for &k in self.squares.get(&(x, y)).into_iter().flatten() {
                        let (a, b) = (self.points[k], self.points[k + 1]);
                        let d = b - a;
                        let s = ((at - a).dot(d) / d.length_squared().max(1e-12)).clamp(0.0, 1.0);
                        let gap = (a + d * s).distance_squared(at);
                        if gap < best.1 {
                            best = ((k as f64 + s) / last, gap);
                        }
                    }
                }
            }
            let reach = ring as f64 * self.size;
            if best.1 <= reach * reach {
                break;
            }
        }
        (best.0, best.1.sqrt())
    }
}

/// A closed outline's segments filed by the square they lie over, to find the nearest
/// quickly.
struct Segments {
    size: f64,
    squares: std::collections::HashMap<(i64, i64), Vec<(Point, Point)>>,
}

impl Segments {
    #[expect(clippy::cast_possible_truncation, reason = "grid squares")]
    fn new(outline: &[Point], size: f64) -> Self {
        let mut squares: std::collections::HashMap<(i64, i64), Vec<(Point, Point)>> =
            std::collections::HashMap::new();
        let key = |v: f64| (v / size).floor() as i64;
        for (i, &a) in outline.iter().enumerate() {
            let b = outline[(i + 1) % outline.len()];
            for x in key(a.x.min(b.x))..=key(a.x.max(b.x)) {
                for y in key(a.y.min(b.y))..=key(a.y.max(b.y)) {
                    squares.entry((x, y)).or_default().push((a, b));
                }
            }
        }
        Self { size, squares }
    }

    /// The distance from `at` to the nearest segment, looking no further than `reach`.
    #[expect(clippy::cast_possible_truncation, reason = "grid squares")]
    fn distance(&self, at: Point, reach: f64) -> f64 {
        let key = |v: f64| (v / self.size).floor() as i64;
        let mut best = f64::INFINITY;
        for x in key(at.x - reach)..=key(at.x + reach) {
            for y in key(at.y - reach)..=key(at.y + reach) {
                for &(a, b) in self.squares.get(&(x, y)).into_iter().flatten() {
                    let d = b - a;
                    let s = ((at - a).dot(d) / d.length_squared().max(1e-12)).clamp(0.0, 1.0);
                    best = best.min((a + d * s).distance_squared(at));
                }
            }
        }
        if best.is_finite() { best.sqrt() } else { reach }
    }
}

/// A closed outline with its corners cut twice over (Chaikin).
fn rounded_ring(ring: &[Point]) -> Vec<Point> {
    let mut out = ring.to_vec();
    for _ in 0..2 {
        if out.len() < 3 {
            break;
        }
        out = (0..out.len())
            .flat_map(|i| {
                let (a, b) = (out[i], out[(i + 1) % out.len()]);
                [a.lerp(b, 0.25), a.lerp(b, 0.75)]
            })
            .collect();
    }
    out
}

/// `line` with its corners cut three times over (Chaikin), so the distance to it has no
/// creases where its points were.
fn rounded(line: &[Point]) -> Vec<Point> {
    let mut out = line.to_vec();
    for _ in 0..3 {
        if out.len() < 3 {
            break;
        }
        let mut next = vec![out[0]];
        for w in out.windows(2) {
            next.push(w[0].lerp(w[1], 0.25));
            next.push(w[0].lerp(w[1], 0.75));
        }
        next.push(out[out.len() - 1]);
        out = next;
    }
    out
}

/// Points along `line` no further apart than `step`, its ends kept.
fn resample(line: &[Point], step: f64) -> Vec<Point> {
    let Some(&first) = line.first() else {
        return Vec::new();
    };
    let mut out = vec![first];
    let mut last = first;
    for &p in &line[1..] {
        if p.distance(last) >= step {
            out.push(p);
            last = p;
        }
    }
    if let (Some(&end), Some(&kept)) = (line.last(), out.last())
        && end != kept
    {
        out.push(end);
    }
    out
}

/// Each raised cell the mean of the raised cells round it, taking off the steps of the
/// cell-by-cell distances.
fn soften(grid: &mut Grid) {
    let (w, h) = (grid.columns, grid.rows);
    let old = grid.heights.clone();
    for r in 0..h {
        for c in 0..w {
            if old[r * w + c].is_nan() {
                continue;
            }
            let (mut sum, mut count) = (0.0, 0.0);
            for y in r.saturating_sub(1)..(r + 2).min(h) {
                for x in c.saturating_sub(1)..(c + 2).min(w) {
                    let v = old[y * w + x];
                    if !v.is_nan() {
                        sum += v;
                        count += 1.0;
                    }
                }
            }
            grid.heights[r * w + c] = sum / count;
        }
    }
}

/// The mean height (nothing raised counting as 0) within a square `reach` mm round each cell.
#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "cell counts are small"
)]
fn blur(grid: &Grid, reach: f64) -> Vec<f64> {
    let (w, h) = (grid.columns, grid.rows);
    let k = ((reach / grid.cell).round() as usize).max(1);
    let mut sum = vec![0.0; (w + 1) * (h + 1)];
    for r in 0..h {
        for c in 0..w {
            let v = grid.heights[r * w + c];
            let v = if v.is_nan() { 0.0 } else { v };
            let at = (r + 1) * (w + 1) + c + 1;
            sum[at] = v + sum[at - 1] + sum[at - w - 1] - sum[at - w - 2];
        }
    }
    let mut out = vec![0.0; w * h];
    for r in 0..h {
        for c in 0..w {
            let (ra, ca) = (r.saturating_sub(k), c.saturating_sub(k));
            let (rb, cb) = ((r + k + 1).min(h), (c + k + 1).min(w));
            let total = sum[rb * (w + 1) + cb] - sum[ra * (w + 1) + cb] - sum[rb * (w + 1) + ca]
                + sum[ra * (w + 1) + ca];
            #[expect(clippy::cast_precision_loss, reason = "cell counts are small")]
            let count = ((rb - ra) * (cb - ca)) as f64;
            out[r * w + c] = total / count;
        }
    }
    out
}

/// Every hollow narrower than a bit of `radius` filled (a closing by its flat disc: the
/// highest point within the radius of each cell, then the lowest of those within the radius
/// again). A slot or crease the bit cannot get into comes up to where the bit stops, and
/// what it can reach, the walls and floors round the pieces, stays as it is. Cells left
/// unraised stay so unless they are filled.
#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_possible_wrap,
    clippy::cast_precision_loss,
    clippy::many_single_char_names,
    reason = "cell counts are small; rows, columns and cells as on paper"
)]
pub fn close(grid: &mut Grid, radius: f64) {
    let k = (radius / grid.cell).floor() as i64;
    // each row of the disc: how far it reaches either side
    let spans: Vec<(i64, i64)> = (-k..=k)
        .map(|dy| {
            let half = ((radius / grid.cell).powi(2) - (dy * dy) as f64)
                .max(0.0)
                .sqrt();
            (dy, half.floor() as i64)
        })
        .collect();
    let (w, h) = (grid.columns as i64, grid.rows as i64);
    let floor: Vec<f64> = grid
        .heights
        .iter()
        .map(|&v| if v.is_nan() { 0.0 } else { v })
        .collect();
    let pass = |from: &[f64], most: bool| -> Vec<f64> {
        let mut out = vec![0.0; from.len()];
        for r in 0..h {
            for c in 0..w {
                let mut best = if most {
                    f64::NEG_INFINITY
                } else {
                    f64::INFINITY
                };
                for &(dy, half) in &spans {
                    let y = r + dy;
                    if !(0..h).contains(&y) {
                        // past the grid is the floor
                        best = if most { best.max(0.0) } else { best.min(0.0) };
                        continue;
                    }
                    for x in (c - half)..=(c + half) {
                        let v = if (0..w).contains(&x) {
                            from[(y * w + x) as usize]
                        } else {
                            0.0
                        };
                        best = if most { best.max(v) } else { best.min(v) };
                    }
                }
                out[(r * w + c) as usize] = best;
            }
        }
        out
    };
    let closed = pass(&pass(&floor, true), false);
    for (i, cut) in closed.into_iter().enumerate() {
        let cut = cut.max(floor[i]);
        if !grid.heights[i].is_nan() || cut > 0.0 {
            grid.heights[i] = cut;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn distances_count_cells_to_the_nearest_outside_one() {
        // a 5 by 5 square inside a 7 by 7 grid
        let inside: Vec<bool> = (0..49)
            .map(|i| (1..6).contains(&(i % 7)) && (1..6).contains(&(i / 7)))
            .collect();
        let d = distances(&inside, 7, 7);
        assert!(d[0].abs() < 1e-9);
        assert!((d[7 + 1] - 1.0).abs() < 1e-9);
        assert!((d[3 * 7 + 3] - 9.0).abs() < 1e-9);
    }

    #[test]
    fn a_hollow_narrower_than_the_ball_is_filled() {
        // two 5 mm walls 1 mm apart; a 3 mm ball cannot get between them
        let area = Rect::new(Point::ZERO, Point::new(20.0, 10.0));
        let mut grid = Grid::new(area, 0.25);
        for r in 0..grid.rows {
            for c in 0..grid.columns {
                let x = grid.centre(c, r).x;
                grid.heights[r * grid.columns + c] =
                    if (9.5..10.5).contains(&x) { 0.0 } else { 5.0 };
            }
        }
        close(&mut grid, 1.5);
        let mid = grid.rows / 2 * grid.columns + grid.columns / 2;
        assert!(grid.heights[mid] > 4.0, "{}", grid.heights[mid]);
    }

    #[test]
    fn a_broad_hollow_keeps_its_floor() {
        let area = Rect::new(Point::ZERO, Point::new(20.0, 10.0));
        let mut grid = Grid::new(area, 0.25);
        for r in 0..grid.rows {
            for c in 0..grid.columns {
                let x = grid.centre(c, r).x;
                grid.heights[r * grid.columns + c] =
                    if (5.0..15.0).contains(&x) { 0.0 } else { 5.0 };
            }
        }
        close(&mut grid, 1.5);
        let mid = grid.rows / 2 * grid.columns + grid.columns / 2;
        assert!(grid.heights[mid].abs() < 1e-9);
    }
}
