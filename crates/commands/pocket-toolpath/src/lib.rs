//! Pocket Toolpath: clears the inside of the selected closed vectors at each depth, with
//! rings that step in from the outline (worked from the middle outwards), with raster
//! lines and a profile pass round the edge, or adaptively, the tool keeping a thin, steady
//! bite everywhere so it never cuts with more than a sliver of its edge.

use cut_craft_core::adaptive::{self, Clearing, Piece};
use cut_craft_core::document::Document;
use cut_craft_core::entry::{
    Direction, Entry, EntrySettings, Loops, Motion, Ramp, Ramping, Room, Ways, path_moves, trochoid,
};
use cut_craft_core::geometry::{Path, Point, winding};
use cut_craft_core::polygon::{self, Corners};
use cut_craft_core::toolpath::{
    Move, MoveKind, Tool, cut_points, hatch_lines, made_by, project_onto_model,
};
use cut_craft_core::{
    Change, Command, CommandError, Form, Meta, ensure_positive, tool_does_not_fit,
};
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
    /// Straight lines across, at the raster angle, back and forth: each line steps over to
    /// the next along the wall without lifting
    Raster,
    /// Keeps the bit taking a thin, steady bite everywhere, never cutting full width in
    /// a corner: gentler on the bit and machine, so it can go a full pass deep with confidence
    Adaptive,
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
    /// Round the edge before the raster lines (Raster only: adaptive clearing always goes
    /// round last)
    #[schemars(extend("x-show-when" = {"strategy": "raster"}))]
    First,
    /// Round the edge after the clearing
    #[default]
    Last,
    /// No pass round the edge: it is left rough
    None,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
pub struct Params {
    /// What the toolpath is called in the toolpath list
    pub name: String,
    /// Below the material surface
    #[schemars(extend("x-unit" = "mm"), range(min = 0))]
    #[schemars(title = "Cut Depth", extend("x-group" = "Cutting Depths"))]
    #[schemars(extend("x-dim" = "D", "x-diagram" = "diagrams/depth_pocket.png"))]
    pub depth: f64,
    /// The bit that cuts the walls and finishes the pocket; Select… picks one from your tools
    pub tool: Tool,
    /// Bigger tools that clear the pocket first, largest first: each makes a toolpath of its
    /// own and each tool after it cuts only what the ones before it left (rest machining)
    #[serde(default)]
    #[schemars(title = "Clearance Tools")]
    pub clearance_tools: Vec<Tool>,
    /// The pattern that clears the inside of the pocket
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
    /// Whether a pass round the edge cleans up the wall, and when (First is offered for
    /// Raster only: adaptive clearing always goes round last, if at all; offset rings end on
    /// the edge anyway)
    #[serde(default)]
    #[schemars(extend("x-show-when" = {"strategy": ["raster", "adaptive"]}))]
    #[schemars(extend("x-group" = "Clear Pocket"))]
    pub profile_pass: ProfilePass,
    /// How much the clearing leaves on the walls for the pass round the edge to take off in
    /// one thin, even skim, for a straighter, cleaner wall; 0 clears right up to it
    #[serde(default)]
    #[schemars(title = "Leave for Edge Pass", extend("x-unit" = "mm"), range(min = 0))]
    #[schemars(extend("x-dim" = "A", "x-diagram" = "diagrams/edge_pass.png"))]
    #[schemars(extend("x-group" = "Clear Pocket"))]
    pub edge_allowance: f64,
    /// Go round the edge once, at the full depth, after all the clearing passes: one clean
    /// wall with no lines between the passes. Only where the bit's flutes reach that deep;
    /// otherwise it goes round at each pass
    #[serde(default)]
    #[schemars(title = "Edge Pass at Full Depth")]
    #[schemars(extend("x-group" = "Clear Pocket"))]
    pub edge_full_depth: bool,
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
    /// How toolpaths made before entry modes went down: zig zag over the ramp distance
    /// instead of straight down. Kept for them; with `entry` unset they go down as they did
    #[serde(default)]
    #[schemars(skip)]
    pub ramp_plunge_moves: bool,
    #[serde(default = "ramp_distance")]
    #[schemars(skip)]
    pub ramp_distance: f64,
    /// How the tool goes down into the material (helix, zig-zag, ramp or straight down), set
    /// in the Entry window: once in each area of the pocket, fitted to the room there, then
    /// each ring links on to the next at depth. Unset on toolpaths made before entry modes
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(skip)]
    pub entry: Option<EntrySettings>,
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
            edge_allowance: 0.0,
            edge_full_depth: false,
            ways: Ways::One,
            rings: RingOrder::MiddleOut,
            motion: Motion::Straight,
            loops: Loops::default(),
            ramp_plunge_moves: false,
            ramp_distance: ramp_distance(),
            entry: Some(EntrySettings::default()),
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
            form: Form::Toolpath,
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
            // each bit its share of the progress bar, its rest machining included
            let toolpath = cut_craft_core::progress::part(i, tools.len(), || {
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
                Ok::<_, CommandError>(toolpath)
            })?;
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

