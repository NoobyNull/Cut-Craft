//! Toolpath tolerance (Aspire's Options ▸ Toolpath Settings): cutting points that lie within
//! the tolerance of the straight move past them are dropped, so the G-code is shorter.

use crate::toolpath::{Move, MoveKind, Toolpath};
use glam::DVec3;

/// Which of the Options' tolerances a toolpath takes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Family {
    TwoD,
    ThreeD,
    VCarve,
}

/// The family a toolpath's strategy belongs to.
// ponytail: by strategy; merged and array-copied toolpaths count as 2D
pub fn family(toolpath: &Toolpath) -> Family {
    let s = toolpath.strategy.as_str();
    if s.contains("vcarve") || matches!(s, "prism_toolpath" | "inlay_toolpath" | "chamfer_toolpath")
    {
        Family::VCarve
    } else if s.starts_with("3d_")
        || matches!(
            s,
            "texture_toolpath"
                | "moulding_toolpath"
                | "rounding_toolpath"
                | "sketch_carving_toolpath"
        )
    {
        Family::ThreeD
    } else {
        Family::TwoD
    }
}

/// `moves` with every cut point within `tolerance` of the line through its neighbours left
/// out (Douglas–Peucker over each run of cuts); other moves are kept as they are.
pub fn simplify(moves: &[Move], tolerance: f64) -> Vec<Move> {
    if tolerance <= 0.0 {
        return moves.to_vec();
    }
    let mut out = Vec::with_capacity(moves.len());
    let mut i = 0;
    while i < moves.len() {
        if moves[i].kind != MoveKind::Cut {
            out.push(moves[i]);
            i += 1;
            continue;
        }
        let end = moves[i..]
            .iter()
            .position(|m| m.kind != MoveKind::Cut)
            .map_or(moves.len(), |n| i + n);
        // the run starts where the move before it left the tool
        let Some(from) = out.last().map(|m: &Move| m.to) else {
            out.push(moves[i]);
            i += 1;
            continue;
        };
        let points: Vec<DVec3> = std::iter::once(from)
            .chain(moves[i..end].iter().map(|m| m.to))
            .collect();
        for k in kept(&points, tolerance).into_iter().skip(1) {
            out.push(Move {
                kind: MoveKind::Cut,
                to: points[k],
            });
        }
        i = end;
    }
    out
}

/// The indices of `points` Douglas–Peucker keeps, in order.
fn kept(points: &[DVec3], tolerance: f64) -> Vec<usize> {
    let n = points.len();
    let mut keep = vec![false; n];
    keep[0] = true;
    keep[n - 1] = true;
    // a stack, not recursion: a 3D run can be millions of points long
    let mut spans = vec![(0, n - 1)];
    while let Some((a, b)) = spans.pop() {
        let (far, distance) = (a + 1..b)
            .map(|k| (k, off_line(points[k], points[a], points[b])))
            .fold((a, 0.0), |best, d| if d.1 > best.1 { d } else { best });
        if distance > tolerance {
            keep[far] = true;
            spans.push((a, far));
            spans.push((far, b));
        }
    }
    (0..n).filter(|&k| keep[k]).collect()
}

/// How far `p` is from the segment `a`–`b`.
fn off_line(p: DVec3, a: DVec3, b: DVec3) -> f64 {
    let ab = b - a;
    let t = if ab.length_squared() > 0.0 {
        ((p - a).dot(ab) / ab.length_squared()).clamp(0.0, 1.0)
    } else {
        0.0
    };
    p.distance(a + ab * t)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cut(x: f64, y: f64, z: f64) -> Move {
        Move {
            kind: MoveKind::Cut,
            to: DVec3::new(x, y, z),
        }
    }

    #[test]
    fn points_on_the_line_go_and_corners_and_other_moves_stay() {
        let plunge = Move {
            kind: MoveKind::Plunge,
            to: DVec3::new(0.0, 0.0, -1.0),
        };
        let moves = [
            plunge,
            cut(1.0, 0.0, -1.0),
            cut(2.0, 0.001, -1.0), // within the tolerance of the line
            cut(3.0, 0.0, -1.0),
            cut(3.0, 3.0, -1.0), // a corner
            Move {
                kind: MoveKind::Rapid,
                to: DVec3::new(3.0, 3.0, 5.0),
            },
        ];
        let out = simplify(&moves, 0.01);
        let at: Vec<(f64, f64)> = out.iter().map(|m| (m.to.x, m.to.y)).collect();
        assert_eq!(at, [(0.0, 0.0), (3.0, 0.0), (3.0, 3.0), (3.0, 3.0)]);
        assert_eq!(out[0], plunge);
        assert_eq!(out[3].kind, MoveKind::Rapid);
        // nothing moves further than the tolerance from the cut it replaces
        assert_eq!(simplify(&moves, 0.0), moves);
        assert_eq!(simplify(&moves, 0.0001).len(), moves.len());
    }

    #[test]
    fn a_long_gentle_curve_keeps_only_what_the_tolerance_needs() {
        let arc: Vec<Move> = (0..=10_000)
            .map(|i| {
                let a = f64::from(i) / 10_000.0 * std::f64::consts::FRAC_PI_2;
                cut(100.0 * a.cos(), 100.0 * a.sin(), 0.0)
            })
            .collect();
        let out = simplify(&arc, 0.01);
        assert!(out.len() < 200, "{} points kept", out.len());
        for m in &out {
            assert!(
                (m.to.truncate().length() - 100.0).abs() < 1e-9,
                "kept points are on it"
            );
        }
    }
}
