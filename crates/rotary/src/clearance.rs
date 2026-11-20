//! Where the bit's holder fits. Above the flutes come the collet nut and the spindle, much
//! wider than the bit: down in a slot or beside a wing they would hit the material long before
//! the bit reaches the bottom. The chuck holds one end of the material and the tailstock the
//! other, and nothing may run into them either.
//!
//! [`Head`] is the holder, the chuck and the tailstock; [`Clearance`] is, for A held at one
//! turn, the lowest the bit's tip may go anywhere without any of them touching what is left
//! of the material (or the chuck and tailstock themselves).

use crate::frame::{Pose, to_machine};
use crate::voxels::Voxels;
use cut_craft_core::collision::{Holding, bands};
use cut_craft_core::rotary_axis::{CHUCK_RADIUS, Surroundings, TAILSTOCK_RADIUS};
use cut_craft_core::toolpath::Tool;

/// The holder above a bit, and the chuck and tailstock holding the material.
#[derive(Clone, Debug, PartialEq)]
pub struct Head {
    /// The round parts above the tip, lowest first: how far above the tip each starts, and
    /// how far out from the bit's axis it reaches.
    bands: Vec<(f64, f64)>,
    /// How far out the bit itself reaches.
    reach: f64,
    /// How far along the axis the chuck holds the material, and how far out its body reaches;
    /// none when it is not known.
    chuck: Option<(f64, f64)>,
    /// From how far along the axis the tailstock reaches, and how far out (as the chuck, at
    /// least the material's swing: what it holds is kept whole); none without one.
    tailstock: Option<(f64, f64)>,
}

impl Head {
    /// `tool` in the holder `around` says, on material `length` long that swings `swing` out
    /// from the axis, between its chuck and tailstock.
    #[must_use]
    pub fn new(tool: &Tool, around: &Surroundings, length: f64, swing: f64) -> Self {
        let holding = Holding {
            holder: around.holder,
            per_bit: false,
        };
        let bands = bands(tool, &holding)
            .into_iter()
            .map(|b| (b.rise, b.radius))
            .collect();
        Self {
            bands,
            reach: tool.reach().max(1e-6),
            chuck: (around.chuck > 0.0).then(|| (around.chuck, CHUCK_RADIUS.max(swing))),
            tailstock: (around.tailstock > 0.0)
                .then(|| (length - around.tailstock, TAILSTOCK_RADIUS.max(swing))),
        }
    }

    /// The round parts above the tip: (how far above it each starts, how far out it reaches).
    #[must_use]
    pub fn bands(&self) -> &[(f64, f64)] {
        &self.bands
    }

    /// How far out from the bit's axis anything of the head reaches.
    #[must_use]
    pub fn widest(&self) -> f64 {
        self.bands.iter().map(|b| b.1).fold(self.reach, f64::max)
    }

    /// Whether `along` the axis is held by the chuck or the tailstock: no cutting there.
    #[must_use]
    pub fn held(&self, along: f64) -> bool {
        self.chuck.is_some_and(|(to, _)| along <= to)
            || self.tailstock.is_some_and(|(from, _)| along >= from)
    }

    /// Where along the axis the material is free of the chuck and the tailstock.
    #[must_use]
    pub fn free(&self) -> (f64, f64) {
        (
            self.chuck.map_or(f64::NEG_INFINITY, |(to, _)| to),
            self.tailstock.map_or(f64::INFINITY, |(from, _)| from),
        )
    }

    /// How high the chuck or tailstock stands at `along`, `across` from the axis (seen from
    /// above: they are round, so A makes no difference); none where neither is.
    fn fixture(&self, along: f64, across: f64) -> Option<f64> {
        let round = |r: f64| (across.abs() < r).then(|| (r * r - across * across).sqrt());
        if let Some((_, r)) = self.chuck.filter(|(to, _)| along <= *to) {
            round(r)
        } else {
            self.tailstock
                .filter(|(from, _)| along >= *from)
                .and_then(|(_, r)| round(r))
        }
    }

