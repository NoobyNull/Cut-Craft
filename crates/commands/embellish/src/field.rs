//! The shape being embellished, read as a distance field: how far each point is from the
//! outline. Insets of the outline (where the border band and the bead row run), the room
//! there is for a piece, and the shape's middle all come from it.

use cut_craft_core::geometry::Point;
use std::collections::HashMap;

const INF: f64 = 1e20;

/// The squared distance transform of one row (Felzenszwalb and Huttenlocher).
fn edt1(f: &[f64], d: &mut [f64], v: &mut [usize], z: &mut [f64]) {
    let n = f.len();
    let sq = |q: usize| {
        let q = q as f64;
        q * q
    };
    let fl = |q: usize| q as f64;
    let mut k = 0;
    v[0] = 0;
    z[0] = -INF;
    z[1] = INF;
    for q in 1..n {
        let mut s;
        loop {
            s = ((f[q] + sq(q)) - (f[v[k]] + sq(v[k]))) / (2.0 * fl(q) - 2.0 * fl(v[k]));
            if s <= z[k] && k > 0 {
                k -= 1;
            } else {
                break;
            }
        }
        if s <= z[k] {
            // k is 0 and the new parabola is lower everywhere
            v[0] = q;
            z[0] = -INF;
            z[1] = INF;
            continue;
        }
        k += 1;
        v[k] = q;
        z[k] = s;
        z[k + 1] = INF;
    }
    k = 0;
    for (q, out) in d.iter_mut().enumerate().take(n) {
        while z[k + 1] < fl(q) {
            k += 1;
        }
        let dq = fl(q) - fl(v[k]);
        *out = dq * dq + f[v[k]];
    }
}

/// Squared distances, in cells, from each cell to the nearest cell holding 0.
pub fn edt2(f: &mut [f64], w: usize, h: usize) {
    let n = w.max(h);
    let (mut g, mut d, mut v, mut z) = (vec![0.0; n], vec![0.0; n], vec![0; n], vec![0.0; n + 1]);
    for x in 0..w {
        for y in 0..h {
            g[y] = f[y * w + x];
        }
        edt1(&g[..h], &mut d[..h], &mut v, &mut z);
        for y in 0..h {
            f[y * w + x] = d[y];
        }
    }
    for y in 0..h {
        g[..w].copy_from_slice(&f[y * w..(y + 1) * w]);
        edt1(&g[..w], &mut d[..w], &mut v, &mut z);
        f[y * w..(y + 1) * w].copy_from_slice(&d[..w]);
    }
}

/// Fills `f` with `value` inside `rings` (even-odd), the grid's points `res` apart from 0.
pub fn fill_rings(f: &mut [f64], nx: usize, ny: usize, res: f64, rings: &[Vec<Point>], value: f64) {
    let mut xs = vec![];
    for j in 0..ny {
        let y = j as f64 * res;
        xs.clear();
        for p in rings {
            let n = p.len();
            for a in 0..n {
                let b = (a + n - 1) % n;
                if (p[a].y > y) != (p[b].y > y) {
                    xs.push(p[a].x + (y - p[a].y) * (p[b].x - p[a].x) / (p[b].y - p[a].y));
                }
            }
        }
        xs.sort_by(f64::total_cmp);
        for k in (0..xs.len().saturating_sub(1)).step_by(2) {
            let i0 = (xs[k] / res).ceil().max(0.0) as usize;
            let i1 = (xs[k + 1] / res).floor();
            if i1 < 0.0 {
                continue;
            }
            for i in i0..=(i1 as usize).min(nx - 1) {
                f[j * nx + i] = value;
            }
        }
    }
}

