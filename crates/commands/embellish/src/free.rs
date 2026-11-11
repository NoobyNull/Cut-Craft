//! The design for any shape: a border band and bead row following the outline, a vine down
//! the middle of the shape between its two farthest ends with curls rolled at both, scrolls
//! branching off it side to side, leaf fans filling the sharp points, flowers in the
//! roomiest places, then scrolls and small pieces wherever there is room left.

use crate::design::{Gen, Settings, Where};
use crate::field::{Field, loop_len, resample_loop, resample_open};
use crate::motif::{
    Item, Level, Profile, Track, band, circ, curl_path, dir, dot_trio, fan_burst, flower_ring,
    koru_items, move_items, normal, place, polar, pt, rosette, sh, spiral_s, sunburst,
};
use cut_craft_core::geometry::Point;
use std::cmp::Reverse;
use std::collections::BinaryHeap;
use std::f64::consts::{PI, TAU};

/// The radius of the circle through the points three either side of `i`.
fn curve_radius(p: &[Point], i: usize) -> f64 {
    let (a, b, c) = (p[i.saturating_sub(3)], p[i], p[(i + 3).min(p.len() - 1)]);
    let (ab, bc, ca) = (a.distance(b), b.distance(c), c.distance(a));
    let cr = ((b - a).perp_dot(c - a)).abs();
    if cr < 1e-9 {
        1e9
    } else {
        ab * bc * ca / (2.0 * cr)
    }
}

fn headings(p: &[Point]) -> Vec<f64> {
    (0..p.len())
        .map(|i| {
            let (a, c) = (p[i.saturating_sub(1)], p[(i + 1).min(p.len() - 1)]);
            (c.y - a.y).atan2(c.x - a.x)
        })
        .collect()
}

/// A cost that orders by f64.
#[derive(PartialEq)]
struct Cost(f64);
impl Eq for Cost {}
impl PartialOrd for Cost {
    fn partial_cmp(&self, o: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(o))
    }
}
impl Ord for Cost {
    fn cmp(&self, o: &Self) -> std::cmp::Ordering {
        self.0.total_cmp(&o.0)
    }
}

/// Shortest paths over a grid of cells that are `ok`, from `src`; weighted to keep to the
/// wide parts when `cost` is given.
fn dijkstra(
    gx: usize,
    gy: usize,
    ok: &[bool],
    src: usize,
    cost: Option<&dyn Fn(usize) -> f64>,
) -> (Vec<f64>, Vec<usize>) {
    let n = gx * gy;
    let (mut d, mut prev) = (vec![f64::INFINITY; n], vec![usize::MAX; n]);
    d[src] = 0.0;
    let mut heap = BinaryHeap::new();
    heap.push(Reverse((Cost(0.0), src)));
    let steps: [(isize, isize, f64); 8] = [
        (1, 0, 1.0),
        (-1, 0, 1.0),
        (0, 1, 1.0),
        (0, -1, 1.0),
        (1, 1, 1.414),
        (1, -1, 1.414),
        (-1, 1, 1.414),
        (-1, -1, 1.414),
    ];
    while let Some(Reverse((Cost(du), u))) = heap.pop() {
        if du > d[u] {
            continue;
        }
        let (ui, uj) = (u % gx, u / gx);
        for &(dx, dy, l) in &steps {
            let (Some(vi), Some(vj)) = (ui.checked_add_signed(dx), uj.checked_add_signed(dy))
            else {
                continue;
            };
            if vi >= gx || vj >= gy {
                continue;
            }
            let v = vj * gx + vi;
            if !ok[v] {
                continue;
            }
            let c = cost.map_or(l, |f| l * f(v));
            if du + c < d[v] {
                d[v] = du + c;
                prev[v] = u;
                heap.push(Reverse((Cost(du + c), v)));
            }
        }
    }
    (d, prev)
}

fn farthest(d: &[f64]) -> usize {
    let mut best = (0, -1.0);
    for (i, &v) in d.iter().enumerate() {
        if v.is_finite() && v > best.1 {
            best = (i, v);
        }
    }
    best.0
}

/// The vine kept so far: its area, its band, its centre line and widths, and its end curls.
type Vine = (f64, Item, Track, Vec<f64>, Vec<End>);

