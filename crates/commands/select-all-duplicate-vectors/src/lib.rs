//! Select All Duplicate Vectors: the extra copies of vectors that lie exactly on top of
//! another, so deleting the selection leaves one of each.

use cut_craft_core::document::Document;
use cut_craft_core::geometry::Path;
use cut_craft_core::{Change, Command, CommandError, Meta, NoParams};

pub struct SelectAllDuplicateVectors;

impl Command for SelectAllDuplicateVectors {
    const ID: &'static str = "select_all_duplicate_vectors";
    type Params = NoParams;

    fn meta() -> Meta {
        Meta {
            label: "Select All Duplicate Vectors",
            group: "Edit",
            icon: "",
            shortcut: "",
        }
    }

    fn run(doc: &Document, _: &NoParams) -> Result<Change, CommandError> {
        // ponytail: exact copies only (what copy, paste and import make); a tolerance if
        // near-duplicates turn up
        let mut seen: Vec<&Path> = vec![];
        let mut copies = vec![];
        for (&id, path) in doc.vectors.iter().filter(|(id, _)| doc.editable(**id)) {
            if seen.contains(&path) {
                copies.push(id);
            } else {
                seen.push(path);
            }
        }
        let mut c = Change::builder(doc);
        c.select(copies);
        Ok(c.finish())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cut_craft_core::document::EntityId;
    use cut_craft_core::testing::{document_with, square};

    cut_craft_core::contract_tests!(SelectAllDuplicateVectors);

    #[test]
    fn selects_every_copy_but_the_first() {
        let s = square(0.0, 0.0, 10.0);
        let mut doc = document_with(vec![s.clone(), square(20.0, 0.0, 10.0), s.clone(), s]);
        let change = SelectAllDuplicateVectors::run(&doc, &NoParams {}).unwrap();
        doc.apply(&change).unwrap();
        assert_eq!(doc.selection, [EntityId(3), EntityId(4)].into());
    }
}
