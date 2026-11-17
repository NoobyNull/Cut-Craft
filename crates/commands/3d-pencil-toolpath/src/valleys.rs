//! Finding the valleys a pencil pass runs along, on a grid of tool tip heights: where the
//! surface the bit's tip can reach bends up on both sides (a crease), thinned to lines one
//! cell wide and traced into polylines. Cut Craft's own method.

use cut_craft_core::geometry::Point;

/// A grid of values, row by row from the bottom, cells `cell` across, the first one's centre
/// half a cell in from `origin`.
#[derive(Clone, Copy, Debug)]
pub struct Grid<'a> {
    pub values: &'a [f64],
    pub columns: usize,
    pub rows: usize,
    pub cell: f64,
    pub origin: Point,
}

impl Grid<'_> {
    fn at(&self, c: isize, r: isize) -> Option<f64> {
        let (columns, rows) = (self.columns.cast_signed(), self.rows.cast_signed());
        ((0..columns).contains(&c) && (0..rows).contains(&r))
            .then(|| self.values[r.cast_unsigned() * self.columns + c.cast_unsigned()])
    }

    /// The centre of cell `i`.
    #[expect(clippy::cast_precision_loss, reason = "small cell counts")]
    pub fn centre(&self, i: usize) -> Point {
        let (c, r) = (i % self.columns, i / self.columns);
        self.origin + Point::new(c as f64 + 0.5, r as f64 + 0.5) * self.cell
    }
}

/// The four directions across a cell a valley is looked for in: along a row, a column and
/// both diagonals.
const ACROSS: [(isize, isize); 4] = [(1, 0), (0, 1), (1, 1), (1, -1)];

/// How sharply the tip surface bends up through cell (`c`, `r`) going `d`, in radians, and
/// how much it dips there (the second difference, mm). The bend is the larger of the slopes
/// either side of the cell against each other, and the slopes a cell further out (so a
/// crease between two cell centres counts in full). `None` near the edge.
fn bend(tips: &Grid, c: isize, r: isize, d: (isize, isize)) -> Option<(f64, f64)> {
    let step = |k: isize| tips.at(c + k * d.0, r + k * d.1);
    let (b2, b1, here, a1, a2) = (step(-2)?, step(-1)?, step(0)?, step(1)?, step(2)?);
    #[expect(clippy::cast_precision_loss, reason = "a step of one or two cells")]
    let length = tips.cell * ((d.0 * d.0 + d.1 * d.1) as f64).sqrt();
    let slope = |from: f64, to: f64| ((to - from) / length).atan();
    let near = slope(here, a1) - slope(b1, here);
    let wide = slope(a1, a2) - slope(b2, b1);
    Some((near.max(wide), a1 + b1 - 2.0 * here))
}

/// The cells a valley runs through: where the tip surface bends up on both sides by at least
/// `sharpest` radians, at the bottom of the bend (the cell that dips most across it). Cells
/// `keep` says no to are left out. Two cells wide where a valley runs between cell centres;
/// [`thin`] makes that one.
pub fn creases(tips: &Grid, sharpest: f64, keep: &dyn Fn(usize) -> bool) -> Vec<bool> {
    let mut found = vec![false; tips.values.len()];
    for (i, cell) in found.iter_mut().enumerate() {
        if !keep(i) {
            continue;
        }
        let (c, r) = (
            (i % tips.columns).cast_signed(),
            (i / tips.columns).cast_signed(),
        );
        // the direction it dips most in, for its length
        let best = ACROSS
            .iter()
            .filter_map(|&d| {
                let (angle, dip) = bend(tips, c, r, d)?;
                let scale = (d.0 * d.0 + d.1 * d.1).cast_unsigned();
                #[expect(clippy::cast_precision_loss, reason = "1 or 2")]
                Some((d, angle, dip / scale as f64))
            })
            .max_by(|a, b| a.2.total_cmp(&b.2));
        let Some((d, angle, dip)) = best else {
            continue;
        };
        if angle < sharpest || dip <= 0.0 {
            continue;
        }
        // the bottom of the bend: no neighbour across it dips more
        let neighbour = |k: isize| {
            bend(tips, c + k * d.0, r + k * d.1, d).map_or(f64::NEG_INFINITY, |(_, dip)| {
                let scale = (d.0 * d.0 + d.1 * d.1).cast_unsigned();
                #[expect(clippy::cast_precision_loss, reason = "1 or 2")]
                let dip = dip / scale as f64;
                dip
            })
        };
        *cell = dip >= neighbour(-1) && dip >= neighbour(1);
    }
    found
}

