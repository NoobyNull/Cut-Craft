//! 3D Finishing Toolpath: a ball nose rasters over the model, as low as it can go at each
//! point without cutting into it. The model's top sits at the material surface. With earlier
//! tools named (rest machining) it cuts only what they left.

use cut_craft_core::component::{MachiningBoundary, machining_boundary, placed_model};
use cut_craft_core::document::Document;
use cut_craft_core::geometry::Point;
use cut_craft_core::polygon::{self, Corners};
use cut_craft_core::simulation::rest_areas;
use cut_craft_core::toolpath::{
    Drop, Tool, ToolDatabase, ToolShape, cut_points, follow, made_by, raster_lines,
};
use cut_craft_core::{Change, Command, CommandError, Meta, ensure_positive};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub struct FinishingToolpath3d;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Strategy {
    /// Straight lines across, at the raster angle
    #[default]
    Raster,
    /// Rings stepping in from the machining boundary
    Offset,
}

/// How far a straight move may pass under where the tool should be before it is split.
const GOUGE: f64 = 0.002;

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
pub struct Params {
    pub name: String,
    /// A ball nose follows curves best; its stepover spaces the raster lines
    pub tool: Tool,
    /// Rest machining: the tools that finished before this one, in order; only what they
    /// could not reach is cut
    #[schemars(title = "Rest Machining After")]
    pub after: Vec<Tool>,
    /// Rest machining: leave alone unmachined areas smaller across than this
    #[schemars(extend("x-unit" = "mm"), range(min = 0))]
    pub minimum_detail: f64,
    /// Rest machining: grow every unmachined area by this before cutting it
    #[schemars(extend("x-unit" = "mm"), range(min = 0))]
    pub boundary_offset: f64,
    /// Where the tool's tip may go
    #[serde(default)]
    #[schemars(title = "Machining Limit Boundary")]
    pub machining_boundary: MachiningBoundary,
    /// Grows the machining boundary, so the tool can go on past a raised edge
    #[serde(default)]
    #[schemars(title = "Boundary Offset", extend("x-unit" = "mm"))]
    pub machining_boundary_offset: f64,
    #[serde(default)]
    pub strategy: Strategy,
    /// 0° along X, 90° along Y
    #[serde(default)]
    #[schemars(
        extend("x-unit" = "°", "x-show-when" = {"strategy": "raster"}),
        range(min = 0, max = 90)
    )]
    pub raster_angle: f64,
}

impl Default for Params {
    fn default() -> Self {
        Self {
            name: "3D Finishing".into(),
            tool: ToolDatabase::starter()
                .tools
                .into_iter()
                .find(|t| t.shape == ToolShape::BallNose)
                .expect("the starter tools include a ball nose"),
            after: vec![],
            // ponytail: Aspire's defaults are not known; a millimetre each
            minimum_detail: 1.0,
            boundary_offset: 1.0,
            machining_boundary: MachiningBoundary::Material,
            machining_boundary_offset: 0.0,
            strategy: Strategy::Raster,
            raster_angle: 0.0,
        }
    }
}

impl Command for FinishingToolpath3d {
    const ID: &'static str = "3d_finishing_toolpath";
    type Params = Params;

    fn meta() -> Meta {
        Meta {
            label: "3D Finishing Toolpath",
            group: "3D Toolpaths",
            icon: "toolpaths/3d_finishing_toolpath.png",
            shortcut: "",
        }
    }

