//! Profile Toolpath: cuts along the selected vectors, outside, inside or on the line.

use cut_craft_core::document::Document;
use cut_craft_core::entry::{
    Direction, Entry, EntrySettings, Fallbacks, Lead, Leads, Loops, Motion, Ramp, Ramping, Room,
    TabSettings, TabSpacing, Tabs, Ways, directed_moves, trochoid,
};
use cut_craft_core::geometry::{
    Path, Point, Rect, inside, polyline_closest, polyline_distance, winding,
};
use cut_craft_core::params::Anchor;
use cut_craft_core::polygon::{self, Corners};
use cut_craft_core::toolpath::{Tool, cut_points, made_by, project_onto_model};
use cut_craft_core::{
    Change, Command, CommandError, Form, Meta, ensure_positive, tool_does_not_fit,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub struct ProfileToolpath;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Side {
    /// Cut a part out: the bit goes round the outside of each shape, so the part is the size
    /// drawn
    #[default]
    #[schemars(title = "Outside")]
    Outside,
    /// Cut a hole: the bit goes round the inside of each shape, so the hole is the size
    /// drawn
    #[schemars(title = "Inside")]
    Inside,
    /// The bit's middle follows the line itself, open lines too, so the cut is as wide as the
    /// bit
    #[schemars(title = "On the Line")]
    On,
}

/// The heading of the form's block that picks [`Side`], drawn just under the cut depth: the
/// choice that decides whether a part comes out the size drawn.
pub const SIDE_GROUP: &str = "Which Side of the Line";

/// The ways of ordering the cuts to try: the one with the shortest moves between cuts is used.
/// Shapes inside others are always cut first, so a part stays held as long as it can.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(default)]
#[expect(clippy::struct_excessive_bools, reason = "one tick box per option")]
pub struct Order {
    /// In the order the vectors were selected
    pub vector_selection_order: bool,
    /// From the parts on the left across to the right
    pub left_to_right: bool,
    /// From the parts at the bottom up to the top
    pub bottom_to_top: bool,
    /// Square by square over a grid the size of the biggest part
    pub grid: bool,
    /// On each time to the nearest shape not yet cut
    pub shortest_path: bool,
}

impl Default for Order {
    fn default() -> Self {
        // ponytail: our own defaults; trying them all costs little
        Self {
            vector_selection_order: true,
            left_to_right: true,
            bottom_to_top: true,
            grid: true,
            shortest_path: true,
        }
    }
}

/// Where the cut round each closed shape starts.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum StartAt {
    /// At each vector's own start point (Make Start Point moves it)
    #[schemars(title = "Keep Current Start Points")]
    KeepCurrent,
    /// Wherever makes the moves between cuts shortest
    #[default]
    #[schemars(title = "Optimize Start Points")]
    Optimize,
    /// At the corner of the path nearest a point on each vector's bounding box
    #[schemars(title = "Closest on Bounding Box")]
    BoundingBox,
}

/// How far out a sharp corner may go, in bit radii, before it is squared off: corners down to
/// about 29° come to a point.
// ponytail: the limit is not known
pub const MITRE_REACH: f64 = 4.0;