/// The selected closed vectors `p` pockets, and where its tool's centre keeps inside them.
fn pocket_edge(
    doc: &Document,
    p: &Params,
) -> Result<(Vec<cut_craft_core::document::EntityId>, Vec<Path>), CommandError> {
    let (ids, paths) = doc.selected_paths(|path| path.closed);
    if ids.is_empty() {
        return Err(CommandError::Failed(
            "select the closed vectors to pocket".into(),
        ));
    }
    // a shape inside another is an island the pocket goes round (a ring between two
    // circles), as in the other toolpaths
    // ponytail: overlapping shapes go even-odd too, their overlap left uncut
    let edge = polygon::offset_regions(
        &polygon::regions(&paths),
        -(p.tool.diameter / 2.0 + p.allowance),
        Corners::Round,
    );
    if edge.is_empty() {
        return Err(tool_does_not_fit(&p.tool, None));
    }
    Ok((ids, edge))
}

/// The pocket for `p`'s own tool.
fn pocket(doc: &Document, p: &Params) -> Result<cut_craft_core::toolpath::Toolpath, CommandError> {
    {
        check(p)?;
        let (ids, edge) = pocket_edge(doc, p)?;
        let step = p.tool.stepover * p.tool.diameter;
        let walls_at = EdgePass::of(p, &edge);
        let rings = rings_in(walls_at.inner.clone(), step);
        // the same rings from the outline in: turning the same way, they cut conventional
        // where the rings from the middle out cut climb (or the other way about)
        let outline_in: Vec<Path> = rings.iter().rev().cloned().collect();
        let clearing = doc.job.clearing;
        let keep_climb = clearing.keeps_climb();
        let mut loops = vec![];
        let mut loops_back = vec![];
        if p.motion == Motion::Trochoidal {
            loops = trochoidal(&edge, p, &clearing.loops, false);
            // run back the other way, still turning the same way: still climb
            loops_back = trochoidal(&edge, p, &clearing.loops, true);
            if loops.is_empty() {
                loops.clone_from(&rings); // too narrow for loops: rings
            }
        }
        let plan = adaptive_plan(p, &walls_at.inner, &clearing);
        // a long adaptive clearing stops where it got to: nothing more to make
        cut_craft_core::progress::stop_if_cancelled()?;
        let safe = doc.job.clearance;
        let entry = entry_for(p, &edge);
        // each path at depth `z`, entering from the level above; `back` runs it the other
        // way (Both directions' every other pass)
        // `lines`: open paths (raster lines) link on at depth too, each from where the last
        // ended, along the wall
        let cut_any = |paths: &[Path], z: f64, above: f64, back: bool, lines: bool| -> Vec<Move> {
            let way = |r: &Path| {
                let mut points = if r.closed {
                    oriented(p.direction, r)
                } else {
                    cut_points(r)
                };
                if back {
                    points.reverse();
                }
                points
            };
            if p.entry.is_none() {
                return paths
                    .iter()
                    .flat_map(|r| path_moves(&way(r), r.closed, &[z], above, safe, &entry))
                    .collect();
            }
            linked(
                paths.iter().map(|r| (way(r), r.closed)).collect(),
                (z, above),
                safe,
                &entry,
                lines,
            )
        };
        // ponytail: each ring or line retracts between cuts; link them with a stepover move
        // to save time
        let mut moves = vec![];
        let mut above = 0.0;
        for (pass, z) in p.tool.pass_depths(p.depth).into_iter().enumerate() {
            let back = p.ways == Ways::Both && pass % 2 == 1;
            let cut_way = |paths: &[Path], back: bool| cut_any(paths, z, above, back, false);
            let cut = |paths: &[Path]| cut_way(paths, back);
            match (p.motion, p.strategy) {
                (_, Strategy::Adaptive) => {
                    let walls = walls_at.each_pass.then_some(&edge[..]);
                    moves.extend(adaptive_pass(&plan, walls, p, (z, above), safe, &entry));
                }
                (Motion::Trochoidal, _) => {
                    if back && keep_climb && !loops_back.is_empty() {
                        moves.extend(cut_way(&loops_back, false));
                        moves.extend(cut_way(&edge, true));
                    } else {
                        moves.extend(cut(&loops));
                        moves.extend(cut(&edge));
                    }
                }
                (Motion::Straight, Strategy::Offset) => {
                    let inward = p.rings.inward(pass);
                    let mut all = if inward {
                        outline_in.clone()
                    } else {
                        rings.clone()
                    };
                    // the skim left for the edge pass, last
                    if walls_at.each_pass && walls_at.leaves {
                        all.extend(edge.iter().cloned());
                    }
                    moves.extend(cut_way(&all, back));
                }
                (Motion::Straight, Strategy::Raster) => {
                    // rows kept one way stay that way on a pass back
                    let all = raster_run(&walls_at, &edge, step, p, keep_climb);
                    moves.extend(cut_any(&all, z, above, back && !keep_climb, true));
                }
            }
            above = z;
        }
        if walls_at.at_full_depth {
            edge_at_full_depth(&mut moves, &edge, p, above, safe, &entry);
        }
        if p.project_onto_model {
            moves = project_onto_model(doc, &p.tool, &moves)?;
        }
        let mut toolpath = made_by::<PocketToolpath>(p, p.name.clone(), p.tool.clone(), ids, moves);
        toolpath.fallbacks = entry.take_fallbacks();
        Ok(toolpath)
    }
}

