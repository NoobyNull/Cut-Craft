//! Toolpaths: the tool, the strategy command that made them (so they can be recalculated),
//! and the computed moves.

use crate::component::placed_model;
use crate::document::{Document, EntityId};
use crate::geometry::{Path, Point};
use crate::simulation::Material;
use crate::{Command, CommandError, polygon};
use glam::DVec3;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ToolShape {
    /// Flat bottomed
    #[default]
    EndMill,
    /// Round bottomed, for 3D finishing
    BallNose,
    /// Pointed, for V-carving
    VBit,
    /// Flat bottomed with rounded corners (a bull nose, or toroidal cutter): stronger than an
    /// end mill, leaves less cusp than one when 3D roughing or finishing
    #[schemars(title = "Radiused End Mill")]
    RadiusedEndMill,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Tool {
    pub name: String,
    #[serde(default)]
    pub shape: ToolShape,
    /// Radius of a radiused end mill's rounded corners
    #[serde(default = "default_tip_radius")]
    #[schemars(title = "Tip Radius", extend("x-unit" = "mm"), range(min = 0))]
    pub tip_radius: f64,
    /// Included angle of a V-bit's point
    #[serde(default = "default_angle")]
    #[schemars(extend("x-unit" = "°"), range(min = 1, max = 179))]
    pub angle: f64,
    #[schemars(extend("x-unit" = "mm"), range(min = 0))]
    pub diameter: f64,
    /// Depth cut on each pass
    #[schemars(extend("x-unit" = "mm"), range(min = 0))]
    pub pass_depth: f64,
    /// Sideways step between pocketing passes, as a fraction of the diameter
    #[schemars(range(min = 0, max = 1))]
    pub stepover: f64,
    #[schemars(extend("x-unit" = "mm/min"), range(min = 0))]
    pub feed_rate: f64,
    #[schemars(extend("x-unit" = "mm/min"), range(min = 0))]
    pub plunge_rate: f64,
    #[schemars(extend("x-unit" = "rpm"), range(min = 0))]
    pub spindle_speed: f64,
    /// Cutting edges, for the chip load (added after the first tool format)
    #[serde(default = "default_flutes")]
    #[schemars(title = "Number of Flutes", range(min = 1, max = 12))]
    pub flutes: u32,
}

fn default_angle() -> f64 {
    90.0
}

fn default_flutes() -> u32 {
    2
}

fn default_tip_radius() -> f64 {
    1.0
}

impl Tool {
    /// How much each flute cuts per turn (mm): the feed rate over the spindle speed and
    /// flutes, to compare with the cutter maker's figure. `None` with no spindle or flutes.
    pub fn chip_load(&self) -> Option<f64> {
        let teeth_per_minute = self.spindle_speed * f64::from(self.flutes);
        (teeth_per_minute > 0.0).then(|| self.feed_rate / teeth_per_minute)
    }

    /// How far above its tip the cutting edge is at `r` from the axis; `None` beyond the tool.
    pub fn profile(&self, r: f64) -> Option<f64> {
        let radius = self.diameter / 2.0;
        (r <= radius).then(|| match self.shape {
            ToolShape::EndMill => 0.0,
            ToolShape::BallNose => radius - (radius * radius - r * r).max(0.0).sqrt(),
            ToolShape::VBit => r / (self.angle.to_radians() / 2.0).tan(),
            ToolShape::RadiusedEndMill => {
                let tip = self.tip_radius.clamp(0.0, radius);
                let past_flat = (r - (radius - tip)).max(0.0);
                tip - (tip * tip - past_flat * past_flat).max(0.0).sqrt()
            }
        })
    }
}

impl Default for Tool {
    fn default() -> Self {
        Self {
            name: "6 mm End Mill".into(),
            shape: ToolShape::EndMill,
            tip_radius: default_tip_radius(),
            angle: default_angle(),
            diameter: 6.0,
            pass_depth: 3.0,
            stepover: 0.4,
            feed_rate: 1000.0,
            plunge_rate: 300.0,
            spindle_speed: 18000.0,
            flutes: default_flutes(),
        }
    }
}

/// The shared tools. A toolpath copies its tool, so editing the database never changes
/// toolpaths already made.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ToolDatabase {
    pub tools: Vec<Tool>,
}

