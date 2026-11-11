//! The occupancy grid: which cells pieces already cover, each grown by the smallest gap, so
//! a new piece is kept only where it clears every other piece and the shape's edge.

use crate::motif::Item;
use cut_craft_core::geometry::Point;
use std::collections::HashMap;

pub struct Occ {
    c: f64,
    g: f64,
    nx: usize,
    ny: usize,
    taken: Vec<bool>,
    inside: Vec<bool>,
    discs: HashMap<i64, Vec<(isize, isize)>>,
}

impl Occ {
    /// A grid over `w` by `h`, a cell inside where `inside(p, g)` (clear of the edge by the
    /// gap).
    pub fn new(w: f64, h: f64, g: f64, inside: impl Fn(Point, f64) -> bool) -> Self {
        let mut c = (g * 0.5).clamp(0.35, 1.5);
        while (w / c) * (h / c) > 700_000.0 {
            c *= 1.2;
        }
        let (nx, ny) = ((w / c).ceil() as usize + 1, (h / c).ceil() as usize + 1);
        let mut ins = vec![false; nx * ny];
        for j in 0..ny {
            for i in 0..nx {
                ins[j * nx + i] = inside(Point::new((i as f64 + 0.5) * c, (j as f64 + 0.5) * c), g);
            }
        }
        Self {
            c,
            g,
            nx,
            ny,
            taken: vec![false; nx * ny],
            inside: ins,
            discs: HashMap::new(),
        }
    }

    fn idx(&self, i: isize, j: isize) -> Option<usize> {
        (i >= 0 && j >= 0 && (i as usize) < self.nx && (j as usize) < self.ny)
            .then(|| j as usize * self.nx + i as usize)
    }

    fn cell(&self, p: Point) -> (isize, isize) {
        (
            (p.x / self.c).floor() as isize,
            (p.y / self.c).floor() as isize,
        )
    }

    fn disc(&mut self, r: f64) -> Vec<(isize, isize)> {
        let key = (r / self.c * 4.0).round() as i64;
        let c = self.c;
        self.discs
            .entry(key)
            .or_insert_with(|| {
                let rc = r / c;
                let n = rc.ceil() as isize;
                let mut l = vec![];
                for dy in -n..=n {
                    for dx in -n..=n {
                        let d2 = (dx * dx + dy * dy) as f64;
                        if d2 <= rc * rc + 0.25 {
                            l.push((dx, dy));
                        }
                    }
                }
                l
            })
            .clone()
    }

    /// The cells inside `rings` (even-odd) and along their edges; `None` stands for a cell off
    /// the grid.
    fn cells_of(&self, rings: &[Vec<Point>], out: &mut Vec<Option<usize>>) {
        let (mut min_y, mut max_y) = (f64::INFINITY, f64::NEG_INFINITY);
        for q in rings.iter().flatten() {
            min_y = min_y.min(q.y);
            max_y = max_y.max(q.y);
        }
        if !min_y.is_finite() {
            return;
        }
        let c = self.c;
        let (j0, j1) = ((min_y / c).floor() as isize, (max_y / c).floor() as isize);
        let mut xs = vec![];
        for j in j0..=j1 {
            let y = (j as f64 + 0.5) * c;
            xs.clear();
            for p in rings {
                let n = p.len();
                for a in 0..n {
                    let b = (a + n - 1) % n;
                    let (ya, yb) = (p[a].y, p[b].y);
                    if (ya > y) != (yb > y) {
                        xs.push(p[a].x + (y - ya) * (p[b].x - p[a].x) / (yb - ya));
                    }
                }
            }
            xs.sort_by(f64::total_cmp);
            for k in (0..xs.len().saturating_sub(1)).step_by(2) {
                let (i0, i1) = (
                    (xs[k] / c - 0.5).ceil() as isize,
                    (xs[k + 1] / c - 0.5).floor() as isize,
                );
                for i in i0..=i1 {
                    out.push(self.idx(i, j));
                }
            }
        }
        for p in rings {
            let n = p.len();
            for a in 0..n {
                let (pa, pb) = (p[a], p[(a + 1) % n]);
                let steps = (pa.distance(pb) / (c * 0.5)).ceil().max(1.0) as usize;
                for t in 0..steps {
                    let q = pa.lerp(pb, t as f64 / steps as f64);
                    let (i, j) = self.cell(q);
                    out.push(self.idx(i, j));
                }
            }
        }
    }

