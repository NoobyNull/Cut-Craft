//! Toolpaths: the tool, the strategy command that made them (so they can be recalculated),
//! and the computed moves.

use crate::component::placed_model;
use crate::document::{Document, EntityId};
use crate::geometry::{Path, Point};
use crate::simulation::Material;
use crate::{Command, CommandError};
use glam::DVec3;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
mod database;
pub use database::*;
mod tool;
pub use tool::*;

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
    #[serde(with = "crate::packed::moves")]
    pub moves: Vec<Move>,
    /// Hidden toolpaths are not drawn or previewed (added after the first job format).
    #[serde(default = "shown")]
    pub visible: bool,
    /// A laser toolpath's power (percent of the laser's maximum) for each move; empty for
    /// a router or mill (added with lasers).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub power: Vec<f64>,
    /// What it was made from has changed since (its vectors, or the model it cuts):
    /// recalculating makes it fresh again.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub stale: bool,
    /// The toolpath group it is listed under, if any (added with toolpath groups).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub group: Option<String>,
    /// A drill bank toolpath's bank and the plunges of its head; its moves drill each hole
    /// one after another (for the preview), and the post writes one plunge for each hit
    /// (added with drill banks).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub drill_bank: Option<crate::drill_bank::BankToolpath>,
    /// Where its entries went down another way than its settings asked, there being no
    /// room there (added with entry modes).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub fallbacks: Vec<crate::entry::Fallback>,
    /// A rotary toolpath that holds the bit off the turning axis: how far across from it the
    /// bit is for each move, mm (positive the way the material's surface turns away from A=0).
    /// Its moves are still where the tip cuts on the unrolled material, so the preview shows
    /// them true; the post works out A, the across axis and Z from both
    /// ([`crate::rotary_axis::held_at`]). Empty when the bit stays over the axis (added with
    /// off-centre and indexed rotary cutting).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub across: Vec<f64>,
}

/// The strategies that always cut the 3D model; others do when projected onto it.
const MODEL_STRATEGIES: [&str; 4] = [
    "3d_roughing_toolpath",
    "3d_finishing_toolpath",
    "3d_pencil_toolpath",
    "sketch_carving_toolpath",
];

/// The key a toolpath's own preview colour is kept under in its params: `[r, g, b]`, or
/// null for no fill (the material's colour).
pub const PREVIEW_COLOUR: &str = "preview_colour";

/// The key a toolpath's notes for the Toolpath Summary are kept under in its params.
pub const SUMMARY_NOTES: &str = "summary_notes";

/// What a toolpath keeps in its params that is its own, not one of its settings: kept when
/// it is recalculated.
pub const OWN_PARAMS: [&str; 2] = [PREVIEW_COLOUR, SUMMARY_NOTES];

/// The colours toolpaths take in a Toolpath Color preview until given their own, in list
/// order.
const PREVIEW_PALETTE: [[u8; 3]; 8] = [
    [220, 60, 60],
    [60, 140, 220],
    [70, 170, 80],
    [230, 170, 40],
    [150, 80, 200],
    [40, 180, 180],
    [230, 110, 170],
    [130, 130, 130],
];

fn shown() -> bool {
    true
}

impl Toolpath {
    /// Its notes for the Toolpath Summary.
    pub fn summary_notes(&self) -> &str {
        self.params
            .get(SUMMARY_NOTES)
            .and_then(serde_json::Value::as_str)
            .unwrap_or_default()
    }

    /// Its colour in a Toolpath Color preview, `place` being where it is in the list: the
    /// one it was given, none if it was given no fill, else the palette's for its place.
    pub fn preview_colour(&self, place: usize) -> Option<[u8; 3]> {
        match self.params.get(PREVIEW_COLOUR) {
            Some(given) => serde_json::from_value(given.clone()).ok(),
            None => Some(PREVIEW_PALETTE[place % PREVIEW_PALETTE.len()]),
        }
    }