impl ToolDatabase {
    /// What a first run starts with: common end mills, ball noses and V-bits.
    pub fn starter() -> Self {
        let end_mill =
            |name: &str, diameter: f64, pass_depth: f64, feed_rate: f64, plunge_rate: f64| Tool {
                name: name.into(),
                diameter,
                pass_depth,
                feed_rate,
                plunge_rate,
                ..Tool::default()
            };
        let shaped =
            |name: &str, shape: ToolShape, diameter: f64, angle: f64, stepover: f64| Tool {
                shape,
                angle,
                stepover,
                ..end_mill(name, diameter, 3.0, 1500.0, 500.0)
            };
        Self {
            tools: vec![
                end_mill("3 mm End Mill", 3.0, 1.5, 800.0, 250.0),
                Tool::default(),
                end_mill("12 mm End Mill", 12.0, 4.0, 2000.0, 500.0),
                end_mill("1/8 in End Mill", 3.175, 1.5, 800.0, 250.0),
                end_mill("1/4 in End Mill", 6.35, 3.0, 1000.0, 300.0),
                shaped("3 mm Ball Nose", ToolShape::BallNose, 3.0, 90.0, 0.1),
                shaped("6 mm Ball Nose", ToolShape::BallNose, 6.0, 90.0, 0.1),
                shaped(
                    "6 mm Radiused End Mill",
                    ToolShape::RadiusedEndMill,
                    6.0,
                    90.0,
                    0.2,
                ),
                // a V-bit's stepover spaces V-carving passes, so it is small
                shaped("90° V-Bit", ToolShape::VBit, 12.7, 90.0, 0.02),
                shaped("60° V-Bit", ToolShape::VBit, 12.7, 60.0, 0.02),
            ],
        }
    }

    pub fn get(&self, name: &str) -> Option<&Tool> {
        self.tools.iter().find(|t| t.name == name)
    }

    /// Replaces the tool with the same name, or adds it at the end.
    pub fn put(&mut self, tool: Tool) {
        match self.tools.iter_mut().find(|t| t.name == tool.name) {
            Some(existing) => *existing = tool,
            None => self.tools.push(tool),
        }
    }

    pub fn to_json(&self) -> Vec<u8> {
        serde_json::to_vec_pretty(self).expect("tools always serialize")
    }

