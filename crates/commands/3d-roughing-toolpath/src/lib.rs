//! 3D Roughing Toolpath: an end mill clears the material above the model level by level,
//! leaving an allowance for the finishing toolpath: flat at each level (Z level), or riding
//! over the model a pass depth at a time (3D raster).

use cut_craft_core::component::{MachiningBoundary, machining_boundary, placed_model};
use cut_craft_core::document::Document;
use cut_craft_core::entry::{Entry, Ramp, Ramping, path_moves};
use cut_craft_core::geometry::{Path, Point};
use cut_craft_core::simulation::Material;
use cut_craft_core::toolpath::{
    Drop, Move, MoveKind, Tool, cut_points, follow, made_by, pass_depths, raster_lines,
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
pub enum ProfilePass {
    /// Round the edge before the raster lines
    First,
    /// Round the edge after them
    Last,
    /// Raster lines only
    #[default]
    None,
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
    #[schemars(extend("x-group" = "Machining Allowance"))]
    pub allowance: f64,
    /// Its pass depth spaces the levels, its stepover the lines
    pub tool: Tool,
    /// Where the tool's tip may go
    #[serde(default)]
    #[schemars(title = "Machining Limit Boundary")]
    #[schemars(extend("x-group" = "Machining Limit Boundary"))]
    pub machining_boundary: MachiningBoundary,
    /// Grows the machining boundary, so the tool can go on past a raised edge
    /// The level whose components make the boundary
    #[serde(default)]
    #[schemars(title = "Level", extend("x-show-when" = {"machining_boundary": "selected_level"}))]
    #[schemars(extend("x-group" = "Machining Limit Boundary"))]
    pub boundary_level: String,
    #[serde(default)]
    #[schemars(title = "Boundary Offset", extend("x-unit" = "mm"))]
    #[schemars(extend("x-group" = "Machining Limit Boundary"))]
    pub machining_boundary_offset: f64,
    #[serde(default)]
    #[schemars(title = "Roughing Strategy")]
    #[schemars(extend("x-group" = "Strategy"))]
    pub strategy: Strategy,
    #[serde(default)]
    #[schemars(extend("x-show-when" = {"strategy": "z_level"}))]
    #[schemars(extend("x-group" = "Strategy"))]
    pub order: Order,
    /// A pass round the edge of each level, before or after its raster lines
    #[serde(default)]
    #[schemars(title = "Profile", extend("x-show-when" = {"strategy": "z_level"}))]
    #[schemars(extend("x-group" = "Strategy"))]
    pub profile: ProfilePass,
    /// Step from line to line the other way: top to bottom along X, left to right along Y
    #[serde(default)]
    #[schemars(extend("x-show-when" = {"strategy": "z_level"}))]
    #[schemars(extend("x-group" = "Strategy"))]
    pub reverse_step_direction: bool,
    /// Leave out what an earlier pass already cut down to the model
    #[serde(default)]
    #[schemars(extend("x-show-when" = {"strategy": "raster3d"}))]
    #[schemars(extend("x-group" = "Strategy"))]
    pub avoid_machined_areas: bool,
    /// 0° along X, 90° along Y
    #[serde(default)]
    #[schemars(extend("x-unit" = "°"), range(min = 0, max = 90))]
    #[schemars(extend("x-group" = "Strategy"))]
    pub raster_angle: f64,
    /// Zig zag down from the level above over the ramp distance instead of plunging (Z level)
    #[serde(default)]
    #[schemars(extend("x-group" = "Ramp Plunge Moves"))]
    pub ramp_plunge_moves: bool,
    #[serde(default = "ramp_distance")]
    #[schemars(extend("x-unit" = "mm"), range(min = 0))]
    #[schemars(extend(
        "x-group" = "Ramp Plunge Moves",
        "x-enable-when" = {"ramp_plunge_moves": "true"}
    ))]
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
            boundary_level: String::new(),
            machining_boundary_offset: 0.0,
            strategy: Strategy::ZLevel,
            order: Order::LevelByLevel,
            profile: ProfilePass::None,
            reverse_step_direction: false,
            avoid_machined_areas: false,
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
        let regions = machining_boundary(
            doc,
            p.machining_boundary,
            p.machining_boundary_offset,
            &p.boundary_level,
        )?;
        let mut lines = raster_lines(&regions, step, p.raster_angle);
        if p.reverse_step_direction {
            lines.reverse();
        }
        let material = doc.job.area();
        let inside = |at: Point| material.contains(at);
        let peaks = leave.peaks();
        // down to the lowest of the allowance
        let levels = pass_depths(-(lowest + p.allowance).min(0.0), tool.pass_depth);
        let z_level = p.strategy == Strategy::ZLevel;
        // ponytail: the allowance is vertical, not a true 3D offset
        let profile = if z_level {
            p.profile
        } else {
            ProfilePass::None
        };
        let depth_first_order = z_level && p.order == Order::DepthFirst;
        let tips = if depth_first_order || profile != ProfilePass::None {
            tips(&leave, tool, &regions)
        } else {
            vec![]
        };
        let runs: Vec<Vec<Vec<DVec3>>> = levels
            .iter()
            .enumerate()
            .map(|(i, &level)| {
                let ceiling = if z_level {
                    level
                } else if p.avoid_machined_areas && i > 0 {
                    // higher than the level above: the last pass cut it already
                    levels[i - 1]
                } else {
                    f64::INFINITY
                };
                let drop = Drop {
                    grid: &leave,
                    tool,
                    tolerance: TOLERANCE,
                    floor: level,
                    ceiling,
                    // a 3D raster still cuts up to where it rides over the last pass's cut
                    over_ceiling: !z_level,
                    peaks: &peaks,
                };
                level_runs(&drop, &lines, &inside, profile, &tips)
            })
            .collect();
        let order = if depth_first_order {
            depth_first(&leave, &levels, &runs, &tips)
        } else {
            (0..levels.len())
                .flat_map(|i| (0..runs[i].len()).map(move |j| (i, j)))
                .collect()
        };
        let clear = |a, b| leave.overcut_on(tool, a, b, &peaks) <= TOLERANCE;
        let moves = connect(
            &runs,
            order,
            &levels,
            p,
            2.0 * step,
            doc.job.clearance,
            &clear,
        );
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

