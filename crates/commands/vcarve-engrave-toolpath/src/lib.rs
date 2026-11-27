//! V-Carve / Engrave Toolpath (V-bit carving): a V-bit carves the selected shapes, going deeper the
//! further it is from their edges so its sides cut the edges exactly.

use cut_craft_core::document::Document;
use cut_craft_core::entry::{Entry, EntryMode, EntrySettings, Fallback, Fallbacks, path_moves};
use cut_craft_core::geometry::{Path, Point, inside};
use cut_craft_core::glam::DVec3;
use cut_craft_core::medial;
use cut_craft_core::polygon::{self, Corners};
use cut_craft_core::toolpath::{
    Move, MoveKind, Tool, ToolDatabase, ToolShape, contour_moves, cut_points, made_by,
    project_onto_model,
};
use cut_craft_core::{Change, Command, CommandError, Form, Meta, ensure_positive};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

mod link;
mod tour;

pub struct VcarveEngraveToolpath;

/// Where the V-bit carves, measured from the vectors.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Place {
    /// Carve the whole shape: the tip runs down its middle, as deep as its width needs, so
    /// the sides meet in a sharp crease (the usual V-carved lettering). Its passes go at the
    /// bit's pass depth; Edit Passes is for the carves of a set depth
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

/// How the passes are cut, and how the bit gets from one cut to the next.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Style {
    /// Every pass clears its whole floor, never cutting more than a pass depth: the most
    /// moves, for hard woods and light machines
    #[schemars(title = "Light passes")]
    LightPasses,
    /// One ring round the floor each pass on the way down, the centre lines once in the last
    /// pass; the bit stays down in each shape
    #[default]
    #[schemars(title = "Stay down")]
    StayDown,
    /// As Stay down, but each stretch of centre line is cut once, lifting between them: the
    /// fewest moves
    #[schemars(title = "Lift between cuts")]
    Lift,
}

/// Jobs saved before there was a choice keep the passes they were made with.
fn light_passes() -> Style {
    Style::LightPasses
}

/// The order Stay down carves the centre lines in.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Order {
    /// The same way round every shape, out and back along each branch, so every point is
    /// cut alike
    #[default]
    #[schemars(title = "Round the shape")]
    RoundTheShape,
    /// The nearest cut next
    #[schemars(title = "Shortest path")]
    ShortestPath,
}