    pub fn from_json(bytes: &[u8]) -> Result<Self, serde_json::Error> {
        serde_json::from_slice(bytes)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MoveKind {
    /// Full speed, clear of the material
    Rapid,
    /// Straight down into the material at the plunge rate
    Plunge,
    /// Cutting at the feed rate
    Cut,
}

/// A straight move to `to` (x, y in job millimetres; z relative to the material surface).
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Move {
    pub kind: MoveKind,
    pub to: DVec3,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Toolpath {
    pub name: String,
    /// Id of the command that calculates it, re-run with `params` to recalculate.
    pub strategy: String,
    pub params: serde_json::Value,
    pub tool: Tool,
    /// The vectors it was made from: recalculating re-runs the strategy on these.
    #[serde(default)]
    pub vectors: Vec<EntityId>,
    pub moves: Vec<Move>,
    /// Hidden toolpaths are not drawn or previewed (added after the first job format).
    #[serde(default = "shown")]
    pub visible: bool,
    /// A laser toolpath's power (percent of the laser's maximum) for each move; empty for
    /// a router or mill (added with lasers).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub power: Vec<f64>,
}

fn shown() -> bool {
    true
}

impl Toolpath {
    /// Estimated machining time in minutes: each move's length at its rate (rapids at
    /// `rapid_rate`, mm/min), from the first move on.
    // ponytail: no acceleration; Toolpath Summary's scale factor stands in for it, as Aspire's does
    pub fn machining_minutes(&self, rapid_rate: f64) -> f64 {
        self.moves
            .windows(2)
            .map(|pair| {
                let rate = match pair[1].kind {
                    MoveKind::Rapid => rapid_rate,
                    MoveKind::Plunge => self.tool.plunge_rate,
                    MoveKind::Cut => self.tool.feed_rate,
                };
                let length = pair[0].to.distance(pair[1].to);
                if rate > 0.0 { length / rate } else { 0.0 }
            })
            .sum()
    }
}

#[cfg(test)]
#[test]
fn machining_time_is_each_move_at_its_rate() {
    let at = |kind, x, z| Move {
        kind,
        to: DVec3::new(x, 0.0, z),
    };
    let tp = Toolpath {
        name: "t".into(),
        strategy: String::new(),
        params: serde_json::Value::Null,
        tool: Tool {
            feed_rate: 1000.0,
            plunge_rate: 100.0,
            ..Tool::default()
        },
        vectors: vec![],
        moves: vec![
            at(MoveKind::Rapid, 0.0, 5.0),
            at(MoveKind::Plunge, 0.0, -5.0),
            at(MoveKind::Cut, 500.0, -5.0),
            at(MoveKind::Rapid, 500.0, 5.0),
        ],
        visible: true,
        power: vec![],
    };
    // 10 mm plunge at 100, 500 mm cut at 1000, 10 mm rapid at 5000
    assert!((tp.machining_minutes(5000.0) - (0.1 + 0.5 + 0.002)).abs() < 1e-12);
}

/// Depth of each pass (negative z) down to `depth`, in equal steps no deeper than `pass_depth`.
pub fn pass_depths(depth: f64, pass_depth: f64) -> Vec<f64> {
    // ponytail: equal steps; a lighter final "finish" pass is a later option
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "a small positive count"
    )]
    let passes = (depth / pass_depth).ceil().max(1.0) as u32;
    let step = depth / f64::from(passes);
    (1..=passes)
        .map(|i| {
            if i == passes {
                -depth
            } else {
                -step * f64::from(i)
            }
        })
        .collect()
}

/// Cuts along `points` once per depth: rapid above the start, plunge, cut, and finally
/// retract. A closed contour plunges straight to its next pass; an open one retracts and
/// returns to its start first.
pub fn contour_moves(points: &[Point], closed: bool, depths: &[f64], safe_z: f64) -> Vec<Move> {
    let Some((&start, rest)) = points.split_first() else {
        return vec![];
    };
    let at = |p: Point, z: f64| DVec3::new(p.x, p.y, z);
    let end = if closed {
        start
    } else {
        *points.last().unwrap_or(&start)
    };
    let mut moves = vec![Move {
        kind: MoveKind::Rapid,
        to: at(start, safe_z),
    }];
    for (pass, &z) in depths.iter().enumerate() {
        if pass > 0 && !closed {
            moves.push(Move {
                kind: MoveKind::Rapid,
                to: at(end, safe_z),
            });
            moves.push(Move {
                kind: MoveKind::Rapid,
                to: at(start, safe_z),
            });
        }
        moves.push(Move {
            kind: MoveKind::Plunge,
            to: at(start, z),
        });
        moves.extend(rest.iter().map(|&p| Move {
            kind: MoveKind::Cut,
            to: at(p, z),
        }));
        if closed {
            moves.push(Move {
                kind: MoveKind::Cut,
                to: at(start, z),
            });
        }
    }
    moves.push(Move {
        kind: MoveKind::Rapid,
        to: at(end, safe_z),
    });
    moves
}

/// A toolpath stamped with the command that made it and its params, so it can be recalculated.
pub fn made_by<C: Command>(
    params: &C::Params,
    name: String,
    tool: Tool,
    vectors: Vec<EntityId>,
    moves: Vec<Move>,
) -> Toolpath {
    let params = serde_json::to_value(params).expect("command params must serialize");
    Toolpath {
        name,
        strategy: C::ID.into(),
        params,
        tool,
        vectors,
        moves,
        visible: true,
        power: vec![],
    }
}

/// The points to cut along: the path flattened to the polygon tolerance, without repeating
/// the start at the end of a closed path.
pub fn cut_points(path: &Path) -> Vec<Point> {
    let mut points = path.flatten(polygon::TOLERANCE);
    if path.closed && points.len() > 1 && points.first() == points.last() {
        points.pop();
    }
    points
}

/// Most times a move between two zigzag samples is halved to stay within the tolerance.
const HALVINGS: u32 = 6;

/// A zigzag of `tool` over `grid`: lines along x `step` apart, sampled at each cell centre,
/// each point as low as the tool goes there without touching `grid`, and never below `floor`.
/// Every move is checked: one that would cut more than `tolerance` into `grid` is halved, and
/// one too short to halve is lifted over. Points in the middle of level runs are dropped.
pub fn zigzag(grid: &Material, tool: &Tool, step: f64, tolerance: f64, floor: f64) -> Vec<DVec3> {
    zigzag_within(grid, tool, step, tolerance, floor, &|_| true).concat()
}

/// [`zigzag`] over only the points `inside` says: each line split into runs where it leaves
/// them, in cutting order (every other line reversed).
#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_precision_loss,
    reason = "line counts are small and non-negative"
)]
pub fn zigzag_within(
    grid: &Material,
    tool: &Tool,
    step: f64,
    tolerance: f64,
    floor: f64,
    inside: &dyn Fn(Point) -> bool,
) -> Vec<Vec<DVec3>> {
    let (first, last) = (
        grid.centre(0, 0),
        grid.centre(grid.columns - 1, grid.rows - 1),
    );
    let lines = ((last.y - first.y) / step).floor() as usize + 1;
    let mut runs: Vec<Vec<DVec3>> = vec![];
    for line in 0..lines {
        let y = first.y + line as f64 * step;
        let at = |x: f64| {
            let p = Point::new(x, y);
            p.extend(grid.drop_tool(tool, p).max(floor))
        };
        let mut line_runs: Vec<Vec<DVec3>> = vec![];
        let mut run: Vec<DVec3> = vec![];
        for column in 0..grid.columns {
            let x = grid.centre(column, 0).x;
            if !inside(Point::new(x, y)) {
                if !run.is_empty() {
                    line_runs.push(std::mem::take(&mut run));
                }
                continue;
            }
            let next = at(x);
            if let Some(&previous) = run.last() {
                Refine {
                    grid,
                    tool,
                    tolerance,
                    at: &at,
                }
                .between(previous, next, HALVINGS, &mut run);
            }
            run.push(next);
        }
        if !run.is_empty() {
            line_runs.push(run);
        }
        if line % 2 == 1 {
            line_runs.reverse();
            for run in &mut line_runs {
                run.reverse();
            }
        }
        for run in line_runs {
            let mut points: Vec<DVec3> = vec![];
            for point in run {
                let n = points.len();
                if n >= 2 && level_run(points[n - 2], points[n - 1], point) {
                    points.pop();
                }
                points.push(point);
            }
            runs.push(points);
        }
    }
    runs
}