/// One level's runs: along the raster lines, with a profile pass round the edge of what the
/// level cuts first or last, riding up no more than a cell.
fn level_runs(
    drop: &Drop,
    lines: &[Vec<Point>],
    inside: &dyn Fn(Point) -> bool,
    profile: ProfilePass,
    tips: &[f64],
) -> Vec<Vec<DVec3>> {
    let mut runs: Vec<Vec<DVec3>> = lines.iter().flat_map(|l| drop.along(l, inside)).collect();
    if profile != ProfilePass::None {
        let edge = Drop {
            ceiling: drop.floor + drop.grid.cell,
            over_ceiling: false,
            ..*drop
        };
        let rings: Vec<Vec<DVec3>> = outline(drop.grid, tips, drop.floor)
            .iter()
            .flat_map(|ring| edge.along(ring, inside))
            .collect();
        if profile == ProfilePass::First {
            runs.splice(0..0, rings);
        } else {
            runs.extend(rings);
        }
    }
    runs
}

/// The runs cut in `order`, each started with a ramp (Z level, if asked) or a plunge, or
/// joined at depth to the run before on its level when that ended no more than `reach` away
/// and `clear` says the move between keeps out of the model.
fn connect(
    runs: &[Vec<Vec<DVec3>>],
    order: Vec<(usize, usize)>,
    levels: &[f64],
    p: &Params,
    reach: f64,
    safe: f64,
    clear: &dyn Fn(DVec3, DVec3) -> bool,
) -> Vec<Move> {
    let z_level = p.strategy == Strategy::ZLevel;
    let entry = Entry {
        ramp: Ramp::ZigZag,
        ramping: Ramping {
            distance: p.ramp_distance,
            ..Ramping::default()
        },
        ..Entry::default()
    };
    let mut moves: Vec<Move> = vec![];
    // where the run before ended, and on which level
    let mut last: Option<(usize, DVec3)> = None;
    for (i, j) in order {
        let run = &runs[i][j];
        let level = levels[i];
        // stay down to the next run on the same level when it starts close by
        let near = last.is_some_and(|(at, end)| {
            at == i && end.truncate().distance(run[0].truncate()) <= reach && clear(end, run[0])
        });
        last = run.last().map(|&end| (i, end));
        if near {
            moves.pop(); // the rapid up at the end of the run before
            moves.extend(run.iter().map(|&to| Move {
                kind: MoveKind::Cut,
                to,
            }));
            moves.push(Move {
                kind: MoveKind::Rapid,
                to: run[run.len() - 1].truncate().extend(safe),
            });
            continue;
        }
        let flat = run.iter().all(|q| (q.z - level).abs() < 1e-9);
        if p.ramp_plunge_moves && z_level && flat && run.len() > 1 {
            let above = if i == 0 { 0.0 } else { levels[i - 1] };
            let points: Vec<Point> = run.iter().map(|q| q.truncate()).collect();
            moves.extend(path_moves(&points, false, &[level], above, safe, &entry));
        } else {
            moves.extend(follow(run, safe));
        }
    }
    moves
}

/// Where the tool's tip rests at each cell of the grid inside the boundary; infinite
/// outside it.
fn tips(grid: &Material, tool: &Tool, boundary: &[Path]) -> Vec<f64> {
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
    (0..rows * columns)
        .map(|i| {
            if inside[i] {
                grid.drop_tool_on(tool, grid.centre(i % columns, i / columns), &peaks)
            } else {
                f64::INFINITY
            }
        })
        .collect()
}

