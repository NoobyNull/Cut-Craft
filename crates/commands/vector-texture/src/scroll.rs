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
    /// Broad tapering C and S scrolls wrapped in lobed acanthus leaves, as Dutch and French
    /// ornament carving, mirrored left to right.
    Baroque,
    /// Western (Sheridan-style) leaf: scroll bands curling into round seeds, broad wavy
    /// leaves with a vein, and layered flowers, as tooled leather.
    Western,
}

/// What the scrolls make: the shapes to merge into one another (stems and the leaves on
/// them), and the details that sit apart (flowers, veins, a floral style's lines).
#[derive(Default)]
pub struct Made {
    pub shapes: Vec<Stroke>,
    pub details: Vec<Stroke>,
    /// The composed styles' shapes as raised pieces, for the relief, lowest first.
    pub pieces: Vec<Piece>,
}

/// What a raised piece of a relief is, which says how it is rounded.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Part {
    /// A scroll's band, rolling along its length into the volute.
    Band,
    /// A leaf lying along its band, curling up towards its tip.
    Leaf,
    /// A volute's eye or a flower's heart: a round button.
    Eye,
    /// A ring of petals, each domed.
    Petals,
}

/// A raised piece of a relief: its outline, the line down its middle from its foot to its
/// tip (the centre, for a round one), what it is, and how high its top stands as a share of
/// the relief's height.
#[derive(Clone, Debug)]
pub struct Piece {
    pub part: Part,
    pub outline: Vec<Point>,
    pub spine: Vec<Point>,
    pub rise: f64,
}

impl Piece {
    pub fn new(part: Part, outline: Vec<Point>, spine: Vec<Point>, rise: f64) -> Self {
        Self {
            part,
            outline,
            spine,
            rise,
        }
    }

