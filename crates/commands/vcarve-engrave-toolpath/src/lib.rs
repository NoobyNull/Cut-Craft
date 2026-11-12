//! V-Carve / Engrave Toolpath (V-bit carving): a V-bit carves the selected shapes, going deeper the
//! further it is from their edges so its sides cut the edges exactly.

use cut_craft_core::document::Document;
use cut_craft_core::geometry::{Path, Point, inside};
use cut_craft_core::glam::DVec3;
use cut_craft_core::medial;
use cut_craft_core::polygon::{self, Corners};
use cut_craft_core::toolpath::{
    Move, MoveKind, Tool, ToolDatabase, ToolShape, contour_moves, cut_points, made_by,
    project_onto_model,
};
use cut_craft_core::{Change, Command, CommandError, Meta, ensure_positive};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub struct VcarveEngraveToolpath;

/// Where the V-bit carves, measured from the vectors.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Place {
    /// Carve the whole shape: the tip runs down its middle, as deep as its width needs, so
    /// the sides meet in a sharp crease (the usual V-carved lettering)
    #[default]
    #[schemars(title = "Fill")]
    Fill,
    /// A V groove just inside each shape's edge, one side of the V on the vector, as deep as
    /// set
    #[schemars(title = "Inside")]
    Inside,
    /// The tip follows the vectors themselves, open ones too, as deep as set
    #[schemars(title = "On the Line")]
    OnTheLine,
    /// A V groove just outside each shape's edge, one side of the V on the vector, as deep as
    /// set; the shape stays its full size
    #[schemars(title = "Outside")]
    Outside,
}

fn one() -> f64 {
    1.0
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
pub struct Params {
    /// What the toolpath is called in the toolpath list
    pub name: String,
    /// Below the material surface where carving starts (the floor of an earlier pocket)
    #[serde(default)]
    #[schemars(extend("x-unit" = "mm"), range(min = 0))]
    #[schemars(extend("x-group" = "Cutting Depths"))]
    pub start_depth: f64,
    /// Deepest the bit goes below the start depth; wider parts are cut flat there (0: as
    /// deep as it can)
    #[schemars(extend("x-unit" = "mm"), range(min = 0))]
    #[schemars(extend("x-group" = "Cutting Depths", "x-show-when" = {"place": "fill"}))]
    pub flat_depth: f64,
    /// Fill carves the whole shape, as deep as its width needs; Inside, On the Line and
    /// Outside carve a V groove of the depth set
    #[serde(default)]
    #[schemars(title = "Carve", extend("x-group" = "Cutting Depths"))]
    pub place: Place,
    /// How deep the groove's point goes below the start depth (Fill works its depth out
    /// from each shape's width instead)
    #[serde(default = "one")]
    #[schemars(
        title = "Groove depth",
        extend("x-unit" = "mm", "x-group" = "Cutting Depths", "x-hide-when" = {"place": "fill"}),
        range(min = 0)
    )]
    pub depth: f64,
    /// A V-bit; its stepover sets how close together the carving passes are
    pub tool: Tool,
    /// End mills that clear the flat bottom first, largest first (needs a flat depth): each
    /// makes a toolpath of its own, and each tool after it, the V-bit last, cuts only what
    /// the ones before it left (rest machining)
    #[serde(default)]
    #[schemars(title = "Clearance Tools")]
    pub clearance_tools: Vec<Tool>,
    /// This toolpath is this clearance tool's pass over the flat bottom
    #[serde(default)]
    #[schemars(skip)]
    pub clearing: Option<Tool>,
    /// The V-bit's toolpath after clearance tools: their smallest radius, so it carves the
    /// slopes and only this far into the flat bottom they cleared
    #[serde(default)]
    #[schemars(skip)]
    pub cleared_within: Option<f64>,
    /// After calculating, drop the toolpath onto the 3D model: its depths are then below the
    /// model's surface, going no lower than the bottom of the material
    #[serde(default)]
    #[schemars(title = "Project toolpath onto 3D Model")]
    pub project_onto_model: bool,
    /// Each pass starts as near as it can to the start point of the vector it follows;
    /// otherwise where it starts is left to the program
    #[serde(default)]
    pub use_vector_start_points: bool,
    /// Carve the shapes one after another in the order they were selected; otherwise all
    /// are carved together, a pass depth at a time
    #[serde(default)]
    pub use_vector_selection_order: bool,
}

impl Default for Params {
    fn default() -> Self {
        Self {
            name: "V-Carve".into(),
            start_depth: 0.0,
            flat_depth: 0.0,
            place: Place::Fill,
            depth: 1.0,
            tool: ToolDatabase::starter()
                .tools
                .into_iter()
                .find(|t| t.shape == ToolShape::VBit)
                .expect("the starter tools include a V-bit"),
            clearance_tools: vec![],
            clearing: None,
            cleared_within: None,
            project_onto_model: false,
            use_vector_start_points: false,
            use_vector_selection_order: false,
        }
    }
}

impl Command for VcarveEngraveToolpath {
    const ID: &'static str = "vcarve_engrave_toolpath";
    type Params = Params;

    fn meta() -> Meta {
        Meta {
            label: "V-Carve / Engrave Toolpath",
            group: "Toolpath Operations",
            icon: "toolpaths/vcarve_engrave_toolpath.png",
            shortcut: "",
        }
    }

