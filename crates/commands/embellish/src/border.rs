//! Patterned borders that follow the outline: rope, chain, a two-strand twist, a
//! three-strand braid, bead and reel, laurel leaves, feathers and chevrons. Each runs along a centre line, the outline moved in
//! and rounded so the band never folds over itself, with a whole number of repeats so the
//! pattern meets itself. Pieces are drawn along the line (`s`) and across it (`v`, outward
//! positive), then laid onto it.

use crate::field::{Field, loop_len, resample_loop, resample_open};
use crate::motif::{Item, Profile, Track, band, pt, sh};
use crate::seq::{self, Seq};
use cut_craft_core::geometry::Point;
use std::f64::consts::{PI, TAU};

/// The border's pattern.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Style {
    Rope,
    Chain,
    Twist,
    Braid,
    BeadAndReel,
    Laurel,
    Feathers,
    Chevrons,
    BandBeads,
    /// A vine with scrolls, flowers, leaves and dots ([`crate::vine`]).
    Vine(crate::vine::Kind),
}

/// How the border is drawn, in mm.
#[derive(Clone, Copy, Debug)]
#[expect(clippy::struct_excessive_bools, reason = "one tick box each")]
pub struct Spec {
    pub style: Style,
    pub width: f64,
    /// The repeat's length as a share of the width (1 as designed).
    pub repeat: f64,
    /// The rope's lay angle, radians.
    pub lay: f64,
    /// How far in from the edge it starts.
    pub inset: f64,
    /// Raised rails along both its sides.
    pub rails: bool,
    /// One row only, down the middle of a narrow band, rather than one along each edge.
    pub single: bool,
    /// The pattern runs the other way along its line.
    pub reverse: bool,
    /// The vine's pieces and how they are drawn.
    pub vine: VineSpec,
    /// The vine's pieces stay upright rather than turning with the line.
    pub upright: bool,
    /// How each repeat in turn is drawn (empty: each as designed).
    pub sequence: Seq,
    /// Repeats of different sizes or turns, and the pieces' shape.
    pub vary: Vary,
}

/// What the vine pattern has, and how it is drawn.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct VineSpec {
    pub parts: crate::vine::Parts,
    pub detail: f64,
    pub fill: f64,
}

/// What may differ from one repeat to the next, and the number that picks the pieces'
/// shape and how they differ.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Vary {
    pub size: bool,
    pub turn: bool,
    pub seed: u32,
}

impl Default for VineSpec {
    fn default() -> Self {
        Self {
            parts: crate::vine::Parts::ALL,
            detail: 0.5,
            fill: 0.5,
        }
    }
}

impl Spec {
    /// How far the band reaches either side of its centre line, its rails too, with the gap
    /// `g` between the pieces.
    pub fn half_band(&self, g: f64) -> f64 {
        let hw = self.width / 2.0;
        if self.rails {
            hw + g + (g * 1.1).max(hw * 0.16)
        } else {
            hw
        }
    }
}

/// A centre line round the shape, or (not `closed`) along an open line from one end to the
/// other.
pub struct Loop {
    p: Vec<Point>,
    h: Vec<f64>,
    k: Vec<f64>,
    sgn: f64,
    pub len: f64,
    ds: f64,
    closed: bool,
}

fn wrap(a: f64) -> f64 {
    let mut a = a;
    while a > PI {
        a -= TAU;
    }
    while a < -PI {
        a += TAU;
    }
    a
}

/// The radius of the circle through the points `k` either side of `i`, round a ring.
fn ring_radius(p: &[Point], i: usize, k: usize) -> f64 {
    let n = p.len();
    if n < 3 {
        return 1e9;
    }
    let k = k.min((n - 1) / 2).max(1);
    let (a, b, c) = (p[(i + n - k) % n], p[i], p[(i + k) % n]);
    let cr = (b - a).perp_dot(c - a).abs();
    if cr < 1e-9 {
        1e9
    } else {
        a.distance(b) * b.distance(c) * c.distance(a) / (2.0 * cr)
    }
}

impl Loop {
    /// The board point `v` across the line at `s` along it. Past an open line's ends, the
    /// line goes on straight.
    pub fn at(&self, s: f64, v: f64) -> Point {
        let n = self.p.len();
        let nf = n as f64;
        if !self.closed {
            let side = |h: f64| pt(-h.sin(), h.cos()) * (v * self.sgn);
            let along = |h: f64| pt(h.cos(), h.sin());
            if s <= 0.0 {
                return self.p[0] + along(self.h[0]) * s + side(self.h[0]);
            }
            if s >= self.len {
                let h = self.h[n - 1];
                return self.p[n - 1] + along(h) * (s - self.len) + side(h);
            }
        }
        let u = ((s / self.ds) % nf + nf) % nf;
        let i = (u.floor() as usize).min(n - 1);
        let f = u - i as f64;
        let j = (i + 1) % n;
        let h = self.h[i] + wrap(self.h[j] - self.h[i]) * f;
        let vv = v * self.sgn;
        self.p[i].lerp(self.p[j], f) + pt(-h.sin(), h.cos()) * vv
    }

    /// How much a length along the line shrinks (or grows) at `v` across it.
    fn stretch(&self, s: f64, v: f64) -> f64 {
        let n = self.p.len();
        let nf = n as f64;
        let i = if self.closed {
            ((((s / self.ds) % nf + nf) % nf).floor() as usize).min(n - 1)
        } else {
            ((s / self.ds).max(0.0).floor() as usize).min(n - 1)
        };
        (1.0 - self.k[i] * v * self.sgn).max(0.2)
    }
}