// Laid out as the 2D Profile form: cutting depths, the tool, how the vectors are
// machined, tabs, then a tab each for ramps, leads, order, start points and corners.
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
pub struct Params {
    /// Below the material surface
    #[schemars(
        title = "Cut Depth",
        extend("x-unit" = "mm", "x-group" = "Cutting Depths"),
        range(min = 0)
    )]
    #[schemars(extend("x-dim" = "D", "x-diagram" = "diagrams/depth_profile.svg"))]
    pub depth: f64,
    /// Down to the bottom of the material: the cut depth is the material's thickness in Job
    /// Setup, and follows it if the thickness changes. Kept for toolpaths made with it; the
    /// form no longer offers it (a cut deeper than the material is warned of instead)
    #[serde(default)]
    #[schemars(skip)]
    pub through: bool,
    /// The bit that cuts round the shapes; Select… picks one from your bits
    pub tool: Tool,
    /// Outside to cut a part out, Inside to cut a hole, On the Line to follow the line itself
    #[schemars(
        title = "Cut",
        extend(
            "x-group" = "Which Side of the Line",
            "x-widget" = "named-icons",
            "x-icon" = "toolpaths/side_{}.png"
        )
    )]
    pub side: Side,
    /// Which way round closed shapes are cut: climb is clockwise round a part (and along a
    /// closed shape's line), anticlockwise round a hole; an open line is cut the way it was
    /// drawn
    #[serde(default)]
    #[schemars(skip)]
    pub direction: Direction,
    /// Every pass the same way, or each back the way the last came
    #[serde(default)]
    #[schemars(skip)]
    pub ways: Ways,
    /// Straight, or in small loops in the waste beside the line, then once along it straight
    #[serde(default)]
    #[schemars(skip)]
    pub motion: Motion,
    /// Set in the Cutting window, with the motion trochoidal
    #[serde(default)]
    #[schemars(skip)]
    pub loops: Loops,
    /// Positive leaves the part bigger (or the hole smaller), negative cuts into it
    #[serde(default)]
    #[schemars(
        title = "Allowance Offset",
        extend("x-unit" = "mm", "x-group" = "Machine Vectors…")
    )]
    pub allowance: f64,
    /// Bridges left to hold the part in place
    #[serde(default)]
    #[schemars(title = "Add tabs to toolpath", extend("x-group" = "Tabs"))]
    pub tabs: Tabs,
    /// Their size, how many and where: set in the Edit Tabs window, so the form stays short
    #[serde(default)]
    #[schemars(skip)]
    pub tab: TabSettings,
    /// Tabs placed by clicking in the 2D view, on the vectors
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[schemars(skip)]
    pub tab_points: Vec<Point>,
    /// How toolpaths made before entry modes went down into each pass, kept for them: with
    /// `entry` unset they go down as they did. The form sets `entry` instead
    #[serde(default)]
    #[schemars(skip)]
    pub ramp: Ramp,
    #[serde(default)]
    #[schemars(skip)]
    pub ramping: Ramping,
    /// How the bit goes down into the material (helix, zig-zag, ramp or straight down), set
    /// in the Entry window; each place it goes down fits it to the room there. Unset on
    /// toolpaths made before entry modes, which keep their ramp
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(skip)]
    pub entry: Option<EntrySettings>,
    /// How the bit comes onto each path, from the waste side; a ramp then runs along the
    /// lead in, and there is no spiral
    #[serde(default)]
    #[schemars(title = "Lead in / out", extend("x-tab" = "Leads"))]
    pub lead: Lead,
    #[serde(default)]
    #[schemars(extend(
        "x-section" = true,
        "x-tab" = "Leads",
        "x-enable-when" = {"lead": ["straight", "circular"]}
    ))]
    pub leads: Leads,
    #[serde(default)]
    #[schemars(title = "Order ways to try", extend("x-section" = true, "x-tab" = "Order"))]
    pub order: Order,
    #[serde(default)]
    #[schemars(extend("x-tab" = "Start At"))]
    pub start_at: StartAt,
    /// The point of each vector's bounding box its cut starts nearest
    #[serde(default = "top_left")]
    #[schemars(extend("x-tab" = "Start At", "x-enable-when" = {"start_at": "bounding_box"}))]
    pub start_near: Anchor,
    /// At outside corners the bit follows the vector's angle out to a point instead of
    /// rolling round the corner
    #[serde(default)]
    #[schemars(title = "Sharp External Corners", extend("x-tab" = "Corners"))]
    pub sharp_corners: bool,
    /// Drop the finished toolpath onto the 3D model: its depth below the material surface
    /// becomes depth below the model's surface, going no lower than the bottom of the material
    #[serde(default)]
    #[schemars(title = "Project toolpath onto 3D Model")]
    pub project_onto_model: bool,
    /// What the toolpath is called in the toolpath list
    pub name: String,
}

fn top_left() -> Anchor {
    Anchor::TopLeft
}

impl Default for Params {
    fn default() -> Self {
        Self {
            name: "Profile".into(),
            depth: 6.0,
            through: false,
            side: Side::Outside,
            tool: Tool::default(),
            direction: Direction::Climb,
            ways: Ways::One,
            motion: Motion::Straight,
            loops: Loops::default(),
            allowance: 0.0,
            ramp: Ramp::None,
            ramping: Ramping::default(),
            entry: Some(EntrySettings::default()),
            lead: Lead::None,
            leads: Leads::default(),
            tabs: Tabs::None,
            tab: TabSettings::default(),
            tab_points: vec![],
            project_onto_model: false,
            order: Order::default(),
            start_at: StartAt::Optimize,
            start_near: top_left(),
            sharp_corners: false,
        }
    }
}

