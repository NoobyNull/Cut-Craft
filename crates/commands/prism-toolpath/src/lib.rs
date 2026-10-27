//! Prism Carving Toolpath: a V-bit runs along the edges of the selected shapes deep enough
//! that its sloping sides leave each shape standing as a raised, ridged bevel.

use cut_craft_core::document::Document;
use cut_craft_core::geometry::{Path, winding};
use cut_craft_core::polygon::{self, Corners};
use cut_craft_core::toolpath::{
    Tool, ToolDatabase, ToolShape, contour_moves, cut_points, made_by, pass_depths,
};
use cut_craft_core::{Change, Command, CommandError, Meta, ensure_positive};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub struct PrismToolpath;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum CutDirection {
    /// Clockwise round the shapes
    #[default]
    Climb,
    /// Anticlockwise round the shapes
    Conventional,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
pub struct Params {
    pub name: String,
    /// Below the material surface where the prism's top is (the floor of an earlier pocket)
    #[schemars(extend("x-unit" = "mm"), range(min = 0))]
    pub start_depth: f64,
    /// Of the prism's base below the start depth; too shallow and the top is cut flat
    #[schemars(extend("x-unit" = "mm"), range(min = 0))]
    pub prism_depth: f64,
    /// Instead, use the least depth that brings the widest shape to a point
    pub depth_for_full_prism: bool,
    pub cut_direction: CutDirection,
    /// A V-bit
    pub tool: Tool,
}

impl Default for Params {
    fn default() -> Self {
        Self {
            name: "Prism".into(),
            start_depth: 0.0,
            prism_depth: 5.0,
            depth_for_full_prism: false,
            cut_direction: CutDirection::Climb,
            tool: ToolDatabase::starter()
                .tools
                .into_iter()
                .find(|t| t.shape == ToolShape::VBit)
                .expect("the starter tools include a V-bit"),
        }
    }
}

/// Radius of the largest circle inside the regions `paths` enclose (half the widest point),
/// to about a thousandth of the shapes' size.
pub fn widest_half(paths: &[Path]) -> f64 {
    let regions = polygon::regions(paths);
    let Some(b) = paths
        .iter()
        .map(Path::bounds)
        .reduce(cut_craft_core::geometry::Rect::union)
    else {
        return 0.0;
    };
    let size = b.max - b.min;
    let (mut inside, mut gone) = (0.0, size.x.min(size.y) / 2.0 + 1e-9);
    for _ in 0..12 {
        let middle = f64::midpoint(inside, gone);
        if polygon::offset_regions(&regions, -middle, Corners::Round).is_empty() {
            gone = middle;
        } else {
            inside = middle;
        }
    }
    inside
}

impl Command for PrismToolpath {
    const ID: &'static str = "prism_toolpath";
    type Params = Params;

    fn meta() -> Meta {
        Meta {
            label: "Prism Carving Toolpath",
            group: "Toolpath Operations",
            icon: "toolpaths/prism_carving.png",
            shortcut: "",
        }
    }

    fn run(doc: &Document, p: &Params) -> Result<Change, CommandError> {
        let tool = &p.tool;
        if tool.shape != ToolShape::VBit {
            return Err(CommandError::InvalidParams(
                "prism carving needs a V-bit".into(),
            ));
        }
        ensure_positive("tool diameter", tool.diameter)?;
        ensure_positive("pass depth", tool.pass_depth)?;
        if p.start_depth < 0.0 {
            return Err(CommandError::InvalidParams(
                "start depth cannot be negative".into(),
            ));
        }
        let (ids, paths) = doc.selected_paths(|path| path.closed);
        if ids.is_empty() {
            return Err(CommandError::Failed(
                "select the closed vectors to prism carve".into(),
            ));
        }
        let spread = (tool.angle.to_radians() / 2.0).tan();
        let depth = if p.depth_for_full_prism {
            let half = widest_half(&paths);
            if half > tool.diameter / 2.0 {
                return Err(CommandError::Failed(format!(
                    "the V-bit is too narrow for a full prism: the widest shape needs {:.2} mm across",
                    2.0 * half
                )));
            }
            half / spread
        } else {
            ensure_positive("prism depth", p.prism_depth)?;
            p.prism_depth
        };
        let depths = pass_depths(p.start_depth + depth, tool.pass_depth);
        let safe = doc.job.clearance;
        let moves = paths
            .iter()
            .flat_map(|path| {
                let mut points = cut_points(path);
                let clockwise = winding(&points) < 0.0;
                if clockwise != (p.cut_direction == CutDirection::Climb) {
                    points.reverse();
                }
                contour_moves(&points, true, &depths, safe)
            })
            .collect();
        let mut c = Change::builder(doc);
        c.add_toolpath(made_by::<Self>(p, p.name.clone(), tool.clone(), ids, moves));
        Ok(c.finish())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cut_craft_core::document::Job;
    use cut_craft_core::geometry::Point;
    use cut_craft_core::simulation::Material;
    use cut_craft_core::testing::made_toolpath;
    use cut_craft_core::toolpath::{MoveKind, Toolpath};

    cut_craft_core::contract_tests!(PrismToolpath);

    /// A strip 10 mm wide and 80 long.
    fn strip() -> Path {
        Path::polyline(
            &[
                Point::new(10.0, 10.0),
                Point::new(90.0, 10.0),
                Point::new(90.0, 20.0),
                Point::new(10.0, 20.0),
            ],
            true,
        )
    }

    fn prism(p: &Params) -> Toolpath {
        made_toolpath::<PrismToolpath>(vec![strip()], p).unwrap()
    }

    fn full() -> Params {
        Params {
            depth_for_full_prism: true,
            ..Params::default()
        }
    }

    fn deepest(t: &Toolpath) -> f64 {
        t.moves.iter().map(|m| m.to.z).fold(f64::INFINITY, f64::min)
    }

    #[test]
    fn full_prism_depth_is_half_the_widest_point_over_the_slope() {
        // a 90 degree bit slopes 1:1, so a 10 mm wide strip needs 5 mm
        let t = prism(&full());
        assert!((deepest(&t) + 5.0).abs() < 0.01, "{}", deepest(&t));
    }

    #[test]
    fn leaves_a_ridge_down_the_middle_sloping_to_the_edges() {
        let t = prism(&full());
        let job = Job {
            width: 100.0,
            height: 30.0,
            ..Job::default()
        };
        let mut m = Material::new(&job, 400);
        m.cut(&t);
        let z = |y| m.at(Point::new(50.0, y)).unwrap();
        assert!(z(15.0) > -0.3, "the ridge is at the surface: {}", z(15.0));
        assert!(
            (z(12.0) + 3.0).abs() < 0.4,
            "2 mm in, 3 mm down: {}",
            z(12.0)
        );
        assert!(z(10.0) < -4.5, "the base at the edge: {}", z(10.0));
    }

    #[test]
    fn a_set_depth_and_start_depth_are_used_as_given() {
        let p = Params {
            depth_for_full_prism: false,
            prism_depth: 2.0,
            start_depth: 1.0,
            ..Params::default()
        };
        assert!((deepest(&prism(&p)) + 3.0).abs() < 1e-9);
    }

    #[test]
    fn climb_runs_clockwise_and_conventional_anticlockwise() {
        let turn = |d| {
            let t = prism(&Params {
                cut_direction: d,
                ..Params::default()
            });
            let ring: Vec<Point> = t
                .moves
                .iter()
                .skip_while(|m| m.kind != MoveKind::Plunge)
                .take_while(|m| m.kind != MoveKind::Plunge || m.to.z > -10.0)
                .filter(|m| m.kind == MoveKind::Cut)
                .take(4)
                .map(|m| m.to.truncate())
                .collect();
            winding(&ring)
        };
        assert!(turn(CutDirection::Climb) < 0.0);
        assert!(turn(CutDirection::Conventional) > 0.0);
    }

    #[test]
    fn a_full_prism_wider_than_the_bit_is_refused() {
        let wide = cut_craft_core::testing::square(0.0, 0.0, 40.0);
        let why = made_toolpath::<PrismToolpath>(vec![wide], &full()).unwrap_err();
        assert!(why.contains("too narrow"), "{why}");
    }
}