/// The centre lines `d` in from the edge, rounded to at least twice `hw`.
pub fn centre_loops(field: &Field, d: f64, hw: f64) -> Vec<Loop> {
    let mut out = vec![];
    // a line too short to go round a circle the pattern can bend to holds none of it
    for ring in field.rings_longer(d, (4.0 * d + 6.0).min(TAU * 2.0 * hw)) {
        let mut p = resample_loop(&ring, 1.0);
        if p.len() < 20 {
            continue;
        }
        let rmin = hw * 2.0;
        for it in 0..900 {
            let n = p.len();
            let mut q = p.clone();
            let mut bad = false;
            for i in 0..n {
                if ring_radius(&p, i, 2) < rmin {
                    bad = true;
                    q[i] = (p[(i + n - 1) % n] + p[i] * 2.0 + p[(i + 1) % n]) / 4.0;
                }
            }
            p = q;
            if !bad {
                break;
            }
            if it % 20 == 19 {
                p = resample_loop(&p, 1.0);
            }
        }
        for _ in 0..3 {
            let n = p.len();
            p = (0..n)
                .map(|i| (p[(i + n - 1) % n] + p[i] * 2.0 + p[(i + 1) % n]) / 4.0)
                .collect();
        }
        let p = resample_loop(&p, 0.5);
        let n = p.len();
        if n < 20 {
            continue;
        }
        let h: Vec<f64> = (0..n)
            .map(|i| {
                let d = p[(i + 2) % n] - p[(i + n - 2) % n];
                d.y.atan2(d.x)
            })
            .collect();
        let len = loop_len(&p);
        let ds = len / n as f64;
        let k: Vec<f64> = (0..n)
            .map(|i| wrap(h[(i + 2) % n] - h[(i + n - 2) % n]) / (4.0 * ds))
            .collect();
        // +v outward: a step along the normal comes nearer the edge
        let m = n / 3;
        let nrm = pt(-h[m].sin(), h[m].cos()) * 1.5;
        let sgn = if field.sample(p[m] + nrm) < field.sample(p[m] - nrm) {
            1.0
        } else {
            -1.0
        };
        out.push(Loop {
            p,
            h,
            k,
            sgn,
            len,
            ds,
            closed: true,
        });
    }
    out
}

/// The centre line along the open line `line`, its bends rounded to at least twice `hw` (so
/// the band never folds over itself) and its ends where they were. `None` for a line too
/// short to hold any of the pattern.
pub fn open_line(line: &[Point], hw: f64) -> Option<Loop> {
    if line.len() < 2 {
        return None;
    }
    let mut p = resample_open(line, 1.0);
    let last = *line.last().expect("two points or more");
    if p.last().is_some_and(|q| q.distance(last) > 1e-6) {
        p.push(last);
    }
    if p.len() < 3 {
        return None;
    }
    let rmin = hw * 2.0;
    let radius = |p: &[Point], i: usize| -> f64 {
        let n = p.len();
        let k = 2.min(i).min(n - 1 - i);
        if k == 0 {
            return 1e9;
        }
        let (a, b, c) = (p[i - k], p[i], p[i + k]);
        let cr = (b - a).perp_dot(c - a).abs();
        if cr < 1e-9 {
            1e9
        } else {
            a.distance(b) * b.distance(c) * c.distance(a) / (2.0 * cr)
        }
    };
    let soften = |p: &[Point], i: usize| (p[i - 1] + p[i] * 2.0 + p[i + 1]) / 4.0;
    for it in 0..900 {
        let n = p.len();
        let mut q = p.clone();
        let mut bad = false;
        for (i, qi) in q.iter_mut().enumerate().take(n - 1).skip(1) {
            if radius(&p, i) < rmin {
                bad = true;
                *qi = soften(&p, i);
            }
        }
        p = q;
        if !bad {
            break;
        }
        if it % 20 == 19 {
            p = resample_open(&p, 1.0);
            p.push(last);
        }
    }
    for _ in 0..3 {
        let n = p.len();
        p = (0..n)
            .map(|i| {
                if i == 0 || i == n - 1 {
                    p[i]
                } else {
                    soften(&p, i)
                }
            })
            .collect();
    }
    let len: f64 = p.windows(2).map(|w| w[0].distance(w[1])).sum();
    if len < 1e-6 {
        return None;
    }
    let n = ((len / 0.5).ceil() as usize).max(2);
    let mut p = resample_open(&p, len / n as f64);
    p.truncate(n);
    p.push(last);
    let n = p.len();
    let ds = len / (n - 1) as f64;
    let h: Vec<f64> = (0..n)
        .map(|i| {
            let d = p[(i + 2).min(n - 1)] - p[i.saturating_sub(2)];
            d.y.atan2(d.x)
        })
        .collect();
    let k: Vec<f64> = (0..n)
        .map(|i| {
            let (a, b) = (i.saturating_sub(2), (i + 2).min(n - 1));
            wrap(h[b] - h[a]) / ((b - a) as f64 * ds)
        })
        .collect();
    Some(Loop {
        p,
        h,
        k,
        sgn: 1.0,
        len,
        ds,
        closed: false,
    })
}

fn ellipse(s0: f64, v0: f64, a: f64, b: f64, n: usize) -> Vec<Point> {
    (0..n)
        .map(|i| {
            let t = TAU * i as f64 / n as f64;
            pt(s0 + t.cos() * a, v0 + t.sin() * b)
        })
        .collect()
}

fn bar(s0: f64, s1: f64, w: f64, h: f64) -> Item {
    sh(
        vec![
            pt(s0, -w / 2.0),
            pt(s1, -w / 2.0),
            pt(s1, w / 2.0),
            pt(s0, w / 2.0),
        ],
        h,
    )
}

/// A pattern's pieces, and where each repeat's pieces start among them (none for a
/// pattern of unbroken strands).
type Units = (Vec<Item>, Vec<usize>);

fn count(len: f64, p: f64, least: f64) -> (f64, f64) {
    let n = (len / p).round().max(least);
    (n, len / n)
}

