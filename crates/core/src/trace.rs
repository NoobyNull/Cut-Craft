//! Tracing: outlines of the filled cells of a grid (bitmap pixels, component cells), and
//! fitting lines, arcs or Béziers to points within a tolerance. Trace Bitmap, Vector Boundary
//! and Fit Curves are built on these.

use crate::geometry::{Path, Point, Segment};
use std::collections::HashMap;

/// A grid of square cells: `columns` × `rows`, row 0 at the bottom, its corner at `origin`.
#[derive(Clone, Copy, Debug)]
pub struct Grid {
    pub columns: usize,
    pub rows: usize,
    pub origin: Point,
    pub cell: f64,
}

/// The outlines of the cells `inside` says are filled, along the cell edges: closed loops,
/// each region's outside anticlockwise and its holes clockwise. Cells touching only at a
/// corner are separate regions.
#[expect(
    clippy::cast_possible_wrap,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_precision_loss,
    reason = "grid sizes are far below i64 and f64 limits, and checked non-negative"
)]
pub fn outlines(grid: Grid, inside: impl Fn(usize, usize) -> bool) -> Vec<Vec<Point>> {
    let filled = |c: i64, r: i64| {
        c >= 0
            && r >= 0
            && (c as usize) < grid.columns
            && (r as usize) < grid.rows
            && inside(c as usize, r as usize)
    };
    // directed edges with the filled cell on their left, keyed by where they start
    let mut from: HashMap<(i64, i64), Vec<(i64, i64)>> = HashMap::new();
    for r in 0..grid.rows as i64 {
        for c in 0..grid.columns as i64 {
            if !filled(c, r) {
                continue;
            }
            let sides = [
                ((c, r), (c + 1, r), (c, r - 1)),
                ((c + 1, r), (c + 1, r + 1), (c + 1, r)),
                ((c + 1, r + 1), (c, r + 1), (c, r + 1)),
                ((c, r + 1), (c, r), (c - 1, r)),
            ];
            for (a, b, neighbour) in sides {
                if !filled(neighbour.0, neighbour.1) {
                    from.entry(a).or_default().push(b);
                }
            }
        }
    }
    let mut loops = vec![];
    while let Some(&start) = from.keys().next() {
        let mut ring = vec![start];
        let (mut prev, mut at) = (start, start);
        while let Some(nexts) = from.get_mut(&at) {
            // at a corner where two regions touch, turn left (round the filled cell) so they
            // stay apart
            let heading = (at.0 - prev.0, at.1 - prev.1);
            let turn = |n: &(i64, i64)| {
                let d = (n.0 - at.0, n.1 - at.1);
                heading.0 * d.1 - heading.1 * d.0
            };
            let pick = (0..nexts.len())
                .max_by_key(|&i| turn(&nexts[i]))
                .unwrap_or(0);
            let next = nexts.swap_remove(pick);
            if nexts.is_empty() {
                from.remove(&at);
            }
            (prev, at) = (at, next);
            if at == start {
                break;
            }
            ring.push(at);
        }
        loops.push(ring);
    }
    loops
        .into_iter()
        .map(|ring| {
            corners_only(&ring)
                .into_iter()
                .map(|(c, r)| grid.origin + Point::new(c as f64, r as f64) * grid.cell)
                .collect()
        })
        .collect()
}

/// The ring without the points in the middle of straight runs.
fn corners_only(ring: &[(i64, i64)]) -> Vec<(i64, i64)> {
    let n = ring.len();
    (0..n)
        .filter(|&i| {
            let (a, b, c) = (ring[(i + n - 1) % n], ring[i], ring[(i + 1) % n]);
            (b.0 - a.0) * (c.1 - b.1) != (b.1 - a.1) * (c.0 - b.0)
        })
        .map(|i| ring[i])
        .collect()
}

/// Twice the signed area of a closed ring: positive anticlockwise.
pub fn ring_area(ring: &[Point]) -> f64 {
    crate::geometry::winding(ring) / 2.0
}

/// Distance from `p` to the segment `a`–`b`.
fn to_segment(p: Point, a: Point, b: Point) -> f64 {
    let ab = b - a;
    let t = if ab.length_squared() > 0.0 {
        ((p - a).dot(ab) / ab.length_squared()).clamp(0.0, 1.0)
    } else {
        0.0
    };
    p.distance(a + ab * t)
}

