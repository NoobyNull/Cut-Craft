//! Subtract Vectors.

use cut_craft_core::document::Document;
use cut_craft_core::polygon;
use cut_craft_core::{Change, Command, CommandError, Meta, NoParams};

pub struct SubtractVectors;

impl Command for SubtractVectors {
    const ID: &'static str = "subtract_vectors";
    type Params = NoParams;

    fn meta() -> Meta {
        Meta {
            label: "Subtract Vectors",
            group: "Edit Objects",
            icon: "drawing/weld_and_offset/subtract_vectors.svg",
            shortcut: "",
        }
    }

    fn run(doc: &Document, _: &NoParams) -> Result<Change, CommandError> {
        let (ids, mut paths) = doc.selected_paths(|p| p.closed);
        if ids.len() < 2 {
            return Err(CommandError::Failed(
                "select at least two closed vectors to subtract".into(),
            ));
        }
        // the largest shape is cut by the others, whatever order they were selected in
        let largest = (0..paths.len())
            .max_by(|&a, &b| polygon::area(&paths[a]).total_cmp(&polygon::area(&paths[b])))
            .unwrap_or(0);
        let subject = paths.swap_remove(largest);
        Ok(Change::swap(
            doc,
            ids,
            polygon::difference(&[subject], &paths),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use cut_craft_core::geometry::Path;
    use cut_craft_core::testing::{document_with, square};

    fn subtract(paths: Vec<Path>) -> Document {
        let mut doc = document_with(paths);
        doc.apply(&SubtractVectors::run(&doc, &NoParams {}).unwrap())
            .unwrap();
        doc
    }

    #[test]
    fn the_largest_shape_is_cut_by_the_others() {
        // small square listed first: the big one is still the one kept
        let doc = subtract(vec![square(15.0, 15.0, 10.0), square(0.0, 0.0, 20.0)]);
        let area: f64 = doc
            .vectors
            .values()
            .map(cut_craft_core::polygon::area)
            .sum();
        assert!(
            (area - 375.0).abs() < 1e-6,
            "400 - the 25 overlap, got {area}"
        );
    }

    #[test]
    fn a_shape_inside_leaves_a_hole() {
        assert_eq!(
            subtract(vec![square(0.0, 0.0, 20.0), square(5.0, 5.0, 5.0)])
                .vectors
                .len(),
            2
        );
    }

    cut_craft_core::contract_tests!(SubtractVectors);
}
