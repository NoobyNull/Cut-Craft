//! Staying down between the cuts of one shape: the bit goes from where one cut ends to
//! where the next starts without lifting, raised only as far as keeps its sides off the
//! shape's edges, straight across where that stays inside the shape and otherwise along its
//! centre lines.

use cut_craft_core::geometry::{Path, Point};
use cut_craft_core::glam::DVec3;
use cut_craft_core::medial::CentreLine;
use cut_craft_core::toolpath::cut_points;
use std::cmp::Reverse;
use std::collections::{BinaryHeap, HashMap};
use std::sync::OnceLock;

/// Grid cells for finding the nearest edge, mm.
const CELL: f64 = 1.0;
/// The longest step taken across, mm: the bit is raised by about this over the tangent of
/// half its angle to be sure its sides miss an edge it passes.
const STEP: f64 = 0.1;
/// Closer to an edge than this, mm, a link is only taken next to where it starts or ends.
const TOUCH: f64 = 1e-3;
/// How near its start or end a link may touch an edge, mm: a cut runs into a corner there.
const NEAR_END: f64 = 0.05;
/// Centre lines whose ends are this close, mm, meet there.
const MEET: f64 = 0.1;

type Cell = (i64, i64);

#[expect(
    clippy::cast_possible_truncation,
    reason = "grid cells of a design on a machine bed"
)]
fn cell(q: Point) -> Cell {
    ((q.x / CELL).floor() as i64, (q.y / CELL).floor() as i64)
}

/// How far points are from the edges of one shape.
struct Clearance {
    grid: HashMap<Cell, Vec<(Point, Point)>>,
    low: Cell,
    high: Cell,
}

impl Clearance {
    fn new(piece: &[Path]) -> Self {
        let mut grid: HashMap<Cell, Vec<(Point, Point)>> = HashMap::new();
        let (mut low, mut high) = ((i64::MAX, i64::MAX), (i64::MIN, i64::MIN));
        for path in piece {
            let points = cut_points(path);
            for (i, &a) in points.iter().enumerate() {
                let b = points[(i + 1) % points.len()];
                let (c0, c1) = (cell(a.min(b)), cell(a.max(b)));
                low = (low.0.min(c0.0), low.1.min(c0.1));
                high = (high.0.max(c1.0), high.1.max(c1.1));
                for x in c0.0..=c1.0 {
                    for y in c0.1..=c1.1 {
                        grid.entry((x, y)).or_default().push((a, b));
                    }
                }
            }
        }
        Self { grid, low, high }
    }

    /// The distance from `q` to the nearest edge.
    fn at(&self, q: Point) -> f64 {
        let (cx, cy) = cell(q);
        let mut best = f64::INFINITY;
        let mut ring = 0i64;
        loop {
            for x in cx - ring..=cx + ring {
                for y in cy - ring..=cy + ring {
                    if (x - cx).abs() != ring && (y - cy).abs() != ring {
                        continue;
                    }
                    for &(a, b) in self.grid.get(&(x, y)).into_iter().flatten() {
                        best = best.min(segment_distance(a, b, q));
                    }
                }
            }
            #[expect(clippy::cast_precision_loss, reason = "a small ring count")]
            let reached = ring as f64 * CELL;
            let outside = cx - ring <= self.low.0
                && cy - ring <= self.low.1
                && cx + ring >= self.high.0
                && cy + ring >= self.high.1;
            if best <= reached || outside {
                return best;
            }
            ring += 1;
        }
    }
}

fn segment_distance(from: Point, to: Point, at: Point) -> f64 {
    let along = to - from;
    let share = if along.length_squared() > 0.0 {
        ((at - from).dot(along) / along.length_squared()).clamp(0.0, 1.0)
    } else {
        0.0
    };
    (from + along * share).distance(at)
}

/// The ways across one shape (with its holes) between its cuts.
pub(crate) struct Links<'a, F: Fn(f64) -> f64> {
    clearance: Clearance,
    lines: &'a [CentreLine],
    /// The deepest the tip may go this far from the edges.
    deepest: F,
    graph: OnceLock<Graph>,
}

