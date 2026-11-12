//! V-Carve / Engrave Toolpath: a V-bit carves the selected shapes, going deeper the further
//! it is from their edges so its sides cut the edges exactly.

use cut_craft_core::document::Document;
use cut_craft_core::geometry::{Path, Point, inside};
use cut_craft_core::polygon::{self, Corners};
use cut_craft_core::toolpath::{
    Move, Tool, ToolDatabase, ToolShape, contour_moves, cut_points, made_by, project_onto_model,
};
use cut_craft_core::{Change, Command, CommandError, Meta, ensure_positive};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub struct VcarveEngraveToolpath;

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
    #[schemars(extend("x-group" = "Cutting Depths"))]
    pub flat_depth: f64,
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
        let (ids, paths) = doc.selected_paths(|path| path.closed);
        if ids.is_empty() {
            return Err(CommandError::Failed(
                "select the closed vectors to carve".into(),
            ));
        }
        if p.start_depth < 0.0 {
            return Err(CommandError::InvalidParams(
                "set start depth to 0 or more".into(),
            ));
        }
        let mut moves = carve(&paths, p, doc.job.clearance);
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
/// The edges are cut at the start depth; further in it goes deeper until the cone is as
/// wide as the bit or reaches the flat depth.
/// With the vector selection order, each outline (and the vectors inside it) is carved
/// completely before the next one selected.
pub fn carve(paths: &[Path], p: &Params, safe: f64) -> Vec<Move> {
    if !p.use_vector_selection_order {
        return carve_together(paths, p, safe);
    }
    shapes(paths)
        .iter()
        .flat_map(|shape| carve_together(shape, p, safe))
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

/// The carving of all `paths` together, ring by ring from the edges in.
fn carve_together(paths: &[Path], p: &Params, safe: f64) -> Vec<Move> {
    let starts: Vec<Point> = if p.use_vector_start_points {
        paths.iter().map(|path| path.start).collect()
    } else {
        vec![]
    };
    let tool = &p.tool;
    // how far the cone reaches sideways for each millimetre down
    let spread = (tool.angle.to_radians() / 2.0).tan();
    // further in than this the cone cannot reach the edges, so the bottom is cut flat
    let reach = match p.flat_depth {
        limit if limit > 0.0 => (tool.diameter / 2.0).min(limit * spread),
        _ => tool.diameter / 2.0,
    };
    // the V-bit's rings make the carved surface: a finishing stepover
    let step = tool.finishing_stepover() * tool.diameter;
    // the flat bottom's middle, cleared by end mills, is left alone
    let last = p.cleared_within.map_or(f64::INFINITY, |r| reach + r);
    let mut moves = vec![];
    for (offset, rings) in inward_offsets(&polygon::regions(paths), step) {
        if offset > last {
            break;
        }
        let depths = tool.pass_depths(p.start_depth + offset.min(reach) / spread);
        for ring in &rings {
            let points = started_near(cut_points(ring), &starts);
            moves.extend(contour_moves(&points, true, &depths, safe));
        }
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

/// The regions shrunk by `step`, `2 step`, ... until nothing is left, plus about where the
/// last of them vanishes, so shapes narrower than a step are still carved down the middle.
/// Each pass shrinks the one before it by a step (shrinking by a then by b, with round
/// corners, is shrinking by a + b), so later passes work on less and less: shrinking the
/// whole design each time grew slower the deeper it went, a minute on a 3,000-shape SVG.
// ponytail: concentric passes; a medial-axis path would carve the middle in one pass
fn inward_offsets(regions: &[Path], step: f64) -> Vec<(f64, Vec<Path>)> {
    let shrink = |from: &[Path], by: f64| polygon::offset_regions_again(from, -by);
    let mut offsets: Vec<(f64, Vec<Path>)> = vec![];
    let mut offset = step;
    let mut rings = shrink(regions, step);
    while !rings.is_empty() {
        let next = shrink(&rings, step);
        offsets.push((offset, rings));
        rings = next;
        offset += step;
    }
    // between the last pass and a step further in, where its rings vanish
    let (inside, from) = offsets
        .last()
        .map_or((0.0, regions), |(o, r)| (*o, r.as_slice()));
    let (mut low, mut high) = (inside, offset);
    let mut last = None;
    for _ in 0..6 {
        let middle = f64::midpoint(low, high);
        let rings = shrink(from, middle - inside);
        if rings.is_empty() {
            high = middle;
        } else {
            low = middle;
            last = Some((middle, rings));
        }
    }
    offsets.extend(last);
    offsets
}

#[cfg(test)]
mod tests {
    use super::*;
    use cut_craft_core::document::Job;
    use cut_craft_core::geometry::polyline_distance;
    use cut_craft_core::simulation::Material;
    use cut_craft_core::testing::{made_toolpath, square};
    use cut_craft_core::toolpath::{MoveKind, Toolpath};

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

    #[test]
    fn each_pass_is_the_shapes_shrunk_by_its_offset() {
        // a star with a hole, and a thin sliver that vanishes between passes
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
        let offsets = inward_offsets(&regions, 0.5);
        assert!(offsets.len() > 5);
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
        // the sliver, 1.1 wide, still gets a pass near its middle
        let near_middle = |y: f64| (y - 0.55).abs() < 0.1;
        assert!(offsets.iter().any(|(_, rings)| rings.iter().any(|r| {
            let b = r.bounds();
            b.min.x > 19.0 && near_middle(b.center().y)
        })));
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