/// The eight cells round cell `i` that are in `mask`, as indices, with diagonals left out
/// where a side neighbour already joins them (so a staircase is a line, not a tangle).
fn joined(mask: &[bool], columns: usize, rows: usize, i: usize) -> Vec<usize> {
    let (c, r) = ((i % columns).cast_signed(), (i / columns).cast_signed());
    let set = |dc: isize, dr: isize| {
        let (cc, rr) = (c + dc, r + dr);
        ((0..columns.cast_signed()).contains(&cc) && (0..rows.cast_signed()).contains(&rr))
            .then(|| rr.cast_unsigned() * columns + cc.cast_unsigned())
            .filter(|&j| mask[j])
    };
    let mut out = vec![];
    for (dc, dr) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
        out.extend(set(dc, dr));
    }
    for (dc, dr) in [(1, 1), (1, -1), (-1, 1), (-1, -1)] {
        if set(dc, 0).is_none() && set(0, dr).is_none() {
            out.extend(set(dc, dr));
        }
    }
    out
}

/// `mask` thinned to lines one cell wide, keeping their ends and how they join
/// (Zhang and Suen's thinning).
pub fn thin(mask: &[bool], columns: usize, rows: usize) -> Vec<bool> {
    let mut mask = mask.to_vec();
    let at = |m: &[bool], c: isize, r: isize| {
        (0..columns.cast_signed()).contains(&c)
            && (0..rows.cast_signed()).contains(&r)
            && m[r.cast_unsigned() * columns + c.cast_unsigned()]
    };
    loop {
        let mut changed = false;
        for pass in 0..2 {
            let mut gone = vec![];
            for i in (0..mask.len()).filter(|&i| mask[i]) {
                let (c, r) = ((i % columns).cast_signed(), (i / columns).cast_signed());
                // round from above, clockwise: P2..P9
                let p = [
                    at(&mask, c, r + 1),
                    at(&mask, c + 1, r + 1),
                    at(&mask, c + 1, r),
                    at(&mask, c + 1, r - 1),
                    at(&mask, c, r - 1),
                    at(&mask, c - 1, r - 1),
                    at(&mask, c - 1, r),
                    at(&mask, c - 1, r + 1),
                ];
                let neighbours = p.iter().filter(|&&b| b).count();
                let turns = (0..8).filter(|&k| !p[k] && p[(k + 1) % 8]).count();
                let (north, east, south, west) = (p[0], p[2], p[4], p[6]);
                // the first pass takes cells off the south-east, the second the north-west
                let side = if pass == 0 {
                    !(east && south && (north || west))
                } else {
                    !(north && west && (east || south))
                };
                if (2..=6).contains(&neighbours) && turns == 1 && side {
                    gone.push(i);
                }
            }
            changed |= !gone.is_empty();
            for i in gone {
                mask[i] = false;
            }
        }
        if !changed {
            return mask;
        }
    }
}

/// A line of cells: their indices in order, and whether it closes on itself.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Chain {
    pub cells: Vec<usize>,
    pub closed: bool,
}

/// The lines one cell wide in `mask` (after [`thin`]), each traced end to end: from one end
/// or junction to the next, and rings with neither as closed loops.
pub fn chains(mask: &[bool], columns: usize, rows: usize) -> Vec<Chain> {
    let links: Vec<Vec<usize>> = (0..mask.len())
        .map(|i| {
            if mask[i] {
                joined(mask, columns, rows, i)
            } else {
                vec![]
            }
        })
        .collect();
    let mut walked = std::collections::HashSet::new();
    let edge = |a: usize, b: usize| (a.min(b), a.max(b));
    let mut out = vec![];
    let walk = |start: usize, first: usize, walked: &mut std::collections::HashSet<_>| {
        let mut cells = vec![start, first];
        walked.insert(edge(start, first));
        let mut here = first;
        while links[here].len() == 2 && here != start {
            let Some(&next) = links[here]
                .iter()
                .find(|&&n| !walked.contains(&edge(here, n)))
            else {
                break;
            };
            walked.insert(edge(here, next));
            cells.push(next);
            here = next;
        }
        let closed = cells.len() > 2 && cells.first() == cells.last();
        if closed {
            cells.pop();
        }
        Chain { cells, closed }
    };
    // from every end and junction first, then what is left: rings
    for start in (0..mask.len()).filter(|&i| mask[i] && links[i].len() != 2) {
        for &first in &links[start] {
            if !walked.contains(&edge(start, first)) {
                out.push(walk(start, first, &mut walked));
            }
        }
    }
    for start in (0..mask.len()).filter(|&i| mask[i] && links[i].len() == 2) {
        if let Some(&first) = links[start]
            .iter()
            .find(|&&n| !walked.contains(&edge(start, n)))
        {
            out.push(walk(start, first, &mut walked));
        }
    }
    out
}

