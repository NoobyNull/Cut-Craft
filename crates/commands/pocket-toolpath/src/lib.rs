//! Pocket Toolpath: clears the inside of the selected closed vectors at each depth, with
//! rings that step in from the outline (worked from the middle outwards) or with raster
//! lines and a profile pass round the edge; or, trochoidal, with the tool circling in small
//! loops as it moves on, so it never cuts with more than a sliver of its edge.

use cut_craft_core::document::Document;
use cut_craft_core::entry::{
    Direction, Entry, Loops, Motion, Ramp, Ramping, Ways, path_moves, trochoid,
};
use cut_craft_core::geometry::{Path, Point, winding};
use cut_craft_core::polygon::{self, Corners};
use cut_craft_core::toolpath::{Move, Tool, cut_points, hatch_lines, made_by, project_onto_model};
use cut_craft_core::{Change, Command, CommandError, Meta, ensure_positive, tool_does_not_fit};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub struct PocketToolpath;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Strategy {
    /// Rings stepping in from the outline
    #[default]
    #[serde(alias = "trochoidal")]
    Offset,
    /// Straight lines across, at the raster angle
    Raster,
}

/// Which way an offset pocket's rings step. Turning the cut direction's way, rings stepping
/// from the middle out cut it (climb, say) and rings stepping from the outline in cut the
/// other (conventional): the wall's own ring always cuts the chosen direction.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum RingOrder {
    /// The middle ring first, the outline's last
    #[default]
    #[schemars(title = "From the middle out")]
    MiddleOut,
    /// The outline's ring first, the middle's last
    #[schemars(title = "From the outline in")]
    OutlineIn,
    /// Middle out on the first pass, outline in on the next, and so on: each pass starts
    /// where the last ended, cutting the chosen direction then the other
    Alternate,
}