    /// Points every `step` along an open line, its end included.
    fn samples(p: &[Point], step: f64) -> Vec<Point> {
        let mut out = vec![];
        for w in p.windows(2) {
            let n = (w[0].distance(w[1]) / step).ceil().max(1.0) as usize;
            for t in 0..n {
                out.push(w[0].lerp(w[1], t as f64 / n as f64));
            }
        }
        if let Some(&z) = p.last() {
            out.push(z);
        }
        out
    }

    fn open(&self, q: Option<usize>) -> bool {
        q.is_some_and(|q| !self.taken[q] && self.inside[q])
    }

    /// Whether every piece of `items` lies in free cells inside.
    pub fn fits(&mut self, items: &[Item]) -> bool {
        let mut tmp = vec![];
        for it in items {
            match it {
                Item::Shape { rings, .. } => {
                    tmp.clear();
                    self.cells_of(rings, &mut tmp);
                    if !tmp.iter().all(|&q| self.open(q)) {
                        return false;
                    }
                }
                Item::Line { points, width } => {
                    let d = self.disc(width / 2.0);
                    for q in Self::samples(points, self.c * 0.5) {
                        let (i, j) = self.cell(q);
                        if !d
                            .iter()
                            .all(|&(dx, dy)| self.open(self.idx(i + dx, j + dy)))
                        {
                            return false;
                        }
                    }
                }
            }
        }
        true
    }

    /// Takes the cells of `items`, grown by the gap.
    pub fn mark(&mut self, items: &[Item]) {
        let mut tmp = vec![];
        for it in items {
            let (line, r) = match it {
                Item::Shape { rings, .. } => {
                    tmp.clear();
                    self.cells_of(rings, &mut tmp);
                    for q in tmp.iter().flatten() {
                        self.taken[*q] = true;
                    }
                    let mut all = vec![];
                    for p in rings {
                        let mut closed = p.clone();
                        if let Some(&f) = p.first() {
                            closed.push(f);
                        }
                        all.extend(Self::samples(&closed, self.c));
                    }
                    (all, self.g)
                }
                Item::Line { points, width } => {
                    (Self::samples(points, self.c * 0.5), width / 2.0 + self.g)
                }
            };
            let d = self.disc(r);
            for q in line {
                let (i, j) = self.cell(q);
                for &(dx, dy) in &d {
                    if let Some(k) = self.idx(i + dx, j + dy) {
                        self.taken[k] = true;
                    }
                }
            }
        }
    }

    /// Whether a disc of radius `r` at `p` is free and inside.
    pub fn free(&mut self, p: Point, r: f64) -> bool {
        let d = self.disc(r);
        let (i, j) = self.cell(p);
        d.iter()
            .all(|&(dx, dy)| self.open(self.idx(i + dx, j + dy)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::motif::{circ, sh};

    #[test]
    fn a_piece_fits_once_and_keeps_the_gap_round_it() {
        let mut occ = Occ::new(100.0, 100.0, 2.0, |p, m| {
            p.x > m && p.y > m && p.x < 100.0 - m && p.y < 100.0 - m
        });
        let a = vec![sh(circ(Point::new(50.0, 50.0), 10.0), 1.0)];
        assert!(occ.fits(&a));
        occ.mark(&a);
        assert!(!occ.fits(&a), "taken");
        let near = vec![sh(circ(Point::new(50.0, 61.0), 0.5), 1.0)];
        assert!(!occ.fits(&near), "within the gap");
        let clear = vec![sh(circ(Point::new(50.0, 64.0), 0.5), 1.0)];
        assert!(occ.fits(&clear));
        let out = vec![sh(circ(Point::new(1.0, 50.0), 0.5), 1.0)];
        assert!(!occ.fits(&out), "too near the edge");
    }
}
