//! Each piece lying on its side, as a height above its middle at every point of its side
//! view: its foot at `x = 0`, its top at `x = height`, `y` across it. Cut from above, that
//! height is the top half; cut from below too (a two-sided job), it is the whole piece.
//!
//! Every shape is made so a cutter from above reaches all of it (nothing overhangs), which
//! is what lets it be cut on a 3-axis router.

#![expect(
    clippy::many_single_char_names,
    reason = "side-view points are x and y, distances d and radii r"
)]

use crate::profile::{Piece, Profile};

/// A point of a piece's side view: how far inside its outline (mm, negative outside) and
/// how high it stands there above the middle.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Sample {
    pub inside: f64,
    pub height: f64,
}

const OUTSIDE: Sample = Sample {
    inside: -1.0,
    height: 0.0,
};

/// A flat part (the knight's head, the king's cross): an outline in the side view, `half`
/// thick each side of the middle, its edges rounded over `edge` mm.
struct Plate {
    outline: Vec<(f64, f64)>,
    half: f64,
    edge: f64,
    /// How much higher its middle stands than its edges (a rounded cheek), and how far in
    /// that takes.
    dome: (f64, f64),
    /// Small hollows (an eye): centre, radius, depth.
    dimples: Vec<((f64, f64), f64, f64)>,
}

impl Plate {
    fn sample(&self, x: f64, y: f64) -> Sample {
        let d = distance_inside(&self.outline, (x, y));
        if d <= 0.0 {
            return Sample {
                inside: d,
                height: 0.0,
            };
        }
        // a quarter ellipse: upright at the edge, flat after `edge`
        let t = (1.0 - d / self.edge).max(0.0);
        let (rise, over) = self.dome;
        let u = (d / over).min(1.0);
        let mut height = (self.half - rise + rise * u * (2.0 - u)) * (1.0 - t * t).sqrt();
        for &((cx, cy), r, depth) in &self.dimples {
            let q = ((x - cx).powi(2) + (y - cy).powi(2)) / (r * r);
            if q < 1.0 {
                height -= depth * (1.0 - q).sqrt();
            }
        }
        Sample { inside: d, height }
    }
}

/// How far `p` is inside the closed `outline` (negative outside).
fn distance_inside(outline: &[(f64, f64)], p: (f64, f64)) -> f64 {
    let mut nearest = f64::INFINITY;
    let mut inside = false;
    for (i, &a) in outline.iter().enumerate() {
        let b = outline[(i + 1) % outline.len()];
        let (dx, dy) = (b.0 - a.0, b.1 - a.1);
        let t = (((p.0 - a.0) * dx + (p.1 - a.1) * dy) / (dx * dx + dy * dy)).clamp(0.0, 1.0);
        let (qx, qy) = (a.0 + t * dx - p.0, a.1 + t * dy - p.1);
        nearest = nearest.min((qx * qx + qy * qy).sqrt());
        if (a.1 > p.1) != (b.1 > p.1) && p.0 < a.0 + (p.1 - a.1) / (b.1 - a.1) * (b.0 - a.0) {
            inside = !inside;
        }
    }
    if inside { nearest } else { -nearest }
}

/// A straight slot: a point on its middle line, the line's direction, its width.
type Slit = ((f64, f64), (f64, f64), f64);

/// A piece's side view, ready to sample.
pub struct Shape {
    pub piece: Piece,
    profile: Profile,
    plates: Vec<Plate>,
    /// Rook battlements: from this far up, slots `depth` deep into the tower's wall.
    battlements: Option<(f64, f64)>,
    /// The bishop's mitre slit: a point on it, its direction, its width.
    slit: Option<Slit>,
}

