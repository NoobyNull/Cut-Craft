//! Pocket Toolpath: clears the inside of the selected closed vectors at each depth, with
//! rings that step in from the outline (worked from the middle outwards) or with raster
//! lines and a profile pass round the edge, or trochoidally: the tool circling in small loops
//! as it moves on, so it never cuts with more than a sliver of its edge.

use cut_craft_core::document::Document;
use cut_craft_core::entry::{Direction, Entry, Ramp, Ramping, path_moves};
use cut_craft_core::geometry::{Path, Point};
use cut_craft_core::polygon::{self, Corners};
use cut_craft_core::toolpath::{Move, Tool, cut_points, hatch_lines, made_by, project_onto_model};
use cut_craft_core::{Change, Command, CommandError, Meta, ensure_positive};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub struct PocketToolpath;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Strategy {
    /// Rings stepping in from the outline
    #[default]
    Offset,
    /// Straight lines across, at the raster angle
    Raster,
    /// Small loops moving on round bands from the outline in, then a pass round the edge:
    /// a light cut at full depth, for hard material or a small machine
    Trochoidal,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ProfilePass {
    /// Round the edge before the raster lines
    First,
    /// Round the edge after them
    #[default]
    Last,
    /// No pass round the edge: it is left rough
    None,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
pub struct Params {
    pub name: String,
    /// Below the material surface
    #[schemars(extend("x-unit" = "mm"), range(min = 0))]
    #[schemars(title = "Cut Depth", extend("x-group" = "Cutting Depths"))]
    pub depth: f64,
    pub tool: Tool,
    /// Bigger tools that clear the pocket first, largest first: each makes a toolpath of its
    /// own and each tool after it cuts only what the ones before it left (rest machining)
    #[serde(default)]
    #[schemars(title = "Clearance Tools")]
    pub clearance_tools: Vec<Tool>,
    #[serde(default)]
    #[schemars(extend("x-group" = "Clear Pocket"))]
    pub strategy: Strategy,
    /// Which way round the rings and the profile pass run
    #[serde(default)]
    #[schemars(extend("x-group" = "Clear Pocket"))]
    pub direction: Direction,
    /// Left on the walls; negative cuts past the vectors
    #[serde(default)]
    #[schemars(title = "Pocket Allowance", extend("x-unit" = "mm"))]
    #[schemars(extend("x-group" = "Clear Pocket"))]
    pub allowance: f64,
    /// 0° along X, 90° along Y
    #[serde(default)]
    #[schemars(
        extend("x-unit" = "°", "x-show-when" = {"strategy": "raster"}),
        range(min = 0, max = 90)
    )]
    #[schemars(extend("x-group" = "Clear Pocket"))]
    pub raster_angle: f64,
    #[serde(default)]
    #[schemars(extend("x-show-when" = {"strategy": "raster"}))]
    #[schemars(extend("x-group" = "Clear Pocket"))]
    pub profile_pass: ProfilePass,
    /// How wide each loop is, as a share of the tool's diameter
    #[serde(default = "loop_width")]
    #[schemars(
        title = "Loop Width",
        extend("x-unit" = "%", "x-show-when" = {"strategy": "trochoidal"}),
        range(min = 10, max = 400)
    )]
    #[schemars(extend("x-group" = "Clear Pocket"))]
    pub loop_width: f64,
    /// How far each loop moves on from the last, as a share of the tool's diameter: less is
    /// a lighter cut and a longer toolpath
    #[serde(default = "loop_step")]
    #[schemars(
        title = "Loop Step",
        extend("x-unit" = "%", "x-show-when" = {"strategy": "trochoidal"}),
        range(min = 1, max = 100)
    )]
    #[schemars(extend("x-group" = "Clear Pocket"))]
    pub loop_step: f64,
    /// Zig zag down over the ramp distance instead of plunging straight down
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
    /// Drop the finished toolpath onto the 3D model: its depth below the material surface
    /// becomes depth below the model's surface, going no lower than the bottom of the material
    #[serde(default)]
    #[schemars(title = "Project toolpath onto 3D Model")]
    pub project_onto_model: bool,
}

fn loop_width() -> f64 {
    100.0
}