/// Rope: slanted lens-shaped strands laid side by side.
fn rope(lp: &Loop, hw: f64, p: f64, g: f64, lay: f64) -> Units {
    let (n, pp) = count(lp.len, p, 3.0);
    let (slope, bow) = (lay.tan(), pp * 0.22);
    let f = |v: f64| slope * v + bow * (1.0 - (v / hw) * (v / hw));
    let half = (pp - g) / 2.0;
    if half < g * 0.4 {
        return (vec![], vec![]);
    }
    let mut out = vec![];
    let mut starts = vec![];
    for k in 0..n as usize {
        starts.push(out.len());
        let c = (k as f64 + 0.5) * pp;
        let (mut l, mut r) = (vec![], vec![]);
        for i in 0..=24 {
            let v = -hw + 2.0 * hw * f64::from(i) / 24.0;
            let st = lp.stretch(c + f(v), v);
            let w = (half * (1.0 - (v.abs() / hw).powi(3)).max(0.0).sqrt()
                - g / 2.0 * (1.0 / st - 1.0))
                .max(0.0);
            l.push(pt(c + f(v) - w, v));
            r.push(pt(c + f(v) + w, v));
        }
        r.reverse();
        l.extend(r);
        out.push(sh(l, 1.0));
    }
    (out, starts)
}

/// Chain: oval links seen face on, joined by links seen edge on that pass through them.
fn chain(lp: &Loop, hw: f64, p: f64, g: f64) -> Units {
    let (n, pp) = count(lp.len, p, 3.0);
    let (a, b) = (pp * 0.4, hw);
    let t = (g * 1.2).max(hw * 0.36);
    let wb = (g * 1.1).max(t * 0.95);
    if b - t < wb / 2.0 + g || a - t < g * 2.0 {
        return (vec![], vec![]);
    }
    let mut out = vec![];
    let mut starts = vec![];
    for k in 0..n as usize {
        starts.push(out.len());
        let s0 = k as f64 * pp;
        out.push(Item::Shape {
            rings: vec![
                ellipse(s0, 0.0, a, b, 56),
                ellipse(s0, 0.0, a - t, b - t, 48),
            ],
            h: 0.9,
            profile: Profile::Pillow,
            sink: false,
        });
        let s1 = s0 + pp;
        let (lo, hi) = (s0 + a + g, s1 - a - g);
        if hi - lo > g * 0.5 {
            out.push(bar(lo, hi, wb, 1.0));
        }
        // the edge link's ends, inside the face links' holes
        for (c, dir) in [(s0, 1.0), (s1, -1.0)] {
            let (e0, e1) = (c + dir * (a - t - g), c + dir * g.max((a - t) * 0.25));
            if dir * (e0 - e1) > g {
                out.push(bar(e0.min(e1), e0.max(e1), wb, 0.7));
            }
        }
    }
    (out, starts)
}

/// Plaits: `k` strands weaving over and under one another. At each crossing the strand
/// coming in from the edge goes over (for two strands, the one rising); the one under is
/// cut back to clear the one over by the gap.
fn plait(lp: &Loop, hw: f64, p: f64, g: f64, k: usize) -> Units {
    let (_, period) = count(lp.len, p, 2.0);
    let ws = hw * if k == 2 { 0.55 } else { 0.48 };
    let amp = hw - ws / 2.0;
    if ws < g * 1.2 {
        return (vec![], vec![]);
    }
    let kf = k as f64;
    let ph = |i: usize, s: f64| TAU * s / period + TAU * i as f64 / kf;
    let val = |i: usize, s: f64| amp * ph(i, s).sin();
    let slope = |i: usize, s: f64| amp * TAU / period * ph(i, s).cos();
    let ds = 0.25;
    let steps = (lp.len / ds).ceil() as usize;
    let mut keep = vec![vec![true; steps]; k];
    for (i, kept) in keep.iter_mut().enumerate() {
        for (q, keep_q) in kept.iter_mut().enumerate() {
            let s = q as f64 * ds;
            for j in (0..k).filter(|&j| j != i) {
                if (val(i, s) - val(j, s)).abs() > (ws + g) * 2.5 {
                    continue;
                }
                // the nearest crossing of the two
                let mut sc = s;
                for _ in 0..20 {
                    let (d, dd) = (val(i, sc) - val(j, sc), slope(i, sc) - slope(j, sc));
                    if dd.abs() < 1e-9 {
                        break;
                    }
                    sc -= d / dd;
                    if (sc - s).abs() > period {
                        break;
                    }
                }
                if (sc - s).abs() > period * 0.5 {
                    continue;
                }
                let (di, dj, vc) = (slope(i, sc), slope(j, sc), val(i, sc));
                let over = if k == 2 {
                    di > 0.0
                } else {
                    di.signum() == -vc.signum()
                };
                if over {
                    continue;
                }
                let sin = (di - dj).abs() / (di.hypot(1.0) * dj.hypot(1.0));
                let cover = (ws + g) / sin.max(0.2);
                if (s - sc).abs() * di.hypot(1.0) < cover {
                    *keep_q = false;
                }
            }
        }
    }
    let mut out = vec![];
    for (i, kept) in keep.iter().enumerate() {
        // runs start at a gap, so none wraps round the end
        let start = if lp.closed {
            kept.iter().position(|&x| !x).unwrap_or(0)
        } else {
            0
        };
        let mut run: Vec<f64> = vec![];
        let mut flush = |run: &mut Vec<f64>| {
            if run.len() > 4 {
                let t = Track {
                    p: run.iter().map(|&s| pt(s, val(i, s))).collect(),
                    h: run.iter().map(|&s| slope(i, s).atan()).collect(),
                };
                let w = vec![ws; run.len()];
                out.push(sh(band(&t, &w, false, false), 1.0));
            }
            run.clear();
        };
        for c in 0..steps {
            if kept[(start + c) % steps] {
                run.push((start + c) as f64 * ds);
            } else {
                flush(&mut run);
            }
        }
        flush(&mut run);
    }
    (out, vec![])
}