/// Once round `edge` at the bottom, `z`, after `moves` cleared the pocket: from where the
/// clearing ended, at depth, for one clean wall.
fn edge_at_full_depth(
    moves: &mut Vec<Move>,
    edge: &[Path],
    p: &Params,
    z: f64,
    safe: f64,
    entry: &Entry,
) {
    let from = moves
        .iter()
        .rev()
        .find(|m| m.kind != MoveKind::Rapid)
        .filter(|m| (m.to.z - z).abs() < 1e-9)
        .map(|m| m.to);
    while from.is_some() && moves.last().is_some_and(|m| m.kind == MoveKind::Rapid) {
        moves.pop();
    }
    let rings: Vec<Vec<Point>> = edge.iter().map(|r| oriented(p.direction, r)).collect();
    moves.extend(walls(&rings, from, z, safe, entry));
}

/// Where a pocket's clearing stops and when its pass round the edge comes.
struct EdgePass {
    /// Where the clearing's tool centre keeps inside: the edge, or less the skim left for
    /// the edge pass.
    inner: Vec<Path>,
    /// Whether `inner` stops short of the edge.
    leaves: bool,
    /// Round the edge at each pass depth (with the clearing).
    each_pass: bool,
    /// Round the edge once, at the full depth, after all the clearing.
    at_full_depth: bool,
}

impl EdgePass {
    /// For `p`, whose tool centre keeps inside `edge`: offset rings always end on the edge,
    /// raster and adaptive clearing go round it unless their edge pass is None. Full depth
    /// only where the bit's flutes reach (or their length is not known); else each pass.
    fn of(p: &Params, edge: &[Path]) -> Self {
        let goes_round = p.strategy == Strategy::Offset || p.profile_pass != ProfilePass::None;
        let loops = p.motion == Motion::Trochoidal && p.strategy != Strategy::Adaptive;
        if !goes_round || loops {
            return Self {
                inner: edge.to_vec(),
                leaves: false,
                each_pass: goes_round,
                at_full_depth: false,
            };
        }
        let reaches = p
            .tool
            .known_flute_length()
            .is_none_or(|flutes| flutes >= p.depth - 1e-9);
        let at_full_depth = p.edge_full_depth && reaches;
        let leave = p.edge_allowance.max(0.0);
        let inner = if leave > 0.0 {
            polygon::offset_regions_again(edge, -leave)
        } else {
            vec![]
        };
        let leaves = !inner.is_empty();
        Self {
            inner: if leaves { inner } else { edge.to_vec() },
            leaves,
            each_pass: !at_full_depth,
            at_full_depth,
        }
    }
}