/// Keeps the moves along one zigzag line from cutting into the grid.
struct Refine<'a, F> {
    grid: &'a Material,
    tool: &'a Tool,
    tolerance: f64,
    /// The point on the line at an x, as low as the tool goes there.
    at: &'a F,
}

impl<F: Fn(f64) -> DVec3> Refine<'_, F> {
    /// Points to put between `a` and `b` so the moves through them stay out of the grid.
    fn between(&self, a: DVec3, b: DVec3, halvings: u32, out: &mut Vec<DVec3>) {
        if self.grid.overcut(self.tool, a, b) <= self.tolerance {
            return;
        }
        if halvings == 0 {
            // too short to halve again: up, across level, and down
            let level = a.z.max(b.z);
            let across = |z: f64| (a.truncate().extend(z), b.truncate().extend(z));
            let (from, to) = across(level);
            let lifted = level + self.grid.overcut(self.tool, from, to).max(0.0);
            let (from, to) = across(lifted);
            out.extend([from, to]);
            return;
        }
        let middle = (self.at)(f64::midpoint(a.x, b.x));
        self.between(a, middle, halvings - 1, out);
        out.push(middle);
        self.between(middle, b, halvings - 1, out);
    }
}

/// Three points along one line at one height.
fn level_run(a: DVec3, b: DVec3, c: DVec3) -> bool {
    let same = |u: f64, v: f64| (u - v).abs() < 1e-9;
    same(a.y, b.y) && same(b.y, c.y) && same(a.z, b.z) && same(b.z, c.z)
}

