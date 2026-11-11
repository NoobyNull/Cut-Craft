//! The pieces an embellishment is built from: curls that roll up as log spirals, bands along
//! them, sunbursts, rosettes, petals and beads. Every piece is a closed outline (with a
//! height for the carving) or a carved line, in millimetres.

use crate::rng::Rng;
use cut_craft_core::geometry::Point;
use std::f64::consts::{PI, TAU};

/// How a piece rises from its outline to its height.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Profile {
    /// Rounded over its whole width, highest along its middle.
    Pillow,
    /// Rounded over this far from its edges, flat on top beyond.
    Round(f64),
    /// A dome, as a bead.
    Ball,
}

/// One piece: a raised (or, `sink`, sunk) shape, or a V-carved line.
#[derive(Clone, Debug)]
pub enum Item {
    Shape {
        /// Its outline; more than one ring are taken even-odd (a band with a hole).
        rings: Vec<Vec<Point>>,
        /// Its height as a share of the carving's.
        h: f64,
        profile: Profile,
        sink: bool,
    },
    Line {
        points: Vec<Point>,
        width: f64,
    },
}

impl Item {
    pub fn map(&self, f: &impl Fn(Point) -> Point) -> Self {
        match self {
            Self::Shape {
                rings,
                h,
                profile,
                sink,
            } => Self::Shape {
                rings: rings
                    .iter()
                    .map(|r| r.iter().map(|&p| f(p)).collect())
                    .collect(),
                h: *h,
                profile: *profile,
                sink: *sink,
            },
            Self::Line { points, width } => Self::Line {
                points: points.iter().map(|&p| f(p)).collect(),
                width: *width,
            },
        }
    }

    /// Every point of it.
    pub fn points(&self) -> Box<dyn Iterator<Item = Point> + '_> {
        match self {
            Self::Shape { rings, .. } => Box::new(rings.iter().flatten().copied()),
            Self::Line { points, .. } => Box::new(points.iter().copied()),
        }
    }

    /// The area of its outline (its first ring); a line has none.
    pub fn area(&self) -> f64 {
        match self {
            Self::Shape { rings, .. } => rings.first().map_or(0.0, |r| ring_area(r)),
            Self::Line { .. } => 0.0,
        }
    }
}

/// A raised shape of height `h` with a pillow profile.
pub fn sh(ring: Vec<Point>, h: f64) -> Item {
    Item::Shape {
        rings: vec![ring],
        h,
        profile: Profile::Pillow,
        sink: false,
    }
}

pub fn ring_area(p: &[Point]) -> f64 {
    let n = p.len();
    let mut s = 0.0;
    for i in 0..n {
        let (a, b) = (p[(i + n - 1) % n], p[i]);
        s += (a.x + b.x) * (a.y - b.y);
    }
    s.abs() / 2.0
}

pub fn items_area(items: &[Item]) -> f64 {
    items.iter().map(Item::area).sum()
}

pub fn clamp(x: f64, a: f64, b: f64) -> f64 {
    x.max(a).min(b)
}

