use aspire_core::document::{Document, EntityId};
use aspire_core::geometry::{Path, Point, Rect};
use aspire_core::glam::DAffine2;
use aspire_core::params::Anchor;
use aspire_core::{Change, CommandError};

fn doc_with_squares() -> Document {
    let mut doc = Document::default();
    let mut c = Change::builder(&doc);
    let a = c.add(Path::polyline(
        &[
            Point::new(0.0, 0.0),
            Point::new(10.0, 0.0),
            Point::new(10.0, 10.0),
        ],
        true,
    ));
    c.add(Path::polyline(
        &[
            Point::new(50.0, 50.0),
            Point::new(60.0, 50.0),
            Point::new(60.0, 60.0),
        ],
        true,
    ));
    c.select([a]);
    doc.apply(&c.finish()).unwrap();
    doc
}

#[test]
fn transforms_only_the_selected_paths() {
    let mut doc = doc_with_squares();
    let change =
        Change::transform_selected(&doc, |_| DAffine2::from_translation(Point::new(5.0, 0.0)))
            .unwrap();
    doc.apply(&change).unwrap();
    assert_eq!(
        doc.vectors[&EntityId(1)].bounds(),
        Rect::new(Point::new(5.0, 0.0), Point::new(15.0, 10.0))
    );
    assert_eq!(
        doc.vectors[&EntityId(2)].bounds().min,
        Point::new(50.0, 50.0)
    );
}

#[test]
fn nothing_selected_is_an_error() {
    let mut doc = doc_with_squares();
    let mut c = Change::builder(&doc);
    c.select([]);
    doc.apply(&c.finish()).unwrap();
    assert_eq!(
        Change::transform_selected(&doc, |_| DAffine2::IDENTITY),
        Err(CommandError::NothingSelected)
    );
}

#[test]
fn identity_transforms_make_no_edits() {
    let doc = doc_with_squares();
    assert!(
        Change::transform_selected(&doc, |_| DAffine2::IDENTITY)
            .unwrap()
            .is_empty()
    );
}

#[test]
fn anchors_pick_points_of_a_rectangle() {
    let r = Rect::new(Point::new(0.0, 0.0), Point::new(10.0, 20.0));
    assert_eq!(Anchor::BottomLeft.point(r), Point::new(0.0, 0.0));
    assert_eq!(Anchor::Center.point(r), Point::new(5.0, 10.0));
    assert_eq!(Anchor::TopRight.point(r), Point::new(10.0, 20.0));
    assert_eq!(Anchor::Left.point(r), Point::new(0.0, 10.0));
    assert_eq!(Anchor::Top.point(r), Point::new(5.0, 20.0));
}

#[test]
fn align_selected_moves_each_path_by_an_offset_from_its_bounds() {
    let mut doc = doc_with_squares();
    let mut c = Change::builder(&doc);
    c.select([EntityId(1), EntityId(2)]);
    doc.apply(&c.finish()).unwrap();
    // left-align: each path's left edge to the selection's left edge
    let change =
        Change::align_selected(&doc, |path, sel| Point::new(sel.min.x - path.min.x, 0.0)).unwrap();
    doc.apply(&change).unwrap();
    assert!(doc.vectors.values().all(|p| p.bounds().min.x == 0.0));
}