/// Which end of each stretch of centre line Lift between cuts starts at.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Direction {
    /// From the surface at a point down to depth: no deep plunges, so points chip less
    #[default]
    #[schemars(title = "Shallow in")]
    ShallowIn,
    /// Straight down at the deep end, up to the surface at the point
    #[schemars(title = "Depth out")]
    DepthOut,
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
    #[schemars(extend("x-dim" = "D1", "x-diagram" = "diagrams/depth_vcarve.svg"))]
    pub start_depth: f64,
    /// Deepest the bit goes below the start depth; wider parts are cut flat there (0: as
    /// deep as it can)
    #[schemars(extend("x-unit" = "mm"), range(min = 0))]
    #[schemars(extend("x-group" = "Cutting Depths", "x-show-when" = {"place": "fill"}))]
    #[schemars(extend("x-dim" = "D"))]
    pub flat_depth: f64,
    /// Fill carves the whole shape, as deep as its width needs; Inside, On the Line and
    /// Outside carve a V groove of the depth set
    #[serde(default)]
    #[schemars(
        title = "Carve",
        extend("x-group" = "Cutting Depths", "x-widget" = "drop-down")
    )]
    pub place: Place,
    /// How deep the groove's point goes below the start depth (Fill works its depth out
    /// from each shape's width instead)
    #[serde(default = "one")]
    #[schemars(
        title = "Groove depth",
        extend("x-unit" = "mm", "x-group" = "Cutting Depths", "x-hide-when" = {"place": "fill"}),
        range(min = 0)
    )]
    #[schemars(extend("x-dim" = "D"))]
    pub depth: f64,
    /// A V-bit; its stepover sets how close together the carving passes are
    pub tool: Tool,
    /// How the passes are cut, and how the bit gets from one cut to the next
    #[serde(default = "light_passes")]
    #[schemars(
        title = "Carving style",
        extend(
            "x-group" = "Carving Style",
            "x-widget" = "drop-down",
            "x-diagram" = "diagrams/vcarve_{}.png",
            "x-show-when" = {"place": ["fill", "inside", "outside"]}
        )
    )]
    pub style: Style,
    /// The order the centre lines are carved in
    #[serde(default)]
    #[schemars(extend(
        "x-group" = "Carving Style",
        "x-widget" = "drop-down",
        "x-diagram" = "diagrams/vcarve_order_{}.png",
        "x-show-when" = {"style": "stay_down", "place": ["fill", "inside", "outside"]}
    ))]
    pub order: Order,
    /// Which end of each stretch of centre line the bit starts at
    #[serde(default)]
    #[schemars(extend(
        "x-group" = "Carving Style",
        "x-widget" = "drop-down",
        "x-diagram" = "diagrams/vcarve_direction_{}.png",
        "x-show-when" = {"style": "lift", "place": ["fill", "inside", "outside"]}
    ))]
    pub direction: Direction,
    /// End mills that clear the flat bottom first, largest first (needs a flat depth): each
    /// makes a toolpath of its own, and each bit after it, the V-bit last, cuts only what
    /// the ones before it left (rest machining)
    #[serde(default)]
    #[schemars(title = "Clearance Bits")]
    pub clearance_tools: Vec<Tool>,
    /// This toolpath is this clearance bit's pass over the flat bottom
    #[serde(default)]
    #[schemars(skip)]
    pub clearing: Option<Tool>,
    /// The V-bit's toolpath after clearance bits: their smallest radius, so it carves the
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
    /// How the bit goes down into each shape (straight down, a zig-zag or a ramp along the
    /// groove's own centre line, never below the depth its width sets; no helix), set in
    /// the Entry window. Unset on toolpaths made before entry modes: straight down
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(skip)]
    pub entry: Option<EntrySettings>,
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
                .expect("the starter bits include a V-bit"),
            style: Style::default(),
            order: Order::default(),
            direction: Direction::default(),
            clearance_tools: vec![],
            clearing: None,
            cleared_within: None,
            project_onto_model: false,
            use_vector_start_points: false,
            use_vector_selection_order: false,
            entry: Some(EntrySettings::default()),
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
            form: Form::TOOLPATH,
        }
    }

    fn run(doc: &Document, p: &Params) -> Result<Change, CommandError> {
        let mut c = Change::builder(doc);
        if p.clearance_tools.is_empty() {
            let toolpath = vcarve(doc, p)?;
            if let Some(note) = bit_too_short(doc, p, &toolpath) {
                c.note(note);
            }
            c.add_toolpath(toolpath);
            return Ok(c.finish());
        }
        if p.place != Place::Fill {
            return Err(CommandError::InvalidParams(
                "clearance bits clear a Fill's flat bottom: set Carve to Fill, or take them off"
                    .into(),
            ));
        }
        if p.flat_depth <= 0.0 {
            return Err(CommandError::InvalidParams(
                "clearance bits clear the flat bottom: give a flat depth".into(),
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
            // each bit its share of the progress bar, its rest machining included, the
            // V-bit's last
            let bits = p.clearance_tools.len() + 1;
            let toolpath = cut_craft_core::progress::part(i, bits, || {
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
                Ok::<_, CommandError>(toolpath)
            })?;
            c.add_toolpath(toolpath);
        }
        let vbit = Params {
            name: format!("{} [{}]", p.name, p.clearance_tools.len() + 1),
            clearance_tools: vec![],
            cleared_within: Some(smallest),
            ..p.clone()
        };
        let last = p.clearance_tools.len();
        c.add_toolpath(cut_craft_core::progress::part(last, last + 1, || {
            vcarve(doc, &vbit)
        })?);
        Ok(c.finish())
    }
}

/// Where Fill's widest parts need the bit deeper than its V is tall, so it cuts them flat
/// at that depth: what to do about it (nothing to say when a flat depth was asked for, or
/// the material's bottom is what stops it).
fn bit_too_short(
    doc: &Document,
    p: &Params,
    toolpath: &cut_craft_core::toolpath::Toolpath,
) -> Option<String> {
    let tool = &p.tool;
    if p.place != Place::Fill || p.flat_depth > 0.0 || p.clearing.is_some() {
        return None;
    }
    let tall = (tool.diameter / 2.0) / (tool.angle.to_radians() / 2.0).tan();
    let full = p.start_depth + tall;
    let deepest = -toolpath
        .moves
        .iter()
        .map(|m| m.to.z)
        .fold(f64::INFINITY, f64::min);
    let material = if doc.job.thickness > 0.0 && !p.project_onto_model {
        doc.job.thickness
    } else {
        f64::INFINITY
    };
    (deepest >= full - 1e-6 && material > full + 1e-6).then(|| {
        format!(
            "Parts of these shapes are wider than {} can carve to a point: they are cut flat \
             {} deep, as deep as its V goes. Clearance Bits clear that flat bottom quicker; a \
             wider V-bit carves them deeper",
            tool.name,
            doc.job.units.length(tall, 2)
        )
    })
}

/// The toolpath for `p`'s own bit: the V-bit's carving, or (`clearing`) an end mill's
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
        ensure_positive("bit diameter", tool.diameter)?;
        ensure_positive("pass depth", tool.pass_depth)?;
        let stepover = tool.finishing_stepover();
        if !(stepover > 0.0 && stepover <= 1.0) {
            return Err(CommandError::InvalidParams(
                "the bit's finishing stepover must be more than 0% and at most 100%: set it in the Bit Library".into(),
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
        let notes = Fallbacks::default();
        // dropping onto the model takes its own share of the progress bar after the carving
        let steps = if p.project_onto_model { 2 } else { 1 };
        let mut moves = cut_craft_core::progress::part(0, steps, || {
            carve_above(&paths, p, doc.job.clearance, bottom, &notes)
        });
        if p.project_onto_model {
            moves = cut_craft_core::progress::part(1, 2, || project_onto_model(doc, tool, &moves))?;
        }
        let mut toolpath =
            made_by::<VcarveEngraveToolpath>(p, p.name.clone(), tool.clone(), ids, moves);
        toolpath.fallbacks = std::mem::take(
            &mut *notes
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner),
        );
        Ok(toolpath)
    }
}

/// A clearance bit's pass: the flat bottom (where the V-bit, `p.flat_depth` down, is
/// narrower than the shape) pocketed with rings stepping in, at each pass depth.
fn clearing(
    doc: &Document,
    p: &Params,
    tool: &Tool,
) -> Result<cut_craft_core::toolpath::Toolpath, CommandError> {
    ensure_positive("bit diameter", tool.diameter)?;
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
            "{} is too big to clear any of the flat bottom. Pick a smaller flat-bottom bit",
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
    carve_above(paths, p, safe, None, &Fallbacks::default())
}

/// [`carve`] going no deeper than `bottom` below the material's top, when given.
/// Where an entry goes down another way than asked, it is noted in `notes`.
fn carve_above(
    paths: &[Path],
    p: &Params,
    safe: f64,
    bottom: Option<f64>,
    notes: &Fallbacks,
) -> Vec<Move> {
    if p.place == Place::OnTheLine {
        return along_the_lines(paths, p, safe, bottom, notes);
    }
    if !p.use_vector_selection_order {
        return carve_together(paths, p, safe, bottom, notes);
    }
    let shapes = shapes(paths);
    shapes
        .iter()
        .enumerate()
        .flat_map(|(i, shape)| {
            cut_craft_core::progress::part(i, shapes.len(), || {
                carve_together(shape, p, safe, bottom, notes)
            })
        })
        .collect()
}

/// `p`'s entry for a V-carve: no helix (a helix would carve outside the groove), so one
/// asked for zig-zags.
fn v_entry(p: &Params) -> Option<EntrySettings> {
    p.entry.map(|e| match e.mode {
        EntryMode::Helix => EntrySettings {
            mode: EntryMode::ZigZag,
            zigzag_angle: e.helix_angle,
            ..e
        },
        _ => e,
    })
}

/// The tip along each of `paths` (in their order), `p.depth` below the start depth (no
/// deeper than `bottom`), a pass depth at a time.
fn along_the_lines(
    paths: &[Path],
    p: &Params,
    safe: f64,
    bottom: Option<f64>,
    notes: &Fallbacks,
) -> Vec<Move> {
    let depth = (p.start_depth + p.depth).min(bottom.unwrap_or(f64::INFINITY));
    let depths = p.tool.pass_depths(depth);
    let settings = v_entry(p).filter(|e| e.mode != EntryMode::Plunge);
    // how long a leg the first pass's entry takes
    let first = p.tool.pass_depth.min(p.depth).max(0.0);
    let leg = settings.map_or(0.0, |e| {
        if e.mode == EntryMode::Ramp {
            e.ramp_distance
        } else {
            let slope = e.zigzag_angle.clamp(0.1, 89.0).to_radians().tan();
            first / (f64::from(e.zigzag_passes.max(1)) * slope)
        }
    });
    let entry = Entry {
        enter: settings,
        tool_diameter: p.tool.diameter,
        fallbacks: notes.clone(),
        ..Entry::default()
    };
    paths
        .iter()
        .flat_map(|path| {
            let points = cut_points(path);
            let length: f64 = points.windows(2).map(|w| w[0].distance(w[1])).sum::<f64>()
                + if path.closed {
                    points[points.len() - 1].distance(points[0])
                } else {
                    0.0
                };
            match settings {
                Some(_) if leg <= length + 1e-9 => {
                    path_moves(&points, path.closed, &depths, -p.start_depth, safe, &entry)
                }
                // straight down where the line is too short for the leg (Matthew's rule)
                Some(e) => {
                    notes
                        .lock()
                        .unwrap_or_else(std::sync::PoisonError::into_inner)
                        .push(Fallback {
                            at: points[0],
                            wanted: e.mode,
                            used: EntryMode::Plunge,
                        });
                    contour_moves(&points, path.closed, &depths, safe)
                }
                None => contour_moves(&points, path.closed, &depths, safe),
            }
        })
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
fn carve_together(
    paths: &[Path],
    p: &Params,
    safe: f64,
    bottom: Option<f64>,
    notes: &Fallbacks,
) -> Vec<Move> {
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
    if reach <= 0.0 {
        return vec![];
    }
    let carving = Carving {
        p,
        spread,
        reach,
        // the flat bottom's middle, cleared by end mills, is left alone
        cleared: p.cleared_within.map_or(f64::INFINITY, |r| reach + r),
        starts: &starts,
    };
    let regions = grooved(&polygon::regions(paths), p.place, p.depth * spread);
    let mut pieces: Vec<(Vec<Path>, Vec<Point>)> = polygon::pieces(&regions)
        .into_iter()
        .map(|piece| {
            let marks = marks(&piece);
            (piece, marks)
        })
        .collect();
    // the first piece by the first vector's start point, when asked
    let mut from = starts
        .first()
        .copied()
        .or_else(|| pieces.first().and_then(|(_, marks)| marks.first().copied()))
        .unwrap_or(Point::ZERO);
    let mut moves = vec![];
    // the progress bar moves on as each piece is carved, by how many points it has: the
    // centre lines of a big piece take longer to find than a small one's
    let size = |piece: &[Path]| {
        piece
            .iter()
            .map(|path| cut_points(path).len())
            .sum::<usize>()
    };
    let total: usize = pieces.iter().map(|(piece, _)| size(piece)).sum();
    let mut carved = 0;
    // each piece of the shapes (an outline with its holes) carved in full, the bit staying
    // down from its first cut to its last, then the nearest piece next
    while !pieces.is_empty() {
        if cut_craft_core::progress::cancelled() {
            break;
        }
        let away = |marks: &[Point]| {
            marks
                .iter()
                .map(|m| m.distance(from))
                .fold(f64::INFINITY, f64::min)
        };
        let nearest = (0..pieces.len())
            .min_by(|&i, &k| away(&pieces[i].1).total_cmp(&away(&pieces[k].1)))
            .expect("pieces are left");
        let (piece, _) = pieces.swap_remove(nearest);
        carved += size(&piece);
        let lines = medial::centre_lines(&piece, CENTRE_SPACING);
        let runs = carving.runs(&piece, &lines, from);
        if let Some(last) = runs.last().and_then(Run::end) {
            from = last;
        }
        let ways = link::Links::new(&piece, &lines, |r| carving.depth_at(r));
        let enter = |run: &Run| entered(run, -p.start_depth, v_entry(p), p.tool.diameter, notes);
        // Lift between cuts goes up and over to each cut; the others stay down
        let lift = p.style == Style::Lift;
        let link = |a, b| if lift { None } else { ways.between(a, b) };
        moves.extend(run_moves(&runs, safe, link, enter));
        cut_craft_core::progress::report_steps(carved, total);
    }
    // the centre lines are found from points along the edges, so they wobble by a few
    // thousandths of a millimetre: cut straight past what is that close to the line
    cut_craft_core::simplify::simplify(&moves, SMOOTH)
}

/// How far, mm, a cut may pass a point of the carving rather than going through it: the
/// centre lines' wobble, far below what a router cuts to.
const SMOOTH: f64 = 0.01;

/// A few points round `piece`'s outlines, to find which piece is nearest quickly.
fn marks(piece: &[Path]) -> Vec<Point> {
    const MARKS: usize = 16;
    let points: Vec<Point> = piece.iter().flat_map(cut_points).collect();
    let every = points.len().div_ceil(MARKS).max(1);
    points.into_iter().step_by(every).collect()
}

/// How one toolpath carves each piece of its shapes.
struct Carving<'a> {
    p: &'a Params,
    /// How far the cone reaches sideways for each millimetre down.
    spread: f64,
    /// Further in from the edges than this the bottom is cut flat.
    reach: f64,
    /// Further in than this the bottom was cleared by end mills.
    cleared: f64,
    starts: &'a [Point],
}

impl Carving<'_> {
    /// How deep the tip goes `r` from the edges.
    fn depth_at(&self, r: f64) -> f64 {
        -(self.p.start_depth + r.min(self.reach) / self.spread)
    }

    /// The cuts carving `piece` (whose centre lines are `lines`), pass after pass, starting
    /// nearest `from`. Light passes carve the centre lines and clear the floor in every pass;
    /// the other styles cut one ring round each floor on the way down and carve the centre
    /// lines once, in the last pass, in the style's order.
    fn runs(&self, piece: &[Path], lines: &[medial::CentreLine], mut from: Point) -> Vec<Run> {
        let (p, tool, spread) = (self.p, &self.p.tool, self.spread);
        let widest_shape = lines
            .iter()
            .flat_map(|l| l.radii.iter().copied())
            .fold(0.0, f64::max);
        let widest = widest_shape.min(self.reach);
        if widest <= 0.0 {
            return vec![];
        }
        let light = p.style == Style::LightPasses;
        let mut all = vec![];
        let surface = -p.start_depth;
        let mut above = surface;
        // passes set by hand were set against the Groove depth, which Fill doesn't cut to:
        // its passes go at the pass depth (a thin last pass still counts)
        let even = Tool {
            passes: vec![],
            ..tool.clone()
        };
        let levels = even.pass_depths(p.start_depth + widest / spread);
        let last_level = levels.last().copied().unwrap_or(above);
        for level in levels {
            if level >= above - 1e-9 {
                continue;
            }
            let finishing = (level - last_level).abs() < 1e-9;
            let wanted = |r: f64| r <= self.cleared;
            let mut runs = if light {
                // the centre lines, where they are deeper than the last pass, down to this one
                nearest_first(
                    centre_runs(
                        lines,
                        level,
                        above,
                        |r| self.depth_at(r) < above - 1e-9 && wanted(r),
                        |r| self.depth_at(r),
                    ),
                    from,
                )
            } else if finishing {
                // the centre lines all the way from the surface, once
                let depth = |r: f64| self.depth_at(r).max(level);
                match (p.style, p.order) {
                    (Style::Lift, _) => tour::stretches(
                        lines,
                        depth,
                        wanted,
                        p.direction == Direction::ShallowIn,
                        from,
                    ),
                    (_, Order::RoundTheShape) => tour::round(lines, depth, wanted, from),
                    (_, Order::ShortestPath) => nearest_first(
                        centre_runs(
                            lines,
                            level,
                            surface,
                            |r| self.depth_at(r) < surface - 1e-9 && wanted(r),
                            |r| self.depth_at(r),
                        ),
                        from,
                    ),
                }
            } else {
                vec![]
            };
            if let Some(last) = runs.last().and_then(Run::end) {
                from = last;
            }
            // the floor: where this pass is too deep for the cone to reach the edges
            let floor_reach = ((-level - p.start_depth) * spread).min(self.reach);
            // (none where only the centre line itself is that far from the edges)
            if floor_reach > 1e-6 && floor_reach < widest_shape - CENTRE_SPACING {
                let floor = polygon::offset_regions(piece, -floor_reach, Corners::Round);
                // a pass on the way down only has to leave ridges low enough for the next
                // pass to take with the rest: a quarter of a pass depth
                let step = if finishing {
                    tool.finishing_stepover() * tool.diameter
                } else {
                    (tool.pass_depth * spread / 2.0)
                        .min(tool.diameter / 2.0)
                        .max(tool.finishing_stepover() * tool.diameter)
                };
                // on the way down, other than in light passes, only the ring round its edge:
                // the passes below take the rest with their own
                let within = if light || finishing {
                    self.cleared - floor_reach
                } else {
                    (self.cleared - floor_reach).min(0.0)
                };
                let mut rings = floor_rings(&floor, step, within, level, self.starts);
                // from the middle out, the edge of the floor last
                rings.reverse();
                let rings = if light || finishing {
                    rings
                } else {
                    nearest_first(rings, from)
                };
                if !light && !finishing {
                    // the ring first, then the last pass's centre lines from where it ends
                    runs.splice(0..0, rings);
                } else {
                    runs.extend(rings);
                }
            }
            if let Some(last) = runs.last().and_then(Run::end) {
                from = last;
            }
            all.extend(runs);
            above = level;
        }
        all
    }
}