/// Bead and reel: oval beads with a pair of thin discs between each two.
fn bead_and_reel(lp: &Loop, hw: f64, p: f64, g: f64) -> Units {
    let (n, pp) = count(lp.len, p, 3.0);
    let (ab, tr) = (pp * 0.3, (g * 1.1).max(pp * 0.07));
    let mut out = vec![];
    let mut starts = vec![];
    for k in 0..n as usize {
        starts.push(out.len());
        let s0 = k as f64 * pp;
        out.push(sh(ellipse(s0, 0.0, ab, hw * 0.92, 40), 1.0));
        let sm = s0 + pp / 2.0;
        for d in [-1.0, 1.0] {
            out.push(sh(
                ellipse(sm + d * (tr / 2.0 + g / 2.0), 0.0, tr / 2.0, hw * 0.7, 28),
                0.8,
            ));
        }
    }
    (out, starts)
}

/// The border's pieces on the board, and how far in from the edge it reaches.
pub fn design(field: &Field, b: &Spec, g: f64) -> (Vec<Item>, f64) {
    let (loops, reach) = design_loops(field, b, g);
    (loops.into_iter().flatten().collect(), reach)
}

/// [`design`], each centre line's pieces apart (a shape with a hole has one round each edge).
pub fn design_loops(field: &Field, b: &Spec, g: f64) -> (Vec<Vec<Item>>, f64) {
    let hw = b.width / 2.0;
    let rw = (g * 1.1).max(hw * 0.16);
    let rails = if b.rails { rw + g } else { 0.0 };
    let dc = b.inset + rails + hw;
    let mut loops = vec![];
    let mut lines = centre_loops(field, dc, hw);
    // a single row down a narrow band's middle: where the line is too broken up to follow,
    // a little nearer the edge
    let mut d = dc;
    while b.single && lines.is_empty() && d > b.inset + field.res {
        d -= field.res * 0.5;
        lines = centre_loops(field, d, hw);
    }
    if b.single {
        // the two lines either side of the band's middle: the longer one alone
        lines.sort_by(|a, c| c.len.total_cmp(&a.len));
        lines.truncate(1);
    }
    for lp in lines {
        loops.push(lay(&lp, b, g));
    }
    (loops, dc + hw + rails)
}

/// The pattern along the open line `line`: whole repeats, the first and last as near its ends
/// as they fit, the band centred on it.
pub fn along(line: &[Point], b: &Spec, g: f64) -> Vec<Item> {
    open_line(line, b.width / 2.0).map_or_else(Vec::new, |lp| lay(&lp, b, g))
}

/// One centre line's pieces.
fn lay(lp: &Loop, b: &Spec, g: f64) -> Vec<Item> {
    if let Style::Vine(kind) = b.style {
        return lay_vine(lp, b, g, kind);
    }
    let hw = b.width / 2.0;
    let rw = (g * 1.1).max(hw * 0.16);
    let p = b.repeat * b.width;
    let (parts, starts) = match b.style {
        Style::Rope => rope(lp, hw, p * 0.5, g, b.lay),
        Style::Chain => chain(lp, hw, p * 1.4, g),
        Style::Twist => plait(lp, hw, p * 2.4, g, 2),
        Style::Braid => plait(lp, hw, p * 3.2, g, 3),
        Style::BeadAndReel => bead_and_reel(lp, hw, p * 0.9, g),
        Style::Laurel => laurel(lp, hw, p * 0.75, g),
        Style::Feathers => feathers(lp, hw, p * 2.8, g),
        Style::Chevrons => chevrons(lp, hw, p * 0.6, g),
        Style::BandBeads => beads_beside_band(lp, hw, p * 0.5, g),
        Style::Vine(_) => unreachable!("laid by lay_vine"),
    };
    // the repeats, each its pieces; a plait is one piece all along, with no repeats to turn
    let turns = !starts.is_empty();
    let mut units = if turns {
        split(parts, &starts)
    } else {
        parts.into_iter().map(|it| vec![it]).collect()
    };
    let (mut from, mut to) = (0.0, lp.len);
    let mut shift = 0.0;
    if !lp.closed {
        // along an open line: the whole repeats that fit between its ends, centred
        let slack = g * 0.5;
        units.retain(|u| span(u).is_some_and(|(lo, hi)| lo.x >= -slack && hi.x <= lp.len + slack));
        if let Some((lo, hi)) = span(&units.concat()) {
            shift = (lp.len - lo.x - hi.x) / 2.0;
            (from, to) = ((lo.x + shift).max(0.0), (hi.x + shift).min(lp.len));
        }
    }
    // each repeat drawn as its digit in the sequence says, a space for a 0
    let seq = b.sequence;
    let mut parts: Vec<Item> = vec![];
    for (k, unit) in units.iter().enumerate() {
        let turn = if turns { seq.turn(k) } else { seq::turn(1) };
        let Some(turn) = turn else { continue };
        let Some((lo, hi)) = span(unit) else { continue };
        // each repeat its own size and lean, when they vary
        let mut r = crate::rng::Rng::new(b.vary.seed.wrapping_add(k as u32 * 7919));
        let scale = if b.vary.size || turn.size {
            r.range(0.7, 1.0)
        } else {
            1.0
        };
        let lean = if b.vary.turn || turn.lean {
            r.range(-PI / 4.0, PI / 4.0)
        } else {
            0.0
        };
        let turn = seq::Turn {
            angle: turn.angle + lean,
            ..turn
        };
        parts.extend(oriented(unit, lo, hi, turn, scale));
    }
    // mirrored along the line when reversed: what pointed forward points back
    let len = lp.len;
    parts = parts
        .iter()
        .map(|it| {
            it.map(&|q: Point| {
                let x = if b.reverse { len - q.x } else { q.x };
                pt(x + shift, q.y)
            })
        })
        .collect();
    let laid = !parts.is_empty();
    let mut items: Vec<Item> = parts
        .iter()
        .map(|it| it.map(&|q: Point| lp.at(q.x, q.y)))
        .collect();
    // the band beside the beads, all the way round (or along)
    if b.style == Style::BandBeads && laid {
        let (wb, side) = band_and_bead_sizes(hw, g);
        let v = if b.reverse { -side } else { side };
        items.push(strip(lp, v, wb, from, to, 0.9));
    }
    // the laurel's stem, one band all the way round (or along)
    if b.style == Style::Laurel && laid {
        let ws = stem_width(hw, g);
        items.push(strip(lp, 0.0, ws, from, to, 0.8));
    }
    if b.rails {
        for side in [1.0, -1.0] {
            let v = side * (hw + g + rw / 2.0);
            items.push(strip(lp, v, rw, 0.0, lp.len, 0.85));
        }
    }
    items
}