/// The centre lines' points, joined along each line and where lines meet.
struct Graph {
    points: Vec<(Point, f64)>,
    next: Vec<Vec<(usize, f64)>>,
}

impl<'a, F: Fn(f64) -> f64> Links<'a, F> {
    pub(crate) fn new(piece: &[Path], lines: &'a [CentreLine], deepest: F) -> Self {
        Self {
            clearance: Clearance::new(piece),
            lines,
            deepest,
            graph: OnceLock::new(),
        }
    }

    /// The points to cut through from `a` to `b` (neither included) without lifting, or
    /// `None` when there is no way that stays inside the shape.
    pub(crate) fn between(&self, a: DVec3, b: DVec3) -> Option<Vec<DVec3>> {
        if let Some(points) = self.straight(a, b) {
            return Some(points);
        }
        self.along_centre_lines(a, b)
    }

    /// Straight across from `a` to `b`, the tip lowered evenly from one to the other but
    /// raised where its sides would reach an edge.
    fn straight(&self, a: DVec3, b: DVec3) -> Option<Vec<DVec3>> {
        let (from, to) = (a.truncate(), b.truncate());
        let length = from.distance(to);
        let mut samples: Vec<(f64, f64)> = vec![];
        let mut along = 0.0;
        loop {
            let q = from.lerp(to, if length > 0.0 { along / length } else { 1.0 });
            let clear = self.clearance.at(q);
            let near_end = along.min(length - along) <= NEAR_END;
            if clear < TOUCH && !near_end {
                return None;
            }
            samples.push((along, clear));
            if along >= length {
                break;
            }
            along = (along + (0.9 * clear).clamp(TOUCH, STEP)).min(length);
        }
        let mut points: Vec<DVec3> = vec![];
        for (i, &(at, clear)) in samples.iter().enumerate() {
            // the edge may come nearer between samples by at most the step to either side
            let step = [i.checked_sub(1), Some(i + 1)]
                .into_iter()
                .flatten()
                .filter_map(|k| samples.get(k))
                .map(|&(other, _)| (other - at).abs())
                .fold(0.0, f64::max);
            let even = if length > 0.0 {
                a.z + (b.z - a.z) * at / length
            } else {
                b.z
            };
            let z = even.max((self.deepest)((clear - step).max(0.0)));
            let q = from.lerp(to, if length > 0.0 { at / length } else { 1.0 });
            points.push(DVec3::new(q.x, q.y, z));
        }
        // the ends are the cuts' own
        let inner = points.get(1..points.len().saturating_sub(1))?.to_vec();
        Some(straightened(a, &inner, b))
    }

    /// Across to the nearest centre line, along the centre lines, and across to `b`, the tip
    /// no deeper than the shallower of `a` and `b`.
    fn along_centre_lines(&self, a: DVec3, b: DVec3) -> Option<Vec<DVec3>> {
        let graph = self.graph.get_or_init(|| Graph::new(self.lines));
        let top = a.z.max(b.z);
        let on = |q: DVec3| -> Option<(usize, Vec<DVec3>)> {
            let mut nearest: Vec<usize> = (0..graph.points.len()).collect();
            nearest.sort_by(|&i, &k| {
                let d = |n: usize| graph.points[n].0.distance(q.truncate());
                d(i).total_cmp(&d(k))
            });
            nearest.into_iter().take(16).find_map(|n| {
                let (p, r) = graph.points[n];
                let at = DVec3::new(p.x, p.y, top.max((self.deepest)(r)));
                self.straight(q, at).map(|mut via| {
                    via.push(at);
                    (n, via)
                })
            })
        };
        let (start, mut points) = on(a)?;
        let (end, mut back) = on(b)?;
        let route = graph.route(start, end)?;
        for pair in route.windows(2) {
            let (p, r) = graph.points[pair[1]];
            let narrowest = r.min(graph.points[pair[0]].1);
            points.push(DVec3::new(p.x, p.y, top.max((self.deepest)(narrowest))));
        }
        // back from b to its centre line point, turned round
        back.pop();
        back.reverse();
        points.extend(back);
        Some(points)
    }
}

