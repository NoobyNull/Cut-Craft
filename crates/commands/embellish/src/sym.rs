//! The design for a shape with n-fold mirror symmetry (a regular polygon, a star, a polygon
//! with hanging-chain sides): one wedge, from a corner to the middle of the next side, is
//! designed and then mirrored and turned into every other. Corners get a stem and a leaf
//! fan, sides a half sunburst, the middle a rosette with a petal for every corner and side,
//! and the scrolls are searched for the biggest that fits the wedge.

use crate::design::{Gen, Settings, Where};
use crate::field::Field;
use crate::motif::{
    Item, Level, Profile, band, circ, curl_path, dir, dot, dot_trio, fan_burst, flower_ring,
    items_area, koru_items, move_items, place, polar, pt, sh, spiral_s, sunburst, trace,
    willow_leaf,
};
use cut_craft_core::geometry::Point;
use std::f64::consts::{PI, TAU};

/// A shape's symmetry: `n` corners round `centre`, the first at `corner` radians, each
/// mirrored about its own line.
#[derive(Clone, Copy, Debug)]
pub struct Symmetry {
    pub n: u32,
    pub centre: Point,
    pub corner: f64,
}

/// How many directions the shape's outline is read in, to find its symmetry: divisible by
/// every count of corners looked for.
const RAYS: usize = 2520;

/// How far from `centre` the outline `ring` is at angle `ang` (the nearest crossing).
fn reach(ring: &[Point], centre: Point, ang: f64) -> f64 {
    let d = dir(ang);
    let n = ring.len();
    let mut best = f64::INFINITY;
    for i in 0..n {
        let (a, b) = (ring[i] - centre, ring[(i + 1) % n] - centre);
        let e = b - a;
        let den = d.perp_dot(e);
        if den.abs() < 1e-12 {
            continue;
        }
        let t = a.perp_dot(e) / den;
        let u = a.perp_dot(d) / den;
        if t > 0.0 && (0.0..=1.0).contains(&u) {
            best = best.min(t);
        }
    }
    best
}

/// The shape's symmetry, when it is one closed outline that matches itself turned by a
/// whole share of a turn (3 to 12) and mirrored about each corner's line.
pub fn detect(field: &Field) -> Option<Symmetry> {
    let [ring] = field.outline.as_slice() else {
        return None;
    };
    // the area's centroid
    let n = ring.len();
    let (mut a, mut c) = (0.0, Point::ZERO);
    for i in 0..n {
        let (p, q) = (ring[i], ring[(i + 1) % n]);
        let cr = p.perp_dot(q);
        a += cr;
        c += (p + q) * cr;
    }
    if a.abs() < 1e-9 {
        return None;
    }
    let centre = c / (3.0 * a);
    if field.sample(centre) <= 0.0 {
        return None;
    }
    let r: Vec<f64> = (0..RAYS)
        .map(|i| reach(ring, centre, TAU * i as f64 / RAYS as f64))
        .collect();
    if r.iter().any(|v| !v.is_finite()) {
        return None;
    }
    let (rmin, rmax) = r.iter().fold((f64::INFINITY, 0.0_f64), |(lo, hi), &v| {
        (lo.min(v), hi.max(v))
    });
    if (rmax - rmin) / rmax < 0.03 {
        return None; // round: no corners to work from
    }
    let tol = rmax * 0.015;
    // the corner: the outline's farthest point, and the ray nearest it
    let far = ring
        .iter()
        .copied()
        .max_by(|a, b| a.distance(centre).total_cmp(&b.distance(centre)))?;
    let corner = (far - centre).y.atan2((far - centre).x).rem_euclid(TAU);
    let top = ((corner / TAU * RAYS as f64).round() as usize) % RAYS;
    for corners in (3..=12_u32).rev() {
        if !RAYS.is_multiple_of(corners as usize) {
            continue;
        }
        let step = RAYS / corners as usize;
        let turned = (0..RAYS).all(|i| (r[i] - r[(i + step) % RAYS]).abs() <= tol);
        let mirrored =
            (0..RAYS).all(|d| (r[(top + d) % RAYS] - r[(top + RAYS - d) % RAYS]).abs() <= tol);
        if turned && mirrored {
            return Some(Symmetry {
                n: corners,
                centre,
                corner,
            });
        }
    }
    None
}