impl Params {
    /// How deep the cut goes in material `thickness` thick: right through it when asked, else
    /// the cut depth.
    #[must_use]
    pub fn cut_depth(&self, thickness: f64) -> f64 {
        if self.through { thickness } else { self.depth }
    }
}

impl Command for ProfileToolpath {
    const ID: &'static str = "profile_toolpath";
    type Params = Params;

    fn meta() -> Meta {
        Meta {
            label: "Profile Toolpath",
            group: "Toolpath Operations",
            icon: "toolpaths/profile_toolpath.png",
            shortcut: "",
            form: Form::TOOLPATH,
        }
    }

    fn run(doc: &Document, p: &Params) -> Result<Change, CommandError> {
        let depth = p.cut_depth(doc.job.thickness);
        ensure_positive("depth", depth)?;
        ensure_positive("bit diameter", p.tool.diameter)?;
        ensure_positive("pass depth", p.tool.pass_depth)?;
        let (ids, paths) = doc.selected_paths(|path| path.closed || p.side == Side::On);
        if ids.is_empty() {
            return Err(CommandError::Failed("select the vectors to profile".into()));
        }
        let radius = p.tool.diameter / 2.0 + p.allowance;
        // a vector inside another is a hole in it (an "O"), so offsetting the regions cuts
        // each on its proper side
        // ponytail: overlapping vectors go even-odd too, their overlap a hole; the way
        // with overlaps is not known
        let offset = |by: f64| {
            let regions = polygon::regions(&paths);
            if p.sharp_corners {
                polygon::offset_regions_mitred(&regions, by, MITRE_REACH)
            } else {
                polygon::offset_regions(&regions, by, Corners::Round)
            }
        };
        let centre_line = match p.side {
            Side::Outside => offset(radius),
            Side::Inside => offset(-radius),
            Side::On => paths.clone(),
        };
        if centre_line.is_empty() {
            return Err(tool_does_not_fit(&p.tool, Some("cut Outside instead")));
        }
        let depths = p.tool.pass_depths(depth);
        let tab = p.tab.sized(p.tool.pass_depth, depth);
        let entry = entry_for(p, depth, tab);
        let lines = ordered(cuts(&centre_line, &paths, p), &paths, p);
        // a helix goes in the waste: inside a hole's centre line, or outside every part's
        let parts: Vec<Vec<Point>> = lines
            .iter()
            .filter(|c| c.closed)
            .map(|c| c.points.clone())
            .collect();
        // each clicked tab goes on the path nearest it
        let nearest = |at: Point| {
            (0..lines.len()).min_by(|&a, &b| {
                let gap = |i: usize| polyline_distance(&lines[i].points, at);
                gap(a).total_cmp(&gap(b))
            })
        };
        let owners: Vec<Option<usize>> = p.tab_points.iter().map(|&t| nearest(t)).collect();
        if p.tabs != Tabs::None && p.tab.spacing == TabSpacing::Placed && p.tab_points.is_empty() {
            return Err(CommandError::Failed(
                "click on the vectors in the 2D view to place the tabs".into(),
            ));
        }
        let mut moves: Vec<_> = lines
            .iter()
            .enumerate()
            .flat_map(|(i, cut)| {
                // leads come from the waste: outside a part's outline, inside a hole; an
                // anticlockwise path has its outside on the right
                let anticlockwise = cut.closed && winding(&cut.points) > 0.0;
                let outside = if anticlockwise { -1.0 } else { 1.0 };
                let side = if !cut.closed {
                    1.0
                } else if cut.waste_inside {
                    -outside
                } else {
                    outside
                };
                let placed = p
                    .tab_points
                    .iter()
                    .zip(&owners)
                    .filter(|&(_, &owner)| owner == Some(i))
                    .map(|(&t, _)| t)
                    .collect();
                let room = match (cut.closed && p.side != Side::On, cut.waste_inside) {
                    (false, _) => None,
                    (true, true) => Some(Room::inside(vec![cut.points.clone()])),
                    (true, false) => Some(Room::outside(parts.clone())),
                };
                let entry = Entry {
                    side,
                    placed,
                    room,
                    ..entry.clone()
                };
                let safe = doc.job.clearance;
                match p.motion {
                    Motion::Straight => {
                        directed_moves(p.ways, &cut.points, cut.closed, &depths, 0.0, safe, &entry)
                    }
                    Motion::Trochoidal => {
                        trochoidal(cut, &depths, &entry, safe, p, &doc.job.clearing)
                    }
                }
            })
            .collect();
        if p.project_onto_model {
            moves = project_onto_model(doc, &p.tool, &moves)?;
        }
        let mut toolpath = made_by::<Self>(p, p.name.clone(), p.tool.clone(), ids, moves);
        toolpath.fallbacks = entry.take_fallbacks();
        let mut c = Change::builder(doc);
        c.add_toolpath(toolpath);
        Ok(c.finish())
    }
}

