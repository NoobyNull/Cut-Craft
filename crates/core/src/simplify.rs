//! Toolpath tolerance (Options ▸ Toolpath Settings): cutting points that lie within
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
        || s.starts_with("rotary_")
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
    kept_moves(moves, &[], &[], tolerance)
        .into_iter()
        .map(|i| moves[i])
        .collect()
}

impl Toolpath {
    /// This toolpath with its cuts thinned to `tolerance` (see [`simplify`]), each move's place
    /// across the turning axis and laser power kept with it: a point where the bit moves
    /// across, or the power changes, stays. Lists that don't match the moves are left as they
    /// are, all of it, for the post to refuse.
    #[must_use]
    pub fn thinned(&self, tolerance: f64) -> Self {
        let n = self.moves.len();
        let fits = |v: &[f64]| v.is_empty() || v.len() == n;
        if tolerance <= 0.0 || !fits(&self.across) || !fits(&self.power) {
            return self.clone();
        }
        let keep = kept_moves(&self.moves, &self.across, &self.power, tolerance);
        let pick = |v: &[f64]| {
            if v.is_empty() {
                vec![]
            } else {
                keep.iter().map(|&i| v[i]).collect()
            }
        };
        Self {
            moves: keep.iter().map(|&i| self.moves[i]).collect(),
            across: pick(&self.across),
            power: pick(&self.power),
            ..self.clone()
        }
    }
}

/// The indices of the moves thinning keeps: every move but a cut, and of each run of cuts at
/// one power and held at one place across the axis the points Douglas–Peucker keeps. `across` and `power` are empty or one a move.
fn kept_moves(moves: &[Move], across: &[f64], power: &[f64], tolerance: f64) -> Vec<usize> {
    let at = |v: &[f64], i: usize| v.get(i).copied().unwrap_or(0.0);
    let mut out = Vec::with_capacity(moves.len());
    let mut i = 0;
    while i < moves.len() {
        if moves[i].kind != MoveKind::Cut || out.is_empty() {
            out.push(i);
            i += 1;
            continue;
        }
        let (lit, held) = (at(power, i), at(across, i));
        // a run is held at one place across the axis too: the post turns a move that changes
        // it into A and Z, which a straight line between kept points doesn't follow
        let end = moves[i..]
            .iter()
            .enumerate()
            .position(|(k, m)| {
                m.kind != MoveKind::Cut
                    || at(power, i + k) != lit
                    || (at(across, i + k) - held).abs() > 1e-9
            })
            .map_or(moves.len(), |n| i + n);
        // the run starts where the move before it left the tool
        let from = out[out.len() - 1];
        let points: Vec<(DVec3, f64)> = std::iter::once(from)
            .chain(i..end)
            .map(|k| (moves[k].to, at(across, k)))
            .collect();
        out.extend(
            kept(&points, tolerance)
                .into_iter()
                .skip(1)
                .map(|k| i + k - 1),
        );
        i = end;
    }
    out
}

/// The indices of `points` Douglas–Peucker keeps, in order; each point is a place and how far
/// across the axis it is held.
fn kept(points: &[(DVec3, f64)], tolerance: f64) -> Vec<usize> {
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

/// How far `p` is from the segment `a`–`b`, across the axis counted as a fourth direction.
fn off_line(point: (DVec3, f64), from: (DVec3, f64), to: (DVec3, f64)) -> f64 {
    let (line, line_across) = (to.0 - from.0, to.1 - from.1);
    let length = line.length_squared() + line_across * line_across;
    let along = if length > 0.0 {
        (((point.0 - from.0).dot(line) + (point.1 - from.1) * line_across) / length).clamp(0.0, 1.0)
    } else {
        0.0
    };
    let off_across = point.1 - (from.1 + line_across * along);
    (point.0.distance_squared(from.0 + line * along) + off_across * off_across).sqrt()
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

    /// A toolpath of `moves` with `across` and `power` beside them.
    fn with(moves: Vec<Move>, across: Vec<f64>, power: Vec<f64>) -> Toolpath {
        Toolpath {
            name: "Line".into(),
            strategy: "profile_toolpath".into(),
            params: serde_json::Value::Null,
            tool: crate::toolpath::Tool::default(),
            vectors: vec![],
            moves,
            visible: true,
            power,
            stale: false,
            group: None,
            drill_bank: None,
            fallbacks: vec![],
            across,
        }
    }

    #[test]
    fn thinning_keeps_each_moves_place_across_the_axis_and_laser_power_with_it() {
        let line = vec![
            cut(0.0, 0.0, -1.0),
            cut(1.0, 0.0, -1.0),
            cut(2.0, 0.0, -1.0),
            cut(3.0, 0.0, -1.0),
            cut(4.0, 0.0, -1.0),
        ];
        // held the same way across all along: the middle goes, across thinned with it
        let same = with(line.clone(), vec![2.0; 5], vec![]).thinned(0.01);
        assert_eq!(same.moves.len(), 2);
        assert_eq!(same.across, [2.0, 2.0]);
        // the bit moves across as it goes: every point stays, with where it was held, as the
        // post turns each into A and Z and a straight line between fewer of them wouldn't
        // follow the cut (held -10, 0 and 10 at radius 15, the middle is 3.8 mm higher)
        let moved = with(line.clone(), vec![0.0, 1.5, 3.0, 1.5, 0.0], vec![]).thinned(0.01);
        let kept: Vec<(f64, f64)> = moved
            .moves
            .iter()
            .zip(&moved.across)
            .map(|(m, &a)| (m.to.x, a))
            .collect();
        assert_eq!(
            kept,
            [(0.0, 0.0), (1.0, 1.5), (2.0, 3.0), (3.0, 1.5), (4.0, 0.0)]
        );
        // a laser's power changing part way: the move before the change ends where it did, and
        // the lit run after it is thinned on its own
        let lit = with(line.clone(), vec![], vec![50.0, 50.0, 80.0, 80.0, 80.0]).thinned(0.01);
        let kept: Vec<(f64, f64)> = lit
            .moves
            .iter()
            .zip(&lit.power)
            .map(|(m, &p)| (m.to.x, p))
            .collect();
        assert_eq!(kept, [(0.0, 50.0), (1.0, 50.0), (4.0, 80.0)]);
        // lists that already don't match their moves are left for the post to refuse
        let odd = with(line, vec![1.0, 2.0], vec![]).thinned(0.01);
        assert_eq!((odd.moves.len(), odd.across.len()), (5, 2));
    }

    #[test]
    fn rotary_carving_and_lathe_toolpaths_take_the_3d_tolerance() {
        for strategy in [
            "rotary_machining_toolpath",
            "rotary_lathe_toolpath",
            "rotary_roughing_toolpath",
            "rotary_finishing_toolpath",
            "rotary_locked_angles_toolpath",
            "rotary_details_toolpath",
        ] {
            let t = Toolpath {
                strategy: strategy.into(),
                ..with(vec![], vec![], vec![])
            };
            assert_eq!(family(&t), Family::ThreeD, "{strategy}");
        }
    }
}
