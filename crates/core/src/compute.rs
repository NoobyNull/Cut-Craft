//! Where the grid kernels run: the tool dropped onto a height grid, swept along moves and
//! checked for gouging, over many cells at once. [`Cpu`] is the reference (a band of rows
//! or a share of the items per core); the app registers a GPU one with [`use_compute`] when
//! the window has a card, and the kernels' callers go through [`compute`].

use crate::geometry::Point;
use crate::simulation::{Material, by_rows, each};
use crate::toolpath::{Move, Tool};
use glam::DVec3;
use std::sync::{Arc, RwLock};

pub trait Compute: Send + Sync {
    /// Where `tool`'s tip rests over each of `points` on `grid` ([`Material::drop_tool_on`]).
    fn drop_at(&self, grid: &Material, tool: &Tool, points: &[Point]) -> Vec<f64>;
    /// [`Material::drop_tool_on`] at every cell centre of `grid`, row by row from y = 0.
    fn drop_everywhere(&self, grid: &Material, tool: &Tool) -> Vec<f64>;
    /// [`Material::overcut_on`] of each straight move, from `.0` to `.1`.
    fn overcut_along(&self, grid: &Material, tool: &Tool, moves: &[(DVec3, DVec3)]) -> Vec<f64>;
    /// Lowers `material` to what `tool` leaves following `moves` ([`Material::cut_along`]).
    fn cut_along(&self, material: &mut Material, tool: &Tool, moves: &[Move]);
}

/// The kernels on the CPU: [`crate::simulation`]'s code, across the cores.
pub struct Cpu;

impl Compute for Cpu {
    fn drop_at(&self, grid: &Material, tool: &Tool, points: &[Point]) -> Vec<f64> {
        let peaks = grid.peaks();
        each(points, |&p| grid.drop_tool_on(tool, p, &peaks))
    }

    fn drop_everywhere(&self, grid: &Material, tool: &Tool) -> Vec<f64> {
        let peaks = grid.peaks();
        let mut tips = vec![0.0; grid.columns * grid.rows];
        by_rows(&mut tips, grid.columns, |first, band| {
            for (i, tip) in band.iter_mut().enumerate() {
                let (column, row) = (i % grid.columns, first + i / grid.columns);
                *tip = grid.drop_tool_on(tool, grid.centre(column, row), &peaks);
            }
        });
        tips
    }

    fn overcut_along(&self, grid: &Material, tool: &Tool, moves: &[(DVec3, DVec3)]) -> Vec<f64> {
        let peaks = grid.peaks();
        each(moves, |&(a, b)| grid.overcut_on(tool, a, b, &peaks))
    }

    fn cut_along(&self, material: &mut Material, tool: &Tool, moves: &[Move]) {
        let mut heights = std::mem::take(&mut material.heights);
        by_rows(&mut heights, material.columns, |first, band| {
            for pair in moves.windows(2) {
                material.sweep_band(pair[0].to, pair[1].to, tool, first, band);
            }
        });
        material.heights = heights;
    }
}

static CURRENT: RwLock<Option<Arc<dyn Compute>>> = RwLock::new(None);

/// Runs the kernels on `compute` from now on; `None` for the CPU.
pub fn use_compute(compute: Option<Arc<dyn Compute>>) {
    *CURRENT.write().expect("not poisoned") = compute;
}

/// Where the kernels run now.
pub fn compute() -> Arc<dyn Compute> {
    CURRENT
        .read()
        .expect("not poisoned")
        .clone()
        .unwrap_or_else(|| Arc::new(Cpu))
}
