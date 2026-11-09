//! Keep-out zones: areas of the material a tool must not pass over, such as clamps (Aspire's
//! help: Keep-out Zones). They are closed paths of their own, made from selected vectors and
//! kept when those are deleted, each with a name shown as its label ("Keep-out: Clamp 1").
//!
//! A cutting move counts as entering a zone when the cutter's edge (its radius out from the
//! tool's centre) reaches it: clamps stand above the material, so the edge is what hits them.
//! Rapids run at the safe height and only go round the zones' own outlines. With clipping
//! allowed ([`clip`]) the parts of a toolpath that would enter a zone are left out, the tool
//! lifting over the zone and coming back down past it: whatever was under the zone stays
//! uncut, as if the zone had been subtracted from it.

use crate::geometry::{Path, Point, inside};
use crate::toolpath::{Move, MoveKind};
use serde::{Deserialize, Serialize};

/// How far outside a zone's outline a rapid routed round it passes, mm: clear of the
/// outline itself, which counts as inside.
const PASS_BY: f64 = 0.01;

/// How much further than the cutter's radius a clipped toolpath keeps from a zone, mm: clear
/// of the outline (which counts as inside) even where its round corners are flattened.
const CLIP_MARGIN: f64 = 0.05;

/// What a zone is called when it was made without a name (jobs saved before zones had them).
pub const UNNAMED: &str = "Zone";

/// A keep-out zone: its outline and the name its label shows.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct KeepOutZone {
    pub name: String,
    pub path: Path,
}

impl KeepOutZone {
    pub fn new(name: impl Into<String>, path: Path) -> Self {
        Self {
            name: name.into(),
            path,
        }
    }

    /// What its label says.
    #[must_use]
    pub fn label(&self) -> String {
        format!("Keep-out: {}", self.name)
    }

    /// Whether `p` is inside it.
    #[must_use]
    pub fn contains(&self, p: Point) -> bool {
        inside(&[self.path.flatten(crate::polygon::TOLERANCE)], p)
    }
}

// jobs saved before zones had names keep each one as a bare outline
impl<'de> Deserialize<'de> for KeepOutZone {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        #[serde(untagged)]
        enum Saved {
            Named { name: String, path: Path },
            Bare(Path),
        }
        Ok(match Saved::deserialize(d)? {
            Saved::Named { name, path } => Self { name, path },
            Saved::Bare(path) => Self::new(UNNAMED, path),
        })
    }
}

/// `count` names from `base`: the name itself for one zone, numbered from `first` for more
/// ("Clamp 1", "Clamp 2").
#[must_use]
pub fn names(base: &str, count: usize, first: usize) -> Vec<String> {
    let base = if base.trim().is_empty() {
        UNNAMED
    } else {
        base.trim()
    };
    if count == 1 && first == 1 {
        return vec![base.to_owned()];
    }
    (first..first + count)
        .map(|n| format!("{base} {n}"))
        .collect()
}

/// The zones grown by `by`, as rings: the area the tool's centre must keep out of when the
/// cutter's edge is to keep out of the zones.
fn grown_rings(zones: &[Path], by: f64) -> Vec<Vec<Point>> {
    if by <= 0.0 {
        return rings_of(zones);
    }
    rings_of(&crate::polygon::offset(
        zones,
        by,
        crate::polygon::Corners::Round,
    ))
}

/// Whether a tool of radius `radius` following `moves` cuts into any of the `zones` (its
/// edge reaching one on a cut or plunge) or rapids over one: a toolpath that cannot be made
/// as it is without clipping.
pub fn enters(zones: &[Path], moves: &[Move], radius: f64) -> bool {
    let centre = rings_of(zones);
    if centre.is_empty() || moves.is_empty() {
        return false;
    }
    let edge = grown_rings(zones, radius.max(0.0));
    let mut from = moves[0].to;
    moves.iter().enumerate().any(|(i, m)| {
        let (a, b) = (Point::new(from.x, from.y), plan(*m));
        from = m.to;
        if m.kind == MoveKind::Rapid {
            if i == 0 {
                in_any(&centre, b)
            } else {
                crosses(&centre, a, b)
            }
        } else {
            crosses(&edge, a, b)
        }
    })
}

/// Which of `zones` a tool of radius `radius` following `moves` [`enters`].
pub fn entered<'a>(
    zones: impl IntoIterator<Item = &'a KeepOutZone>,
    moves: &[Move],
    radius: f64,
) -> Vec<&'a KeepOutZone> {
    zones
        .into_iter()
        .filter(|z| enters(std::slice::from_ref(&z.path), moves, radius))
        .collect()
}

