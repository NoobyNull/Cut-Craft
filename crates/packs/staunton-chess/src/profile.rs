//! The turned outline of each piece: its radius at each height, drawn from the classic
//! Staunton pattern of 1849 (long out of copyright) and modelled here from scratch.
#![expect(
    clippy::many_single_char_names,
    reason = "an outline's points are heights y and radii r"
)]

use std::f64::consts::PI;

/// A piece's outline as (height up its axis, radius) points, bottom to top, heights never
/// going down; millimetres.
#[derive(Clone, Debug, Default)]
pub struct Profile {
    pub points: Vec<(f64, f64)>,
}

impl Profile {
    fn start() -> Self {
        Self {
            points: vec![(0.0, 0.0)],
        }
    }

    fn last(&self) -> (f64, f64) {
        *self.points.last().expect("a profile starts with a point")
    }

    fn to(&mut self, y: f64, r: f64) -> &mut Self {
        self.points.push((y, r));
        self
    }

    /// A quadratic curve from the last point to `(y, r)`, pulled towards `(cy, cr)`.
    fn curve(&mut self, (cy, cr): (f64, f64), (y, r): (f64, f64), steps: u32) -> &mut Self {
        let (y0, r0) = self.last();
        for i in 1..=steps {
            let t = f64::from(i) / f64::from(steps);
            let (a, b, c) = ((1.0 - t) * (1.0 - t), 2.0 * (1.0 - t) * t, t * t);
            self.points
                .push((a * y0 + b * cy + c * y, a * r0 + b * cr + c * r));
        }
        self
    }

    /// Part of a circle centred at height `cy`, radius `cr` out from the axis, from angle
    /// `from` to `to` in degrees: 0 points straight out, 90 straight up.
    fn arc(
        &mut self,
        (cy, cr): (f64, f64),
        radius: f64,
        from: f64,
        to: f64,
        steps: u32,
    ) -> &mut Self {
        for i in 0..=steps {
            let a = (from + (to - from) * f64::from(i) / f64::from(steps)) * PI / 180.0;
            self.points
                .push((cy + radius * a.sin(), cr + radius * a.cos()));
        }
        self
    }

    /// A rounded ring sticking out to radius `r`, `thick` high, centred at height `y`.
    fn ring(&mut self, y: f64, r: f64, thick: f64) -> &mut Self {
        self.arc((y, r - thick / 2.0), thick / 2.0, -90.0, 90.0, 8)
    }

    /// The radius at height `y`: the widest the outline is there (0 above or below it).
    pub fn radius_at(&self, y: f64) -> f64 {
        self.points
            .windows(2)
            .filter_map(|w| {
                let ((y0, r0), (y1, r1)) = (w[0], w[1]);
                if y < y0.min(y1) || y > y0.max(y1) {
                    None
                } else if (y1 - y0).abs() < 1e-12 {
                    Some(r0.max(r1))
                } else {
                    Some(r0 + (r1 - r0) * (y - y0) / (y1 - y0))
                }
            })
            .fold(0.0, f64::max)
    }

    pub fn height(&self) -> f64 {
        self.points.iter().map(|p| p.0).fold(0.0, f64::max)
    }
}

/// The stepped foot every piece stands on, `r` its radius; the outline ends at the stem.
fn foot(r: f64) -> (Profile, f64) {
    let mut p = Profile::start();
    p.to(0.0, r).to(0.07 * r, r);
    p.ring(0.16 * r, r, 0.18 * r);
    p.to(0.27 * r, 0.90 * r);
    p.curve((0.30 * r, 0.97 * r), (0.42 * r, 0.86 * r), 8);
    p.curve((0.50 * r, 0.72 * r), (0.52 * r, 0.60 * r), 6);
    (p, 0.52 * r)
}

