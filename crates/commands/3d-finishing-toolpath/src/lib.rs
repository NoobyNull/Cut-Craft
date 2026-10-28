//! 3D Finishing Toolpath: a ball nose rasters over the model, as low as it can go at each
//! point without cutting into it. The model's top sits at the material surface. With several
//! tools it makes a toolpath for each (rest machining): each later one cuts only what the
//! tools before it left.

use cut_craft_core::component::placed_model;
use cut_craft_core::document::Document;
use cut_craft_core::simulation::rest_areas;
use cut_craft_core::toolpath::{
    Tool, ToolDatabase, ToolShape, Toolpath, follow, made_by, zigzag_within,
};
use cut_craft_core::{Change, Command, CommandError, Meta, ensure_positive};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub struct FinishingToolpath3d;

/// How far a straight move may pass under where the tool should be before it is split.
const GOUGE: f64 = 0.002;

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
pub struct Params {
    pub name: String,
    /// In the order they run, a toolpath each: the first finishes all it can, each later one
    /// only what the tools before it left (rest machining). A ball nose follows curves best;
    /// its stepover spaces the raster lines
    #[serde(default)]
    pub tools: Vec<Tool>,
    /// Rest machining: leave alone unmachined areas smaller across than this
    #[schemars(extend("x-unit" = "mm"), range(min = 0))]
    pub minimum_detail: f64,
    /// Rest machining: grow every unmachined area by this before cutting it
    #[schemars(extend("x-unit" = "mm"), range(min = 0))]
    pub boundary_offset: f64,
    /// One toolpath of a set (and every toolpath saved before the tool list): its tool, which
    /// finishes after `after`
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(skip)]
    pub tool: Option<Tool>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[schemars(skip)]
    pub after: Vec<Tool>,
}

impl Default for Params {
    fn default() -> Self {
        Self {
            name: "3D Finishing".into(),
            tools: vec![
                ToolDatabase::starter()
                    .tools
                    .into_iter()
                    .find(|t| t.shape == ToolShape::BallNose)
                    .expect("the starter tools include a ball nose"),
            ],
            // ponytail: Aspire's defaults are not known; a millimetre each
            minimum_detail: 1.0,
            boundary_offset: 1.0,
            tool: None,
            after: vec![],
        }
    }
}

impl Params {
    /// A toolpath's own params for each tool: that tool after the ones before it, so each
    /// recalculates on its own. Several are named `[1]`, `[2]`... in running order.
    fn each_tool(&self) -> Vec<(Tool, Params)> {
        if let Some(tool) = &self.tool {
            return vec![(tool.clone(), self.clone())];
        }
        let several = self.tools.len() > 1;
        self.tools
            .iter()
            .enumerate()
            .map(|(i, tool)| {
                let one = Params {
                    name: if several {
                        format!("{} [{}]", self.name, i + 1)
                    } else {
                        self.name.clone()
                    },
                    tools: vec![],
                    tool: Some(tool.clone()),
                    after: self.tools[..i].to_vec(),
                    ..self.clone()
                };
                (tool.clone(), one)
            })
            .collect()
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
        let each = p.each_tool();
        if each.is_empty() {
            return Err(CommandError::InvalidParams(
                "choose at least one tool".into(),
            ));
        }
        let mut c = Change::builder(doc);
        // ponytail: each toolpath works out the rest areas of every tool before it again
        for (tool, own) in &each {
            c.add_toolpath(finish(doc, tool, own)?);
        }
        Ok(c.finish())
    }
}