/// Rapid to above the first point, plunge to it, cut through the rest, rapid back up.
pub fn follow(points: &[DVec3], safe_z: f64) -> Vec<Move> {
    let (Some(&first), Some(&last)) = (points.first(), points.last()) else {
        return vec![];
    };
    let above = |p: DVec3| p.truncate().extend(safe_z);
    let mut moves = vec![
        Move {
            kind: MoveKind::Rapid,
            to: above(first),
        },
        Move {
            kind: MoveKind::Plunge,
            to: first,
        },
    ];
    moves.extend(points[1..].iter().map(|&to| Move {
        kind: MoveKind::Cut,
        to,
    }));
    moves.push(Move {
        kind: MoveKind::Rapid,
        to: above(last),
    });
    moves
}

/// Project toolpath onto 3D Model: `moves` dropped in Z onto the job's 3D model, so a depth
/// below (or a lift above) the material surface becomes the same below (or above) the model's
/// surface, never below the material's bottom. Cuts are split into steps no longer than one of
/// the model's cells (sized for `tool`) so they follow it; rapids keep their height.
pub fn project_onto_model(
    doc: &Document,
    tool: &Tool,
    moves: &[Move],
) -> Result<Vec<Move>, CommandError> {
    let surface = placed_model(doc, tool)?;
    // the grid's far edges belong to its last cells, as does anything past them
    #[expect(clippy::cast_precision_loss, reason = "cell counts are small")]
    let far = surface.origin
        + Point::new(surface.columns as f64, surface.rows as f64) * surface.cell
        - Point::splat(1e-9);
    // ponytail: the tip follows the model's height under it, as the option is described; the
    // cutter is not dropped as a whole, so a wide tool's sides can touch a steep slope
    let drop = |p: DVec3| {
        let model = surface
            .at(p.truncate().clamp(surface.origin, far))
            .unwrap_or(0.0);
        p.truncate().extend((model + p.z).max(surface.bottom))
    };
    let mut projected = Vec::with_capacity(moves.len());
    let mut from: Option<DVec3> = None;
    for m in moves {
        match (m.kind, from) {
            (MoveKind::Rapid, _) => projected.push(*m),
            (kind, Some(a)) => {
                #[expect(
                    clippy::cast_possible_truncation,
                    clippy::cast_sign_loss,
                    reason = "a small positive count"
                )]
                let steps = (a.truncate().distance(m.to.truncate()) / surface.cell)
                    .ceil()
                    .max(1.0) as u32;
                projected.extend((1..=steps).map(|i| Move {
                    kind,
                    to: drop(a.lerp(m.to, f64::from(i) / f64::from(steps))),
                }));
            }
            (kind, None) => projected.push(Move {
                kind,
                to: drop(m.to),
            }),
        }
        from = Some(m.to);
    }
    Ok(projected)
}

/// Parallel lines `step` apart at `angle` degrees from the x axis, clipped to `regions`
/// (holes left out): each row's pieces as (start, end), rows in order across the regions.
pub fn hatch_lines(regions: &[Path], step: f64, angle: f64) -> Vec<Vec<(Point, Point)>> {
    let turn = Point::from_angle(angle.to_radians());
    // work with the rows along x: turn the shapes the other way, and back at the end
    let level = |p: Point| Point::new(turn.x * p.x + turn.y * p.y, turn.x * p.y - turn.y * p.x);
    let rings: Vec<Vec<Point>> = regions
        .iter()
        .map(|r| cut_points(r).into_iter().map(level).collect())
        .collect();
    let (low, high) = rings
        .iter()
        .flatten()
        .fold((f64::INFINITY, f64::NEG_INFINITY), |(l, h), p| {
            (l.min(p.y), h.max(p.y))
        });
    let mut rows = vec![];
    let mut y = low + step / 2.0;
    while y < high {
        let mut xs: Vec<f64> = rings
            .iter()
            .flat_map(|ring| {
                let n = ring.len();
                (0..n).filter_map(move |i| {
                    let (a, b) = (ring[i], ring[(i + 1) % n]);
                    ((a.y <= y) != (b.y <= y)).then(|| a.x + (y - a.y) / (b.y - a.y) * (b.x - a.x))
                })
            })
            .collect();
        xs.sort_by(f64::total_cmp);
        rows.push(
            xs.as_chunks::<2>()
                .0
                .iter()
                .map(|&[a, b]| (turn.rotate(Point::new(a, y)), turn.rotate(Point::new(b, y))))
                .collect(),
        );
        y += step;
    }
    rows
}