    fn run(doc: &Document, p: &Params) -> Result<Change, CommandError> {
        let mut c = Change::builder(doc);
        if p.clearance_tools.is_empty() {
            c.add_toolpath(vcarve(doc, p)?);
            return Ok(c.finish());
        }
        if p.place != Place::Fill {
            return Err(CommandError::InvalidParams(
                "clearance tools clear a Fill's flat bottom: set Carve to Fill, or take them off"
                    .into(),
            ));
        }
        if p.flat_depth <= 0.0 {
            return Err(CommandError::InvalidParams(
                "clearance tools clear the flat bottom: give a flat depth".into(),
            ));
        }
        cut_craft_core::rest::check_clearing(&p.tool, &p.clearance_tools, false)?;
        // each clearance tool, largest first (each kept to what the ones before it left),
        // then the V-bit on the slopes and the edge of the flat bottom
        let mut sorted = p.clearance_tools.clone();
        sorted.sort_by(|a, b| b.diameter.total_cmp(&a.diameter));
        let p = &Params {
            clearance_tools: sorted,
            ..p.clone()
        };
        let smallest = p
            .clearance_tools
            .iter()
            .map(|t| t.diameter / 2.0)
            .fold(f64::INFINITY, f64::min);
        for (i, tool) in p.clearance_tools.iter().enumerate() {
            let own = Params {
                name: format!("{} [{}]", p.name, i + 1),
                clearance_tools: vec![],
                clearing: Some(tool.clone()),
                ..p.clone()
            };
            let mut toolpath = vcarve(doc, &own)?;
            if i > 0 {
                let rest = cut_craft_core::rest::Rest {
                    after: p.clearance_tools[..i].to_vec(),
                    ..Default::default()
                };
                toolpath.params["rest"] =
                    serde_json::to_value(&rest).expect("rest settings serialise");
                let params = toolpath.params.clone();
                cut_craft_core::rest::apply(&doc.job, &mut toolpath, &params)?;
            }
            c.add_toolpath(toolpath);
        }
        let vbit = Params {
            name: format!("{} [{}]", p.name, p.clearance_tools.len() + 1),
            clearance_tools: vec![],
            cleared_within: Some(smallest),
            ..p.clone()
        };
        c.add_toolpath(vcarve(doc, &vbit)?);
        Ok(c.finish())
    }
}

/// The toolpath for `p`'s own tool: the V-bit's carving, or (`clearing`) an end mill's
/// pocket of the flat bottom.
fn vcarve(doc: &Document, p: &Params) -> Result<cut_craft_core::toolpath::Toolpath, CommandError> {
    {
        let tool = &p.tool;
        if let Some(clearer) = &p.clearing {
            return clearing(doc, p, clearer);
        }
        if tool.shape != ToolShape::VBit {
            return Err(CommandError::InvalidParams(
                "pick a V-bit: V-carving needs one".into(),
            ));
        }
        ensure_positive("tool diameter", tool.diameter)?;
        ensure_positive("pass depth", tool.pass_depth)?;
        let stepover = tool.finishing_stepover();
        if !(stepover > 0.0 && stepover <= 1.0) {
            return Err(CommandError::InvalidParams(
                "the tool's finishing stepover must be more than 0% and at most 100%: set it in the Tool Database".into(),
            ));
        }
        // on the line follows open vectors too
        let on_line = p.place == Place::OnTheLine;
        let (ids, paths) = doc.selected_paths(|path| on_line || path.closed);
        if ids.is_empty() {
            return Err(CommandError::Failed(if on_line {
                "select the vectors to carve along".into()
            } else {
                "select the closed vectors to carve".into()
            }));
        }
        if p.place != Place::Fill {
            ensure_positive("groove depth", p.depth)?;
            let widest = (tool.diameter / 2.0) / (tool.angle.to_radians() / 2.0).tan();
            if p.depth > widest + 1e-9 {
                return Err(CommandError::InvalidParams(format!(
                    "{} carves a groove at most {widest:.2} mm deep before its cone is as wide as \
                     the bit: set the groove depth to that or less, or pick a wider bit",
                    tool.name
                )));
            }
        }
        if p.start_depth < 0.0 {
            return Err(CommandError::InvalidParams(
                "set start depth to 0 or more".into(),
            ));
        }
        // dropped onto the model it is kept above the material's bottom there instead
        let bottom =
            (!p.project_onto_model && doc.job.thickness > 0.0).then_some(doc.job.thickness);
        let mut moves = carve_above(&paths, p, doc.job.clearance, bottom);
        if p.project_onto_model {
            moves = project_onto_model(doc, tool, &moves)?;
        }
        Ok(made_by::<VcarveEngraveToolpath>(
            p,
            p.name.clone(),
            tool.clone(),
            ids,
            moves,
        ))
    }
}

/// A clearance tool's pass: the flat bottom (where the V-bit, `p.flat_depth` down, is
/// narrower than the shape) pocketed with rings stepping in, at each pass depth.
fn clearing(
    doc: &Document,
    p: &Params,
    tool: &Tool,
) -> Result<cut_craft_core::toolpath::Toolpath, CommandError> {
    ensure_positive("tool diameter", tool.diameter)?;
    ensure_positive("pass depth", tool.pass_depth)?;
    let (ids, paths) = doc.selected_paths(|path| path.closed);
    if ids.is_empty() {
        return Err(CommandError::Failed(
            "select the closed vectors to carve".into(),
        ));
    }
    // the V-bit's cone at the flat depth bounds the flat bottom
    let spread = (p.tool.angle.to_radians() / 2.0).tan();
    let flat = polygon::offset_regions(
        &polygon::regions(&paths),
        -(p.flat_depth * spread + tool.diameter / 2.0),
        Corners::Round,
    );
    if flat.is_empty() {
        return Err(CommandError::Failed(format!(
            "{} is too big to clear any of the flat bottom. Pick a smaller flat-bottom tool",
            tool.name
        )));
    }
    let step = tool.stepover * tool.diameter;
    let depths = tool.pass_depths(p.start_depth + p.flat_depth);
    let mut rings = vec![];
    let mut ring = flat;
    while !ring.is_empty() {
        // as regions, so a shape inside another (the middle of an "O") stays an island
        let next = polygon::offset_regions_again(&ring, -step);
        rings.extend(ring);
        ring = next;
    }
    rings.reverse(); // from the middle outwards
    let moves = rings
        .iter()
        .flat_map(|r| contour_moves(&cut_points(r), true, &depths, doc.job.clearance))
        .collect();
    Ok(made_by::<VcarveEngraveToolpath>(
        p,
        p.name.clone(),
        tool.clone(),
        ids,
        moves,
    ))
}