/// `unit` (its box `lo` to `hi`) mirrored and turned about its middle as `turn` says, and
/// `scale` times its size; a turned one is made smaller where it would reach out of its box.
pub(crate) fn oriented(
    unit: &[Item],
    lo: Point,
    hi: Point,
    turn: seq::Turn,
    scale: f64,
) -> Vec<Item> {
    let c = (lo + hi) / 2.0;
    let (sin, cos) = turn.angle.sin_cos();
    let spin = |q: Point| {
        let d = pt(
            if turn.back { c.x - q.x } else { q.x - c.x },
            if turn.across { c.y - q.y } else { q.y - c.y },
        );
        pt(d.x * cos - d.y * sin, d.x * sin + d.y * cos)
    };
    // the turned repeat's box, against the room it had
    let fit_k = if turn.angle == 0.0 {
        1.0
    } else {
        let turned: Vec<Point> = unit.iter().flat_map(Item::points).map(spin).collect();
        let reach = turned.iter().fold(pt(0.0, 0.0), |m, q| {
            pt(m.x.max(q.x.abs()), m.y.max(q.y.abs()))
        });
        let room = (hi - lo) / 2.0;
        let fit = |r: f64, n: f64| if n > 1e-9 { r / n } else { 1.0 };
        fit(room.x, reach.x).min(fit(room.y, reach.y)).min(1.0)
    };
    let k = fit_k * scale;
    unit.iter()
        .map(|it| it.map(&|q: Point| c + spin(q) * k))
        .collect()
}

/// `parts` cut into its repeats, each starting at the index in `starts`.
fn split(parts: Vec<Item>, starts: &[usize]) -> Vec<Vec<Item>> {
    let mut units: Vec<Vec<Item>> = vec![];
    for (i, it) in parts.into_iter().enumerate() {
        if units.is_empty() || starts.contains(&i) {
            units.push(vec![]);
        }
        if let Some(last) = units.last_mut() {
            last.push(it);
        }
    }
    units
}

/// The box round `items`' points, along (x) and across (y) the line.
fn span(items: &[Item]) -> Option<(Point, Point)> {
    items.iter().flat_map(Item::points).fold(None, |b, q| {
        Some(b.map_or((q, q), |(lo, hi): (Point, Point)| (lo.min(q), hi.max(q))))
    })
}

/// The vine pattern along one line: its half-waves fitted between the ends, each piece
/// turned with the line or (`upright`) only moved along it.
fn lay_vine(lp: &Loop, b: &Spec, g: f64, kind: crate::vine::Kind) -> Vec<Item> {
    let hw = b.width / 2.0;
    let o = crate::vine::Opts {
        kind,
        parts: b.vine.parts,
        detail: b.vine.detail,
        fill: b.vine.fill,
        seed: b.vary.seed,
        vary_size: b.vary.size,
        vary_turn: b.vary.turn,
        need_main: g > 0.1,
        seq: b.sequence,
    };
    let len = lp.len;
    let flip = |q: Point| if b.reverse { pt(len - q.x, q.y) } else { q };
    let mut items: Vec<Item> = crate::vine::pieces(len, lp.closed, hw, b.repeat * b.width, g, &o)
        .into_iter()
        .map(|piece| match piece.anchor.filter(|_| b.upright) {
            Some(a) => {
                let (a, at) = (flip(a), lp.at(flip(a).x, flip(a).y));
                piece.item.map(&|q: Point| at + (flip(q) - a))
            }
            None => piece.item.map(&|q: Point| {
                let q = flip(q);
                lp.at(q.x, q.y)
            }),
        })
        .collect();
    if b.rails {
        let rw = (g * 1.1).max(hw * 0.16);
        for side in [1.0, -1.0] {
            items.push(strip(lp, side * (hw + g + rw / 2.0), rw, 0.0, len, 0.85));
        }
    }
    items
}

/// A rounded strip `w` wide, `v` across the line, from `from` to `to` along it: a band all
/// the way round a closed line.
fn strip(lp: &Loop, v: f64, w: f64, from: f64, to: f64, h: f64) -> Item {
    let n = ((to - from) / 0.8).ceil().max(2.0) as usize;
    let rings = if lp.closed {
        let ring = |off: f64| -> Vec<Point> {
            (0..n)
                .map(|i| lp.at(lp.len * i as f64 / n as f64, v + off))
                .collect()
        };
        vec![ring(-w / 2.0), ring(w / 2.0)]
    } else {
        let at = |i: usize, off: f64| lp.at(from + (to - from) * i as f64 / n as f64, v + off);
        let mut ring: Vec<Point> = (0..=n).map(|i| at(i, -w / 2.0)).collect();
        ring.extend((0..=n).rev().map(|i| at(i, w / 2.0)));
        vec![ring]
    };
    Item::Shape {
        rings,
        h,
        profile: Profile::Round(w * 0.5),
        sink: false,
    }
}

