//! Composed ornament for the carving styles (Baroque and Western leaf): rather than stems
//! grown at random, each piece is a whole scroll designed as a carver would draw one (a
//! tapering band sweeping round into a volute, leaves wrapped round its outside at set
//! points, a smaller scroll curling off it, a leaf or flower at its foot), sized to fit, the
//! biggest first, then smaller ones in the room left.

use crate::fill::{Grower, Probe, Stroke, along};
use crate::scroll::{Look, Made, acanthus, centre_line, circle, rosette, wavy_leaf};
use cut_craft_core::geometry::Point;
use std::f64::consts::{PI, TAU};

/// One scroll in its own frame (its foot at the origin heading along +x, curling
/// anticlockwise): the closed shapes that merge into one another, and the details apart.
#[derive(Clone, Default)]
struct Motif {
    shapes: Vec<Vec<Point>>,
    details: Vec<Stroke>,
}

impl Motif {
    fn map(&self, f: impl Fn(Point) -> Point) -> Self {
        Self {
            shapes: self
                .shapes
                .iter()
                .map(|s| s.iter().map(|&p| f(p)).collect())
                .collect(),
            details: self
                .details
                .iter()
                .map(|d| Stroke {
                    points: d.points.iter().map(|&p| f(p)).collect(),
                    closed: d.closed,
                })
                .collect(),
        }
    }

    /// Turned by `angle`, mirrored across its own x axis if `flip`, then moved by `by`.
    fn placed(&self, by: Point, angle: f64, flip: bool) -> Self {
        let turn = Point::from_angle(angle);
        let y = if flip { -1.0 } else { 1.0 };
        self.map(|p| by + turn.rotate(Point::new(p.x, p.y * y)))
    }

    fn add(&mut self, other: Self) {
        self.shapes.extend(other.shapes);
        self.details.extend(other.details);
    }

    fn centre(&self) -> Point {
        let (mut lo, mut hi) = (Point::splat(f64::INFINITY), Point::splat(f64::NEG_INFINITY));
        for p in self.shapes.iter().flatten() {
            lo = lo.min(*p);
            hi = hi.max(*p);
        }
        (lo + hi) / 2.0
    }

    /// Points round every outline and along every detail, no further apart than `step`.
    fn samples(&self, step: f64) -> Vec<Point> {
        let mut out = Vec::new();
        for ring in &self.shapes {
            for i in 0..ring.len() {
                out.extend(along(ring[i], ring[(i + 1) % ring.len()], step));
            }
        }
        for d in &self.details {
            out.extend(d.points.windows(2).flat_map(|w| along(w[0], w[1], step)));
        }
        out
    }
}

/// A scroll's centre line: from the origin along +x, its curve tightening until its radius
/// is `volute`, then holding, stopping before it comes within `clear` of itself. Points,
/// headings and curvatures.
#[expect(clippy::cast_precision_loss, reason = "points along a spine")]
fn spine(length: f64, volute: f64, clear: f64, step: f64) -> (Vec<Point>, Vec<f64>, Vec<f64>) {
    let start = 0.5 / length;
    let most = 1.0 / volute;
    let rate = (most / start).max(1.01).ln() / length;
    let total = length + TAU * volute * 1.25;
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "a few hundred steps"
    )]
    let steps = (total / step).ceil() as usize;
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "a few steps"
    )]
    let skip = (clear * 1.6 / step).ceil() as usize + 1;
    let (mut points, mut headings, mut bends) = (vec![Point::ZERO], vec![0.0], vec![start]);
    let (mut p, mut h) = (Point::ZERO, 0.0);
    for i in 1..=steps {
        let s = i as f64 * step;
        let k = (start * (rate * s).exp()).min(most);
        h += k * step;
        p += Point::from_angle(h) * step;
        let crowded = points
            .iter()
            .take(points.len().saturating_sub(skip))
            .any(|q| q.distance(p) < clear);
        if crowded {
            break;
        }
        points.push(p);
        headings.push(h);
        bends.push(k);
    }
    (points, headings, bends)
}

/// A band along `points` tapering from `width` at its foot to a point, never wider than
/// its curve allows.
#[expect(clippy::cast_precision_loss, reason = "points along a spine")]
fn band(points: &[Point], headings: &[f64], bends: &[f64], width: f64) -> Vec<Point> {
    let n = points.len();
    let (mut left, mut right) = (Vec::with_capacity(n), Vec::with_capacity(n));
    for i in 0..n {
        let u = i as f64 / (n - 1).max(1) as f64;
        let half = (width / 2.0 * (1.0 - u).powf(0.8))
            .min(0.6 / bends[i].max(1e-9))
            .max(width * 0.03);
        let normal = Point::from_angle(headings[i]).perp() * half;
        left.push(points[i] + normal);
        right.push(points[i] - normal);
    }
    right.reverse();
    left.extend(right);
    left
}