    fn run(doc: &Document, p: &Params) -> Result<Change, CommandError> {
        let tool = &p.tool;
        ensure_positive("tool diameter", tool.diameter)?;
        if !(tool.stepover > 0.0 && tool.stepover <= 1.0) {
            return Err(CommandError::InvalidParams(
                "stepover must be more than 0 and at most 1".into(),
            ));
        }
        let surface = placed_model(doc, tool)?;
        // where every earlier tool left material, each worked out on its own grid
        let mut rest = vec![];
        for earlier in &p.after {
            ensure_positive("earlier tool diameter", earlier.diameter)?;
            let model = placed_model(doc, earlier)?;
            let left = model.left_by(earlier);
            rest.push(rest_areas(
                &model,
                &left,
                p.minimum_detail,
                p.boundary_offset,
            ));
        }
        let material = doc.job.area();
        let inside = |at: Point| material.contains(at) && rest.iter().all(|area| area(at));
        let regions = machining_boundary(doc, p.machining_boundary, p.machining_boundary_offset)?;
        let step = tool.stepover * tool.diameter;
        let lines: Vec<Vec<Point>> = match p.strategy {
            Strategy::Raster => raster_lines(&regions, step, p.raster_angle),
            Strategy::Offset => {
                let mut lines = vec![];
                let mut rings = regions;
                while !rings.is_empty() {
                    lines.extend(rings.iter().map(|r| {
                        let mut points = cut_points(r);
                        points.push(points[0]);
                        points
                    }));
                    rings = polygon::offset_regions(&rings, -step, Corners::Round);
                }
                lines
            }
        };
        // at each point as low as the tool goes without touching the model
        let peaks = surface.peaks();
        let drop = Drop {
            grid: &surface,
            tool,
            tolerance: GOUGE,
            floor: f64::NEG_INFINITY,
            ceiling: f64::INFINITY,
            peaks: &peaks,
        };
        let runs: Vec<_> = lines.iter().flat_map(|l| drop.along(l, &inside)).collect();
        let moves = runs
            .iter()
            .flat_map(|run| follow(run, doc.job.clearance))
            .collect();
        let mut c = Change::builder(doc);
        c.add_toolpath(made_by::<Self>(
            p,
            p.name.clone(),
            tool.clone(),
            vec![],
            moves,
        ));
        Ok(c.finish())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cut_craft_core::component::model;
    use cut_craft_core::geometry::Point;
    use cut_craft_core::simulation::Material;
    use cut_craft_core::testing::domed;
    use cut_craft_core::toolpath::Toolpath;

    cut_craft_core::contract_tests!(FinishingToolpath3d, domed);

    fn finish(doc: &Document, params: &Params) -> Toolpath {
        let mut doc = doc.clone();
        let change = FinishingToolpath3d::run(&doc, params).unwrap();
        doc.apply(&change).unwrap();
        doc.toolpaths.into_values().next().unwrap()
    }

    #[test]
    fn it_follows_the_model_closely_without_cutting_into_it() {
        let doc = domed();
        let params = Params {
            tool: ToolDatabase::starter()
                .get("6 mm Ball Nose")
                .unwrap()
                .clone(),
            ..Params::default()
        };
        let toolpath = finish(&doc, &params);
        // the grid the toolpath was worked out on: cells a twentieth of the tool
        let cells = 134;
        let mut cut = Material::new(&doc.job, cells);
        cut.cut(&toolpath);
        let shape = model(&doc, cells);
        let top = shape
            .heights
            .iter()
            .copied()
            .fold(f64::NEG_INFINITY, f64::max);
        let gouges = cut
            .heights
            .iter()
            .zip(&shape.heights)
            .filter(|&(&z, &h)| z < h - top - 0.01)
            .count();
        assert_eq!(gouges, 0, "cells cut more than 0.01 mm below the model");
        // the top, halfway down the side, and the flat around it are cut close
        for (x, y) in [(20.0, 20.0), (25.0, 20.0), (3.0, 3.0)] {
            let at = Point::new(x, y);
            let (z, model_z) = (cut.at(at).unwrap(), shape.at(at).unwrap() - top);
            assert!(
                z <= model_z + 0.05,
                "({x}, {y}) left at {z}, model at {model_z}"
            );
        }
    }

    #[test]
    fn vertical_walls_are_not_gouged() {
        use cut_craft_core::component::{Combine, Component};
        use cut_craft_core::geometry::Rect;
        let mut doc = domed();
        doc.components.clear();
        let area = Rect::new(Point::new(15.0, 15.0), Point::new(25.0, 25.0));
        let block = Component::from_fn("block", Combine::Add, area, 0.2, |_| Some(6.0));
        let mut c = cut_craft_core::Change::builder(&doc);
        c.add_component(block);
        let change = c.finish();
        doc.apply(&change).unwrap();
        let params = Params {
            tool: ToolDatabase::starter()
                .get("6 mm Ball Nose")
                .unwrap()
                .clone(),
            ..Params::default()
        };
        let toolpath = finish(&doc, &params);
        let cells = 134;
        let mut cut = Material::new(&doc.job, cells);
        cut.cut(&toolpath);
        let shape = model(&doc, cells);
        let gouges = cut
            .heights
            .iter()
            .zip(&shape.heights)
            .filter(|&(&z, &h)| z < h - 6.0 - 0.01)
            .count();
        assert_eq!(gouges, 0, "cells cut more than 0.01 mm below the model");
    }

    #[test]
    fn a_tapered_ball_nose_keeps_its_taper_off_a_tall_wall() {
        use cut_craft_core::component::{Combine, Component};
        use cut_craft_core::geometry::Rect;
        let mut doc = domed();
        doc.components.clear();
        // a block 15 mm tall: a 1 mm ball fits beside it, its 8° taper leans into it
        let area = Rect::new(Point::new(15.0, 15.0), Point::new(25.0, 25.0));
        let block = Component::from_fn("block", Combine::Add, area, 0.2, |_| Some(15.0));
        let mut c = cut_craft_core::Change::builder(&doc);
        c.add_component(block);
        doc.apply(&c.finish()).unwrap();
        doc.job.thickness = 20.0;
        let tool = Tool {
            name: "tapered".into(),
            shape: ToolShape::BallNose,
            diameter: 1.0,
            taper: 8.0,
            shank_diameter: 6.0,
            stepover: 0.5,
            ..Tool::default()
        };
        let params = Params {
            tool: tool.clone(),
            ..Params::default()
        };
        let toolpath = finish(&doc, &params);
        let cells = 400;
        let mut cut = Material::new(&doc.job, cells);
        cut.cut(&toolpath);
        let shape = model(&doc, cells);
        let gouges = cut
            .heights
            .iter()
            .zip(&shape.heights)
            .filter(|&(&z, &h)| z < h - 15.0 - 0.02)
            .count();
        assert_eq!(
            gouges, 0,
            "cells the taper cut more than 0.02 mm into the model"
        );
        // at the foot of the wall the taper holds the tip up: material is left there
        let foot = Point::new(25.6, 20.0);
        assert!(
            cut.at(foot).unwrap() > -15.0 + 1.0,
            "{}",
            cut.at(foot).unwrap()
        );
    }

    #[test]
    fn rest_machining_cuts_only_what_the_earlier_tools_left() {
        let starter = ToolDatabase::starter();
        let ball = |name: &str| starter.get(name).unwrap().clone();
        let params = Params {
            tool: ball("3 mm Ball Nose"),
            after: vec![ball("6 mm Ball Nose")],
            ..Params::default()
        };
        let toolpath = finish(&domed(), &params);
        // a 6 mm ball rounds the inside corner where the 8 mm dome meets the flat: a ring
        // from about 7.7 to 10.6 mm out, grown by the boundary offset
        let cuts: Vec<f64> = toolpath
            .moves
            .iter()
            .filter(|m| m.to.z < 0.0)
            .map(|m| m.to.truncate().distance(Point::new(20.0, 20.0)))
            .collect();
        assert!(!cuts.is_empty(), "the 3 mm ball has something to do");
        assert!(
            cuts.iter().all(|&r| (6.0..12.5).contains(&r)),
            "only round the foot of the dome: {:?}",
            cuts.iter()
                .copied()
                .fold((f64::MAX, 0.0_f64), |(lo, hi), r| (lo.min(r), hi.max(r)))
        );
    }

    #[test]
    fn the_model_must_fit_in_the_material() {
        let mut doc = domed();
        doc.job.thickness = 5.0;
        let why = FinishingToolpath3d::run(&doc, &Params::default())
            .err()
            .map(|e| e.to_string())
            .unwrap_or_default();
        assert!(
            why.contains("8.0 mm tall") && why.contains("5.0 mm thick"),
            "{why}"
        );
    }

    #[test]
    fn it_needs_a_model() {
        let why = FinishingToolpath3d::run(&Document::default(), &Params::default())
            .err()
            .map(|e| e.to_string())
            .unwrap_or_default();
        assert!(why.contains("3D model"), "{why}");
    }

    fn cut_by(params: &Params) -> Material {
        let doc = domed();
        let mut cut = Material::new(&doc.job, 134);
        cut.cut(&finish(&doc, params));
        cut
    }

    #[test]
    fn the_model_boundary_keeps_the_tip_on_the_model() {
        let cut = cut_by(&Params {
            machining_boundary: MachiningBoundary::Model,
            ..Params::default()
        });
        // the dome is 8 mm across its foot round (20, 20): the corner is left alone
        assert_eq!(cut.at(Point::new(3.0, 3.0)), Some(0.0));
        let grown = cut_by(&Params {
            machining_boundary: MachiningBoundary::Model,
            machining_boundary_offset: 4.0,
            ..Params::default()
        });
        let foot = Point::new(20.0 + 10.0, 20.0);
        assert_eq!(cut.at(foot), Some(0.0), "outside the model");
        assert!(
            grown.at(foot).unwrap() < 0.0,
            "the offset reaches past the foot"
        );
    }

    #[test]
    fn offset_rings_and_angled_rasters_follow_the_model_too() {
        for params in [
            Params {
                strategy: Strategy::Offset,
                ..Params::default()
            },
            Params {
                raster_angle: 45.0,
                ..Params::default()
            },
        ] {
            let doc = domed();
            let toolpath = finish(&doc, &params);
            let cells = 134;
            let mut cut = Material::new(&doc.job, cells);
            cut.cut(&toolpath);
            let shape = model(&doc, cells);
            let top = shape
                .heights
                .iter()
                .copied()
                .fold(f64::NEG_INFINITY, f64::max);
            let gouges = cut
                .heights
                .iter()
                .zip(&shape.heights)
                .filter(|&(&z, &h)| z < h - top - 0.01)
                .count();
            assert_eq!(gouges, 0, "{:?}", params.strategy);
            let flat = cut.at(Point::new(3.0, 3.0)).unwrap();
            assert!(flat < -7.9, "{:?}: {flat}", params.strategy);
        }
    }
}
