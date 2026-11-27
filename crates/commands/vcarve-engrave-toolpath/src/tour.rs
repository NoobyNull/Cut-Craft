//! The orders the last pass carves the centre lines in, other than the shortest path: round
//! each shape (Stay down, Round the shape) or one stretch at a time from one end (Lift
//! between cuts).

use crate::Run;
use crate::link::Graph;
use cut_craft_core::geometry::Point;
use cut_craft_core::glam::DVec3;
use cut_craft_core::medial::CentreLine;
use std::collections::HashSet;

/// Depths closer than this, mm, count as level.
const LEVEL: f64 = 1e-6;

/// The graph's points with the ends of lines that meet made one point (the first of them),
/// so a walk turning at a meeting sees every way out of it: each point's neighbours, none for
/// a point merged into another.
fn merged(graph: &Graph) -> Vec<Vec<usize>> {
    let count = graph.points.len();
    let mut leader: Vec<usize> = (0..count).collect();
    for &(a, b) in &graph.meets {
        let (a, b) = (find(&mut leader, a), find(&mut leader, b));
        if a != b {
            leader[a.max(b)] = a.min(b);
        }
    }
    let mut next: Vec<Vec<usize>> = vec![vec![]; count];
    for (i, ways) in graph.next.iter().enumerate() {
        let a = find(&mut leader, i);
        for &(m, _) in ways {
            let b = find(&mut leader, m);
            if a != b && !next[a].contains(&b) {
                next[a].push(b);
            }
        }
    }
    next
}

/// The point `i` was merged into, shortening the way there as it goes.
fn find(leader: &mut [usize], mut i: usize) -> usize {
    while leader[i] != i {
        leader[i] = leader[leader[i]];
        i = leader[i];
    }
    i
}

/// A join between two points, the lower numbered first.
fn join(a: usize, b: usize) -> (usize, usize) {
    (a.min(b), a.max(b))
}

/// How far round anticlockwise, in radians, `to` is from `from`: just over 0 to a full turn
/// (the same way is a full turn, so going back the way it came is the last choice).
fn turn(from: Point, to: Point) -> f64 {
    let angle = from.perp_dot(to).atan2(from.dot(to));
    if angle <= 1e-12 {
        angle + std::f64::consts::TAU
    } else {
        angle
    }
}

/// The centre lines carved round each shape: from the point nearest `from`, along each join
/// not yet cut, always taking the first way round anticlockwise from the way it came, and
/// back along the way it came when a branch ends. So every shape is gone round the same way,
/// out and back along each branch in turn. Each point at `depth` (given its distance to the
/// edge); points not `wanted` (cleared already) are left out, the walk split there.
pub(crate) fn round(
    lines: &[CentreLine],
    depth: impl Fn(f64) -> f64,
    wanted: impl Fn(f64) -> bool,
    mut from: Point,
) -> Vec<Run> {
    let graph = Graph::new(lines);
    let next = merged(&graph);
    let count = graph.points.len();
    let mut cut: HashSet<(usize, usize)> = HashSet::new();
    let mut reached = vec![false; count];
    let mut runs = vec![];
    while let Some(start) = (0..count)
        .filter(|&i| !reached[i] && !next[i].is_empty())
        .min_by(|&a, &b| {
            let away = |i: usize| graph.points[i].0.distance(from);
            away(a).total_cmp(&away(b))
        })
    {
        let walk = walk_round(&graph, &next, start, &mut cut, &mut reached);
        if let Some(&last) = walk.last() {
            from = graph.points[last].0;
        }
        runs.extend(split(&graph, &walk, &depth, &wanted));
    }
    runs
}

/// One shape's walk round from `start` (see [`round`]), as points in order.
fn walk_round(
    graph: &Graph,
    joined: &[Vec<usize>],
    start: usize,
    cut: &mut HashSet<(usize, usize)>,
    reached: &mut [bool],
) -> Vec<usize> {
    let at = |i: usize| graph.points[i].0;
    reached[start] = true;
    let mut walk = vec![start];
    // the way back to the start from where it is
    let mut back_to = vec![start];
    // the walk up to its last new join: the way back from the last branch is not cut again
    let mut wanted_length = 1;
    while let Some(&here) = back_to.last() {
        let came = back_to.len().checked_sub(2).map(|i| back_to[i]);
        // the way it came, turned round: the first way anticlockwise from it is taken
        let behind = came.map_or(Point::new(-1.0, 0.0), |c| at(c) - at(here));
        let next = joined[here]
            .iter()
            .copied()
            .filter(|&m| !cut.contains(&join(here, m)))
            .min_by(|&a, &b| {
                let way = |m: usize| turn(behind, at(m) - at(here));
                way(a).total_cmp(&way(b))
            });
        if let Some(m) = next {
            cut.insert(join(here, m));
            walk.push(m);
            if reached[m] {
                // a way round a hole: there and straight back
                walk.push(here);
            } else {
                reached[m] = true;
                back_to.push(m);
            }
            wanted_length = walk.len();
        } else {
            back_to.pop();
            if let Some(&previous) = back_to.last() {
                walk.push(previous);
            }
        }
    }
    walk.truncate(wanted_length);
    walk
}

