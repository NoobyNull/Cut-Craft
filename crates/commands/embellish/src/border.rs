//! Patterned borders that follow the outline: rope, chain, a two-strand twist, a
//! three-strand braid, bead and reel, laurel leaves, feathers and chevrons. Each runs along a centre line, the outline moved in
//! and rounded so the band never folds over itself, with a whole number of repeats so the
//! pattern meets itself. Pieces are drawn along the line (`s`) and across it (`v`, outward
//! positive), then laid onto it.

use crate::field::{Field, loop_len, resample_loop};
use crate::motif::{Item, Profile, Track, band, pt, sh};
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
}

/// How the border is drawn, in mm.
#[derive(Clone, Copy, Debug)]
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
}

/// A centre line round the shape.
pub struct Loop {
    p: Vec<Point>,
    h: Vec<f64>,
    k: Vec<f64>,
    sgn: f64,
    pub len: f64,
    ds: f64,
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
    /// The board point `v` across the line at `s` along it.
    pub fn at(&self, s: f64, v: f64) -> Point {
        let n = self.p.len();
        let nf = n as f64;
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
        let i = ((((s / self.ds) % nf + nf) % nf).floor() as usize).min(n - 1);
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
        });
    }
    out
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

fn count(len: f64, p: f64, least: f64) -> (f64, f64) {
    let n = (len / p).round().max(least);
    (n, len / n)
}

/// Rope: slanted lens-shaped strands laid side by side.
fn rope(lp: &Loop, hw: f64, p: f64, g: f64, lay: f64) -> Vec<Item> {
    let (n, pp) = count(lp.len, p, 3.0);
    let (slope, bow) = (lay.tan(), pp * 0.22);
    let f = |v: f64| slope * v + bow * (1.0 - (v / hw) * (v / hw));
    let half = (pp - g) / 2.0;
    if half < g * 0.4 {
        return vec![];
    }
    let mut out = vec![];
    for k in 0..n as usize {
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
    out
}

/// Chain: oval links seen face on, joined by links seen edge on that pass through them.
fn chain(lp: &Loop, hw: f64, p: f64, g: f64) -> Vec<Item> {
    let (n, pp) = count(lp.len, p, 3.0);
    let (a, b) = (pp * 0.4, hw);
    let t = (g * 1.2).max(hw * 0.36);
    let wb = (g * 1.1).max(t * 0.95);
    if b - t < wb / 2.0 + g || a - t < g * 2.0 {
        return vec![];
    }
    let mut out = vec![];
    for k in 0..n as usize {
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
    out
}

/// Plaits: `k` strands weaving over and under one another. At each crossing the strand
/// coming in from the edge goes over (for two strands, the one rising); the one under is
/// cut back to clear the one over by the gap.
fn plait(lp: &Loop, hw: f64, p: f64, g: f64, k: usize) -> Vec<Item> {
    let (_, period) = count(lp.len, p, 2.0);
    let ws = hw * if k == 2 { 0.55 } else { 0.48 };
    let amp = hw - ws / 2.0;
    if ws < g * 1.2 {
        return vec![];
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
        let start = kept.iter().position(|&x| !x).unwrap_or(0);
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
    out
}

/// Bead and reel: oval beads with a pair of thin discs between each two.
fn bead_and_reel(lp: &Loop, hw: f64, p: f64, g: f64) -> Vec<Item> {
    let (n, pp) = count(lp.len, p, 3.0);
    let (ab, tr) = (pp * 0.3, (g * 1.1).max(pp * 0.07));
    let mut out = vec![];
    for k in 0..n as usize {
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
    out
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
    let p = b.repeat * b.width;
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
        let mut items = vec![];
        let parts = match b.style {
            Style::Rope => rope(&lp, hw, p * 0.5, g, b.lay),
            Style::Chain => chain(&lp, hw, p * 1.4, g),
            Style::Twist => plait(&lp, hw, p * 2.4, g, 2),
            Style::Braid => plait(&lp, hw, p * 3.2, g, 3),
            Style::BeadAndReel => bead_and_reel(&lp, hw, p * 0.9, g),
            Style::Laurel => laurel(&lp, hw, p * 0.75, g),
            Style::Feathers => feathers(&lp, hw, p * 2.8, g),
            Style::Chevrons => chevrons(&lp, hw, p * 0.6, g),
        };
        let laid = !parts.is_empty();
        items.extend(parts.iter().map(|it| it.map(&|q: Point| lp.at(q.x, q.y))));
        // the laurel's stem, one band all the way round
        if b.style == Style::Laurel && laid {
            let ws = stem_width(hw, g);
            let n = (lp.len / 0.8).ceil() as usize;
            let ring = |v: f64| -> Vec<Point> {
                (0..n)
                    .map(|i| lp.at(lp.len * i as f64 / n as f64, v))
                    .collect()
            };
            items.push(Item::Shape {
                rings: vec![ring(-ws / 2.0), ring(ws / 2.0)],
                h: 0.8,
                profile: Profile::Round(ws * 0.5),
                sink: false,
            });
        }
        if b.rails {
            let n = (lp.len / 0.8).ceil() as usize;
            for side in [1.0, -1.0] {
                let v = side * (hw + g + rw / 2.0);
                let ring = |off: f64| -> Vec<Point> {
                    (0..n)
                        .map(|i| lp.at(lp.len * i as f64 / n as f64, v + off))
                        .collect()
                };
                items.push(Item::Shape {
                    rings: vec![ring(-rw / 2.0), ring(rw / 2.0)],
                    h: 0.85,
                    profile: Profile::Round(rw * 0.5),
                    sink: false,
                });
            }
        }
        loops.push(items);
    }
    (loops, dc + hw + rails)
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
fn chevrons(lp: &Loop, hw: f64, p: f64, g: f64) -> Vec<Item> {
    let (n, pp) = count(lp.len, p, 3.0);
    let h = hw * 0.92;
    let back = h * 0.9;
    // the arms slant, so the gap between two is the step along the line times the sine
    let sin = h / back.hypot(h);
    let t = pp - g / sin;
    if t < g * 1.2 {
        return vec![];
    }
    let mut out = vec![];
    for k in 0..n as usize {
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
    out
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
fn laurel(lp: &Loop, hw: f64, p: f64, g: f64) -> Vec<Item> {
    let (n, pp) = count(lp.len, p, 3.0);
    let ws = stem_width(hw, g);
    let ang = 0.62_f64;
    let v0 = ws / 2.0 + g;
    let len = (hw * 0.95 - v0) / ang.sin();
    // two neighbouring leaves lie side by side, the step times the sine apart
    let wide = (pp * ang.sin() - g).min(len * 0.42);
    if len < g * 3.0 || wide < g * 1.2 {
        return vec![];
    }
    let mut out = vec![];
    for k in 0..n as usize {
        let c = k as f64 * pp;
        for side in [1.0, -1.0] {
            let ring = leaf(pt(c, side * v0), side * ang, len, wide);
            out.push(sh(ring, 0.9));
        }
    }
    out
}

/// Feathers: one after another along the line, each a quill down its middle with its vanes
/// cut into slanting barbs either side.
fn feathers(lp: &Loop, hw: f64, p: f64, g: f64) -> Vec<Item> {
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
        return vec![];
    }
    let mut out = vec![];
    for k in 0..n as usize {
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
    out
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

    #[test]
    fn every_style_keeps_inside_its_band_and_clear_of_the_edge() {
        let field = square_field();
        for style in [
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