/// The closed lines where `f` crosses `level` (marching squares), each through `to_xy` of
/// its grid position.
pub fn contour_loops(
    f: &[f32],
    nx: usize,
    ny: usize,
    level: f32,
    to_xy: impl Fn(f64, f64) -> Point,
) -> Vec<Vec<Point>> {
    let key = |i: usize, j: usize, d: usize| (j * nx + i) * 2 + d;
    let mut segs: Vec<[usize; 2]> = vec![];
    for j in 0..ny.saturating_sub(1) {
        for i in 0..nx.saturating_sub(1) {
            let at = |i: usize, j: usize| f[j * nx + i];
            let (a, b, c, d) = (
                at(i, j) > level,
                at(i + 1, j) > level,
                at(i + 1, j + 1) > level,
                at(i, j + 1) > level,
            );
            let mut e = vec![];
            if a != b {
                e.push(key(i, j, 0));
            }
            if b != c {
                e.push(key(i + 1, j, 1));
            }
            if d != c {
                e.push(key(i, j + 1, 0));
            }
            if a != d {
                e.push(key(i, j, 1));
            }
            if e.len() == 2 {
                segs.push([e[0], e[1]]);
            } else if e.len() == 4 {
                let centre =
                    (at(i, j) + at(i + 1, j) + at(i, j + 1) + at(i + 1, j + 1)) / 4.0 > level;
                if centre == a {
                    segs.push([e[0], e[1]]);
                    segs.push([e[2], e[3]]);
                } else {
                    segs.push([e[0], e[3]]);
                    segs.push([e[1], e[2]]);
                }
            }
        }
    }
    let mut ends: HashMap<usize, Vec<usize>> = HashMap::new();
    for (id, s) in segs.iter().enumerate() {
        for &k in s {
            ends.entry(k).or_default().push(id);
        }
    }
    let pos = |k: usize| {
        let (d, cell) = (k & 1, k >> 1);
        let (i, j) = (cell % nx, cell / nx);
        let a = f[j * nx + i];
        let b = if d == 0 {
            f[j * nx + i + 1]
        } else {
            f[(j + 1) * nx + i]
        };
        let t = f64::from((level - a) / (b - a));
        let (fi, fj) = (i as f64, j as f64);
        if d == 0 {
            to_xy(fi + t, fj)
        } else {
            to_xy(fi, fj + t)
        }
    };
    let mut used = vec![false; segs.len()];
    let mut loops = vec![];
    for s0 in 0..segs.len() {
        if used[s0] {
            continue;
        }
        used[s0] = true;
        let mut chain = vec![segs[s0][0], segs[s0][1]];
        loop {
            let k = chain[chain.len() - 1];
            let Some(&nb) = ends.get(&k).and_then(|l| l.iter().find(|&&id| !used[id])) else {
                break;
            };
            used[nb] = true;
            chain.push(if segs[nb][0] == k {
                segs[nb][1]
            } else {
                segs[nb][0]
            });
        }
        if chain.len() > 8 {
            loops.push(chain.into_iter().map(pos).collect());
        }
    }
    loops
}

pub fn loop_len(p: &[Point]) -> f64 {
    let n = p.len();
    (0..n).map(|i| p[i].distance(p[(i + 1) % n])).sum()
}

/// Points every `step` round a closed ring.
pub fn resample_loop(p: &[Point], step: f64) -> Vec<Point> {
    let n = p.len();
    let mut out = vec![];
    let (mut acc, mut next) = (0.0, 0.0);
    for i in 0..n {
        let (a, b) = (p[i], p[(i + 1) % n]);
        let d = a.distance(b);
        while next <= acc + d {
            let t = if d > 0.0 { (next - acc) / d } else { 0.0 };
            out.push(a.lerp(b, t));
            next += step;
        }
        acc += d;
    }
    out
}

/// Points every `step` along an open line.
pub fn resample_open(p: &[Point], step: f64) -> Vec<Point> {
    let mut out = vec![p[0]];
    let (mut acc, mut next) = (0.0, step);
    for w in p.windows(2) {
        let d = w[0].distance(w[1]);
        while next <= acc + d {
            out.push(w[0].lerp(w[1], (next - acc) / d));
            next += step;
        }
        acc += d;
    }
    out
}

/// The shape: its outline rings and distance field, in its own frame (its box from 0, with
/// a margin round it).
pub struct Field {
    pub w: f64,
    pub h: f64,
    pub outline: Vec<Vec<Point>>,
    pub d: Vec<f32>,
    pub nx: usize,
    pub ny: usize,
    pub res: f64,
    pub dmax: f64,
}

