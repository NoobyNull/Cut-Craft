//! Narrow channels cleared in one trip: along the channel's centre line in loops that reach
//! from wall to wall, each loop a light step on from the last, before the rest of the area
//! is cleared adaptively. A channel up to a few bits wide is cleared in a single sweep from
//! end to end, the bite held at the step, instead of in many short cuts and links.
//!
//! Each loop is a true circle (so a post writes it as arcs) sized to the channel where it
//! is, then a short straight step along the wall to the next.

use super::{Clearing, Piece};
use crate::geometry::polyline_distance;
use crate::geometry::{Path, Point};
use crate::medial::centre_lines;
use crate::toolpath::cut_points;
use std::f64::consts::TAU;

/// The widest channel swept: the tool centre's room either side of the centre line, in bit
/// radii (3: a channel four bits wide). Wider areas are cleared adaptively.
const WIDEST: f64 = 3.0;

/// How fast the channel may widen or narrow along its centre line and still be swept: the
/// centre lines into a corner (which narrow at 0.7 and more) are left to the adaptive.
const STEEPEST: f64 = 0.4;

/// How far short of each end of a channel the sweep stops, in bit radii: the adaptive
/// clears the end and its corners, coming at them from the swept channel.
const END: f64 = 1.5;

/// Points round each loop.
const LOOP_POINTS: usize = 48;

/// The loops through the narrow channels of `c`, and the material they clear (a little
/// under, so nothing is taken for gone that is not).
pub fn sweeps(c: &Clearing) -> (Vec<Piece>, Vec<Path>) {
    let (radius, step) = (c.radius, c.bite.min(c.radius));
    if radius <= 0.0 || step <= 0.0 {
        return (vec![], vec![]);
    }
    let mut pieces = vec![];
    let mut cleared = vec![];
    let walls: Vec<Vec<Point>> = c
        .region
        .iter()
        .filter(|r| r.closed)
        .map(cut_points)
        .collect();
    for line in centre_lines(c.region, (radius / 4.0).max(0.1)) {
        for run in runs(&line.points, &line.radii, radius, step) {
            // short of each end: the adaptive takes the end and its corners, which a loop
            // the channel's size can't reach into without biting hard
            if let Some(sweep) = sweep(&run, &walls, step, c.climb, END * radius) {
                cleared.extend(sweep.swept(radius));
                pieces.push(Piece::Enter(sweep.start));
                pieces.push(Piece::Cut(sweep.points));
            }
        }
    }
    // one outline round what all the loops cleared: overlapping circles would read as holes
    (pieces, crate::polygon::union(&cleared))
}

/// The stretches of a centre line narrow enough to sweep: its points with the room either
/// side, at least a step and at most [`WIDEST`] radii, changing gently; at least a bit
/// long.
fn runs(points: &[Point], radii: &[f64], radius: f64, step: f64) -> Vec<Vec<(Point, f64)>> {
    let n = points.len();
    let sweepable = |i: usize| {
        let r = radii[i];
        if r < step || r > WIDEST * radius {
            return false;
        }
        let (a, b) = (i.saturating_sub(1), (i + 1).min(n - 1));
        let along = points[a].distance(points[b]);
        along > 0.0 && (radii[b] - radii[a]).abs() <= STEEPEST * along
    };
    let mut runs = vec![];
    let mut run: Vec<(Point, f64)> = vec![];
    for i in 0..n {
        if sweepable(i) {
            run.push((points[i], radii[i]));
        } else if !run.is_empty() {
            runs.push(std::mem::take(&mut run));
        }
    }
    runs.push(run);
    runs.retain(|r| length(r) >= 2.0 * radius);
    runs
}

/// The room a loop takes of `r`: a whisker under, for the centre line's own accuracy.
fn room(r: f64) -> f64 {
    (r - 0.02).max(0.0)
}

fn length(run: &[(Point, f64)]) -> f64 {
    run.windows(2).map(|w| w[0].0.distance(w[1].0)).sum()
}