/// Indices of the points of the open run `points` kept by Douglas–Peucker: every point left
/// out is within `tolerance` of the line between the kept ones round it.
fn kept(points: &[Point], tolerance: f64) -> Vec<usize> {
    let mut keep = vec![false; points.len()];
    let last = points.len() - 1;
    keep[0] = true;
    keep[last] = true;
    let mut spans = vec![(0, last)];
    while let Some((a, b)) = spans.pop() {
        let far = (a + 1..b)
            .map(|i| (i, to_segment(points[i], points[a], points[b])))
            .max_by(|x, y| x.1.total_cmp(&y.1));
        if let Some((i, d)) = far
            && d > tolerance
        {
            keep[i] = true;
            spans.push((a, i));
            spans.push((i, b));
        }
    }
    (0..points.len()).filter(|&i| keep[i]).collect()
}

/// The closed ring as fewer points, each left out within `tolerance` of the outline.
pub fn simplify(ring: &[Point], tolerance: f64) -> Vec<Point> {
    if ring.len() < 4 {
        return ring.to_vec();
    }
    // split at the point farthest from the first so both halves are open runs
    let far = (1..ring.len())
        .max_by(|&a, &b| {
            ring[a]
                .distance(ring[0])
                .total_cmp(&ring[b].distance(ring[0]))
        })
        .unwrap_or(1);
    let first: Vec<Point> = ring[..=far].to_vec();
    let mut second: Vec<Point> = ring[far..].to_vec();
    second.push(ring[0]);
    let mut out: Vec<Point> = kept(&first, tolerance)
        .into_iter()
        .map(|i| first[i])
        .collect();
    out.pop();
    out.extend(kept(&second, tolerance).into_iter().map(|i| second[i]));
    out.pop();
    out
}

/// What Fit Curves puts through the points.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Fit {
    Lines,
    Arcs,
    Beziers,
}

/// A path through `points` (a closed ring, or an open run) within `tolerance`: straight
/// lines, circular arcs or cubic Béziers. Corners turning more than `corner` radians stay
/// sharp; the curves are smooth across the others; an open run's ends stay where they are.
pub fn fit(points: &[Point], closed: bool, fit: Fit, tolerance: f64, corner: f64) -> Path {
    if !closed {
        return fit_open(points, fit, tolerance, corner);
    }
    let ring = points;
    let simple = simplify(ring, tolerance);
    if fit == Fit::Lines || simple.len() < 3 {
        return Path::polyline(&simple, true);
    }
    // corners, judged over a few tolerances either side so pixel steps are not corners,
    // split the ring into runs; each corner is the sharpest point of its neighbourhood
    let n = ring.len();
    let turn = |i: usize| {
        let ahead = direction_closed(ring, i, 1, tolerance);
        let behind = direction_closed(ring, i, -1, tolerance);
        (-behind).angle_to(ahead).abs()
    };
    let turns: Vec<f64> = (0..n).map(turn).collect();
    let mut at: Vec<usize> = (0..n)
        .filter(|&i| {
            turns[i] > corner && turns[i] >= turns[(i + n - 1) % n] && turns[i] > turns[(i + 1) % n]
        })
        .collect();
    if at.len() < 2 {
        // a smooth closed shape: a run must not end where it starts, so cut it in quarters
        at = (0..4).map(|q| q * n / 4).collect();
        at.dedup();
    }
    let start = ring[at[0]];
    let mut segments = vec![];
    for k in 0..at.len() {
        let (a, b) = (at[k], at[(k + 1) % at.len()]);
        let mut run: Vec<Point> = if a < b {
            ring[a..=b].to_vec()
        } else {
            ring[a..].iter().chain(&ring[..=b]).copied().collect()
        };
        if run.len() == 1 {
            run = ring[a..].iter().chain(&ring[..=a]).copied().collect();
        }
        match fit {
            Fit::Arcs => fit_arcs(&run, tolerance, &mut segments),
            _ => fit_cubics(&run, tolerance, &mut segments),
        }
    }
    Path {
        start,
        segments,
        closed: true,
    }
}