/// How `p` goes down into the pocket whose tool centre keeps inside `edge`.
fn entry_for(p: &Params, edge: &[Path]) -> Entry {
    Entry {
        ramp: if p.ramp_plunge_moves && p.entry.is_none() {
            Ramp::ZigZag
        } else {
            Ramp::None
        },
        ramping: Ramping {
            distance: p.ramp_distance,
            ..Ramping::default()
        },
        enter: p.entry,
        room: Some(Room::inside(edge.iter().map(cut_points).collect())),
        tool_diameter: p.tool.diameter,
        no_plunge: p.tool.cannot_plunge(),
        ..Entry::default()
    }
}

/// The rings and lines `paths` cut at `z`, entering from `above` only where the tool must go
/// down into the material: a closed ring within a bit's width of where the last one ended,
/// with a straight way across to it in the pocket, is cut next, linked to at depth, so each
/// area of the pocket is entered once. With `lines`, an open path (a raster line) whose start
/// is that near is linked to the same way: the step over along the wall from one line to the
/// next, instead of a lift and a plunge into uncut material. The rest keep their order.
fn linked(
    mut paths: Vec<(Vec<Point>, bool)>,
    (z, above): (f64, f64),
    safe: f64,
    entry: &Entry,
    lines: bool,
) -> Vec<Move> {
    let reach = entry.tool_diameter;
    let mut moves: Vec<Move> = vec![];
    // where the tool is, at depth, after a closed ring
    let mut down: Option<Point> = None;
    while !paths.is_empty() {
        let next = down.and_then(|at| {
            paths.iter().enumerate().find_map(|(i, (points, closed))| {
                let (k, gap) = if *closed {
                    nearest_vertex(points, at)?
                } else if lines {
                    (0, points.first()?.distance(at))
                } else {
                    return None;
                };
                let near = gap <= reach + 1e-9;
                let across = entry.room.as_ref().is_none_or(|r| r.holds(at, points[k]));
                (near && across).then_some((i, k))
            })
        });
        let Some((i, k)) = next else {
            let (points, closed) = paths.remove(0);
            moves.extend(path_moves(&points, closed, &[z], above, safe, entry));
            down = (closed || lines).then(|| at_depth(&moves, z)).flatten();
            continue;
        };
        let (mut points, closed) = paths.remove(i);
        points.rotate_left(k);
        // off with the lift after the last ring, across at depth, round without entering
        if moves.last().is_some_and(|m| m.kind == MoveKind::Rapid) {
            moves.pop();
        }
        moves.push(Move {
            kind: MoveKind::Cut,
            to: points[0].extend(z),
        });
        let ring = path_moves(&points, closed, &[z], z, safe, entry);
        let first_cut = ring
            .iter()
            .position(|m| m.kind == MoveKind::Cut)
            .unwrap_or(ring.len());
        moves.extend_from_slice(&ring[first_cut..]);
        down = at_depth(&moves, z);
    }
    moves
}

/// Where the tool was at depth `z` before it lifted at the end of `moves`.
fn at_depth(moves: &[Move], z: f64) -> Option<Point> {
    let last = moves.iter().rev().find(|m| m.kind != MoveKind::Rapid)?;
    ((last.to.z - z).abs() < 1e-9).then(|| last.to.truncate())
}

/// The vertex of `points` nearest `at`, and how far it is.
fn nearest_vertex(points: &[Point], at: Point) -> Option<(usize, f64)> {
    points
        .iter()
        .map(|p| p.distance(at))
        .enumerate()
        .min_by(|a, b| a.1.total_cmp(&b.1))
}

/// The adaptive clearing of the pocket whose tool centre keeps inside `edge`, the same at
/// every depth; empty unless `p` clears adaptively.
fn adaptive_plan(
    p: &Params,
    edge: &[Path],
    clearing: &cut_craft_core::entry::ClearingSettings,
) -> Vec<Piece> {
    if p.strategy != Strategy::Adaptive {
        return vec![];
    }
    adaptive::clear(&Clearing {
        region: edge,
        cleared: &[],
        radius: p.tool.diameter / 2.0,
        bite: adaptive_bite(&p.tool, clearing),
        climb: p.direction == Direction::Climb,
    })
}

