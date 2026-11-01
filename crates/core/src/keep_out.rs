//! Keep-out zones: areas of the material a tool must not pass over, such as clamps (Aspire's
//! help: Keep-out Zones). They are closed paths of their own, made from selected vectors and
//! kept when those are deleted.

use crate::geometry::{Path, Point, inside};
use crate::toolpath::{Move, MoveKind};

/// How far outside a zone's outline a rapid routed round it passes, mm: clear of the
/// outline itself, which counts as inside.
const PASS_BY: f64 = 0.01;

/// Whether the tool, following `moves` in plan, passes over any of the `zones`.
pub fn crossed(zones: &[Path], moves: &[Move]) -> bool {
    let rings: Vec<Vec<Point>> = zones
        .iter()
        .map(|z| z.flatten(crate::polygon::TOLERANCE))
        .collect();
    if rings.is_empty() {
        return false;
    }
    let plan = |m: &Move| Point::new(m.to.x, m.to.y);
    if moves.first().is_some_and(|m| inside(&rings, plan(m))) {
        return true;
    }
    moves.windows(2).any(|w| {
        let (a, b) = (plan(&w[0]), plan(&w[1]));
        inside(&rings, b) || rings.iter().any(|ring| meets(a, b, ring))
    })
}

/// `moves` with each rapid that would pass over one of the `zones` routed round it at the
/// same height, the shortest way round the zone's outline, as Aspire's rapids between parts
/// go round its keep-out zones; and whether the tool still passes over one (a cut, a plunge,
/// or a rapid that starts or ends inside one or can find no way round): a toolpath that
/// cannot abide by the zones.
pub fn route(zones: &[Path], moves: &[Move]) -> (Vec<Move>, bool) {
    let rings = rings_of(zones);
    if rings.is_empty() || moves.is_empty() {
        return (moves.to_vec(), false);
    }
    let mut out = vec![moves[0]];
    for w in moves.windows(2) {
        let (a, b) = (w[0], w[1]);
        if b.kind == MoveKind::Rapid
            && crosses(&rings, plan(a), plan(b))
            && let Some(way) = way_round(&rings, plan(a), plan(b))
        {
            let z = a.to.z.max(b.to.z);
            out.extend(way.into_iter().map(|p| Move {
                kind: MoveKind::Rapid,
                to: glam::DVec3::new(p.x, p.y, z),
            }));
        }
        out.push(b);
    }
    let violates = crossed(zones, &out);
    (out, violates)
}

fn rings_of(zones: &[Path]) -> Vec<Vec<Point>> {
    zones
        .iter()
        .map(|z| z.flatten(crate::polygon::TOLERANCE))
        .filter(|r| r.len() > 2)
        .collect()
}

fn plan(m: Move) -> Point {
    Point::new(m.to.x, m.to.y)
}

/// Whether `p` is inside one of the `rings` (each on its own, so overlapping zones do not
/// cancel).
fn in_any(rings: &[Vec<Point>], p: Point) -> bool {
    rings.iter().any(|r| inside(std::slice::from_ref(r), p))
}

/// Whether going straight from `a` to `b` passes over a ring.
fn crosses(rings: &[Vec<Point>], a: Point, b: Point) -> bool {
    in_any(rings, a)
        || in_any(rings, b)
        || in_any(rings, (a + b) / 2.0)
        || rings.iter().any(|r| meets(a, b, r))
}

/// The corners of a way from `a` to `b` round the rings in the way, `a` and `b` left out;
/// `None` when either end is inside one or the rings leave no way round.
fn way_round(rings: &[Vec<Point>], a: Point, b: Point) -> Option<Vec<Point>> {
    if in_any(rings, a) || in_any(rings, b) {
        return None;
    }
    let mut way = vec![a, b];
    // ponytail: each pass goes round one ring the way meets; nested or touching zones that
    // need more passes than there are rings get no way round
    for _ in 0..=2 * rings.len() {
        let Some((i, ring)) = (0..way.len() - 1).find_map(|i| {
            rings
                .iter()
                .find(|r| crosses(std::slice::from_ref(r), way[i], way[i + 1]))
                .map(|r| (i, r))
        }) else {
            return Some(way[1..way.len() - 1].to_vec());
        };
        let round = round_hull(&hull(ring), way[i], way[i + 1])?;
        way.splice(i + 1..=i, round);
    }
    None
}

/// The ring's convex hull, each corner moved [`PASS_BY`] out from its middle, anticlockwise.
fn hull(ring: &[Point]) -> Vec<Point> {
    let mut points = ring.to_vec();
    points.sort_by(|p, q| p.x.total_cmp(&q.x).then(p.y.total_cmp(&q.y)));
    points.dedup();
    let turn = |o: Point, a: Point, b: Point| (a - o).perp_dot(b - o);
    let half = |points: &mut dyn Iterator<Item = Point>| {
        let mut h: Vec<Point> = vec![];
        for p in points {
            while h.len() >= 2 && turn(h[h.len() - 2], h[h.len() - 1], p) <= 0.0 {
                h.pop();
            }
            h.push(p);
        }
        h.pop();
        h
    };
    let mut h = half(&mut points.iter().copied());
    h.extend(half(&mut points.iter().rev().copied()));
    #[expect(clippy::cast_precision_loss, reason = "a ring's corner count")]
    let middle = h.iter().fold(Point::ZERO, |s, &p| s + p) / h.len() as f64;
    h.iter()
        .map(|&p| p + (p - middle).normalize_or_zero() * PASS_BY)
        .collect()
}