/// A whole scroll `length` long in its own frame; `depth` 0 for a main scroll, which also
/// gets a smaller scroll off it and a leaf or flower at its foot.
#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_precision_loss,
    reason = "indices along the spine"
)]
fn scroll(g: &mut Grower, length: f64, look: Look, depth: u32) -> Motif {
    let gap = g.gap();
    let baroque = look == Look::Baroque;
    let width = (length * if baroque { 0.09 } else { 0.05 }).max(gap * 0.8);
    let volute = (length * 0.09).max(gap * 1.5 + width);
    let (points, headings, bends) = spine(length, volute, gap + width, g.step);
    let count = points.len();
    let at = |u: f64| ((u * (count - 1) as f64).round() as usize).min(count - 1);
    let mut motif = Motif::default();
    motif.shapes.push(band(&points, &headings, &bends, width));
    // the volute ends in a round eye (a seed, on western leaf)
    let end = count - 1;
    let eye = (width * 0.6).max(gap * 0.5);
    motif.shapes.push(circle(
        points[end] - Point::from_angle(headings[end]) * eye * 0.6,
        eye,
    ));
    // leaves round the outside of the curve (the right, as it curls left), getting smaller
    // towards the volute; inside it, a few smaller ones
    let leaves: &[(f64, f64, f64)] = if baroque {
        &[
            (0.06, 0.36, -1.0),
            (0.22, 0.3, -1.0),
            (0.4, 0.24, -1.0),
            (0.56, 0.18, -1.0),
        ]
    } else {
        &[
            (0.08, 0.3, -1.0),
            (0.24, 0.26, -1.0),
            (0.4, 0.24, -1.0),
            (0.56, 0.18, -1.0),
        ]
    };
    let skip_for_scroll = depth == 0 && length * 0.4 >= gap * 12.0;
    for &(u, size, side) in leaves {
        if skip_for_scroll && (u - 0.4).abs() < 1e-9 {
            continue;
        }
        let i = at(u);
        let leaf_length = length * size * g.range(0.9, 1.1);
        let (base, heading) = (points[i], headings[i]);
        if baroque {
            // lying along the band, curling round with it
            let heading = heading + side * g.range(0.2, 0.35);
            let bend = bends[i] * leaf_length * 1.1 + g.range(0.25, 0.5);
            let lobes = (leaf_length / (gap * 3.5)).floor().clamp(2.0, 5.0);
            let wide = (leaf_length * 0.5).max(gap * 0.8);
            motif.shapes.push(acanthus(
                base,
                heading,
                leaf_length,
                wide,
                bend,
                side,
                lobes,
            ));
        } else {
            let heading = heading + side * g.range(0.6, 0.9);
            let bend = side * g.range(0.3, 0.6);
            let wide = (leaf_length * 0.42).max(gap * 0.8);
            motif
                .shapes
                .push(wavy_leaf(base, heading, leaf_length, wide, bend));
            if wide >= gap * 2.6 {
                let vein = centre_line(base, heading, leaf_length, bend, 0.18, 0.72);
                motif.details.push(Stroke::open(vein));
            }
        }
    }
    if depth > 0 {
        return motif;
    }
    // a smaller scroll off the outside, curling the other way
    if skip_for_scroll {
        let i = at(0.4);
        let small = scroll(g, length * 0.4, look, depth + 1);
        motif.add(small.placed(points[i], headings[i] - g.range(0.6, 0.9), true));
    }
    // at the foot, a leaf turning back (acanthus) or a flower (western)
    if baroque {
        let leaf_length = length * 0.3;
        let wide = leaf_length * 0.5;
        let lobes = (leaf_length / (gap * 3.5)).floor().clamp(2.0, 5.0);
        motif.shapes.push(acanthus(
            Point::ZERO,
            PI - g.range(0.3, 0.5),
            leaf_length,
            wide,
            -g.range(0.5, 0.9),
            1.0,
            lobes,
        ));
    } else {
        let radius = (length * 0.11).max(gap * 3.4);
        let centre = Point::new(-radius * 0.7, 0.0);
        motif.add(flower(centre, radius, gap, g.range(0.0, TAU)));
    }
    motif
}

