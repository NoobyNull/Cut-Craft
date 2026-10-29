//! Profile Toolpath: cuts along the selected vectors, outside, inside or on the line.

use cut_craft_core::document::Document;
use cut_craft_core::entry::{
    Direction, Entry, Lead, Leads, Ramp, Ramping, TabSettings, TabSpacing, Tabs, path_moves,
};
use cut_craft_core::geometry::{
    Path, Point, Rect, inside, polyline_closest, polyline_distance, winding,
};
use cut_craft_core::params::Anchor;
use cut_craft_core::polygon::{self, Corners};
use cut_craft_core::toolpath::{Tool, cut_points, made_by, pass_depths, project_onto_model};
use cut_craft_core::{Change, Command, CommandError, Meta, ensure_positive};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub struct ProfileToolpath;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Side {
    /// Around the outside of closed shapes
    #[default]
    Outside,
    /// Around the inside of closed shapes
    Inside,
    /// Along the line itself, open vectors included
    On,
}

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
        // ponytail: Aspire's defaults are not known; trying them all costs little
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

/// How far out a sharp corner may go, in tool radii, before it is squared off: corners down to
/// about 29° come to a point.
// ponytail: Aspire's limit is not known
pub const MITRE_REACH: f64 = 4.0;

// Laid out as Aspire's 2D Profile form: cutting depths, the tool, how the vectors are
// machined, tabs, then a tab each for ramps, leads, order, start points and corners.
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
pub struct Params {
    /// Below the material surface
    #[schemars(
        title = "Cut Depth",
        extend("x-unit" = "mm", "x-group" = "Cutting Depths"),
        range(min = 0)
    )]
    pub depth: f64,
    pub tool: Tool,
    #[schemars(title = "Machine", extend("x-group" = "Machine Vectors…"))]
    pub side: Side,
    /// Which way round closed shapes are cut
    #[serde(default)]
    #[schemars(extend("x-group" = "Machine Vectors…", "x-widget" = "row"))]
    pub direction: Direction,
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
    #[serde(default)]
    #[schemars(
        title = "Tab Size",
        extend(
            "x-section" = true,
            "x-group" = "Tabs",
            "x-enable-when" = {"tabs": ["flat", "triangular"]}
        )
    )]
    pub tab: TabSettings,
    /// Tabs placed by clicking in the 2D view, on the vectors
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[schemars(skip)]
    pub tab_points: Vec<Point>,
    /// How the tool goes down into each pass (ramps run at the plunge rate)
    #[serde(default)]
    #[schemars(title = "Add ramps to toolpath", extend("x-tab" = "Ramp"))]
    pub ramp: Ramp,
    #[serde(default)]
    #[schemars(
        title = "Specify Ramp",
        extend(
            "x-section" = true,
            "x-tab" = "Ramp",
            "x-enable-when" = {"ramp": ["smooth", "zig_zag"]}
        )
    )]
    pub ramping: Ramping,
    /// How the tool comes onto each path, from the waste side; a ramp then runs along the
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
    /// At outside corners the tool follows the vector's angle out to a point instead of
    /// rolling round the corner
    #[serde(default)]
    #[schemars(title = "Sharp External Corners", extend("x-tab" = "Corners"))]
    pub sharp_corners: bool,
    /// Drop the finished toolpath onto the 3D model: its depth below the material surface
    /// becomes depth below the model's surface, going no lower than the bottom of the material
    #[serde(default)]
    #[schemars(title = "Project toolpath onto 3D Model")]
    pub project_onto_model: bool,
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
            side: Side::Outside,
            tool: Tool::default(),
            direction: Direction::Climb,
            allowance: 0.0,
            ramp: Ramp::None,
            ramping: Ramping::default(),
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

impl Command for ProfileToolpath {
    const ID: &'static str = "profile_toolpath";
    type Params = Params;

    fn meta() -> Meta {
        Meta {
            label: "Profile Toolpath",
            group: "Toolpath Operations",
            icon: "toolpaths/profile_toolpath.png",
            shortcut: "",
        }
    }

