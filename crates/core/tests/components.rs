use cut_craft_core::component::{Combine, Component, model};
use cut_craft_core::document::{Document, Job};
use cut_craft_core::file::{load_job, save_job};
use cut_craft_core::geometry::{Point, Rect};
use cut_craft_core::testing::square;
use cut_craft_core::{Change, History};

/// `height` everywhere over `area`, in 1 mm cells.
fn flat(area: Rect, height: f64, combine: Combine) -> Component {
    Component::from_fn("flat", combine, area, 1.0, |_| Some(height))
}

fn rect(x0: f64, y0: f64, x1: f64, y1: f64) -> Rect {
    Rect::new(Point::new(x0, y0), Point::new(x1, y1))
}

fn small_job() -> Document {
    Document {
        job: Job {
            width: 100.0,
            height: 100.0,
            ..Job::default()
        },
        ..Document::default()
    }
}

#[test]
fn components_sample_their_function_at_cell_centres() {
    let c = Component::from_fn("ramp", Combine::Add, rect(0.0, 0.0, 10.0, 4.0), 2.0, |p| {
        (p.x < 6.0).then_some(p.x)
    });
    assert_eq!((c.columns, c.rows), (5, 2));
    assert_eq!(
        c.height_at(Point::new(2.5, 3.0)),
        Some(3.0),
        "the cell centred on x = 3"
    );
    assert_eq!(c.height_at(Point::new(9.0, 1.0)), None, "outside the shape");
    assert_eq!(c.height_at(Point::new(11.0, 1.0)), None, "outside the grid");
}

#[test]
fn components_are_undoable_and_take_ids_no_vector_or_toolpath_has() {
    let mut doc = Document::default();
    let mut history = History::default();
    let mut c = Change::builder(&doc);
    let vector = c.add(square(0.0, 0.0, 10.0));
    let component = c.add_component(flat(rect(0.0, 0.0, 10.0, 10.0), 1.0, Combine::Add));
    let change = c.finish();
    history.apply(&mut doc, change).unwrap();
    assert_ne!(vector, component);
    assert!(doc.next_id().0 > component.0);
    let mut c = Change::builder(&doc);
    c.remove_component(component);
    let change = c.finish();
    history.apply(&mut doc, change).unwrap();
    assert!(doc.components.is_empty());
    history.undo(&mut doc);
    history.undo(&mut doc);
    assert_eq!(doc, Document::default());
}

#[test]
fn the_model_combines_components_in_the_order_they_were_made() {
    let cases = [
        (Combine::Add, 14.0, 4.0),
        (Combine::Subtract, 6.0, -4.0),
        (Combine::Merge, 10.0, 4.0),
        (Combine::Low, 4.0, 0.0),
        (Combine::Multiply, 40.0, 0.0),
    ];
    for (combine, overlap, alone) in cases {
        let mut doc = small_job();
        let mut c = Change::builder(&doc);
        c.add_component(flat(rect(0.0, 0.0, 50.0, 50.0), 10.0, Combine::Add));
        c.add_component(flat(rect(25.0, 25.0, 75.0, 75.0), 4.0, combine));
        let change = c.finish();
        doc.apply(&change).unwrap();
        let m = model(&doc, 100);
        assert_eq!(
            m.at(Point::new(30.0, 30.0)),
            Some(overlap),
            "{combine:?} overlap"
        );
        assert_eq!(
            m.at(Point::new(60.0, 60.0)),
            Some(alone),
            "{combine:?} alone"
        );
        assert_eq!(m.at(Point::new(90.0, 10.0)), Some(0.0), "nothing there");
    }
}

#[test]
fn components_are_saved_and_old_jobs_have_none() {
    let mut doc = small_job();
    let mut c = Change::builder(&doc);
    c.add_component(flat(rect(0.0, 0.0, 3.0, 2.0), 2.5, Combine::Merge));
    let change = c.finish();
    doc.apply(&change).unwrap();
    assert_eq!(load_job(&save_job(&doc)).unwrap(), doc);
    let mut json: serde_json::Value = serde_json::from_slice(&save_job(&doc)).unwrap();
    json["document"]
        .as_object_mut()
        .unwrap()
        .remove("components");
    let old = load_job(&serde_json::to_vec(&json).unwrap()).unwrap();
    assert!(old.components.is_empty());
}

#[test]
fn the_placed_model_has_its_top_at_the_surface_and_must_fit() {
    use cut_craft_core::component::placed_model;
    use cut_craft_core::toolpath::Tool;
    let mut doc = small_job();
    let tool = Tool::default(); // 6 mm: cells of 0.3 mm
    let why = placed_model(&doc, &tool).unwrap_err().to_string();
    assert!(why.contains("no 3D model"), "{why}");
    let mut c = Change::builder(&doc);
    c.add_component(flat(rect(10.0, 10.0, 30.0, 30.0), 6.0, Combine::Add));
    let change = c.finish();
    doc.apply(&change).unwrap();
    let placed = placed_model(&doc, &tool).unwrap();
    assert!((placed.cell - 0.3).abs() < 0.01);
    assert_eq!(placed.at(Point::new(20.0, 20.0)), Some(0.0));
    assert_eq!(placed.at(Point::new(50.0, 50.0)), Some(-6.0));
    doc.job.thickness = 5.0;
    let why = placed_model(&doc, &tool).unwrap_err().to_string();
    assert!(
        why.contains("6.0 mm tall") && why.contains("5.0 mm thick"),
        "{why}"
    );
}
