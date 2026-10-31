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

    pub fn contains(self, p: Point) -> bool {
        p.cmpge(self.min).all() && p.cmple(self.max).all()
    }

    pub fn contains_rect(self, other: Self) -> bool {
        self.contains(other.min) && self.contains(other.max)
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

    /// Points along the path whose connecting chords stay within `tolerance` of it.
    pub fn flatten(&self, tolerance: f64) -> Vec<Point> {
        let mut from = self.start;
        let mut points = vec![from];
        for &s in &self.segments {
            match s {
                Segment::Line { to } => points.push(to),
                Segment::Arc { to, center, ccw } => {
                    let r = from.distance(center);
                    let total = arc_sweep(from, to, center, ccw);
                    // chord sag r(1 - cos(step/2)) <= tolerance
                    let max_step = 2.0 * (1.0 - (tolerance / r).min(1.0)).acos();
                    let n = pieces(total.abs() / max_step);
                    let a0 = angle(from - center);
                    let step = total / f64::from(n);
                    points.extend(
                        (1..n).map(|i| center + r * DVec2::from_angle(a0 + step * f64::from(i))),
                    );
                    points.push(to);
                }
                Segment::Cubic { c1, c2, to } => {
                    // uniform steps: error <= 3/4 * max|second difference| / n^2
                    let m = (from - 2.0 * c1 + c2)
                        .length()
                        .max((c1 - 2.0 * c2 + to).length());
                    let n = pieces((0.75 * m / tolerance).sqrt());
                    points.extend(
                        (1..n).map(|i| cubic_at(from, c1, c2, to, f64::from(i) / f64::from(n))),
                    );
                    points.push(to);
                }
            }
            from = s.end();
        }
        points
    }

    /// The same path walked from its end back to its start.
    #[must_use]
    pub fn reversed(&self) -> Self {
        let mut points = vec![self.start];
        points.extend(self.segments.iter().map(Segment::end));
        let end = points.pop().expect("a path has a start");
        let segments = self
            .segments
            .iter()
            .zip(points)
            .rev()
            .map(|(s, to)| match *s {
                Segment::Line { .. } => Segment::Line { to },
                Segment::Arc { center, ccw, .. } => Segment::Arc {
                    to,
                    center,
                    ccw: !ccw,
                },
                Segment::Cubic { c1, c2, .. } => Segment::Cubic { c1: c2, c2: c1, to },
            })
            .collect();
        Self {
            start: end,
            segments,
            closed: self.closed,
        }
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

/// The point `radius` from `center` at `degrees` counter-clockwise from +X.
pub fn polar(center: Point, radius: f64, degrees: f64) -> Point {
    center + radius * DVec2::from_angle(degrees.to_radians())
}

/// Shortest distance from `at` to the polyline through `points`.
/// Whether `p` is inside the closed `rings`; a ring inside another is a hole (even-odd).
pub fn inside(rings: &[Vec<Point>], p: Point) -> bool {
    let mut inside = false;
    for ring in rings {
        for (i, &a) in ring.iter().enumerate() {
            let b = ring[(i + 1) % ring.len()];
            if (a.y > p.y) != (b.y > p.y) && p.x < a.x + (p.y - a.y) * (b.x - a.x) / (b.y - a.y) {
                inside = !inside;
            }
        }
    }
    inside
}

/// Twice the signed area of a closed ring: positive when it runs anticlockwise.
pub fn winding(points: &[Point]) -> f64 {
    let n = points.len();
    (0..n)
        .map(|i| points[i].perp_dot(points[(i + 1) % n]))
        .sum()
}

/// The point on the polyline through `points` nearest `at`.
pub fn polyline_closest(points: &[Point], at: Point) -> Option<Point> {
    if let [only] = points {
        return Some(*only);
    }
    points
        .windows(2)
        .map(|w| {
            let (a, d) = (w[0], w[1] - w[0]);
            let t = if d.length_squared() > 0.0 {
                ((at - a).dot(d) / d.length_squared()).clamp(0.0, 1.0)
            } else {
                0.0
            };
            a + d * t
        })
        .min_by(|p, q| p.distance(at).total_cmp(&q.distance(at)))
}

pub fn polyline_distance(points: &[Point], at: Point) -> f64 {
    if let [only] = points {
        return only.distance(at);
    }
    points
        .windows(2)
        .map(|w| {
            let (a, d) = (w[0], w[1] - w[0]);
            let t = if d.length_squared() > 0.0 {
                ((at - a).dot(d) / d.length_squared()).clamp(0.0, 1.0)
            } else {
                0.0
            };
            (a + d * t).distance(at)
        })
        .fold(f64::INFINITY, f64::min)
}

/// Does any part of the polyline through `points` lie inside or on `r`?
pub fn polyline_touches(points: &[Point], r: Rect) -> bool {
    points.iter().any(|&p| r.contains(p))
        || points.windows(2).any(|w| segment_touches(w[0], w[1], r))
}

/// Liang–Barsky: clips the segment against `area`; it touches if any of it survives.
fn segment_touches(from: Point, to: Point, area: Rect) -> bool {
    let along = to - from;
    let (mut enter, mut leave) = (0.0_f64, 1.0_f64);
    // (rate the segment approaches an edge, distance inside that edge at the start)
    for (rate, inside) in [
        (-along.x, from.x - area.min.x),
        (along.x, area.max.x - from.x),
        (-along.y, from.y - area.min.y),
        (along.y, area.max.y - from.y),
    ] {
        if rate == 0.0 {
            if inside < 0.0 {
                return false;
            }
        } else {
            let crossing = inside / rate;
            if rate < 0.0 {
                enter = enter.max(crossing);
            } else {
                leave = leave.min(crossing);
            }
            if enter > leave {
                return false;
            }
        }
    }
    true
}

pub(crate) fn angle(v: DVec2) -> f64 {
    v.y.atan2(v.x)
}

/// Is angle `t` within the sweep from `a0` to `a1`? Equal endpoints mean a full turn.
pub(crate) fn in_sweep(a0: f64, a1: f64, ccw: bool, t: f64) -> bool {
    let (span, offset) = if ccw {
        ((a1 - a0).rem_euclid(TAU), (t - a0).rem_euclid(TAU))
    } else {
        ((a0 - a1).rem_euclid(TAU), (a0 - t).rem_euclid(TAU))
    };
    span == 0.0 || offset <= span
}

/// Signed sweep angle of an arc (positive = counter-clockwise); a closed arc sweeps a full turn.
pub fn arc_sweep(from: Point, to: Point, center: Point, ccw: bool) -> f64 {
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
    let total = arc_sweep(from, to, center, ccw);
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

/// Number of pieces needed (at least 1), from a fractional estimate.
fn pieces(estimate: f64) -> u32 {
    // ponytail: capped so a degenerate tolerance cannot allocate unboundedly
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "clamped to 1..=10_000"
    )]
    let n = estimate.ceil().clamp(1.0, 10_000.0) as u32;
    n
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
