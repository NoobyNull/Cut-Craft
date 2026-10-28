//! 3D Roughing Toolpath: an end mill clears the material above the model level by level,
//! leaving an allowance for the finishing toolpath: flat at each level (Z level), or riding
//! over the model a pass depth at a time (3D raster).

use cut_craft_core::component::{MachiningBoundary, machining_boundary, placed_model};
use cut_craft_core::document::Document;
use cut_craft_core::entry::{Entry, Ramp, Ramping, path_moves};
use cut_craft_core::geometry::{Path, Point};
use cut_craft_core::simulation::Material;
use cut_craft_core::toolpath::{
    Drop, Move, Tool, cut_points, follow, made_by, pass_depths, raster_lines,
};
use cut_craft_core::{Change, Command, CommandError, Meta, ensure_positive};
use glam::DVec3;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub struct RoughingToolpath3d;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Strategy {
    /// Flat at each level, round the model
    #[default]
    #[schemars(title = "Z Level")]
    ZLevel,
    /// Over the model, never deeper than each level: a more even skin for finishing
    #[schemars(title = "3D Raster")]
    Raster3d,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Order {
    /// All of each level before the next one down
    #[default]
    LevelByLevel,
    /// Each separate area all the way down before moving to the next
    DepthFirst,
}

/// How far a straight move may pass under the allowance before it is split.
const TOLERANCE: f64 = 0.01;

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
pub struct Params {
    pub name: String,
    /// Left on the model for the finishing toolpath to take off
    #[schemars(extend("x-unit" = "mm"), range(min = 0))]
    pub allowance: f64,
    /// Its pass depth spaces the levels, its stepover the lines
    pub tool: Tool,
    /// Where the tool's tip may go
    #[serde(default)]
    #[schemars(title = "Machining Limit Boundary")]
    pub machining_boundary: MachiningBoundary,
    /// Grows the machining boundary, so the tool can go on past a raised edge
    #[serde(default)]
    #[schemars(title = "Boundary Offset", extend("x-unit" = "mm"))]
    pub machining_boundary_offset: f64,
    #[serde(default)]
    #[schemars(title = "Roughing Strategy")]
    pub strategy: Strategy,
    #[serde(default)]
    #[schemars(extend("x-show-when" = {"strategy": "z_level"}))]
    pub order: Order,
    /// 0° along X, 90° along Y
    #[serde(default)]
    #[schemars(extend("x-unit" = "°"), range(min = 0, max = 90))]
    pub raster_angle: f64,
    /// Zig zag down from the level above over the ramp distance instead of plunging (Z level)
    #[serde(default)]
    pub ramp_plunge_moves: bool,
    #[serde(default = "ramp_distance")]
    #[schemars(extend("x-unit" = "mm"), range(min = 0))]
    pub ramp_distance: f64,
}

fn ramp_distance() -> f64 {
    Ramping::default().distance
}

impl Default for Params {
    fn default() -> Self {
        Self {
            name: "3D Roughing".into(),
            allowance: 0.5,
            tool: Tool::default(),
            machining_boundary: MachiningBoundary::Material,
            machining_boundary_offset: 0.0,
            strategy: Strategy::ZLevel,
            order: Order::LevelByLevel,
            raster_angle: 0.0,
            ramp_plunge_moves: false,
            ramp_distance: ramp_distance(),
        }
    }
}

impl Command for RoughingToolpath3d {
    const ID: &'static str = "3d_roughing_toolpath";
    type Params = Params;

    fn meta() -> Meta {
        Meta {
            label: "3D Roughing Toolpath",
            group: "3D Toolpaths",
            icon: "toolpaths/3d_roughing_toolpath.png",
            shortcut: "",
        }
    }

    fn run(doc: &Document, p: &Params) -> Result<Change, CommandError> {
        let tool = &p.tool;
        ensure_positive("tool diameter", tool.diameter)?;
        ensure_positive("pass depth", tool.pass_depth)?;
        if !(tool.stepover > 0.0 && tool.stepover <= 1.0) {
            return Err(CommandError::InvalidParams(
                "stepover must be more than 0 and at most 1".into(),
            ));
        }
        if p.allowance.is_nan() || p.allowance < 0.0 {
            return Err(CommandError::InvalidParams(
                "the allowance cannot be negative".into(),
            ));
        }
        // what must be left: the model and the allowance on top of it
        let mut leave = placed_model(doc, tool)?;
        let lowest = leave.heights.iter().copied().fold(0.0, f64::min);
        for height in &mut leave.heights {
            *height += p.allowance;
        }
        let step = tool.stepover * tool.diameter;
        let regions = machining_boundary(doc, p.machining_boundary, p.machining_boundary_offset)?;
        let lines = raster_lines(&regions, step, p.raster_angle);
        let material = doc.job.area();
        let inside = |at: Point| material.contains(at);
        let peaks = leave.peaks();
        // down to the lowest of the allowance
        let levels = pass_depths(-(lowest + p.allowance).min(0.0), tool.pass_depth);
        let z_level = p.strategy == Strategy::ZLevel;
        // ponytail: each run retracts and the next plunges; link neighbouring runs if
        // roughing time matters, and the allowance is vertical, not a true 3D offset
        let runs: Vec<Vec<Vec<DVec3>>> = levels
            .iter()
            .map(|&level| {
                let drop = Drop {
                    grid: &leave,
                    tool,
                    tolerance: TOLERANCE,
                    floor: level,
                    ceiling: if z_level { level } else { f64::INFINITY },
                    peaks: &peaks,
                };
                lines.iter().flat_map(|l| drop.along(l, &inside)).collect()
            })
            .collect();
        let order = if z_level && p.order == Order::DepthFirst {
            depth_first(&leave, tool, &levels, &runs, &regions)
        } else {
            (0..levels.len())
                .flat_map(|i| (0..runs[i].len()).map(move |j| (i, j)))
                .collect()
        };
        let entry = Entry {
            ramp: Ramp::ZigZag,
            ramping: Ramping {
                distance: p.ramp_distance,
                ..Ramping::default()
            },
            ..Entry::default()
        };
        let safe = doc.job.clearance;
        let mut moves: Vec<Move> = vec![];
        for (i, j) in order {
            let run = &runs[i][j];
            let level = levels[i];
            let flat = run.iter().all(|q| (q.z - level).abs() < 1e-9);
            if p.ramp_plunge_moves && z_level && flat && run.len() > 1 {
                let above = if i == 0 { 0.0 } else { levels[i - 1] };
                let points: Vec<Point> = run.iter().map(|q| q.truncate()).collect();
                moves.extend(path_moves(&points, false, &[level], above, safe, &entry));
            } else {
                moves.extend(follow(run, safe));
            }
        }
        let mut c = Change::builder(doc);
        c.add_toolpath(made_by::<Self>(
            p,
            p.name.clone(),
            tool.clone(),
            vec![],
            moves,
        ));
        Ok(c.finish())
    }
}

/// The separate areas cut at each level, as a label per cell of the grid: where the tool's
/// tip, inside the boundary, rests no higher than the level.
fn areas(
    grid: &Material,
    tool: &Tool,
    levels: &[f64],
    boundary: &[Path],
) -> Vec<Vec<Option<usize>>> {
    let (columns, rows) = (grid.columns, grid.rows);
    let peaks = grid.peaks();
    // the cells inside the boundary, a row at a time (even-odd, so holes stay out)
    let rings: Vec<Vec<Point>> = boundary.iter().map(cut_points).collect();
    let mut inside = vec![false; rows * columns];
    for r in 0..rows {
        let y = grid.centre(0, r).y;
        let mut xs: Vec<f64> = rings
            .iter()
            .flat_map(|ring| {
                (0..ring.len()).filter_map(move |i| {
                    let (a, b) = (ring[i], ring[(i + 1) % ring.len()]);
                    ((a.y <= y) != (b.y <= y)).then(|| a.x + (y - a.y) / (b.y - a.y) * (b.x - a.x))
                })
            })
            .collect();
        xs.sort_by(f64::total_cmp);
        for pair in xs.as_chunks::<2>().0 {
            for (c, cell) in inside[r * columns..(r + 1) * columns]
                .iter_mut()
                .enumerate()
            {
                let x = grid.centre(c, r).x;
                *cell |= x >= pair[0] && x <= pair[1];
            }
        }
    }
    // where the tool's tip rests at each cell inside
    let tips: Vec<f64> = (0..rows * columns)
        .map(|i| {
            if inside[i] {
                grid.drop_tool_on(tool, grid.centre(i % columns, i / columns), &peaks)
            } else {
                f64::INFINITY
            }
        })
        .collect();
    // the separate areas cut at each level, as a label per cell
    levels
        .iter()
        .map(|&level| {
            let mut label = vec![None; rows * columns];
            let mut next = 0;
            for start in 0..rows * columns {
                if label[start].is_some() || tips[start] > level + 1e-9 {
                    continue;
                }
                let mut stack = vec![start];
                label[start] = Some(next);
                while let Some(i) = stack.pop() {
                    let (c, r) = (i % columns, i / columns);
                    let around = [
                        (c > 0).then(|| i - 1),
                        (c + 1 < columns).then(|| i + 1),
                        (r > 0).then(|| i - columns),
                        (r + 1 < rows).then(|| i + columns),
                    ];
                    for j in around.into_iter().flatten() {
                        if label[j].is_none() && tips[j] <= level + 1e-9 {
                            label[j] = Some(next);
                            stack.push(j);
                        }
                    }
                }
                next += 1;
            }
            label
        })
        .collect()
}

/// The runs (level, index) in depth-first order: each separate area at a level, then the
/// areas under it all the way down, before the next area.
#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "cell indices from points on the grid"
)]
fn depth_first(
    grid: &Material,
    tool: &Tool,
    levels: &[f64],
    runs: &[Vec<Vec<DVec3>>],
    boundary: &[Path],
) -> Vec<(usize, usize)> {
    let (columns, rows) = (grid.columns, grid.rows);
    let labels = areas(grid, tool, levels, boundary);
    // the area at a level under a point, or under a cell beside it: a run can start on the
    // boundary, just off the cells inside it
    let label = |level: usize, p: DVec3| {
        let c = ((p.x - grid.origin.x) / grid.cell).floor() as isize;
        let r = ((p.y - grid.origin.y) / grid.cell).floor() as isize;
        [
            (0, 0),
            (-1, 0),
            (1, 0),
            (0, -1),
            (0, 1),
            (-1, -1),
            (1, -1),
            (-1, 1),
            (1, 1),
        ]
        .iter()
        .find_map(|&(dc, dr)| {
            let (c, r) = (c + dc, r + dr);
            let on = c >= 0 && r >= 0 && (c as usize) < columns && (r as usize) < rows;
            on.then(|| labels[level][r as usize * columns + c as usize])
                .flatten()
        })
    };
    // each run's area; a run whose cells were never labelled goes with the first area
    let area = |level: usize, run: &[DVec3]| run.iter().find_map(|&q| label(level, q)).unwrap_or(0);
    let mut order = vec![];
    // areas still to do, deepest pushed last: (level, area)
    let first: Vec<usize> = {
        let mut seen = vec![];
        for run in &runs[0] {
            let a = area(0, run);
            if !seen.contains(&a) {
                seen.push(a);
            }
        }
        seen
    };
    let mut stack: Vec<(usize, usize)> = first.into_iter().rev().map(|a| (0, a)).collect();
    while let Some((level, a)) = stack.pop() {
        order.extend(
            runs[level]
                .iter()
                .enumerate()
                .filter(|(_, run)| area(level, run) == a)
                .map(|(j, _)| (level, j)),
        );
        if level + 1 < levels.len() {
            // the areas one level down that lie under this one
            let mut under = vec![];
            for run in &runs[level + 1] {
                let b = area(level + 1, run);
                let parent = run.iter().find_map(|&q| label(level, q)).unwrap_or(0);
                if parent == a && !under.contains(&b) {
                    under.push(b);
                }
            }
            stack.extend(under.into_iter().rev().map(|b| (level + 1, b)));
        }
    }
    order
}