pub fn smoothstep(e0: f64, e1: f64, x: f64) -> f64 {
    let t = clamp((x - e0) / (e1 - e0), 0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

pub fn pt(x: f64, y: f64) -> Point {
    Point::new(x, y)
}

/// The unit vector at `a` radians.
pub fn dir(a: f64) -> Point {
    pt(a.cos(), a.sin())
}

/// The left normal of heading `h`.
pub fn normal(h: f64) -> Point {
    pt(-h.sin(), h.cos())
}

/// A centre line with its heading at each point.
#[derive(Clone, Debug, Default)]
pub struct Track {
    pub p: Vec<Point>,
    pub h: Vec<f64>,
}

/// A curl: its centre line, its width and how far it has turned at each point, and the
/// radius it ends at.
#[derive(Clone, Debug)]
pub struct Curl {
    pub track: Track,
    pub w: Vec<f64>,
    pub turned: Vec<f64>,
    pub rho_end: f64,
}

/// Walks `length` from `at` heading `h`, turning by `k(t, s)` per millimetre.
pub fn trace(at: Point, mut h: f64, length: f64, k: impl Fn(f64, f64) -> f64, ds: f64) -> Track {
    let ds = if ds > 0.0 {
        ds
    } else {
        (length / 300.0).max(0.15)
    };
    let (mut p, mut s) = (at, 0.0);
    let mut out = Track {
        p: vec![p],
        h: vec![h],
    };
    while s < length - 1e-9 {
        let st = ds.min(length - s);
        let kk = k(s / length, s);
        h += kk * st / 2.0;
        p += dir(h) * st;
        h += kk * st / 2.0;
        s += st;
        out.p.push(p);
        out.h.push(h);
    }
    out
}

/// How a curl is drawn.
#[derive(Clone, Copy, Debug)]
pub struct CurlSpec {
    /// The radius it starts rolling up at.
    pub rho0: f64,
    /// How fast the radius shrinks per millimetre along it.
    pub b: f64,
    /// A lead-in before it starts to roll, curving from `lead_k` up to the curl.
    pub lead: f64,
    pub lead_k: f64,
    pub wmax: f64,
    pub max_turn: f64,
    pub g: f64,
    /// How far it takes to grow from a point to its full width.
    pub taper: f64,
    pub ds: f64,
}

/// A log-spiral curl from (0, 0) heading along +x, rolling to the left. Its width keeps the
/// gap `g` between its turns, so the turns never touch; `None` when it is too small to.
pub fn curl_path(o: &CurlSpec) -> Option<Curl> {
    const F: f64 = 0.58;
    let sp = 1.0 - (-TAU * o.b).exp();
    let rho_min = o.g / ((1.0 - F) * sp);
    if o.rho0 <= rho_min * 1.05 {
        return None;
    }
    let ds = if o.ds > 0.0 {
        o.ds
    } else {
        (o.rho0 / 120.0).max(0.15)
    };
    let width = |s: f64, rho: f64| {
        let w = o.wmax.min(F * sp * rho);
        if o.taper > 0.0 {
            w * smoothstep(0.0, o.taper, s)
        } else {
            w
        }
    };
    let (mut p, mut h, mut s, mut turned, mut rho) = (Point::ZERO, 0.0, 0.0, 0.0, o.rho0);
    let mut c = Curl {
        track: Track {
            p: vec![p],
            h: vec![0.0],
        },
        w: vec![width(0.0, o.rho0)],
        turned: vec![0.0],
        rho_end: o.rho0,
    };
    for _ in 0..100_000 {
        let k;
        if s < o.lead {
            let u = smoothstep(0.0, 1.0, s / o.lead);
            k = o.lead_k + (1.0 / o.rho0 - o.lead_k) * u;
            rho = o.rho0;
        } else {
            let r = o.rho0 - o.b * (s - o.lead);
            if r < rho_min || turned > o.max_turn {
                break;
            }
            rho = r;
            k = 1.0 / rho;
            turned += k * ds;
        }
        h += k * ds / 2.0;
        p += dir(h) * ds;
        h += k * ds / 2.0;
        s += ds;
        c.track.p.push(p);
        c.track.h.push(h);
        c.w.push(width(s, rho));
        c.turned.push(turned);
    }
    c.rho_end = rho;
    (c.track.p.len() >= 6).then_some(c)
}

/// The outline between offsets `a` and `b` (to the left) of a centre line, with round caps.
pub fn ribbon(t: &Track, a: &[f64], b: &[f64], cap_start: bool, cap_end: bool) -> Vec<Point> {
    let n = t.p.len();
    let mut poly: Vec<Point> = (0..n).map(|i| t.p[i] + normal(t.h[i]) * b[i]).collect();
    let cap = |poly: &mut Vec<Point>, i: usize, from: f64| {
        let (m, r) = (f64::midpoint(a[i], b[i]), (b[i] - a[i]) / 2.0);
        if r < 1e-6 {
            return;
        }
        let c = t.p[i] + normal(t.h[i]) * m;
        for k in 1..14 {
            poly.push(c + dir(from - PI * f64::from(k) / 14.0) * r);
        }
    };
    if cap_end {
        cap(&mut poly, n - 1, t.h[n - 1] + PI / 2.0);
    }
    for i in (0..n).rev() {
        poly.push(t.p[i] + normal(t.h[i]) * a[i]);
    }
    if cap_start {
        cap(&mut poly, 0, t.h[0] - PI / 2.0);
    }
    poly
}

/// A band of widths `w` centred on a line.
pub fn band(t: &Track, w: &[f64], cap_start: bool, cap_end: bool) -> Vec<Point> {
    let a: Vec<f64> = w.iter().map(|w| -w / 2.0).collect();
    let b: Vec<f64> = w.iter().map(|w| w / 2.0).collect();
    ribbon(t, &a, &b, cap_start, cap_end)
}

pub fn circ(c: Point, r: f64) -> Vec<Point> {
    let seg = (r * 6.0).round().clamp(10.0, 48.0) as usize;
    circ_n(c, r, seg)
}

pub fn circ_n(c: Point, r: f64, seg: usize) -> Vec<Point> {
    (0..seg)
        .map(|i| c + dir(TAU * i as f64 / seg as f64) * r)
        .collect()
}

fn arc_pts(c: Point, r: f64, a0: f64, a1: f64, n: usize) -> Vec<Point> {
    (0..=n)
        .map(|i| c + dir(a0 + (a1 - a0) * i as f64 / n as f64) * r)
        .collect()
}

/// A ring between radii `ri` and `ro` from `a0` to `a1`; a wedge when `ri` is nothing.
pub fn half_ring(c: Point, ri: f64, ro: f64, a0: f64, a1: f64) -> Vec<Point> {
    let n = ((a1 - a0) * ro * 1.5).round().max(12.0) as usize;
    let mut o = arc_pts(c, ro, a0, a1, n);
    if ri <= 0.01 {
        o.push(c);
        return o;
    }
    o.extend(arc_pts(c, ri, a1, a0, n));
    o
}

/// A petal along the ray at `ang` from `r1` to `r2`, `hw(t, r)` wide either side.
pub fn radial(c: Point, ang: f64, r1: f64, r2: f64, hw: impl Fn(f64, f64) -> f64) -> Vec<Point> {
    const STEPS: usize = 20;
    let (d, n) = (dir(ang), normal(ang));
    let (mut l, mut rr) = (vec![], vec![]);
    for i in 0..=STEPS {
        let t = i as f64 / STEPS as f64;
        let r = r1 + (r2 - r1) * t;
        let w = hw(t, r).max(0.0);
        let p = c + d * r;
        l.push(p + n * w);
        rr.push(p - n * w);
    }
    rr.reverse();
    l.extend(rr);
    l
}

/// A leaf round `c` from `r1` to `r2`, its middle at angle `a0 + bend(t)` and
/// `half(t, r)` radians wide either side.
pub fn polar(
    c: Point,
    a0: f64,
    r1: f64,
    r2: f64,
    bend: impl Fn(f64) -> f64,
    half: impl Fn(f64, f64) -> f64,
) -> Vec<Point> {
    const STEPS: usize = 24;
    let (mut l, mut rr) = (vec![], vec![]);
    for i in 0..=STEPS {
        let t = i as f64 / STEPS as f64;
        let r = r1 + (r2 - r1) * t;
        let (ac, ha) = (a0 + bend(t), half(t, r).max(0.0));
        l.push(c + dir(ac + ha) * r);
        rr.push(c + dir(ac - ha) * r);
    }
    rr.reverse();
    l.extend(rr);
    l
}

/// Corner cutting round a closed ring.
pub fn chaikin(p: &[Point], times: usize) -> Vec<Point> {
    let mut p = p.to_vec();
    for _ in 0..times {
        let n = p.len();
        p = (0..n)
            .flat_map(|i| {
                let (a, b) = (p[i], p[(i + 1) % n]);
                [a * 0.75 + b * 0.25, a * 0.25 + b * 0.75]
            })
            .collect();
    }
    p
}

/// A sunburst at `c` of radius `r0` between angles `a0` and `a1`: a hub and rays.
pub fn sunburst(
    items: &mut Vec<Item>,
    c: Point,
    r0_out: f64,
    g: f64,
    s: f64,
    a0: f64,
    a1: f64,
    h: f64,
) {
    let r0 = (g * 0.7).max(r0_out * 0.3);
    if a1 - a0 >= TAU - 1e-6 {
        items.push(sh(circ(c, r0), h + 0.15));
    } else {
        items.push(sh(half_ring(c, 0.0, r0, a0, a1), h + 0.15));
    }
    let r1 = r0 + g;
    if r0_out - r1 < 2.0 * g {
        return;
    }
    let span = a1 - a0;
    let n = clamp(
        (span * f64::midpoint(r1, r0_out) / (1.8 * g + s * 0.035)).round(),
        5.0,
        28.0,
    ) as usize;
    let d = span / n as f64;
    if r0_out * (d / 2.0).sin() - g / 2.0 < g * 0.35 {
        return;
    }
    for k in 0..n {
        let a = a0 + d * (k as f64 + 0.5);
        items.push(sh(
            radial(c, a, r1, r0_out, |t, r| {
                (r * (d / 2.0).sin() - g / 2.0) * (1.0 - t.powf(2.5))
            }),
            h,
        ));
    }
}

/// The eye a curl rolls round: a sunburst when big (and `big`), else a bead.
pub fn eye_at(items: &mut Vec<Item>, cp: &Curl, g: f64, s: f64, big: bool) {
    let n = cp.track.p.len();
    let (pe, he, rho) = (cp.track.p[n - 1], cp.track.h[n - 1], cp.rho_end);
    let c = pe + normal(he) * rho;
    let er = (rho - cp.w[n - 1] / 2.0 - g) * 0.88;
    if big && er > 4.2 * g {
        sunburst(items, c, er, g, s, 0.0, TAU, 0.8);
    } else if er > g * 0.5 {
        items.push(sh(circ(c, er), 0.9));
    }
}

/// What the motifs need to know.
pub struct Ctx {
    pub g: f64,
    /// 0 to 1: how much fine detail goes in.
    pub detail: f64,
    pub rng: Rng,
}

#[expect(clippy::unnecessary_wraps, reason = "a maker: others can fail")]
pub fn dot(s: f64, c: &mut Ctx) -> Option<Vec<Item>> {
    Some(vec![sh(circ(Point::ZERO, (c.g * 0.45).max(s * 0.3)), 0.75)])
}

#[expect(clippy::unnecessary_wraps, reason = "a maker: others can fail")]
pub fn dot_trio(s: f64, c: &mut Ctx) -> Option<Vec<Item>> {
    let g = c.g;
    let r = (g * 0.5).max(s * 0.17);
    let mut items = vec![sh(circ(Point::ZERO, r), 0.8)];
    let (mut x, mut a) = (r, 0.0);
    for f in [0.72, 0.5] {
        let rr = (g * 0.45).max(r * f);
        x += g + rr;
        a += 0.5;
        items.push(sh(circ(dir(a * 0.5) * x, rr), 0.75));
        x += rr;
    }
    Some(items)
}

#[expect(clippy::unnecessary_wraps, reason = "a maker: others can fail")]
pub fn fan_burst(s: f64, c: &mut Ctx) -> Option<Vec<Item>> {
    let g = c.g;
    let mut items = vec![];
    let r0 = s * 0.36;
    sunburst(&mut items, Point::ZERO, r0, g, s, PI, TAU, 0.85);
    let wb = (g * 1.1).max(s * 0.05);
    let (ri, ro) = (r0 + g, r0 + g + wb);
    items.push(sh(half_ring(Point::ZERO, ri, ro, PI, TAU), 1.0));
    let rd = (g * 0.55).max(s * 0.022);
    let rr = ro + g + rd;
    let m = (PI * rr / (2.0 * rd + g * 1.2)).floor() as usize;
    for k in 0..m {
        let a = PI + PI * (k as f64 + 0.5) / m as f64;
        items.push(sh(circ(dir(a) * rr, rd), 0.65));
    }
    Some(items)
}

pub fn flower_ring(s: f64, c: &mut Ctx) -> Option<Vec<Item>> {
    let g = c.g;
    let rc = (g * 0.8).max(s * 0.1);
    let n = c.rng.int(6, 9);
    let (r1, r2, hm) = (rc + g, s * 0.44, s * 0.11);
    if r2 - r1 < 3.0 * g {
        return None;
    }
    let mut items = vec![sh(circ(Point::ZERO, rc), 1.05)];
    let a0 = c.rng.range(0.0, TAU);
    let nf = f64::from(n);
    for k in 0..n {
        let a = a0 + TAU * f64::from(k) / nf;
        items.push(sh(
            radial(Point::ZERO, a, r1, r2, |t, r| {
                (hm * (PI * t.powf(0.7)).sin().powf(0.8)).min(r * (PI / nf).sin() - g / 2.0)
            }),
            0.85,
        ));
    }
    Some(items)
}

pub fn spiral_s(s: f64, c: &mut Ctx) -> Option<Vec<Item>> {
    let g = c.g;
    let wmax = (g * 1.1).max(s * 0.075);
    let o = CurlSpec {
        rho0: s * c.rng.range(0.17, 0.22),
        b: c.rng.range(0.12, 0.17),
        lead: s * 0.12,
        lead_k: 0.0,
        wmax,
        g,
        max_turn: TAU * c.rng.range(0.9, 1.4),
        taper: 0.0,
        ds: (s / 200.0).max(0.2),
    };
    let o2 = CurlSpec {
        rho0: o.rho0 * c.rng.range(0.75, 1.0),
        max_turn: TAU * c.rng.range(0.8, 1.3),
        ..o
    };
    let a = curl_path(&o)?;
    let bb = curl_path(&o2)?;
    let mut t = Track::default();
    let mut w = vec![];
    for i in (0..a.track.p.len()).rev() {
        t.p.push(a.track.p[i]);
        t.h.push(a.track.h[i] + PI);
        w.push(a.w[i]);
    }
    for i in 1..bb.track.p.len() {
        t.p.push(-bb.track.p[i]);
        t.h.push(bb.track.h[i] + PI);
        w.push(bb.w[i]);
    }
    let mut items = vec![sh(band(&t, &w, true, true), 0.95)];
    eye_at(&mut items, &a, g, s, false);
    let mut e2 = vec![];
    eye_at(&mut e2, &bb, g, s, false);
    items.extend(e2.iter().map(|it| it.map(&|q| -q)));
    Some(items)
}

pub fn rosette(s: f64, c: &mut Ctx) -> Option<Vec<Item>> {
    let g = c.g;
    let rc = (g * 0.8).max(s * 0.09);
    let n = c.rng.int(6, 8);
    let (r1, r2) = (rc + g, s * 0.25);
    let n2 = n * 2;
    let (r3, r4) = (r2 + g, s * 0.45);
    if r2 - r1 < 2.0 * g {
        return None;
    }
    let mut items = vec![sh(circ(Point::ZERO, rc), 1.1)];
    let a0 = c.rng.range(0.0, TAU);
    let (nf, n2f) = (f64::from(n), f64::from(n2));
    for k in 0..n {
        items.push(sh(
            radial(Point::ZERO, a0 + TAU * f64::from(k) / nf, r1, r2, |t, r| {
                (s * 0.07 * (PI * (t * 1.15 + 0.05).min(1.0)).sin().sqrt())
                    .min(r * (PI / nf).sin() - g / 2.0)
            }),
            1.0,
        ));
    }
    for k in 0..n2 {
        items.push(sh(
            radial(
                Point::ZERO,
                a0 + TAU * (f64::from(k) + 0.5) / n2f,
                r3,
                r4,
                |t, r| (s * 0.05 * (PI * t).sin().powf(0.8)).min(r * (PI / n2f).sin() - g / 2.0),
            ),
            0.8,
        ));
    }
    Some(items)
}

/// A willow leaf of length `l` and width `wl` from `at` heading `h`, bending by `k`, with a
/// carved vein when `rib`.
pub fn willow_leaf(at: Point, h: f64, l: f64, wl: f64, k: f64, g: f64, rib: bool) -> Vec<Item> {
    let tr = trace(at, h, l, |_, _| k, (l / 80.0).max(0.15));
    let n = tr.p.len();
    let w: Vec<f64> = (0..n)
        .map(|i| wl * (PI * i as f64 / (n - 1) as f64).sin().powf(0.75))
        .collect();
    let mut out = vec![sh(band(&tr, &w, false, false), 0.8)];
    if rib && wl > g * 2.4 {
        let (i0, i1) = ((n as f64 * 0.12) as usize, (n as f64 * 0.82) as usize);
        out.push(Item::Line {
            points: tr.p[i0..i1].to_vec(),
            width: (g * 0.5).max(0.4),
        });
    }
    out
}

/// A curl dressed as a koru: its band (striped where wide enough), a bead row along its
/// outer side when `dots`, and an eye.
pub fn koru_items(cp: &Curl, s: f64, c: &Ctx, dots: bool) -> Vec<Item> {
    let g = c.g;
    let n = cp.track.p.len();
    let (p, h) = (&cp.track.p, &cp.track.h);
    let wmax = cp.w.iter().copied().fold(0.0, f64::max);
    let striped = wmax * 0.38 - g / 2.0 >= g * 0.8;
    let (mut a1, mut b1, mut a2, mut b2, mut q) = (vec![], vec![], vec![], vec![], vec![]);
    for &w in &cp.w {
        let qq = if striped {
            let th = 0.38 * w - g / 2.0;
            clamp((th - 0.7 * g) / (0.5 * g), 0.0, 1.0)
        } else {
            0.0
        };
        q.push(qq);
        a2.push(-w / 2.0);
        b2.push(-w / 2.0 + qq * (0.38 * w - g / 2.0));
        a1.push(-w / 2.0 + qq * (0.38 * w + g / 2.0));
        b1.push(w / 2.0);
    }
    let mut items = vec![sh(ribbon(&cp.track, &a1, &b1, false, true), 1.0)];
    if striped {
        let mut i = 0;
        while i < n {
            while i < n && q[i] <= 0.0 {
                i += 1;
            }
            let i0 = i;
            while i < n && q[i] > 0.0 {
                i += 1;
            }
            if i - i0 >= 5 {
                let t = Track {
                    p: p[i0..i].to_vec(),
                    h: h[i0..i].to_vec(),
                };
                items.push(sh(ribbon(&t, &a2[i0..i], &b2[i0..i], false, false), 0.75));
            }
        }
    }
    if dots && c.detail >= 0.45 {
        let rd = (g * 0.55).max(s * 0.022);
        let (mut acc, mut next) = (0.0, s * 0.18);
        for i in 1..n {
            acc += p[i].distance(p[i - 1]);
            if acc >= next && cp.turned[i] < TAU * 0.7 && cp.w[i] >= g {
                let o = -(cp.w[i] / 2.0 + g + rd);
                items.push(sh(circ(p[i] + normal(h[i]) * o, rd), 0.65));
                next = acc + 2.0 * rd + g * 1.15;
            }
        }
    }
    eye_at(&mut items, cp, g, s, c.detail >= 0.4);
    items
}

/// `items` turned by `h` (mirrored first when `flip`) and moved to `at`.
pub fn move_items(items: &[Item], at: Point, h: f64, flip: bool) -> Vec<Item> {
    let (c, s) = (h.cos(), h.sin());
    items
        .iter()
        .map(|it| {
            it.map(&|q: Point| {
                let (u, v) = (q.x, if flip { -q.y } else { q.y });
                at + pt(u * c - v * s, u * s + v * c)
            })
        })
        .collect()
}

/// `items` centred on `at` (by their box), turned by `rot`, mirrored first when `flip`.
pub fn place(items: &[Item], at: Point, rot: f64, flip: bool) -> Vec<Item> {
    let (mut lo, mut hi) = (Point::splat(f64::INFINITY), Point::splat(f64::NEG_INFINITY));
    for it in items {
        for q in it.points() {
            lo = lo.min(q);
            hi = hi.max(q);
        }
    }
    let centre = (lo + hi) / 2.0;
    let moved: Vec<Item> = items.iter().map(|it| it.map(&|q| q - centre)).collect();
    move_items(&moved, at, rot, flip)
}

/// A motif maker: a piece of size `s`, or `None` when it cannot be made that small.
pub type Maker = fn(f64, &mut Ctx) -> Option<Vec<Item>>;

/// Small pieces of one size: their size as a share of the room, how many tries, the makers
/// with their weights, and the refinement they need.
pub type Level = (f64, f64, &'static [(Maker, f64)], f64);

/// Points spread at random no closer than `r`, over a `w` by `h` box.
pub fn poisson(w: f64, h: f64, r: f64, rng: &mut Rng) -> Vec<Point> {
    let cs = r / std::f64::consts::SQRT_2;
    let (gw, gh) = ((w / cs).ceil() as usize + 1, (h / cs).ceil() as usize + 1);
    let mut grid = vec![usize::MAX; gw * gh];
    let mut pts: Vec<Point> = vec![];
    let mut act: Vec<usize> = vec![];
    let cell = |q: Point| ((q.y / cs) as usize, (q.x / cs) as usize);
    let first = pt(rng.range(0.0, w), rng.range(0.0, h));
    let (j, i) = cell(first);
    grid[j * gw + i] = 0;
    pts.push(first);
    act.push(0);
    while !act.is_empty() {
        let ai = ((rng.f() * act.len() as f64) as usize).min(act.len() - 1);
        let p = pts[act[ai]];
        let mut found = false;
        for _ in 0..24 {
            let q = p + dir(rng.range(0.0, TAU)) * rng.range(r, 2.0 * r);
            if q.x < 0.0 || q.y < 0.0 || q.x >= w || q.y >= h {
                continue;
            }
            let (gy, gx) = cell(q);
            let mut ok = true;
            'near: for y in gy.saturating_sub(2)..=(gy + 2).min(gh - 1) {
                for x in gx.saturating_sub(2)..=(gx + 2).min(gw - 1) {
                    let id = grid[y * gw + x];
                    if id != usize::MAX && pts[id].distance(q) < r {
                        ok = false;
                        break 'near;
                    }
                }
            }
            if ok {
                grid[gy * gw + gx] = pts.len();
                act.push(pts.len());
                pts.push(q);
                found = true;
                break;
            }
        }
        if !found {
            act.swap_remove(ai);
        }
    }
    let _ = gh as isize;
    pts
}

#[cfg(test)]
mod tests {
    use super::*;

    fn spec(rho0: f64, g: f64) -> CurlSpec {
        CurlSpec {
            rho0,
            b: 0.12,
            lead: 0.0,
            lead_k: 0.0,
            wmax: 5.0,
            max_turn: TAU * 1.5,
            g,
            taper: 0.0,
            ds: 0.0,
        }
    }

    #[test]
    fn a_curls_turns_never_come_closer_than_the_gap() {
        let g = 1.5;
        let c = curl_path(&spec(20.0, g)).expect("a curl");
        let (p, w) = (&c.track.p, &c.w);
        // points a full turn apart along the curl keep the gap between their bands
        for i in 0..p.len() {
            for j in i + 1..p.len() {
                if c.turned[j] - c.turned[i] > TAU * 0.9 {
                    let between = p[i].distance(p[j]) - f64::midpoint(w[i], w[j]);
                    assert!(between > g * 0.9, "{between} at {i}, {j}");
                }
            }
        }
    }

    #[test]
    fn too_small_a_curl_is_none() {
        assert!(curl_path(&spec(2.0, 1.5)).is_none());
    }

    #[test]
    fn a_ring_round_a_square_has_its_area() {
        let sq = vec![pt(0.0, 0.0), pt(2.0, 0.0), pt(2.0, 3.0), pt(0.0, 3.0)];
        assert!((ring_area(&sq) - 6.0).abs() < 1e-9);
    }

    #[test]
    fn spread_points_keep_their_distance() {
        let mut rng = Rng::new(3);
        let pts = poisson(40.0, 30.0, 2.4, &mut rng);
        assert!(pts.len() > 100);
        for (i, a) in pts.iter().enumerate() {
            for b in &pts[i + 1..] {
                assert!(a.distance(*b) >= 2.4 - 1e-9);
            }
        }
    }
}