/// A curl rolled at a vine's end.
struct End {
    curl: crate::motif::Curl,
    flip: bool,
    h: f64,
    at: Point,
}

pub fn design(field: &Field, s: Settings, pre: &[Item]) -> Vec<Item> {
    let g = s.gap;
    let (det, den, sc) = (s.detail, s.density, s.scale);
    let dm = field.dmax;
    let wb = (g * 1.3).max(dm * 0.06);
    let rd = (g * 0.6).max(dm * 0.018);
    let show_border = s.place != Where::Fill;
    let beads_on = show_border && det >= 0.35;
    let ib = g + wb + g * 1.7 + rd;
    let ii = if show_border {
        if beads_on { ib + rd + g } else { g + wb + g }
    } else {
        g
    }
    .max(s.inner_from);
    let iband = ii + dm * 0.45;
    let border_only = s.place == Where::Border;
    let mut gn = Gen::new(field, s, |p, m| {
        let v = field.sample(p);
        v > m && (!border_only || v < iband)
    });
    gn.occ.mark(pre);
    // border band and bead row, at fixed distances in from the outline
    if show_border {
        let mut rings = field.rings(g);
        rings.extend(field.rings(g + wb));
        gn.add(
            vec![Item::Shape {
                rings,
                h: 1.0,
                profile: Profile::Round((wb * 0.45).max(0.8)),
                sink: false,
            }],
            true,
        );
    }
    if beads_on {
        let sp = 2.0 * rd + g * 1.3;
        for l in field.rings(ib) {
            let pts = resample_loop(&l, sp);
            if pts.len() < 4 {
                continue;
            }
            for q in pts {
                gn.add(vec![sh(circ(q, rd), 0.7)], false);
            }
        }
    }
    let ws = (g * 1.4).max(dm * 0.05);
    let rcal = (g * 0.9).max(dm * 0.04);
    if !border_only {
        vine(&mut gn, ii, ws, sc, den);
    }
    // sharp points of the outline (its outer rings only)
    let kk = ((dm * 0.12 / 1.5).round() as usize).max(3);
    let mut tips = vec![];
    for (r, ring) in field.outline.iter().enumerate() {
        let hole = field
            .outline
            .iter()
            .enumerate()
            .filter(|&(o, other)| o != r && inside_ring(other, ring[0]))
            .count()
            % 2
            == 1;
        if hole || loop_len(ring) < 10.0 {
            continue;
        }
        let ol = resample_loop(ring, 1.5);
        let n = ol.len();
        if n < 4 * kk + 2 {
            continue;
        }
        let turn: Vec<f64> = (0..n)
            .map(|i| {
                let (a, p, c) = (ol[(i + n - kk) % n], ol[i], ol[(i + kk) % n]);
                let (v1, v2) = (p - a, c - p);
                v1.perp_dot(v2).atan2(v1.dot(v2))
            })
            .collect();
        let sgn = if turn.iter().sum::<f64>() < 0.0 {
            -1.0
        } else {
            1.0
        };
        for i in 0..n {
            let t = turn[i] * sgn;
            if t < 1.2 {
                continue;
            }
            let peak = (1..=2 * kk)
                .all(|d| turn[(i + d) % n] * sgn <= t && turn[(i + n - d) % n] * sgn <= t);
            if peak {
                tips.push(ol[i]);
            }
        }
    }
    // flowers at the roomiest places, the peaks of the distance field
    let st = ((dm * 0.25 / field.res).round() as usize).max(2);
    let (nx, ny) = (field.nx, field.ny);
    let mut peaks = vec![];
    if nx > 2 * st && ny > 2 * st {
        for j in (st..ny - st).step_by(2) {
            for i in (st..nx - st).step_by(2) {
                let v = field.d[j * nx + i];
                if f64::from(v) < dm * 0.45 {
                    continue;
                }
                let top = (j - st..=j + st).step_by(2).all(|jj| {
                    (i - st..=i + st)
                        .step_by(2)
                        .all(|ii| field.d[jj * nx + ii] <= v)
                });
                if top {
                    peaks.push((pt(i as f64 * field.res, j as f64 * field.res), f64::from(v)));
                }
            }
        }
    }
    peaks.sort_by(|a, b| b.1.total_cmp(&a.1));
    let mut flowers: Vec<(Point, f64)> = vec![];
    for (p, v) in peaks {
        if flowers.iter().any(|f| f.0.distance(p) < f.1 + v * 0.8) {
            continue;
        }
        let rf = (v - ii - g).min(dm * 0.42) * if flowers.is_empty() { 1.0 } else { 0.75 };
        if rf < 4.0 * g {
            continue;
        }
        let Some(loc) = rosette(rf / 0.45, &mut gn.ctx) else {
            continue;
        };
        let rot = gn.r(0.0, TAU);
        if gn.add(place(&loc, p, rot, false), false) {
            flowers.push((p, rf));
        }
    }
    // a leaf fan fills each sharp point
    for tp in tips {
        let mut p = tp;
        let mut guard = 0;
        while field.sample(p) < ii + rcal * 1.5 && guard < 400 {
            let gr = field.grad(p);
            let l = gr.length().max(1e-9);
            p += gr / l * 0.8;
            guard += 1;
        }
        let base = p;
        let into = tp - base;
        let ang = into.y.atan2(into.x);
        if !gn.add(vec![sh(circ(base, rcal), 1.0)], false) {
            continue;
        }
        let sig = gn.r(0.36, 0.48);
        for j in [0.0, 1.0, -1.0, 2.0, -2.0_f64] {
            let d2 = ang + j * sig;
            let mut l = 0.0;
            while l < dm * 1.5 && field.sample(base + dir(d2) * (rcal + l)) > ii {
                l += 0.5;
            }
            l = (l - g) * if j == 0.0 { 1.0 } else { 0.9 };
            if l < 3.0 * g {
                continue;
            }
            let leaf = polar(
                base,
                d2,
                rcal + g,
                rcal + l,
                |t| j.signum() * if j == 0.0 { 0.0 } else { 0.2 * t * t },
                |t, r| {
                    let room = (field.sample(base + dir(d2) * r) - ii) * 0.8;
                    (room.min(dm * 0.07 * (PI * t.powf(0.8)).sin().powf(0.8)) / r)
                        .atan()
                        .min(sig / 2.0 - g / (2.0 * r))
                },
            );
            gn.add(vec![sh(leaf, 0.85)], false);
        }
    }
    // scrolls, the biggest that fits first
    let rounds = (8.0 + 40.0 * den).round() as usize;
    for _ in 0..rounds {
        let mut best: Option<(f64, Vec<Item>)> = None;
        for _ in 0..160 {
            let Some(q) = random_inside(&mut gn, ii + g) else {
                continue;
            };
            let room = field.sample(q);
            let size = (dm * 1.4 * sc).min(room * gn.r(1.2, 3.2) * sc);
            let spec = gn.curl(
                size,
                (0.2, 0.32),
                (0.1, 0.14),
                (0.4, 1.1),
                0.8,
                (g * 1.5).max(size * 0.16),
                (1.0, 1.6),
                (0.2, 0.35),
            );
            let Some(cp) = curl_path(&spec) else { continue };
            let rot = gn.r(0.0, TAU);
            let flip = gn.chance(0.5);
            let items = place(
                &koru_items(&cp, size, &gn.ctx, size > dm * 0.5),
                q,
                rot,
                flip,
            );
            if !gn.occ.fits(&items) {
                continue;
            }
            let a = crate::motif::items_area(&items);
            if best.as_ref().is_none_or(|b| a > b.0) {
                best = Some((a, items));
            }
        }
        match best {
            Some((a, items)) if a >= dm * dm * 0.02 => {
                gn.add(items, true);
            }
            _ => break,
        }
    }
    // small pieces, sized from the shape
    let levels: [Level; 3] = [
        (
            0.5,
            300.0,
            &[(flower_ring, 1.0), (spiral_s, 2.0), (fan_burst, 1.0)],
            0.0,
        ),
        (0.3, 400.0, &[(spiral_s, 2.0), (flower_ring, 1.0)], 0.35),
        (0.18, 300.0, &[(spiral_s, 1.0), (dot_trio, 1.0)], 0.65),
    ];
    for (f, tries, makers, dmin) in levels {
        if det < dmin {
            continue;
        }
        for _ in 0..(tries * den * 1.6).round() as usize {
            let Some(q) = random_inside(&mut gn, ii + g) else {
                continue;
            };
            let size = dm * f * gn.r(0.85, 1.15);
            let make = gn.rng().pick(makers);
            let Some(loc) = make(size, &mut gn.ctx) else {
                continue;
            };
            let rot = gn.r(0.0, TAU);
            let flip = gn.chance(0.5);
            gn.add(place(&loc, q, rot, flip), false);
        }
    }
    gn.stipple();
    gn.out
}