/// `moves` clipped by the `zones` for a tool of radius `radius`: each cut or plunge that
/// would bring the cutter's edge into a zone is left out, the tool rising straight up to
/// `safe` (or the toolpath's own highest point, if higher) where it would enter, and coming
/// back down past the zone where it would leave; rapids that would end in one are left out
/// too. Rapids that cross a zone are for [`route`] to take round it.
pub fn clip(zones: &[Path], moves: &[Move], radius: f64, safe: f64) -> Vec<Move> {
    let rings = grown_rings(zones, radius.max(0.0) + CLIP_MARGIN);
    if rings.is_empty() || moves.is_empty() {
        return moves.to_vec();
    }
    let safe = moves.iter().fold(safe, |z, m| z.max(m.to.z));
    let above = |p: glam::DVec3| glam::DVec3::new(p.x, p.y, safe);
    let mut out: Vec<Move> = Vec::with_capacity(moves.len());
    let push = |out: &mut Vec<Move>, kind, to| {
        if out.last().is_none_or(|m: &Move| m.to != to) {
            out.push(Move { kind, to });
        }
    };
    let mut lifted = false;
    let mut from = moves[0].to;
    for &m in moves {
        let pieces = if m.kind == MoveKind::Rapid {
            vec![(from, m.to)]
        } else {
            split(&rings, from, m.to)
        };
        from = m.to;
        for (p, q) in pieces {
            // a rapid by where it ends; a cut by its middle, each piece being wholly in or out
            let blocked = if m.kind == MoveKind::Rapid {
                in_any(&rings, plan_of(q))
            } else {
                in_any(&rings, plan_of((p + q) / 2.0))
            };
            if blocked {
                // up and out of the way where the cutter would reach the zone
                if !lifted && let Some(last) = out.last().map(|m| m.to) {
                    push(&mut out, MoveKind::Rapid, above(last));
                }
                lifted = true;
            } else if m.kind == MoveKind::Rapid {
                if lifted {
                    push(&mut out, MoveKind::Rapid, above(q));
                    lifted = false;
                }
                push(&mut out, MoveKind::Rapid, q);
            } else {
                // back down past the zone, then on as it was
                if lifted {
                    push(&mut out, MoveKind::Rapid, above(p));
                    push(&mut out, MoveKind::Plunge, p);
                    lifted = false;
                }
                push(&mut out, m.kind, q);
            }
        }
    }
    out
}

fn plan_of(p: glam::DVec3) -> Point {
    Point::new(p.x, p.y)
}

/// The straight move from `a` to `b` cut where it crosses the rings' edges, as pieces each
/// wholly in or out of them.
fn split(
    rings: &[Vec<Point>],
    from: glam::DVec3,
    to: glam::DVec3,
) -> Vec<(glam::DVec3, glam::DVec3)> {
    let (start, end) = (plan_of(from), plan_of(to));
    let along = end - start;
    let mut cuts: Vec<f64> = vec![0.0, 1.0];
    if along.length_squared() > 0.0 {
        for ring in rings {
            for i in 0..ring.len() {
                let (corner, next) = (ring[i], ring[(i + 1) % ring.len()]);
                let edge = next - corner;
                let den = along.perp_dot(edge);
                if den.abs() < 1e-15 {
                    continue;
                }
                // where on the move, and on the edge, the two meet
                let on_move = (corner - start).perp_dot(edge) / den;
                let on_edge = (corner - start).perp_dot(along) / den;
                if (0.0..=1.0).contains(&on_edge) && on_move > 0.0 && on_move < 1.0 {
                    cuts.push(on_move);
                }
            }
        }
    }
    cuts.sort_by(f64::total_cmp);
    cuts.dedup_by(|x, y| (*x - *y).abs() < 1e-12);
    if cuts.len() == 2 && start == end {
        return vec![(from, to)];
    }
    cuts.windows(2)
        .map(|w| (from.lerp(to, w[0]), from.lerp(to, w[1])))
        .collect()
}

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
    let (out, _, violates) = route_indexed(zones, moves);
    (out, violates)
}