/// Points along the straight line from `a` to `b`, no further apart than `step` (so the line
/// bends with the centre line it is laid on).
fn dense(a: Point, b: Point, step: f64) -> Vec<Point> {
    let n = (a.distance(b) / step).ceil().max(1.0) as usize;
    (0..n).map(|i| a.lerp(b, i as f64 / n as f64)).collect()
}

/// A closed outline through `corners`, with points added along its sides.
fn dense_ring(corners: &[Point], step: f64) -> Vec<Point> {
    let n = corners.len();
    (0..n)
        .flat_map(|i| dense(corners[i], corners[(i + 1) % n], step))
        .collect()
}

/// Chevrons: V shapes pointing along the line, one after another.
fn chevrons(lp: &Loop, hw: f64, p: f64, g: f64) -> Units {
    let (n, pp) = count(lp.len, p, 3.0);
    let h = hw * 0.92;
    let back = h * 0.9;
    // the arms slant, so the gap between two is the step along the line times the sine
    let sin = h / back.hypot(h);
    let t = pp - g / sin;
    if t < g * 1.2 {
        return (vec![], vec![]);
    }
    let mut out = vec![];
    let mut starts = vec![];
    for k in 0..n as usize {
        starts.push(out.len());
        let c = k as f64 * pp;
        let corners = [
            pt(c - back, -h),
            pt(c, 0.0),
            pt(c - back, h),
            pt(c - back - t, h),
            pt(c - t, 0.0),
            pt(c - back - t, -h),
        ];
        out.push(sh(dense_ring(&corners, 0.6), 1.0));
    }
    (out, starts)
}

/// The band and beads' band width, and how far its middle is off the line (on the outer
/// side); the beads fill the rest of the width, a gap from it.
fn band_and_bead_sizes(hw: f64, g: f64) -> (f64, f64) {
    let wb = hw.max(g * 1.2);
    (wb, hw - wb / 2.0)
}

/// Band and beads: a row of round beads beside a rounded band (the band is laid by
/// [`lay`]).
fn beads_beside_band(lp: &Loop, hw: f64, p: f64, g: f64) -> Units {
    let (wb, _) = band_and_bead_sizes(hw, g);
    // the beads between the band's inner edge (a gap off) and the far edge
    let room = 2.0 * hw - wb - g;
    let r = room / 2.0;
    if r < g * 0.5 {
        return (vec![], vec![]);
    }
    let (n, pp) = count(lp.len, p.max(2.0 * r + g), 3.0);
    let v = -hw + r;
    let items: Vec<Item> = (0..n as usize)
        .map(|k| {
            let s0 = (k as f64 + 0.5) * pp;
            Item::Shape {
                rings: vec![ellipse(s0, v, r, r, 32)],
                h: 0.8,
                profile: Profile::Ball,
                sink: false,
            }
        })
        .collect();
    let starts = (0..items.len()).collect();
    (items, starts)
}

/// The laurel's stem width.
fn stem_width(hw: f64, g: f64) -> f64 {
    (g * 1.1).max(hw * 0.16)
}

/// A leaf from `base` along `ang`, `len` long and `wide` across at its widest, pointed at
/// both ends and fullest a little past its middle.
fn leaf(base: Point, ang: f64, len: f64, wide: f64) -> Vec<Point> {
    let (d, nrm) = (pt(ang.cos(), ang.sin()), pt(-ang.sin(), ang.cos()));
    let m = 18;
    let half = |t: f64| wide / 2.0 * (PI * t.powf(0.8)).sin().max(0.0).powf(0.75);
    let mut ring: Vec<Point> = (0..=m)
        .map(|i| {
            let t = f64::from(i) / f64::from(m);
            base + d * (len * t) + nrm * half(t)
        })
        .collect();
    ring.extend((1..m).rev().map(|i| {
        let t = f64::from(i) / f64::from(m);
        base + d * (len * t) - nrm * half(t)
    }));
    ring
}

/// Laurel leaves: pairs of leaves slanting forward from a stem down the middle (the stem is
/// laid by [`design`]).
fn laurel(lp: &Loop, hw: f64, p: f64, g: f64) -> Units {
    let (n, pp) = count(lp.len, p, 3.0);
    let ws = stem_width(hw, g);
    let ang = 0.62_f64;
    let v0 = ws / 2.0 + g;
    let len = (hw * 0.95 - v0) / ang.sin();
    // two neighbouring leaves lie side by side, the step times the sine apart
    let wide = (pp * ang.sin() - g).min(len * 0.42);
    if len < g * 3.0 || wide < g * 1.2 {
        return (vec![], vec![]);
    }
    let mut out = vec![];
    let mut starts = vec![];
    for k in 0..n as usize {
        starts.push(out.len());
        let c = k as f64 * pp;
        for side in [1.0, -1.0] {
            let ring = leaf(pt(c, side * v0), side * ang, len, wide);
            out.push(sh(ring, 0.9));
        }
    }
    (out, starts)
}

