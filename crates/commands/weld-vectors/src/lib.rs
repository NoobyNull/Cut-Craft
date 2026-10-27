//! Weld Vectors.

use cut_craft_core::document::Document;
use cut_craft_core::polygon;
use cut_craft_core::{Change, Command, CommandError, Meta, NoParams};

pub struct WeldVectors;

impl Command for WeldVectors {
    const ID: &'static str = "weld_vectors";
    type Params = NoParams;

    fn meta() -> Meta {
        Meta {
            label: "Weld Vectors",
            group: "Edit Objects",
            icon: "drawing/weld_and_offset/weld_vectors.png",
            shortcut: "",
        }
    }

    fn run(doc: &Document, _: &NoParams) -> Result<Change, CommandError> {
        let (ids, paths) = doc.selected_paths(|p| p.closed);
        if ids.len() < 2 {
            return Err(CommandError::Failed(
                "select at least two closed vectors to weld".into(),
            ));
        }
        Ok(Change::swap(doc, ids, polygon::union(&paths)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use cut_craft_core::testing::{after, document_with, square};

    #[test]
    fn overlapping_shapes_become_one_outline() {
        let doc = document_with(vec![square(0.0, 0.0, 10.0), square(5.0, 5.0, 10.0)]);
        let mut after = doc.clone();
        after
            .apply(&WeldVectors::run(&doc, &NoParams {}).unwrap())
            .unwrap();
        assert_eq!(after.vectors.len(), 1);
        assert!(
            (cut_craft_core::polygon::area(after.vectors.values().next().unwrap()) - 175.0).abs()
                < 1e-6
        );
    }

    #[test]
    fn open_vectors_are_left_alone() {
        let doc = after::<WeldVectors>(&NoParams {});
        assert_eq!(
            doc.vectors.values().filter(|p| !p.closed).count(),
            1,
            "the sample's open line survives"
        );
    }

    #[test]
    fn one_closed_vector_is_not_enough() {
        let doc = document_with(vec![square(0.0, 0.0, 10.0)]);
        assert_eq!(
            WeldVectors::run(&doc, &NoParams {})
                .unwrap_err()
                .to_string(),
            "select at least two closed vectors to weld"
        );
    }

    cut_craft_core::contract_tests!(WeldVectors);
}