fn loop_step() -> f64 {
    10.0
}

fn ramp_distance() -> f64 {
    Ramping::default().distance
}

impl Default for Params {
    fn default() -> Self {
        Self {
            name: "Pocket".into(),
            depth: 3.0,
            tool: Tool::default(),
            clearance_tools: vec![],
            strategy: Strategy::Offset,
            direction: Direction::Climb,
            allowance: 0.0,
            raster_angle: 0.0,
            profile_pass: ProfilePass::Last,
            loop_width: loop_width(),
            loop_step: loop_step(),
            ramp_plunge_moves: false,
            ramp_distance: ramp_distance(),
            project_onto_model: false,
        }
    }
}

impl Command for PocketToolpath {
    const ID: &'static str = "pocket_toolpath";
    type Params = Params;

    fn meta() -> Meta {
        Meta {
            label: "Pocket Toolpath",
            group: "Toolpath Operations",
            icon: "toolpaths/pocket_toolpath.png",
            shortcut: "",
        }
    }

    fn run(doc: &Document, p: &Params) -> Result<Change, CommandError> {
        let mut c = Change::builder(doc);
        if p.clearance_tools.is_empty() {
            c.add_toolpath(pocket(doc, p)?);
            return Ok(c.finish());
        }
        // each tool, largest first, then the pocket's own; a raster only for the first
        let tools: Vec<&Tool> = p.clearance_tools.iter().chain([&p.tool]).collect();
        for (i, &tool) in tools.iter().enumerate() {
            let own = Params {
                name: format!("{} [{}]", p.name, i + 1),
                tool: tool.clone(),
                clearance_tools: vec![],
                strategy: if i == 0 { p.strategy } else { Strategy::Offset },
                ..p.clone()
            };
            let mut toolpath = pocket(doc, &own)?;
            if i > 0 {
                let rest = cut_craft_core::rest::Rest {
                    after: tools[..i].iter().map(|&t| t.clone()).collect(),
                    ..Default::default()
                };
                toolpath.params["rest"] =
                    serde_json::to_value(&rest).expect("rest settings serialise");
                let params = toolpath.params.clone();
                cut_craft_core::rest::apply(&doc.job, &mut toolpath, &params)?;
            }
            c.add_toolpath(toolpath);
        }
        Ok(c.finish())
    }
}

/// The pocket for `p`'s own tool.
fn pocket(doc: &Document, p: &Params) -> Result<cut_craft_core::toolpath::Toolpath, CommandError> {
    {
        ensure_positive("depth", p.depth)?;
        ensure_positive("tool diameter", p.tool.diameter)?;
        ensure_positive("pass depth", p.tool.pass_depth)?;
        if !(p.tool.stepover > 0.0 && p.tool.stepover <= 1.0) {
            return Err(CommandError::InvalidParams(
                "stepover must be more than 0 and at most 1".into(),
            ));
        }
        let (ids, paths) = doc.selected_paths(|path| path.closed);
        if ids.is_empty() {
            return Err(CommandError::Failed(
                "select the closed vectors to pocket".into(),
            ));
        }
        // ponytail: each selected shape is a filled region, so islands (shapes inside shapes)
        // are pocketed over; nest-aware regions when islands are needed
        let ring = polygon::offset(
            &polygon::union(&paths),
            -(p.tool.diameter / 2.0 + p.allowance),
            Corners::Round,
        );
        if ring.is_empty() {
            return Err(CommandError::Failed(
                "the tool is too big to fit inside the selected shapes".into(),
            ));
        }
        let step = p.tool.stepover * p.tool.diameter;
        let edge = ring.clone();
        let rings = rings_in(ring, step);
        let mut loops = vec![];
        if p.strategy == Strategy::Trochoidal {
            loops = trochoidal(&edge, p);
            if loops.is_empty() {
                loops.clone_from(&rings); // too narrow for loops: rings
            }
        }
        let safe = doc.job.clearance;
        let entry = Entry {
            ramp: if p.ramp_plunge_moves {
                Ramp::ZigZag
            } else {
                Ramp::None
            },
            ramping: Ramping {
                distance: p.ramp_distance,
                ..Ramping::default()
            },
            ..Entry::default()
        };
        // each path at depth `z`, ramping down from the level above; `back` runs it the other
        // way (Climb + Conventional's every other pass)
        let cut = |paths: &[Path], z: f64, above: f64, back: bool| -> Vec<Move> {
            paths
                .iter()
                .flat_map(|r| {
                    let mut points = if r.closed {
                        p.direction.orient(cut_points(r), true)
                    } else {
                        cut_points(r)
                    };
                    if back {
                        points.reverse();
                    }
                    path_moves(&points, r.closed, &[z], above, safe, &entry)
                })
                .collect()
        };
        // ponytail: each ring or line retracts between cuts; link them with a stepover move
        // to save time
        let mut moves = vec![];
        let mut above = 0.0;
        for (pass, z) in p.tool.pass_depths(p.depth).into_iter().enumerate() {
            let back = p.direction == Direction::Both && pass % 2 == 1;
            let cut = |paths: &[Path]| cut(paths, z, above, back);
            match p.strategy {
                Strategy::Offset => moves.extend(cut(&rings)),
                Strategy::Trochoidal => {
                    moves.extend(cut(&loops));
                    moves.extend(cut(&edge));
                }
                Strategy::Raster => {
                    if p.profile_pass == ProfilePass::First {
                        moves.extend(cut(&edge));
                    }
                    let lines = raster(&edge, step, p.raster_angle);
                    moves.extend(cut(&lines));
                    if p.profile_pass == ProfilePass::Last {
                        moves.extend(cut(&edge));
                    }
                }
            }
            above = z;
        }
        if p.project_onto_model {
            moves = project_onto_model(doc, &p.tool, &moves)?;
        }
        Ok(made_by::<PocketToolpath>(
            p,
            p.name.clone(),
            p.tool.clone(),
            ids,
            moves,
        ))
    }
}

