//! Celtic Weave Creator (an Aspire gadget): a Celtic knot's loops as smooth closed vectors,
//! the drive rails Extrude and Weave turns into a weave. Strands run diagonally between the
//! crossing points of a knot grid and bounce off its edges; at each crossing a strand carries
//! straight on, unless the crossing is cut: a vertical cut turns it back on the same side
//! left to right, a horizontal cut on the same side top to bottom. A cross-section for
//! Extrude and Weave is added beside the knot if asked.

use cut_craft_core::document::Document;
use cut_craft_core::geometry::{Path, Point};
use cut_craft_core::trace::{Fit, fit};
use cut_craft_core::{Change, Command, CommandError, Meta};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

pub struct CelticWeave;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum CutKind {
    #[default]
    #[schemars(title = "Cut Vertical")]
    Vertical,
    #[schemars(title = "Cut Horizontal")]
    Horizontal,
}

/// A crossing that is cut, counted in half cells from 0 at the bottom left: crossings are
/// inside the grid where column + row is odd.
#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, JsonSchema)]
pub struct Cut {
    pub column: i32,
    pub row: i32,
    #[schemars(extend("x-widget" = "row"))]
    pub cut: CutKind,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
pub struct Params {
    /// The knot grid's size in cells
    #[schemars(range(min = 1, max = 50))]
    pub columns: i32,
    #[schemars(range(min = 1, max = 50))]
    pub rows: i32,
    #[schemars(title = "Cell size", extend("x-unit" = "mm"), range(min = 0))]
    pub cell: f64,
    /// Crossings the strands turn at rather than cross
    pub cuts: Vec<Cut>,
    /// A half-round profile beside the knot, for Extrude and Weave
    #[schemars(title = "Create cross-section")]
    pub cross_section: bool,
}

impl Default for Params {
    fn default() -> Self {
        Self {
            columns: 3,
            rows: 3,
            cell: 20.0,
            cuts: vec![],
            cross_section: true,
        }
    }
}

type At = (i32, i32);

/// The knot grid's loops, each as the lattice points (in half cells) it passes, crossings and
/// the edge points it bounces at.
fn loops(p: &Params) -> Vec<Vec<At>> {
    let (w, h) = (2 * p.columns, 2 * p.rows);
    // strands pass the lattice points where x + y is odd: crossings inside the grid, bounce
    // points on its edge (never its corners)
    let on_grid = |(x, y): At| (0..=w).contains(&x) && (0..=h).contains(&y);
    let crossing = |(x, y): At| (1..w).contains(&x) && (1..h).contains(&y) && (x + y) % 2 == 1;
    let cut_at = |(x, y): At| {
        p.cuts
            .iter()
            .find(|c| (c.column, c.row) == (x, y))
            .map(|c| c.cut)
    };
    let route = |at: At, (dx, dy): At| match cut_at(at) {
        None => (dx, dy),
        Some(CutKind::Vertical) => (-dx, dy),
        Some(CutKind::Horizontal) => (dx, -dy),
    };
    let mut used: HashSet<(At, At)> = HashSet::new();
    let mut out = vec![];
    let starts: Vec<At> = (0..=w)
        .flat_map(|x| (0..=h).map(move |y| (x, y)))
        .filter(|&(x, y)| (x + y) % 2 == 1)
        .collect();
    for &start in &starts {
        for d in [(1, 1), (-1, 1), (1, -1), (-1, -1)] {
            if used.contains(&(start, d)) || !on_grid((start.0 + d.0, start.1 + d.1)) {
                continue;
            }
            let (mut at, mut dir) = (start, d);
            let mut path = vec![];
            loop {
                used.insert((at, dir));
                let next = (at.0 + dir.0, at.1 + dir.1);
                used.insert((next, (-dir.0, -dir.1)));
                path.push(next);
                if crossing(next) {
                    dir = route(next, dir);
                } else {
                    // bounce off the grid's edge
                    if next.0 == 0 || next.0 == w {
                        dir.0 = -dir.0;
                    }
                    if next.1 == 0 || next.1 == h {
                        dir.1 = -dir.1;
                    }
                }
                at = next;
                if (at, dir) == (start, d) {
                    break;
                }
            }
            out.push(path);
        }
    }
    out
}

/// Corner cutting (Chaikin): a smoother closed ring through the same shape.
fn smoothed(ring: &[Point], times: usize) -> Vec<Point> {
    let mut ring = ring.to_vec();
    for _ in 0..times {
        let n = ring.len();
        ring = (0..n)
            .flat_map(|i| {
                let (a, b) = (ring[i], ring[(i + 1) % n]);
                [a * 0.75 + b * 0.25, a * 0.25 + b * 0.75]
            })
            .collect();
    }
    ring
}

impl Command for CelticWeave {
    const ID: &'static str = "celtic_weave";
    type Params = Params;