/// The moves that V-carve the regions `paths` enclose (holes left standing) with `p`'s V-bit.
/// The edges are cut at the start depth; further in it goes deeper, the tip as far below the
/// start depth as half the shape's width there over the tangent of half the bit's angle,
/// until the cone is as wide as the bit or reaches the flat depth.
/// With the vector selection order, each outline (and the vectors inside it) is carved
/// completely before the next one selected.
pub fn carve(paths: &[Path], p: &Params, safe: f64) -> Vec<Move> {
    carve_above(paths, p, safe, None)
}

/// [`carve`] going no deeper than `bottom` below the material's top, when given.
fn carve_above(paths: &[Path], p: &Params, safe: f64, bottom: Option<f64>) -> Vec<Move> {
    if p.place == Place::OnTheLine {
        return along_the_lines(paths, p, safe, bottom);
    }
    if !p.use_vector_selection_order {
        return carve_together(paths, p, safe, bottom);
    }
    shapes(paths)
        .iter()
        .flat_map(|shape| carve_together(shape, p, safe, bottom))
        .collect()
}

/// The tip along each of `paths` (in their order), `p.depth` below the start depth (no
/// deeper than `bottom`), a pass depth at a time.
fn along_the_lines(paths: &[Path], p: &Params, safe: f64, bottom: Option<f64>) -> Vec<Move> {
    let depth = (p.start_depth + p.depth).min(bottom.unwrap_or(f64::INFINITY));
    let depths = p.tool.pass_depths(depth);
    paths
        .iter()
        .flat_map(|path| contour_moves(&cut_points(path), path.closed, &depths, safe))
        .collect()
}

/// The paths grouped as shapes, in their order: each outline not inside another, with the
/// paths inside it (its holes and their islands).
fn shapes(paths: &[Path]) -> Vec<Vec<Path>> {
    let rings: Vec<Vec<Point>> = paths.iter().map(cut_points).collect();
    let within = |i: usize, j: usize| i != j && inside(&rings[j..=j], rings[i][0]);
    (0..paths.len())
        .filter(|&i| !(0..paths.len()).any(|j| within(i, j)))
        .map(|i| {
            (0..paths.len())
                .filter(|&k| k == i || within(k, i))
                .map(|k| paths[k].clone())
                .collect()
        })
        .collect()
}

/// How closely the shapes' edges are followed to find their centre lines, mm: the crease
/// is within about this of the true middle, and runs to within about this of each corner.
const CENTRE_SPACING: f64 = 0.05;

/// The carving of all `paths` together, a pass depth at a time. In each pass the tip runs
/// along the shapes' centre lines, as deep as puts its cone's sides on both edges (no deeper
/// than the pass), so a stroke narrower than the bit is carved in one run down its middle,
/// to a crease, right into its corners. Where the shape is wider than the cone reaches (or
/// than the flat depth allows), the pass's flat floor is cut with rings a stepover apart.
/// Each pass only goes where the carving is deeper than the pass before it, so no pass
/// cuts air, and no cut goes more than a pass depth below the one before.
fn carve_together(paths: &[Path], p: &Params, safe: f64, bottom: Option<f64>) -> Vec<Move> {
    let starts: Vec<Point> = if p.use_vector_start_points {
        paths.iter().map(|path| path.start).collect()
    } else {
        vec![]
    };
    let tool = &p.tool;
    // how far the cone reaches sideways for each millimetre down
    let spread = (tool.angle.to_radians() / 2.0).tan();
    // further in than this the cone cannot reach the edges, so the bottom is cut flat
    let mut reach = match p.flat_depth {
        limit if limit > 0.0 && p.place == Place::Fill => (tool.diameter / 2.0).min(limit * spread),
        _ => tool.diameter / 2.0,
    };
    // and never below the material
    if let Some(bottom) = bottom {
        reach = reach.min(((bottom - p.start_depth) * spread).max(0.0));
    }
    // the flat bottom's middle, cleared by end mills, is left alone
    let cleared = p.cleared_within.map_or(f64::INFINITY, |r| reach + r);
    let regions = grooved(&polygon::regions(paths), p.place, p.depth * spread);
    let lines = medial::centre_lines(&regions, CENTRE_SPACING);
    let widest_shape = lines
        .iter()
        .flat_map(|l| l.radii.iter().copied())
        .fold(0.0, f64::max);
    let widest = widest_shape.min(reach);
    if widest <= 0.0 {
        return vec![];
    }
    let depth_at = |r: f64| -(p.start_depth + r.min(reach) / spread);
    let mut moves = vec![];
    let mut above = -p.start_depth;
    // the first run starts by the first vector's start point, when asked
    let mut from = starts
        .first()
        .copied()
        .unwrap_or(Point::new(f64::NAN, f64::NAN));
    let levels = tool.pass_depths(p.start_depth + widest / spread);
    let last_level = levels.last().copied().unwrap_or(above);
    for level in levels {
        if level >= above - 1e-9 {
            continue;
        }
        // the centre lines, where they are deeper than the last pass, down to this one
        let runs = centre_runs(
            &lines,
            level,
            |r| depth_at(r) < above - 1e-9 && r <= cleared,
            depth_at,
        );
        let mut runs = nearest_first(runs, from);
        // the floor: where this pass is too deep for the cone to reach the edges
        let floor_reach = ((-level - p.start_depth) * spread).min(reach);
        // (none where only the centre line itself is that far from the edges)
        if floor_reach > 1e-6 && floor_reach < widest_shape - CENTRE_SPACING {
            let floor = polygon::offset_regions(&regions, -floor_reach, Corners::Round);
            let finishing = (level - last_level).abs() < 1e-9;
            // a pass on the way down only has to leave ridges low enough for the next
            // pass to take with the rest: a quarter of a pass depth
            let step = if finishing {
                tool.finishing_stepover() * tool.diameter
            } else {
                (tool.pass_depth * spread / 2.0)
                    .min(tool.diameter / 2.0)
                    .max(tool.finishing_stepover() * tool.diameter)
            };
            let mut rings = floor_rings(&floor, step, cleared - floor_reach, level, &starts);
            // from the middle out, the edge of the floor last
            rings.reverse();
            runs.extend(rings);
        }
        if let Some(last) = runs.last().and_then(Run::end) {
            from = last;
        }
        moves.extend(run_moves(&runs, safe));
        above = level;
    }
    moves
}