/// Rings `step` apart from `ring` in, worked from the middle outwards.
fn rings_in(mut ring: Vec<Path>, step: f64) -> Vec<Path> {
    let mut rings = vec![];
    while !ring.is_empty() {
        let next = polygon::offset(&ring, -step, Corners::Round);
        rings.extend(ring);
        ring = next;
    }
    rings.reverse();
    rings
}

/// The trochoidal clearing of the region inside `edge` (where the tool's middle may go):
/// looping bands from the outline in, each band's loops reaching from the one outside it to
/// the one inside, then plain rings for a middle too small for a band.
fn trochoidal(edge: &[Path], p: &Params) -> Vec<Path> {
    let d = p.tool.diameter;
    let radius = d * p.loop_width / 200.0;
    let advance = (d * p.loop_step / 100.0).max(0.01);
    let step = p.tool.stepover * d;
    let mut out = vec![];
    let mut band = polygon::offset(edge, -radius, Corners::Round);
    let mut last = vec![];
    while !band.is_empty() {
        for ring in &band {
            let ring = p.direction.orient(cut_points(ring), true);
            let points = looped(
                &ring,
                radius,
                advance,
                p.direction != Direction::Conventional,
            );
            out.push(Path::polyline(&points, false));
        }
        last = band;
        band = polygon::offset(&last, -(2.0 * radius + step), Corners::Round);
    }
    // the last band cleared to a loop and a tool radius inside it; rings for the rest
    let mut ring = polygon::offset(&last, -(radius + step), Corners::Round);
    while !ring.is_empty() {
        let next = polygon::offset(&ring, -step, Corners::Round);
        out.extend(ring);
        ring = next;
    }
    out
}