    fn run(doc: &Document, p: &Params) -> Result<Change, CommandError> {
        ensure_positive("depth", p.depth)?;
        ensure_positive("tool diameter", p.tool.diameter)?;
        ensure_positive("pass depth", p.tool.pass_depth)?;
        let (ids, paths) = doc.selected_paths(|path| path.closed || p.side == Side::On);
        if ids.is_empty() {
            return Err(CommandError::Failed("select the vectors to profile".into()));
        }
        let radius = p.tool.diameter / 2.0 + p.allowance;
        // a vector inside another is a hole in it (an "O"), so offsetting the regions cuts
        // each on its proper side
        // ponytail: overlapping vectors go even-odd too, their overlap a hole; Aspire's way
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
            return Err(CommandError::Failed(
                "the tool is too big for the inside of the selected shapes".into(),
            ));
        }
        let depths = pass_depths(p.depth, p.tool.pass_depth);
        let tab = p.tab.sized(p.tool.pass_depth, p.depth);
        let entry = Entry {
            ramp: p.ramp,
            ramping: p.ramping,
            lead: p.lead,
            leads: p.leads,
            side: 1.0,
            tabs: p.tabs,
            tab,
            tab_top: -p.depth + tab.thickness,
            tool_diameter: p.tool.diameter,
            placed: vec![],
        };
        let lines = ordered(cuts(&centre_line, &paths, p), &paths, p);
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
                path_moves(
                    &cut.points,
                    cut.closed,
                    &depths,
                    0.0,
                    doc.job.clearance,
                    &Entry {
                        side,
                        placed,
                        ..entry.clone()
                    },
                )
            })
            .collect();
        if p.project_onto_model {
            moves = project_onto_model(doc, &p.tool, &moves)?;
        }
        let mut c = Change::builder(doc);
        c.add_toolpath(made_by::<Self>(
            p,
            p.name.clone(),
            p.tool.clone(),
            ids,
            moves,
        ));
        Ok(c.finish())
    }
}