    /// The lowest the tip may go `along` the axis when the material left is round, `radius`
    /// out at each point along (seen from any turn of A the same): no band of the holder may
    /// come lower than the material within its reach, nor the bit or the holder into the
    /// chuck or tailstock. Looked at every `step` along, each band a step wider than it is,
    /// so nothing between two looks is missed.
    #[must_use]
    pub fn lowest_over(&self, along: f64, radius: impl Fn(f64) -> f64, step: f64) -> f64 {
        let step = step.max(1e-3);
        let mut lowest = f64::NEG_INFINITY;
        let widest = self.widest() + step;
        let mut d = -widest;
        while d <= widest + 1e-9 {
            let at = along + d;
            let r = radius(at);
            for &(rise, out) in &self.bands {
                if d.abs() <= out + step {
                    lowest = lowest.max(r - rise);
                }
            }
            if let Some(stands) = self.fixture(at, 0.0) {
                // the bit's own edge may just touch where the chuck starts, as the cut is
                // placed to; the holder above is kept a step clear
                if d.abs() < self.reach - 1e-6 {
                    lowest = lowest.max(stands);
                }
                for &(rise, out) in &self.bands {
                    if d.abs() <= out + step {
                        lowest = lowest.max(stands - rise);
                    }
                }
            }
            d += step;
        }
        lowest
    }
}

/// With A held at one turn: the lowest the bit's tip may go at each point (along, across),
/// for the holder to stay clear of what is left of the material and the bit and holder to
/// stay clear of the chuck and tailstock. Worked out on a grid; each point counts the whole
/// of the cell it is in, so the grid never lets the holder closer than it really may.
#[derive(Clone, Debug)]
pub struct Clearance {
    along: f64,
    across: f64,
    cell: f64,
    cols: usize,
    rows: usize,
    /// Row by row (across), each `cols` long (along).
    lowest: Vec<f64>,
}

impl Clearance {
    /// For A turned `turn`, the bit in `head`, over what is left of the `material`, on a
    /// grid `cell` across (no finer than the material's cells).
    #[must_use]
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        clippy::cast_precision_loss,
        reason = "grid indices are small and positive"
    )]
    pub fn of(material: &Voxels, turn: f64, head: &Head, cell: f64) -> Self {
        let cell = cell.max(material.cell());
        let widest = head.widest();
        let (lo, hi) = material.bounds();
        let swing = material.swing() + material.cell();
        let along = lo.x - widest - cell;
        let across = -swing - widest - cell;
        let cols = ((hi.x + widest + cell - along) / cell).ceil() as usize + 1;
        let rows = ((swing + widest + cell - across) / cell).ceil() as usize + 1;
        // the highest thing in each cell seen from above, the material and the fixtures apart
        let mut stuff = vec![f64::NEG_INFINITY; cols * rows];
        let mut fixed = vec![f64::NEG_INFINITY; cols * rows];
        // a cell's middle is in the material: its corners may be up to half a diagonal higher
        let corner = material.cell() * 0.87;
        let index = |a: f64, c: f64| -> Option<usize> {
            let (i, j) = ((a - along) / cell, (c - across) / cell);
            (i >= 0.0 && j >= 0.0 && (i as usize) < cols && (j as usize) < rows)
                .then(|| j as usize * cols + i as usize)
        };
        material.each_solid(|p| {
            let m = to_machine(p, turn);
            if let Some(k) = index(m.x, m.y) {
                stuff[k] = stuff[k].max(m.z + corner);
            }
        });
        for j in 0..rows {
            for i in 0..cols {
                // the whole cell: the fixture's highest anywhere in it
                let (a0, c0) = (along + i as f64 * cell, across + j as f64 * cell);
                let nearest = if c0 > 0.0 {
                    c0
                } else if c0 + cell < 0.0 {
                    c0 + cell
                } else {
                    0.0
                };
                let mut top = f64::NEG_INFINITY;
                for a in [a0, a0 + cell] {
                    if let Some(h) = head.fixture(a, nearest) {
                        top = top.max(h);
                    }
                }
                fixed[j * cols + i] = top;
            }
        }
        let both: Vec<f64> = stuff.iter().zip(&fixed).map(|(a, b)| a.max(*b)).collect();
        let mut lowest = spread(&fixed, cols, rows, head.reach / cell + 1.0);
        for &(rise, out) in &head.bands {
            let reached = spread(&both, cols, rows, out / cell + 1.0);
            for (l, r) in lowest.iter_mut().zip(reached) {
                *l = l.max(r - rise);
            }
        }
        Self {
            along,
            across,
            cell,
            cols,
            rows,
            lowest,
        }
    }

    /// The lowest the tip may go at `along`, `across`: its height above the axis (minus
    /// infinity where nothing is in the way).
    #[must_use]
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "checked to be on the grid first"
    )]
    pub fn lowest(&self, along: f64, across: f64) -> f64 {
        let (i, j) = (
            ((along - self.along) / self.cell).floor(),
            ((across - self.across) / self.cell).floor(),
        );
        if i < 0.0 || j < 0.0 {
            return f64::NEG_INFINITY;
        }
        let (i, j) = (i as usize, j as usize);
        if i >= self.cols || j >= self.rows {
            return f64::NEG_INFINITY;
        }
        self.lowest[j * self.cols + i]
    }

    /// Whether the bit can stand at `pose` (A turned as the clearance was worked out for)
    /// with nothing of the holder touching the material, and neither bit nor holder the
    /// chuck or tailstock.
    #[must_use]
    pub fn allows(&self, pose: Pose) -> bool {
        pose.height >= self.lowest(pose.along, pose.across) - 1e-9
    }
}

