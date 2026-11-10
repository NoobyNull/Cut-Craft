//! Floral vines and baroque scrolls: long stems that wave through the area and curl into
//! volutes at their ends, side scrolls curling off their outer curves, and leaves (and, on
//! the vines, flowers) where there is room.

use crate::fill::{Grower, Stroke, Walk, leaf};
use cut_craft_core::geometry::Point;
use std::f64::consts::{PI, TAU};

/// Which of the two the scrolls make.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Look {
    /// Thin stems with leaves and flowers.
    Floral,
    /// Broad tapering C and S scrolls with lobed acanthus leaves, mirrored left to right.
    Baroque,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Kind {
    /// A main stem, grown both ways from where it starts.
    Main,
    /// A main stem grown one way only, from the mirror line.
    FromAxis,
    /// A side scroll curling off its parent.
    Side,
    /// A floral stem that runs gently out to a flower.
    Flower,
}

/// A stem still to grow, from a point on its parent.
#[derive(Clone, Copy)]
struct Bud {
    at: Point,
    heading: f64,
    /// Which way it curls: 1 anticlockwise, -1 clockwise.
    curl: f64,
    /// How long it may grow (each way, for a main stem).
    length: f64,
    depth: u32,
    parent: Option<u32>,
    kind: Kind,
}

/// A leaf or flower to try once the stems are grown.
struct Spot {
    at: Point,
    heading: f64,
    parent: Option<u32>,
    size: f64,
    flower: bool,
}

/// A grown stem: its centre line, the heading and curvature at each point, and which
/// stroke each point belongs to (a main stem is two, one each way).
struct Stem {
    points: Vec<Point>,
    headings: Vec<f64>,
    bends: Vec<f64>,
    strokes: Vec<u32>,
}

/// The scrolls grown over the grower's area, `size` their scale; `axis` is the mirror line
/// (its stroke and x) for baroque scrolls, grown on its left only.
pub fn scrolls(g: &mut Grower, size: f64, look: Look, axis: Option<(u32, f64)>) -> Vec<Stroke> {
    let gap = g.gap();
    let baroque = look == Look::Baroque;
    // the stems' half-width as the clearance sees it: baroque scrolls are broad ribbons
    let reach = if baroque {
        (size * 0.09).max(gap * 0.4)
    } else {
        0.0
    };
    let mut strokes = Vec::new();
    let mut spots = Vec::new();
    let mut buds = Vec::new();
    // baroque scrolls start from the mirror line, so the halves join there
    if let Some((line, x)) = axis {
        let (lo, hi) = (g.area.bounds.min.y, g.area.bounds.max.y);
        for t in [0.3, 0.7] {
            let y = lo + (hi - lo) * (t + g.range(-0.08, 0.08));
            let up = if t < 0.5 { -1.0 } else { 1.0 };
            buds.push(Bud {
                at: Point::new(x - reach * 0.5, y),
                heading: PI + up * g.range(0.3, 0.7),
                curl: up,
                length: size * g.range(2.0, 2.8),
                depth: 0,
                parent: Some(line),
                kind: Kind::FromAxis,
            });
        }
    }
    let mut misses = 0;
    while misses < 50 && !g.full() {
        if buds.is_empty() {
            let room = size * if baroque { 0.3 } else { 0.5 } + reach;
            let Some(at) = roomy(g, room, axis) else {
                misses += 1;
                continue;
            };
            let heading = g.range(0.0, TAU);
            let curl = g.sign();
            let length = size * g.range(1.5, 2.5);
            buds.push(Bud {
                at,
                heading,
                curl,
                length,
                depth: 0,
                parent: None,
                kind: Kind::Main,
            });
        }
        let bud = buds.remove(0);
        let Some(stem) = grow(g, &bud, size, reach) else {
            if bud.parent.is_none() {
                misses += 1;
            }
            continue;
        };
        misses = 0;
        sprout(g, &bud, &stem, size, look, &mut buds, &mut spots);
        if bud.kind == Kind::Flower {
            let end = stem.points.len() - 1;
            spots.push(Spot {
                at: stem.points[end],
                heading: stem.headings[end],
                parent: Some(stem.strokes[end]),
                size,
                flower: true,
            });
        }
        strokes.push(if baroque {
            ribbon(&stem, &bud, reach, g.step)
        } else {
            Stroke::open(stem.points)
        });
    }
    for spot in spots {
        if spot.flower {
            strokes.extend(flower(g, &spot));
        } else {
            strokes.extend(leaf_at(g, &spot, look, reach));
        }
    }
    // the gaps left between vines take a flower of their own
    if look == Look::Floral {
        let mut misses = 0;
        while misses < 20 && !g.full() {
            match roomy(g, size * 0.3, None) {
                Some(at) => {
                    let spot = Spot {
                        at,
                        heading: 0.0,
                        parent: None,
                        size: size * 0.8,
                        flower: true,
                    };
                    let made = flower_at(g, &spot, at);
                    misses = if made.is_empty() { misses + 1 } else { 0 };
                    strokes.extend(made);
                }
                None => misses += 1,
            }
        }
    }
    strokes
}