/// `points` smoothed: each moved towards the middle of its neighbours a few times, so a line
/// traced along cells loses its staircase. The ends of an open line stay put.
pub fn smoothed(points: &[Point], closed: bool) -> Vec<Point> {
    let mut points = points.to_vec();
    let n = points.len();
    if n < 3 {
        return points;
    }
    for _ in 0..3 {
        let before = points.clone();
        for i in 0..n {
            let (prev, next) = if closed {
                ((i + n - 1) % n, (i + 1) % n)
            } else if i == 0 || i == n - 1 {
                continue;
            } else {
                (i - 1, i + 1)
            };
            points[i] = before[prev] * 0.25 + before[i] * 0.5 + before[next] * 0.25;
        }
    }
    points
}

/// `points` moved `by` sideways, to the left going along them (negative: the right).
pub fn offset(points: &[Point], closed: bool, by: f64) -> Vec<Point> {
    let n = points.len();
    (0..n)
        .map(|i| {
            let (prev, next) = if closed {
                (points[(i + n - 1) % n], points[(i + 1) % n])
            } else {
                (points[i.saturating_sub(1)], points[(i + 1).min(n - 1)])
            };
            let along = (next - prev).normalize_or_zero();
            points[i] + along.perp() * by
        })
        .collect()
}