/// `top` (row by row, `cols` long) with each cell given the highest of every cell within
/// `radius` cells of it: a round band that far out, lowered onto it.
#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_possible_wrap,
    clippy::cast_precision_loss,
    reason = "grid indices are small"
)]
fn spread(top: &[f64], cols: usize, rows: usize, radius: f64) -> Vec<f64> {
    let reach = radius.ceil() as i64;
    let mut out = vec![f64::NEG_INFINITY; top.len()];
    let mut windowed = vec![f64::NEG_INFINITY; cols];
    for dj in -reach..=reach {
        // how far either way along the band reaches on the row dj away
        let half = (radius * radius - (dj * dj) as f64).max(0.0).sqrt().ceil() as usize;
        for j in 0..rows {
            let from = j as i64 + dj;
            if from < 0 || from >= rows as i64 {
                continue;
            }
            let row = &top[from as usize * cols..(from as usize + 1) * cols];
            sliding_max(row, half, &mut windowed);
            for (o, w) in out[j * cols..(j + 1) * cols].iter_mut().zip(&windowed) {
                *o = o.max(*w);
            }
        }
    }
    out
}

/// Each of `row` the highest within `half` either side of it, into `out`.
fn sliding_max(row: &[f64], half: usize, out: &mut [f64]) {
    let n = row.len();
    let mut window: std::collections::VecDeque<usize> = std::collections::VecDeque::new();
    let mut next = 0;
    for (i, o) in out.iter_mut().enumerate().take(n) {
        let last = (i + half).min(n - 1);
        while next <= last {
            while window.back().is_some_and(|&b| row[b] <= row[next]) {
                window.pop_back();
            }
            window.push_back(next);
            next += 1;
        }
        while window.front().is_some_and(|&f| f + half < i) {
            window.pop_front();
        }
        *o = window.front().map_or(f64::NEG_INFINITY, |&f| row[f]);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::stock::{Section, Stock};
    use cut_craft_core::collision::Holder;
    use cut_craft_core::toolpath::ToolShape;

    fn bit() -> Tool {
        Tool {
            shape: ToolShape::BallNose,
            diameter: 6.0,
            flute_length: 20.0,
            stickout: 30.0,
            ..Tool::default()
        }
    }

    fn around() -> Surroundings {
        Surroundings {
            holder: Holder {
                nut_diameter: 30.0,
                nut_length: 20.0,
                spindle_diameter: 60.0,
            },
            chuck: 10.0,
            tailstock: 0.0,
        }
    }

    #[test]
    fn the_holder_stays_above_material_beside_the_bit_not_just_under_it() {
        // a 40 mm round bar: beside it the nut (15 out, 30 up) must stay above its top
        let stock = Stock::new(Section::Round { diameter: 40.0 }, 100.0);
        let material = Voxels::of(&stock, 1.0);
        let head = Head::new(&bit(), &around(), 100.0, 20.0);
        let clear = Clearance::of(&material, 0.0, &head, 1.0);
        // straight over the bar in the middle: the nut's bottom 30 up clears the bar's 20
        assert!(clear.lowest(50.0, 0.0) < 0.0, "{}", clear.lowest(50.0, 0.0));
        // 30 mm out to the side, the bit could go to the axis, but the nut (15 out) reaches
        // back over the bar to 15 across, where it stands sqrt(20² - 15²) = 13.2 high: no
        // lower than that less the stick-out (and a cell's corner)
        let beside = clear.lowest(50.0, 30.0);
        assert!(beside > -16.8 && beside < -15.0, "{beside}");
        // far out, nothing is near
        assert!(clear.lowest(50.0, 80.0) < -100.0);
    }

    #[test]
    fn the_bit_and_holder_keep_out_of_the_chuck() {
        let stock = Stock::new(Section::Round { diameter: 40.0 }, 100.0);
        let material = Voxels::of(&stock, 1.0);
        let head = Head::new(&bit(), &around(), 100.0, 20.0);
        let clear = Clearance::of(&material, 0.0, &head, 1.0);
        // over the chuck itself: above its body
        assert!(clear.lowest(5.0, 0.0) >= CHUCK_RADIUS - 1e-9);
        // just past it: the nut still reaches over the chuck's body
        let near = clear.lowest(15.0, 0.0);
        assert!(near >= CHUCK_RADIUS - 30.0 - 1e-9, "{near}");
        assert!(head.held(5.0) && !head.held(50.0));
    }

    #[test]
    fn the_lowest_over_a_round_profile_counts_the_holder_beside_the_cut() {
        let head = Head::new(&bit(), &around(), 200.0, 20.0);
        // material 20 out everywhere: cutting a groove at 100, the nut 30 up over it
        let lowest = head.lowest_over(100.0, |_| 20.0, 0.5);
        assert!((lowest - (20.0 - 30.0)).abs() < 1e-9, "{lowest}");
        // with the material turned down to 5 out within 20 of the groove, the nut (15 out) is
        // only over the low part, 30 above it; the spindle (30 out, 50 up) reaches back over
        // the full 20, but is higher
        let turned = |a: f64| if (a - 100.0).abs() < 20.0 { 5.0 } else { 20.0 };
        let lowest = head.lowest_over(100.0, turned, 0.5);
        assert!((lowest - (5.0 - 30.0)).abs() < 1e-9, "{lowest}");
    }

    #[test]
    fn a_band_reaches_round_not_square() {
        let mut top = vec![f64::NEG_INFINITY; 21 * 21];
        top[10 * 21 + 10] = 1.0;
        let out = spread(&top, 21, 21, 5.0);
        assert_eq!(out[10 * 21 + 15], 1.0);
        assert_eq!(out[15 * 21 + 10], 1.0);
        // the corner of the square around it is out of reach
        assert_eq!(out[15 * 21 + 15], f64::NEG_INFINITY);
        assert_eq!(out[13 * 21 + 14], 1.0);
    }
}