/// One pass depth of an adaptive pocket: `plan` cleared at depth `z` coming down from
/// `above`, then once round the wall, `edge`, when there is one to go round at this depth.
fn adaptive_pass(
    plan: &[Piece],
    edge: Option<&[Path]>,
    p: &Params,
    (z, above): (f64, f64),
    safe: f64,
    entry: &Entry,
) -> Vec<Move> {
    let mut moves = adaptive_moves(plan, z, above, safe, entry);
    let from = moves.last().map(|m| m.to);
    let Some(edge) = edge else {
        // no pass round the wall (or one at the full depth later): up, clear of it
        if let Some(at) = from {
            moves.push(Move {
                kind: MoveKind::Rapid,
                to: at.truncate().extend(safe),
            });
        }
        return moves;
    };
    let rings: Vec<Vec<Point>> = edge.iter().map(|r| oriented(p.direction, r)).collect();
    moves.extend(walls(&rings, from, z, safe, entry));
    moves
}

/// How wide a bite adaptive clearing takes with `tool`: the job's light step (Clearing
/// Settings), a share of the bit's diameter, so every bit takes the same gentle bite.
fn adaptive_bite(tool: &Tool, clearing: &cut_craft_core::entry::ClearingSettings) -> f64 {
    clearing.light_bite(tool.diameter)
}

/// The moves that clear `plan` at depth `z`, coming down from `above`: each area entered
/// once (the way `entry` says, on the first stretch of cutting), then at depth throughout,
/// lifting only where the plan jumps. Ends at depth where the last cut ended.
fn adaptive_moves(plan: &[Piece], z: f64, above: f64, safe: f64, entry: &Entry) -> Vec<Move> {
    let mut moves: Vec<Move> = vec![];
    let mut entering = None;
    let at_z = |p: Point| p.extend(z);
    for piece in plan {
        match piece {
            Piece::Enter(p) => {
                // already down from an earlier start: straight up before going across
                if let Some(last) = moves.last().map(|m| m.to).filter(|at| at.z < safe) {
                    moves.push(Move {
                        kind: MoveKind::Rapid,
                        to: last.truncate().extend(safe),
                    });
                }
                entering = Some(*p);
            }
            Piece::Cut(points) if entering.is_some() => {
                entering = None;
                let mut down = path_moves(points, false, &[z], above, safe, entry);
                // the clearing carries on from where this ends, at depth
                while down.last().is_some_and(|m| m.kind == MoveKind::Rapid) {
                    down.pop();
                }
                moves.extend(down);
            }
            Piece::Cut(points) | Piece::Link(points) => {
                moves.extend(points.iter().skip(1).map(|&p| Move {
                    kind: MoveKind::Cut,
                    to: at_z(p),
                }));
            }
            Piece::Jump(p) => {
                if let Some(last) = moves.last().map(|m| m.to) {
                    moves.push(Move {
                        kind: MoveKind::Rapid,
                        to: last.truncate().extend(safe),
                    });
                }
                // down into space already cleared at this depth
                moves.push(Move {
                    kind: MoveKind::Rapid,
                    to: p.extend(safe),
                });
                moves.push(Move {
                    kind: MoveKind::Rapid,
                    to: p.extend(above),
                });
                moves.push(Move {
                    kind: MoveKind::Plunge,
                    to: at_z(*p),
                });
            }
        }
    }
    moves
}