/// The wedge's frame: the corner along -y, the middle of the next side `PI / n` round from
/// it, the first of the symmetry's mirrored copies.
struct Frame {
    n: u32,
    centre: Point,
    rot: f64,
    most: f64,
}

impl Frame {
    fn to_board(&self, q: Point) -> Point {
        let (c, s) = (self.rot.cos(), self.rot.sin());
        self.centre + pt(q.x * c - q.y * s, q.x * s + q.y * c)
    }

    /// Every copy of the wedge: each turn, plain and mirrored.
    fn copies(&self, mirrored: bool) -> Vec<(f64, f64)> {
        let mut out = vec![];
        for k in 0..self.n {
            let a = TAU * f64::from(k) / f64::from(self.n);
            out.push((a, 1.0));
            if mirrored {
                out.push((a, -1.0));
            }
        }
        out
    }

    fn map(&self, items: &[Item], (a, m): (f64, f64)) -> Vec<Item> {
        let (c, s) = (a.cos(), a.sin());
        items
            .iter()
            .map(|it| {
                it.map(&|q: Point| {
                    let x = q.x * m;
                    self.to_board(pt(x * c - q.y * s, x * s + q.y * c))
                })
            })
            .collect()
    }
}

pub fn design(field: &Field, sym: Symmetry, s: Settings, pre: &[Item]) -> Vec<Item> {
    let g = s.gap;
    let (det, den, sc) = (s.detail, s.density, s.scale);
    let n = sym.n;
    let nf = f64::from(n);
    let fr = Frame {
        n,
        centre: sym.centre,
        rot: sym.corner + PI / 2.0,
        most: field.w.hypot(field.h),
    };
    let reach_at = |ang: f64, inset: f64| {
        let d = dir(ang + fr.rot);
        let (mut lo, mut hi) = (0.0, fr.most);
        for _ in 0..30 {
            let mid = f64::midpoint(lo, hi);
            if field.sample(fr.centre + d * mid) > inset {
                lo = mid;
            } else {
                hi = mid;
            }
        }
        lo
    };
    let phi_m = -PI / 2.0 + PI / nf;
    let apothem = reach_at(phi_m, 0.0);
    let wb = (g * 1.3).max(apothem * 0.05);
    let rd = (g * 0.6).max(apothem * 0.017);
    let show_border = s.place != Where::Fill;
    let beads_on = show_border && det >= 0.35;
    let ib = g + wb + g + rd;
    let ii = if show_border {
        if beads_on { ib + rd + g } else { g + wb + g }
    } else {
        g
    }
    .max(s.inner_from);
    let iband = ii + apothem * 0.3;
    let border_only = s.place == Where::Border;
    let mut gn = Gen::new(field, s, |p, m| {
        let v = field.sample(p);
        v > m && (!border_only || v < iband)
    });
    gn.occ.mark(pre);
    let all = fr.copies(true);
    let turns = fr.copies(false);
    let put = |gn: &mut Gen, items: &[Item], list: &[(f64, f64)]| {
        for &t in list {
            let mi = fr.map(items, t);
            gn.occ.mark(&mi);
            gn.out.extend(mi);
        }
    };
    let u = dir(phi_m);
    let in_wedge = |items: &[Item]| {
        items.iter().all(|it| {
            let m = g / 2.0
                + match it {
                    Item::Line { width, .. } => width / 2.0,
                    Item::Shape { .. } => 0.0,
                };
            let ring: Box<dyn Iterator<Item = Point>> = match it {
                Item::Shape { rings, .. } => Box::new(rings[0].iter().copied()),
                Item::Line { points, .. } => Box::new(points.iter().copied()),
            };
            ring.into_iter()
                .all(|q| q.x >= m && q.x * u.y - q.y * u.x >= m)
        })
    };
    let to_board = |items: &[Item]| fr.map(items, (0.0, 1.0));
    let try_wedge = |gn: &mut Gen, items: Option<Vec<Item>>| -> bool {
        let Some(items) = items else { return false };
        if items.is_empty() || !in_wedge(&items) || !gn.occ.fits(&to_board(&items)) {
            return false;
        }
        put(gn, &items, &all);
        true
    };
    // the frame: a rounded border band, and a bead row inside it
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
        let path: Vec<Point> = (0..=200)
            .map(|i| {
                let ang = phi_m - (PI / nf) * f64::from(i) / 200.0;
                dir(ang) * reach_at(ang, ib)
            })
            .collect();
        let mut beads = vec![];
        let (mut acc, mut next) = (0.0, sp / 2.0);
        for w in path.windows(2) {
            acc += w[0].distance(w[1]);
            if acc >= next {
                beads.push(sh(circ(w[1], rd), 0.7));
                next += sp;
            }
        }
        let ok: Vec<Item> = beads
            .into_iter()
            .filter(|b| in_wedge(std::slice::from_ref(b)))
            .collect();
        put(&mut gn, &ok, &all);
        let rv = reach_at(-PI / 2.0, ib);
        if rv > rd * 3.0 {
            put(&mut gn, &[sh(circ(pt(0.0, -rv), rd * 1.2), 0.75)], &turns);
        }
    }
    let ai = reach_at(phi_m, ii);
    let rci = reach_at(-PI / 2.0, ii);
    // the centre rosette: long petals toward the corners, short ones toward the sides
    let r_ros = ai * gn.r(0.2, 0.26);
    let rc = g.max(r_ros * 0.28);
    if !border_only {
        put(&mut gn, &[sh(circ(Point::ZERO, rc), 1.1)], &[(0.0, 1.0)]);
        let cap = |r: f64| PI / (2.0 * nf) - g / (2.0 * r);
        let long = sh(
            polar(
                Point::ZERO,
                -PI / 2.0,
                rc + g,
                r_ros,
                |_| 0.0,
                |t, r| (r_ros * 0.22 * (PI * t).sin() / r).atan().min(cap(r)),
            ),
            0.95,
        );
        let short = sh(
            polar(
                Point::ZERO,
                phi_m,
                rc + g,
                rc + g + (r_ros - rc - g) * 0.7,
                |_| 0.0,
                |t, r| (r_ros * 0.17 * (PI * t).sin() / r).atan().min(cap(r)),
            ),
            0.8,
        );
        put(&mut gn, &[long], &turns);
        put(&mut gn, &[short], &turns);
        let rdd = (g * 0.5).max(r_ros * 0.06);
        let rr2 = r_ros + g + rdd;
        let per = 2.0 * nf;
        let m2 = ((TAU * rr2 / (rdd * 2.0 + g * 1.4) / per).round() * per) as usize;
        for k in 0..m2 {
            let ang = -PI / 2.0 + TAU * (k as f64 + 0.5) / m2 as f64;
            gn.add(to_board(&[sh(circ(dir(ang) * rr2, rdd), 0.7)]), false);
        }
    }
    // the corner's leaf fan and the stem that feeds it
    let ws = (g * 1.4).max(ai * 0.06);
    let rcal = (g * 0.9).max(ai * 0.035);
    let lp = ai * gn.r(0.26, 0.34);
    let rp = rci - lp - g * 1.5;
    let r_start = r_ros + g * 1.2 + (g * 0.5).max(r_ros * 0.06) * 2.0 + g;
    let stem = !border_only && rp - rcal - g - r_start > 3.0 * g;
    if stem {
        let st = trace(
            pt(0.0, -r_start),
            -PI / 2.0,
            rp - rcal - g - r_start,
            |_, _| 0.0,
            0.4,
        );
        let k = st.p.len();
        let sw: Vec<f64> = (0..k)
            .map(|i| ws * (1.0 - 0.25 * i as f64 / (k - 1) as f64))
            .collect();
        put(&mut gn, &[sh(band(&st, &sw, true, true), 0.9)], &turns);
    }
    put(&mut gn, &[sh(circ(pt(0.0, -rp), rcal), 1.0)], &turns);
    {
        let sig = gn.r(0.38, 0.5);
        let inside_i = |q: Point| field.sample(fr.to_board(q)) > ii;
        let mut leaves = vec![];
        for j in 0..=3 {
            let psi = f64::from(j) * sig;
            let d = -PI / 2.0 + psi;
            let mut l = 0.0;
            while l < ai && inside_i(pt(0.0, -rp) + dir(d) * (rcal + l)) {
                l += 0.5;
            }
            l = (l - g * 1.2) * if j == 0 { 1.0 } else { 0.92 };
            if l < 3.0 * g {
                break;
            }
            let bend = move |t: f64| if j == 0 { 0.0 } else { 0.22 * t * t };
            leaves.push(sh(
                polar(pt(0.0, -rp), d, rcal + g, rcal + l, bend, |t, r| {
                    (ai * 0.06 * (PI * t.powf(0.8)).sin().powf(0.8) / r)
                        .atan()
                        .min(sig / 2.0 - g / (2.0 * r))
                }),
                0.85,
            ));
        }
        if !leaves.is_empty() {
            put(&mut gn, &leaves[..1], &turns);
            let side: Vec<Item> = leaves[1..]
                .iter()
                .filter(|l| in_wedge(std::slice::from_ref(l)))
                .cloned()
                .collect();
            put(&mut gn, &side, &all);
        }
    }
    // a half sunburst against each side
    {
        let rf = (ai * 0.2).min(ai * (PI / nf).tan() * 0.32);
        let ctr = u * (ai - g * 0.3);
        let ang = u.y.atan2(u.x);
        let mut it = vec![];
        sunburst(
            &mut it,
            ctr,
            rf,
            g,
            ai * 0.5,
            ang + PI / 2.0,
            ang + 3.0 * PI / 2.0,
            0.85,
        );
        let fit: Vec<Item> = it
            .into_iter()
            .filter(|x| gn.occ.fits(&to_board(std::slice::from_ref(x))))
            .collect();
        put(&mut gn, &fit, &turns);
    }
    // the main scrolls: searched, off the stem or anywhere in the wedge
    let passes: &[bool] = if den > 0.7 {
        &[false, true, true]
    } else if den < 0.3 {
        &[false]
    } else {
        &[false, true]
    };
    for &second in passes {
        let mut best: Option<(f64, Vec<Item>)> = None;
        for t in 0..if second { 400 } else { 900 } {
            let off_stem = t % 2 == 1;
            if border_only && off_stem {
                continue;
            }
            let r = gn.r(r_start + ws, rp - rcal);
            let x0 = ws / 2.0 + g * 1.7;
            let h0 = -PI / 2.0 + gn.r(0.2, 1.3);
            let flip = gn.chance(0.5);
            let smax = ai * if second { 0.6 } else { 1.15 } * sc;
            let size = gn.r(0.3, 1.0) * smax;
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
            let koru = koru_items(&cp, size, &gn.ctx, true);
            let items = if off_stem {
                move_items(&koru, pt(x0, -r), h0, flip)
            } else {
                let rr0 = gn.r(r_ros, rci);
                let aa = -PI / 2.0 + gn.r(0.0, PI / nf);
                let rot = gn.r(0.0, TAU);
                place(&koru, dir(aa) * rr0, rot, flip)
            };
            if !in_wedge(&items) || !gn.occ.fits(&to_board(&items)) {
                continue;
            }
            let a = items_area(&items) * (1.0 + 0.3 * gn.rng().f());
            if best.as_ref().is_none_or(|b| a > b.0) {
                best = Some((a, items));
            }
        }
        if let Some((_, items)) = best {
            put(&mut gn, &items, &all);
        }
    }
    // more scrolls, smaller, wherever they fit
    for _ in 0..(420.0 * den).round() as usize {
        let size = ai * gn.r(0.18, 0.42) * sc;
        let spec = gn.curl(
            size,
            (0.22, 0.34),
            (0.1, 0.15),
            (0.3, 0.9),
            0.7,
            (g * 1.4).max(size * 0.15),
            (0.9, 1.5),
            (0.22, 0.4),
        );
        let Some(cp) = curl_path(&spec) else { continue };
        let rr0 = gn.r(r_ros, rci);
        let aa = -PI / 2.0 + gn.r(0.0, PI / nf);
        let rot = gn.r(0.0, TAU);
        let flip = gn.chance(0.5);
        let items = move_items(
            &koru_items(&cp, size, &gn.ctx, size > ai * 0.28),
            dir(aa) * rr0,
            rot,
            flip,
        );
        try_wedge(&mut gn, Some(items));
    }
    if stem {
        let l0 = rp - rcal - g - r_start;
        for f in [0.3, 0.62] {
            let y = -(r_start + l0 * f);
            let l = (ai * 0.2).min(l0 * 0.3);
            if l < 4.0 * g {
                continue;
            }
            let leaf = willow_leaf(
                pt(ws / 2.0 + g * 1.6, y),
                -PI / 2.0 + 0.75,
                l,
                l * 0.32,
                -0.7 / l,
                g,
                l * 0.32 > g * 2.4,
            );
            try_wedge(&mut gn, Some(leaf));
        }
    }
    // small pieces, sized from the room left
    let levels: [Level; 4] = [
        (
            0.24,
            160.0,
            &[(flower_ring, 1.0), (spiral_s, 2.0), (fan_burst, 1.0)],
            0.0,
        ),
        (0.15, 200.0, &[(spiral_s, 2.0), (flower_ring, 1.0)], 0.35),
        (0.1, 120.0, &[(spiral_s, 1.0), (dot_trio, 0.5)], 0.65),
        (0.05, 260.0, &[(dot, 1.0)], 0.88),
    ];
    for (f, tries, makers, dmin) in levels {
        if det < dmin {
            continue;
        }
        for _ in 0..(tries * den * 1.6).round() as usize {
            let size = ai * f * gn.r(0.85, 1.15);
            let make = gn.rng().pick(makers);
            let Some(loc) = make(size, &mut gn.ctx) else {
                continue;
            };
            let rr0 = gn.r(0.0, rci);
            let aa = -PI / 2.0 + gn.r(0.0, PI / nf);
            let rot = gn.r(0.0, TAU);
            let flip = gn.chance(0.5);
            try_wedge(&mut gn, Some(place(&loc, dir(aa) * rr0, rot, flip)));
        }
    }
    gn.stipple();
    gn.out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn polygon(n: u32, r: f64, c: Point, start: f64) -> Vec<Point> {
        (0..n)
            .map(|k| c + dir(start + TAU * f64::from(k) / f64::from(n)) * r)
            .collect()
    }

    #[test]
    fn regular_polygons_are_found_with_their_corners() {
        for n in [3, 4, 5, 6, 8] {
            let ring = polygon(n, 50.0, pt(60.0, 60.0), 0.3);
            let field = Field::new(vec![ring], 120.0, 120.0);
            let sym = detect(&field).unwrap_or_else(|| panic!("{n} sides"));
            assert_eq!(sym.n, n);
            assert!(sym.centre.distance(pt(60.0, 60.0)) < 1.0);
            // the corner found is one of the polygon's
            let off = (sym.corner - 0.3).rem_euclid(TAU / f64::from(n));
            assert!(off < 0.03 || off > TAU / f64::from(n) - 0.03, "{n}: {off}");
        }
    }

    #[test]
    fn a_circle_or_a_lopsided_shape_has_none() {
        let circle = polygon(200, 50.0, pt(60.0, 60.0), 0.0);
        assert!(detect(&Field::new(vec![circle], 120.0, 120.0)).is_none());
        let lopsided = vec![pt(10.0, 10.0), pt(110.0, 10.0), pt(30.0, 90.0)];
        assert!(detect(&Field::new(vec![lopsided], 120.0, 100.0)).is_none());
    }
}
