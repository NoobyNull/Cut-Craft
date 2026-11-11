//! Material Setup (opened before the first toolpath): the material's
//! thickness, Z zero, XY datum, rapid Z gaps and home, as in Job Setup, and where the 3D model
//! sits in the material, by the gap above its top (Gap Outside Model on rotary jobs) or below
//! its bottom (Gap Inside Model, from the axis), with the model's thickness, which Set scales.
//! 3D toolpaths and the wrapped 3D view place the model by it.

use cmd_job_setup::{DatumPosition, JobSetup, RapidGaps};
use cmd_scale_model_z_height::{ScaleModelZHeight, model_height};
use cut_craft_core::document::{Document, Home, Job, ModelPosition, ZZero};
use cut_craft_core::{Change, Command, CommandError, Meta};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub struct MaterialSetup;

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
pub struct Params {
    /// The material's thickness (flat jobs; a rotary job's size is its diameter)
    #[schemars(
        title = "Material Thickness",
        extend("x-unit" = "mm", "x-section" = true),
        range(min = 0)
    )]
    pub thickness: f64,
    #[schemars(
        title = "Z Zero Position",
        extend("x-section" = true),
        extend("x-icon" = "job_setup/z_zero_{}.png")
    )]
    pub z_zero: ZZero,
    #[schemars(
        title = "XY Datum Position",
        extend("x-section" = true),
        extend("x-icon" = "job_setup/xy_datum_{position}.png")
    )]
    pub datum: DatumPosition,
    #[schemars(title = "Model Position in Material", extend("x-section" = true))]
    pub position: ModelPosition,
    /// The 3D model's height from its lowest to its highest point; a new value scales the
    /// model to it (Set). 0 when there is no model, or to leave it alone
    #[schemars(title = "Model Thickness", extend("x-unit" = "mm"), range(min = 0))]
    pub model_thickness: f64,
    #[schemars(
        title = "Rapid Z Gaps above Material",
        extend("x-section" = true),
        extend("x-icon" = "job_setup/clearance_z1.png")
    )]
    pub gaps: RapidGaps,
    /// Where the tool starts from and goes back to at the end
    #[schemars(title = "Home / Start Position", extend("x-section" = true))]
    pub home: Home,
}

impl Default for Params {
    fn default() -> Self {
        Self::of(&Document::default())
    }
}

impl Params {
    fn of(doc: &Document) -> Self {
        let job = cmd_job_setup::Params::from(&doc.job);
        Self {
            thickness: doc.job.thickness,
            z_zero: job.z_zero,
            datum: job.datum,
            position: doc.job.model_position,
            model_thickness: if doc.components.is_empty() {
                0.0
            } else {
                model_height(doc)
            },
            gaps: job.gaps,
            home: job.home,
        }
    }
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
        Params::of(doc)
    }

    fn run(doc: &Document, p: &Params) -> Result<Change, CommandError> {
        // the material, as Job Setup sets it (a new datum moves the coordinates, not the design)
        let mut job = cmd_job_setup::Params::from(&doc.job);
        if doc.job.rotary.is_none() {
            job.size.thickness = p.thickness;
        }
        job.z_zero = p.z_zero;
        job.datum = p.datum.clone();
        job.gaps = p.gaps.clone();
        job.home = p.home;
        let thickness = if doc.job.rotary.is_none() {
            p.thickness
        } else {
            doc.job.thickness
        };
        if p.position.gap < 0.0 || p.position.gap > thickness {
            return Err(CommandError::InvalidParams(format!(
                "set the gap between 0 and the material's {thickness:.1} mm"
            )));
        }
        let material = |d: &Document| JobSetup::run(d, &job);
        let position = |d: &Document| {
            let mut c = Change::builder(d);
            c.set_job(Job {
                model_position: p.position,
                ..d.job
            });
            Ok(c.finish())
        };
        let scale = |d: &Document| {
            let now = Params::of(d).model_thickness;
            // 0 leaves the model as it is
            if d.components.is_empty()
                || p.model_thickness <= 0.0
                || (p.model_thickness - now).abs() < 1e-9
            {
                Ok(Change::default())
            } else {
                let height = cmd_scale_model_z_height::Params {
                    height: p.model_thickness,
                };
                ScaleModelZHeight::run(d, &height)
            }
        };
        Change::chain(doc, &[&material, &position, &scale])
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
        let base = Params::of(&doc);
        run_on::<MaterialSetup>(
            &mut doc,
            &Params {
                position: ModelPosition {
                    from: GapFrom::Above,
                    gap: 1.5,
                },
                ..base.clone()
            },
        );
        assert!((placed_top(&doc) + 1.5).abs() < 0.05);
        let base = Params::of(&doc);
        run_on::<MaterialSetup>(
            &mut doc,
            &Params {
                position: ModelPosition {
                    from: GapFrom::Below,
                    gap: 0.0,
                },
                ..base.clone()
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
        let base = Params::of(&doc);
        run_on::<MaterialSetup>(
            &mut doc,
            &Params {
                position: ModelPosition {
                    from: GapFrom::Above,
                    gap: 5.0,
                },
                ..base.clone()
            },
        );
        assert!(placed_model(&doc, &Tool::default()).is_err());
        let too_big = Params {
            position: ModelPosition {
                from: GapFrom::Below,
                gap: 11.0,
            },
            ..Params::of(&doc)
        };
        assert!(MaterialSetup::run(&doc, &too_big).is_err());
    }

    #[test]
    fn it_sets_the_material_as_job_setup_does_and_scales_the_model_to_a_new_thickness() {
        let mut doc = domed();
        let p = Params::of(&doc);
        assert!(
            (p.model_thickness - 8.0).abs() < 0.05,
            "{}",
            p.model_thickness
        );
        run_on::<MaterialSetup>(
            &mut doc,
            &Params {
                thickness: 15.0,
                z_zero: ZZero::MachineBed,
                gaps: RapidGaps {
                    clearance: 12.0,
                    plunge: 3.0,
                },
                model_thickness: 4.0,
                ..p
            },
        );
        assert_eq!(doc.job.thickness, 15.0);
        assert_eq!(doc.job.z_zero, ZZero::MachineBed);
        assert_eq!((doc.job.clearance, doc.job.plunge_gap()), (12.0, 3.0));
        let after = Params::of(&doc);
        assert!(
            (after.model_thickness - 4.0).abs() < 0.05,
            "{}",
            after.model_thickness
        );
    }
}