/// The most cells the field may have; a bigger shape gets coarser cells.
const MOST_CELLS: f64 = 2_500_000.0;

impl Field {
    /// The field of `rings` (even-odd), which lie within `w` by `h` from 0.
    pub fn new(rings: Vec<Vec<Point>>, w: f64, h: f64) -> Self {
        let res = 0.5_f64.max((w * h / MOST_CELLS).sqrt());
        let (nx, ny) = ((w / res).ceil() as usize + 1, (h / res).ceil() as usize + 1);
        let mut f = vec![0.0; nx * ny];
        fill_rings(&mut f, nx, ny, res, &rings, INF);
        edt2(&mut f, nx, ny);
        let d: Vec<f32> = f.iter().map(|v| (v.sqrt() * res) as f32).collect();
        let dmax = f64::from(d.iter().copied().fold(0.0, f32::max));
        Self {
            w,
            h,
            outline: rings,
            d,
            nx,
            ny,
            res,
            dmax,
        }
    }

    /// The distance to the outline at `p`, 0 outside.
    pub fn sample(&self, p: Point) -> f64 {
        let (u, v) = (p.x / self.res, p.y / self.res);
        let (fi, fj) = (u.floor(), v.floor());
        if fi < 0.0 || fj < 0.0 || fi >= (self.nx - 1) as f64 || fj >= (self.ny - 1) as f64 {
            return 0.0;
        }
        let (i, j) = (fi as usize, fj as usize);
        let (fx, fy) = (u - fi, v - fj);
        let at = |i: usize, j: usize| f64::from(self.d[j * self.nx + i]);
        let (a, b, c, d) = (at(i, j), at(i + 1, j), at(i, j + 1), at(i + 1, j + 1));
        a + (b - a) * fx + (c - a) * fy + (a - b - c + d) * fx * fy
    }

    pub fn grad(&self, p: Point) -> Point {
        let e = 0.6;
        Point::new(
            (self.sample(p + Point::X * e) - self.sample(p - Point::X * e)) / (2.0 * e),
            (self.sample(p + Point::Y * e) - self.sample(p - Point::Y * e)) / (2.0 * e),
        )
    }

    /// The outline moved in by `inset`, smoothed: one ring per piece it falls into.
    pub fn rings(&self, inset: f64) -> Vec<Vec<Point>> {
        let res = self.res;
        contour_loops(&self.d, self.nx, self.ny, inset as f32, |i, j| {
            Point::new(i * res, j * res)
        })
        .into_iter()
        .filter(|l| loop_len(l) > 4.0 * inset + 6.0)
        .map(|l| crate::motif::chaikin(&resample_loop(&l, 1.2), 1))
        .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn square(at: f64, side: f64) -> Vec<Point> {
        vec![
            Point::new(at, at),
            Point::new(at + side, at),
            Point::new(at + side, at + side),
            Point::new(at, at + side),
        ]
    }

    #[test]
    fn a_squares_field_is_its_distance_to_the_edge() {
        let f = Field::new(vec![square(5.0, 100.0)], 110.0, 110.0);
        assert!((f.dmax - 50.0).abs() < 1.0, "{}", f.dmax);
        assert!((f.sample(Point::new(15.0, 55.0)) - 10.0).abs() < 0.8);
        assert_eq!(f.sample(Point::new(2.0, 2.0)), 0.0);
        let inset = f.rings(10.0);
        assert_eq!(inset.len(), 1);
        for p in &inset[0] {
            assert!((f.sample(*p) - 10.0).abs() < 0.8, "{p:?}");
        }
    }

    #[test]
    fn a_hole_is_outside() {
        let f = Field::new(vec![square(5.0, 100.0), square(45.0, 20.0)], 110.0, 110.0);
        assert_eq!(f.sample(Point::new(55.0, 55.0)), 0.0);
        assert!(f.sample(Point::new(25.0, 55.0)) > 15.0);
        assert_eq!(
            f.rings(5.0).len(),
            2,
            "round the outside and round the hole"
        );
    }
}