#[cfg(test)]
mod tests {
    use super::*;
    use cut_craft_core::component::model;
    use cut_craft_core::geometry::Point;
    use cut_craft_core::simulation::Material;
    use cut_craft_core::testing::domed;
    use cut_craft_core::toolpath::{MoveKind, Toolpath};

    cut_craft_core::contract_tests!(RoughingToolpath3d, domed);

    fn rough(params: &Params) -> Toolpath {
        let mut doc = domed();
        let change = RoughingToolpath3d::run(&doc, params).unwrap();
        doc.apply(&change).unwrap();
        doc.toolpaths.into_values().next().unwrap()
    }

    #[test]
    fn it_clears_down_to_the_allowance_without_touching_it() {
        let doc = domed();
        let toolpath = rough(&Params::default());
        let cells = 134; // a twentieth of the 6 mm tool
        let mut cut = Material::new(&doc.job, cells);
        cut.cut(&toolpath);
        let shape = model(&doc, cells);
        let top = shape
            .heights
            .iter()
            .copied()
            .fold(f64::NEG_INFINITY, f64::max);
        let into_allowance = cut
            .heights
            .iter()
            .zip(&shape.heights)
            // where model and allowance stand above the surface there is nothing to leave
            .filter(|&(&z, &h)| z < (h - top + 0.5).min(0.0) - 0.01)
            .count();
        assert_eq!(into_allowance, 0, "cells cut into the allowance");
        let floor = cut.at(Point::new(3.0, 3.0)).unwrap();
        assert!(
            (floor - (-8.0 + 0.5)).abs() < 0.02,
            "the flat around it: {floor}"
        );
        assert_eq!(cut.at(Point::new(20.0, 20.0)), Some(0.0), "the top is left");
    }

