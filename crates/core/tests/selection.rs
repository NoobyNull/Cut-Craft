use aspire_core::document::{Document, EntityId};
use aspire_core::geometry::{Path, Point, Rect};
use aspire_core::{Change, History};

fn p(x: f64, y: f64) -> Point {
    Point::new(x, y)
}

/// Square A at 0..10, circle B centred (50,50) r 10, line C from (100,0) to (120,20).
fn doc() -> Document {
    let mut doc = Document::default();
    let mut c = Change::builder(&doc);
    c.add(Path::polyline(
        &[p(0.0, 0.0), p(10.0, 0.0), p(10.0, 10.0), p(0.0, 10.0)],
        true,
    ));
    c.add(Path::circle(p(50.0, 50.0), 10.0));
    c.add(Path::polyline(&[p(100.0, 0.0), p(120.0, 20.0)], false));
    doc.apply(&c.finish()).unwrap();
    doc
}

#[test]
fn undo_still_works_after_selecting_outside_the_history() {
    let mut doc = doc();
    let mut history = History::default();
    let draw = Change::add_and_select(&doc, Path::circle(p(0.0, 0.0), 1.0));
    history.apply(&mut doc, draw).unwrap();
    // a click elsewhere changes the selection without recording it
    let mut c = Change::builder(&doc);
    c.select([EntityId(1)]);
    doc.apply(&c.finish()).unwrap();
    assert!(
        history.undo(&mut doc),
        "undo must not trip over the changed selection"
    );
    assert_eq!(doc.vectors.len(), 3);
}

#[test]
fn pick_finds_the_path_under_the_pointer() {
    let doc = doc();
    assert_eq!(
        doc.pick(p(10.2, 5.0), 0.5),
        Some(EntityId(1)),
        "near the square's right edge"
    );
    assert_eq!(
        doc.pick(p(60.0, 50.0), 0.5),
        Some(EntityId(2)),
        "on the circle"
    );
    assert_eq!(
        doc.pick(p(110.0, 10.0), 0.5),
        Some(EntityId(3)),
        "on the line"
    );
}

#[test]
fn pick_ignores_the_empty_inside_of_shapes_and_far_points() {
    let doc = doc();
    assert_eq!(
        doc.pick(p(50.0, 50.0), 0.5),
        None,
        "centre of the circle is not on it"
    );
    assert_eq!(doc.pick(p(200.0, 200.0), 0.5), None);
}

#[test]
fn window_selects_only_paths_fully_inside() {
    let doc = doc();
    let found = doc.window(Rect::new(p(-1.0, -1.0), p(55.0, 55.0)), false);
    assert_eq!(found, [EntityId(1)], "the circle sticks out past x 55");
}

#[test]
fn crossing_also_selects_paths_the_box_touches() {
    let doc = doc();
    let found = doc.window(Rect::new(p(-1.0, -1.0), p(55.0, 55.0)), true);
    assert_eq!(found, [EntityId(1), EntityId(2)]);
}

#[test]
fn crossing_does_not_select_a_shape_whose_hole_contains_the_box() {
    let doc = doc();
    let found = doc.window(Rect::new(p(48.0, 48.0), p(52.0, 52.0)), true);
    assert!(
        found.is_empty(),
        "a box inside the circle touches none of its edges"
    );
}
