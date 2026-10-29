//! The Edit menu's selection commands: Select All Vectors, Select All Open Vectors, Select All
//! Duplicate Vectors and Unselect All. Each picks from the vectors on visible, unlocked layers.

use cut_craft_core::document::{Document, EntityId};
use cut_craft_core::geometry::Path;
use cut_craft_core::{Change, Command, CommandError, Meta, NoParams};

fn meta(label: &'static str, shortcut: &'static str) -> Meta {
    Meta {
        label,
        group: "Edit",
        icon: "",
        shortcut,
    }
}

/// The change that selects exactly `ids`.
fn selecting(doc: &Document, ids: impl IntoIterator<Item = EntityId>) -> Change {
    let mut c = Change::builder(doc);
    c.select(ids);
    c.finish()
}

pub struct SelectAllVectors;

impl Command for SelectAllVectors {
    const ID: &'static str = "select_all_vectors";
    type Params = NoParams;

    fn meta() -> Meta {
        meta("Select All Vectors", "Ctrl+A")
    }

    fn run(doc: &Document, _: &NoParams) -> Result<Change, CommandError> {
        Ok(selecting(
            doc,
            doc.vectors.keys().copied().filter(|&id| doc.editable(id)),
        ))
    }
}

pub struct SelectAllOpenVectors;

impl Command for SelectAllOpenVectors {
    const ID: &'static str = "select_all_open_vectors";
    type Params = NoParams;

    fn meta() -> Meta {
        meta("Select All Open Vectors", "")
    }

    fn run(doc: &Document, _: &NoParams) -> Result<Change, CommandError> {
        Ok(selecting(
            doc,
            doc.vectors
                .iter()
                .filter(|(id, path)| !path.closed && doc.editable(**id))
                .map(|(&id, _)| id),
        ))
    }
}

/// The extra copies of vectors that lie exactly on top of another, so deleting the selection
/// leaves one of each.
pub struct SelectAllDuplicateVectors;

impl Command for SelectAllDuplicateVectors {
    const ID: &'static str = "select_all_duplicate_vectors";
    type Params = NoParams;

    fn meta() -> Meta {
        meta("Select All Duplicate Vectors", "")
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
        Ok(selecting(doc, copies))
    }
}

pub struct UnselectAll;

impl Command for UnselectAll {
    const ID: &'static str = "unselect_all";
    type Params = NoParams;

    fn meta() -> Meta {
        meta("Unselect All", "")
    }

    fn run(doc: &Document, _: &NoParams) -> Result<Change, CommandError> {
        Ok(selecting(doc, []))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cut_craft_core::testing::{
        after, document_with, edit_layer, run_on, sample_document, square,
    };

    mod select_all_vectors {
        use super::*;
        cut_craft_core::contract_tests!(SelectAllVectors);

        #[test]
        fn selects_every_vector() {
            let mut doc = sample_document();
            run_on::<UnselectAll>(&mut doc, &NoParams {});
            run_on::<SelectAllVectors>(&mut doc, &NoParams {});
            assert_eq!(doc.selection.len(), doc.vectors.len());
        }

        #[test]
        fn skips_vectors_on_hidden_or_locked_layers() {
            let mut doc = sample_document();
            let first = doc.layers[0].id;
            edit_layer(&mut doc, first, |l| l.locked = true);
            run_on::<SelectAllVectors>(&mut doc, &NoParams {});
            assert!(doc.selection.is_empty());
        }
    }

    mod select_all_open_vectors {
        use super::*;
        cut_craft_core::contract_tests!(SelectAllOpenVectors);

        #[test]
        fn selects_only_the_open_ones() {
            let doc = after::<SelectAllOpenVectors>(&NoParams {});
            assert_eq!(doc.selection.len(), 1, "the sample has one open polyline");
            assert!(doc.selection.iter().all(|id| !doc.vectors[id].closed));
        }
    }

    mod select_all_duplicate_vectors {
        use super::*;
        cut_craft_core::contract_tests!(SelectAllDuplicateVectors);

        #[test]
        fn selects_every_copy_but_the_first() {
            let s = square(0.0, 0.0, 10.0);
            let mut doc = document_with(vec![s.clone(), square(20.0, 0.0, 10.0), s.clone(), s]);
            run_on::<SelectAllDuplicateVectors>(&mut doc, &NoParams {});
            assert_eq!(doc.selection, [EntityId(3), EntityId(4)].into());
        }
    }

    mod unselect_all {
        use super::*;
        cut_craft_core::contract_tests!(UnselectAll);

        #[test]
        fn clears_the_selection() {
            assert!(after::<UnselectAll>(&NoParams {}).selection.is_empty());
        }
    }
}