/// How `p` gets into and out of its paths, for a cut `depth` deep with tabs `tab`; the side,
/// the room and the placed tabs are each path's own.
fn entry_for(p: &Params, depth: f64, tab: TabSettings) -> Entry {
    Entry {
        ramp: if p.entry.is_some() {
            Ramp::None
        } else {
            p.ramp
        },
        ramping: p.ramping,
        lead: p.lead,
        leads: p.leads,
        side: 1.0,
        tabs: p.tabs,
        tab,
        tab_top: -depth + tab.thickness,
        tool_diameter: p.tool.diameter,
        placed: vec![],
        enter: p.entry,
        room: None,
        fallbacks: Fallbacks::default(),
        no_plunge: p.tool.cannot_plunge(),
    }
}

/// The trochoidal cut of `cut`: loops whose centres ride a loop's radius out into the waste
/// (on the line itself for On), so each loop just reaches the line, then the line straight.
/// The loops cut every pass down to the tabs' tops and the line the rest, so the tabs are
/// left; with no tabs the loops go all the way and the line is cut once at the bottom.
/// The loops' size and turn come from the job's Clearing Settings; with Both directions,
/// a pass back runs the other way along with its loops still turning the same way when they
/// keep climb.
fn trochoidal(
    cut: &Cut,
    depths: &[f64],
    entry: &Entry,
    safe: f64,
    p: &Params,
    clearing: &cut_craft_core::entry::ClearingSettings,
) -> Vec<cut_craft_core::toolpath::Move> {
    let (radius, advance) = clearing.loops.sized(p.tool.diameter);
    let anticlockwise = clearing.loops.anticlockwise();
    let rings: Vec<(Vec<Point>, bool)> = if cut.closed && p.side != Side::On {
        let out = if cut.waste_inside { -radius } else { radius };
        let ring = Path::polyline(&cut.points, true);
        polygon::offset(&[ring], out, Corners::Round)
            .iter()
            .map(|r| (p.direction.orient(cut_points(r), cut.waste_inside), true))
            .collect()
    } else {
        vec![(cut.points.clone(), cut.closed)]
    };
    let (loop_depths, line_depths): (Vec<f64>, Vec<f64>) = if entry.tabs == Tabs::None {
        (
            depths.to_vec(),
            depths.last().copied().into_iter().collect(),
        )
    } else {
        depths.iter().partition(|&&z| z > entry.tab_top)
    };
    let looping = Entry {
        lead: Lead::None,
        tabs: Tabs::None,
        ..entry.clone()
    };
    let mut moves: Vec<_> = rings
        .iter()
        .flat_map(|(ring, closed)| {
            let loops = trochoid(ring, *closed, radius, advance, anticlockwise);
            if p.ways == Ways::One || !clearing.keeps_climb() || loop_depths.len() < 2 {
                return directed_moves(p.ways, &loops, false, &loop_depths, 0.0, safe, &looping);
            }
            let reversed: Vec<Point> = ring.iter().rev().copied().collect();
            let back = trochoid(&reversed, *closed, radius, advance, anticlockwise);
            let mut above = 0.0;
            let mut moves = vec![];
            for (pass, &z) in loop_depths.iter().enumerate() {
                let way = if pass % 2 == 0 { &loops } else { &back };
                moves.extend(cut_craft_core::entry::path_moves(
                    way,
                    false,
                    &[z],
                    above,
                    safe,
                    &looping,
                ));
                above = z;
            }
            moves
        })
        .collect();
    let top = loop_depths.last().copied().unwrap_or(0.0);
    moves.extend(directed_moves(
        p.ways,
        &cut.points,
        cut.closed,
        &line_depths,
        top,
        safe,
        entry,
    ));
    moves
}