/// The stretches of `lines` where `deeper` (given the distance to the edge) holds, one point
/// more each end to come up to where the last pass was, at `depth_at` but no lower than
/// `level`.
fn centre_runs(
    lines: &[medial::CentreLine],
    level: f64,
    deeper: impl Fn(f64) -> bool,
    depth_at: impl Fn(f64) -> f64,
) -> Vec<Run> {
    let mut runs = vec![];
    for line in lines {
        let count = line.points.len();
        let mut i = 0;
        while i < count {
            if !deeper(line.radii[i]) {
                i += 1;
                continue;
            }
            let first = i.saturating_sub(1);
            while i < count && deeper(line.radii[i]) {
                i += 1;
            }
            let points: Vec<DVec3> = (first..(i + 1).min(count))
                .map(|k| {
                    let q = line.points[k];
                    DVec3::new(q.x, q.y, depth_at(line.radii[k]).max(level))
                })
                .collect();
            if points.len() > 1 {
                runs.push(Run {
                    points,
                    closed: false,
                });
            }
        }
    }
    runs
}

/// Rings round `floor` at `level`, its edge and then a `step` further in each time while no
/// more than `within` in, each started nearest `starts`.
fn floor_rings(floor: &[Path], step: f64, within: f64, level: f64, starts: &[Point]) -> Vec<Run> {
    let mut sets = vec![(0.0, floor.to_vec())];
    // the centre line runs down the middle of each: no ring where one vanishes
    if !floor.is_empty() && step > 0.0 {
        sets.extend(stepped_in(floor, step));
    }
    sets.into_iter()
        .take_while(|(offset, _)| *offset <= within)
        .flat_map(|(_, set)| set)
        .map(|ring| Run {
            points: started_near(cut_points(&ring), starts)
                .into_iter()
                .map(|q| DVec3::new(q.x, q.y, level))
                .collect(),
            closed: true,
        })
        .collect()
}

/// What `place` carves of `regions`: all of them for Fill, or the band `2 half_width` wide
/// inside or outside their edges (its middle `half_width` from them), so filling it carves a
/// groove `half_width` from the edge with sharp corners.
fn grooved(regions: &[Path], place: Place, half_width: f64) -> Vec<Path> {
    let width = 2.0 * half_width;
    match place {
        Place::Fill | Place::OnTheLine => regions.to_vec(),
        Place::Inside => polygon::difference(
            regions,
            &polygon::offset_regions(regions, -width, Corners::Round),
        ),
        Place::Outside => polygon::difference(
            &polygon::offset_regions(regions, width, Corners::Round),
            regions,
        ),
    }
}

/// A stretch the tip cuts along without lifting.
struct Run {
    points: Vec<DVec3>,
    closed: bool,
}

impl Run {
    fn end(&self) -> Option<Point> {
        let last = if self.closed {
            self.points.first()
        } else {
            self.points.last()
        }?;
        Some(last.truncate())
    }
}

/// The runs in the order that goes next to the nearest one still to cut (either way
/// along), starting nearest `from`. Their ends are kept in a grid, so a design of thousands
/// of shapes is ordered in moments.
fn nearest_first(mut runs: Vec<Run>, mut from: Point) -> Vec<Run> {
    const CELL: f64 = 2.0;
    #[expect(
        clippy::cast_possible_truncation,
        reason = "grid cells of a design on a machine bed"
    )]
    let cell = |q: Point| ((q.x / CELL).floor() as i64, (q.y / CELL).floor() as i64);
    let mut grid: std::collections::HashMap<(i64, i64), Vec<(usize, bool)>> =
        std::collections::HashMap::new();
    let (mut low, mut high) = ((i64::MAX, i64::MAX), (i64::MIN, i64::MIN));
    for (i, run) in runs.iter().enumerate() {
        let ends = [
            (run.points[0], false),
            (*run.points.last().expect("runs have points"), true),
        ];
        for (end, flip) in ends {
            let c = cell(end.truncate());
            low = (low.0.min(c.0), low.1.min(c.1));
            high = (high.0.max(c.0), high.1.max(c.1));
            grid.entry(c).or_default().push((i, flip));
        }
    }
    let mut used = vec![false; runs.len()];
    let mut order: Vec<(usize, bool)> = Vec::with_capacity(runs.len());
    if !from.is_finite() {
        from = runs.first().map_or(Point::ZERO, |r| r.points[0].truncate());
    }
    while order.len() < runs.len() {
        let (cx, cy) = cell(from);
        let mut best: Option<(usize, bool, f64)> = None;
        let mut ring = 0i64;
        loop {
            // every cell `ring` cells round from's
            for x in cx - ring..=cx + ring {
                for y in cy - ring..=cy + ring {
                    if (x - cx).abs() != ring && (y - cy).abs() != ring {
                        continue;
                    }
                    for &(i, flip) in grid.get(&(x, y)).into_iter().flatten() {
                        if used[i] {
                            continue;
                        }
                        let end = if flip {
                            *runs[i].points.last().expect("runs have points")
                        } else {
                            runs[i].points[0]
                        };
                        let d = end.truncate().distance(from);
                        if best.is_none_or(|b| d < b.2) {
                            best = Some((i, flip, d));
                        }
                    }
                }
            }
            #[expect(clippy::cast_precision_loss, reason = "a small ring count")]
            let reached = ring as f64 * CELL;
            let outside = cx - ring <= low.0
                && cy - ring <= low.1
                && cx + ring >= high.0
                && cy + ring >= high.1;
            if best.is_some_and(|b| b.2 <= reached) || outside {
                break;
            }
            ring += 1;
        }
        let (i, flip, _) = best.expect("runs are left");
        used[i] = true;
        order.push((i, flip));
        let run = &runs[i];
        // where it ends: a ring where it starts, a line at its other end
        let last = *run.points.last().expect("runs have points");
        let end = if run.closed == flip {
            last
        } else {
            run.points[0]
        };
        from = end.truncate();
    }
    let mut taken: Vec<Option<Run>> = runs.drain(..).map(Some).collect();
    order
        .into_iter()
        .map(|(i, flip)| {
            let mut run = taken[i].take().expect("each run once");
            if flip {
                run.points.reverse();
            }
            run
        })
        .collect()
}

