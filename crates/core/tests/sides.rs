use cut_craft_core::Change;
use cut_craft_core::document::{Document, DoubleSided, Flip, Sheet};
use cut_craft_core::geometry::{Point, Rect};
use cut_craft_core::testing::{sample_document, square};

fn double_sided() -> Document {
    let mut doc = sample_document();
    let mut c = Change::builder(&doc);
    c.set_job(cut_craft_core::document::Job {
        double_sided: Some(DoubleSided::default()),
        ..doc.job
    });
    doc.apply(&c.finish()).unwrap();
    doc
}

#[test]
fn each_side_keeps_its_own_objects() {
    let mut doc = double_sided();
    let top = doc.vectors.clone();
    let before = doc.clone();
    let mut c = Change::builder(&doc);
    c.swap_sides();
    let change = c.finish();
    doc.apply(&change).unwrap();
    assert!(
        doc.bottom_side && doc.vectors.is_empty(),
        "the bottom is empty"
    );
    assert_eq!(doc.other_side.vectors.len(), top.len());
    let mut c = Change::builder(&doc);
    let on_bottom = c.add(square(1.0, 1.0, 2.0));
    doc.apply(&c.finish()).unwrap();
    assert!(!top.contains_key(&on_bottom), "a fresh id");
    let mut c = Change::builder(&doc);
    c.swap_sides();
    doc.apply(&c.finish()).unwrap();
    assert!(!doc.bottom_side);
    assert_eq!(doc.vectors, top);
    assert_eq!(doc.other_side.vectors.len(), 1);
    let mut undone = before.clone();
    undone.apply(&change).unwrap();
    undone.apply(&change.inverse()).unwrap();
    assert_eq!(undone, before);
}

#[test]
fn sides_stay_with_their_sheet() {
    let mut doc = double_sided();
    let mut c = Change::builder(&doc);
    c.swap_sides();
    let mut sheets = doc.sheets.clone();
    sheets.push(Sheet::new("Sheet 2", doc.job));
    c.set_sheets(sheets, 0);
    doc.apply(&c.finish()).unwrap();
    let stored_top = doc.other_side.clone();
    let mut c = Change::builder(&doc);
    c.activate_sheet(1);
    doc.apply(&c.finish()).unwrap();
    assert!(
        !doc.bottom_side && doc.other_side.is_empty(),
        "sheet 2 has nothing on either side"
    );
    let mut c = Change::builder(&doc);
    c.activate_sheet(0);
    doc.apply(&c.finish()).unwrap();
    assert!(doc.bottom_side, "back on sheet 1's bottom");
    assert_eq!(doc.other_side, stored_top);
}

#[test]
fn flipping_mirrors_across_the_material() {
    let area = Rect::new(Point::ZERO, Point::new(100.0, 50.0));
    let p = Point::new(10.0, 5.0);
    assert_eq!(
        Flip::Horizontal.mirror(area).transform_point2(p),
        Point::new(90.0, 5.0)
    );
    assert_eq!(
        Flip::Vertical.mirror(area).transform_point2(p),
        Point::new(10.0, 45.0)
    );
}
