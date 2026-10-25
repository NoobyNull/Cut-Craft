//! Select All Open Vectors: every open vector on a visible, unlocked layer.

use cut_craft_core::document::Document;
use cut_craft_core::{Change, Command, CommandError, Meta, NoParams};

pub struct SelectAllOpenVectors;

impl Command for SelectAllOpenVectors {
    const ID: &'static str = "select_all_open_vectors";
    type Params = NoParams;

    fn meta() -> Meta {
        Meta {
            label: "Select All Open Vectors",
            group: "Edit",
            icon: "",
            shortcut: "",
        }
    }

    fn run(doc: &Document, _: &NoParams) -> Result<Change, CommandError> {
        let mut c = Change::builder(doc);
        c.select(
            doc.vectors
                .iter()
                .filter(|(id, path)| !path.closed && doc.editable(**id))
                .map(|(&id, _)| id),
        );
        Ok(c.finish())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    cut_craft_core::contract_tests!(SelectAllOpenVectors);

    #[test]
    fn selects_only_the_open_ones() {
        let doc = cut_craft_core::testing::after::<SelectAllOpenVectors>(&NoParams {});
        assert_eq!(doc.selection.len(), 1, "the sample has one open polyline");
        assert!(doc.selection.iter().all(|id| !doc.vectors[id].closed));
    }
}