/// Somewhere with `room` clear round it (left of the mirror line, if there is one).
fn roomy(g: &mut Grower, room: f64, axis: Option<(u32, f64)>) -> Option<Point> {
    (0..30)
        .filter_map(|_| g.area.random_inside(&mut g.random))
        .filter(|p| axis.is_none_or(|(_, x)| p.x < x))
        .find(|&p| g.space.room(p, room) >= room)
}

/// How a stem bends along it: a main floral stem waves, then every stem tightens into a
/// volute towards its end; a flower's stem barely bends.
struct Bend {
    curl: f64,
    k0: f64,
    most: f64,
    tighten: f64,
    /// Where the volute starts; before it a floral main stem waves.
    volute: f64,
    wave: Option<(f64, f64, f64)>,
}

impl Bend {
    fn new(g: &mut Grower, bud: &Bud, size: f64, reach: f64, waves: bool) -> Self {
        let gap = g.gap();
        let scale = 0.65f64.powi(i32::try_from(bud.depth).unwrap_or(4));
        // the volute's tightest radius: carvable, and in proportion
        let tightest = (size * scale / 9.0).max(gap * 1.3 + reach * 2.0);
        let most = 1.0 / tightest;
        let k0 = g.range(0.5, 1.0) / (size * scale * 1.2);
        let volute = if waves { bud.length * 0.55 } else { 0.0 };
        let tighten = ((bud.length - volute) * 0.8) / (most / k0).max(1.01).ln();
        let wave = waves.then(|| {
            let wavelength = size * g.range(1.6, 2.4);
            let amount = g.range(0.6, 1.0) / size;
            (wavelength, amount, g.range(0.0, TAU))
        });
        Self {
            curl: bud.curl,
            k0,
            most,
            tighten,
            volute,
            wave,
        }
    }

    fn at(&self, s: f64, kind: Kind) -> f64 {
        if kind == Kind::Flower {
            return self.curl * self.k0 * 0.5;
        }
        if let Some((wavelength, amount, phase)) = self.wave
            && s < self.volute
        {
            return amount * (TAU * s / wavelength + phase).sin();
        }
        self.curl * (self.k0 * ((s - self.volute) / self.tighten).exp()).min(self.most)
    }
}

/// One way grown from `bud`: the stroke, points, headings and bends.
type Half = (u32, Vec<Point>, Vec<f64>, Vec<f64>);

fn half(
    g: &mut Grower,
    bud: &Bud,
    heading: f64,
    parent: Option<u32>,
    size: f64,
    reach: f64,
) -> Option<Half> {
    let gap = g.gap();
    let waves = bud.kind == Kind::Main && reach == 0.0;
    let bend = Bend::new(g, bud, size, reach, waves);
    let walk = Walk {
        start: bud.at,
        heading,
        most: bud.length + TAU / bend.most,
        reach,
        junction: parent.map(|p| (p, bud.at)),
    };
    let shortest = match bud.kind {
        Kind::Main | Kind::FromAxis => size * 0.6,
        Kind::Flower => size * 0.4,
        Kind::Side => (size * 0.25).max(gap * 3.0),
    };
    let mut bends = vec![0.0];
    let (stroke, points, headings) = g.walk(&walk, shortest, |g, _, s| {
        let k = bend.at(s, bud.kind);
        bends.push(k);
        Some(k * g.step)
    })?;
    bends.truncate(points.len());
    Some((stroke, points, headings, bends))
}

