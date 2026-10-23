//! 3D Finishing Toolpath: a ball nose rasters over the model, as low as it can go at each
//! point without cutting into it. The model's top sits at the material surface.

use cut_craft_core::component::placed_model;
use cut_craft_core::document::Document;
use cut_craft_core::toolpath::{Tool, ToolDatabase, ToolShape, follow, made_by, zigzag};
use cut_craft_core::{Change, Command, CommandError, Meta, ensure_positive};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub struct FinishingToolpath3d;

/// How far a straight move may pass under where the tool should be before it is split.
const GOUGE: f64 = 0.002;

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
pub struct Params {
    pub name: String,
    /// A ball nose follows curves best; its stepover spaces the raster lines
    pub tool: Tool,
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
            icon: "toolpaths/3d_finishing_toolpath.svg",
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
        // at each point as low as the tool goes without touching the model
        let points = zigzag(
            &surface,
            tool,
            tool.stepover * tool.diameter,
            GOUGE,
            f64::NEG_INFINITY,
        );
        let moves = follow(&points, doc.job.clearance);
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
}
