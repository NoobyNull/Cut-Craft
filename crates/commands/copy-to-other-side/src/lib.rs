//! Copy to Other Side: in a double-sided job, the selected vectors copied onto the other side,
//! mirrored as the material is turned over so they line up through it (Aspire's right-click
//! Copy to ▸ Other Side).

use cut_craft_core::document::{Document, StoredVector};
use cut_craft_core::{Change, Command, CommandError, Meta, NoParams};

pub struct CopyToOtherSide;

impl Command for CopyToOtherSide {
    const ID: &'static str = "copy_to_other_side";
    type Params = NoParams;

    fn meta() -> Meta {
        Meta {
            label: "Copy to Other Side",
            group: "Edit",
            icon: "",
            shortcut: "",
        }
    }

    fn run(doc: &Document, _: &NoParams) -> Result<Change, CommandError> {
        let Some(sides) = doc.job.double_sided else {
            return Err(CommandError::Failed(
                "only a double-sided job has another side (Job Setup)".into(),
            ));
        };
        if doc.selection.is_empty() {
            return Err(CommandError::NothingSelected);
        }
        let turn = sides.flip.mirror(doc.job.area());
        let mut c = Change::builder(doc);
        let (bottom, mut other) = c.side().clone();
        for id in &doc.selection {
            let copy = StoredVector {
                path: doc.vectors[id].transformed(&turn),
                layer: Some(doc.current_layer),
                group: None,
            };
            let fresh = c.fresh_id();
            other.vectors.insert(fresh, copy);
        }
        c.set_side(bottom, other);
        Ok(c.finish())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cut_craft_core::document::{DoubleSided, Flip, Job};
    use cut_craft_core::testing::{rejection, run_on, sample_document};

    fn double_sided() -> Document {
        let mut doc = sample_document();
        doc.job = Job {
            double_sided: Some(DoubleSided {
                flip: Flip::Horizontal,
                zero_same_side: false,
            }),
            ..doc.job
        };
        doc
    }

    cut_craft_core::contract_tests!(CopyToOtherSide, double_sided);

    #[test]
    fn the_copies_are_mirrored_onto_the_other_side() {
        let mut doc = double_sided();
        let area = doc.job.area();
        let first = doc.vectors[doc.selection.iter().next().unwrap()].bounds();
        run_on::<CopyToOtherSide>(&mut doc, &NoParams {});
        assert_eq!(doc.other_side.vectors.len(), doc.selection.len());
        let copy = doc
            .other_side
            .vectors
            .values()
            .next()
            .unwrap()
            .path
            .bounds();
        let mirrored_max = area.min.x + area.max.x - first.min.x;
        assert!(
            (copy.max.x - mirrored_max).abs() < 1e-9,
            "{copy:?} vs {first:?}"
        );
        assert!(
            doc.other_side
                .vectors
                .keys()
                .all(|id| !doc.vectors.contains_key(id))
        );
    }

    #[test]
    fn needs_a_double_sided_job() {
        assert!(rejection::<CopyToOtherSide>(&NoParams {}).contains("double-sided"));
    }
}
