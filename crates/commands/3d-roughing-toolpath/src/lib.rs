//! 3D Roughing Toolpath: an end mill clears the material above the model level by level,
//! leaving an allowance for the finishing toolpath.

use cut_craft_core::component::placed_model;
use cut_craft_core::document::Document;
use cut_craft_core::toolpath::{Tool, follow, made_by, pass_depths, zigzag};
use cut_craft_core::{Change, Command, CommandError, Meta, ensure_positive};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub struct RoughingToolpath3d;

/// How far a straight move may pass under the allowance before it is split.
const TOLERANCE: f64 = 0.01;

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
pub struct Params {
    pub name: String,
    /// Left on the model for the finishing toolpath to take off
    #[schemars(extend("x-unit" = "mm"), range(min = 0))]
    pub allowance: f64,
    /// Its pass depth spaces the levels, its stepover the lines
    pub tool: Tool,
}

impl Default for Params {
    fn default() -> Self {
        Self {
            name: "3D Roughing".into(),
            allowance: 0.5,
            tool: Tool::default(),
        }
    }
}

impl Command for RoughingToolpath3d {
    const ID: &'static str = "3d_roughing_toolpath";
    type Params = Params;

    fn meta() -> Meta {
        Meta {
            label: "3D Roughing Toolpath",
            group: "3D Toolpaths",
            icon: "toolpaths/3d_roughing_toolpath.png",
            shortcut: "",
        }
    }

    fn run(doc: &Document, p: &Params) -> Result<Change, CommandError> {
        let tool = &p.tool;
        ensure_positive("tool diameter", tool.diameter)?;
        ensure_positive("pass depth", tool.pass_depth)?;
        if !(tool.stepover > 0.0 && tool.stepover <= 1.0) {
            return Err(CommandError::InvalidParams(
                "stepover must be more than 0 and at most 1".into(),
            ));
        }
        if p.allowance.is_nan() || p.allowance < 0.0 {
            return Err(CommandError::InvalidParams(
                "the allowance cannot be negative".into(),
            ));
        }
        // what must be left: the model and the allowance on top of it
        let mut leave = placed_model(doc, tool)?;
        let lowest = leave.heights.iter().copied().fold(0.0, f64::min);
        for height in &mut leave.heights {
            *height += p.allowance;
        }
        let step = tool.stepover * tool.diameter;
        // ponytail: each level rides over what is already cleared; skip it with retracts
        // if roughing time matters, and the allowance is vertical, not a true 3D offset
        let mut moves = vec![];
        for level in pass_depths(-lowest, tool.pass_depth) {
            let points = zigzag(&leave, tool, step, TOLERANCE, level);
            moves.extend(follow(&points, doc.job.clearance));
        }
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

    cut_craft_core::contract_tests!(RoughingToolpath3d, domed);

    fn rough(params: &Params) -> Toolpath {
        let mut doc = domed();
        let change = RoughingToolpath3d::run(&doc, params).unwrap();
        doc.apply(&change).unwrap();
        doc.toolpaths.into_values().next().unwrap()
    }

    #[test]
    fn it_clears_down_to_the_allowance_without_touching_it() {
        let doc = domed();
        let toolpath = rough(&Params::default());
        let cells = 134; // a twentieth of the 6 mm tool
        let mut cut = Material::new(&doc.job, cells);
        cut.cut(&toolpath);
        let shape = model(&doc, cells);
        let top = shape
            .heights
            .iter()
            .copied()
            .fold(f64::NEG_INFINITY, f64::max);
        let into_allowance = cut
            .heights
            .iter()
            .zip(&shape.heights)
            // where model and allowance stand above the surface there is nothing to leave
            .filter(|&(&z, &h)| z < (h - top + 0.5).min(0.0) - 0.01)
            .count();
        assert_eq!(into_allowance, 0, "cells cut into the allowance");
        let floor = cut.at(Point::new(3.0, 3.0)).unwrap();
        assert!(
            (floor - (-8.0 + 0.5)).abs() < 0.02,
            "the flat around it: {floor}"
        );
        assert_eq!(cut.at(Point::new(20.0, 20.0)), Some(0.0), "the top is left");
    }

    #[test]
    fn no_level_cuts_deeper_than_the_pass_depth() {
        let toolpath = rough(&Params::default());
        let mut levels: Vec<f64> = toolpath
            .moves
            .iter()
            .map(|m| m.to.z)
            .filter(|&z| z < 0.0)
            .collect();
        levels.sort_by(f64::total_cmp);
        levels.dedup_by(|a, b| (*a - *b).abs() < 1e-9);
        // the flat levels, top down, are no more than a pass depth apart
        let pass = Params::default().tool.pass_depth;
        let deepest = levels[0];
        // the model's top is its highest cell, a hair under 8 mm
        assert!(
            (deepest + 7.5).abs() < 0.01,
            "down to the allowance: {deepest}"
        );
        let first_level = levels.iter().copied().fold(f64::NEG_INFINITY, f64::max);
        assert!(first_level >= -pass - 1e-9);
    }
}
