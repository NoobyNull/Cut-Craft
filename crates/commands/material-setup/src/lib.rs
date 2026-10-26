//! Material Setup: where the 3D model sits in the material, by the gap above its top (Gap
//! Outside Model on rotary jobs) or below its bottom (Gap Inside Model, from the axis). 3D
//! toolpaths and the wrapped 3D view place the model by it.

use cut_craft_core::document::{Document, Job, ModelPosition};
use cut_craft_core::{Change, Command, CommandError, Meta};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub struct MaterialSetup;

#[derive(Clone, Debug, Default, Serialize, Deserialize, JsonSchema)]
pub struct Params {
    #[schemars(title = "Model Position in Material", extend("x-section" = true))]
    pub position: ModelPosition,
}

impl Command for MaterialSetup {
    const ID: &'static str = "material_setup";
    type Params = Params;

    fn meta() -> Meta {
        Meta {
            label: "Material Setup",
            group: "Toolpaths",
            icon: "",
            shortcut: "",
        }
    }

    fn initial_params(doc: &Document) -> Params {
        Params {
            position: doc.job.model_position,
        }
    }

    fn run(doc: &Document, p: &Params) -> Result<Change, CommandError> {
        if p.position.gap < 0.0 || p.position.gap > doc.job.thickness {
            return Err(CommandError::InvalidParams(format!(
                "the gap must be between 0 and the material's {:.1} mm",
                doc.job.thickness
            )));
        }
        let mut c = Change::builder(doc);
        c.set_job(Job {
            model_position: p.position,
            ..doc.job
        });
        Ok(c.finish())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cut_craft_core::component::placed_model;
    use cut_craft_core::document::GapFrom;
    use cut_craft_core::geometry::Point;
    use cut_craft_core::testing::{domed, run_on};
    use cut_craft_core::toolpath::Tool;

    cut_craft_core::contract_tests!(MaterialSetup, domed);

    fn placed_top(doc: &Document) -> f64 {
        let tool = Tool::default();
        let m = placed_model(doc, &tool).unwrap();
        m.at(Point::new(20.0, 20.0)).unwrap()
    }

    #[test]
    fn a_gap_above_lowers_the_model_and_a_gap_below_sets_its_bottom() {
        // the dome is 8 tall in 10 mm material
        let mut doc = domed();
        assert!(
            placed_top(&doc).abs() < 0.05,
            "top at the surface by default"
        );
        run_on::<MaterialSetup>(
            &mut doc,
            &Params {
                position: ModelPosition {
                    from: GapFrom::Above,
                    gap: 1.5,
                },
            },
        );
        assert!((placed_top(&doc) + 1.5).abs() < 0.05);
        run_on::<MaterialSetup>(
            &mut doc,
            &Params {
                position: ModelPosition {
                    from: GapFrom::Below,
                    gap: 0.0,
                },
            },
        );
        // the base plane on the material's bottom: the 8 mm top is 2 below the surface
        assert!(
            (placed_top(&doc) + 2.0).abs() < 0.05,
            "{}",
            placed_top(&doc)
        );
    }

    #[test]
    fn a_gap_that_pushes_the_model_out_of_the_material_is_refused_at_machining() {
        let mut doc = domed();
        run_on::<MaterialSetup>(
            &mut doc,
            &Params {
                position: ModelPosition {
                    from: GapFrom::Above,
                    gap: 5.0,
                },
            },
        );
        assert!(placed_model(&doc, &Tool::default()).is_err());
        let too_big = Params {
            position: ModelPosition {
                from: GapFrom::Below,
                gap: 11.0,
            },
        };
        assert!(MaterialSetup::run(&doc, &too_big).is_err());
    }
}