    /// Whether the 3D model is what it cuts, so changing the model changes it.
    pub fn reads_model(&self) -> bool {
        MODEL_STRATEGIES.contains(&self.strategy.as_str())
            || self.params["project_onto_model"].as_bool() == Some(true)
    }

    /// Whether the job's Clearing Settings shape it: adaptive and raster pockets, and
    /// anything cut in trochoidal loops.
    pub fn reads_clearing(&self) -> bool {
        let pocket = self.strategy == "pocket_toolpath"
            && matches!(
                self.params["strategy"].as_str(),
                Some("adaptive" | "raster")
            );
        pocket || self.params["motion"].as_str() == Some("trochoidal")
    }

    /// Estimated machining time in minutes: each move's length at its rate (rapids at
    /// `rapid_rate`, mm/min), from the first move on.
    // ponytail: no acceleration; see crate::cut_time for the full estimate
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
fn nothing_beyond_the_reach_below_a_height_is_that_low() {
    for shape in [
        ToolShape::EndMill,
        ToolShape::BallNose,
        ToolShape::VBit,
        ToolShape::RadiusedEndMill,
    ] {
        let tool = Tool {
            shape,
            diameter: 12.0,
            angle: 60.0,
            tip_radius: 2.0,
            ..Tool::default()
        };
        for height in [0.0, 0.1, 0.5, 1.5, 3.0, 6.0, 20.0] {
            let reach = tool.reach_below(height);
            assert!(reach <= tool.reach() + 1e-12, "{shape:?} {height}");
            // just past it the tool's edge is above `height`, or past the tool altogether
            for out in [1e-6, 0.01, 0.5, 3.0] {
                if let Some(edge) = tool.profile(reach + out) {
                    assert!(edge > height - 1e-9, "{shape:?} {height} {out}: {edge}");
                }
            }
        }
    }
    let v = Tool {
        shape: ToolShape::VBit,
        diameter: 12.0,
        angle: 60.0,
        ..Tool::default()
    };
    assert!(
        v.reach_below(1.0) < 1.0,
        "a shallow V-carve touches little: {}",
        v.reach_below(1.0)
    );
}

#[cfg(test)]
#[test]
fn passes_set_by_hand_never_leave_a_step_deeper_than_the_pass_depth() {
    let tool = Tool {
        pass_depth: 3.0,
        passes: vec![2.0, 4.0, 6.0],
        passes_for: 6.0,
        ..Tool::default()
    };
    // as set, down to the depth they were set for
    assert_eq!(tool.pass_depths(6.0), [-2.0, -4.0, -6.0]);
    // the cut made deeper later: what is left below them goes in pass-depth steps, not
    // one 12 mm pass
    let deeper = tool.pass_depths(18.0);
    assert_eq!(&deeper[..3], [-2.0, -4.0, -6.0]);
    assert_eq!(*deeper.last().unwrap(), -18.0);
    let steps: Vec<f64> = deeper.windows(2).map(|w| w[0] - w[1]).collect();
    assert!(steps.iter().all(|&s| s <= 3.0 + 1e-9), "{deeper:?}");
    // one hand-set pass at 0.5 over an 8 mm V-carve fill
    let shallow = Tool {
        pass_depth: 2.0,
        passes: vec![0.5],
        passes_for: 1.0,
        ..Tool::default()
    };
    let fill = shallow.pass_depths(8.0);
    assert_eq!(fill[0], -0.5);
    assert!(
        fill.windows(2).all(|w| w[0] - w[1] <= 2.0 + 1e-9),
        "{fill:?}"
    );
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
        stale: false,
        group: None,
        drill_bank: None,
        fallbacks: vec![],
        across: vec![],
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
        stale: false,
        group: None,
        drill_bank: None,
        fallbacks: vec![],
        across: vec![],
    }
}

