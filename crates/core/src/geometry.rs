//! 2D geometry: points, rectangles and vector paths made of lines, arcs and cubic Béziers.
//! All coordinates are millimetres.

use glam::{DAffine2, DVec2};
use serde::{Deserialize, Serialize};
use std::f64::consts::{FRAC_PI_2, TAU};

pub type Point = DVec2;

/// Axis-aligned rectangle; `min` ≤ `max` component-wise.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Rect {
    pub min: Point,
    pub max: Point,
}

impl Rect {
    /// Rectangle spanning two corners, in either order.
    pub fn new(a: Point, b: Point) -> Self {
        Self {
            min: a.min(b),
            max: a.max(b),
        }
    }

    pub fn point(p: Point) -> Self {
        Self { min: p, max: p }
    }

    #[must_use]
    pub fn union(self, other: Self) -> Self {
        Self {
            min: self.min.min(other.min),
            max: self.max.max(other.max),
        }
    }

    #[must_use]
    pub fn include(self, p: Point) -> Self {
        self.union(Self::point(p))
    }

    pub fn center(self) -> Point {
        (self.min + self.max) / 2.0
    }

    pub fn size(self) -> DVec2 {
        self.max - self.min
    }
}

/// One span of a path, starting where the previous span ended.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Segment {
    Line {
        to: Point,
    },
    /// Circular arc around `center`; `to` must lie on the same circle as the start point.
    Arc {
        to: Point,
        center: Point,
        ccw: bool,
    },
    Cubic {
        c1: Point,
        c2: Point,
        to: Point,
    },
}

impl Segment {
    pub fn end(&self) -> Point {
        match *self {
            Self::Line { to } | Self::Arc { to, .. } | Self::Cubic { to, .. } => to,
        }
    }

    fn bounds(&self, from: Point) -> Rect {
        match *self {
            Self::Line { to } => Rect::new(from, to),
            Self::Arc { to, center, ccw } => {
                let r = from.distance(center);
                let (a0, a1) = (angle(from - center), angle(to - center));
                (0..4)
                    .map(|k| f64::from(k) * FRAC_PI_2)
                    .filter(|&t| in_sweep(a0, a1, ccw, t))
                    .fold(Rect::new(from, to), |b, t| {
                        b.include(center + r * DVec2::from_angle(t))
                    })
            }
            Self::Cubic { c1, c2, to } => cubic_extrema(from, c1, c2, to)
                .into_iter()
                .fold(Rect::new(from, to), |b, t| {
                    b.include(cubic_at(from, c1, c2, to, t))
                }),
        }
    }
}

/// A connected run of segments. A closed path's last segment ends back at `start`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Path {
    pub start: Point,
    pub segments: Vec<Segment>,
    pub closed: bool,
}

impl Path {
    /// Straight lines through `points`; closing adds the line back to the first point.
    pub fn polyline(points: &[Point], closed: bool) -> Self {
        let mut segments: Vec<Segment> =
            points[1..].iter().map(|&to| Segment::Line { to }).collect();
        if closed {
            segments.push(Segment::Line { to: points[0] });
        }
        Self {
            start: points[0],
            segments,
            closed,
        }
    }

    /// Full circle as two counter-clockwise half arcs, starting at its rightmost point.
    pub fn circle(center: Point, radius: f64) -> Self {
        let right = center + DVec2::X * radius;
        let left = center - DVec2::X * radius;
        Self {
            start: right,
            segments: vec![
                Segment::Arc {
                    to: left,
                    center,
                    ccw: true,
                },
                Segment::Arc {
                    to: right,
                    center,
                    ccw: true,
                },
            ],
            closed: true,
        }
    }

    pub fn bounds(&self) -> Rect {
        let mut from = self.start;
        let mut bounds = Rect::point(from);
        for s in &self.segments {
            bounds = bounds.union(s.bounds(from));
            from = s.end();
        }
        bounds
    }

