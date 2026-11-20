//! The material as a fine grid of solid cells in the part's frame, cut by the bit moving
//! exactly as a file moves it.

use crate::cutter::Cutter;
use crate::frame::{Pose, to_machine, to_part};
use crate::stock::Stock;
use cut_craft_core::glam::DVec3;
use std::collections::HashMap;

/// The material, cell by cell: each cell a cube `cell` across, solid or cut away. A cell
/// is solid while its middle is in the material and no bit has reached its middle.
#[derive(Clone, Debug, PartialEq)]
pub struct Voxels {
    /// The corner of the first cell, in the part's frame.
    origin: DVec3,
    cell: f64,
    /// Cells along, across (`y`) and up (`z`).
    dims: [usize; 3],
    /// Words of 64 cells up each column (`z`), column by column, `y` within along.
    words: usize,
    bits: Vec<u64>,
    /// How far out from the axis any of it reaches.
    swing: f64,
}

impl Voxels {
    /// `stock` in cells `cell` across, with an empty cell all round.
    #[must_use]
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "cell counts are small and positive"
    )]
    pub fn of(stock: &Stock, cell: f64) -> Self {
        let (lo, hi) = stock.bounds();
        let origin = lo - DVec3::splat(cell);
        let size = hi - origin + DVec3::splat(cell);
        let dims = [
            (size.x / cell).ceil() as usize + 1,
            (size.y / cell).ceil() as usize + 1,
            (size.z / cell).ceil() as usize + 1,
        ];
        let words = dims[2].div_ceil(64);
        let mut v = Self {
            origin,
            cell,
            dims,
            words,
            bits: vec![0; dims[0] * dims[1] * words],
            swing: stock.swing(),
        };
        for i in 0..dims[0] {
            for j in 0..dims[1] {
                for k in 0..dims[2] {
                    if stock.holds(v.centre(i, j, k)) {
                        v.set(i, j, k, true);
                    }
                }
            }
        }
        v
    }

    #[must_use]
    pub fn cell(&self) -> f64 {
        self.cell
    }

    #[must_use]
    pub fn swing(&self) -> f64 {
        self.swing
    }

    #[expect(clippy::cast_precision_loss, reason = "cell indices are small")]
    fn centre(&self, i: usize, j: usize, k: usize) -> DVec3 {
        self.origin + (DVec3::new(i as f64, j as f64, k as f64) + 0.5) * self.cell
    }

    fn word(&self, i: usize, j: usize, w: usize) -> usize {
        (i * self.dims[1] + j) * self.words + w
    }

    fn get(&self, i: usize, j: usize, k: usize) -> bool {
        self.bits[self.word(i, j, k / 64)] >> (k % 64) & 1 == 1
    }

    fn set(&mut self, i: usize, j: usize, k: usize, solid: bool) {
        let w = self.word(i, j, k / 64);
        if solid {
            self.bits[w] |= 1 << (k % 64);
        } else {
            self.bits[w] &= !(1 << (k % 64));
        }
    }

    /// The cell `p` (in the part's frame) is in, if it is in the grid.
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "checked to be in the grid first"
    )]
    fn index(&self, p: DVec3) -> Option<[usize; 3]> {
        let q = (p - self.origin) / self.cell;
        #[expect(clippy::cast_precision_loss, reason = "cell counts are small")]
        let inside = |v: f64, n: usize| v >= 0.0 && v < n as f64;
        let all =
            inside(q.x, self.dims[0]) && inside(q.y, self.dims[1]) && inside(q.z, self.dims[2]);
        all.then_some([q.x as usize, q.y as usize, q.z as usize])
    }

    /// Whether the material is still solid at `p` (in the part's frame).
    #[must_use]
    pub fn solid(&self, p: DVec3) -> bool {
        self.index(p).is_some_and(|[i, j, k]| self.get(i, j, k))
    }

    /// How many cells are solid.
    #[must_use]
    pub fn count(&self) -> usize {
        self.bits.iter().map(|w| w.count_ones() as usize).sum()
    }

    /// How much material is left, mm³.
    #[must_use]
    #[expect(clippy::cast_precision_loss, reason = "cell counts are small")]
    pub fn volume(&self) -> f64 {
        self.count() as f64 * self.cell.powi(3)
    }

    /// Cuts away what `cutter` reaches moving in a straight move `from` → `to`: every axis
    /// moved in step, A included, as a controller moves them. Checked at steps no further
    /// apart than half a cell anywhere on the material ([`Self::samples`]). How many cells
    /// it cut.
    pub fn cut(&mut self, cutter: &Cutter, from: Pose, to: Pose) -> usize {
        let steps = self.samples(from, to);
        self.cut_samples(cutter, from, to, 0..=steps)
    }

    /// How many steps [`Self::cut`] checks a move `from` → `to` in: no further apart than
    /// half a cell anywhere on the material, A's turn measured at the material's swing.
    #[must_use]
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "step counts are small and positive"
    )]
    pub fn samples(&self, from: Pose, to: Pose) -> usize {
        let far = (to.along - from.along)
            .abs()
            .max((to.across - from.across).abs())
            .max((to.height - from.height).abs())
            .max((to.turn - from.turn).abs() * self.swing);
        ((far / (self.cell / 2.0)).ceil() as usize).max(1)
    }

    /// Cuts the move `from` → `to` at those of its [`Self::samples`] steps in `which` (0 is
    /// `from`): a move cut a stretch at a time cuts just what it cuts all at once.
    #[expect(clippy::cast_precision_loss, reason = "step counts are small")]
    pub fn cut_samples(
        &mut self,
        cutter: &Cutter,
        from: Pose,
        to: Pose,
        which: std::ops::RangeInclusive<usize>,
    ) -> usize {
        let steps = self.samples(from, to);
        let (first, last) = (*which.start(), (*which.end()).min(steps));
        (first..=last)
            .map(|s| self.cut_at(cutter, from.lerp(to, s as f64 / steps as f64)))
            .sum()
    }

    /// The cell size that keeps `stock`'s grid to about `most` cells.
    #[must_use]
    #[expect(clippy::cast_precision_loss, reason = "a rough cell count")]
    pub fn cell_for(stock: &Stock, most: usize) -> f64 {
        let (lo, hi) = stock.bounds();
        let size = hi - lo;
        (size.x * size.y * size.z / most.max(1) as f64)
            .cbrt()
            .max(0.01)
    }

    /// Cuts away what `cutter` holds standing at `pose`. How many cells it cut.
    pub fn cut_at(&mut self, cutter: &Cutter, pose: Pose) -> usize {
        let mut cut = vec![];
        self.each_held(cutter, pose, |c| {
            cut.push(c);
            true
        });
        for &[i, j, k] in &cut {
            self.set(i, j, k, false);
        }
        cut.len()
    }

    /// Calls `f` with the middle of every solid cell, in the part's frame.
    pub fn each_solid(&self, mut f: impl FnMut(DVec3)) {
        for i in 0..self.dims[0] {
            for j in 0..self.dims[1] {
                for w in 0..self.words {
                    let mut bits = self.bits[self.word(i, j, w)];
                    while bits != 0 {
                        let b = bits.trailing_zeros() as usize;
                        bits &= bits - 1;
                        f(self.centre(i, j, w * 64 + b));
                    }
                }
            }
        }
    }

    /// The corners of the box the cells fill, in the part's frame.
    #[must_use]
    #[expect(clippy::cast_precision_loss, reason = "cell counts are small")]
    pub fn bounds(&self) -> (DVec3, DVec3) {
        let size = DVec3::new(
            self.dims[0] as f64,
            self.dims[1] as f64,
            self.dims[2] as f64,
        ) * self.cell;
        (self.origin, self.origin + size)
    }

    /// How many solid cells `cutter` would cut standing at `pose`, counting no further than
    /// `enough`: whether the bit there cuts anything at all, or only air.
    #[must_use]
    pub fn held(&self, cutter: &Cutter, pose: Pose, enough: usize) -> usize {
        let mut n = 0;
        self.each_held(cutter, pose, |_| {
            n += 1;
            n < enough
        });
        n
    }

    /// Calls `f` with each solid cell `cutter` holds standing at `pose` while it says to
    /// carry on.
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "clamped to the grid first"
    )]
    fn each_held(&self, cutter: &Cutter, pose: Pose, mut f: impl FnMut([usize; 3]) -> bool) {
        let reach = cutter.reach();
        let top = (pose.height + cutter.length()).min(self.swing + self.cell);
        if pose.height > top {
            return;
        }
        // the bit's box, in the part's frame
        let (mut lo, mut hi) = (DVec3::splat(f64::INFINITY), DVec3::splat(f64::NEG_INFINITY));
        for across in [pose.across - reach, pose.across + reach] {
            for height in [pose.height, top] {
                let p = to_part(DVec3::new(pose.along, across, height), pose.turn);
                lo = lo.min(p);
                hi = hi.max(p);
            }
        }
        lo.x = pose.along - reach;
        hi.x = pose.along + reach;
        let range = |a: f64, b: f64, axis: usize| {
            let o = self.origin[axis];
            let first = ((a - o) / self.cell - 0.5).floor().max(0.0) as usize;
            let last =
                (((b - o) / self.cell - 0.5).ceil().max(0.0) as usize).min(self.dims[axis] - 1);
            first..=last
        };
        let (ri, rj, rk) = (
            range(lo.x, hi.x, 0),
            range(lo.y, hi.y, 1),
            range(lo.z, hi.z, 2),
        );
        for i in ri {
            for j in rj.clone() {
                for k in rk.clone() {
                    if !self.get(i, j, k) {
                        continue;
                    }
                    let m = to_machine(self.centre(i, j, k), pose.turn);
                    let r = (m.x - pose.along).hypot(m.y - pose.across);
                    if cutter.holds(r, m.z - pose.height) && !f([i, j, k]) {
                        return;
                    }
                }
            }
        }
    }

    /// The material's surface, as points and triangles in the part's frame, facing out: a
    /// point in each corner between eight cells where some are solid and some not, joined
    /// across every face between a solid cell and an empty one.
    #[must_use]
    pub fn surface(&self) -> (Vec<DVec3>, Vec<[u32; 3]>) {
        let mut points = vec![];
        let mut at: HashMap<[usize; 3], u32> = HashMap::new();
        let mut triangles = vec![];
        let [nx, ny, nz] = self.dims;
        let mut corner = |c: [usize; 3], points: &mut Vec<DVec3>| -> u32 {
            *at.entry(c).or_insert_with(|| {
                points.push(self.corner_point(c));
                u32::try_from(points.len() - 1).unwrap_or(u32::MAX)
            })
        };
        let mut quad = |cells: [[usize; 3]; 4], out: bool, points: &mut Vec<DVec3>| {
            let v = cells.map(|c| corner(c, points));
            let v = if out { v } else { [v[3], v[2], v[1], v[0]] };
            triangles.push([v[0], v[1], v[2]]);
            triangles.push([v[0], v[2], v[3]]);
        };
        // the grid has an empty cell all round, so every face is between inner cells
        for i in 0..nx - 1 {
            for j in 0..ny - 1 {
                for k in 0..nz - 1 {
                    let here = self.get(i, j, k);
                    if j > 0 && k > 0 && here != self.get(i + 1, j, k) {
                        let cells = [[i, j - 1, k - 1], [i, j, k - 1], [i, j, k], [i, j - 1, k]];
                        quad(cells, here, &mut points);
                    }
                    if i > 0 && k > 0 && here != self.get(i, j + 1, k) {
                        let cells = [[i - 1, j, k - 1], [i - 1, j, k], [i, j, k], [i, j, k - 1]];
                        quad(cells, here, &mut points);
                    }
                    if i > 0 && j > 0 && here != self.get(i, j, k + 1) {
                        let cells = [[i - 1, j - 1, k], [i, j - 1, k], [i, j, k], [i - 1, j, k]];
                        quad(cells, here, &mut points);
                    }
                }
            }
        }
        (points, triangles)
    }

    /// The point for the corner between cells `c` and `c + 1` each way: the middle of the
    /// edges there between solid and empty.
    fn corner_point(&self, corner: [usize; 3]) -> DVec3 {
        let [i, j, k] = corner;
        let mut sum = DVec3::ZERO;
        let mut crossings = 0.0;
        let solid = |d: [usize; 3]| self.get(i + d[0], j + d[1], k + d[2]);
        let centre = |d: [usize; 3]| self.centre(i + d[0], j + d[1], k + d[2]);
        let corners: [[usize; 3]; 8] = [
            [0, 0, 0],
            [1, 0, 0],
            [0, 1, 0],
            [1, 1, 0],
            [0, 0, 1],
            [1, 0, 1],
            [0, 1, 1],
            [1, 1, 1],
        ];
        for a in corners {
            for b in corners {
                let apart: usize = (0..3).map(|x| a[x].abs_diff(b[x])).sum();
                if apart == 1 && a < b && solid(a) != solid(b) {
                    sum += (centre(a) + centre(b)) / 2.0;
                    crossings += 1.0;
                }
            }
        }
        if crossings > 0.0 {
            sum / crossings
        } else {
            centre([0, 0, 0]) + self.cell / 2.0
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::stock::Section;

    #[test]
    fn a_grid_of_a_round_blank_holds_its_volume() {
        let stock = Stock::new(Section::Round { diameter: 20.0 }, 30.0);
        let v = Voxels::of(&stock, 0.25);
        let want = stock.volume();
        assert!(
            (v.volume() - want).abs() / want < 0.01,
            "{} vs {want}",
            v.volume()
        );
    }

    #[test]
    fn a_bit_knows_whether_it_would_cut_anything_before_it_cuts() {
        use cut_craft_core::toolpath::{Tool, ToolShape};
        let stock = Stock::new(Section::Round { diameter: 20.0 }, 30.0);
        let mut v = Voxels::of(&stock, 0.25);
        let ball = Cutter::new(&Tool {
            diameter: 6.0,
            shape: ToolShape::BallNose,
            ..Tool::default()
        });
        let into = Pose::new(15.0, 0.0, 8.0, 0.0);
        let above = Pose::new(15.0, 0.0, 10.5, 0.0);
        assert_eq!(v.held(&ball, above, 1), 0, "clear of the material");
        assert_eq!(v.held(&ball, into, 5), 5, "counts no further than asked");
        let all = v.held(&ball, into, usize::MAX);
        assert!(all > 5);
        assert_eq!(
            v.cut_at(&ball, into),
            all,
            "what it would cut is what it cuts"
        );
        assert_eq!(
            v.held(&ball, into, usize::MAX),
            0,
            "and then there is nothing left there"
        );
    }
}