/// One path the tool's centre follows.
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
            let points = if offset {
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

/// The cuts in the order they run, each with the point it starts at, and how far the tool
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
mod tests {
    use super::*;
    use cut_craft_core::geometry::Rect;
    use cut_craft_core::testing::{cut_bounds, made_toolpath, square};
    use cut_craft_core::toolpath::{MoveKind, Toolpath};

    cut_craft_core::contract_tests!(ProfileToolpath);

    fn profile(side: Side, square_side: f64) -> Result<Toolpath, String> {
        made_toolpath::<ProfileToolpath>(
            vec![square(0.0, 0.0, square_side)],
            &Params {
                side,
                ..Params::default()
            },
        )
    }

    fn near(a: Rect, min: (f64, f64), max: (f64, f64)) -> bool {
        a.min.distance(Point::new(min.0, min.1)) < 0.02
            && a.max.distance(Point::new(max.0, max.1)) < 0.02
    }

    #[test]
    fn outside_runs_a_tool_radius_outside_the_shape() {
        let b = cut_bounds(&profile(Side::Outside, 10.0).unwrap());
        assert!(near(b, (-3.0, -3.0), (13.0, 13.0)), "{b:?}");
    }

    #[test]
    fn inside_runs_a_tool_radius_inside_the_shape() {
        let b = cut_bounds(&profile(Side::Inside, 10.0).unwrap());
        assert!(near(b, (3.0, 3.0), (7.0, 7.0)), "{b:?}");
    }

    #[test]
    fn on_follows_the_line_itself() {
        let b = cut_bounds(&profile(Side::On, 10.0).unwrap());
        assert!(near(b, (0.0, 0.0), (10.0, 10.0)), "{b:?}");
    }

    #[test]
    fn cuts_in_passes_down_to_the_depth() {
        let tp = profile(Side::Outside, 10.0).unwrap();
        let plunges: Vec<f64> = tp
            .moves
            .iter()
            .filter(|m| m.kind == MoveKind::Plunge)
            .map(|m| m.to.z)
            .collect();
        assert_eq!(plunges, [-3.0, -6.0], "6 mm deep in 3 mm passes");
    }

    #[test]
    fn remembers_how_it_was_made_so_it_can_be_recalculated() {
        let tp = profile(Side::Inside, 10.0).unwrap();
        assert_eq!(tp.strategy, "profile_toolpath");
        assert_eq!(tp.params["side"], "inside");
        assert_eq!(tp.vectors.len(), 1);
    }

    #[test]
    fn a_shape_smaller_than_the_tool_cannot_be_cut_inside() {
        assert_eq!(
            profile(Side::Inside, 4.0).unwrap_err(),
            "the tool is too big for the inside of the selected shapes"
        );
    }

    #[test]
    fn leads_come_from_outside_for_an_outside_profile_and_inside_for_an_inside_one() {
        for (side, from_outside) in [(Side::Outside, true), (Side::Inside, false)] {
            let tp = made_toolpath::<ProfileToolpath>(
                vec![square(0.0, 0.0, 20.0)],
                &Params {
                    side,
                    lead: Lead::Straight,
                    leads: Leads {
                        angle: 90.0,
                        length: 2.0,
                        ..Leads::default()
                    },
                    ..Params::default()
                },
            )
            .unwrap();
            let plunge = tp
                .moves
                .iter()
                .find(|m| m.kind == MoveKind::Plunge)
                .unwrap();
            let p = plunge.to.truncate();
            let inside_square = p.x > 0.0 && p.x < 20.0 && p.y > 0.0 && p.y < 20.0;
            let off_line = !cut_bounds(&tp).contains(p) || inside_square;
            assert!(off_line, "{side:?}: {p}");
            assert_eq!(!inside_square, from_outside, "{side:?}: {p}");
        }
    }

    #[test]
    fn tabs_are_left_at_the_bottom_of_the_cut() {
        let tp = made_toolpath::<ProfileToolpath>(
            vec![square(0.0, 0.0, 40.0)],
            &Params {
                tabs: Tabs::Flat,
                ramp: Ramp::Smooth,
                ..Params::default()
            },
        )
        .unwrap();
        let sized = TabSettings::default().sized(Tool::default().pass_depth, 6.0);
        let tab_top = -6.0 + sized.thickness;
        assert!(
            tp.moves
                .iter()
                .any(|m| m.kind == MoveKind::Cut && (m.to.z - tab_top).abs() < 1e-9)
        );
        assert!(tp.moves.iter().any(|m| (m.to.z + 6.0).abs() < 1e-9));
    }

    /// The way round the first closed pass at full depth runs: true anticlockwise.
    fn anticlockwise(tp: &Toolpath) -> bool {
        let cuts: Vec<Point> = tp
            .moves
            .iter()
            .filter(|m| m.kind == MoveKind::Cut)
            .map(|m| m.to.truncate())
            .collect();
        winding(&cuts) > 0.0
    }

    #[test]
    fn climb_runs_clockwise_round_a_part_and_anticlockwise_round_a_hole() {
        for (side, direction, expect) in [
            (Side::Outside, Direction::Climb, false),
            (Side::Outside, Direction::Conventional, true),
            (Side::Inside, Direction::Climb, true),
            (Side::Inside, Direction::Conventional, false),
        ] {
            let tp = made_toolpath::<ProfileToolpath>(
                vec![square(0.0, 0.0, 10.0)],
                &Params {
                    side,
                    direction,
                    ..Params::default()
                },
            )
            .unwrap();
            assert_eq!(anticlockwise(&tp), expect, "{side:?} {direction:?}");
        }
    }

    #[test]
    fn the_allowance_leaves_the_part_bigger() {
        let tp = made_toolpath::<ProfileToolpath>(
            vec![square(0.0, 0.0, 10.0)],
            &Params {
                allowance: 1.0,
                ..Params::default()
            },
        )
        .unwrap();
        let b = cut_bounds(&tp);
        assert!(near(b, (-4.0, -4.0), (14.0, 14.0)), "{b:?}");
    }

    #[test]
    fn clicked_tabs_sit_where_clicked_on_the_path() {
        let tp = made_toolpath::<ProfileToolpath>(
            vec![square(0.0, 0.0, 40.0)],
            &Params {
                tabs: Tabs::Flat,
                tab: TabSettings {
                    spacing: TabSpacing::Placed,
                    size_from_tool: false,
                    length: 10.0,
                    thickness: 2.0,
                    ..TabSettings::default()
                },
                // on the vector's bottom edge, 10 mm in
                tab_points: vec![Point::new(10.0, 0.0)],
                ..Params::default()
            },
        )
        .unwrap();
        let top = -6.0 + 2.0;
        let lifted: Vec<Point> = tp
            .moves
            .iter()
            .filter(|m| (m.to.z - top).abs() < 1e-9)
            .map(|m| m.to.truncate())
            .collect();
        // one tab, lifted over its length and the tool, round x = 10 on the line 3 mm out
        assert!(lifted.len() > 1, "lifted over the tab");
        assert!(
            lifted
                .iter()
                .all(|p| (p.y + 3.0).abs() < 1e-6 && (p.x - 10.0).abs() <= 8.0 + 1e-6),
            "{lifted:?}"
        );
        let none = made_toolpath::<ProfileToolpath>(
            vec![square(0.0, 0.0, 40.0)],
            &Params {
                tabs: Tabs::Flat,
                tab: TabSettings {
                    spacing: TabSpacing::Placed,
                    ..TabSettings::default()
                },
                ..Params::default()
            },
        );
        assert!(none.unwrap_err().contains("click"));
    }

    /// The tabs a profile round a `w` x `h` rectangle leaves, cut `depth` deep with a 1/4"
    /// bit in 1/4" passes: (how many, their thickness, lifted length on the centre line).
    fn tabs_on(w: f64, h: f64, depth: f64) -> (usize, f64, f64) {
        let quarter = 6.35;
        let tool = Tool {
            diameter: quarter,
            pass_depth: quarter,
            ..Tool::default()
        };
        let tp = made_toolpath::<ProfileToolpath>(
            vec![cut_craft_core::geometry::Path::polyline(
                &[(0.0, 0.0), (w, 0.0), (w, h), (0.0, h)].map(|(x, y)| Point::new(x, y)),
                true,
            )],
            &Params {
                depth,
                tool,
                tabs: Tabs::Flat,
                ..Params::default()
            },
        )
        .unwrap();
        // the last pass: straight up onto each tab and straight down off it
        let ups: Vec<(Point, f64)> = tp
            .moves
            .windows(2)
            .filter(|w| {
                w[1].kind == MoveKind::Cut
                    && w[0].to.truncate() == w[1].to.truncate()
                    && w[1].to.z > w[0].to.z
                    && (w[0].to.z + depth).abs() < 1e-9
            })
            .map(|w| (w[1].to.truncate(), w[1].to.z))
            .collect();
        let downs: Vec<Point> = tp
            .moves
            .windows(2)
            .filter(|w| {
                w[1].kind == MoveKind::Cut
                    && w[0].to.truncate() == w[1].to.truncate()
                    && w[1].to.z < w[0].to.z
                    && (w[1].to.z + depth).abs() < 1e-9
            })
            .map(|w| w[1].to.truncate())
            .collect();
        let thickness = ups.first().map_or(0.0, |&(_, z)| z + depth);
        let length = ups
            .first()
            .zip(downs.first())
            .map_or(0.0, |((a, _), b)| a.distance(*b));
        (ups.len(), thickness, length)
    }

    #[test]
    fn tabs_are_sized_from_the_tool_and_spread_about_every_150_mm() {
        // 3/4" deep: 1/4" thick, 1/2" long (lifted over that and the bit's 1/4")
        let (count, thickness, length) = tabs_on(400.0, 800.0, 19.05);
        assert!((thickness - 6.35).abs() < 1e-6, "{thickness}");
        assert!((length - 3.0 * 6.35).abs() < 1e-6, "{length}");
        // round 2,440 mm of centre line: about one every 150 mm
        assert_eq!(count, 16);
        // a small part still gets three, and a shallow cut thinner tabs
        let (count, thickness, _) = tabs_on(60.0, 60.0, 6.0);
        assert_eq!(count, 3);
        assert!((thickness - 3.0).abs() < 1e-6, "half the cut: {thickness}");
        // a tiny one is not all tab
        assert_eq!(tabs_on(15.0, 15.0, 19.05).0, 1);
    }

    /// The cutting moves of each path, in the order the paths are cut: each starts where the
    /// tool comes down from a rapid.
    fn cuts(tp: &Toolpath) -> Vec<Vec<Point>> {
        let mut out: Vec<Vec<Point>> = vec![];
        for w in tp.moves.windows(2) {
            if w[0].kind == MoveKind::Rapid && w[1].kind == MoveKind::Plunge {
                out.push(vec![w[1].to.truncate()]);
            } else if w[1].kind == MoveKind::Cut {
                out.last_mut().unwrap().push(w[1].to.truncate());
            }
        }
        out
    }

    fn box_of(points: &[Point]) -> Rect {
        points
            .iter()
            .map(|&q| Rect::point(q))
            .reduce(Rect::union)
            .unwrap()
    }

    fn profiled(paths: Vec<Path>, p: &Params) -> Toolpath {
        made_toolpath::<ProfileToolpath>(paths, p).unwrap()
    }

    /// A 30 mm square with a 10 mm square hole in its middle, like the letter "O".
    fn o_shape() -> Vec<Path> {
        vec![square(0.0, 0.0, 30.0), square(10.0, 10.0, 10.0)]
    }

    #[test]
    fn an_o_is_cut_on_the_right_side_of_both_rings_the_inner_one_first() {
        for (side, inner, outer) in [
            (Side::Outside, (13.0, 17.0), (-3.0, 33.0)),
            (Side::Inside, (7.0, 23.0), (3.0, 27.0)),
        ] {
            let tp = profiled(
                o_shape(),
                &Params {
                    side,
                    ..Params::default()
                },
            );
            let cuts = cuts(&tp);
            assert_eq!(cuts.len(), 2, "{side:?}");
            let (a, b) = (box_of(&cuts[0]), box_of(&cuts[1]));
            assert!(
                near(a, (inner.0, inner.0), (inner.1, inner.1)),
                "{side:?} {a:?}"
            );
            assert!(
                near(b, (outer.0, outer.0), (outer.1, outer.1)),
                "{side:?} {b:?}"
            );
            // climb: anticlockwise round the waste inside a ring, clockwise round a part
            let waste_inside = side == Side::Outside;
            assert_eq!(winding(&cuts[0]) > 0.0, waste_inside, "{side:?}");
            assert_eq!(winding(&cuts[1]) > 0.0, !waste_inside, "{side:?}");
        }
    }

    /// The middle x (or y, `across` false) of each path cut, in order, with these orderings.
    fn cut_order(paths: Vec<Path>, order: Order, across: bool) -> Vec<f64> {
        let tp = profiled(
            paths,
            &Params {
                order,
                ..Params::default()
            },
        );
        cuts(&tp)
            .iter()
            .map(|c| {
                let b = box_of(c);
                let mid = (b.min + b.max) / 2.0;
                (if across { mid.x } else { mid.y }).round()
            })
            .collect()
    }

    const NONE: Order = Order {
        vector_selection_order: false,
        left_to_right: false,
        bottom_to_top: false,
        grid: false,
        shortest_path: false,
    };

    #[test]
    fn cuts_in_selection_order_left_to_right_or_bottom_to_top() {
        let row = || {
            vec![
                square(80.0, 0.0, 10.0),
                square(0.0, 0.0, 10.0),
                square(40.0, 0.0, 10.0),
            ]
        };
        let selection = Order {
            vector_selection_order: true,
            ..NONE
        };
        assert_eq!(cut_order(row(), selection, true), [85.0, 5.0, 45.0]);
        let left_to_right = Order {
            left_to_right: true,
            ..NONE
        };
        assert_eq!(cut_order(row(), left_to_right, true), [5.0, 45.0, 85.0]);
        let column = vec![
            square(0.0, 40.0, 10.0),
            square(0.0, 80.0, 10.0),
            square(0.0, 0.0, 10.0),
        ];
        let bottom_to_top = Order {
            bottom_to_top: true,
            ..NONE
        };
        assert_eq!(cut_order(column, bottom_to_top, false), [5.0, 45.0, 85.0]);
        // with both ticked the shorter wins, and so does the default
        let both = Order {
            vector_selection_order: true,
            left_to_right: true,
            ..NONE
        };
        assert_eq!(cut_order(row(), both, true), [5.0, 45.0, 85.0]);
        assert_eq!(cut_order(row(), Order::default(), true), [5.0, 45.0, 85.0]);
        // none ticked: as selected
        assert_eq!(cut_order(row(), NONE, true), [85.0, 5.0, 45.0]);
    }

    #[test]
    fn a_grid_finishes_one_row_of_squares_before_the_next_turning_back_along_it() {
        let parts = vec![
            square(0.0, 40.0, 10.0),
            square(40.0, 0.0, 10.0),
            square(40.0, 40.0, 10.0),
            square(0.0, 0.0, 10.0),
        ];
        let grid = Order { grid: true, ..NONE };
        let tp = profiled(
            parts,
            &Params {
                order: grid,
                ..Params::default()
            },
        );
        let middles: Vec<(f64, f64)> = cuts(&tp)
            .iter()
            .map(|c| {
                let b = box_of(c);
                let mid = (b.min + b.max) / 2.0;
                (mid.x.round(), mid.y.round())
            })
            .collect();
        assert_eq!(
            middles,
            [(5.0, 5.0), (45.0, 5.0), (45.0, 45.0), (5.0, 45.0)]
        );
    }

    #[test]
    fn inner_shapes_are_cut_first_whatever_the_order() {
        for order in [
            Order {
                vector_selection_order: true,
                ..NONE
            },
            Order {
                left_to_right: true,
                ..NONE
            },
            Order {
                shortest_path: true,
                ..NONE
            },
        ] {
            let mut paths = o_shape();
            // a separate part off to the left, selected last
            paths.push(square(-40.0, 0.0, 10.0));
            let tp = profiled(
                paths,
                &Params {
                    order,
                    ..Params::default()
                },
            );
            let boxes: Vec<Rect> = cuts(&tp).iter().map(|c| box_of(c)).collect();
            let hole = boxes.iter().position(|b| b.min.x > 10.0).unwrap();
            let ring = boxes
                .iter()
                .position(|b| b.min.x < 0.0 && b.max.x > 30.0)
                .unwrap();
            assert!(hole < ring, "{order:?}");
        }
    }

    fn first_start(paths: Vec<Path>, start_at: StartAt, start_near: Anchor) -> Point {
        let tp = profiled(
            paths,
            &Params {
                start_at,
                start_near,
                ..Params::default()
            },
        );
        cuts(&tp)[0][0]
    }

    #[test]
    fn keeps_the_vectors_own_start_point_moved_out_to_the_tool() {
        let p = Point::new;
        // a square starting half way along its bottom edge
        let drawn = Path::polyline(
            &[
                p(5.0, 0.0),
                p(10.0, 0.0),
                p(10.0, 10.0),
                p(0.0, 10.0),
                p(0.0, 0.0),
            ],
            true,
        );
        let at = first_start(vec![drawn], StartAt::KeepCurrent, Anchor::TopLeft);
        assert!(at.distance(p(5.0, -3.0)) < 1e-6, "{at}");
    }

    #[test]
    fn optimized_starts_are_the_nearest_points_to_where_the_tool_is() {
        let p = Point::new;
        let corner = 20.0 - 3.0 * std::f64::consts::FRAC_1_SQRT_2;
        let at = first_start(
            vec![square(20.0, 20.0, 10.0)],
            StartAt::Optimize,
            Anchor::TopLeft,
        );
        assert!(
            at.distance(p(corner, corner)) < 0.05,
            "from the origin: {at}"
        );
        // the next part starts on its corner nearest the first's start, not its own start
        let tp = profiled(
            vec![square(20.0, 20.0, 10.0), square(60.0, 20.0, 10.0)],
            &Params::default(),
        );
        let cuts = cuts(&tp);
        let (first, second) = (cuts[0][0], cuts[1][0]);
        assert!(first.distance(p(corner, corner)) < 0.05, "{first}");
        assert!(
            second.x < 57.1 && (second.y - 20.0).abs() < 0.5,
            "{first} then {second}"
        );
    }

    #[test]
    fn starts_at_the_span_end_nearest_the_chosen_point_of_the_bounding_box() {
        let at = first_start(
            vec![square(0.0, 0.0, 10.0)],
            StartAt::BoundingBox,
            Anchor::TopRight,
        );
        let corner = 10.0 + 3.0 * std::f64::consts::FRAC_1_SQRT_2;
        assert!(at.distance(Point::new(corner, corner)) < 0.3, "{at}");
        let at = first_start(
            vec![square(0.0, 0.0, 10.0)],
            StartAt::BoundingBox,
            Anchor::BottomLeft,
        );
        assert!(at.x < 0.0 && at.y < 0.0, "{at}");
    }

    #[test]
    fn sharp_corners_follow_the_vectors_angle_out_to_a_point_within_the_limit() {
        let sharp = Params {
            sharp_corners: true,
            ..Params::default()
        };
        let tp = profiled(vec![square(0.0, 0.0, 10.0)], &sharp);
        let points: Vec<Point> = cuts(&tp).concat();
        assert!(
            points
                .iter()
                .any(|q| q.distance(Point::new(-3.0, -3.0)) < 1e-6)
        );
        assert!(near(cut_bounds(&tp), (-3.0, -3.0), (13.0, 13.0)));
        // an 11° tip would mitre out 30 mm; it stops at the limit
        let p = Point::new;
        let thin = Path::polyline(&[p(0.0, 0.0), p(50.0, 0.0), p(50.0, 10.0)], true);
        let tp = profiled(vec![thin], &sharp);
        let furthest = cuts(&tp)
            .concat()
            .iter()
            .filter(|q| q.x < 0.0)
            .map(|q| q.length())
            .fold(0.0, f64::max);
        assert!(
            furthest > 3.0 && furthest <= MITRE_REACH * 3.0 + 1e-3,
            "{furthest}"
        );
    }

    #[test]
    fn projected_onto_the_model_it_follows_the_dome() {
        let mut doc = cut_craft_core::testing::domed();
        let mut c = Change::builder(&doc);
        let id = c.add(square(14.0, 14.0, 12.0));
        c.select(vec![id]);
        doc.apply(&c.finish()).unwrap();
        let p = Params {
            side: Side::On,
            depth: 1.0,
            project_onto_model: true,
            ..Params::default()
        };
        doc.apply(&ProfileToolpath::run(&doc, &p).unwrap()).unwrap();
        let tp = doc.toolpaths.values().next().unwrap();
        // the square's corners lie on the dome's slope: lower than its middles
        let at = |x: f64, y: f64| {
            tp.moves
                .iter()
                .filter(|m| m.kind == MoveKind::Cut)
                .find(|m| m.to.truncate().distance(Point::new(x, y)) < 0.2)
                .map(|m| m.to.z)
                .unwrap()
        };
        assert!(at(14.0, 14.0) < at(20.0, 14.0) - 0.5);
        assert!(
            at(20.0, 14.0) < -1.0,
            "a millimetre below the dome's surface"
        );
    }
}