    /// The path under `t`. Arcs survive similarity transforms (mirrors flip their direction);
    /// any other transform turns them into cubics, which stay exact under affine maps.
    #[must_use]
    pub fn transformed(&self, t: &DAffine2) -> Self {
        let (x, y) = (t.matrix2.x_axis, t.matrix2.y_axis);
        let similarity =
            (x.length() - y.length()).abs() < 1e-12 * x.length().max(1.0) && x.dot(y).abs() < 1e-12;
        let mirror = t.matrix2.determinant() < 0.0;
        let p = |q: Point| t.transform_point2(q);

        let mut from = self.start;
        let mut segments = Vec::with_capacity(self.segments.len());
        for &s in &self.segments {
            match s {
                Segment::Line { to } => segments.push(Segment::Line { to: p(to) }),
                Segment::Cubic { c1, c2, to } => segments.push(Segment::Cubic {
                    c1: p(c1),
                    c2: p(c2),
                    to: p(to),
                }),
                Segment::Arc { to, center, ccw } if similarity => {
                    segments.push(Segment::Arc {
                        to: p(to),
                        center: p(center),
                        ccw: ccw != mirror,
                    });
                }
                Segment::Arc { to, center, ccw } => {
                    segments.extend(arc_to_cubics(from, to, center, ccw).into_iter().map(
                        |c| match c {
                            Segment::Cubic { c1, c2, to } => Segment::Cubic {
                                c1: p(c1),
                                c2: p(c2),
                                to: p(to),
                            },
                            other => other,
                        },
                    ));
                }
            }
            from = s.end();
        }
        Self {
            start: p(self.start),
            segments,
            closed: self.closed,
        }
    }
}

fn angle(v: DVec2) -> f64 {
    v.y.atan2(v.x)
}

/// Is angle `t` within the sweep from `a0` to `a1`? Equal endpoints mean a full turn.
fn in_sweep(a0: f64, a1: f64, ccw: bool, t: f64) -> bool {
    let (span, offset) = if ccw {
        ((a1 - a0).rem_euclid(TAU), (t - a0).rem_euclid(TAU))
    } else {
        ((a0 - a1).rem_euclid(TAU), (a0 - t).rem_euclid(TAU))
    };
    span == 0.0 || offset <= span
}

/// Signed sweep angle of an arc (positive = counter-clockwise); a closed arc sweeps a full turn.
fn sweep(from: Point, to: Point, center: Point, ccw: bool) -> f64 {
    let (a0, a1) = (angle(from - center), angle(to - center));
    let span = if ccw {
        (a1 - a0).rem_euclid(TAU)
    } else {
        (a0 - a1).rem_euclid(TAU)
    };
    let span = if span == 0.0 { TAU } else { span };
    if ccw { span } else { -span }
}

/// Approximates an arc with cubics of at most 90° each (radial error under 0.03%).
pub fn arc_to_cubics(from: Point, to: Point, center: Point, ccw: bool) -> Vec<Segment> {
    let r = from.distance(center);
    let total = sweep(from, to, center, ccw);
    // ponytail: at most 4 pieces, so the cast cannot truncate
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "1..=4"
    )]
    let n = (total.abs() / FRAC_PI_2).ceil().max(1.0) as u32;
    let step = total / f64::from(n);
    let k = 4.0 / 3.0 * (step / 4.0).tan();
    let a0 = angle(from - center);
    (0..n)
        .map(|i| {
            let (t0, t1) = (a0 + step * f64::from(i), a0 + step * f64::from(i + 1));
            let (u0, u1) = (DVec2::from_angle(t0), DVec2::from_angle(t1));
            let p0 = center + r * u0;
            let p3 = if i + 1 == n { to } else { center + r * u1 };
            Segment::Cubic {
                c1: p0 + k * r * u0.perp(),
                c2: p3 - k * r * u1.perp(),
                to: p3,
            }
        })
        .collect()
}

fn cubic_at(p0: Point, p1: Point, p2: Point, p3: Point, t: f64) -> Point {
    let u = 1.0 - t;
    u * u * u * p0 + 3.0 * u * u * t * p1 + 3.0 * u * t * t * p2 + t * t * t * p3
}

/// Parameters in (0, 1) where the cubic's x or y derivative is zero.
fn cubic_extrema(p0: Point, p1: Point, p2: Point, p3: Point) -> Vec<f64> {
    let a = -p0 + 3.0 * p1 - 3.0 * p2 + p3;
    let b = 2.0 * (p0 - 2.0 * p1 + p2);
    let c = p1 - p0;
    let mut roots = Vec::new();
    for (a, b, c) in [(a.x, b.x, c.x), (a.y, b.y, c.y)] {
        if a.abs() < 1e-12 {
            if b.abs() > 1e-12 {
                roots.push(-c / b);
            }
        } else {
            let disc = b * b - 4.0 * a * c;
            if disc >= 0.0 {
                let s = disc.sqrt();
                roots.extend([(-b + s) / (2.0 * a), (-b - s) / (2.0 * a)]);
            }
        }
    }
    roots.retain(|t| *t > 0.0 && *t < 1.0);
    roots
}