    fn meta() -> Meta {
        Meta {
            label: "Celtic Weave Creator",
            group: "Gadgets",
            icon: "",
            shortcut: "",
        }
    }

    fn run(doc: &Document, p: &Params) -> Result<Change, CommandError> {
        if p.columns < 1 || p.rows < 1 || p.cell <= 0.0 {
            return Err(CommandError::InvalidParams(
                "the knot grid needs a cell or more and a cell size".into(),
            ));
        }
        let half = p.cell / 2.0;
        let size = Point::new(f64::from(p.columns), f64::from(p.rows)) * p.cell;
        let origin = doc.job.area().center() - size / 2.0;
        let place = |(x, y): At| origin + Point::new(f64::from(x), f64::from(y)) * half;
        let mut c = Change::builder(doc);
        let mut ids = vec![];
        for knot in loops(p) {
            let ring: Vec<Point> = knot.into_iter().map(place).collect();
            // ponytail: four rounds of corner cutting, fitted within a fiftieth of a cell
            let smooth = smoothed(&ring, 4);
            ids.push(c.add(fit(
                &smooth,
                true,
                Fit::Beziers,
                p.cell / 50.0,
                std::f64::consts::PI,
            )));
        }
        if p.cross_section {
            // a half-round as wide as a strand, beside the knot
            let width = half / 2.0;
            let at = origin + Point::new(size.x + p.cell, 0.0);
            let profile: Vec<Point> = (0..=16)
                .map(|i| {
                    let a = std::f64::consts::PI * f64::from(i) / 16.0;
                    at + Point::new(width / 2.0 * (1.0 - a.cos()), width / 2.0 * a.sin())
                })
                .collect();
            ids.push(c.add(Path::polyline(&profile, false)));
        }
        c.select(ids);
        Ok(c.finish())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cut_craft_core::testing::rejection;

    fn crossings_passed(p: &Params) -> usize {
        let (w, h) = (2 * p.columns, 2 * p.rows);
        loops(p)
            .iter()
            .flatten()
            .filter(|&&(x, y)| (1..w).contains(&x) && (1..h).contains(&y))
            .count()
    }

    fn crossings(p: &Params) -> usize {
        let (w, h) = (2 * p.columns, 2 * p.rows);
        (1..w)
            .flat_map(|x| (1..h).map(move |y| (x, y)))
            .filter(|(x, y)| (x + y) % 2 == 1)
            .count()
    }

    cut_craft_core::contract_tests!(CelticWeave);

    #[test]
    fn every_crossing_is_passed_twice() {
        for (columns, rows) in [(1, 1), (2, 3), (4, 4)] {
            let p = Params {
                columns,
                rows,
                ..Params::default()
            };
            assert_eq!(
                crossings_passed(&p),
                2 * crossings(&p),
                "{columns} × {rows}"
            );
            let cut = Params {
                cuts: vec![Cut {
                    column: 1,
                    row: 1,
                    cut: CutKind::Vertical,
                }],
                ..p.clone()
            };
            assert_eq!(
                crossings_passed(&cut),
                2 * crossings(&cut),
                "cut, {columns} × {rows}"
            );
        }
    }

    #[test]
    fn a_cut_changes_how_the_strands_join() {
        let p = Params {
            columns: 2,
            rows: 1,
            ..Params::default()
        };
        assert_eq!(loops(&p).len(), 1, "one strand through the middle crossing");
        let cut = Params {
            cuts: vec![Cut {
                column: 2,
                row: 1,
                cut: CutKind::Vertical,
            }],
            ..p.clone()
        };
        assert_eq!(loops(&cut).len(), 2, "cut vertically, a loop in each cell");
    }

    #[test]
    fn the_loops_are_closed_vectors_centred_on_the_job_with_a_cross_section() {
        let mut doc = Document::default();
        cut_craft_core::testing::run_on::<CelticWeave>(&mut doc, &Params::default());
        let closed = doc.vectors.values().filter(|v| v.closed).count();
        assert_eq!(closed, loops(&Params::default()).len());
        assert_eq!(doc.vectors.len(), closed + 1, "and the cross-section");
        let knot = doc
            .vectors
            .values()
            .filter(|v| v.closed)
            .map(cut_craft_core::geometry::Path::bounds)
            .reduce(cut_craft_core::geometry::Rect::union)
            .unwrap();
        assert!(
            (knot.center() - doc.job.area().center()).length() < 1.0,
            "{knot:?}"
        );
    }

    #[test]
    fn needs_a_grid() {
        let p = Params {
            columns: 0,
            ..Params::default()
        };
        assert!(rejection::<CelticWeave>(&p).contains("knot grid"));
    }
}
