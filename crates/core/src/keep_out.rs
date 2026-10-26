//! Keep-out zones: areas of the material a tool must not pass over, such as clamps (Aspire's
//! help: Keep-out Zones). They are closed paths of their own, made from selected vectors and
//! kept when those are deleted.

use crate::geometry::{Path, Point, inside};
use crate::toolpath::Move;

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
