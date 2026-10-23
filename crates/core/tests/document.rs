use aspire_core::document::{Document, EntityId};
use aspire_core::geometry::{Path, Point};
use aspire_core::{ApplyError, Change, History};

fn square(x: f64) -> Path {
    Path::polyline(
        &[
            Point::new(x, 0.0),
            Point::new(x + 1.0, 0.0),
            Point::new(x + 1.0, 1.0),
            Point::new(x, 1.0),
        ],
        true,
    )
}

fn add_squares(doc: &Document, xs: &[f64]) -> Change {
    let mut c = Change::builder(doc);
    for &x in xs {
        c.add(square(x));
    }
    c.finish()
}

#[test]
fn new_document_is_empty() {
    let doc = Document::default();
    assert!(doc.vectors.is_empty() && doc.selection.is_empty());
    assert_eq!(doc.next_id(), EntityId(1));
}

#[test]
fn builder_hands_out_distinct_ids_after_existing_ones() {
    let mut doc = Document::default();
    doc.apply(&add_squares(&doc, &[0.0, 2.0])).unwrap();
    let change = add_squares(&doc, &[4.0]);
    doc.apply(&change).unwrap();
    assert_eq!(
        doc.vectors.keys().copied().collect::<Vec<_>>(),
        [EntityId(1), EntityId(2), EntityId(3)]
    );
}

#[test]
fn inverse_restores_the_document_exactly() {
    let mut doc = Document::default();
    doc.apply(&add_squares(&doc, &[0.0, 2.0])).unwrap();
    let before = doc.clone();

    let mut c = Change::builder(&doc);
    c.replace(EntityId(1), square(9.0));
    c.remove(EntityId(2));
    c.select([EntityId(1)]);
    let change = c.finish();

    doc.apply(&change).unwrap();
    assert_ne!(doc, before);
    doc.apply(&change.inverse()).unwrap();
    assert_eq!(doc, before);
}

#[test]
fn apply_rejects_edits_to_missing_entities_and_leaves_document_untouched() {
    let mut doc = Document::default();
    doc.apply(&add_squares(&doc, &[0.0])).unwrap();
    let other = {
        let mut d = doc.clone();
        d.apply(&add_squares(&d, &[5.0])).unwrap();
        d
    };
    // a change built against a different document: removes id 2, which `doc` lacks
    let mut c = Change::builder(&other);
    c.replace(EntityId(1), square(3.0));
    c.remove(EntityId(2));
    let before = doc.clone();
    assert_eq!(
        doc.apply(&c.finish()),
        Err(ApplyError::Missing(EntityId(2)))
    );
    assert_eq!(doc, before, "a failed change must not be half-applied");
}

#[test]
fn chain_runs_steps_against_the_result_of_the_previous_step() {
    let doc = Document::default();
    let change = Change::chain(
        &doc,
        &[
            &|d: &Document| Ok(add_squares(d, &[0.0])),
            &|d: &Document| Ok(add_squares(d, &[2.0])),
        ],
    )
    .unwrap();
    let mut result = doc.clone();
    result.apply(&change).unwrap();
    assert_eq!(
        result.vectors.len(),
        2,
        "second step must not reuse the first step's id"
    );
}

#[test]
fn history_undo_redo_and_new_change_clears_redo() {
    let mut doc = Document::default();
    let mut history = History::default();
    let change = add_squares(&doc, &[0.0]);
    history.apply(&mut doc, change).unwrap();
    let change = add_squares(&doc, &[2.0]);
    history.apply(&mut doc, change).unwrap();
    assert!(history.undo(&mut doc));
    assert_eq!(doc.vectors.len(), 1);
    assert!(history.redo(&mut doc));
    assert_eq!(doc.vectors.len(), 2);
    assert!(history.undo(&mut doc));
    let change = add_squares(&doc, &[7.0]);
    history.apply(&mut doc, change).unwrap();
    assert!(
        !history.redo(&mut doc),
        "a new change discards the redo stack"
    );
}

#[test]
fn empty_changes_are_not_recorded_in_history() {
    let mut doc = Document::default();
    let mut history = History::default();
    history.apply(&mut doc, Change::default()).unwrap();
    assert!(!history.undo(&mut doc));
}

#[test]
fn document_round_trips_through_json() {
    let mut doc = Document::default();
    doc.apply(&add_squares(&doc, &[0.0, 2.0])).unwrap();
    let json = serde_json::to_string(&doc).unwrap();
    assert_eq!(serde_json::from_str::<Document>(&json).unwrap(), doc);
}

#[test]
fn add_and_select_makes_the_new_path_the_only_selection() {
    let mut doc = Document::default();
    let first = add_squares(&doc, &[0.0]);
    doc.apply(&first).unwrap();
    let change = Change::add_and_select(&doc, square(5.0));
    doc.apply(&change).unwrap();
    assert_eq!(
        doc.selection.iter().copied().collect::<Vec<_>>(),
        [EntityId(2)]
    );
}

#[test]
fn added_lists_the_paths_a_change_creates() {
    let doc = Document::default();
    let change = add_squares(&doc, &[0.0, 5.0]);
    let added: Vec<&Path> = change.added().collect();
    assert_eq!(added, [&square(0.0), &square(5.0)]);
}