    #[test]
    fn no_level_cuts_deeper_than_the_pass_depth() {
        let toolpath = rough(&Params::default());
        let mut levels: Vec<f64> = toolpath
            .moves
            .iter()
            .map(|m| m.to.z)
            .filter(|&z| z < 0.0)
            .collect();
        levels.sort_by(f64::total_cmp);
        levels.dedup_by(|a, b| (*a - *b).abs() < 1e-9);
        // the flat levels, top down, are no more than a pass depth apart
        let pass = Params::default().tool.pass_depth;
        let deepest = levels[0];
        // the model's top is its highest cell, a hair under 8 mm
        assert!(
            (deepest + 7.5).abs() < 0.01,
            "down to the allowance: {deepest}"
        );
        let first_level = levels.iter().copied().fold(f64::NEG_INFINITY, f64::max);
        assert!(first_level >= -pass - 1e-9);
    }

    /// The depths of the flat levels a toolpath cuts at, top down.
    fn levels(tp: &Toolpath) -> Vec<f64> {
        let mut levels: Vec<f64> = tp
            .moves
            .iter()
            .filter(|m| m.kind == MoveKind::Cut && m.to.z < -1e-9)
            .map(|m| m.to.z)
            .collect();
        levels.sort_by(|a, b| b.total_cmp(a));
        levels.dedup_by(|a, b| (*a - *b).abs() < 1e-6);
        levels
    }