/// The shorter way from `a` to `b` round the convex `hull`, as its corners, pulled tight.
fn round_hull(hull: &[Point], a: Point, b: Point) -> Option<Vec<Point>> {
    let count = hull.len();
    if count < 3 {
        return None;
    }
    let nearest =
        |p: Point| (0..count).min_by(|&i, &j| hull[i].distance(p).total_cmp(&hull[j].distance(p)));
    let (from, to) = (nearest(a)?, nearest(b)?);
    let blocked = |p: Point, q: Point| meets(p, q, hull) || inside(&[hull.to_vec()], (p + q) / 2.0);
    let walk = |step: usize| {
        let mut corners = vec![];
        let mut i = from;
        loop {
            corners.push(hull[i]);
            if i == to {
                break;
            }
            i = (i + step) % count;
        }
        // pulled tight: from each point, straight to the furthest corner it can see
        let mut way = vec![a];
        let all: Vec<Point> = corners.into_iter().chain([b]).collect();
        let mut k = 0;
        while k < all.len() {
            let here = *way.last().expect("starts at a");
            let far = (k..all.len())
                .rev()
                .find(|&j| !blocked(here, all[j]))
                .unwrap_or(k);
            way.push(all[far]);
            k = far + 1;
        }
        let length: f64 = way.windows(2).map(|w| w[0].distance(w[1])).sum();
        way.pop();
        way.remove(0);
        (length, way)
    };
    let (one, other) = (walk(1), walk(count - 1));
    Some(if one.0 <= other.0 { one.1 } else { other.1 })
}

/// Whether the segment `a`–`b` crosses an edge of the closed `ring`.
fn meets(a: Point, b: Point, ring: &[Point]) -> bool {
    let side = |p: Point, q: Point, r: Point| (q - p).perp_dot(r - p);
    (0..ring.len()).any(|i| {
        let (c, d) = (ring[i], ring[(i + 1) % ring.len()]);
        side(a, b, c) * side(a, b, d) < 0.0 && side(c, d, a) * side(c, d, b) < 0.0
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::square;
    use crate::toolpath::MoveKind;
    use glam::DVec3;

    fn path(points: &[(f64, f64)]) -> Vec<Move> {
        points
            .iter()
            .map(|&(x, y)| Move {
                kind: MoveKind::Cut,
                to: DVec3::new(x, y, -1.0),
            })
            .collect()
    }

    fn rapid(x: f64, y: f64) -> Move {
        Move {
            kind: MoveKind::Rapid,
            to: DVec3::new(x, y, 5.0),
        }
    }

    #[test]
    fn a_rapid_over_a_zone_goes_round_it_and_a_cut_through_one_violates() {
        let zones = [square(10.0, 10.0, 10.0)];
        let moves = [rapid(0.0, 14.0), rapid(30.0, 16.0)];
        let (routed, violates) = route(&zones, &moves);
        assert!(!violates, "{routed:?}");
        assert!(routed.len() > 2, "a way round");
        assert!(!crossed(&zones, &routed));
        assert_eq!((routed[0], *routed.last().unwrap()), (moves[0], moves[1]));
        assert!(
            routed
                .iter()
                .all(|m| m.kind == MoveKind::Rapid && (m.to.z - 5.0).abs() < 1e-12)
        );
        // round the near side: no further than the zone's own corners
        assert!(
            routed.iter().all(|m| (5.0..=25.0).contains(&m.to.y)),
            "{routed:?}"
        );
        // a cut through it is not moved and is a violation
        let (cut, violates) = route(&zones, &path(&[(0.0, 15.0), (30.0, 15.0)]));
        assert!(violates);
        assert_eq!(cut.len(), 2);
        // nothing in the way: as it was
        let clear = [rapid(0.0, 0.0), rapid(30.0, 0.0)];
        assert_eq!(route(&zones, &clear), (clear.to_vec(), false));
        // round two zones in a row
        let two = [square(10.0, 10.0, 10.0), square(40.0, 10.0, 10.0)];
        let (routed, violates) = route(&two, &[rapid(0.0, 15.0), rapid(60.0, 15.0)]);
        assert!(!violates && !crossed(&two, &routed), "{routed:?}");
    }

    #[test]
    fn a_tool_passing_over_a_zone_is_caught() {
        let zone = [square(10.0, 10.0, 10.0)];
        assert!(
            crossed(&zone, &path(&[(0.0, 15.0), (30.0, 15.0)])),
            "straight through"
        );
        assert!(
            crossed(&zone, &path(&[(0.0, 0.0), (15.0, 15.0)])),
            "ending inside"
        );
        assert!(crossed(&zone, &path(&[(15.0, 15.0)])), "starting inside");
        assert!(
            !crossed(&zone, &path(&[(0.0, 0.0), (30.0, 0.0), (30.0, 30.0)])),
            "round it"
        );
        assert!(!crossed(&[], &path(&[(15.0, 15.0)])), "no zones");
    }
}