impl Shape {
    pub fn new(piece: Piece) -> Self {
        let (r, h) = piece.size();
        let profile = piece.profile();
        let mut plates = Vec::new();
        let mut battlements = None;
        let mut slit = None;
        match piece {
            Piece::King => {
                // a cross standing on the crown, a little sunk into it
                let foot = profile.height() - 0.06 * r;
                let (w, arm, bar, bar_h) = (0.13 * r, 0.32 * r, h - 0.30 * r, 0.11 * r);
                let outline = vec![
                    (foot, -w),
                    (bar - bar_h, -w),
                    (bar - bar_h, -arm),
                    (bar + bar_h, -arm),
                    (bar + bar_h, -w),
                    (h, -w),
                    (h, w),
                    (bar + bar_h, w),
                    (bar + bar_h, arm),
                    (bar - bar_h, arm),
                    (bar - bar_h, w),
                    (foot, w),
                ];
                plates.push(Plate {
                    outline,
                    half: 0.17 * r,
                    edge: 0.07 * r,
                    dome: (0.0, 1.0),
                    dimples: Vec::new(),
                });
            }
            Piece::Knight => plates.push(knight_head(r, h)),
            Piece::Rook => battlements = Some((h - 0.32 * r, 0.26 * r)),
            Piece::Bishop => {
                // from the front of the mitre, slanting down to just past its middle;
                // 3.2 mm wide, so a 1/8″ cutter fits it
                let mitre = 0.52 * h + 0.30 * r;
                let at = (mitre + 0.62 * (h - 0.24 * r - mitre), 0.0);
                slit = Some((at, (0.5, 0.866), 3.2));
            }
            _ => {}
        }
        Self {
            piece,
            profile,
            plates,
            battlements,
            slit,
        }
    }

    /// Its extent in the side view: (length along it, half its width).
    pub fn extent(&self) -> (f64, f64) {
        let (r, h) = self.piece.size();
        (h, r)
    }

    pub fn sample(&self, x: f64, y: f64) -> Sample {
        let r = self.profile.radius_at(x);
        let mut best = OUTSIDE;
        if r > 0.0 {
            let inside = r - y.abs();
            let mut height = (r * r - y * y).max(0.0).sqrt();
            if let Some((from, depth)) = self.battlements
                && x > from
                && inside > 0.0
            {
                // four slots round the tower, at 45° either side of straight up
                let angle = height.atan2(y).to_degrees();
                if (angle - 45.0).abs() < 16.0 || (angle - 135.0).abs() < 16.0 {
                    height = ((r - depth).powi(2) - y * y).max(0.0).sqrt();
                }
            }
            best = Sample { inside, height };
            if let Some(((sx, sy), (dx, dy), width)) = self.slit
                && y > -0.08 * r
            {
                // across the slit, from its middle line; it starts behind the middle
                let off = ((x - sx) * dy - (y - sy) * dx).abs() - width / 2.0;
                let start = y + 0.08 * r;
                let cut = off.max(-start);
                if cut < 0.0 {
                    best = Sample {
                        inside: best.inside.min(cut),
                        height,
                    };
                }
            }
        }
        for plate in &self.plates {
            let s = plate.sample(x, y);
            if s.inside > 0.0 && (best.inside <= 0.0 || s.height > best.height) {
                best = Sample {
                    inside: s.inside.max(best.inside),
                    height: s.height,
                };
            } else if best.inside <= 0.0 {
                best.inside = best.inside.max(s.inside);
            }
        }
        best
    }
}