/// Loops `radius` round the closed `ring`, each a whole turn at most `advance` on from the
/// last, anticlockwise (climb, as round a hole) or clockwise. They end where they began; run
/// backwards (Climb + Conventional's every other pass) they turn the other way.
#[expect(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
fn looped(ring: &[Point], radius: f64, advance: f64, anticlockwise: bool) -> Vec<Point> {
    const PER_LOOP: u32 = 24;
    let mut points = ring.to_vec();
    points.extend(ring.first());
    let mut along = vec![0.0];
    for w in points.windows(2) {
        along.push(along[along.len() - 1] + w[0].distance(w[1]));
    }
    let length = along[along.len() - 1];
    if points.len() < 2 || length <= 0.0 {
        return vec![];
    }
    let at = |s: f64| {
        let s = s.rem_euclid(length);
        let i = along
            .partition_point(|&a| a <= s)
            .clamp(1, points.len() - 1);
        let t = (s - along[i - 1]) / (along[i] - along[i - 1]).max(1e-12);
        points[i - 1].lerp(points[i], t)
    };
    let side = if anticlockwise { 1.0 } else { -1.0 };
    let samples = (length / advance).ceil().max(1.0) as u32 * PER_LOOP;
    (0..=samples)
        .map(|k| {
            let s = length * f64::from(k) / f64::from(samples);
            // the way on, over a loop's width, so a corner in the ring turns the loops smoothly
            let ahead = (at(s + radius) - at(s - radius))
                .try_normalize()
                .unwrap_or(Point::X);
            let a = std::f64::consts::TAU * f64::from(k % PER_LOOP) / f64::from(PER_LOOP);
            at(s) + radius * (a.sin() * ahead - side * a.cos() * ahead.perp())
        })
        .collect()
}