/// [`fit`] for an open run: its ends and corners split it into runs.
fn fit_open(points: &[Point], fit: Fit, tolerance: f64, corner: f64) -> Path {
    if points.len() < 3 || fit == Fit::Lines {
        let kept = if points.len() < 3 {
            (0..points.len()).collect()
        } else {
            kept(points, tolerance)
        };
        let line: Vec<Point> = kept.into_iter().map(|i| points[i]).collect();
        return Path::polyline(&line, false);
    }
    let n = points.len();
    let mut at = vec![0];
    for i in 1..n - 1 {
        let turn = (-direction(points, i, -1, tolerance))
            .angle_to(direction(points, i, 1, tolerance))
            .abs();
        if turn > corner {
            at.push(i);
        }
    }
    at.push(n - 1);
    let mut segments = vec![];
    for w in at.windows(2) {
        let run = &points[w[0]..=w[1]];
        if run.len() < 2 {
            continue;
        }
        match fit {
            Fit::Arcs => fit_arcs(run, tolerance, &mut segments),
            _ => fit_cubics(run, tolerance, &mut segments),
        }
    }
    Path {
        start: points[0],
        segments,
        closed: false,
    }
}

/// The circle through three points, or `None` when they are in a line.
fn circle_through(a: Point, b: Point, c: Point) -> Option<(Point, f64)> {
    let d = 2.0 * (a.x * (b.y - c.y) + b.x * (c.y - a.y) + c.x * (a.y - b.y));
    if d.abs() < 1e-12 {
        return None;
    }
    let sq = |p: Point| p.length_squared();
    let centre = Point::new(
        (sq(a) * (b.y - c.y) + sq(b) * (c.y - a.y) + sq(c) * (a.y - b.y)) / d,
        (sq(a) * (c.x - b.x) + sq(b) * (a.x - c.x) + sq(c) * (b.x - a.x)) / d,
    );
    Some((centre, centre.distance(a)))
}

/// Arcs (or lines) through the open run `points`, each as long as stays within `tolerance`.
fn fit_arcs(points: &[Point], tolerance: f64, out: &mut Vec<Segment>) {
    let points = &smoothed(points);
    let mut from = 0;
    while from + 1 < points.len() {
        // the longest run from `from` a single arc (or line) holds within the tolerance
        let mut best = (from + 1, None);
        for to in from + 2..points.len() {
            let ends = (points[from], points[usize::midpoint(from, to)], points[to]);
            let circle = circle_through(ends.0, ends.1, ends.2);
            let holds = |p: &Point| match circle {
                Some((centre, radius)) => (p.distance(centre) - radius).abs() <= tolerance,
                None => to_segment(*p, ends.0, ends.2) <= tolerance,
            };
            if !points[from..=to].iter().all(holds) {
                break;
            }
            best = (to, circle);
        }
        let (to, circle) = best;
        let (start, end) = (points[from], points[to]);
        match circle {
            // a huge circle is as good as a line
            Some((centre, radius)) if radius < 1e3 * start.distance(end).max(tolerance) => {
                let middle = points[usize::midpoint(from, to)];
                let ccw = (middle - start).perp_dot(end - middle) > 0.0;
                out.push(Segment::Arc {
                    to: end,
                    center: centre,
                    ccw,
                });
            }
            _ => out.push(Segment::Line { to: end }),
        }
        from = to;
    }
}

/// One light smoothing pass to take the edge off pixel steps; the ends stay put.
fn smoothed(points: &[Point]) -> Vec<Point> {
    let mut smooth = points.to_vec();
    for i in 1..points.len().saturating_sub(1) {
        smooth[i] = (points[i - 1] + points[i] * 2.0 + points[i + 1]) / 4.0;
    }
    smooth
}

/// Cubic Béziers through the open run `points` within `tolerance` (Schneider's method:
/// chord-length parameters, least-squares control points, split at the worst point).
fn fit_cubics(points: &[Point], tolerance: f64, out: &mut Vec<Segment>) {
    if points.len() < 2 {
        return;
    }
    let smooth = smoothed(points);
    let start = direction(&smooth, 0, 1, tolerance);
    let end = direction(&smooth, smooth.len() - 1, -1, tolerance);
    cubic_run(&smooth, start, end, tolerance, out, 0);
}

/// As [`direction`], round a closed ring.
fn direction_closed(ring: &[Point], at: usize, step: isize, tolerance: f64) -> Point {
    let n = ring.len();
    let reach = 4.0 * tolerance;
    let mut i = at;
    for _ in 1..n {
        i = if step > 0 {
            (i + 1) % n
        } else {
            (i + n - 1) % n
        };
        if ring[i].distance(ring[at]) >= reach {
            break;
        }
    }
    (ring[i] - ring[at]).normalize_or_zero()
}