impl Graph {
    fn new(lines: &[CentreLine]) -> Self {
        let mut points = vec![];
        let mut next: Vec<Vec<(usize, f64)>> = vec![];
        let mut ends: HashMap<Cell, Vec<usize>> = HashMap::new();
        for line in lines {
            let first = points.len();
            for (k, (&p, &r)) in line.points.iter().zip(&line.radii).enumerate() {
                points.push((p, r));
                next.push(vec![]);
                if k > 0 {
                    let (i, j) = (first + k - 1, first + k);
                    let d = points[i].0.distance(p);
                    next[i].push((j, d));
                    next[j].push((i, d));
                }
            }
            if points.len() > first {
                for n in [first, points.len() - 1] {
                    ends.entry(cell(points[n].0)).or_default().push(n);
                }
            }
        }
        for (&(cx, cy), here) in &ends {
            for &i in here {
                for x in cx - 1..=cx + 1 {
                    for y in cy - 1..=cy + 1 {
                        for &j in ends.get(&(x, y)).into_iter().flatten() {
                            let d = points[i].0.distance(points[j].0);
                            if i < j && d <= MEET {
                                next[i].push((j, d));
                                next[j].push((i, d));
                            }
                        }
                    }
                }
            }
        }
        Self { points, next }
    }

    /// The shortest way from `start` to `end`, both included.
    fn route(&self, start: usize, end: usize) -> Option<Vec<usize>> {
        let mut best = vec![f64::INFINITY; self.points.len()];
        let mut came = vec![usize::MAX; self.points.len()];
        let mut queue = BinaryHeap::new();
        best[start] = 0.0;
        queue.push(Reverse((Ordered(0.0), start)));
        while let Some(Reverse((Ordered(d), n))) = queue.pop() {
            if n == end {
                break;
            }
            if d > best[n] {
                continue;
            }
            for &(m, step) in &self.next[n] {
                if d + step < best[m] {
                    best[m] = d + step;
                    came[m] = n;
                    queue.push(Reverse((Ordered(d + step), m)));
                }
            }
        }
        if !best[end].is_finite() {
            return None;
        }
        let mut route = vec![end];
        while let Some(&n) = route.last()
            && n != start
        {
            route.push(came[n]);
        }
        route.reverse();
        Some(route)
    }
}

/// A distance that can be ordered (never NaN here).
#[derive(PartialEq)]
struct Ordered(f64);

impl Eq for Ordered {}

impl PartialOrd for Ordered {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Ordered {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.0.total_cmp(&other.0)
    }
}

/// `inner` (between `a` and `b`) without the points that a straight cut past them misses by
/// under a ten-thousandth of a millimetre, and never by going deeper.
fn straightened(a: DVec3, inner: &[DVec3], b: DVec3) -> Vec<DVec3> {
    let off = |from: DVec3, to: DVec3, q: DVec3| {
        let d = to - from;
        let t = if d.length_squared() > 0.0 {
            ((q - from).dot(d) / d.length_squared()).clamp(0.0, 1.0)
        } else {
            0.0
        };
        let on = from + d * t;
        on.distance(q) > 1e-4 || on.z < q.z - 1e-6
    };
    let mut kept: Vec<DVec3> = vec![];
    // the points left out since the last kept one
    let mut skipped: Vec<DVec3> = vec![];
    for (i, &q) in inner.iter().enumerate() {
        let before = kept.last().copied().unwrap_or(a);
        let after = inner.get(i + 1).copied().unwrap_or(b);
        if skipped.iter().chain([&q]).any(|&s| off(before, after, s)) {
            kept.push(q);
            skipped.clear();
        } else {
            skipped.push(q);
        }
    }
    kept
}