/// The toolpath for `tool`, cutting only what `p.after` left.
fn finish(doc: &Document, tool: &Tool, p: &Params) -> Result<Toolpath, CommandError> {
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
    let inside = |at| rest.iter().all(|area| area(at));
    // at each point as low as the tool goes without touching the model
    let runs = zigzag_within(
        &surface,
        tool,
        tool.stepover * tool.diameter,
        GOUGE,
        f64::NEG_INFINITY,
        &inside,
    );
    let moves = runs
        .iter()
        .flat_map(|run| follow(run, doc.job.clearance))
        .collect();
    Ok(made_by::<FinishingToolpath3d>(
        p,
        p.name.clone(),
        tool.clone(),
        vec![],
        moves,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use cut_craft_core::component::model;
    use cut_craft_core::document::EntityId;
    use cut_craft_core::geometry::Point;
    use cut_craft_core::simulation::Material;
    use cut_craft_core::testing::domed;

    cut_craft_core::contract_tests!(FinishingToolpath3d, domed);

    fn finish(doc: &Document, params: &Params) -> Toolpath {
        finished(doc, params)
            .0
            .toolpaths
            .into_values()
            .next()
            .unwrap()
    }

    /// The document with the toolpaths `params` make, and their ids in order.
    fn finished(doc: &Document, params: &Params) -> (Document, Vec<EntityId>) {
        let mut doc = doc.clone();
        let change = FinishingToolpath3d::run(&doc, params).unwrap();
        doc.apply(&change).unwrap();
        let ids = doc.toolpaths.keys().copied().collect();
        (doc, ids)
    }

    fn ball(name: &str) -> Tool {
        ToolDatabase::starter().get(name).unwrap().clone()
    }

    #[test]
    fn it_follows_the_model_closely_without_cutting_into_it() {
        let doc = domed();
        let params = Params {
            tools: vec![
                ToolDatabase::starter()
                    .get("6 mm Ball Nose")
                    .unwrap()
                    .clone(),
            ],
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
            tools: vec![
                ToolDatabase::starter()
                    .get("6 mm Ball Nose")
                    .unwrap()
                    .clone(),
            ],
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
    fn several_tools_make_a_toolpath_each_in_the_order_they_run() {
        let params = Params {
            tools: vec![ball("6 mm Ball Nose"), ball("3 mm Ball Nose")],
            ..Params::default()
        };
        let (doc, ids) = finished(&domed(), &params);
        let made: Vec<&Toolpath> = ids.iter().map(|id| &doc.toolpaths[id]).collect();
        let names: Vec<&str> = made.iter().map(|t| t.name.as_str()).collect();
        assert_eq!(names, ["3D Finishing [1]", "3D Finishing [2]"]);
        assert_eq!(made[0].tool.name, "6 mm Ball Nose");
        assert_eq!(made[1].tool.name, "3 mm Ball Nose");
        // the first finishes everything, as it would on its own
        let alone = finish(
            &domed(),
            &Params {
                tools: vec![ball("6 mm Ball Nose")],
                ..Params::default()
            },
        );
        assert_eq!(made[0].moves, alone.moves);
        assert_eq!(alone.name, "3D Finishing", "one tool keeps the plain name");
    }

    #[test]
    fn each_toolpath_of_a_set_recalculates_on_its_own() {
        let params = Params {
            tools: vec![ball("6 mm Ball Nose"), ball("3 mm Ball Nose")],
            ..Params::default()
        };
        let (doc, ids) = finished(&domed(), &params);
        let mut registry = cut_craft_core::Registry::default();
        registry.add::<FinishingToolpath3d>();
        for id in ids {
            let change = registry.recalculate(&doc, id).unwrap();
            assert!(
                change.is_empty(),
                "{} recalculates the same",
                doc.toolpaths[&id].name
            );
        }
    }

    #[test]
    fn toolpaths_saved_with_one_tool_still_load_and_recalculate() {
        // a rest machining toolpath saved before the tool list
        let saved = serde_json::json!({
            "name": "Rest",
            "tool": ball("3 mm Ball Nose"),
            "after": [ball("6 mm Ball Nose")],
            "minimum_detail": 1.0,
            "boundary_offset": 1.0,
        });
        let old: Params = serde_json::from_value(saved).unwrap();
        let (doc, ids) = finished(&domed(), &old);
        assert_eq!(ids.len(), 1);
        let set = Params {
            tools: vec![ball("6 mm Ball Nose"), ball("3 mm Ball Nose")],
            ..Params::default()
        };
        let (set, set_ids) = finished(&domed(), &set);
        assert_eq!(doc.toolpaths[&ids[0]].name, "Rest");
        assert_eq!(
            doc.toolpaths[&ids[0]].moves, set.toolpaths[&set_ids[1]].moves,
            "the same as the second of a set"
        );
    }

    #[test]
    fn it_needs_a_tool() {
        let none = Params {
            tools: vec![],
            ..Params::default()
        };
        let why = FinishingToolpath3d::run(&domed(), &none)
            .err()
            .map(|e| e.to_string())
            .unwrap_or_default();
        assert!(why.contains("tool"), "{why}");
    }

    #[test]
    fn rest_machining_cuts_only_what_the_earlier_tools_left() {
        let params = Params {
            tools: vec![ball("6 mm Ball Nose"), ball("3 mm Ball Nose")],
            ..Params::default()
        };
        let (doc, ids) = finished(&domed(), &params);
        let toolpath = &doc.toolpaths[&ids[1]];
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
}