/// Raster lines `step` apart at `angle` across the region inside `edge`, every other one
/// back the other way.
fn raster(edge: &[Path], step: f64, angle: f64) -> Vec<Path> {
    hatch_lines(edge, step, angle)
        .into_iter()
        .enumerate()
        .flat_map(|(row, spans)| {
            let mut spans: Vec<Path> = spans
                .into_iter()
                .map(|(a, b)| Path::polyline(&[a, b], false))
                .collect();
            if row % 2 == 1 {
                spans.reverse();
                spans = spans.iter().map(Path::reversed).collect();
            }
            spans
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use cut_craft_core::geometry::polyline_distance;
    use cut_craft_core::testing::{cut_bounds, made_toolpath, square};
    use cut_craft_core::toolpath::{MoveKind, Toolpath};

    cut_craft_core::contract_tests!(PocketToolpath);

    fn pocket(size: f64, p: &Params) -> Result<Toolpath, String> {
        made_toolpath::<PocketToolpath>(vec![square(0.0, 0.0, size)], p)
    }

    #[test]
    fn stays_a_tool_radius_inside_the_shape() {
        let b = cut_bounds(&pocket(20.0, &Params::default()).unwrap());
        assert!(
            b.min.x >= 3.0 - 0.02
                && b.max.x <= 17.0 + 0.02
                && b.min.y >= 3.0 - 0.02
                && b.max.y <= 17.0 + 0.02,
            "{b:?}"
        );
    }

    #[test]
    fn the_tool_reaches_the_middle() {
        let tp = pocket(20.0, &Params::default()).unwrap();
        let middle = Point::new(10.0, 10.0);
        // distance to the cut segments themselves, not just their end points
        let closest = tp
            .moves
            .windows(2)
            .filter(|w| w[1].kind == MoveKind::Cut)
            .map(|w| polyline_distance(&[w[0].to.truncate(), w[1].to.truncate()], middle))
            .fold(f64::MAX, f64::min);
        assert!(
            closest <= Tool::default().diameter / 2.0,
            "the middle is {closest} mm from the nearest cut"
        );
    }

    /// The nearest a cutting move comes to `at`.
    fn nearest_cut(tp: &Toolpath, at: Point) -> f64 {
        tp.moves
            .windows(2)
            .filter(|w| w[1].kind == MoveKind::Cut)
            .map(|w| polyline_distance(&[w[0].to.truncate(), w[1].to.truncate()], at))
            .fold(f64::MAX, f64::min)
    }

    #[test]
    fn trochoidal_loops_clear_the_whole_pocket_and_stay_inside_it() {
        let p = Params {
            strategy: Strategy::Trochoidal,
            ..Params::default()
        };
        let tp = pocket(40.0, &p).unwrap();
        let r = Tool::default().diameter / 2.0;
        let b = cut_bounds(&tp);
        assert!(
            b.min.x >= r - 0.02 && b.max.x <= 40.0 - r + 0.02,
            "inside the walls: {b:?}"
        );
        assert!(b.min.y >= r - 0.02 && b.max.y <= 40.0 - r + 0.02, "{b:?}");
        // every place the tool's middle may go is cut
        for i in 0..=20 {
            for j in 0..=20 {
                let at = Point::new(
                    r + (40.0 - 2.0 * r) * f64::from(i) / 20.0,
                    r + (40.0 - 2.0 * r) * f64::from(j) / 20.0,
                );
                let gap = nearest_cut(&tp, at);
                assert!(gap <= r + 0.05, "{at:?} is {gap} mm from the nearest cut");
            }
        }
        // it loops: within a dozen moves the tool heads back the way it came, again and again
        let offset = pocket(40.0, &Params::default()).unwrap();
        let turns = |tp: &Toolpath| {
            tp.moves
                .windows(13)
                .filter(|w| w.iter().all(|m| m.kind == MoveKind::Cut))
                .filter(|w| (w[1].to - w[0].to).dot(w[12].to - w[11].to) < 0.0)
                .count()
        };
        assert!(
            turns(&tp) > 10 * turns(&offset).max(1),
            "{} turns",
            turns(&tp)
        );
    }

    #[test]
    fn climb_and_conventional_take_turns_pass_by_pass() {
        // twice the area the cuts at `z` go round: positive anticlockwise (climb in a pocket)
        let turning = |tp: &Toolpath, z: f64| -> f64 {
            tp.moves
                .windows(2)
                .filter(|w| w[1].kind == MoveKind::Cut)
                .filter(|w| w.iter().all(|m| (m.to.z - z).abs() < 1e-9))
                .map(|w| w[0].to.truncate().perp_dot(w[1].to.truncate()))
                .sum()
        };
        for strategy in [Strategy::Offset, Strategy::Trochoidal] {
            let p = Params {
                depth: 9.0,
                strategy,
                direction: Direction::Both,
                ..Params::default()
            };
            let tp = pocket(30.0, &p).unwrap();
            let ways: Vec<f64> = [-3.0, -6.0, -9.0].map(|z| turning(&tp, z)).to_vec();
            assert!(
                ways[0] > 0.0 && ways[1] < 0.0 && ways[2] > 0.0,
                "{strategy:?}: climb, conventional, climb: {ways:?}"
            );
        }
    }

    #[test]
    fn clears_each_depth_before_going_deeper() {
        let tp = pocket(
            20.0,
            &Params {
                depth: 6.0,
                ..Params::default()
            },
        )
        .unwrap();
        let depths: Vec<f64> = tp
            .moves
            .iter()
            .filter(|m| m.kind == MoveKind::Plunge)
            .map(|m| m.to.z)
            .collect();
        assert!(
            depths.windows(2).all(|w| w[1] <= w[0]),
            "never climbs back to a shallower pass: {depths:?}"
        );
        assert_eq!(depths.first(), Some(&-3.0));
        assert_eq!(depths.last(), Some(&-6.0));
    }

    #[test]
    fn clearance_tools_clear_first_and_the_pocket_tool_cuts_only_what_they_left() {
        use cut_craft_core::testing::document_with;
        let mill = |d: f64| Tool {
            name: format!("{d} mm End Mill"),
            diameter: d,
            ..Tool::default()
        };
        let p = Params {
            tool: mill(2.0),
            clearance_tools: vec![mill(12.0)],
            ..Params::default()
        };
        let mut doc = document_with(vec![square(0.0, 0.0, 40.0)]);
        doc.apply(&PocketToolpath::run(&doc, &p).unwrap()).unwrap();
        let made: Vec<&Toolpath> = doc.toolpaths.values().collect();
        assert_eq!(made.len(), 2);
        assert_eq!(made[0].name, "Pocket [1]");
        assert_eq!(made[0].tool.diameter, 12.0);
        assert_eq!(made[1].name, "Pocket [2]");
        assert_eq!(made[1].tool.diameter, 2.0);
        // the small tool only goes where the big one could not: far less than a full pocket
        let alone = pocket(
            40.0,
            &Params {
                tool: mill(2.0),
                ..Params::default()
            },
        )
        .unwrap();
        assert!(
            made[1].moves.len() * 2 < alone.moves.len(),
            "{} vs {}",
            made[1].moves.len(),
            alone.moves.len()
        );
        // and it keeps its rest settings, so recalculating it does the same
        assert!(cut_craft_core::rest::Rest::of(&made[1].params).is_some());
    }

    #[test]
    fn a_shape_smaller_than_the_tool_cannot_be_pocketed() {
        assert_eq!(
            pocket(4.0, &Params::default()).unwrap_err(),
            "the tool is too big to fit inside the selected shapes"
        );
    }

    #[test]
    fn stepover_must_be_a_fraction_of_the_tool() {
        let p = Params {
            tool: Tool {
                stepover: 0.0,
                ..Tool::default()
            },
            ..Params::default()
        };
        assert_eq!(
            pocket(20.0, &p).unwrap_err(),
            "invalid parameters: stepover must be more than 0 and at most 1"
        );
    }

    #[test]
    fn raster_lines_run_at_the_angle_with_a_profile_pass_round_the_edge() {
        let p = Params {
            strategy: Strategy::Raster,
            raster_angle: 90.0,
            ..Params::default()
        };
        let tp = pocket(20.0, &p).unwrap();
        let b = cut_bounds(&tp);
        assert!(b.min.x >= 3.0 - 0.02 && b.max.x <= 17.0 + 0.02, "{b:?}");
        // the lines run along Y: most cuts keep x and change y
        let along_y = tp
            .moves
            .windows(2)
            .filter(|w| w[1].kind == MoveKind::Cut)
            .filter(|w| (w[0].to.x - w[1].to.x).abs() < 1e-9 && (w[0].to.y - w[1].to.y).abs() > 5.0)
            .count();
        assert!(along_y >= 4, "{along_y}");
        // the last cuts go round the edge, a tool radius in
        let last = tp
            .moves
            .iter()
            .rev()
            .find(|m| m.kind == MoveKind::Cut)
            .unwrap();
        let p = last.to.truncate();
        let on_edge = [p.x - 3.0, 17.0 - p.x, p.y - 3.0, 17.0 - p.y]
            .iter()
            .any(|d| d.abs() < 0.05);
        assert!(on_edge, "{p}");
    }

    #[test]
    fn ramp_plunge_moves_zig_zag_down_from_the_level_above() {
        let p = Params {
            depth: 6.0,
            ramp_plunge_moves: true,
            ramp_distance: 4.0,
            ..Params::default()
        };
        let tp = pocket(20.0, &p).unwrap();
        // straight down only to the surface or the level already cleared, never into
        // material
        let straight: Vec<f64> = tp
            .moves
            .windows(2)
            .filter(|w| {
                let (a, b) = (w[0].to, w[1].to);
                b.z < a.z - 1e-9 && a.truncate().distance(b.truncate()) < 1e-9
            })
            .map(|w| w[1].to.z)
            .collect();
        assert!(
            straight
                .iter()
                .all(|&z| z.abs() < 1e-9 || (z + 3.0).abs() < 1e-9),
            "{straight:?}"
        );
    }

    #[test]
    fn the_allowance_leaves_a_wall_and_rings_run_the_chosen_way() {
        let p = Params {
            allowance: 1.0,
            ..Params::default()
        };
        let b = cut_bounds(&pocket(20.0, &p).unwrap());
        assert!(
            (b.min.x - 4.0).abs() < 0.02 && (b.max.x - 16.0).abs() < 0.02,
            "{b:?}"
        );
        for (direction, anticlockwise) in
            [(Direction::Climb, true), (Direction::Conventional, false)]
        {
            let tp = pocket(
                20.0,
                &Params {
                    direction,
                    ..Params::default()
                },
            )
            .unwrap();
            // the outer ring is the last one cut
            let start = tp
                .moves
                .iter()
                .rposition(|m| m.kind == MoveKind::Plunge)
                .unwrap();
            let ring: Vec<Point> = tp.moves[start..]
                .iter()
                .filter(|m| m.kind == MoveKind::Cut)
                .map(|m| m.to.truncate())
                .collect();
            let turn = cut_craft_core::geometry::winding(&ring);
            assert_eq!(turn > 0.0, anticlockwise, "{direction:?}");
        }
    }
}