/// One pass round each of the pocket's `rings` (turned the way they cut) at depth `z`, for a
/// clean wall after adaptive clearing: from the tool's place `from` (at depth, in the cleared
/// pocket) across to the nearest corner of each ring where the way is clear, else up and
/// over. Ends lifted to `safe`.
fn walls(
    rings: &[Vec<Point>],
    from: Option<cut_craft_core::glam::DVec3>,
    z: f64,
    safe: f64,
    entry: &Entry,
) -> Vec<Move> {
    let mut moves = vec![];
    let mut at = from;
    for ring in rings {
        let Some(here) = at else {
            // not down yet: the ring's own entry
            moves.extend(path_moves(ring, true, &[z], z, safe, entry));
            at = moves.last().map(|m| m.to);
            continue;
        };
        let Some((k, _)) = nearest_vertex(ring, here.truncate()) else {
            continue;
        };
        let mut ring = ring.clone();
        ring.rotate_left(k);
        let start = ring[0];
        let down = (here.z - z).abs() < 1e-9;
        let across = entry
            .room
            .as_ref()
            .is_none_or(|r| r.holds(here.truncate(), start));
        if !(down && across) {
            moves.push(Move {
                kind: MoveKind::Rapid,
                to: here.truncate().extend(safe),
            });
            moves.push(Move {
                kind: MoveKind::Rapid,
                to: start.extend(safe),
            });
            // the pocket is cleared to this depth: straight down into it
        }
        let kind = if down && across {
            MoveKind::Cut
        } else {
            MoveKind::Plunge
        };
        moves.push(Move {
            kind,
            to: start.extend(z),
        });
        moves.extend(ring.iter().skip(1).chain([&start]).map(|&p| Move {
            kind: MoveKind::Cut,
            to: p.extend(z),
        }));
        at = Some(start.extend(z));
    }
    if let Some(last) = moves.last().map(|m| m.to) {
        moves.push(Move {
            kind: MoveKind::Rapid,
            to: last.truncate().extend(safe),
        });
    }
    moves
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
/// the one inside, then plain rings for a middle too small for a band. `back`: each band runs
/// the other way round, its loops still turning `loops`' way.
fn trochoidal(edge: &[Path], p: &Params, loops: &Loops, back: bool) -> Vec<Path> {
    let (radius, advance) = loops.sized(p.tool.diameter);
    let step = p.tool.stepover * p.tool.diameter;
    let mut out = vec![];
    let mut band = polygon::offset_regions(edge, -radius, Corners::Round);
    let mut last = vec![];
    while !band.is_empty() {
        for ring in &band {
            let mut ring = oriented(p.direction, ring);
            if back {
                ring.reverse();
            }
            let points = trochoid(&ring, true, radius, advance, loops.anticlockwise());
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

/// A raster pocket's rows (across `walls_at`'s inner region) and its pass round `edge` when
/// it goes round at each pass, in the order they are cut, as one run so each links on to the
/// next; rows kept climb (or `p`'s direction) with `keep_climb`.
fn raster_run(
    walls_at: &EdgePass,
    edge: &[Path],
    step: f64,
    p: &Params,
    keep_climb: bool,
) -> Vec<Path> {
    let climb = p.direction == Direction::Climb;
    let lines = raster(
        &walls_at.inner,
        step,
        p.raster_angle,
        keep_climb.then_some(climb),
    );
    if !walls_at.each_pass {
        return lines;
    }
    match p.profile_pass {
        ProfilePass::First => [edge, &lines].concat(),
        ProfilePass::Last => [lines.as_slice(), edge].concat(),
        ProfilePass::None => lines,
    }
}

/// Raster lines `step` apart at `angle` across the region inside `edge`, every other one
/// back the other way; or, with `one_way` (climb or not), every row cutting that way, each
/// followed by the way back through it, so the next row starts where it did.
fn raster(edge: &[Path], step: f64, angle: f64, one_way: Option<bool>) -> Vec<Path> {
    let rows = hatch_lines(edge, step, angle);
    if let Some(climb) = one_way {
        // a row's spans run along the angle and the rows step to its left, the material
        // ahead of them there: climb (the material on the bit's right) runs back along it
        let spans: Vec<Path> = rows
            .into_iter()
            .flatten()
            .map(|(a, b)| {
                let (from, to) = if climb { (b, a) } else { (a, b) };
                Path::polyline(&[from, to], false)
            })
            .collect();
        let last = spans.len().saturating_sub(1);
        return spans
            .into_iter()
            .enumerate()
            .flat_map(|(i, span)| {
                let back = (i < last).then(|| span.reversed());
                std::iter::once(span).chain(back)
            })
            .collect();
    }
    rows.into_iter()
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

/// A bit Suggest may pick for clearing, and whether it is loaded in the tool changer.
#[derive(Clone, Debug)]
pub struct Candidate {
    pub tool: Tool,
    pub loaded: bool,
}

/// What a tool change costs, minutes: by hand, or by the tool changer.
#[derive(Clone, Copy, Debug)]
pub struct Changes {
    pub by_hand: f64,
    pub by_changer: f64,
}

/// Suggest's answer: the clearing bits that finish the pocket soonest (none when the
/// finishing bit alone is quickest), how long that takes, and how long the finishing bit
/// takes alone, minutes (cutting at `rapid` and changing bits included).
#[derive(Clone, Debug)]
pub struct Suggestion {
    pub clearing: Vec<Tool>,
    pub minutes: f64,
    pub alone: f64,
}

/// The most clearing bits of different sizes Suggest weighs (one at a time and in pairs).
const SUGGEST_SIZES: usize = 6;

/// Within this share of each other two plans count as equally quick: then the one with
/// more bits already in the tool changer wins, then the one with fewer bits.
const SAME_TIME: f64 = 0.03;

/// The quickest way to `p`'s pocket finished by its own tool: alone, after one bigger bit
/// from `candidates`, or after two. Bigger end mills only, one of each size (a loaded one
/// first); each plan is calculated as Calculate would make it and timed at `rapid`, with
/// `changes` for each bit change (the finishing bit's own `finishing_loaded`).
pub fn suggest(
    doc: &Document,
    p: &Params,
    candidates: &[Candidate],
    finishing_loaded: bool,
    rapid: f64,
    changes: Changes,
) -> Result<Suggestion, CommandError> {
    use cut_craft_core::toolpath::ToolShape;
    let usable = |t: &Tool| {
        t.shape == ToolShape::EndMill
            && t.diameter > p.tool.diameter + 1e-6
            && t.pass_depth > 0.0
            && t.stepover > 0.0
            && t.stepover <= 1.0
            && t.feed_rate > 0.0
    };
    // one of each size, a loaded one before one changed by hand
    let mut sizes: Vec<&Candidate> = vec![];
    for c in candidates.iter().filter(|c| usable(&c.tool)) {
        match sizes
            .iter_mut()
            .find(|k| (k.tool.diameter - c.tool.diameter).abs() < 1e-6)
        {
            Some(kept) if c.loaded && !kept.loaded => *kept = c,
            Some(_) => {}
            None => sizes.push(c),
        }
    }
    sizes.sort_by(|a, b| b.tool.diameter.total_cmp(&a.tool.diameter));
    // too many: an even spread of sizes, biggest and smallest kept
    if sizes.len() > SUGGEST_SIZES {
        let last = sizes.len() - 1;
        sizes = (0..SUGGEST_SIZES)
            .map(|i| sizes[i * last / (SUGGEST_SIZES - 1)])
            .collect();
    }
    let mut plans: Vec<Vec<&Candidate>> = vec![vec![]];
    for (i, a) in sizes.iter().enumerate() {
        plans.push(vec![a]);
        for b in &sizes[i + 1..] {
            plans.push(vec![a, b]);
        }
    }
    let rapid = rapid.max(1.0);
    let time = |plan: &[&Candidate]| -> Option<f64> {
        let tried = Params {
            clearance_tools: plan.iter().map(|c| c.tool.clone()).collect(),
            ..p.clone()
        };
        let change = PocketToolpath::run(doc, &tried).ok()?;
        let cutting: f64 = change
            .edits
            .iter()
            .filter_map(|e| match e {
                cut_craft_core::Edit::AddToolpath { toolpath, .. } => {
                    Some(toolpath.machining_minutes(rapid))
                }
                _ => None,
            })
            .sum();
        // a change to each bit after the first: the ones after the first clearing bit,
        // then the finishing bit
        let incoming = plan
            .iter()
            .skip(1)
            .map(|c| c.loaded)
            .chain((!plan.is_empty()).then_some(finishing_loaded));
        let changing: f64 = incoming
            .map(|loaded| {
                if loaded && plan.iter().all(|c| c.loaded) && finishing_loaded {
                    changes.by_changer
                } else {
                    changes.by_hand
                }
            })
            .sum();
        Some(cutting + changing)
    };
    let alone = time(&[]).ok_or_else(|| {
        // the finishing bit alone fails: say why, as Calculate would
        PocketToolpath::run(doc, p)
            .err()
            .unwrap_or_else(|| CommandError::Failed("the pocket could not be timed".into()))
    })?;
    let mut best = (Vec::<&Candidate>::new(), alone);
    for plan in plans.iter().skip(1) {
        let Some(minutes) = time(plan) else { continue };
        let loaded = |plan: &[&Candidate]| plan.iter().filter(|c| c.loaded).count();
        let better = if minutes < best.1 * (1.0 - SAME_TIME) {
            true
        } else if minutes <= best.1 * (1.0 + SAME_TIME) {
            (loaded(plan), std::cmp::Reverse(plan.len()))
                > (loaded(&best.0), std::cmp::Reverse(best.0.len()))
                && !best.0.is_empty()
        } else {
            false
        };
        if better {
            best = (plan.clone(), minutes);
        }
    }
    Ok(Suggestion {
        clearing: best.0.iter().map(|c| c.tool.clone()).collect(),
        minutes: best.1,
        alone,
    })
}

#[cfg(test)]
mod tests;