/// One channel's sweep.
struct Sweep {
    /// Where it goes down: the middle of the first loop.
    start: Point,
    points: Vec<Point>,
    /// Each loop's middle and size.
    loops: Vec<(Point, f64)>,
}

impl Sweep {
    /// What the sweep clears: each loop's circle grown by the bit.
    fn swept(&self, radius: f64) -> Vec<Path> {
        // a little under: what is left the adaptive takes
        let grow = radius - 0.02;
        self.loops
            .iter()
            .map(|&(c, r)| Path::circle(c, r + grow))
            .collect()
    }
}

/// The loops along `run`: from its start, down in the middle and out in a spiral a step
/// each turn to the channel's size, then a loop every `step` along it, each as big as the
/// channel is narrow along that step, stopping `trim` short of either end. Anticlockwise
/// climbing, else clockwise.
fn sweep(
    run: &[(Point, f64)],
    walls: &[Vec<Point>],
    step: f64,
    climb: bool,
    trim: f64,
) -> Option<Sweep> {
    let total = length(run) - trim;
    if total <= trim + step {
        return None;
    }
    let turn = if climb { 1.0 } else { -1.0 };
    // the room round a point: how far the nearest wall is (the centre line's own radii are
    // only at its points)
    let room_at = |q: Point| {
        let near = walls
            .iter()
            .map(|w| {
                let mut ring = w.clone();
                ring.push(w[0]);
                polyline_distance(&ring, q)
            })
            .fold(f64::MAX, f64::min);
        room(near)
    };
    // along the run: the point, the way along, and the room either side `at` from its start
    let at = |s: f64| -> (Point, Point, f64) {
        let mut gone = 0.0;
        for w in run.windows(2) {
            let piece = w[0].0.distance(w[1].0);
            if gone + piece >= s && piece > 0.0 {
                let t = ((s - gone) / piece).clamp(0.0, 1.0);
                let along = (w[1].0 - w[0].0) / piece;
                let q = w[0].0.lerp(w[1].0, t);
                return (q, along, room_at(q));
            }
            gone += piece;
        }
        let n = run.len();
        let along = (run[n - 1].0 - run[n - 2].0).normalize_or(Point::X);
        (run[n - 1].0, along, room_at(run[n - 1].0))
    };
    let (first, along, room) = at(trim);
    let mut points = vec![first];
    let mut loops = vec![];
    // the loops start on the wall on the bit's material side, going along
    let side = |along: Point| -turn * along.perp();
    // out from the middle in a spiral, a step a turn, ending on that wall going along
    let turns = (room / step).ceil().max(1.0);
    #[expect(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let spiral = (turns as usize) * LOOP_POINTS;
    let start_angle = side(along).to_angle();
    for k in 1..=spiral {
        #[expect(clippy::cast_precision_loss)]
        let f = k as f64 / spiral as f64;
        let a = start_angle + turn * TAU * turns * (f - 1.0);
        points.push(first + room * f * Point::from_angle(a));
    }
    loops.push((first, room));
    // a loop a step along, as big as the channel lets it be over that step; where the
    // channel widens, a shorter step, so the loop's front reaches no more than a step past
    // the last one's
    let mut s = trim;
    let mut last = room;
    while s + step <= total {
        let mut advance = step;
        let (mut centre, mut along, mut r) = (first, Point::X, 0.0);
        for _ in 0..3 {
            let (c, a, here) = at(s + advance);
            let (_, _, before) = at(s + advance - step);
            (centre, along, r) = (c, a, here.min(before));
            let grown = r - last;
            if grown <= 0.0 || advance + grown <= step * 1.05 {
                break;
            }
            advance = (step - grown).max(0.25 * step);
        }
        s += advance;
        last = r;
        let begin = centre + r * side(along);
        points.push(begin);
        let a0 = side(along).to_angle();
        for k in 1..=LOOP_POINTS {
            #[expect(clippy::cast_precision_loss)]
            let a = a0 + turn * TAU * k as f64 / LOOP_POINTS as f64;
            points.push(centre + r * Point::from_angle(a));
        }
        loops.push((centre, r));
    }
    Some(Sweep {
        start: first,
        points,
        loops,
    })
}