#[cfg(test)]
mod projection {
    use super::*;
    use crate::testing::domed;

    /// A cut `depth` deep along y = 20 over the middle of the dome, from flat to flat.
    fn across(depth: f64) -> Vec<Move> {
        let at = |kind, x: f64, z: f64| Move {
            kind,
            to: DVec3::new(x, 20.0, z),
        };
        vec![
            at(MoveKind::Rapid, 2.0, 5.0),
            at(MoveKind::Plunge, 2.0, -depth),
            at(MoveKind::Cut, 38.0, -depth),
            at(MoveKind::Rapid, 38.0, 5.0),
        ]
    }

    /// The dome's surface along that line: 8 mm tall with its top at the material surface.
    fn dome(x: f64) -> f64 {
        let r = (x - 20.0).abs();
        if r < 8.0 {
            (64.0 - r * r).sqrt() - 8.0
        } else {
            -8.0
        }
    }

    fn cuts(moves: &[Move]) -> Vec<DVec3> {
        moves
            .iter()
            .filter(|m| m.kind == MoveKind::Cut)
            .map(|m| m.to)
            .collect()
    }

    #[test]
    fn a_cut_follows_the_model_at_its_depth_below_it() {
        let moved = project_onto_model(&domed(), &Tool::default(), &across(1.0)).unwrap();
        let cuts = cuts(&moved);
        // split into steps no longer than a model cell, a twentieth of the 6 mm tool
        assert!(cuts.len() > 100, "{} pieces", cuts.len());
        assert!(
            cuts.windows(2)
                .all(|w| w[0].truncate().distance(w[1].truncate()) <= 0.3 + 1e-9)
        );
        for p in &cuts {
            // off the true dome by no more than its slope across a cell
            assert!(
                (p.z - (dome(p.x) - 1.0)).abs() < 0.5,
                "at x = {} cut at {}, the model at {}",
                p.x,
                p.z,
                dome(p.x)
            );
        }
        assert!(
            cuts.iter().any(|p| (p.z + 1.0).abs() < 0.05),
            "over the top"
        );
        // right on the material's far edge too
        let edge = [Move {
            kind: MoveKind::Plunge,
            to: DVec3::new(40.0, 40.0, -1.0),
        }];
        let dropped = project_onto_model(&domed(), &Tool::default(), &edge).unwrap();
        assert!((dropped[0].to.z + 9.0).abs() < 0.01, "{}", dropped[0].to.z);
        // rapids keep their height above the material
        assert!((moved.first().unwrap().to.z - 5.0).abs() < 1e-12);
        assert!((moved.last().unwrap().to.z - 5.0).abs() < 1e-12);
    }

    #[test]
    fn it_never_goes_below_the_material_bottom() {
        // 3 mm below the flat 8 mm down would be 11 mm into 10 mm thick material
        let moved = project_onto_model(&domed(), &Tool::default(), &across(3.0)).unwrap();
        let lowest = moved.iter().map(|m| m.to.z).fold(f64::INFINITY, f64::min);
        assert!((lowest + 10.0).abs() < 1e-9, "lowest {lowest}");
        let top = cuts(&moved)
            .into_iter()
            .find(|p| (p.x - 20.0).abs() < 0.2)
            .unwrap();
        assert!((top.z + 3.0).abs() < 0.05, "over the top at {}", top.z);
    }

    #[test]
    fn it_needs_a_model() {
        let why = project_onto_model(&Document::default(), &Tool::default(), &across(1.0))
            .unwrap_err()
            .to_string();
        assert!(why.contains("3D model"), "{why}");
    }
}
