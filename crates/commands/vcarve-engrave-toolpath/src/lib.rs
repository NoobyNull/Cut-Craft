//! V-Carve / Engrave Toolpath: a V-bit carves the selected shapes, going deeper the further
//! it is from their edges so its sides cut the edges exactly.

use cut_craft_core::document::Document;
use cut_craft_core::geometry::Path;
use cut_craft_core::polygon::{self, Corners};
use cut_craft_core::toolpath::{
    Move, Tool, ToolDatabase, ToolShape, contour_moves, cut_points, made_by, pass_depths,
    project_onto_model,
};
use cut_craft_core::{Change, Command, CommandError, Meta, ensure_positive};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub struct VcarveEngraveToolpath;

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
pub struct Params {
    pub name: String,
    /// Below the material surface where carving starts (the floor of an earlier pocket)
    #[serde(default)]
    #[schemars(extend("x-unit" = "mm"), range(min = 0))]
    pub start_depth: f64,
    /// Deepest the bit goes below the start depth; wider parts are cut flat there (0: as
    /// deep as it can)
    #[schemars(extend("x-unit" = "mm"), range(min = 0))]
    pub flat_depth: f64,
    /// A V-bit; its stepover sets how close together the carving passes are
    pub tool: Tool,
    /// After calculating, drop the toolpath onto the 3D model: its depths are then below the
    /// model's surface, going no lower than the bottom of the material
    #[serde(default)]
    #[schemars(title = "Project toolpath onto 3D Model")]
    pub project_onto_model: bool,
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
            project_onto_model: false,
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
            icon: "toolpaths/vcarve_engrave_toolpath.svg",
            shortcut: "",
        }
    }

    fn run(doc: &Document, p: &Params) -> Result<Change, CommandError> {
        let tool = &p.tool;
        if tool.shape != ToolShape::VBit {
            return Err(CommandError::InvalidParams(
                "V-carving needs a V-bit".into(),
            ));
        }
        ensure_positive("tool diameter", tool.diameter)?;
        ensure_positive("pass depth", tool.pass_depth)?;
        if !(tool.stepover > 0.0 && tool.stepover <= 1.0) {
            return Err(CommandError::InvalidParams(
                "stepover must be more than 0 and at most 1".into(),
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
                "start depth cannot be negative".into(),
            ));
        }
        let mut moves = carve(&paths, p, doc.job.clearance);
        if p.project_onto_model {
            moves = project_onto_model(doc, tool, &moves)?;
        }
        let mut c = Change::builder(doc);
        c.add_toolpath(made_by::<Self>(p, p.name.clone(), tool.clone(), ids, moves));
        Ok(c.finish())
    }
}

/// The moves that V-carve the regions `paths` enclose (holes left standing) with `p`'s V-bit.
/// The edges are cut at the start depth; further in it goes deeper until the cone is as
/// wide as the bit or reaches the flat depth.
pub fn carve(paths: &[Path], p: &Params, safe: f64) -> Vec<Move> {
    let tool = &p.tool;
    // how far the cone reaches sideways for each millimetre down
    let spread = (tool.angle.to_radians() / 2.0).tan();
    // further in than this the cone cannot reach the edges, so the bottom is cut flat
    let reach = match p.flat_depth {
        limit if limit > 0.0 => (tool.diameter / 2.0).min(limit * spread),
        _ => tool.diameter / 2.0,
    };
    let step = tool.stepover * tool.diameter;
    let mut moves = vec![];
    for (offset, rings) in inward_offsets(&polygon::regions(paths), step) {
        let depths = pass_depths(p.start_depth + offset.min(reach) / spread, tool.pass_depth);
        for ring in &rings {
            moves.extend(contour_moves(&cut_points(ring), true, &depths, safe));
        }
    }
    moves
}

/// The regions shrunk by `step`, `2 step`, ... until nothing is left, plus about where the
/// last of them vanishes, so shapes narrower than a step are still carved down the middle.
// ponytail: concentric passes; a medial-axis path would carve the middle in one pass
fn inward_offsets(regions: &[Path], step: f64) -> Vec<(f64, Vec<Path>)> {
    let shrink = |by: f64| polygon::offset_regions(regions, -by, Corners::Round);
    let mut offsets = vec![];
    let mut offset = step;
    loop {
        let rings = shrink(offset);
        if rings.is_empty() {
            let (mut inside, mut gone) = (offset - step, offset);
            let mut last = None;
            for _ in 0..6 {
                let middle = f64::midpoint(inside, gone);
                let rings = shrink(middle);
                if rings.is_empty() {
                    gone = middle;
                } else {
                    inside = middle;
                    last = Some((middle, rings));
                }
            }
            offsets.extend(last);
            return offsets;
        }
        offsets.push((offset, rings));
        offset += step;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cut_craft_core::document::Job;
    use cut_craft_core::geometry::{Path, Point, inside};
    use cut_craft_core::simulation::Material;
    use cut_craft_core::testing::{made_toolpath, square};
    use cut_craft_core::toolpath::{MoveKind, Toolpath};

    cut_craft_core::contract_tests!(VcarveEngraveToolpath);

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
    fn a_90_degree_bit_carves_a_slot_as_deep_as_half_its_width() {
        let t = carve(vec![rectangle(100.0, 10.0)], &Params::default());
        let step = t.tool.stepover * t.tool.diameter;
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
        let step = t.tool.stepover * t.tool.diameter;
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