/// The moves cutting `runs` in order: one running on from where the last ended continues
/// without lifting; otherwise up to `safe`, across, and down.
fn run_moves(runs: &[Run], safe: f64) -> Vec<Move> {
    let mut moves: Vec<Move> = vec![];
    let mut at: Option<DVec3> = None;
    for run in runs {
        let Some(&start) = run.points.first() else {
            continue;
        };
        let joined = at.is_some_and(|a| a.distance(start) < 1e-9);
        if !joined {
            if at.is_some() {
                let up = moves.last().expect("a cut came before").to;
                moves.push(Move {
                    kind: MoveKind::Rapid,
                    to: DVec3::new(up.x, up.y, safe),
                });
            }
            moves.push(Move {
                kind: MoveKind::Rapid,
                to: DVec3::new(start.x, start.y, safe),
            });
            moves.push(Move {
                kind: MoveKind::Plunge,
                to: start,
            });
        }
        let back = run.closed.then_some(start);
        moves.extend(run.points[1..].iter().chain(back.as_ref()).map(|&to| Move {
            kind: MoveKind::Cut,
            to,
        }));
        at = moves.last().map(|m| m.to);
    }
    if let Some(last) = moves.last().map(|m| m.to) {
        moves.push(Move {
            kind: MoveKind::Rapid,
            to: DVec3::new(last.x, last.y, safe),
        });
    }
    moves
}

/// The closed ring of `points` begun at its point nearest any of `starts` (as it is when
/// there are none).
fn started_near(mut points: Vec<Point>, starts: &[Point]) -> Vec<Point> {
    let distance = |q: &Point| {
        starts
            .iter()
            .map(|s| s.distance(*q))
            .fold(f64::INFINITY, f64::min)
    };
    let nearest =
        (0..points.len()).min_by(|&a, &b| distance(&points[a]).total_cmp(&distance(&points[b])));
    if let (Some(nearest), false) = (nearest, starts.is_empty()) {
        points.rotate_left(nearest);
    }
    points
}

/// The regions shrunk by `step`, `2 step`, ... until nothing is left. Each shrinks the one
/// before it by a step (shrinking by a then by b, with round corners, is shrinking by a + b),
/// so later ones work on less and less: shrinking the whole design each time grew slower the
/// deeper it went, a minute on a 3,000-shape SVG.
fn stepped_in(regions: &[Path], step: f64) -> Vec<(f64, Vec<Path>)> {
    let mut offsets = vec![];
    let mut offset = step;
    let mut rings = polygon::offset_regions_again(regions, -step);
    while !rings.is_empty() {
        let next = polygon::offset_regions_again(&rings, -step);
        offsets.push((offset, rings));
        rings = next;
        offset += step;
    }
    offsets
}

#[cfg(test)]
mod tests {
    use super::*;
    use cut_craft_core::document::Job;
    use cut_craft_core::geometry::polyline_distance;
    use cut_craft_core::simulation::Material;
    use cut_craft_core::testing::{made_toolpath, square};
    use cut_craft_core::toolpath::Toolpath;

    cut_craft_core::contract_tests!(VcarveEngraveToolpath);

    fn moved(path: &Path, by: Point) -> Path {
        path.transformed(&cut_craft_core::glam::DAffine2::from_translation(by))
    }

    #[test]
    fn in_selection_order_each_shape_is_carved_before_the_next() {
        let right = moved(&rectangle(10.0, 10.0), Point::new(40.0, 0.0));
        let left = rectangle(10.0, 10.0);
        let p = Params {
            use_vector_selection_order: true,
            ..Params::default()
        };
        let cuts = |t: &Toolpath| -> Vec<bool> {
            t.moves
                .iter()
                .filter(|m| m.kind == MoveKind::Cut)
                .map(|m| m.to.x > 30.0)
                .collect()
        };
        let ordered = cuts(&carve(vec![right.clone(), left.clone()], &p));
        let first_left = ordered.iter().position(|&r| !r).unwrap();
        assert!(
            ordered[..first_left].iter().all(|&r| r),
            "the right one first"
        );
        assert!(
            ordered[first_left..].iter().all(|&r| !r),
            "then the left, once"
        );
        // a letter with a hole stays one shape
        let hole = moved(&rectangle(4.0, 4.0), Point::new(3.0, 3.0));
        assert_eq!(
            shapes(&[left, hole, right])
                .iter()
                .map(Vec::len)
                .collect::<Vec<_>>(),
            [2, 1]
        );
    }

    #[test]
    fn with_vector_start_points_the_first_pass_starts_by_the_vectors_start() {
        // the square starts at its top right corner
        let path = Path::polyline(
            &[
                Point::new(20.0, 20.0),
                Point::new(0.0, 20.0),
                Point::new(0.0, 0.0),
                Point::new(20.0, 0.0),
            ],
            true,
        );
        let start = |use_vector_start_points| {
            let t = carve(
                vec![path.clone()],
                &Params {
                    use_vector_start_points,
                    ..Params::default()
                },
            );
            let m = t.moves.iter().find(|m| m.kind != MoveKind::Rapid).unwrap();
            Point::new(m.to.x, m.to.y)
        };
        assert!(
            start(true).distance(Point::new(20.0, 20.0)) < 1.0,
            "{}",
            start(true)
        );
    }

