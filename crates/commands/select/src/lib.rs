//! The Edit menu's selection commands: Select All Vectors, Select All Open Vectors, Select All
//! Duplicate Vectors, Select Vectors Inside and Unselect All. Each picks from the vectors on
//! visible, unlocked layers.

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

/// The smart part selection from the menu: keeps the selection and adds every
/// vector lying wholly inside a selected closed shape (its holes, and anything in them), so
/// a part can be picked whole without grouping it first.
pub struct SelectVectorsInside;

impl Command for SelectVectorsInside {
    const ID: &'static str = "select_vectors_inside";
    type Params = NoParams;

    fn meta() -> Meta {
        meta("Select Vectors Inside", "")
    }

    fn run(doc: &Document, _: &NoParams) -> Result<Change, CommandError> {
        let outlines: Vec<EntityId> = doc
            .selection
            .iter()
            .copied()
            .filter(|id| doc.vectors[id].closed)
            .collect();
        if outlines.is_empty() {
            return Err(CommandError::Failed(
                "Select a closed shape first, then choose Select Vectors Inside to add \
                 everything inside it (or just click inside the shape)."
                    .into(),
            ));
        }
        let inside = doc.vectors_inside(&outlines, CHORD);
        Ok(selecting(doc, doc.selection.iter().copied().chain(inside)))
    }
}

/// How closely curves are followed when testing what lies inside a shape, in mm.
const CHORD: f64 = 0.05;

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

    mod select_vectors_inside {
        use super::*;
        use cut_craft_core::geometry::Point;

        fn select(doc: &mut Document, ids: &[EntityId]) {
            let mut c = Change::builder(doc);
            c.select(ids.iter().copied());
            doc.apply(&c.finish()).unwrap();
        }

        #[test]
        fn adds_what_lies_inside_the_selected_shapes() {
            let mut doc = document_with(vec![
                square(0.0, 0.0, 100.0),
                square(10.0, 10.0, 20.0),
                Path::polyline(&[Point::new(50.0, 50.0), Point::new(90.0, 50.0)], false),
                square(200.0, 0.0, 50.0),
                square(210.0, 10.0, 10.0),
            ]);
            select(&mut doc, &[EntityId(1)]);
            run_on::<SelectVectorsInside>(&mut doc, &NoParams {});
            assert_eq!(
                doc.selection,
                [EntityId(1), EntityId(2), EntityId(3)].into(),
                "the panel keeps its place, its hole and groove join; the other shape stays out"
            );
        }

        #[test]
        fn says_what_to_do_without_a_closed_shape_selected() {
            let mut doc = document_with(vec![Path::polyline(
                &[Point::new(0.0, 0.0), Point::new(9.0, 0.0)],
                false,
            )]);
            let why = SelectVectorsInside::run(&doc, &NoParams {}).unwrap_err();
            assert!(why.to_string().contains("closed shape"), "{why}");
            select(&mut doc, &[]);
            assert!(SelectVectorsInside::run(&doc, &NoParams {}).is_err());
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
