use cut_craft_core::component::{Combine, Component, Level, Mirror, model};
use cut_craft_core::document::{Along, Document, Job};
use cut_craft_core::geometry::{Point, Rect};
use cut_craft_core::testing::square;
use cut_craft_core::{Change, History};

fn rect(x0: f64, y0: f64, x1: f64, y1: f64) -> Rect {
    Rect::new(Point::new(x0, y0), Point::new(x1, y1))
}

fn flat(area: Rect, height: f64, combine: Combine, level: u64) -> Component {
    let mut k = Component::from_fn("flat", combine, area, 1.0, |_| Some(height));
    k.level = level;
    k
}

fn job(width: f64, height: f64) -> Document {
    Document {
        job: Job {
            width,
            height,
            ..Job::default()
        },
        ..Document::default()
    }
}

fn with(mut doc: Document, levels: &[(u64, Level)], components: Vec<Component>) -> Document {
    let mut c = Change::builder(&doc);
    for (id, level) in levels {
        c.set_level(*id, Some(level.clone()));
    }
    for k in components {
        c.add_component(k);
    }
    doc.apply(&c.finish()).unwrap();
    doc
}

fn level(name: &str, combine: Combine) -> Level {
    Level {
        name: name.into(),
        combine,
        ..Level::default()
    }
}

fn at(doc: &Document, x: f64, y: f64) -> f64 {
    model(doc, 100).at(Point::new(x, y)).unwrap()
}

#[test]
fn a_new_job_has_one_level_and_old_files_get_it_too() {
    let doc = Document::default();
    assert_eq!(doc.levels.len(), 1);
    let mut json = serde_json::to_value(&doc).unwrap();
    json.as_object_mut().unwrap().remove("levels");
    let old: Document = serde_json::from_value(json).unwrap();
    assert_eq!(old.levels, doc.levels);
}

#[test]
fn a_level_combines_its_own_components_before_it_combines_with_the_levels_below() {
    // level 0: a 2 mm plate. Level 1, subtracted: a 5 mm plate with a 1 mm plate subtracted
    // from it, so 4 mm comes off; one level would have given 2 + ... in order instead
    let doc = with(
        job(100.0, 100.0),
        &[(1, level("Cut", Combine::Subtract))],
        vec![
            flat(rect(0.0, 0.0, 100.0, 100.0), 6.0, Combine::Add, 0),
            flat(rect(0.0, 0.0, 50.0, 100.0), 5.0, Combine::Add, 1),
            flat(rect(0.0, 0.0, 50.0, 100.0), 1.0, Combine::Subtract, 1),
        ],
    );
    assert!((at(&doc, 25.0, 50.0) - 2.0).abs() < 1e-9);
    assert!(
        (at(&doc, 75.0, 50.0) - 6.0).abs() < 1e-9,
        "the level has no shape there"
    );
}

#[test]
fn hidden_levels_and_base_heights() {
    let mut raised = level("Raised", Combine::Merge);
    raised.base_height = 3.0;
    let mut doc = with(
        job(100.0, 100.0),
        &[(1, raised.clone())],
        vec![
            flat(rect(0.0, 0.0, 100.0, 100.0), 2.0, Combine::Add, 0),
            flat(rect(0.0, 0.0, 50.0, 100.0), 1.0, Combine::Add, 1),
        ],
    );
    assert!(
        (at(&doc, 25.0, 50.0) - 4.0).abs() < 1e-9,
        "1 mm on a 3 mm base, merged"
    );
    raised.visible = false;
    let mut c = Change::builder(&doc);
    c.set_level(1, Some(raised));
    doc.apply(&c.finish()).unwrap();
    assert!((at(&doc, 25.0, 50.0) - 2.0).abs() < 1e-9);
}

#[test]
fn clipping_keeps_a_level_only_inside_its_vectors() {
    let mut clipped = level("Clipped", Combine::Add);
    clipped.clip = vec![square(0.0, 0.0, 50.0)];
    let doc = with(
        job(100.0, 100.0),
        &[(0, clipped)],
        vec![flat(rect(0.0, 0.0, 100.0, 100.0), 2.0, Combine::Add, 0)],
    );
    assert!((at(&doc, 25.0, 25.0) - 2.0).abs() < 1e-9);
    assert!(at(&doc, 75.0, 75.0).abs() < 1e-9);
    assert!(at(&doc, 25.0, 75.0).abs() < 1e-9);
}