    #[must_use]
    pub fn map(&self, f: impl Fn(Point) -> Point) -> Self {
        Self {
            part: self.part,
            outline: self.outline.iter().map(|&p| f(p)).collect(),
            spine: self.spine.iter().map(|&p| f(p)).collect(),
            rise: self.rise,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Kind {
    /// A main stem, grown both ways from where it starts.
    Main,
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
    /// Which side of its stem it grows: 1 left, -1 right.
    side: f64,
}

/// A grown stem: its centre line, the heading and curvature at each point, and which
/// stroke each point belongs to (a main stem is two, one each way).
struct Stem {
    points: Vec<Point>,
    headings: Vec<f64>,
    bends: Vec<f64>,
    strokes: Vec<u32>,
}

/// The scrolls grown over the grower's area, `size` their scale.
pub fn scrolls(g: &mut Grower, size: f64, look: Look) -> Made {
    let gap = g.gap();
    let baroque = look == Look::Baroque;
    // the stems' half-width as the clearance sees it: baroque scrolls are broad ribbons,
    // western ones narrower bands, floral ones lines
    let reach = match look {
        Look::Floral => 0.0,
        Look::Baroque => (size * 0.11).max(gap * 0.4),
        Look::Western => (size * 0.05).max(gap * 0.4),
    };
    let mut made = Made::default();
    let mut buds = Vec::new();
    let mut misses = 0;
    while misses < 50 && !g.full() {
        if buds.is_empty() {
            let room = size * if baroque { 0.3 } else { 0.5 } + reach;
            let Some(at) = roomy(g, room) else {
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
        let mut spots = Vec::new();
        sprout(g, &bud, &stem, size, look, &mut buds, &mut spots);
        if bud.kind == Kind::Flower {
            let end = stem.points.len() - 1;
            spots.push(Spot {
                at: stem.points[end],
                heading: stem.headings[end],
                parent: Some(stem.strokes[end]),
                size,
                flower: true,
                side: 1.0,
            });
        }
        if look == Look::Floral {
            made.details.push(Stroke::open(stem.points));
        } else {
            made.shapes.push(ribbon(&stem, &bud, reach, g.step));
            // a scroll that grew into its volute ends in a round eye (a seed, on western
            // leaf)
            #[expect(clippy::cast_precision_loss, reason = "points along a stem")]
            let grown = (stem.points.len() - 1) as f64 * g.step;
            let whole = if bud.kind == Kind::Main { 1.8 } else { 0.9 };
            if bud.kind != Kind::Flower && grown >= bud.length * whole {
                made.shapes.push(eye(&stem, &bud, reach, gap));
            }
        }
        // the leaves and flowers go down before the side scrolls grow, so the scrolls
        // find their way round them: on a carving the leaves come first
        for spot in spots {
            if spot.flower {
                made.details.extend(flower(g, &spot, look));
            } else if let Some((leaf, vein)) = leaf_at(g, &spot, look, reach) {
                made.shapes.push(leaf);
                made.details.extend(vein);
            }
        }
    }
    // the gaps left between vines take a flower of their own
    if look == Look::Floral {
        let mut misses = 0;
        while misses < 20 && !g.full() {
            match roomy(g, size * 0.3) {
                Some(at) => {
                    let spot = Spot {
                        at,
                        heading: 0.0,
                        parent: None,
                        size: size * 0.8,
                        flower: true,
                        side: 1.0,
                    };
                    let flower = flower_at(g, &spot, at, look);
                    misses = if flower.is_empty() { misses + 1 } else { 0 };
                    made.details.extend(flower);
                }
                None => misses += 1,
            }
        }
    }
    made
}

/// Somewhere with `room` clear round it.
fn roomy(g: &mut Grower, room: f64) -> Option<Point> {
    (0..30)
        .filter_map(|_| g.area.random_inside(&mut g.random))
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
    // floral and western vines wave; baroque scrolls (the broad ones) sweep
    let waves = bud.kind == Kind::Main && reach < size * 0.08;
    let bend = Bend::new(g, bud, size, reach, waves);
    let walk = Walk {
        start: bud.at,
        heading,
        most: bud.length + TAU / bend.most,
        reach,
        junction: parent.map(|p| (p, bud.at)),
    };
    let shortest = match bud.kind {
        Kind::Main => size * 0.6,
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
                let chance = match look {
                    Look::Floral | Look::Western => 0.35,
                    Look::Baroque => 0.2,
                };
                let flower = bud.depth >= 1 && g.chance(chance);
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
            let turn = match look {
                // acanthus leaves lie along the scroll, folding over it
                Look::Baroque => g.range(0.2, 0.45),
                Look::Floral | Look::Western => g.range(0.5, 0.9),
            };
            spots.push(Spot {
                at: stem.points[j],
                heading: stem.headings[j] - outward * turn,
                parent: Some(stem.strokes[j]),
                size: size * scale,
                flower: false,
                side: -outward,
            });
            if look != Look::Floral && g.chance(0.7) {
                // and on the outside of the curve too
                let j = index(s + size * scale * 0.35);
                let turn = if look == Look::Baroque {
                    g.range(0.2, 0.45)
                } else {
                    g.range(0.4, 0.8)
                };
                spots.push(Spot {
                    at: stem.points[j],
                    heading: stem.headings[j] + outward * turn,
                    parent: Some(stem.strokes[j]),
                    size: size * scale,
                    flower: false,
                    side: outward,
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

/// The round eye at a broad scroll's volute.
fn eye(stem: &Stem, bud: &Bud, reach: f64, gap: f64) -> Stroke {
    let scale = 0.7f64.powi(i32::try_from(bud.depth).unwrap_or(4));
    let r = (reach * scale * 0.9).max(gap * 0.5);
    let end = stem.points[stem.points.len() - 1];
    // drawn back from the tip by its radius, so it sits in the curl
    let back = end - Point::from_angle(stem.headings[stem.points.len() - 1]) * r;
    Stroke::shape(
        (0..32)
            .map(|i| back + Point::from_angle(TAU * f64::from(i) / 32.0) * r)
            .collect(),
    )
}

/// A leaf at `spot`, shrinking to fit: the outline, and on western leaf its vein.
fn leaf_at(
    g: &mut Grower,
    spot: &Spot,
    look: Look,
    reach: f64,
) -> Option<(Stroke, Option<Stroke>)> {
    let gap = g.gap();
    let size = spot.size;
    let (length, width, bend) = match look {
        Look::Floral => (size * g.range(0.3, 0.42), 0.36, g.range(-0.5, 0.5)),
        // long, folding round with the scroll's curl
        Look::Baroque => (size * g.range(0.6, 0.9), 0.5, spot.side * g.range(0.5, 1.1)),
        Look::Western => (
            size * g.range(0.45, 0.6),
            0.45,
            spot.side * g.range(0.3, 0.8),
        ),
    };
    let mut length = length;
    while length >= gap * 2.5 {
        let wide = (length * width).max(gap * 0.8);
        let outline = match look {
            Look::Floral => leaf(
                spot.at,
                spot.heading,
                length,
                wide.min(length * 0.45),
                bend,
                0,
            ),
            Look::Baroque => {
                // lobes no finer than a few gaps
                let lobes = (length / (gap * 3.5)).floor().clamp(2.0, 5.0);
                acanthus(spot.at, spot.heading, length, wide, bend, spot.side, lobes)
            }
            Look::Western => wavy_leaf(spot.at, spot.heading, length, wide, bend),
        };
        // a floral leaf keeps clear of its stem but where it joins; the broad styles' leaves
        // lie along their stem and merge with it
        let near = if look == Look::Floral {
            (gap + reach + wide) * 2.0
        } else {
            length + wide + reach + gap
        };
        if g.place(&outline, spot.parent.map(|p| (p, spot.at, near))) {
            // a vein down a western leaf wide enough to keep the gap either side of it
            let vein = (look == Look::Western && wide >= gap * 2.6).then(|| {
                Stroke::open(centre_line(spot.at, spot.heading, length, bend, 0.18, 0.72))
            });
            return Some((Stroke::shape(outline), vein));
        }
        length *= 0.8;
    }
    None
}

/// Points along a leaf's centre line from `from` to `to` of its length.
#[expect(clippy::cast_precision_loss, reason = "a few dozen samples")]
pub(crate) fn centre_line(
    base: Point,
    heading: f64,
    length: f64,
    bend: f64,
    from: f64,
    to: f64,
) -> Vec<Point> {
    const N: usize = 40;
    let (mut p, mut h) = (base, heading);
    let mut out = Vec::new();
    for i in 0..=N {
        let t = i as f64 / N as f64;
        if (from..=to).contains(&t) {
            out.push(p);
        }
        h += bend / N as f64;
        p += Point::from_angle(h) * (length / N as f64);
    }
    out
}

/// A leaf outline along a bending centre line, its half-width either side given by `left`
/// and `right` of the distance along it (0 to 1).
#[expect(clippy::cast_precision_loss, reason = "a few dozen samples")]
fn shaped_leaf(
    base: Point,
    heading: f64,
    length: f64,
    bend: f64,
    left: impl Fn(f64) -> f64,
    right: impl Fn(f64) -> f64,
) -> Vec<Point> {
    const N: usize = 80;
    let (mut at, mut facing) = (base, heading);
    let (mut lefts, mut rights) = (Vec::with_capacity(N), Vec::with_capacity(N));
    for i in 0..=N {
        let t = i as f64 / N as f64;
        let normal = Point::from_angle(facing).perp();
        if i == 0 || i == N {
            lefts.push(at);
        } else {
            lefts.push(at + normal * left(t));
            rights.push(at - normal * right(t));
        }
        facing += bend / N as f64;
        at += Point::from_angle(facing) * (length / N as f64);
    }
    rights.reverse();
    lefts.extend(rights);
    lefts
}

/// An acanthus leaf lying along its stem: narrow on the stem side, and on the other a row of
/// lobes, each rounded and pointing towards the tip, as carved acanthus.
pub(crate) fn acanthus(
    base: Point,
    heading: f64,
    length: f64,
    width: f64,
    bend: f64,
    side: f64,
    lobes: f64,
) -> Vec<Point> {
    let body = |t: f64| (std::f64::consts::PI * t).sin().powf(0.7) * (1.0 - 0.35 * t);
    let scalloped = move |t: f64| {
        let f = (lobes * t).fract();
        width * body(t) * (0.55 + 0.45 * (std::f64::consts::PI * f.powf(1.3)).sin())
    };
    let plain = move |t: f64| width * 0.3 * body(t);
    if side > 0.0 {
        shaped_leaf(base, heading, length, bend, scalloped, plain)
    } else {
        shaped_leaf(base, heading, length, bend, plain, scalloped)
    }
}

/// A western leaf: broad, its edges gently waved, the tip curling.
pub(crate) fn wavy_leaf(
    base: Point,
    heading: f64,
    length: f64,
    width: f64,
    bend: f64,
) -> Vec<Point> {
    let half = move |t: f64| {
        let body = (std::f64::consts::PI * t).sin().powf(0.75) * (1.0 - 0.3 * t);
        width / 2.0 * body * (0.85 + 0.15 * (std::f64::consts::TAU * 3.0 * t).cos())
    };
    shaped_leaf(base, heading, length, bend, half, half)
}

/// A rosette at the end of a stem, the stem running into its edge.
fn flower(g: &mut Grower, spot: &Spot, look: Look) -> Vec<Stroke> {
    let centre = spot.at + Point::from_angle(spot.heading) * (spot.size * 0.12);
    flower_at(g, spot, centre, look)
}

/// Points round a scalloped rosette of `petals` petals.
pub(crate) fn rosette(centre: Point, radius: f64, petals: f64, turn: f64) -> Vec<Point> {
    (0..120)
        .map(|i| {
            let a = turn + TAU * f64::from(i) / 120.0;
            let r = radius * (0.6 + 0.4 * (a * petals / 2.0).cos().abs().powf(0.6));
            centre + Point::from_angle(a) * r
        })
        .collect()
}

pub(crate) fn circle(centre: Point, radius: f64) -> Vec<Point> {
    (0..32)
        .map(|i| centre + Point::from_angle(TAU * f64::from(i) / 32.0) * radius)
        .collect()
}

/// A scalloped flower with a round centre at `centre`, shrinking to fit; on western leaf a
/// second ring of petals inside, when there is room for it.
fn flower_at(g: &mut Grower, spot: &Spot, centre: Point, look: Look) -> Vec<Stroke> {
    let gap = g.gap();
    let petals = if g.chance(0.5) { 5.0 } else { 6.0 };
    let turn = g.range(0.0, TAU);
    let mut radius = (spot.size * g.range(0.16, 0.24)).max(gap * 3.4);
    if look == Look::Western {
        radius *= 1.3;
    }
    while radius >= gap * 3.4 {
        // the stem's end sits inside the flower
        let centre = if spot.parent.is_some() {
            spot.at + Point::from_angle(spot.heading) * radius * 0.7
        } else {
            centre
        };
        let outline = rosette(centre, radius, petals, turn);
        let near = radius + gap * 3.0;
        if g.place(&outline, spot.parent.map(|p| (p, spot.at, near))) {
            let mut out = vec![Stroke::shape(outline)];
            // an inner ring wholly inside the outer's notches, a gap clear of them
            let inner = radius * 0.6 - gap;
            let heart = (inner * 0.22).max(gap * 0.5);
            if look == Look::Western && inner * 0.6 - gap >= heart {
                out.push(Stroke::shape(rosette(
                    centre,
                    inner,
                    petals,
                    turn + PI / petals,
                )));
                out.push(Stroke::shape(circle(centre, heart)));
            } else {
                out.push(Stroke::shape(circle(centre, radius * 0.22)));
            }
            return out;
        }
        radius *= 0.8;
    }
    vec![]
}