/// The points to cut along: the path flattened to the polygon tolerance, without repeating
/// the start at the end of a closed path.
pub fn cut_points(path: &Path) -> Vec<Point> {
    let mut points = path.flatten(crate::accuracy::curve());
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
    inside: &(dyn Fn(Point) -> bool + Sync),
) -> Vec<Vec<DVec3>> {
    let (first, last) = (
        grid.centre(0, 0),
        grid.centre(grid.columns - 1, grid.rows - 1),
    );
    let lines = ((last.y - first.y) / step).floor() as usize + 1;
    let drop = Drop {
        grid,
        tool,
        tolerance,
        floor,
        ceiling: f64::INFINITY,
        over_ceiling: false,
    };
    let lines: Vec<Vec<Point>> = (0..lines)
        .map(|line| {
            let y = first.y + line as f64 * step;
            let mut ends = vec![Point::new(first.x, y), Point::new(last.x, y)];
            if line % 2 == 1 {
                ends.reverse();
            }
            ends
        })
        .collect();
    drop.along_all(&lines, inside)
}

/// How to drop a tool along lines over a grid: never below `floor`, every move checked to
/// cut no more than `tolerance` into the grid, and runs split where the tool would sit above
/// `ceiling` (a Z level's flat cut stops where the model stands above it).
#[derive(Clone, Copy)]
pub struct Drop<'a> {
    pub grid: &'a Material,
    pub tool: &'a Tool,
    pub tolerance: f64,
    pub floor: f64,
    pub ceiling: f64,
    /// Runs split at the ceiling keep the point just over it at each end, so the move up to
    /// it is still cut.
    pub over_ceiling: bool,
}