#[test]
fn mirror_left_to_right_reflects_the_left_half() {
    let mut mirrored = level("Mirrored", Combine::Add);
    mirrored.mirror = Mirror::LeftToRight;
    let ramp = |p: Point| Some(p.x / 10.0);
    let mut k = Component::from_fn("ramp", Combine::Add, rect(0.0, 0.0, 40.0, 100.0), 1.0, ramp);
    k.level = 0;
    let doc = with(job(100.0, 100.0), &[(0, mirrored)], vec![k]);
    assert!((at(&doc, 10.5, 50.0) - at(&doc, 89.5, 50.0)).abs() < 1e-9);
    let mut quarter = level("Quarter", Combine::Add);
    quarter.mirror = Mirror::BottomLeft;
    let spot = flat(rect(10.0, 10.0, 20.0, 20.0), 3.0, Combine::Add, 0);
    let doc = with(job(100.0, 100.0), &[(0, quarter)], vec![spot]);
    for (x, y) in [(15.0, 15.0), (85.0, 15.0), (15.0, 85.0), (85.0, 85.0)] {
        assert!((at(&doc, x, y) - 3.0).abs() < 1e-9, "({x}, {y})");
    }
}

#[test]
fn wrapping_brings_what_runs_off_one_side_of_a_rotary_job_round_the_other() {
    let mut doc = Document {
        job: Job::rotary(100.0, 100.0 / std::f64::consts::PI, Along::X),
        ..Document::default()
    };
    // the circumference (y) is 100 mm; the strip runs from y 90 to 110
    let strip = flat(rect(0.0, 90.0, 100.0, 110.0), 2.0, Combine::Add, 0);
    doc = with(doc, &[], vec![strip]);
    assert!(at(&doc, 50.0, 5.0).abs() < 1e-9, "cut off without wrapping");
    let mut wrapped = doc.levels[&0].clone();
    wrapped.wrapping = true;
    let mut c = Change::builder(&doc);
    c.set_level(0, Some(wrapped));
    doc.apply(&c.finish()).unwrap();
    assert!((at(&doc, 50.0, 5.0) - 2.0).abs() < 1e-9);
    assert!((at(&doc, 50.0, 95.0) - 2.0).abs() < 1e-9);
}

#[test]
fn level_edits_undo() {
    let mut doc = Document::default();
    let mut history = History::default();
    let mut c = Change::builder(&doc);
    c.set_level(7, Some(level("New", Combine::Add)));
    let change = c.finish();
    history.apply(&mut doc, change).unwrap();
    assert!(doc.levels.contains_key(&7));
    let mut c = Change::builder(&doc);
    c.set_level(7, None);
    let change = c.finish();
    history.apply(&mut doc, change).unwrap();
    assert!(!doc.levels.contains_key(&7));
    history.undo(&mut doc);
    assert_eq!(doc.levels[&7].name, "New");
    history.undo(&mut doc);
    assert_eq!(doc.levels.len(), 1);
}

#[test]
fn a_component_on_a_missing_level_counts_on_the_first() {
    let doc = with(
        job(100.0, 100.0),
        &[],
        vec![flat(rect(0.0, 0.0, 100.0, 100.0), 2.0, Combine::Add, 9)],
    );
    assert!((at(&doc, 50.0, 50.0) - 2.0).abs() < 1e-9);
}

#[test]
fn a_group_combines_its_members_first_then_where_its_top_member_is() {
    use cut_craft_core::component::Group;
    let cut = Group {
        id: 1,
        name: "Cut".into(),
        combine: Combine::Subtract,
    };
    let member = |height, combine| {
        let mut k = flat(rect(0.0, 0.0, 50.0, 100.0), height, combine, 0);
        k.group = Some(cut.clone());
        k
    };
    // 6 mm plate, then the group (5 mm less a 1 mm plate, so 4 mm comes off), then 0.5 mm on
    let doc = with(
        job(100.0, 100.0),
        &[],
        vec![
            flat(rect(0.0, 0.0, 100.0, 100.0), 6.0, Combine::Add, 0),
            member(5.0, Combine::Add),
            member(1.0, Combine::Subtract),
            flat(rect(0.0, 0.0, 100.0, 100.0), 0.5, Combine::Add, 0),
        ],
    );
    assert!(
        (at(&doc, 25.0, 50.0) - 2.5).abs() < 1e-9,
        "{}",
        at(&doc, 25.0, 50.0)
    );
    assert!((at(&doc, 75.0, 50.0) - 6.5).abs() < 1e-9);
}