impl RingOrder {
    /// Whether pass `pass` (from 0) steps from the outline in.
    fn inward(self, pass: usize) -> bool {
        match self {
            Self::MiddleOut => false,
            Self::OutlineIn => true,
            Self::Alternate => pass % 2 == 1,
        }
    }
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
    #[schemars(skip)]
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
    /// Every pass the same way, or each back the way the last came
    #[serde(default)]
    #[schemars(skip)]
    pub ways: Ways,
    /// Which way the offset rings step; trochoidal bands always step from the outline in
    /// (added with ring order)
    #[serde(default)]
    #[schemars(skip)]
    pub rings: RingOrder,
    /// Straight, or in small loops: trochoidal clears in looped bands from the outline in,
    /// then cuts once round the edge
    #[serde(default)]
    #[schemars(skip)]
    pub motion: Motion,
    /// Set in the Cutting window, with the motion trochoidal
    #[serde(default)]
    #[schemars(skip)]
    pub loops: Loops,
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
            ways: Ways::One,
            rings: RingOrder::MiddleOut,
            motion: Motion::Straight,
            loops: Loops::default(),
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
        cut_craft_core::rest::check_clearing(&p.tool, &p.clearance_tools, true)?;
        // each tool, largest first, then the pocket's own; a raster only for the first
        let mut clearing: Vec<&Tool> = p.clearance_tools.iter().collect();
        clearing.sort_by(|a, b| b.diameter.total_cmp(&a.diameter));
        let tools: Vec<&Tool> = clearing.into_iter().chain([&p.tool]).collect();
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

/// The depth and the tool's sizes, checked.
fn check(p: &Params) -> Result<(), CommandError> {
    ensure_positive("depth", p.depth)?;
    ensure_positive("tool diameter", p.tool.diameter)?;
    ensure_positive("pass depth", p.tool.pass_depth)?;
    if !(p.tool.stepover > 0.0 && p.tool.stepover <= 1.0) {
        return Err(CommandError::InvalidParams(
            "set the tool's clearing stepover to more than 0% and at most 100% in the Tool Database".into(),
        ));
    }
    Ok(())
}

/// The pocket for `p`'s own tool.
fn pocket(doc: &Document, p: &Params) -> Result<cut_craft_core::toolpath::Toolpath, CommandError> {
    {
        check(p)?;
        let (ids, paths) = doc.selected_paths(|path| path.closed);
        if ids.is_empty() {
            return Err(CommandError::Failed(
                "select the closed vectors to pocket".into(),
            ));
        }
        // a shape inside another is an island the pocket goes round (a ring between two
        // circles), as in the other toolpaths
        // ponytail: overlapping shapes go even-odd too, their overlap left uncut
        let ring = polygon::offset_regions(
            &polygon::regions(&paths),
            -(p.tool.diameter / 2.0 + p.allowance),
            Corners::Round,
        );
        if ring.is_empty() {
            return Err(tool_does_not_fit(&p.tool, None));
        }
        let step = p.tool.stepover * p.tool.diameter;
        let edge = ring.clone();
        let rings = rings_in(ring, step);
        // the same rings from the outline in: turning the same way, they cut conventional
        // where the rings from the middle out cut climb (or the other way about)
        let outline_in: Vec<Path> = rings.iter().rev().cloned().collect();
        let mut loops = vec![];
        if p.motion == Motion::Trochoidal {
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
        // way (Both directions' every other pass)
        let cut = |paths: &[Path], z: f64, above: f64, back: bool| -> Vec<Move> {
            paths
                .iter()
                .flat_map(|r| {
                    let mut points = if r.closed {
                        oriented(p.direction, r)
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
            let back = p.ways == Ways::Both && pass % 2 == 1;
            let cut_way = |paths: &[Path], back: bool| cut(paths, z, above, back);
            let cut = |paths: &[Path]| cut_way(paths, back);
            match (p.motion, p.strategy) {
                (Motion::Trochoidal, _) => {
                    moves.extend(cut(&loops));
                    moves.extend(cut(&edge));
                }
                (Motion::Straight, Strategy::Offset) => {
                    let inward = p.rings.inward(pass);
                    moves.extend(cut_way(if inward { &outline_in } else { &rings }, back));
                }
                (Motion::Straight, Strategy::Raster) => {
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
        let next = polygon::offset_regions_again(&ring, -step);
        rings.extend(ring);
        ring = next;
    }
    rings.reverse();
    rings
}

/// A ring of the pocket's regions turned to cut `direction`: an outline (anticlockwise) has
/// the waste inside it, an island's (clockwise) outside it.
fn oriented(direction: Direction, ring: &Path) -> Vec<Point> {
    let points = cut_points(ring);
    let outline = winding(&points) > 0.0;
    direction.orient(points, outline)
}

/// The trochoidal clearing of the region inside `edge` (where the tool's middle may go):
/// looping bands from the outline in, each band's loops reaching from the one outside it to
/// the one inside, then plain rings for a middle too small for a band.
fn trochoidal(edge: &[Path], p: &Params) -> Vec<Path> {
    let (radius, advance) = p.loops.sized(p.tool.diameter);
    let step = p.tool.stepover * p.tool.diameter;
    let mut out = vec![];
    let mut band = polygon::offset_regions(edge, -radius, Corners::Round);
    let mut last = vec![];
    while !band.is_empty() {
        for ring in &band {
            let ring = oriented(p.direction, ring);
            let points = trochoid(&ring, true, radius, advance, p.loops.anticlockwise());
            out.push(Path::polyline(&points, false));
        }
        last = band;
        band = polygon::offset_regions_again(&last, -(2.0 * radius + step));
    }
    // the last band cleared to a loop and a tool radius inside it; rings for the rest
    let mut ring = polygon::offset_regions(&last, -(radius + step), Corners::Round);
    while !ring.is_empty() {
        let next = polygon::offset_regions_again(&ring, -step);
        out.extend(ring);
        ring = next;
    }
    out
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
    use cut_craft_core::geometry::{Point, polyline_distance};
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
    fn a_pocket_clears_with_the_clearing_stepover_not_the_finishing_one() {
        let with = |stepover: f64, finish_stepover: f64| {
            let mut p = Params::default();
            p.tool.stepover = stepover;
            p.tool.finish_stepover = finish_stepover;
            pocket(30.0, &p).unwrap().moves
        };
        assert_eq!(
            with(0.4, 0.05),
            with(0.4, 0.4),
            "finishing does not change it"
        );
        assert_ne!(with(0.2, 0.4), with(0.4, 0.4), "clearing does");
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

    /// A 40 mm square with a 10 mm square island in its middle, pocketed with `p`.
    fn ring_pocket(p: &Params) -> Toolpath {
        made_toolpath::<PocketToolpath>(vec![square(0.0, 0.0, 40.0), square(15.0, 15.0, 10.0)], p)
            .unwrap()
    }

    #[test]
    fn a_shape_inside_another_is_an_island_the_pocket_goes_round() {
        let radius = Tool::default().diameter / 2.0;
        for (strategy, motion) in [
            (Strategy::Offset, Motion::Straight),
            (Strategy::Raster, Motion::Straight),
            (Strategy::Offset, Motion::Trochoidal),
        ] {
            let tp = ring_pocket(&Params {
                strategy,
                motion,
                ..Params::default()
            });
            let middle = nearest_cut(&tp, Point::new(20.0, 20.0));
            assert!(
                middle >= 5.0 + radius - 0.05,
                "{strategy:?} {motion:?}: cuts {middle} mm from the island's middle"
            );
            // and the ring round it is still cleared
            for at in [Point::new(7.5, 20.0), Point::new(32.5, 32.5)] {
                let gap = nearest_cut(&tp, at);
                assert!(
                    gap <= radius,
                    "{strategy:?} {motion:?}: {at:?} {gap} mm from a cut"
                );
            }
        }
    }

    #[test]
    fn the_islands_wall_is_cut_the_chosen_direction_too() {
        for direction in [Direction::Climb, Direction::Conventional] {
            let tp = ring_pocket(&Params {
                direction,
                ..Params::default()
            });
            // the island's wall ring: the cut nearest the island's side
            let wall = nearest_cut(&tp, Point::new(20.0, 20.0));
            let middle = Point::new(20.0, 20.0);
            let mut ring: Vec<Point> = vec![];
            let mut from = None;
            for m in &tp.moves {
                if m.kind == MoveKind::Cut {
                    if ring.is_empty() {
                        ring.extend(from);
                    }
                    ring.push(m.to.truncate());
                } else if !ring.is_empty() {
                    if (polyline_distance(&ring, middle) - wall).abs() < 0.01 {
                        break;
                    }
                    ring.clear();
                }
                from = Some(m.to.truncate());
            }
            assert!(!ring.is_empty(), "{direction:?}: no ring round the island");
            // climb on an island's wall (waste outside it) runs clockwise
            let clockwise = cut_craft_core::geometry::winding(&ring) < 0.0;
            assert_eq!(clockwise, direction == Direction::Climb, "{direction:?}");
        }
    }

    #[test]
    fn shapes_side_by_side_are_each_pocketed() {
        let tp = made_toolpath::<PocketToolpath>(
            vec![square(0.0, 0.0, 20.0), square(30.0, 0.0, 20.0)],
            &Params::default(),
        )
        .unwrap();
        for at in [Point::new(10.0, 10.0), Point::new(40.0, 10.0)] {
            assert!(
                nearest_cut(&tp, at) <= Tool::default().diameter / 2.0,
                "{at:?}"
            );
        }
    }

    #[test]
    fn trochoidal_loops_clear_the_whole_pocket_and_stay_inside_it() {
        let p = Params {
            motion: Motion::Trochoidal,
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
    fn both_directions_take_turns_climb_then_conventional() {
        // twice the area the cuts at `z` go round: positive anticlockwise (climb in a pocket)
        let turning = |tp: &Toolpath, z: f64| -> f64 {
            tp.moves
                .windows(2)
                .filter(|w| w[1].kind == MoveKind::Cut)
                .filter(|w| w.iter().all(|m| (m.to.z - z).abs() < 1e-9))
                .map(|w| w[0].to.truncate().perp_dot(w[1].to.truncate()))
                .sum()
        };
        // each pass goes back round the other way, rings and loops alike
        for motion in [Motion::Straight, Motion::Trochoidal] {
            let p = Params {
                depth: 9.0,
                motion,
                ways: Ways::Both,
                ..Params::default()
            };
            let tp = pocket(30.0, &p).unwrap();
            let ways: Vec<f64> = [-3.0, -6.0, -9.0].map(|z| turning(&tp, z)).to_vec();
            assert!(
                ways[0] > 0.0 && ways[1] < 0.0 && ways[2] > 0.0,
                "{motion:?}: climb, conventional, climb: {ways:?}"
            );
        }
    }

    #[test]
    fn the_loops_turn_their_own_way_whichever_the_cut_direction() {
        use cut_craft_core::entry::Turn;
        // twice the area the first pass's loops go round: positive anticlockwise
        let turning = |direction, turn| -> f64 {
            let p = Params {
                motion: Motion::Trochoidal,
                direction,
                loops: Loops {
                    turn,
                    ..Loops::default()
                },
                ..Params::default()
            };
            let tp = pocket(30.0, &p).unwrap();
            tp.moves
                .windows(2)
                .filter(|w| w.iter().all(|m| m.kind == MoveKind::Cut))
                .map(|w| w[0].to.truncate().perp_dot(w[1].to.truncate()))
                .sum()
        };
        for direction in [Direction::Climb, Direction::Conventional] {
            assert!(
                turning(direction, Turn::Anticlockwise) > 0.0,
                "{direction:?}"
            );
            assert!(turning(direction, Turn::Clockwise) < 0.0, "{direction:?}");
        }
    }

    #[test]
    fn rings_step_from_the_middle_out_the_outline_in_or_by_turns() {
        // how far from the middle each pass's first ring is
        let first_rings = |rings: RingOrder| -> Vec<f64> {
            let p = Params {
                depth: 9.0,
                rings,
                ..Params::default()
            };
            let tp = pocket(30.0, &p).unwrap();
            [-3.0, -6.0, -9.0]
                .map(|z| {
                    let m = tp
                        .moves
                        .iter()
                        .find(|m| m.kind == MoveKind::Cut && (m.to.z - z).abs() < 1e-9)
                        .unwrap();
                    m.to.truncate().distance(Point::new(15.0, 15.0))
                })
                .to_vec()
        };
        let (middle, outline) = (|d: &f64| *d < 5.0, |d: &f64| *d > 9.0);
        let out = first_rings(RingOrder::MiddleOut);
        assert!(out.iter().all(middle), "{out:?}");
        let inward = first_rings(RingOrder::OutlineIn);
        assert!(inward.iter().all(outline), "{inward:?}");
        let turns = first_rings(RingOrder::Alternate);
        assert!(
            middle(&turns[0]) && outline(&turns[1]) && middle(&turns[2]),
            "middle, outline, middle: {turns:?}"
        );
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
    fn a_clearing_bit_not_bigger_than_the_finishing_bit_is_refused() {
        use cut_craft_core::testing::document_with;
        let mill = |d: f64| Tool {
            name: format!("{d} mm End Mill"),
            diameter: d,
            ..Tool::default()
        };
        let p = Params {
            tool: mill(6.0),
            clearance_tools: vec![mill(3.0)],
            ..Params::default()
        };
        let doc = document_with(vec![square(0.0, 0.0, 40.0)]);
        let Err(CommandError::InvalidParams(why)) = PocketToolpath::run(&doc, &p) else {
            panic!("a 3 mm clearing bit before a 6 mm finishing bit");
        };
        assert!(
            why.contains("must be bigger than the finishing bit"),
            "{why}"
        );
    }

    #[test]
    fn clearing_bits_cut_largest_first_whatever_order_they_were_picked_in() {
        use cut_craft_core::testing::document_with;
        let mill = |d: f64| Tool {
            name: format!("{d} mm End Mill"),
            diameter: d,
            ..Tool::default()
        };
        let p = Params {
            tool: mill(2.0),
            clearance_tools: vec![mill(8.0), mill(20.0)],
            ..Params::default()
        };
        let mut doc = document_with(vec![square(0.0, 0.0, 60.0)]);
        doc.apply(&PocketToolpath::run(&doc, &p).unwrap()).unwrap();
        let sizes: Vec<f64> = doc.toolpaths.values().map(|t| t.tool.diameter).collect();
        assert_eq!(sizes, vec![20.0, 8.0, 2.0]);
    }

    #[test]
    fn a_shape_smaller_than_the_tool_cannot_be_pocketed() {
        assert_eq!(
            pocket(4.0, &Params::default()).unwrap_err(),
            "The 6 mm End Mill does not fit inside the selected shapes. Pick a smaller tool, \
             or make the shapes bigger."
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
            "Set the tool's clearing stepover to more than 0% and at most 100% in the Tool Database."
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