/// Grows `bud`: a main stem both ways from its start, anything else one way.
fn grow(g: &mut Grower, bud: &Bud, size: f64, reach: f64) -> Option<Stem> {
    let (one, points, headings, bends) = half(g, bud, bud.heading, bud.parent, size, reach)?;
    let mut stem = Stem {
        strokes: vec![one; points.len()],
        points,
        headings,
        bends,
    };
    if bud.kind != Kind::Main {
        return Some(stem);
    }
    // the other way: the same curl makes a C, the other an S
    let other = Bud {
        curl: if g.chance(0.5) { bud.curl } else { -bud.curl },
        ..*bud
    };
    if let Some((two, p, h, b)) = half(g, &other, bud.heading + PI, Some(one), size, reach) {
        let n = p.len();
        let mut points: Vec<Point> = p.into_iter().skip(1).rev().collect();
        let mut headings: Vec<f64> = h.into_iter().skip(1).rev().map(|h| h + PI).collect();
        let mut bends: Vec<f64> = b.into_iter().skip(1).rev().map(|k| -k).collect();
        let mut strokes = vec![two; n - 1];
        points.append(&mut stem.points);
        headings.append(&mut stem.headings);
        bends.append(&mut stem.bends);
        strokes.append(&mut stem.strokes);
        stem = Stem {
            points,
            headings,
            bends,
            strokes,
        };
    }
    Some(stem)
}

/// Queues side scrolls off the outer side of `stem`'s curves, and leaves and flowers along it.
#[expect(clippy::cast_precision_loss, reason = "points along a stem")]
fn sprout(
    g: &mut Grower,
    bud: &Bud,
    stem: &Stem,
    size: f64,
    look: Look,
    buds: &mut Vec<Bud>,
    spots: &mut Vec<Spot>,
) {
    if bud.kind == Kind::Flower {
        return;
    }
    let scale = 0.65f64.powi(i32::try_from(bud.depth).unwrap_or(4));
    let count = stem.points.len();
    let length = (count - 1) as f64 * g.step;
    let pace = g.step;
    let index = |s: f64| {
        #[expect(
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            reason = "an index along the stem"
        )]
        let i = (s / pace).round().max(0.0) as usize;
        i.min(count - 1)
    };
    let tight = 1.0 / (size * scale * 0.35);
    let mut s = size * scale * g.range(0.3, 0.5);
    let mut alternate = 1.0;
    while s < length - size * scale * 0.3 {
        let i = index(s);
        let (at, heading, bend) = (stem.points[i], stem.headings[i], stem.bends[i]);
        // not in a volute
        if bend.abs() < tight {
            // off the outside of the curve
            let outward = if bend.abs() > 0.15 / size {
                -bend.signum()
            } else {
                alternate = -alternate;
                alternate
            };
            if bud.depth < 2 && g.chance(0.8) {
                let flower = look == Look::Floral && bud.depth >= 1 && g.chance(0.35);
                let length = size * scale * g.range(0.8, 1.4);
                buds.push(Bud {
                    at,
                    heading: heading + outward * g.range(0.5, 0.8),
                    // rolling up away from the stem, as a scroll does
                    curl: outward,
                    length,
                    depth: bud.depth + 1,
                    parent: Some(stem.strokes[i]),
                    kind: if flower { Kind::Flower } else { Kind::Side },
                });
            }
            // a leaf just past the branch, on the other side
            let j = index(s + size * scale * 0.15);
            spots.push(Spot {
                at: stem.points[j],
                heading: stem.headings[j] - outward * g.range(0.5, 0.9),
                parent: Some(stem.strokes[j]),
                size: size * scale,
                flower: false,
            });
            if look == Look::Baroque && g.chance(0.6) {
                // acanthus leaves sprout from the outside too
                let j = index(s + size * scale * 0.3);
                spots.push(Spot {
                    at: stem.points[j],
                    heading: stem.headings[j] + outward * g.range(0.4, 0.8),
                    parent: Some(stem.strokes[j]),
                    size: size * scale,
                    flower: false,
                });
            }
        }
        s += size * scale * g.range(0.5, 0.8);
    }
}

