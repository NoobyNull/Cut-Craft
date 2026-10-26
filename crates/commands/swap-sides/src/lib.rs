//! Swap Sides: in a double-sided job, work on the other side of the material. Each side keeps
//! its own vectors, components, pictures and toolpaths.

use cut_craft_core::document::Document;
use cut_craft_core::{Change, Command, CommandError, Meta, NoParams};

pub struct SwapSides;

impl Command for SwapSides {
    const ID: &'static str = "swap_sides";
    type Params = NoParams;

    fn meta() -> Meta {
        Meta {
            label: "Swap Sides",
            group: "Edit",
            icon: "",
            shortcut: "",
        }
    }

    fn run(doc: &Document, _: &NoParams) -> Result<Change, CommandError> {
        if doc.job.double_sided.is_none() {
            return Err(CommandError::Failed(
                "only a double-sided job has another side (Job Setup)".into(),
            ));
        }
        let mut c = Change::builder(doc);
        c.swap_sides();
        Ok(c.finish())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cut_craft_core::document::{DoubleSided, Job};
    use cut_craft_core::testing::{rejection, run_on, sample_document};

    fn double_sided() -> Document {
        let mut doc = sample_document();
        doc.job = Job {
            double_sided: Some(DoubleSided::default()),
            ..doc.job
        };
        doc
    }

    cut_craft_core::contract_tests!(SwapSides, double_sided);

    #[test]
    fn turns_to_the_other_side_and_back() {
        let mut doc = double_sided();
        let top = doc.vectors.clone();
        run_on::<SwapSides>(&mut doc, &NoParams {});
        assert!(doc.bottom_side && doc.vectors.is_empty());
        run_on::<SwapSides>(&mut doc, &NoParams {});
        assert_eq!(doc.vectors, top);
    }

    #[test]
    fn needs_a_double_sided_job() {
        assert!(rejection::<SwapSides>(&NoParams {}).contains("double-sided"));
    }
}
