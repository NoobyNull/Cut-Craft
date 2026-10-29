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

#[test]
fn hidden_components_are_left_out_of_the_model() {
    let mut doc = small_job();
    let mut c = Change::builder(&doc);
    let shown = c.add_component(flat(rect(0.0, 0.0, 50.0, 50.0), 10.0, Combine::Add));
    let hidden = Component {
        visible: false,
        ..flat(rect(0.0, 0.0, 50.0, 50.0), 4.0, Combine::Add)
    };
    c.add_component(hidden);
    let change = c.finish();
    doc.apply(&change).unwrap();
    assert_eq!(model(&doc, 100).at(Point::new(20.0, 20.0)), Some(10.0));
    assert!(doc.components[&shown].visible, "components start visible");
}

#[test]
fn components_saved_before_visibility_load_visible() {
    let mut doc = small_job();
    let mut c = Change::builder(&doc);
    c.add_component(flat(rect(0.0, 0.0, 3.0, 2.0), 1.0, Combine::Add));
    let change = c.finish();
    doc.apply(&change).unwrap();
    let mut json: serde_json::Value = serde_json::from_slice(&save_job(&doc)).unwrap();
    for component in json["document"]["components"]
        .as_object_mut()
        .unwrap()
        .values_mut()
    {
        component.as_object_mut().unwrap().remove("visible");
    }
    let old = load_job(&serde_json::to_vec(&json).unwrap()).unwrap();
    assert!(old.components.values().all(|c| c.visible));
}

#[test]
fn the_placed_model_is_no_finer_than_the_job_resolution() {
    use cut_craft_core::component::placed_model;
    use cut_craft_core::document::Resolution;
    use cut_craft_core::toolpath::Tool;
    let mut doc = small_job();
    doc.job.width = 2000.0;
    doc.job.height = 1000.0;
    let mut c = Change::builder(&doc);
    c.add_component(flat(rect(10.0, 10.0, 30.0, 30.0), 6.0, Combine::Add));
    let change = c.finish();
    doc.apply(&change).unwrap();
    let tool = Tool::default();
    let mut cell = |resolution| {
        doc.job.resolution = resolution;
        placed_model(&doc, &tool).unwrap().cell
    };
    // 2 m² over a million points is cells of about 1.41 mm, coarser than the tool's 0.3
    let standard = cell(Resolution::Standard);
    assert!((standard - 2f64.sqrt()).abs() < 0.01, "{standard}");
    assert!(cell(Resolution::Highest) < standard / 2.0);
}

#[test]
fn the_chosen_component_is_the_last_picked_else_the_newest() {
    use cut_craft_core::document::EntityId;
    let mut doc = cut_craft_core::testing::two_components();
    assert_eq!(
        doc.chosen_component(),
        Some(EntityId(2)),
        "nothing picked: the newest"
    );
    doc.selected_components = [EntityId(2), EntityId(1)].into();
    assert_eq!(doc.chosen_component(), Some(EntityId(1)));
    doc.selected_components = [EntityId(9)].into();
    assert_eq!(
        doc.chosen_component(),
        Some(EntityId(2)),
        "a gone one is passed over"
    );
}

#[test]
fn the_model_alone_leaves_uncovered_cells_empty() {
    use cut_craft_core::component::{model, model_shape};
    use cut_craft_core::testing::domed;
    let doc = domed();
    let (shape, whole) = (model_shape(&doc, 50), model(&doc, 50));
    let empty = shape.heights.iter().filter(|h| h.is_nan()).count();
    assert!(
        empty > 0 && empty < shape.heights.len(),
        "{empty} of {}",
        shape.heights.len()
    );
    for (s, w) in shape.heights.iter().zip(&whole.heights) {
        if s.is_nan() {
            assert_eq!(*w, 0.0, "the zero plane where nothing is");
        } else {
            assert_eq!(s, w, "the model where it is");
        }
    }
}

#[test]
fn a_model_s_file_is_found_where_it_was_or_where_its_job_moved_with_it() {
    use cut_craft_core::component::Source;
    use std::path::Path;
    let dir = std::env::temp_dir().join(format!("cut-craft-source-{}", std::process::id()));
    let (old, new) = (dir.join("old"), dir.join("new"));
    for job in [&old, &new] {
        std::fs::create_dir_all(job.join("models")).unwrap();
    }
    let file = old.join("models/a.stl");
    std::fs::write(&file, b"solid a").unwrap();
    let mut source = Source::new(&file, b"solid a", serde_json::Value::Null);
    source.relate_to(&old);
    assert_eq!(source.relative.as_deref(), Some(Path::new("models/a.stl")));
    assert_eq!(source.verified(Some(&old)).unwrap(), file);
    // the job's folder and its models moved together
    let moved = new.join("models/a.stl");
    std::fs::rename(&file, &moved).unwrap();
    assert_eq!(source.verified(Some(&new)).unwrap(), moved);
    assert!(source.verified(Some(&old)).is_err(), "not where it was");
    std::fs::write(&moved, b"solid b").unwrap();
    assert!(source.verified(Some(&new)).unwrap_err().contains("changed"));
    let _ = std::fs::remove_dir_all(dir);
}