    fn rectangle(width: f64, height: f64) -> Path {
        Path::polyline(
            &[
                Point::new(0.0, 0.0),
                Point::new(width, 0.0),
                Point::new(width, height),
                Point::new(0.0, height),
            ],
            true,
        )
    }

    fn carve(paths: Vec<Path>, params: &Params) -> Toolpath {
        made_toolpath::<VcarveEngraveToolpath>(paths, params).unwrap()
    }

    fn deepest(t: &Toolpath) -> f64 {
        t.moves.iter().map(|m| m.to.z).fold(f64::INFINITY, f64::min)
    }

    /// The material of a 100 mm long job after the toolpath, in quarter-millimetre cells.
    fn carved(t: &Toolpath, job: &Job) -> Material {
        let mut m = Material::new(job, 400);
        m.cut(t);
        m
    }

    #[test]
    fn the_clearance_tool_goes_round_a_shape_inside_another() {
        use cut_craft_core::testing::{document_with, square};
        let p = Params {
            flat_depth: 3.0,
            clearance_tools: vec![Tool::default()],
            ..Params::default()
        };
        let mut doc = document_with(vec![rectangle(60.0, 40.0), square(25.0, 15.0, 10.0)]);
        doc.apply(&VcarveEngraveToolpath::run(&doc, &p).unwrap())
            .unwrap();
        let mill = doc.toolpaths.values().next().unwrap();
        let island = Point::new(30.0, 20.0);
        let nearest = mill
            .moves
            .windows(2)
            .filter(|w| w[1].kind == MoveKind::Cut)
            .map(|w| {
                cut_craft_core::geometry::polyline_distance(
                    &[w[0].to.truncate(), w[1].to.truncate()],
                    island,
                )
            })
            .fold(f64::MAX, f64::min);
        // the island's 5 mm half-width and the mill's 3 mm radius
        assert!(
            nearest >= 8.0 - 0.05,
            "the mill cuts {nearest} mm from the island's middle"
        );
    }

    #[test]
    fn a_clearance_tool_clears_the_flat_bottom_and_the_v_bit_cuts_what_it_left() {
        use cut_craft_core::testing::document_with;
        let mill = Tool {
            name: "6 mm End Mill".into(),
            ..Tool::default()
        };
        let p = Params {
            flat_depth: 3.0,
            clearance_tools: vec![mill],
            ..Params::default()
        };
        let mut doc = document_with(vec![rectangle(60.0, 40.0)]);
        doc.apply(&VcarveEngraveToolpath::run(&doc, &p).unwrap())
            .unwrap();
        let made: Vec<&Toolpath> = doc.toolpaths.values().collect();
        assert_eq!(made.len(), 2);
        assert_eq!(made[0].tool.name, "6 mm End Mill");
        assert!(
            (deepest(made[0]) + 3.0).abs() < 1e-9,
            "flat at the flat depth"
        );
        assert_eq!(made[1].tool.shape, ToolShape::VBit);
        let alone = carve(
            vec![rectangle(60.0, 40.0)],
            &Params {
                flat_depth: 3.0,
                ..Params::default()
            },
        );
        assert!(
            made[1].moves.len() < alone.moves.len(),
            "the V-bit leaves the middle to the end mill"
        );
        // together they carve what the V-bit alone would: flat in the middle, sloped walls
        let job = Job {
            width: 60.0,
            height: 40.0,
            ..Job::default()
        };
        let both = {
            let mut m = Material::new(&job, 240);
            m.cut(made[0]);
            m.cut(made[1]);
            m
        };
        let solo = carved(&alone, &job);
        for (x, y) in [(30.0, 20.0), (8.0, 20.0), (1.0, 20.0), (4.5, 4.5)] {
            let (a, b) = (
                both.at(Point::new(x, y)).unwrap(),
                solo.at(Point::new(x, y)).unwrap(),
            );
            assert!(
                (a - b).abs() < 0.3,
                "at ({x}, {y}): {a} with clearance, {b} alone"
            );
        }
        // a clearance tool needs a flat bottom to clear
        let no_flat = Params {
            flat_depth: 0.0,
            ..p
        };
        assert!(VcarveEngraveToolpath::run(&doc, &no_flat).is_err());
    }

    #[test]
    fn a_90_degree_bit_carves_a_slot_as_deep_as_half_its_width() {
        let t = carve(vec![rectangle(100.0, 10.0)], &Params::default());
        let step = t.tool.finishing_stepover() * t.tool.diameter;
        assert!((deepest(&t) + 5.0).abs() <= step, "{}", deepest(&t));
        let job = Job {
            width: 100.0,
            height: 10.0,
            ..Job::default()
        };
        // depth is the distance in from the edge; within a pass spacing plus a cell
        let m = carved(&t, &job);
        for (y, depth) in [(1.0, 1.0), (3.0, 3.0), (5.0, 5.0)] {
            let z = m.at(Point::new(50.0, y)).unwrap();
            assert!((z + depth).abs() <= step + 0.25, "at y {y}: {z}");
        }
    }

    /// A 3/4 inch 90° V-bit stepping over 6.2%: 1.18 mm between rings.
    fn big_bit() -> Params {
        let mut p = Params::default();
        p.tool.diameter = 19.05;
        p.tool.stepover = 0.062;
        p.tool.finish_stepover = 0.062;
        p
    }

    fn strip(y: f64, width: f64) -> Path {
        moved(&rectangle(40.0, width), Point::new(0.0, y))
    }

    #[test]
    fn strokes_of_every_width_are_carved_to_a_crease_down_their_middle() {
        // thin strokes beside a wide one used to be missed, or stop a step short of the middle
        let t = carve(
            vec![strip(0.0, 2.0), strip(5.0, 3.6), strip(12.0, 10.0)],
            &big_bit(),
        );
        let job = Job {
            width: 40.0,
            height: 22.0,
            ..Job::default()
        };
        let m = carved(&t, &job);
        for (middle, half) in [(1.0, 1.0), (6.8, 1.8), (17.0, 5.0)] {
            let z = m.at(Point::new(20.0, middle)).unwrap();
            assert!((z + half).abs() < 0.1, "{half} deep at y {middle}: {z}");
            // and sloping up to the edges at the bit's angle
            let z = m.at(Point::new(20.0, middle - half / 2.0)).unwrap();
            assert!(
                (z + half / 2.0).abs() < 0.15,
                "half way out at y {middle}: {z}"
            );
        }
    }