/// One path the bit's centre follows.
struct Cut {
    points: Vec<Point>,
    closed: bool,
    /// Whether the waste is inside it (a hole), not outside (a part).
    waste_inside: bool,
    /// Which of the selected vectors it was made from, by selection order.
    owner: usize,
}

/// The centre line's paths, each turned to run the way `p.direction` cuts and knowing which
/// side its waste is and which of the selected `vectors` it was made from.
fn cuts(centre_line: &[Path], vectors: &[Path], p: &Params) -> Vec<Cut> {
    let vectors: Vec<Vec<Point>> = vectors
        .iter()
        .map(|v| v.flatten(polygon::TOLERANCE))
        .collect();
    centre_line
        .iter()
        .map(|path| {
            let points = cut_points(path);
            let offset = path.closed && p.side != Side::On;
            // an offset ring running anticlockwise is the outline of where the tool's centre
            // goes, clockwise a hole in it: outside the vectors the waste is outside outlines
            // and inside holes, inside them the other way round
            let waste_inside = offset && (winding(&points) > 0.0) == (p.side == Side::Inside);
            // along a closed line, climb runs clockwise as round a part; an open line runs the
            // way it was drawn
            let points = if path.closed {
                p.direction.orient(points, waste_inside)
            } else {
                points
            };
            Cut {
                owner: owner(&points, &vectors),
                points,
                closed: path.closed,
                waste_inside,
            }
        })
        .collect()
}

/// Which of `vectors` the path through `points` was made from: the nearest, over a few points
/// along it.
fn owner(points: &[Point], vectors: &[Vec<Point>]) -> usize {
    let step = (points.len() / 8).max(1);
    let gaps: Vec<f64> = vectors
        .iter()
        .map(|v| {
            points
                .iter()
                .step_by(step)
                .map(|&q| polyline_distance(v, q))
                .sum()
        })
        .collect();
    (0..gaps.len())
        .min_by(|&a, &b| gaps[a].total_cmp(&gaps[b]))
        .unwrap_or(0)
}

/// The cuts in the order each option asks for, the one with the shortest moves between cuts,
/// each closed one starting where `p.start_at` says.
fn ordered(cuts: Vec<Cut>, vectors: &[Path], p: &Params) -> Vec<Cut> {
    let n = cuts.len();
    let area = |c: &Cut| {
        if c.closed {
            winding(&c.points).abs()
        } else {
            0.0
        }
    };
    // the cuts inside each one, which go first
    let inner: Vec<Vec<usize>> = (0..n)
        .map(|i| {
            let ring = std::slice::from_ref(&cuts[i].points);
            (0..n)
                .filter(|&j| {
                    cuts[i].closed
                        && area(&cuts[j]) < area(&cuts[i])
                        && inside(ring, cuts[j].points[0])
                })
                .collect()
        })
        .collect();
    let rings: Vec<Vec<Point>> = cuts
        .iter()
        .map(|c| c.points.iter().chain(&c.points[..1]).copied().collect())
        .collect();
    let boxes: Vec<Rect> = cuts.iter().map(|c| bounds(&c.points)).collect();
    let start = |i: usize, here: Point| {
        let c = &cuts[i];
        if !c.closed {
            return c.points[0];
        }
        let at = match p.start_at {
            StartAt::KeepCurrent => polyline_closest(&rings[i], vectors[c.owner].start),
            StartAt::Optimize => polyline_closest(&rings[i], here),
            StartAt::BoundingBox => {
                // the box round the path itself: round a corner every point is as near the
                // vector's own box
                let near = p.start_near.point(boxes[i]);
                c.points
                    .iter()
                    .copied()
                    .min_by(|a, b| a.distance(near).total_cmp(&b.distance(near)))
            }
        };
        at.unwrap_or(c.points[0])
    };
    let middle = |i: usize| boxes[i].center();
    let o = p.order;
    let mut ranks: Vec<Option<Vec<usize>>> = vec![];
    let none = !(o.vector_selection_order
        || o.left_to_right
        || o.bottom_to_top
        || o.grid
        || o.shortest_path);
    if o.vector_selection_order || none {
        #[expect(clippy::cast_precision_loss, reason = "a count of vectors")]
        ranks.push(Some(rank(n, |i| (cuts[i].owner as f64, 0.0))));
    }
    if o.left_to_right {
        ranks.push(Some(rank(n, |i| (middle(i).x, middle(i).y))));
    }
    if o.bottom_to_top {
        ranks.push(Some(rank(n, |i| (middle(i).y, middle(i).x))));
    }
    if o.grid {
        let all = boxes.iter().fold(boxes[0], |a, &b| a.union(b));
        // squares as big as the biggest part, a row at a time from the bottom, along the
        // first row and back along the next
        let cell = boxes
            .iter()
            .map(|b| b.size().max_element())
            .fold(0.0, f64::max)
            .max(1e-9);
        ranks.push(Some(rank(n, |i| {
            let at = (middle(i) - all.min) / cell;
            let (row, column) = (at.y.floor(), at.x.floor());
            let back = (row.rem_euclid(2.0) - 1.0).abs() < 0.5;
            (row, if back { -column } else { column })
        })));
    }
    if o.shortest_path {
        ranks.push(None);
    }
    let (order, _) = ranks
        .iter()
        .map(|rank| sequence(&cuts, &inner, rank.as_deref(), &start))
        .min_by(|a, b| a.1.total_cmp(&b.1))
        .expect("at least one ordering is tried");
    let mut cuts: Vec<Option<Cut>> = cuts.into_iter().map(Some).collect();
    order
        .into_iter()
        .map(|(i, at)| {
            let cut = cuts[i].take().expect("each cut once");
            if cut.closed {
                Cut {
                    points: starting_at(&cut.points, at),
                    ..cut
                }
            } else {
                cut
            }
        })
        .collect()
}