/// The stretches of `lines` where `deeper` (given the distance to the edge) holds, at
/// `depth_at` but no lower than `level`. Each runs on at either end to where it meets what the
/// pass before cut (`above`): part way to the next point when the carving there is no deeper
/// than that pass, so a later pass stops where the last one reached rather than going on to
/// the next point (on a straight stretch, the far end of it).
fn centre_runs(
    lines: &[medial::CentreLine],
    level: f64,
    above: f64,
    deeper: impl Fn(f64) -> bool,
    depth_at: impl Fn(f64) -> f64,
) -> Vec<Run> {
    let mut runs = vec![];
    for line in lines {
        let count = line.points.len();
        let at = |k: usize| {
            let q = line.points[k];
            DVec3::new(q.x, q.y, depth_at(line.radii[k]))
        };
        // from the first point carved (`inner`) towards one that is not (`outer`): where the
        // carving comes up to the last pass, if it does between them
        let met = |inner: usize, outer: usize| {
            let (a, b) = (at(inner), at(outer));
            if b.z >= above - 1e-9 && a.z < above && b.z > a.z {
                let t = (above - a.z) / (b.z - a.z);
                a.lerp(b, t)
            } else {
                b
            }
        };
        let mut i = 0;
        while i < count {
            if !deeper(line.radii[i]) {
                i += 1;
                continue;
            }
            let first = i;
            while i < count && deeper(line.radii[i]) {
                i += 1;
            }
            let mut points: Vec<DVec3> = vec![];
            if first > 0 {
                points.push(met(first, first - 1));
            }
            points.extend((first..i).map(at));
            if i < count {
                points.push(met(i - 1, i));
            }
            let points = flattened(&points, level);
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

/// `points` kept no lower than `level`, with a point added where they go below it or come
/// back up: cut level there and on down the line, not straight from a point held up to the
/// next one, which would leave the slope between them shallower than carved.
fn flattened(points: &[DVec3], level: f64) -> Vec<DVec3> {
    let mut out: Vec<DVec3> = Vec::with_capacity(points.len());
    for (i, &q) in points.iter().enumerate() {
        if let Some(&before) = i.checked_sub(1).map(|k| &points[k])
            && (before.z < level) != (q.z < level)
            && (before.z - q.z).abs() > 1e-12
        {
            out.push(before.lerp(q, (level - before.z) / (q.z - before.z)));
        }
        out.push(q.truncate().extend(q.z.max(level)));
    }
    out
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

/// Where `p` carves `paths` at the surface: the areas a beginner should see shaded before
/// calculating. Fill carves the closed shapes (every other band of loops nested one in
/// another, a loop there twice cancelling out); Inside and Outside the band the groove's
/// width beside the edge; On the Line the groove along the lines, open ones too.
pub fn carved_area(paths: &[Path], p: &Params) -> Vec<Path> {
    let half_width = p.depth * (p.tool.angle.to_radians() / 2.0).tan();
    if p.place == Place::OnTheLine {
        return polygon::stroke(paths, half_width);
    }
    let closed: Vec<Path> = paths.iter().filter(|path| path.closed).cloned().collect();
    grooved(&polygon::regions(&closed), p.place, half_width)
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

/// Down into `run` from `top` by `settings`, ending at its start, as deep as it starts:
/// straight down, or back and forth along the run's own line (a ramp's leg the ramp
/// distance long, a zig-zag's as its angle and passes make it), each point no lower than
/// the groove there, so it never cuts wider than the shape's width sets. Legs go on until
/// the slope is below the whole stretch they cover. A run too short for its leg (or for a
/// leg a quarter of the bit, `bit`, long) goes straight down, as is usual for a V-carve,
/// noted in `notes`.
fn entered(
    run: &Run,
    top: f64,
    settings: Option<EntrySettings>,
    bit: f64,
    notes: &Fallbacks,
) -> Vec<Move> {
    let start = run.points[0];
    let plunge = vec![Move {
        kind: MoveKind::Plunge,
        to: start,
    }];
    let Some(settings) = settings else {
        return plunge;
    };
    if settings.mode == EntryMode::Plunge || start.z >= top - 1e-9 {
        return plunge;
    }
    let note = |used: EntryMode| {
        notes
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .push(Fallback {
                at: start.truncate(),
                wanted: settings.mode,
                used,
            });
    };
    // the run's line by distance along it
    let mut line: Vec<DVec3> = run.points.clone();
    if run.closed {
        line.push(start);
    }
    let mut at = vec![0.0];
    for w in line.windows(2) {
        at.push(at[at.len() - 1] + w[0].truncate().distance(w[1].truncate()));
    }
    let length = at[at.len() - 1];
    if length < (bit / 4.0).max(0.1) {
        note(EntryMode::Plunge);
        return plunge;
    }
    let point = |s: f64| {
        let i = at.partition_point(|&a| a <= s).clamp(1, line.len() - 1);
        let span = (at[i] - at[i - 1]).max(1e-12);
        line[i - 1].lerp(line[i], ((s - at[i - 1]) / span).clamp(0.0, 1.0))
    };
    let drop = top - start.z;
    let (leg, slope) = if settings.mode == EntryMode::Ramp {
        let distance = settings.ramp_distance.max(0.1);
        (distance, drop / distance)
    } else {
        let slope = settings.zigzag_angle.clamp(0.1, 89.0).to_radians().tan();
        let passes = f64::from(settings.zigzag_passes.max(1));
        (drop / (passes * slope), slope)
    };
    // a ring (a floor's edge) too short for the leg: round and round it on the way down,
    // as steep as the leg would be, arriving at its start
    if leg > length + 1e-9 && run.closed {
        return round_down(&line, &at, top, slope);
    }
    // a groove too short for the leg: straight down (Matthew's rule)
    if leg > length + 1e-9 {
        note(EntryMode::Plunge);
        return plunge;
    }
    // the deepest the stretch the legs cover goes
    let lowest = at
        .iter()
        .zip(&line)
        .filter(|&(&a, _)| a <= leg)
        .map(|(_, q)| q.z)
        .fold(point(leg).z, f64::min);
    let mut moves = vec![Move {
        kind: MoveKind::Plunge,
        to: start.truncate().extend(top),
    }];
    let (mut s, mut travelled) = (0.0_f64, 0.0_f64);
    for i in 0..10_000 {
        let to = if i % 2 == 0 { leg } else { 0.0 };
        let (lo, hi) = (s.min(to), s.max(to));
        let mut stations: Vec<f64> = at.iter().copied().filter(|&a| a > lo && a < hi).collect();
        stations.push(to);
        if to < s {
            stations.sort_by(|a, b| b.total_cmp(a));
        }
        for station in stations {
            travelled += (station - s).abs();
            s = station;
            let q = point(s);
            let z = q.z.max(top - travelled * slope);
            moves.push(Move {
                kind: MoveKind::Plunge,
                to: q.truncate().extend(z),
            });
        }
        if i % 2 == 1 && top - travelled * slope <= lowest + 1e-9 {
            break;
        }
    }
    moves
}

/// Down into a closed ring (`line`, its start repeated at the end, `at` how far along each
/// point is) from `top`, going round it backwards from where the slope reaches `top` so
/// as to arrive at its start as deep as the ring is there, `slope` down for each millimetre
/// along, never below the ring.
fn round_down(line: &[DVec3], at: &[f64], top: f64, slope: f64) -> Vec<Move> {
    let start = line[0];
    let length = at[at.len() - 1];
    let distance = (top - start.z) / slope.max(1e-9);
    // the ring's points behind the start, nearest first, until the slope reaches the top
    let mut behind: Vec<(f64, DVec3)> = vec![];
    let mut laps = 0.0;
    'round: loop {
        for k in (0..line.len() - 1).rev() {
            let back = laps + length - at[k];
            if back >= distance {
                // part way along to where it starts
                let after = line[k + 1];
                let span = (at[k + 1] - at[k]).max(1e-12);
                let over = (back - distance) / span;
                behind.push((distance, line[k].lerp(after, over.clamp(0.0, 1.0))));
                break 'round;
            }
            if back > 0.0 {
                behind.push((back, line[k]));
            }
        }
        laps += length;
    }
    behind.reverse();
    let mut moves: Vec<Move> = behind
        .iter()
        .map(|&(back, q)| Move {
            kind: MoveKind::Plunge,
            to: q
                .truncate()
                .extend(q.z.max(top - (distance - back) * slope)),
        })
        .collect();
    moves.push(Move {
        kind: MoveKind::Plunge,
        to: start,
    });
    moves
}

/// The moves cutting `runs` in order: one running on from where the last ended continues
/// without lifting, as does one `link` finds a way to (cutting through the points it
/// gives); otherwise up to `safe`, across, and down.
fn run_moves(
    runs: &[Run],
    safe: f64,
    link: impl Fn(DVec3, DVec3) -> Option<Vec<DVec3>>,
    enter: impl Fn(&Run) -> Vec<Move>,
) -> Vec<Move> {
    let mut moves: Vec<Move> = vec![];
    let mut at: Option<DVec3> = None;
    for run in runs {
        let Some(&start) = run.points.first() else {
            continue;
        };
        let joined = at.is_some_and(|a| a.distance(start) < 1e-9);
        if !joined {
            if let Some(via) = at.and_then(|a| link(a, start)) {
                moves.extend(via.into_iter().chain([start]).map(|to| Move {
                    kind: MoveKind::Cut,
                    to,
                }));
            } else {
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
                moves.extend(enter(run));
            }
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
mod tests;

#[cfg(test)]
mod style_tests;
