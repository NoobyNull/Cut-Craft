//! V-Carve / Engrave Toolpath: a V-bit carves the selected shapes, going deeper the further
//! it is from their edges so its sides cut the edges exactly.

use aspire_core::document::Document;
use aspire_core::geometry::Path;
use aspire_core::polygon::{self, Corners};
use aspire_core::toolpath::{
    Tool, ToolDatabase, ToolShape, contour_moves, cut_points, made_by, pass_depths,
};
use aspire_core::{Change, Command, CommandError, Meta, ensure_positive};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub struct VcarveEngraveToolpath;

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
pub struct Params {
    pub name: String,
    /// Deepest the bit goes; wider parts are cut flat at this depth (0: as deep as it can)
    #[schemars(extend("x-unit" = "mm"), range(min = 0))]
    pub flat_depth: f64,
    /// A V-bit; its stepover sets how close together the carving passes are
    pub tool: Tool,
}

impl Default for Params {
    fn default() -> Self {
        Self {
            name: "V-Carve".into(),
            flat_depth: 0.0,
            tool: ToolDatabase::starter()
                .tools
                .into_iter()
                .find(|t| t.shape == ToolShape::VBit)
                .expect("the starter tools include a V-bit"),
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
        // how far the cone reaches sideways for each millimetre down
        let spread = (tool.angle.to_radians() / 2.0).tan();
        // further in than this the cone cannot reach the edges, so the bottom is cut flat
        let reach = match p.flat_depth {
            limit if limit > 0.0 => (tool.diameter / 2.0).min(limit * spread),
            _ => tool.diameter / 2.0,
        };
        let step = tool.stepover * tool.diameter;
        let safe = doc.job.clearance;
        let mut moves = vec![];
        for (offset, rings) in inward_offsets(&polygon::regions(&paths), step) {
            let depths = pass_depths(offset.min(reach) / spread, tool.pass_depth);
            for ring in &rings {
                moves.extend(contour_moves(&cut_points(ring), true, &depths, safe));
            }
        }
        let mut c = Change::builder(doc);
        c.add_toolpath(made_by::<Self>(p, p.name.clone(), tool.clone(), ids, moves));
        Ok(c.finish())
    }
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
    use aspire_core::document::Job;
    use aspire_core::geometry::{Path, Point, inside};
    use aspire_core::simulation::Material;
    use aspire_core::testing::{made_toolpath, square};
    use aspire_core::toolpath::{MoveKind, Toolpath};

    aspire_core::contract_tests!(VcarveEngraveToolpath);

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