/// Where each of `n` items comes when sorted by `key`, ties in their own order.
fn rank(n: usize, key: impl Fn(usize) -> (f64, f64)) -> Vec<usize> {
    let mut order: Vec<usize> = (0..n).collect();
    order.sort_by(|&a, &b| {
        let (a, b) = (key(a), key(b));
        a.0.total_cmp(&b.0).then(a.1.total_cmp(&b.1))
    });
    let mut rank = vec![0; n];
    for (place, i) in order.into_iter().enumerate() {
        rank[i] = place;
    }
    rank
}

/// The cuts in the order they run, each with the point it starts at, and how far the bit
/// moves between them (from the origin to the first). `rank` puts them in order; without one
/// each goes on to the nearest. Either way a cut waits for the cuts inside it.
// ponytail: tries every cut at each step (n² start points); a spatial index if jobs get huge
fn sequence(
    cuts: &[Cut],
    inner: &[Vec<usize>],
    rank: Option<&[usize]>,
    start: &dyn Fn(usize, Point) -> Point,
) -> (Vec<(usize, Point)>, f64) {
    let mut done = vec![false; cuts.len()];
    let mut here = Point::ZERO;
    let mut order = vec![];
    let mut travel = 0.0;
    while order.len() < cuts.len() {
        let ready = (0..cuts.len()).filter(|&i| !done[i] && inner[i].iter().all(|&j| done[j]));
        let next = match rank {
            Some(rank) => ready.min_by_key(|&i| rank[i]),
            None => ready.min_by(|&a, &b| {
                let gap = |i: usize| here.distance(start(i, here));
                gap(a).total_cmp(&gap(b))
            }),
        }
        .expect("a cut's inner cuts are smaller than it, so one is always ready");
        let at = start(next, here);
        travel += here.distance(at);
        // a closed cut ends where it began, an open one at its end
        here = if cuts[next].closed {
            at
        } else {
            cuts[next].points[cuts[next].points.len() - 1]
        };
        done[next] = true;
        order.push((next, at));
    }
    (order, travel)
}

/// The closed path through `points` starting at `at`, a point on it.
fn starting_at(points: &[Point], at: Point) -> Vec<Point> {
    let n = points.len();
    let gap = |i: usize| polyline_distance(&[points[i], points[(i + 1) % n]], at);
    let span = (0..n)
        .min_by(|&a, &b| gap(a).total_cmp(&gap(b)))
        .unwrap_or(0);
    let mut out = vec![at];
    out.extend((1..=n).map(|d| points[(span + d) % n]));
    // a start on a corner is not repeated
    out.dedup_by(|a, b| a.distance(*b) < 1e-9);
    if out.len() > 1 && out[out.len() - 1].distance(at) < 1e-9 {
        out.pop();
    }
    out
}

fn bounds(points: &[Point]) -> Rect {
    points
        .iter()
        .fold(Rect::point(points[0]), |r, &q| r.include(q))
}

#[cfg(test)]
mod tests;