/// [`route`], also giving for each move out the index of the move in `moves` it is or leads
/// to (a waypoint, the rapid it goes round the zones for), so ranges of moves can follow.
pub fn route_indexed(zones: &[Path], moves: &[Move]) -> (Vec<Move>, Vec<usize>, bool) {
    let rings = rings_of(zones);
    if rings.is_empty() || moves.is_empty() {
        return (moves.to_vec(), (0..moves.len()).collect(), false);
    }
    let (mut out, mut origin) = (vec![moves[0]], vec![0]);
    for (i, w) in moves.windows(2).enumerate() {
        let (a, b) = (w[0], w[1]);
        if b.kind == MoveKind::Rapid
            && crosses(&rings, plan(a), plan(b))
            && let Some(way) = way_round(&rings, plan(a), plan(b))
        {
            let z = a.to.z.max(b.to.z);
            for p in way {
                out.push(Move {
                    kind: MoveKind::Rapid,
                    to: glam::DVec3::new(p.x, p.y, z),
                });
                origin.push(i + 1);
            }
        }
        out.push(b);
        origin.push(i + 1);
    }
    let violates = crossed(zones, &out);
    (out, origin, violates)
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
    fn a_drill_banks_plunges_keep_their_moves_when_rapids_go_round_a_zone() {
        use crate::drill_bank::{BankHit, BankToolpath, DrillBank};
        let mut doc = crate::testing::with_toolpath();
        doc.keep_out = vec![KeepOutZone::new("Clamp", square(40.0, -10.0, 20.0))];
        let hole = |x: f64| {
            let at = |z: f64| DVec3::new(x, 0.0, z);
            [
                Move {
                    kind: MoveKind::Rapid,
                    to: at(5.0),
                },
                Move {
                    kind: MoveKind::Plunge,
                    to: at(-10.0),
                },
                Move {
                    kind: MoveKind::Rapid,
                    to: at(5.0),
                },
            ]
        };
        let mut t = doc.toolpaths.values().next().unwrap().clone();
        t.moves = [hole(0.0), hole(100.0)].concat();
        let hit = |start, x| BankHit {
            start,
            end: start + 3,
            head: Point::new(x, 0.0),
            drills: vec![],
        };
        t.drill_bank = Some(BankToolpath {
            bank: DrillBank::new(1, 0, 32.0, 5.0),
            hits: vec![hit(0, 0.0), hit(3, 100.0)],
        });
        let mut c = crate::Change::builder(&doc);
        let id = c.add_toolpath(t);
        doc.apply(&c.finish()).unwrap();
        let made = &doc.toolpaths[&id];
        assert!(made.moves.len() > 6, "a way round the zone");
        let hits = &made.drill_bank.as_ref().unwrap().hits;
        assert_eq!(hits[0].end, hits[1].start);
        assert_eq!(hits[1].end, made.moves.len());
        for h in hits {
            let moves = &made.moves[h.start..h.end];
            assert!(
                moves.iter().any(|m| m.kind == MoveKind::Plunge),
                "{moves:?}"
            );
        }
        // the second hit's way round leads into it
        assert_eq!(made.moves[hits[1].end - 2].to.x, 100.0);
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

    fn plunge(x: f64, y: f64) -> Move {
        Move {
            kind: MoveKind::Plunge,
            to: DVec3::new(x, y, -1.0),
        }
    }

    #[test]
    fn a_cut_reaching_a_zone_with_the_cutters_edge_enters_it() {
        let zone = [square(10.0, 10.0, 10.0)];
        // 2 mm below the zone: a 6 mm cutter's edge reaches it, a 2 mm one's does not
        let beside = [rapid(0.0, 8.0), plunge(0.0, 8.0), path(&[(30.0, 8.0)])[0]];
        assert!(enters(&zone, &beside, 3.0));
        assert!(!enters(&zone, &beside, 1.0));
        // a rapid over it only counts by the tool's centre: it goes round at the safe height
        assert!(!enters(&zone, &[rapid(0.0, 8.0), rapid(30.0, 8.0)], 3.0));
        assert!(enters(&zone, &[rapid(0.0, 15.0), rapid(30.0, 15.0)], 3.0));
        assert!(!enters(&[], &beside, 3.0), "no zones");
    }

    #[test]
    fn a_cut_through_a_zone_is_clipped_the_tool_lifting_over_it() {
        let zone = [square(10.0, 10.0, 10.0)];
        let moves = [
            rapid(0.0, 15.0),
            plunge(0.0, 15.0),
            path(&[(30.0, 15.0)])[0],
        ];
        let radius = 1.0;
        assert!(enters(&zone, &moves, radius));
        let clipped = clip(&zone, &moves, radius, 5.0);
        // its rapid over the zone then goes round it
        let (routed, violates) = route(&zone, &clipped);
        assert!(!violates && !enters(&zone, &routed, radius), "{routed:#?}");
        // both sides of the zone still cut, at the depth they were
        let cuts: Vec<_> = clipped.iter().filter(|m| m.kind == MoveKind::Cut).collect();
        assert_eq!(cuts.len(), 2, "{clipped:#?}");
        assert!((cuts[0].to.x - (10.0 - radius)).abs() < 0.1, "{clipped:#?}");
        assert!((cuts[1].to.x - 30.0).abs() < 1e-9);
        assert!(cuts.iter().all(|m| (m.to.z + 1.0).abs() < 1e-9));
        // over it at the safe height, down again past it
        let over: Vec<_> = clipped.iter().filter(|m| m.to.z >= 5.0).collect();
        assert!(over.iter().all(|m| m.kind == MoveKind::Rapid));
        let down = clipped
            .iter()
            .position(|m| m.kind == MoveKind::Plunge && m.to.x > 15.0)
            .expect("a plunge past the zone");
        assert!(
            (clipped[down].to.x - (20.0 + radius)).abs() < 0.1,
            "{clipped:#?}"
        );
        // nothing in the way: as it was
        let clear = [rapid(0.0, 0.0), plunge(0.0, 0.0), path(&[(30.0, 0.0)])[0]];
        assert_eq!(clip(&zone, &clear, radius, 5.0), clear.to_vec());
    }

    #[test]
    fn a_cylinder_cut_under_a_zone_keeps_its_cuts_outside_it() {
        // a raster over a 40 mm disc with a clamp across its middle: the cylinder less the zone
        let p = Point::new;
        let corners = [p(-25.0, -5.0), p(25.0, -5.0), p(25.0, 5.0), p(-25.0, 5.0)];
        let zone = [Path::polyline(&corners, true)];
        let mut moves = vec![rapid(-20.0, -20.0)];
        let mut y = -20.0;
        while y <= 20.0 {
            let half = (400.0_f64 - y * y).max(0.0).sqrt();
            moves.push(plunge(-half, y));
            moves.push(path(&[(half, y)])[0]);
            moves.push(rapid(half, y));
            y += 2.0;
        }
        let radius = 1.5;
        assert!(enters(&zone, &moves, radius));
        let clipped = clip(&zone, &moves, radius, 5.0);
        let cut_ys: Vec<f64> = clipped
            .iter()
            .filter(|m| m.kind == MoveKind::Cut)
            .map(|m| m.to.y)
            .collect();
        assert!(cut_ys.iter().any(|&y| y < -5.0) && cut_ys.iter().any(|&y| y > 5.0));
        assert!(
            cut_ys.iter().all(|&y| y.abs() >= 5.0 + radius),
            "{cut_ys:?}"
        );
        // the rapids then go round what is left of the zone
        let (routed, violates) = route(&zone, &clipped);
        assert!(!violates && !enters(&zone, &routed, radius));
    }

    #[test]
    fn a_toolpath_made_while_clipping_is_on_leaves_out_what_is_under_a_zone() {
        let mut doc = crate::testing::with_toolpath();
        doc.keep_out = vec![KeepOutZone::new("Clamp", square(10.0, 10.0, 10.0))];
        let mut t = doc.toolpaths.values().next().unwrap().clone();
        t.moves = vec![
            rapid(0.0, 15.0),
            plunge(0.0, 15.0),
            path(&[(30.0, 15.0)])[0],
        ];
        let radius = t.tool.diameter / 2.0;
        let made = |doc: &crate::document::Document| {
            let mut c = crate::Change::builder(doc);
            c.add_toolpath(t.clone());
            c.finish().toolpaths_making().next().unwrap().moves.clone()
        };
        // off: made as it is (the app refuses it)
        assert!(enters(&doc.keep_out_zones(), &made(&doc), radius));
        doc.clip_keep_out = true;
        let clipped = made(&doc);
        assert!(
            !enters(&doc.keep_out_zones(), &clipped, radius),
            "{clipped:#?}"
        );
        assert!(
            clipped
                .iter()
                .any(|m| m.kind == MoveKind::Cut && m.to.x > 20.0)
        );
        // as high as the job's safe height over it
        assert!(clipped.iter().any(|m| m.to.z >= doc.job.clearance));
    }

    #[test]
    fn zones_saved_before_they_had_names_open_as_unnamed() {
        let path = square(0.0, 0.0, 10.0);
        let bare = serde_json::to_value(&path).unwrap();
        let zone: KeepOutZone = serde_json::from_value(bare).unwrap();
        assert_eq!(zone, KeepOutZone::new(UNNAMED, path.clone()));
        let named = KeepOutZone::new("Clamp 1", path);
        let again: KeepOutZone =
            serde_json::from_value(serde_json::to_value(&named).unwrap()).unwrap();
        assert_eq!(again, named);
        assert_eq!(named.label(), "Keep-out: Clamp 1");
        assert_eq!(names("Clamp", 1, 1), ["Clamp"]);
        assert_eq!(names("Clamp", 2, 1), ["Clamp 1", "Clamp 2"]);
        assert_eq!(names(" ", 1, 3), ["Zone 3"]);
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