impl Drop<'_> {
    /// The tool along the polyline `line`, sampled about a cell apart, as low as it goes at
    /// each point; split into runs where `inside` says no or it would be above the ceiling.
    /// Points in the middle of level runs are left out.
    pub fn along(
        &self,
        line: &[Point],
        inside: &(dyn Fn(Point) -> bool + Sync),
    ) -> Vec<Vec<DVec3>> {
        self.along_all(&[line.to_vec()], inside)
    }

    /// [`Self::along`] each of `lines`, in order, the heavy work in batches over every line
    /// at once (on the GPU when there is one): the drops at the samples, then each round of
    /// refinement's gouge checks and midpoints.
    pub fn along_all(
        &self,
        lines: &[Vec<Point>],
        inside: &(dyn Fn(Point) -> bool + Sync),
    ) -> Vec<Vec<DVec3>> {
        let samples: Vec<Vec<(Point, bool)>> = lines
            .iter()
            .map(|line| {
                self.samples(line)
                    .into_iter()
                    .map(|p| (p, inside(p)))
                    .collect()
            })
            .collect();
        let wanted: Vec<Point> = samples
            .iter()
            .flatten()
            .filter_map(|&(p, inside)| inside.then_some(p))
            .collect();
        let dropped = crate::compute::compute().drop_at(self.grid, self.tool, &wanted);
        let mut dropped = dropped.into_iter();
        let mut gaps: Vec<(DVec3, DVec3)> = vec![];
        let mut gap = |a, b, run: &mut Vec<Piece>| {
            run.push(Piece::Gap(gaps.len()));
            gaps.push((a, b));
        };
        let mut runs: Vec<Vec<Piece>> = vec![];
        for line in &samples {
            let mut run: Vec<Piece> = vec![];
            // the last point passed over for being above the ceiling, while still inside
            let mut over: Option<DVec3> = None;
            for &(p, inside) in line {
                let point = inside.then(|| {
                    let z = dropped.next().expect("a drop for each sample inside");
                    p.extend(z.max(self.floor))
                });
                let next = point.filter(|q| q.z <= self.ceiling + 1e-9);
                let Some(next) = next else {
                    if let (true, Some(up), Some(&Piece::Point(previous))) =
                        (self.over_ceiling, point, run.last())
                    {
                        gap(previous, up, &mut run);
                        run.push(Piece::Point(up));
                    }
                    if !run.is_empty() {
                        runs.push(std::mem::take(&mut run));
                    }
                    over = point;
                    continue;
                };
                if let (true, Some(up), true) = (self.over_ceiling, over, run.is_empty()) {
                    run.push(Piece::Point(up));
                }
                over = None;
                if let Some(&Piece::Point(previous)) = run.last() {
                    gap(previous, next, &mut run);
                }
                run.push(Piece::Point(next));
            }
            if !run.is_empty() {
                runs.push(run);
            }
        }
        let fills = self.refine_all(&gaps, HALVINGS);
        runs.into_iter()
            .map(|run| {
                tidy(
                    run.into_iter()
                        .flat_map(|piece| match piece {
                            Piece::Point(p) => vec![p],
                            Piece::Gap(i) => fills[i].clone(),
                        })
                        .collect(),
                )
            })
            .collect()
    }

    /// The points along the polyline `line`, about a cell apart.
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "a small positive count"
    )]
    fn samples(&self, line: &[Point]) -> Vec<Point> {
        let mut samples = line.first().copied().into_iter().collect::<Vec<_>>();
        for w in line.windows(2) {
            let steps = (w[0].distance(w[1]) / self.grid.cell - 1e-9)
                .ceil()
                .max(1.0) as u32;
            samples.extend((1..=steps).map(|i| w[0].lerp(w[1], f64::from(i) / f64::from(steps))));
        }
        samples
    }

    /// The points to put in each of `gaps` (a move from one dropped point to the next) so
    /// the moves through them stay out of the grid: none where the move already does, else
    /// the tool dropped at the middle and the two halves refined again, `halvings` deep,
    /// then up, across level and down.
    fn refine_all(&self, gaps: &[(DVec3, DVec3)], halvings: u32) -> Vec<Vec<DVec3>> {
        let mut fills = vec![vec![]; gaps.len()];
        if gaps.is_empty() {
            return fills;
        }
        let compute = crate::compute::compute();
        let over = compute.overcut_along(self.grid, self.tool, gaps);
        let gouging: Vec<usize> = (0..gaps.len())
            .filter(|&i| over[i] > self.tolerance)
            .collect();
        if gouging.is_empty() {
            return fills;
        }
        let across = |i: usize, z: f64| {
            (
                gaps[i].0.truncate().extend(z),
                gaps[i].1.truncate().extend(z),
            )
        };
        if halvings == 0 {
            // too short to halve again: up, across level, and down
            let level = |i: usize| gaps[i].0.z.max(gaps[i].1.z);
            let flats: Vec<(DVec3, DVec3)> = gouging.iter().map(|&i| across(i, level(i))).collect();
            let lifted = compute.overcut_along(self.grid, self.tool, &flats);
            for (&i, lift) in gouging.iter().zip(lifted) {
                let (from, to) = across(i, level(i) + lift.max(0.0));
                fills[i] = vec![from, to];
            }
            return fills;
        }
        let middles: Vec<Point> = gouging
            .iter()
            .map(|&i| (gaps[i].0.truncate() + gaps[i].1.truncate()) / 2.0)
            .collect();
        let middles: Vec<DVec3> = middles
            .iter()
            .zip(compute.drop_at(self.grid, self.tool, &middles))
            .map(|(p, z)| p.extend(z.max(self.floor)))
            .collect();
        let halves: Vec<(DVec3, DVec3)> = gouging
            .iter()
            .zip(&middles)
            .flat_map(|(&i, &middle)| [(gaps[i].0, middle), (middle, gaps[i].1)])
            .collect();
        let mut parts = self.refine_all(&halves, halvings - 1).into_iter();
        for (&i, &middle) in gouging.iter().zip(&middles) {
            let (first, second) = (
                parts.next().expect("a fill per half"),
                parts.next().expect("a fill per half"),
            );
            fills[i].extend(first);
            fills[i].push(middle);
            fills[i].extend(second);
        }
        fills
    }
}

/// A run of a line being dropped: a point, or a gap between two still to be refined.
enum Piece {
    Point(DVec3),
    Gap(usize),
}

/// Leaves out the middle points of straight level runs.
fn tidy(run: Vec<DVec3>) -> Vec<DVec3> {
    let mut points: Vec<DVec3> = vec![];
    for point in run {
        let n = points.len();
        if n >= 2 && level_run(points[n - 2], points[n - 1], point) {
            points.pop();
        }
        points.push(point);
    }
    points
}