/// Feathers: one after another along the line, each a quill down its middle with its vanes
/// cut into slanting barbs either side.
fn feathers(lp: &Loop, hw: f64, p: f64, g: f64) -> Units {
    let (n, pp) = count(lp.len, p, 2.0);
    let len = pp - g;
    let qw = (g * 0.9).max(hw * 0.12);
    let v0 = qw / 2.0 + g;
    // the vane's half width at `t` along the feather, 0 at its base and tip
    let width = |t: f64| {
        let t = t.clamp(0.0, 1.0);
        hw * 0.95 * ((PI * t.powf(0.55)).sin().max(0.0)).powf(0.4)
    };
    let barb = (g * 2.6).max(hw * 0.35);
    // barbs slant back from the quill toward the base
    let slant = 0.9_f64;
    if hw * 0.9 - v0 < g * 1.5 || len < 6.0 * g {
        return (vec![], vec![]);
    }
    let mut out = vec![];
    let mut starts = vec![];
    for k in 0..n as usize {
        starts.push(out.len());
        let s0 = k as f64 * pp;
        let at = |t: f64| s0 + t * len;
        // the quill, from just past the base to near the tip
        let (q0, q1) = (0.0, 0.94);
        let m = 30;
        let quill: Vec<Point> = (0..=m)
            .map(|i| pt(at(q0 + (q1 - q0) * f64::from(i) / f64::from(m)), -qw / 2.0))
            .chain(
                (0..=m)
                    .rev()
                    .map(|i| pt(at(q0 + (q1 - q0) * f64::from(i) / f64::from(m)), qw / 2.0)),
            )
            .collect();
        out.push(sh(quill, 1.0));
        // the vanes: from a tenth of the way along, cut into barbs
        let t_start = 0.1;
        let s_start = at(t_start);
        let s_end = at(1.0);
        let step = barb + g / slant.sin();
        let count_b = ((s_end - s_start) / step).floor() as usize;
        for side in [1.0, -1.0_f64] {
            for b in 0..count_b {
                // the barb's two slanting sides, by where they meet the quill
                let (u0, u1) = (s_start + b as f64 * step, s_start + b as f64 * step + barb);
                let k = 1.0 / slant.tan();
                // along a slanting side from the quill out to the vane's edge
                let side_line = |u: f64| -> Vec<Point> {
                    let mut pts = vec![];
                    let mut v = v0;
                    loop {
                        let s = u + (v - v0) * k;
                        let room = width((s - s0) / len);
                        if v > room || s > s_end {
                            break;
                        }
                        pts.push(pt(s, side * v));
                        v += 0.4;
                    }
                    pts
                };
                let (a, mut c) = (side_line(u0), side_line(u1));
                if a.len() < 2 || c.len() < 2 {
                    continue;
                }
                // out along one side, back along the other: the barb's outer end follows
                // the vane's edge between them
                c.reverse();
                let mut ring = a;
                ring.extend(c);
                out.push(sh(ring, 0.85));
            }
        }
    }
    (out, starts)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn square_field() -> Field {
        let sq = vec![
            pt(5.0, 5.0),
            pt(205.0, 5.0),
            pt(205.0, 205.0),
            pt(5.0, 205.0),
        ];
        Field::new(vec![sq], 210.0, 210.0)
    }

    fn spec(style: Style) -> Spec {
        Spec {
            style,
            width: 14.0,
            repeat: 1.0,
            lay: 45_f64.to_radians(),
            inset: 3.0,
            rails: true,
            single: false,
            reverse: false,
            vine: VineSpec::default(),
            upright: false,
            sequence: Seq::EMPTY,
            vary: Vary::default(),
        }
    }

    #[test]
    fn the_centre_line_is_rounded_and_points_outward() {
        let field = square_field();
        let loops = centre_loops(&field, 10.0, 7.0);
        assert_eq!(loops.len(), 1);
        let lp = &loops[0];
        assert!((lp.len - 4.0 * 180.0).abs() < 40.0, "{}", lp.len);
        for i in 0..200 {
            let s = lp.len * f64::from(i) / 200.0;
            let (mid, out) = (field.sample(lp.at(s, 0.0)), field.sample(lp.at(s, 3.0)));
            assert!(out < mid, "outward at {s}");
            assert!(mid > 9.0, "{mid} in at {s}");
        }
    }

    const ALL: [Style; 12] = [
        Style::Vine(crate::vine::Kind::FolkMix),
        Style::Vine(crate::vine::Kind::Scrolls),
        Style::Vine(crate::vine::Kind::Flowers),
        Style::BandBeads,
        Style::Rope,
        Style::Chain,
        Style::Twist,
        Style::Braid,
        Style::BeadAndReel,
        Style::Laurel,
        Style::Feathers,
        Style::Chevrons,
    ];

    /// An S-curve 300 mm long-ish, from (20, 100) to (320, 100).
    fn wave() -> Vec<Point> {
        (0..=300)
            .map(|i| {
                let x = f64::from(i);
                pt(20.0 + x, 100.0 + 30.0 * (x / 300.0 * TAU).sin())
            })
            .collect()
    }

    fn distance_to(line: &[Point], q: Point) -> f64 {
        line.windows(2)
            .map(|w| {
                let (a, d) = (w[0], w[1] - w[0]);
                let t = ((q - a).dot(d) / d.length_squared()).clamp(0.0, 1.0);
                (a + d * t).distance(q)
            })
            .fold(f64::INFINITY, f64::min)
    }

    #[test]
    fn an_open_lines_centre_line_runs_from_end_to_end() {
        let line = wave();
        let lp = open_line(&line, 7.0).unwrap();
        assert!(lp.at(0.0, 0.0).distance(line[0]) < 0.01);
        assert!(lp.at(lp.len, 0.0).distance(line[300]) < 0.01);
        // the wave's bends are gentle enough to keep: the line is followed closely
        for i in 0..=100 {
            let q = lp.at(lp.len * f64::from(i) / 100.0, 0.0);
            assert!(distance_to(&line, q) < 0.5, "{q:?} off the line");
        }
        // across is square to it, either side
        let (a, b) = (lp.at(lp.len / 2.0, 5.0), lp.at(lp.len / 2.0, -5.0));
        assert!((a.distance(b) - 10.0).abs() < 0.1);
    }

    #[test]
    fn every_style_runs_along_an_open_line_inside_its_band_from_end_to_end() {
        let line = wave();
        for style in ALL {
            // the vine's scrolls and flowers need a wider band at this gap
            let b = match style {
                Style::Vine(_) => Spec {
                    width: 30.0,
                    ..spec(style)
                },
                _ => spec(style),
            };
            let g = if matches!(style, Style::Vine(_)) {
                0.75
            } else {
                1.5
            };
            let items = along(&line, &b, g);
            assert!(items.len() > 8, "{style:?}: {}", items.len());
            let reach = b.half_band(1.5);
            let (mut near_start, mut near_end) = (f64::INFINITY, f64::INFINITY);
            for it in &items {
                for q in it.points() {
                    let d = distance_to(&line, q);
                    assert!(
                        d < reach + 0.5,
                        "{style:?}: {d} from the line, band {reach}"
                    );
                    near_start = near_start.min(q.distance(line[0]));
                    near_end = near_end.min(q.distance(line[300]));
                }
            }
            // the pattern reaches (nearly) both ends, as far one end as the other
            assert!(
                near_start < 20.0 && near_end < 20.0,
                "{style:?}: {near_start} {near_end}"
            );
            assert!(
                (near_start - near_end).abs() < 8.0,
                "{style:?}: {near_start} {near_end}"
            );
        }
    }

    #[test]
    fn a_sharp_corner_on_an_open_line_is_rounded_so_the_band_does_not_fold() {
        let mut line: Vec<Point> = (0..=100).map(|i| pt(f64::from(i), 0.0)).collect();
        line.extend((1..=100).map(|i| pt(100.0, f64::from(i))));
        let lp = open_line(&line, 7.0).unwrap();
        let corner = (0..=200)
            .map(|i| lp.at(lp.len * f64::from(i) / 200.0, 0.0))
            .map(|q| q.distance(pt(100.0, 0.0)))
            .fold(f64::INFINITY, f64::min);
        assert!(corner > 2.0, "cut inside the corner, {corner} from it");
        assert!(lp.at(0.0, 0.0).distance(line[0]) < 0.01);
    }

    #[test]
    fn the_sequence_draws_each_repeat_as_its_entry_says() {
        let line: Vec<Point> = (0..=200).map(|i| pt(f64::from(i), 0.0)).collect();
        let laid = |text: &str, vary: Vary| {
            let b = Spec {
                rails: false,
                sequence: Seq::parse(text),
                vary,
                ..spec(Style::Chevrons)
            };
            along(&line, &b, 1.5)
                .iter()
                .map(|it| it.points().collect::<Vec<Point>>())
                .collect::<Vec<_>>()
        };
        let plain = laid("", Vary::default());
        assert!(plain.len() >= 8);
        assert_eq!(
            laid("1", Vary::default()),
            plain,
            "empty is 1 over and over"
        );
        // a space every other repeat
        assert_eq!(laid("10", Vary::default()).len(), plain.len().div_ceil(2));
        // flipped across: the same count, mirrored top to bottom
        let flipped = laid("2", Vary::default());
        assert_eq!(flipped.len(), plain.len());
        assert_ne!(flipped, plain);
        // turned, leaning at random or a random size: each stays inside the band
        let half = spec(Style::Chevrons).width / 2.0;
        let random = Vary {
            size: true,
            turn: true,
            seed: 3,
        };
        for items in [
            laid("1/4,R1/4,2/8", Vary::default()),
            laid("", random),
            laid("ST(1,2),3", Vary::default()),
        ] {
            assert_eq!(items.len(), plain.len());
            for q in items.iter().flatten() {
                assert!(q.y.abs() <= half + 1e-6, "{q:?}");
            }
        }
        assert_ne!(laid("", random), plain);
        assert_ne!(laid("T1", Vary::default()), plain, "T leans each one");
    }

    #[test]
    fn reverse_runs_the_pattern_the_other_way_along_the_line() {
        // chevrons point along the line: their tips are the pieces' furthest-forward points
        let line: Vec<Point> = (0..=200).map(|i| pt(f64::from(i), 0.0)).collect();
        let tips = |reverse: bool| {
            let b = Spec {
                reverse,
                rails: false,
                ..spec(Style::Chevrons)
            };
            along(&line, &b, 1.5)
                .iter()
                .map(|it| {
                    // the point on the middle line, against the arms' ends' mean
                    let pts: Vec<Point> = it.points().collect();
                    let mid = pts
                        .iter()
                        .filter(|q| q.y.abs() < 0.5)
                        .map(|q| q.x)
                        .fold(f64::NAN, f64::max);
                    let ends = pts.iter().map(|q| q.x).sum::<f64>() / pts.len() as f64;
                    mid - ends
                })
                .collect::<Vec<f64>>()
        };
        assert!(tips(false).iter().all(|&d| d > 0.0), "forward");
        assert!(tips(true).iter().all(|&d| d < 0.0), "back");
    }

    #[test]
    fn band_and_beads_lays_a_band_and_a_row_of_beads_beside_it() {
        let line: Vec<Point> = (0..=200).map(|i| pt(f64::from(i), 0.0)).collect();
        let b = Spec {
            rails: false,
            ..spec(Style::BandBeads)
        };
        let items = along(&line, &b, 1.5);
        let beads = items
            .iter()
            .filter(|it| {
                matches!(
                    it,
                    Item::Shape {
                        profile: Profile::Ball,
                        ..
                    }
                )
            })
            .count();
        assert!(beads > 15, "{beads} beads");
        assert_eq!(items.len(), beads + 1, "and one band");
    }

    #[test]
    fn every_style_keeps_inside_its_band_and_clear_of_the_edge() {
        let field = square_field();
        for style in [
            Style::BandBeads,
            Style::Rope,
            Style::Chain,
            Style::Twist,
            Style::Braid,
            Style::BeadAndReel,
            Style::Laurel,
            Style::Feathers,
            Style::Chevrons,
        ] {
            let (items, reach) = design(&field, &spec(style), 1.5);
            assert!(items.len() > 20, "{style:?}: {}", items.len());
            for it in &items {
                for q in it.points() {
                    let d = field.sample(q);
                    // rounded corners sit further in than the sides
                    assert!(d > 2.0 && d < reach + 10.0, "{style:?}: {d} in");
                }
            }
        }
    }
}