/// A broad scroll's outline: the stem's centre line widened to `reach` either side, widest
/// where it starts (in the middle, for a main stem) and tapering to a point at the volutes,
/// never wider than the curve's radius allows.
#[expect(clippy::cast_precision_loss, reason = "points along a stem")]
fn ribbon(stem: &Stem, bud: &Bud, reach: f64, spacing: f64) -> Stroke {
    let n = stem.points.len();
    let scale = 0.7f64.powi(i32::try_from(bud.depth).unwrap_or(4));
    let mut left = Vec::with_capacity(n);
    let mut right = Vec::with_capacity(n);
    for i in 0..n {
        let u = i as f64 / (n - 1).max(1) as f64;
        let shape = match bud.kind {
            Kind::Main => (PI * u).sin().powf(0.6),
            _ => (1.0 - u).powf(0.7),
        };
        let turning = if i > 0 && i + 1 < n {
            ((stem.headings[i + 1] - stem.headings[i - 1]) / (2.0 * spacing)).abs()
        } else {
            0.0
        };
        let k = stem.bends[i].abs().max(turning);
        let mut half = reach * scale * shape;
        if k > 1e-9 {
            half = half.min(0.6 / k);
        }
        let normal = Point::from_angle(stem.headings[i]).perp() * half.max(reach * 0.05);
        left.push(stem.points[i] + normal);
        right.push(stem.points[i] - normal);
    }
    right.reverse();
    left.extend(right);
    Stroke::shape(left)
}

fn leaf_at(g: &mut Grower, spot: &Spot, look: Look, reach: f64) -> Option<Stroke> {
    let gap = g.gap();
    let size = spot.size;
    let (length, width, bend, lobes) = match look {
        Look::Floral => (size * g.range(0.3, 0.42), 0.36, g.range(-0.5, 0.5), 0),
        Look::Baroque => (
            size * g.range(0.45, 0.65),
            0.4,
            g.sign() * g.range(0.6, 1.2),
            3,
        ),
    };
    let width = (length * width).max(gap * 0.8);
    // a leaf shrinks to fit, down to what can be carved
    let mut length = length;
    while length >= gap * 2.5 {
        let outline = leaf(
            spot.at,
            spot.heading,
            length,
            width.min(length * 0.45),
            bend,
            lobes,
        );
        let near = (gap + reach + width) * 2.0;
        if g.place(&outline, spot.parent.map(|p| (p, spot.at, near))) {
            return Some(Stroke::shape(outline));
        }
        length *= 0.75;
    }
    None
}

/// A rosette at the end of a stem, the stem running into its edge.
fn flower(g: &mut Grower, spot: &Spot) -> Vec<Stroke> {
    let centre = spot.at + Point::from_angle(spot.heading) * (spot.size * 0.12);
    flower_at(g, spot, centre)
}

/// A scalloped flower with a round centre at `centre`, shrinking to fit.
fn flower_at(g: &mut Grower, spot: &Spot, centre: Point) -> Vec<Stroke> {
    let gap = g.gap();
    let petals = if g.chance(0.5) { 5.0 } else { 6.0 };
    let turn = g.range(0.0, TAU);
    let mut radius = (spot.size * g.range(0.16, 0.24)).max(gap * 3.4);
    while radius >= gap * 3.4 {
        // the stem's end sits inside the flower
        let centre = if spot.parent.is_some() {
            spot.at + Point::from_angle(spot.heading) * radius * 0.7
        } else {
            centre
        };
        let outline: Vec<Point> = (0..120)
            .map(|i| {
                let a = turn + TAU * f64::from(i) / 120.0;
                let r = radius * (0.6 + 0.4 * (a * petals / 2.0).cos().abs().powf(0.6));
                centre + Point::from_angle(a) * r
            })
            .collect();
        let near = radius + gap * 3.0;
        if g.place(&outline, spot.parent.map(|p| (p, spot.at, near))) {
            let heart = radius * 0.22;
            let middle: Vec<Point> = (0..32)
                .map(|i| centre + Point::from_angle(TAU * f64::from(i) / 32.0) * heart)
                .collect();
            return vec![Stroke::shape(outline), Stroke::shape(middle)];
        }
        radius *= 0.8;
    }
    vec![]
}