/// Three points along one straight line at one height.
fn level_run(a: DVec3, b: DVec3, c: DVec3) -> bool {
    let same = |u: f64, v: f64| (u - v).abs() < 1e-9;
    let (first, second) = (b.truncate() - a.truncate(), c.truncate() - b.truncate());
    same(first.perp_dot(second), 0.0) && first.dot(second) > 0.0 && same(a.z, b.z) && same(b.z, c.z)
}

/// Raster lines `step` apart at `angle` (degrees from X) across `regions`, every other one
/// back the other way, as lines to drop a tool along.
pub fn raster_lines(regions: &[Path], step: f64, angle: f64) -> Vec<Vec<Point>> {
    hatch_lines(regions, step, angle)
        .into_iter()
        .enumerate()
        .flat_map(|(row, spans)| {
            let mut spans: Vec<Vec<Point>> = spans.into_iter().map(|(a, b)| vec![a, b]).collect();
            if row % 2 == 1 {
                spans.reverse();
                for span in &mut spans {
                    span.reverse();
                }
            }
            spans
        })
        .collect()
}

/// [`follow`] for each run in turn, but staying down from the end of one run to the start
/// of the next when they are no more than `reach` apart: lifted by `lift` over the gap (a
/// stepover retract), when `clear` says each move of that keeps out of the work.
pub fn follow_linked(
    runs: &[Vec<DVec3>],
    safe_z: f64,
    reach: f64,
    lift: f64,
    clear: &dyn Fn(DVec3, DVec3) -> bool,
) -> Vec<Move> {
    let mut moves = vec![];
    let mut last: Option<DVec3> = None;
    for run in runs.iter().filter(|r| !r.is_empty()) {
        let first = run[0];
        let link = last
            .filter(|end| end.truncate().distance(first.truncate()) <= reach)
            .and_then(|end| {
                let (up, over) = (end + DVec3::Z * lift, first + DVec3::Z * lift);
                (clear(end, up) && clear(up, over) && clear(over, first)).then_some([up, over])
            });
        if let Some(link) = link {
            // the run before ended with its rapid up: stay down instead
            moves.pop();
            if lift > 0.0 {
                moves.extend(link.map(|to| Move {
                    kind: MoveKind::Cut,
                    to,
                }));
            }
            moves.push(Move {
                kind: MoveKind::Cut,
                to: first,
            });
            moves.extend(run[1..].iter().map(|&to| Move {
                kind: MoveKind::Cut,
                to,
            }));
            moves.push(Move {
                kind: MoveKind::Rapid,
                to: run[run.len() - 1].truncate().extend(safe_z),
            });
        } else {
            moves.extend(follow(run, safe_z));
        }
        last = run.last().copied();
    }
    moves
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

#[cfg(test)]
mod renamed_tools {
    use super::*;

    #[test]
    fn a_rename_is_followed_to_the_last_name_and_not_to_a_new_bit_of_the_old_name() {
        let mut db = ToolDatabase::starter();
        let six = db.get("6 mm End Mill").unwrap().clone();
        let at = db.index_of("6 mm End Mill").unwrap();
        db.tools[at].name = "Spiral".into();
        db.note_renamed("6 mm End Mill", "Spiral");
        db.tools[at].name = "Spiral 6".into();
        db.note_renamed("Spiral", "Spiral 6");
        assert_eq!(db.renamed_now(&six), Some("Spiral 6"));
        // a different bit under the old name is not the renamed one
        let mut other = six.clone();
        other.diameter = 3.0;
        assert_eq!(db.renamed_now(&other), None);
        // renamed back: its own name is not renamed, the names between lead back to it
        db.tools[at].name = "6 mm End Mill".into();
        db.note_renamed("Spiral 6", "6 mm End Mill");
        assert_eq!(db.renamed_now(&six), None);
        assert!(
            !db.renamed.contains_key("6 mm End Mill"),
            "{:?}",
            db.renamed
        );
        assert_eq!(db.renamed["Spiral"], "6 mm End Mill");
    }
}