/// The way the points run from `at` in the direction `step`, measured to the first point a
/// few tolerances away so pixel steps do not decide it.
fn direction(points: &[Point], at: usize, step: isize, tolerance: f64) -> Point {
    let reach = 4.0 * tolerance;
    let mut i = at;
    while let Some(next) = i.checked_add_signed(step).filter(|&n| n < points.len()) {
        i = next;
        if points[i].distance(points[at]) >= reach {
            break;
        }
    }
    (points[i] - points[at]).normalize_or_zero()
}

fn cubic_run(
    points: &[Point],
    t1: Point,
    t2: Point,
    tolerance: f64,
    out: &mut Vec<Segment>,
    depth: u32,
) {
    let (first, last) = (points[0], points[points.len() - 1]);
    if points.len() == 2 || depth > 12 {
        let third = first.distance(last) / 3.0;
        out.push(Segment::Cubic {
            c1: first + t1 * third,
            c2: last + t2 * third,
            to: last,
        });
        return;
    }
    // chord-length parameters
    let mut u = vec![0.0];
    for w in points.windows(2) {
        u.push(u[u.len() - 1] + w[0].distance(w[1]));
    }
    let total = u[u.len() - 1].max(f64::EPSILON);
    for v in &mut u {
        *v /= total;
    }
    // fit, then move each point's parameter to its nearest place on the curve and refit
    let mut fitted = control_points(points, &u, t1, t2);
    for _ in 0..4 {
        let (c1, c2) = fitted;
        for (t, p) in u.iter_mut().zip(points) {
            *t = nearer(*t, *p, [first, c1, c2, last]);
        }
        fitted = control_points(points, &u, t1, t2);
    }
    let (c1, c2) = fitted;
    let at = |t: f64| bezier([first, c1, c2, last], t);
    let (worst, error) = (1..points.len() - 1)
        .map(|i| (i, at(u[i]).distance(points[i])))
        .max_by(|a, b| a.1.total_cmp(&b.1))
        .unwrap_or((points.len() / 2, 0.0));
    if error <= tolerance {
        out.push(Segment::Cubic { c1, c2, to: last });
        return;
    }
    // split at the worst point, smooth across it
    let middle = (direction(points, worst, 1, tolerance) - direction(points, worst, -1, tolerance))
        .normalize_or_zero();
    cubic_run(&points[..=worst], t1, -middle, tolerance, out, depth + 1);
    cubic_run(&points[worst..], middle, t2, tolerance, out, depth + 1);
}

/// The point at `t` on the cubic Bézier with control points `c`.
fn bezier(c: [Point; 4], t: f64) -> Point {
    let s = 1.0 - t;
    c[0] * (s * s * s) + c[1] * (3.0 * s * s * t) + c[2] * (3.0 * s * t * t) + c[3] * (t * t * t)
}

/// One Newton step moving `t` towards the point on the curve nearest `p`.
fn nearer(t: f64, p: Point, c: [Point; 4]) -> f64 {
    let s = 1.0 - t;
    let d1 = (c[1] - c[0]) * (3.0 * s * s)
        + (c[2] - c[1]) * (6.0 * s * t)
        + (c[3] - c[2]) * (3.0 * t * t);
    let d2 = (c[2] - c[1] * 2.0 + c[0]) * (6.0 * s) + (c[3] - c[2] * 2.0 + c[1]) * (6.0 * t);
    let off = bezier(c, t) - p;
    let denominator = d1.dot(d1) + off.dot(d2);
    if denominator.abs() < 1e-12 {
        return t;
    }
    (t - off.dot(d1) / denominator).clamp(0.0, 1.0)
}

/// Least-squares inner control points along the end tangents (Graphics Gems, Schneider 1990).
fn control_points(points: &[Point], u: &[f64], t1: Point, t2: Point) -> (Point, Point) {
    let (first, last) = (points[0], points[points.len() - 1]);
    let (mut c00, mut c01, mut c11, mut x0, mut x1) = (0.0, 0.0, 0.0, 0.0, 0.0);
    for (p, &t) in points.iter().zip(u) {
        let s = 1.0 - t;
        let (b0, b1, b2, b3) = (s * s * s, 3.0 * s * s * t, 3.0 * s * t * t, t * t * t);
        let (a1, a2) = (t1 * b1, t2 * b2);
        c00 += a1.dot(a1);
        c01 += a1.dot(a2);
        c11 += a2.dot(a2);
        let rest = *p - (first * (b0 + b1) + last * (b2 + b3));
        x0 += a1.dot(rest);
        x1 += a2.dot(rest);
    }
    let det = c00 * c11 - c01 * c01;
    let chord = first.distance(last);
    let (mut alpha1, mut alpha2) = if det.abs() > 1e-12 {
        ((x0 * c11 - c01 * x1) / det, (c00 * x1 - c01 * x0) / det)
    } else {
        (chord / 3.0, chord / 3.0)
    };
    // a degenerate solution falls back to a third of the chord
    if alpha1 < chord * 1e-6 || alpha2 < chord * 1e-6 {
        (alpha1, alpha2) = (chord / 3.0, chord / 3.0);
    }
    (first + t1 * alpha1, last + t2 * alpha2)
}