/// A layered flower: a scalloped rosette, a second ring inside when there is room, and a
/// round heart.
fn flower(centre: Point, radius: f64, gap: f64, turn: f64) -> Motif {
    let petals = 5.0;
    let mut m = Motif::default();
    m.shapes.push(rosette(centre, radius, petals, turn));
    let inner = radius * 0.6 - gap;
    let heart = (inner * 0.22).max(gap * 0.5);
    if inner * 0.6 - gap >= heart {
        m.details.push(Stroke::shape(rosette(
            centre,
            inner,
            petals,
            turn + PI / petals,
        )));
        m.details.push(Stroke::shape(circle(centre, heart)));
    } else {
        m.details.push(Stroke::shape(circle(centre, radius * 0.22)));
    }
    m
}

/// Whether the points can go down: inside, and the gap clear of everything laid so far
/// (but `junction`'s stroke near it).
fn fits(g: &Grower, points: &[Point], junction: Option<(u32, Point, f64)>) -> bool {
    let probe = Probe {
        stroke: u32::MAX,
        index: 0,
        skip: 0,
        junction,
    };
    points
        .iter()
        .all(|&p| g.area.inside(p) && g.space.clear(p, 0.0, probe))
}

/// Lays a placed motif down: its points in the space, its shapes and details in `made`.
fn lay(g: &mut Grower, motif: Motif, points: &[Point], made: &mut Made) {
    let stroke = g.space.stroke();
    for &p in points {
        g.space.add(p, 0.0, stroke, 0);
    }
    made.shapes
        .extend(motif.shapes.into_iter().map(Stroke::shape));
    made.details.extend(motif.details);
}

/// Composed scrolls over the grower's area, the biggest that fits first, down to about
/// `size` across; then flowers in the gaps. `axis` (the mirror line's stroke and x) keeps
/// them to its left, the first scroll's foot on it so it joins its mirror image.
pub fn ornament(g: &mut Grower, size: f64, look: Look, axis: Option<(u32, f64)>) -> Made {
    let gap = g.gap();
    let mut made = Made::default();
    let b = g.area.bounds;
    let smallest = (size * 1.5).max(gap * 10.0);
    let mut length = b.size().x.max(b.size().y) * if axis.is_some() { 0.6 } else { 0.75 };
    // the first scroll stands on the mirror line
    if let Some((line, x)) = axis {
        let mut tries = 0;
        while length >= smallest && tries < 200 {
            tries += 1;
            let motif = scroll(g, length, look, 0);
            let y = b.min.y + b.size().y * g.range(0.3, 0.7);
            let flip = g.chance(0.5);
            let angle = PI + if flip { -1.0 } else { 1.0 } * g.range(0.0, 0.5);
            let placed = motif.placed(Point::new(x, y), angle, flip);
            let points = placed.samples(g.step);
            if fits(g, &points, Some((line, Point::new(x, y), length * 0.15))) {
                lay(g, placed, &points, &mut made);
                break;
            }
            if tries % 10 == 0 {
                length *= 0.85;
            }
        }
    }
    while length >= smallest && !g.full() {
        let mut misses = 0;
        while misses < 150 && !g.full() {
            let Some(at) = g.area.random_inside(&mut g.random) else {
                misses += 1;
                continue;
            };
            if axis.is_some_and(|(_, x)| at.x >= x) {
                misses += 1;
                continue;
            }
            let motif = scroll(g, length, look, 0);
            let angle = g.range(0.0, TAU);
            let flip = g.chance(0.5);
            let placed = motif.placed(Point::ZERO, angle, flip);
            let placed = placed.placed(at - placed.centre(), 0.0, false);
            let points = placed.samples(g.step);
            if fits(g, &points, None) {
                lay(g, placed, &points, &mut made);
                misses = 0;
            } else {
                misses += 1;
            }
        }
        length *= 0.85;
    }
    // small flowers where there is room, not inside a shape already laid
    let shapes: Vec<Vec<Point>> = made.shapes.iter().map(|s| s.points.clone()).collect();
    let mut misses = 0;
    while misses < 20 && !g.full() {
        let radius = (size * g.range(0.2, 0.3)).max(gap * 3.4);
        let Some(at) = g.area.random_inside(&mut g.random) else {
            misses += 1;
            continue;
        };
        let left = axis.is_none_or(|(_, x)| at.x < x);
        let within = shapes
            .iter()
            .any(|s| cut_craft_core::geometry::inside(std::slice::from_ref(s), at));
        if !left || within || g.space.room(at, radius + gap) < radius + gap {
            misses += 1;
            continue;
        }
        let bloom = flower(at, radius, gap, g.range(0.0, TAU));
        let points = bloom.samples(g.step);
        if fits(g, &points, None) {
            lay(g, bloom, &points, &mut made);
            misses = 0;
        } else {
            misses += 1;
        }
    }
    made
}