/// A gently waisted stem from the last point up to `(y, r)`.
fn stem(p: &mut Profile, base_r: f64, (y, r): (f64, f64), waist: f64) {
    let (y0, r0) = p.last();
    p.curve(
        (
            y0 + 0.25 * (y - y0),
            r0.min(r) - waist * base_r + 0.05 * base_r,
        ),
        (y, r),
        12,
    );
}

/// The collar under a piece's head: a ring, and with `small` a smaller ring over it.
fn collar(p: &mut Profile, base_r: f64, y: f64, r: f64, small: Option<f64>) {
    let thick = 0.18 * base_r;
    p.ring(y, r, thick);
    if let Some(s) = small {
        p.to(y + thick / 2.0 + 0.03 * base_r, s);
        p.ring(
            y + thick / 2.0 + 0.11 * base_r,
            s + 0.08 * base_r,
            0.13 * base_r,
        );
    }
}

/// The pieces of the set.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Piece {
    Pawn,
    Rook,
    Knight,
    Bishop,
    Queen,
    King,
}

impl Piece {
    pub const ALL: [Self; 6] = [
        Self::King,
        Self::Queen,
        Self::Bishop,
        Self::Knight,
        Self::Rook,
        Self::Pawn,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Self::Pawn => "Pawn",
            Self::Rook => "Rook",
            Self::Knight => "Knight",
            Self::Bishop => "Bishop",
            Self::Queen => "Queen",
            Self::King => "King",
        }
    }

    /// How many of it one colour has.
    pub fn count(self) -> u32 {
        match self {
            Self::Pawn => 8,
            Self::Rook | Self::Knight | Self::Bishop => 2,
            Self::Queen | Self::King => 1,
        }
    }

    /// Its foot's radius and its height, mm: a 95 mm (3¾″) king, the tournament size, its
    /// foot 36 mm across so a half fits 3/4″ (19 mm) wood.
    pub fn size(self) -> (f64, f64) {
        match self {
            Self::Pawn => (13.5, 46.0),
            Self::Rook => (15.5, 52.0),
            Self::Knight => (15.5, 60.0),
            Self::Bishop => (15.5, 70.0),
            Self::Queen => (17.0, 85.0),
            Self::King => (18.0, 95.0),
        }
    }

    /// The turned part of it. The rook's battlements, the king's cross and the knight's
    /// head are added by [`crate::shape`].
    pub fn profile(self) -> Profile {
        let (r, h) = self.size();
        let (mut p, _) = foot(r);
        match self {
            Self::Pawn => {
                let ball = 0.60 * r;
                let centre = h - ball;
                let c = centre - ball - 0.02 * r;
                stem(&mut p, r, (c - 0.09 * r, 0.34 * r), 0.02);
                collar(&mut p, r, c, 0.64 * r, None);
                p.to(c + 0.10 * r, 0.30 * r);
                p.arc((centre, 0.0), ball, -58.0, 90.0, 16);
            }
            Self::Rook => {
                let top = h - 0.42 * r;
                p.curve((0.82 * r, 0.50 * r), (1.12 * r, 0.56 * r), 6);
                p.curve(
                    (f64::midpoint(1.12 * r, top), 0.50 * r),
                    (top - 0.16 * r, 0.58 * r),
                    12,
                );
                p.ring(top - 0.10 * r, 0.66 * r, 0.12 * r);
                p.to(top - 0.02 * r, 0.62 * r);
                p.curve((top, 0.62 * r), (top + 0.06 * r, 0.76 * r), 4);
                p.to(h, 0.76 * r).to(h, 0.0);
            }
            Self::Knight => {
                // the turned foot and a short stem; the head is cut from a side view
                p.curve((0.62 * r, 0.52 * r), (0.95 * r, 0.56 * r), 6);
                collar(&mut p, r, 1.05 * r, 0.66 * r, None);
                p.to(1.16 * r, 0.50 * r).to(1.16 * r, 0.0);
            }
            Self::Bishop => {
                let c = 0.52 * h;
                stem(&mut p, r, (c - 0.09 * r, 0.30 * r), 0.05);
                collar(&mut p, r, c, 0.62 * r, Some(0.40 * r));
                let m0 = c + 0.30 * r;
                let mh = h - 0.24 * r - m0;
                p.to(m0, 0.30 * r);
                p.curve((m0 + 0.2 * mh, 0.70 * r), (m0 + 0.55 * mh, 0.48 * r), 10);
                p.curve((m0 + 0.85 * mh, 0.30 * r), (m0 + mh, 0.08 * r), 8);
                p.arc((h - 0.11 * r, 0.0), 0.11 * r, -50.0, 90.0, 8);
            }
            Self::Queen | Self::King => {
                let queen = self == Self::Queen;
                let c = if queen { 0.56 * h } else { 0.50 * h };
                stem(&mut p, r, (c - 0.09 * r, 0.30 * r), 0.06);
                collar(&mut p, r, c, 0.62 * r, Some(0.42 * r));
                let k1 = h - if queen { 0.42 * r } else { 0.95 * r };
                p.to(c + 0.30 * r, 0.30 * r);
                p.curve(
                    (c + 0.30 * r + 0.6 * (k1 - c), 0.30 * r),
                    (k1 - 0.06 * r, 0.62 * r),
                    12,
                );
                p.ring(k1, 0.66 * r, 0.12 * r);
                if queen {
                    p.to(k1 + 0.07 * r, 0.50 * r);
                    p.curve((k1 + 0.15 * r, 0.30 * r), (k1 + 0.22 * r, 0.22 * r), 4);
                    p.arc((h - 0.14 * r, 0.0), 0.14 * r, -20.0, 90.0, 8);
                } else {
                    p.to(k1 + 0.07 * r, 0.55 * r);
                    p.curve((k1 + 0.14 * r, 0.55 * r), (k1 + 0.26 * r, 0.18 * r), 6);
                    p.to(k1 + 0.30 * r, 0.12 * r).to(k1 + 0.30 * r, 0.0);
                }
            }
        }
        p
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_outline_climbs_and_stays_inside_its_foot() {
        for piece in Piece::ALL {
            let p = piece.profile();
            let (r, h) = piece.size();
            for w in p.points.windows(2) {
                assert!(w[1].0 >= w[0].0 - 1e-9, "{piece:?} goes down at {w:?}");
            }
            for &(y, rad) in &p.points {
                assert!(rad >= -1e-9 && rad <= r + 1e-9, "{piece:?} at {y}: {rad}");
            }
            // the turned part of the king and knight stops short; the rest reach the top
            let top = p.height();
            assert!(top <= h + 1e-9, "{piece:?}");
            if !matches!(piece, Piece::King | Piece::Knight) {
                assert!((top - h).abs() < 0.01, "{piece:?} is {top} tall, not {h}");
            }
        }
    }

    #[test]
    fn the_foot_is_full_width_and_the_waist_narrow() {
        for piece in Piece::ALL {
            let p = piece.profile();
            let (r, _) = piece.size();
            assert!((p.radius_at(0.0) - r).abs() < 1e-9, "{piece:?}");
            assert!(p.radius_at(0.6 * r) < 0.75 * r, "{piece:?}");
            assert_eq!(p.radius_at(p.height() + 1.0), 0.0);
        }
    }

    #[test]
    fn the_pieces_rise_in_rank() {
        let heights: Vec<f64> = [
            Piece::Pawn,
            Piece::Rook,
            Piece::Knight,
            Piece::Bishop,
            Piece::Queen,
            Piece::King,
        ]
        .iter()
        .map(|p| p.size().1)
        .collect();
        assert!(heights.windows(2).all(|w| w[0] < w[1]), "{heights:?}");
        let total: u32 = Piece::ALL.iter().map(|p| p.count()).sum();
        assert_eq!(total, 16);
    }
}