/// The centre lines in stretches, each cut once from one end to the other: from the
/// shallowest end down (`shallow_in`), or from the deepest end up. Each stretch carries on
/// down from where it starts, as straight as it can, until the way on goes up or is cut
/// already; the next starts nearest where the last ended.
pub(crate) fn stretches(
    lines: &[CentreLine],
    depth: impl Fn(f64) -> f64,
    wanted: impl Fn(f64) -> bool,
    shallow_in: bool,
    mut from: Point,
) -> Vec<Run> {
    let graph = Graph::new(lines);
    let joined = merged(&graph);
    let count = graph.points.len();
    let at = |i: usize| graph.points[i].0;
    let z: Vec<f64> = graph.points.iter().map(|&(_, r)| depth(r)).collect();
    let mut cut: HashSet<(usize, usize)> = HashSet::new();
    let mut walks: Vec<Vec<usize>> = vec![];
    loop {
        let open = |cut: &HashSet<(usize, usize)>, i: usize| {
            joined[i]
                .iter()
                .copied()
                .filter(move |&m| !cut.contains(&join(i, m)))
                .collect::<Vec<_>>()
        };
        // where a stretch can start: a point with a join still to cut and none going higher
        let start = (0..count)
            .filter(|&i| {
                let ways = open(&cut, i);
                !ways.is_empty() && ways.iter().all(|&m| z[m] <= z[i] + LEVEL)
            })
            .min_by(|&a, &b| at(a).distance(from).total_cmp(&at(b).distance(from)));
        let Some(start) = start else { break };
        let mut walk = vec![start];
        let (mut here, mut came) = (start, None::<usize>);
        loop {
            let ahead = came.map(|c| at(here) - at(c));
            let next = open(&cut, here)
                .into_iter()
                .filter(|&m| z[m] <= z[here] + LEVEL)
                .min_by(|&a, &b| {
                    // straight on where it can; from the start, the steepest way down
                    let bend = |m: usize| {
                        ahead.map_or(z[m], |ahead| {
                            let way = at(m) - at(here);
                            -ahead.dot(way) / (ahead.length() * way.length()).max(1e-12)
                        })
                    };
                    bend(a).total_cmp(&bend(b))
                });
            let Some(m) = next else { break };
            cut.insert(join(here, m));
            walk.push(m);
            (came, here) = (Some(here), m);
        }
        from = at(here);
        walks.push(walk);
    }
    let mut runs: Vec<Run> = walks
        .iter()
        .flat_map(|walk| split(&graph, walk, &depth, &wanted))
        .collect();
    if !shallow_in {
        for run in &mut runs {
            run.points.reverse();
        }
        runs = nearest_start_first(runs);
    }
    runs
}

/// `runs` reordered so each starts nearest where the one before ended, none turned round.
fn nearest_start_first(mut runs: Vec<Run>) -> Vec<Run> {
    let mut ordered = Vec::with_capacity(runs.len());
    let Some(first) = (!runs.is_empty()).then(|| runs.remove(0)) else {
        return ordered;
    };
    let mut from = first.points[first.points.len() - 1].truncate();
    ordered.push(first);
    while !runs.is_empty() {
        let nearest = (0..runs.len())
            .min_by(|&a, &b| {
                let away = |i: usize| runs[i].points[0].truncate().distance(from);
                away(a).total_cmp(&away(b))
            })
            .expect("runs are left");
        let run = runs.swap_remove(nearest);
        from = run.points[run.points.len() - 1].truncate();
        ordered.push(run);
    }
    ordered
}

/// The walk's points as runs at `depth`, broken where a point is not `wanted`.
fn split(
    graph: &Graph,
    walk: &[usize],
    depth: &impl Fn(f64) -> f64,
    wanted: &impl Fn(f64) -> bool,
) -> Vec<Run> {
    let mut runs = vec![];
    let mut points: Vec<DVec3> = vec![];
    // where a point of the shape is: the tip at the surface cuts nothing
    let surface = depth(0.0) - 1e-6;
    for &i in walk {
        let (q, r) = graph.points[i];
        if wanted(r) {
            // the same point twice running (lines meeting there) is one point
            let here = DVec3::new(q.x, q.y, depth(r));
            match points.last() {
                Some(p) if p.distance(here) <= 1e-9 => {}
                // along the surface: nothing to cut, so the run ends and the next starts here
                Some(p) if p.z >= surface && here.z >= surface => {
                    if points.len() > 1 {
                        runs.push(Run {
                            points: std::mem::take(&mut points),
                            closed: false,
                        });
                    }
                    points = vec![here];
                }
                _ => points.push(here),
            }
        } else if points.len() > 1 {
            runs.push(Run {
                points: std::mem::take(&mut points),
                closed: false,
            });
        } else {
            points.clear();
        }
    }
    if points.len() > 1 {
        runs.push(Run {
            points,
            closed: false,
        });
    }
    runs
}
