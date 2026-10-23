//! Job Setup: the material block's size, thickness and display units.

use aspire_core::document::{Document, Job, Units};
use aspire_core::{Change, Command, CommandError, Meta, ensure_positive};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub struct JobSetup;

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
pub struct Params {
    /// Along X
    #[schemars(extend("x-unit" = "mm"), range(min = 0))]
    pub width: f64,
    /// Along Y
    #[schemars(extend("x-unit" = "mm"), range(min = 0))]
    pub height: f64,
    /// Along Z
    #[schemars(extend("x-unit" = "mm"), range(min = 0))]
    pub thickness: f64,
    /// Units shown in the interface; the job is always stored in millimetres
    pub units: Units,
    /// Safe height above the material for rapid moves
    #[schemars(extend("x-unit" = "mm"), range(min = 0))]
    pub clearance: f64,
}

impl Default for Params {
    fn default() -> Self {
        let job = Job::default();
        Self {
            width: job.width,
            height: job.height,
            thickness: job.thickness,
            units: job.units,
            clearance: job.clearance,
        }
    }
}

impl Command for JobSetup {
    const ID: &'static str = "job_setup";
    type Params = Params;

    fn meta() -> Meta {
        Meta {
            label: "Job Setup",
            group: "File Operations",
            icon: "file/job_setup.svg",
            shortcut: "",
        }
    }

    fn initial_params(doc: &Document) -> Params {
        let job = doc.job;
        Params {
            width: job.width,
            height: job.height,
            thickness: job.thickness,
            units: job.units,
            clearance: job.clearance,
        }
    }

    fn run(doc: &Document, p: &Params) -> Result<Change, CommandError> {
        ensure_positive("width", p.width)?;
        ensure_positive("height", p.height)?;
        ensure_positive("thickness", p.thickness)?;
        ensure_positive("clearance", p.clearance)?;
        let mut c = Change::builder(doc);
        c.set_job(Job {
            width: p.width,
            height: p.height,
            thickness: p.thickness,
            units: p.units,
            clearance: p.clearance,
        });
        Ok(c.finish())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use aspire_core::testing::{after, after_err, sample_document};

    aspire_core::contract_tests!(JobSetup);

    #[test]
    fn sets_the_material_block() {
        let doc = after::<JobSetup>(&Params {
            width: 1000.0,
            thickness: 12.0,
            units: Units::Inch,
            ..Params::default()
        });
        assert_eq!(
            (
                doc.job.width,
                doc.job.height,
                doc.job.thickness,
                doc.job.units
            ),
            (1000.0, 400.0, 12.0, Units::Inch)
        );
    }

    #[test]
    fn the_form_starts_from_the_current_job() {
        let mut doc = sample_document();
        let mut c = Change::builder(&doc);
        c.set_job(Job {
            width: 123.0,
            ..doc.job
        });
        doc.apply(&c.finish()).unwrap();
        assert!((JobSetup::initial_params(&doc).width - 123.0).abs() < 1e-12);
    }

    #[test]
    fn rejects_a_zero_thickness() {
        assert_eq!(
            after_err::<JobSetup>(&Params {
                thickness: 0.0,
                ..Params::default()
            }),
            "invalid parameters: thickness must be greater than 0"
        );
    }
}