/// The separate areas cut at each level, as a label per cell of the grid: where the tool's
/// tip ([`tips`]) rests no higher than the level.
fn areas(grid: &Material, levels: &[f64], tips: &[f64]) -> Vec<Vec<Option<usize>>> {
    let (columns, rows) = (grid.columns, grid.rows);
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

/// The edges of what a level cuts: round the cells whose tip rests no higher than it, as
/// closed lines (the first point again at the end).
fn outline(grid: &Material, tips: &[f64], level: f64) -> Vec<Vec<Point>> {
    use cut_craft_core::trace::{self, Grid};
    let at = Grid {
        columns: grid.columns,
        rows: grid.rows,
        origin: grid.origin,
        cell: grid.cell,
    };
    trace::outlines(at, |c, r| tips[r * grid.columns + c] <= level + 1e-9)
        .iter()
        .map(|ring| {
            let mut points = trace::simplify(ring, grid.cell / 2.0);
            if let Some(&first) = points.first() {
                points.push(first);
            }
            points
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
    levels: &[f64],
    runs: &[Vec<Vec<DVec3>>],
    tips: &[f64],
) -> Vec<(usize, usize)> {
    let (columns, rows) = (grid.columns, grid.rows);
    let labels = areas(grid, levels, tips);
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
    use cut_craft_core::toolpath::Toolpath;

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

    /// What a toolpath leaves of the domed job's material.
    fn left_by(tp: &Toolpath) -> Vec<f64> {
        let mut m = Material::new(&domed().job, 134);
        m.cut(tp);
        m.heights
    }

    #[test]
    fn a_profile_pass_runs_round_the_model_at_each_level_without_cutting_into_it() {
        let tp = rough(&Params {
            profile: ProfilePass::Last,
            ..Params::default()
        });
        let top_level = levels(&tp)[0];
        // the allowance meets the top level about 6.2 mm out; the tool's centre 3 mm further
        let ring: Vec<Point> = tp
            .moves
            .iter()
            .filter(|m| m.kind == MoveKind::Cut && (m.to.z - top_level).abs() < 1e-6)
            .map(|m| m.to.truncate())
            .filter(|p| (p.distance(Point::new(20.0, 20.0)) - 9.2).abs() < 0.6)
            .collect();
        let quadrants = [(1.0, 1.0), (-1.0, 1.0), (-1.0, -1.0), (1.0, -1.0)]
            .iter()
            .filter(|(sx, sy)| {
                ring.iter()
                    .any(|p| (p.x - 20.0) * sx > 2.0 && (p.y - 20.0) * sy > 2.0)
            })
            .count();
        assert_eq!(quadrants, 4, "all the way round: {ring:?}");
        let without = left_by(&rough(&Params::default()));
        let with = left_by(&tp);
        let shape = model(&domed(), 134);
        let top = shape
            .heights
            .iter()
            .copied()
            .fold(f64::NEG_INFINITY, f64::max);
        let into = with
            .iter()
            .zip(&shape.heights)
            .filter(|&(&z, &h)| z < (h - top + 0.5).min(0.0) - 0.01)
            .count();
        assert_eq!(into, 0, "into the allowance");
        assert!(
            with.iter().sum::<f64>() < without.iter().sum::<f64>(),
            "it cleans up more"
        );
    }

    #[test]
    fn avoiding_machined_areas_cuts_less_for_the_same_result() {
        let raster = |avoid| {
            rough(&Params {
                strategy: Strategy::Raster3d,
                avoid_machined_areas: avoid,
                ..Params::default()
            })
        };
        let (all, avoided) = (raster(false), raster(true));
        assert!(avoided.moves.len() < all.moves.len());
        let worst = left_by(&all)
            .iter()
            .zip(left_by(&avoided))
            .map(|(a, b)| (a - b).abs())
            .fold(0.0, f64::max);
        assert!(worst < 0.05, "{worst}");
    }

    #[test]
    fn reversing_the_step_direction_starts_at_the_far_side() {
        let first_y = |reverse| {
            let tp = rough(&Params {
                reverse_step_direction: reverse,
                ..Params::default()
            });
            tp.moves
                .iter()
                .find(|m| m.kind == MoveKind::Cut)
                .unwrap()
                .to
                .y
        };
        assert!(first_y(false) < 10.0 && first_y(true) > 30.0);
    }

    #[test]
    fn lines_beside_each_other_on_a_level_are_linked() {
        let tp = rough(&Params::default());
        // across from one line to the next at the same depth, without lifting
        let step = Params::default().tool.stepover * Params::default().tool.diameter;
        let links = tp
            .moves
            .windows(2)
            .filter(|w| {
                let (a, b) = (w[0].to, w[1].to);
                w[1].kind == MoveKind::Cut
                    && w[0].kind == MoveKind::Cut
                    && (a.z - b.z).abs() < 1e-9
                    && (a.x - b.x).abs() < 1e-9
                    && ((b.y - a.y).abs() - step).abs() < 1e-6
            })
            .count();
        let plunges = tp
            .moves
            .iter()
            .filter(|m| m.kind == MoveKind::Plunge)
            .count();
        assert!(links > plunges, "{links} links, {plunges} plunges");
    }
}