/// The knight's head and neck, seen from the side facing +y, standing on its collar.
fn knight_head(r: f64, h: f64) -> Plate {
    // (up, forward) in mm for a 60 mm knight with a 15.5 mm foot, scaled to this one
    const SIDE: [(f64, f64); 22] = [
        (16.0, -9.0),
        (26.0, -11.0),
        (36.0, -11.5),
        (45.0, -10.0),
        (52.0, -7.0),
        (57.0, -6.0),
        (60.0, -4.0),
        (56.0, -2.0),
        (57.5, 0.0),
        (54.0, 2.0),
        (50.0, 6.0),
        (44.0, 11.0),
        (39.0, 14.5),
        (36.0, 15.0),
        (33.5, 13.5),
        (34.0, 11.0),
        (32.0, 9.0),
        (31.5, 6.0),
        (30.0, 3.0),
        (25.0, 5.5),
        (20.0, 8.0),
        (16.0, 9.0),
    ];
    let (sx, sy) = (h / 60.0, r / 15.5);
    Plate {
        outline: SIDE.iter().map(|&(x, y)| (x * sx, y * sy)).collect(),
        half: 0.46 * r,
        edge: 0.20 * r,
        dome: (0.14 * r, 0.6 * r),
        dimples: vec![
            ((46.0 * sx, 5.5 * sy), 1.6 * sy, 1.2),
            ((37.5 * sx, 12.5 * sy), 1.0 * sy, 0.8),
        ],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_turned_piece_is_round_in_section() {
        let pawn = Shape::new(Piece::Pawn);
        let (r, _) = Piece::Pawn.size();
        let s = pawn.sample(0.03 * r, 0.0);
        assert!(
            (s.height - r).abs() < 1e-9 && (s.inside - r).abs() < 1e-9,
            "{s:?}"
        );
        let s = pawn.sample(0.03 * r, 0.6 * r);
        assert!((s.height - 0.8 * r).abs() < 1e-9, "{s:?}");
        assert!(pawn.sample(0.03 * r, 1.1 * r).inside < 0.0);
    }

    #[test]
    fn the_knight_has_a_head_and_the_king_a_cross() {
        let knight = Shape::new(Piece::Knight);
        // the muzzle sticks out in front, past the turned part
        let muzzle = knight.sample(36.0, 13.0);
        assert!(muzzle.inside > 0.0 && muzzle.height > 3.0, "{muzzle:?}");
        assert!(knight.sample(36.0, -13.5).inside < 0.0, "behind the mane");
        let eye = knight.sample(46.0, 5.5);
        let cheek = knight.sample(46.0, 3.0);
        assert!(eye.height < cheek.height - 0.5, "{eye:?} {cheek:?}");

        let king = Shape::new(Piece::King);
        let (r, h) = Piece::King.size();
        let arm = king.sample(h - 0.30 * r, 0.25 * r);
        assert!(arm.inside > 0.0, "{arm:?}");
        assert!(
            king.sample(h - 0.05 * r, 0.25 * r).inside < 0.0,
            "above the arm"
        );
    }

    #[test]
    fn the_rook_has_slots_in_its_battlements() {
        let rook = Shape::new(Piece::Rook);
        let (r, h) = Piece::Rook.size();
        let x = h - 0.1 * r;
        let rr = 0.76 * r;
        let slot = rook.sample(x, rr * 45f64.to_radians().cos());
        let merlon = rook.sample(x, 0.0);
        assert!(merlon.height > rr - 1e-6, "{merlon:?}");
        assert!(
            slot.height < rr * 45f64.to_radians().sin() - 1.0,
            "{slot:?}"
        );
    }

    #[test]
    fn the_bishop_has_a_slit_in_the_front_of_its_mitre() {
        let bishop = Shape::new(Piece::Bishop);
        let (r, h) = Piece::Bishop.size();
        let mitre = 0.52 * h + 0.30 * r;
        let x = mitre + 0.62 * (h - 0.24 * r - mitre);
        assert!(bishop.sample(x, 0.0).inside < 0.0, "the slit");
        assert!(bishop.sample(x, -0.2 * r).inside > 0.0, "the back is whole");
        assert!(bishop.sample(x + 3.0, 0.0).inside > 0.0, "beside the slit");
    }

    #[test]
    fn inside_and_outside_an_outline() {
        let square = [(0.0, 0.0), (10.0, 0.0), (10.0, 10.0), (0.0, 10.0)];
        assert!((distance_inside(&square, (5.0, 2.0)) - 2.0).abs() < 1e-12);
        assert!((distance_inside(&square, (13.0, 5.0)) + 3.0).abs() < 1e-12);
    }
}
