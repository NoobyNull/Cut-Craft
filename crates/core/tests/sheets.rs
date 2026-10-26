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