#[cfg(test)]
#[expect(
    clippy::cast_precision_loss,
    clippy::cast_lossless,
    reason = "small test grids"
)]
mod tests {
    use super::*;

    fn grid(columns: usize, rows: usize) -> Grid {
        Grid {
            columns,
            rows,
            origin: Point::ZERO,
            cell: 1.0,
        }
    }

    #[test]
    fn a_square_of_cells_is_one_anticlockwise_square() {
        let loops = outlines(grid(5, 5), |c, r| {
            (1..4).contains(&c) && (1..4).contains(&r)
        });
        assert_eq!(loops.len(), 1);
        assert_eq!(loops[0].len(), 4, "corners only: {:?}", loops[0]);
        assert!((ring_area(&loops[0]) - 9.0).abs() < 1e-12);
    }

    #[test]
    fn a_ring_has_an_outside_and_a_clockwise_hole() {
        let loops = outlines(grid(5, 5), |c, r| !(c == 2 && r == 2));
        assert_eq!(loops.len(), 2);
        let mut areas: Vec<f64> = loops.iter().map(|l| ring_area(l)).collect();
        areas.sort_by(f64::total_cmp);
        assert_eq!(areas, [-1.0, 25.0]);
    }

    #[test]
    fn cells_touching_at_a_corner_are_apart() {
        let loops = outlines(grid(2, 2), |c, r| c == r);
        assert_eq!(loops.len(), 2);
    }

    #[test]
    fn an_open_curve_keeps_its_ends_and_fits_closely() {
        let arc: Vec<Point> = (0..=60)
            .map(|i| {
                let a = f64::from(i) / 60.0 * std::f64::consts::PI;
                Point::new(10.0 * a.cos(), 10.0 * a.sin())
            })
            .collect();
        for kind in [Fit::Beziers, Fit::Arcs, Fit::Lines] {
            let path = fit(&arc, false, kind, 0.05, 1.0);
            assert!(!path.closed);
            assert_eq!(path.start, arc[0]);
            let end = path.flatten(0.01).last().copied().unwrap();
            assert!(end.distance(arc[60]) < 1e-9, "{kind:?}");
            assert!(
                path.segments.len() < 30,
                "{kind:?}: {}",
                path.segments.len()
            );
        }
    }

    #[test]
    fn lines_keep_the_shape_within_the_tolerance() {
        // a staircase diagonal simplifies to its ends
        let stairs: Vec<Point> = (0..10)
            .flat_map(|i| {
                [
                    Point::new(i as f64, i as f64),
                    Point::new(i as f64 + 1.0, i as f64),
                ]
            })
            .chain([Point::new(10.0, 10.0), Point::new(0.0, 10.0)])
            .collect();
        let simple = simplify(&stairs, 1.0);
        assert!(simple.len() <= 4, "{simple:?}");
    }

    #[test]
    fn a_traced_circle_fits_to_a_few_smooth_curves_close_to_it() {
        // a disc of radius 20 in 1 mm cells
        let loops = outlines(grid(50, 50), |c, r| {
            Point::new(c as f64 + 0.5, r as f64 + 0.5).distance(Point::splat(25.0)) < 20.0
        });
        for kind in [Fit::Beziers, Fit::Arcs] {
            let path = fit(&loops[0], true, kind, 0.75, 1.2);
            assert!(
                path.segments.len() < 40,
                "{kind:?}: {}",
                path.segments.len()
            );
            for p in path.flatten(0.05) {
                let r = p.distance(Point::splat(25.0));
                assert!(
                    (r - 20.0).abs() < 1.6,
                    "{kind:?}: off the circle at {p:?} ({r})"
                );
            }
        }
    }
}