    #[test]
    fn the_narrower_the_bit_the_deeper_it_carves() {
        let deepest_with = |angle: f64| {
            let mut p = Params::default();
            p.tool.angle = angle;
            deepest(&carve(vec![rectangle(40.0, 10.0)], &p))
        };
        // half the width over the tangent of half the angle
        for angle in [60.0, 90.0, 100.0] {
            let want = 5.0 / (f64::to_radians(angle) / 2.0).tan();
            let got = -deepest_with(angle);
            assert!(
                (got - want).abs() < 0.02,
                "{angle}°: {got} deep, not {want}"
            );
        }
    }

    #[test]
    fn corners_are_carved_sharp() {
        // a 20° point: the crease runs right into it, rising to the surface
        let spike = Path::polyline(
            &[
                Point::new(0.0, 0.0),
                Point::new(30.0, 5.0 * (30.0 / 30.0)),
                Point::new(0.0, 10.0),
            ],
            true,
        );
        let t = carve(vec![spike], &Params::default());
        let job = Job {
            width: 30.0,
            height: 10.0,
            ..Job::default()
        };
        let m = carved(&t, &job);
        let half_angle = (5.0f64 / 30.0).atan();
        for from_tip in [0.5, 1.0, 2.0, 3.3, 5.0, 8.0] {
            let at = Point::new(30.0 - from_tip * half_angle.cos(), 5.0);
            let want = from_tip * half_angle.sin();
            let z = m.at(at).unwrap();
            assert!(
                (z + want).abs() < 0.08,
                "{from_tip} from the tip: {z}, not {}",
                -want
            );
        }
    }

    #[test]
    fn no_cut_goes_more_than_a_pass_depth_below_what_is_cut_already() {
        let mut p = Params::default();
        p.tool.angle = 60.0;
        p.tool.pass_depth = 2.0;
        let t = carve(vec![rectangle(40.0, 10.0)], &p);
        assert!((deepest(&t) + 8.66).abs() < 0.02, "{}", deepest(&t));
        let job = Job {
            width: 40.0,
            height: 10.0,
            ..Job::default()
        };
        let mut m = Material::new(&job, 200);
        // and next to nothing is cut in air: runs between lifts that cut nothing
        let (mut removed, mut run, mut air, mut all) = (0.0f64, 0.0, 0.0, 0.0);
        for w in t.moves.windows(2) {
            if w[1].kind == MoveKind::Rapid {
                if removed < 0.01 {
                    air += run;
                }
                (removed, run) = (0.0, 0.0);
                continue;
            }
            for q in [w[0].to, w[1].to] {
                if let Some(top) = m.at(q.truncate()) {
                    assert!(q.z >= top - 2.0 - 0.05, "{q} is {} below {top}", top - q.z);
                }
            }
            removed = removed.max(m.overcut(&t.tool, w[0].to, w[1].to));
            run += w[0].to.distance(w[1].to);
            all += w[0].to.distance(w[1].to);
            m.sweep(w[0].to, w[1].to, &t.tool);
        }
        assert!(air < all * 0.02, "{air} mm of {all} in air");
    }

    #[test]
    fn inside_and_outside_carve_a_groove_beside_the_edge_with_sharp_corners() {
        let job = Job {
            width: 30.0,
            height: 30.0,
            ..Job::default()
        };
        let at = |place: Place, x: f64, y: f64| {
            let p = Params {
                place,
                depth: 1.0,
                ..Params::default()
            };
            carved(&carve(vec![square(10.0, 10.0, 10.0)], &p), &job)
                .at(Point::new(x, y))
                .unwrap()
        };
        // a 90° bit 1 mm deep: a groove 2 mm wide, one side on the edge at x 10
        for (x, outside, inside) in [
            (8.0, 0.0, 0.0),
            (8.5, -0.5, 0.0),
            (9.0, -1.0, 0.0),
            (9.5, -0.5, 0.0),
            (10.5, 0.0, -0.5),
            (11.0, 0.0, -1.0),
            (11.5, 0.0, -0.5),
            (12.5, 0.0, 0.0),
            (15.0, 0.0, 0.0),
        ] {
            let (o, i) = (at(Place::Outside, x, 15.0), at(Place::Inside, x, 15.0));
            assert!((o - outside).abs() < 0.12, "outside at x {x}: {o}");
            assert!((i - inside).abs() < 0.12, "inside at x {x}: {i}");
        }
        // inside, the groove runs right into the corner: half a millimetre in from both edges
        // it is half a millimetre deep
        let z = at(Place::Inside, 10.5, 10.5);
        assert!((z + 0.5).abs() < 0.12, "{z}");
    }

    #[test]
    fn on_the_line_the_tip_follows_open_vectors_as_deep_as_set() {
        let line = Path::polyline(&[Point::new(0.0, 5.0), Point::new(20.0, 5.0)], false);
        let p = Params {
            place: Place::OnTheLine,
            depth: 1.5,
            ..Params::default()
        };
        let t = carve(vec![line], &p);
        assert!((deepest(&t) + 1.5).abs() < 1e-9);
        assert!(
            t.moves
                .iter()
                .filter(|m| m.kind == MoveKind::Cut)
                .all(|m| (m.to.y - 5.0).abs() < 1e-9)
        );
        // but a groove deeper than the bit's cone is wide is refused, saying how deep it can go
        let too_deep = Params { depth: 7.0, ..p };
        let why = made_toolpath::<VcarveEngraveToolpath>(vec![rectangle(10.0, 10.0)], &too_deep)
            .unwrap_err();
        assert!(why.contains("6.35 mm deep"), "{why}");
    }