pub fn inside_ring(ring: &[Point], p: Point) -> bool {
    let n = ring.len();
    let mut inside = false;
    for i in 0..n {
        let (a, b) = (ring[i], ring[(i + n - 1) % n]);
        if (a.y > p.y) != (b.y > p.y) && p.x < a.x + (p.y - a.y) * (b.x - a.x) / (b.y - a.y) {
            inside = !inside;
        }
    }
    inside
}

/// A random point at least `m` in from the outline.
fn random_inside(gn: &mut Gen, m: f64) -> Option<Point> {
    let (w, h) = (gn.field.w, gn.field.h);
    for _ in 0..50 {
        let p = pt(gn.r(0.0, w), gn.r(0.0, h));
        if gn.field.sample(p) > m {
            return Some(p);
        }
    }
    None
}

/// The vine: the middle line between the shape's two farthest ends, smoothed, its ends
/// rolled into curls, and scrolls branching off it.
fn vine(gn: &mut Gen, ii: f64, ws: f64, sc: f64, den: f64) {
    let field = gn.field;
    let (g, dm) = (gn.s.gap, field.dmax);
    let cs = 2.0;
    let (gx, gy) = (
        (field.w / cs).ceil() as usize,
        (field.h / cs).ceil() as usize,
    );
    let n = gx * gy;
    let dc: Vec<f64> = (0..n)
        .map(|k| {
            field.sample(pt(
                ((k % gx) as f64 + 0.5) * cs,
                ((k / gx) as f64 + 0.5) * cs,
            ))
        })
        .collect();
    let ok: Vec<bool> = dc.iter().map(|&d| d > ii + ws).collect();
    let src = farthest(&dc);
    if !ok[src] {
        return;
    }
    let weight = |v: usize| (dm / dc[v]).powi(3);
    let a = farthest(&dijkstra(gx, gy, &ok, src, None).0);
    let (_, prev) = dijkstra(gx, gy, &ok, a, Some(&weight));
    let b = farthest(&dijkstra(gx, gy, &ok, a, None).0);
    let mut path = vec![];
    let mut u = b;
    while u != usize::MAX {
        path.push(pt(
            ((u % gx) as f64 + 0.5) * cs,
            ((u / gx) as f64 + 0.5) * cs,
        ));
        u = prev[u];
    }
    if path.len() <= 10 {
        return;
    }
    let mut pts = resample_open(&path, 1.5);
    for _ in 0..60 {
        let q = pts.clone();
        for i in 1..pts.len() - 1 {
            pts[i] = (q[i - 1] + q[i] * 2.0 + q[i + 1]) / 4.0;
        }
    }
    let pts = resample_open(&pts, 1.5);
    let wv = (g * 1.6).max(dm * 0.075);
    // the ends roll up into curls: trims and curl sizes are tried, the fullest kept
    let mut best: Option<Vine> = None;
    for _ in 0..160 {
        let n0 = pts.len();
        let ta = (n0 as f64 * gn.r(0.0, 0.3)) as usize;
        let tb = (n0 as f64 * gn.r(0.0, 0.3)) as usize;
        if n0 < ta + tb + 10 {
            continue;
        }
        let mut p: Vec<Point> = pts[ta..n0 - tb].to_vec();
        let mut ends = vec![];
        let mut all = true;
        for end in [true, false] {
            let h0 = headings(&p);
            let i = if end { p.len() - 1 } else { 0 };
            let h = if end { h0[i] } else { h0[i] + PI };
            let room = field.sample(p[i]);
            let spec = crate::motif::CurlSpec {
                rho0: room * gn.r(0.3, 0.75),
                b: gn.r(0.1, 0.14),
                lead: 0.0,
                lead_k: 0.0,
                wmax: wv,
                max_turn: TAU * gn.r(0.8, 1.4),
                g,
                taper: 0.0,
                ds: 1.5,
            };
            let Some(cp) = curl_path(&spec) else {
                all = false;
                break;
            };
            let flip = gn.chance(0.5);
            let (c, s) = (h.cos(), h.sin());
            let at = p[i];
            let cpts: Vec<Point> = cp.track.p[1..]
                .iter()
                .map(|q| {
                    let (u, v) = (q.x, if flip { -q.y } else { q.y });
                    at + pt(u * c - v * s, u * s + v * c)
                })
                .collect();
            ends.push(End {
                curl: cp,
                flip,
                h,
                at,
            });
            if end {
                p.extend(cpts);
            } else {
                let mut r: Vec<Point> = cpts.into_iter().rev().collect();
                r.extend(p);
                p = r;
            }
        }
        if !all {
            continue;
        }
        let h = headings(&p);
        let w: Vec<f64> = (0..p.len())
            .map(|i| wv.min(0.32 * curve_radius(&p, i).max(0.1)))
            .collect();
        let t = Track { p, h };
        let it = sh(band(&t, &w, true, true), 0.95);
        if !gn.occ.fits(std::slice::from_ref(&it)) {
            continue;
        }
        let a = it.area();
        if best.as_ref().is_none_or(|b| a > b.0) {
            best = Some((a, it, t, w, ends));
        }
    }
    let Some((_, it, t, w, ends)) = best else {
        return;
    };
    gn.add(vec![it], true);
    for e in &ends {
        let cp = &e.curl;
        let m = cp.track.p.len() - 1;
        let (pe, he, rho) = (cp.track.p[m], cp.track.h[m], cp.rho_end);
        let mut c = pe + normal(he) * rho;
        if e.flip {
            c.y = -c.y;
        }
        let x = e.at
            + pt(
                c.x * e.h.cos() - c.y * e.h.sin(),
                c.x * e.h.sin() + c.y * e.h.cos(),
            );
        let er = (rho - wv / 2.0 - g) * 0.85;
        if er > 4.0 * g {
            let mut its = vec![];
            sunburst(&mut its, x, er, g, dm, 0.0, TAU, 0.8);
            gn.add(its, false);
        } else if er > g * 0.5 {
            gn.add(vec![sh(circ(x, er), 0.9)], false);
        }
    }
    // scrolls branch off it, side to side
    let n = t.p.len();
    let step = ((dm * 0.35 / 1.5 / (0.4 + den)).round() as usize).max(5);
    let mut side = 1.0;
    let mut i = (n as f64 * 0.08).round() as usize;
    while (i as f64) < n as f64 * 0.92 {
        let mut best: Option<(f64, Vec<Item>)> = None;
        for k in 0..120 {
            let sd = if k % 2 == 1 { side } else { -side };
            let room = field.sample(t.p[i]);
            let size = room * gn.r(0.8, 2.4) * sc;
            let spec = gn.curl(
                size,
                (0.2, 0.32),
                (0.1, 0.14),
                (0.3, 0.9),
                0.6,
                (g * 1.4).max(size * 0.15),
                (1.0, 1.5),
                (0.2, 0.35),
            );
            let Some(cp) = curl_path(&spec) else { continue };
            let off = w[i] / 2.0 + g * 1.7;
            let x0 = t.p[i] + normal(t.h[i]) * off * sd;
            let turn = gn.r(0.35, 1.0) * if gn.chance(0.5) { 1.0 } else { -1.0 };
            let back = if gn.chance(0.3) { PI } else { 0.0 };
            let h0 = t.h[i] + sd * turn + back;
            let items = move_items(
                &koru_items(&cp, size, &gn.ctx, size > dm * 0.5),
                x0,
                h0,
                sd < 0.0,
            );
            if !gn.occ.fits(&items) {
                continue;
            }
            let a = crate::motif::items_area(&items);
            if best.as_ref().is_none_or(|b| a > b.0) {
                best = Some((a, items));
            }
        }
        if let Some((_, items)) = best {
            gn.add(items, true);
            side = -side;
        }
        i += step;
    }
}
