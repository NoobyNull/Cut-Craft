use cut_craft_core::Change;
use cut_craft_core::document::{Document, Job, Sheet};

fn sized(width: f64, height: f64, thickness: f64) -> Job {
    Job {
        width,
        height,
        thickness,
        ..Job::default()
    }
}

#[test]
fn a_job_has_one_sheet_to_begin_with() {
    let doc = Document::default();
    assert_eq!(doc.sheets.len(), 1);
    assert_eq!(doc.sheets[0].name, "Sheet 1");
    assert_eq!(doc.active_sheet, 0);
    assert_eq!(doc.sheet_job(0), Some(doc.job));
    let old: Document = serde_json::from_str(
        &serde_json::to_string(&Document::default())
            .unwrap()
            .replace(r#""sheets""#, r#""unused""#),
    )
    .unwrap();
    assert_eq!(old.sheets.len(), 1, "older files get the one sheet");
}

#[test]
fn activating_a_sheet_brings_its_setup_and_keeps_the_one_left() {
    let mut doc = Document {
        job: sized(800.0, 800.0, 25.0),
        ..Document::default()
    };
    let mut c = Change::builder(&doc);
    let mut sheets = doc.sheets.clone();
    sheets.push(Sheet::new("Sheet 2", sized(600.0, 300.0, 18.0)));
    c.set_sheets(sheets, 0);
    doc.apply(&c.finish()).unwrap();
    assert_eq!(doc.sheet_job(1).unwrap().width, 600.0);

    // Job Setup changes the active sheet: the list shows it before it is ever activated away
    let mut c = Change::builder(&doc);
    c.set_job(sized(810.0, 800.0, 25.0));
    doc.apply(&c.finish()).unwrap();
    assert_eq!(doc.sheet_job(0).unwrap().width, 810.0);

    let before = doc.clone();
    let mut c = Change::builder(&doc);
    c.activate_sheet(1);
    let change = c.finish();
    doc.apply(&change).unwrap();
    assert_eq!(doc.active_sheet, 1);
    assert_eq!(doc.job.width, 600.0);
    assert_eq!(
        doc.sheet_job(0).unwrap().width,
        810.0,
        "the sheet left keeps its setup"
    );
    doc.apply(&change.inverse()).unwrap();
    assert_eq!(doc, before);
}

#[test]
fn each_sheet_keeps_its_own_objects() {
    use cut_craft_core::testing::{square, with_toolpath};
    let mut doc = with_toolpath();
    let mut c = Change::builder(&doc);
    let a = c.add(square(0.0, 0.0, 10.0));
    let b = c.add(square(20.0, 0.0, 10.0));
    let group = doc.next_group();
    c.set_group(a, Some(group));
    c.set_group(b, Some(group));
    c.select([a]);
    let mut sheets = doc.sheets.clone();
    sheets.push(Sheet::new("Sheet 2", sized(300.0, 200.0, 12.0)));
    c.set_sheets(sheets, 0);
    doc.apply(&c.finish()).unwrap();
    let first = doc.clone();

    let mut c = Change::builder(&doc);
    c.activate_sheet(1);
    let to_second = c.finish();
    doc.apply(&to_second).unwrap();
    assert!(
        doc.vectors.is_empty() && doc.toolpaths.is_empty(),
        "sheet 2 is empty"
    );
    assert!(doc.selection.is_empty());
    let stored = &doc.sheets[0].objects;
    assert_eq!((stored.vectors.len(), stored.toolpaths.len()), (2, 1));

    // something drawn on sheet 2 gets an id no stored object has
    let mut c = Change::builder(&doc);
    let on_second = c.add(square(5.0, 5.0, 5.0));
    doc.apply(&c.finish()).unwrap();
    assert!(on_second.0 > b.0, "{on_second:?} is new");

    let mut c = Change::builder(&doc);
    c.activate_sheet(0);
    doc.apply(&c.finish()).unwrap();
    assert_eq!(
        doc.vectors, first.vectors,
        "the first sheet's vectors are back"
    );
    assert_eq!(doc.vector_groups, first.vector_groups, "with their group");
    assert_eq!(doc.toolpaths, first.toolpaths);
    assert_eq!(
        doc.sheets[1].objects.vectors.len(),
        1,
        "sheet 2 keeps its square"
    );
    assert_eq!(doc.job, first.job);
}

#[test]
fn activating_is_undone_exactly() {
    use cut_craft_core::testing::{square, with_toolpath};
    let mut doc = with_toolpath();
    let mut c = Change::builder(&doc);
    c.add(square(0.0, 0.0, 10.0));
    let mut sheets = doc.sheets.clone();
    sheets.push(Sheet::new("Sheet 2", sized(300.0, 200.0, 12.0)));
    c.set_sheets(sheets, 0);
    doc.apply(&c.finish()).unwrap();
    let before = doc.clone();
    let mut c = Change::builder(&doc);
    c.activate_sheet(1);
    let change = c.finish();
    doc.apply(&change).unwrap();
    doc.apply(&change.inverse()).unwrap();
    assert_eq!(doc, before);
}