    #[test]
    fn it_never_carves_below_the_material() {
        use cut_craft_core::testing::document_with;
        let mut doc = document_with(vec![rectangle(40.0, 10.0)]);
        doc.job.thickness = 3.0;
        let t = VcarveEngraveToolpath::run(&doc, &Params::default()).unwrap();
        doc.apply(&t).unwrap();
        let t = doc.toolpaths.values().next().unwrap();
        assert!((deepest(t) + 3.0).abs() < 1e-6, "{}", deepest(t));
    }

    #[test]
    fn each_floor_ring_is_the_shapes_shrunk_by_its_offset() {
        // a star with a hole, and a thin sliver that vanishes between steps
        let star: Vec<Point> = (0..10)
            .map(|k| {
                let r = if k % 2 == 0 { 12.0 } else { 5.0 };
                Point::from_angle(f64::from(k) * std::f64::consts::PI / 5.0) * r
            })
            .collect();
        let paths = [
            Path::polyline(&star, true),
            moved(&rectangle(2.0, 2.0), Point::new(-1.0, -1.0)),
            moved(&rectangle(20.0, 1.1), Point::new(20.0, 0.0)),
        ];
        let regions = polygon::regions(&paths);
        let offsets = stepped_in(&regions, 0.5);
        assert!(offsets.len() >= 4, "{}", offsets.len());
        for (offset, rings) in &offsets {
            let direct = polygon::offset_regions(&regions, -offset, Corners::Round);
            assert_eq!(rings.len(), direct.len(), "at {offset}");
            // each ring all the way round, back to its first point
            let direct: Vec<Vec<Point>> = direct
                .iter()
                .map(|d| {
                    let mut points = cut_points(d);
                    points.push(points[0]);
                    points
                })
                .collect();
            for ring in rings {
                for q in cut_points(ring) {
                    let off = direct
                        .iter()
                        .map(|d| polyline_distance(d, q))
                        .fold(f64::INFINITY, f64::min);
                    assert!(off < 0.03, "at {offset}: {q} is {off} off");
                }
            }
        }
    }

    #[test]
    fn holes_are_left_standing() {
        let t = carve(
            vec![square(0.0, 0.0, 30.0), square(10.0, 10.0, 10.0)],
            &Params::default(),
        );
        let hole = [square(10.0, 10.0, 10.0).flatten(0.01)];
        assert!(
            t.moves
                .iter()
                .filter(|m| m.kind == MoveKind::Cut)
                .all(|m| !inside(&hole, m.to.truncate())),
            "nothing is cut inside the hole"
        );
    }

    #[test]
    fn the_flat_depth_limits_how_deep_it_goes() {
        let params = Params {
            flat_depth: 2.0,
            ..Params::default()
        };
        let t = carve(vec![rectangle(100.0, 10.0)], &params);
        assert!(deepest(&t) >= -2.0 - 1e-9);
        assert!(deepest(&t) <= -2.0 + 1e-9, "and it does reach it");
    }

    #[test]
    fn a_start_depth_lowers_the_whole_carving() {
        let params = Params {
            start_depth: 3.0,
            flat_depth: 2.0,
            ..Params::default()
        };
        let t = carve(vec![rectangle(100.0, 10.0)], &params);
        assert!((deepest(&t) + 5.0).abs() < 1e-9, "{}", deepest(&t));
        let job = Job {
            width: 100.0,
            height: 10.0,
            ..Job::default()
        };
        // the edge is cut at the start depth, sloping down from there
        let step = t.tool.finishing_stepover() * t.tool.diameter;
        let z = carved(&t, &job).at(Point::new(50.0, 1.0)).unwrap();
        assert!((z + 4.0).abs() <= step + 0.25, "{z}");
    }

    #[test]
    fn projected_onto_the_model_it_carves_below_its_surface() {
        // an 8 mm square on the top of the domed job's 8 mm dome
        let mut doc = cut_craft_core::testing::domed();
        let change = cut_craft_core::Change::swap(&doc, [], vec![square(16.0, 16.0, 8.0)]);
        doc.apply(&change).unwrap();
        let params = Params {
            project_onto_model: true,
            ..Params::default()
        };
        let change = VcarveEngraveToolpath::run(&doc, &params).unwrap();
        doc.apply(&change).unwrap();
        let t = doc.toolpaths.into_values().next().unwrap();
        let surface = |p: Point| {
            let r = p.distance(Point::new(20.0, 20.0));
            (64.0 - r * r).sqrt() - 8.0
        };
        // up to the 4 mm the bit reaches in the middle below the dome (and a cell's slope)
        let cuts: Vec<_> = t
            .moves
            .iter()
            .filter(|m| m.kind == MoveKind::Cut)
            .map(|m| m.to)
            .collect();
        let off = cuts.iter().find(|q| {
            let below = surface(q.truncate()) - q.z;
            !(-0.3..=4.3).contains(&below)
        });
        assert!(off.is_none(), "{off:?}");
        // the corners are cut where the dome has fallen away, not near the material's top
        let from_corner =
            |q: &cut_craft_core::glam::DVec3| q.truncate().distance(Point::new(16.0, 16.0));
        let corner = cuts
            .iter()
            .min_by(|a, b| from_corner(a).total_cmp(&from_corner(b)))
            .unwrap();
        assert!(corner.z < -2.0, "{corner:?}");
    }

    #[test]
    fn it_needs_a_v_bit_and_closed_vectors() {
        let end_mill = Params {
            tool: Tool::default(),
            ..Params::default()
        };
        let why = made_toolpath::<VcarveEngraveToolpath>(vec![rectangle(10.0, 10.0)], &end_mill)
            .unwrap_err();
        assert!(why.contains("V-bit"), "{why}");
        let open = Path::polyline(&[Point::new(0.0, 0.0), Point::new(10.0, 0.0)], false);
        let why =
            made_toolpath::<VcarveEngraveToolpath>(vec![open], &Params::default()).unwrap_err();
        assert!(why.contains("closed"), "{why}");
    }
}