/// How long a polyline is end to end (round, when `closed`).
pub fn length(points: &[Point], closed: bool) -> f64 {
    let open: f64 = points.windows(2).map(|w| w[0].distance(w[1])).sum();
    match (closed, points.first(), points.last()) {
        (true, Some(a), Some(b)) => open + a.distance(*b),
        _ => open,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const CELL: f64 = 0.5;

    fn grid(values: &[f64], columns: usize) -> Grid<'_> {
        Grid {
            values,
            columns,
            rows: values.len() / columns,
            cell: CELL,
            origin: Point::ZERO,
        }
    }

    /// A surface over `columns` × `rows` cells, its height `f` of each centre.
    #[expect(clippy::cast_precision_loss, reason = "small grids")]
    fn surface(columns: usize, rows: usize, f: impl Fn(f64, f64) -> f64) -> Vec<f64> {
        (0..columns * rows)
            .map(|i| {
                let (c, r) = ((i % columns) as f64, (i / columns) as f64);
                f((c + 0.5) * CELL, (r + 0.5) * CELL)
            })
            .collect()
    }

    fn columns_of(found: &[bool], columns: usize) -> Vec<usize> {
        let mut out: Vec<usize> = (0..found.len())
            .filter(|&i| found[i])
            .map(|i| i % columns)
            .collect();
        out.sort_unstable();
        out.dedup();
        out
    }

    #[test]
    fn a_v_groove_is_a_valley_down_its_middle() {
        // a 90° groove along y, its bottom at x = 10.25 (the centre of column 20)
        let values = surface(41, 20, |x, _| (x - 10.25).abs());
        let tips = grid(&values, 41);
        let found = creases(&tips, 20f64.to_radians(), &|_| true);
        assert_eq!(columns_of(&found, 41), [20], "one line, at the bottom");
    }

    #[test]
    fn a_groove_between_cell_centres_is_found_and_thinned_to_one_line() {
        let values = surface(40, 20, |x, _| (x - 10.0).abs());
        let tips = grid(&values, 40);
        let found = creases(&tips, 20f64.to_radians(), &|_| true);
        assert_eq!(columns_of(&found, 40), [19, 20], "both cells either side");
        let thinned = thin(&found, 40, 20);
        let lines = chains(&thinned, 40, 20);
        assert_eq!(lines.len(), 1, "{lines:?}");
        assert!(lines[0].cells.len() >= 18, "the whole way along");
    }

    #[test]
    fn the_foot_of_a_wall_is_a_valley_but_its_top_is_not() {
        // a floor at 0 meeting a step up to 5 at x = 10 (a ball's tip can't get closer)
        let values = surface(40, 10, |x, _| if x < 10.0 { 5.0 } else { 0.0 });
        let tips = grid(&values, 40);
        let found = creases(&tips, 20f64.to_radians(), &|_| true);
        assert_eq!(
            columns_of(&found, 40),
            [20],
            "the floor cell against the wall"
        );
    }

    #[test]
    fn ridges_and_gentle_valleys_are_left_alone() {
        let ridge = surface(40, 10, |x, _| -(x - 10.25).abs());
        assert!(!creases(&grid(&ridge, 40), 0.35, &|_| true).contains(&true));
        // a wide bowl: bends 2° a cell
        let bowl = surface(40, 10, |x, _| (x - 10.25).powi(2) / 15.0);
        assert!(!creases(&grid(&bowl, 40), 0.35, &|_| true).contains(&true));
        // a gentle V: 10° each side, a 20° bend, under the 30° asked for
        let gentle = surface(40, 10, |x, _| (x - 10.25).abs() * 10f64.to_radians().tan());
        assert!(!creases(&grid(&gentle, 40), 30f64.to_radians(), &|_| true).contains(&true));
        assert!(creases(&grid(&gentle, 40), 15f64.to_radians(), &|_| true).contains(&true));
    }

    #[test]
    fn cells_not_kept_are_never_valleys() {
        let values = surface(41, 20, |x, _| (x - 10.25).abs());
        let found = creases(&grid(&values, 41), 0.35, &|i| i / 41 < 10);
        assert!((0..found.len()).filter(|&i| found[i]).all(|i| i / 41 < 10));
        assert!(found.contains(&true));
    }

    #[test]
    fn a_ring_traces_as_one_closed_line() {
        let (columns, rows) = (20, 20);
        let mut mask = vec![false; columns * rows];
        for c in 5..15 {
            mask[5 * columns + c] = true;
            mask[14 * columns + c] = true;
        }
        for r in 5..15 {
            mask[r * columns + 5] = true;
            mask[r * columns + 14] = true;
        }
        let lines = chains(&mask, columns, rows);
        assert_eq!(lines.len(), 1);
        assert!(lines[0].closed);
        assert_eq!(lines[0].cells.len(), 36, "every cell once");
    }

    #[test]
    fn a_junction_splits_lines_and_a_staircase_stays_one() {
        let (columns, rows) = (20, 20);
        let mut mask = vec![false; columns * rows];
        // a T: across row 10, and up column 10 from it
        for c in 2..18 {
            mask[10 * columns + c] = true;
        }
        for r in 11..18 {
            mask[r * columns + 10] = true;
        }
        assert_eq!(chains(&mask, columns, rows).len(), 3);
        // a staircase: right one, up one
        let mut stairs = vec![false; columns * rows];
        for k in 0..8 {
            stairs[(2 + k) * columns + 2 + k] = true;
            stairs[(2 + k) * columns + 3 + k] = true;
        }
        let lines = chains(&stairs, columns, rows);
        assert_eq!(lines.len(), 1, "{lines:?}");
        assert_eq!(lines[0].cells.len(), 16);
    }

    #[test]
    fn offsets_go_sideways_by_the_distance() {
        let line = [
            Point::new(0.0, 0.0),
            Point::new(5.0, 0.0),
            Point::new(10.0, 0.0),
        ];
        let left = offset(&line, false, 2.0);
        assert!(left.iter().all(|p| (p.y - 2.0).abs() < 1e-9));
        let right = offset(&line, false, -2.0);
        assert!(right.iter().all(|p| (p.y + 2.0).abs() < 1e-9));
        assert!((length(&line, false) - 10.0).abs() < 1e-9);
    }

    #[test]
    fn smoothing_keeps_the_ends_and_takes_out_the_steps() {
        let steps: Vec<Point> = (0..10)
            .map(|i| Point::new(f64::from(i), f64::from(i / 2)))
            .collect();
        let smooth = smoothed(&steps, false);
        assert_eq!(smooth[0], steps[0]);
        assert_eq!(smooth[9], steps[9]);
        let wobble = |p: &[Point]| {
            p.windows(3)
                .map(|w| ((w[0] + w[2]) * 0.5 - w[1]).length())
                .fold(0.0, f64::max)
        };
        assert!(wobble(&smooth) < wobble(&steps));
    }
}