    #[test]
    fn z_level_cuts_are_flat_at_each_level() {
        let tp = rough(&Params::default());
        assert_eq!(levels(&tp).len(), 3, "{:?}", levels(&tp));
    }

    #[test]
    fn depth_first_finishes_one_area_before_the_next_and_cuts_the_same() {
        use cut_craft_core::testing::square;
        let mut doc = domed();
        // two separate corners, clear of the dome
        let mut c = cut_craft_core::Change::builder(&doc);
        let ids = vec![
            c.add(square(0.0, 0.0, 10.0)),
            c.add(square(30.0, 30.0, 10.0)),
        ];
        c.select(ids);
        doc.apply(&c.finish()).unwrap();
        let make = |order| {
            let mut d = doc.clone();
            let p = Params {
                order,
                machining_boundary: MachiningBoundary::SelectedVectors,
                ..Params::default()
            };
            d.apply(&RoughingToolpath3d::run(&d, &p).unwrap()).unwrap();
            d.toolpaths.into_values().next().unwrap()
        };
        let (by_level, by_depth) = (make(Order::LevelByLevel), make(Order::DepthFirst));
        let cut = |tp: &Toolpath| {
            let mut m = Material::new(&doc.job, 100);
            m.cut(tp);
            m.heights
        };
        assert_eq!(cut(&by_level), cut(&by_depth), "the same material removed");
        // which corner each cut is in, in order: depth first switches corner once
        let switches = |tp: &Toolpath| {
            let corners: Vec<bool> = tp
                .moves
                .iter()
                .filter(|m| m.kind == MoveKind::Cut)
                .map(|m| m.to.x > 20.0)
                .collect();
            corners.windows(2).filter(|w| w[0] != w[1]).count()
        };
        assert_eq!(switches(&by_depth), 1);
        assert!(switches(&by_level) > 1);
    }

    #[test]
    fn ramp_plunge_moves_never_go_straight_down_into_material() {
        let tp = rough(&Params {
            ramp_plunge_moves: true,
            ..Params::default()
        });
        let levels = levels(&tp);
        for w in tp.moves.windows(2) {
            let (a, b) = (w[0].to, w[1].to);
            if b.z < a.z - 1e-9 && a.truncate().distance(b.truncate()) < 1e-9 {
                // only down to the surface or a level already cleared
                let cleared = &levels[..levels.len() - 1];
                let ok = b.z.abs() < 1e-9 || cleared.iter().any(|&l| (b.z - l).abs() < 1e-9);
                assert!(ok, "{a} to {b}");
            }
        }
    }
}
