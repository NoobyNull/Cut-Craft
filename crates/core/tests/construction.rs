//! Construction lines: vectors that place things but are never cut.

use cut_craft_core::document::{Document, EntityId};
use cut_craft_core::geometry::{Path, Point};
use cut_craft_core::glam::DVec3;
use cut_craft_core::selector::Selector;
use cut_craft_core::testing::{document_with, square, two_sheets};
use cut_craft_core::toolpath::{Move, MoveKind, Tool, made_by};
use cut_craft_core::{Change, Command, CommandError, Form, History, Meta, NoParams, Registry};

/// Test strategy, a toolpath: one plunge at the centre of each selected vector.
struct Centres;

impl Command for Centres {
    const ID: &'static str = "centres";
    type Params = NoParams;

    fn meta() -> Meta {
        Meta {
            label: "Centres",
            group: "Toolpath Operations",
            icon: "",
            shortcut: "",
            form: Form::Toolpath,
        }
    }

    fn run(doc: &Document, p: &NoParams) -> Result<Change, CommandError> {
        let (ids, paths) = doc.selected_paths(|_| true);
        if ids.is_empty() {
            return Err(CommandError::Failed("select the vectors to cut".into()));
        }
        let moves = paths.iter().map(|path| {
            let c = path.bounds().center();
            Move {
                kind: MoveKind::Plunge,
                to: DVec3::new(c.x, c.y, -1.0),
            }
        });
        let mut c = Change::builder(doc);
        c.add_toolpath(made_by::<Self>(
            p,
            "Centres".into(),
            Tool::default(),
            ids,
            moves.collect(),
        ));
        Ok(c.finish())
    }
}

/// The same, but not a toolpath tool: a drawing tool sees construction lines.
struct Count;

impl Command for Count {
    const ID: &'static str = "count";
    type Params = NoParams;

    fn meta() -> Meta {
        Meta {
            label: "Count",
            group: "Edit Objects",
            icon: "",
            shortcut: "",
            form: Form::Apply,
        }
    }

    fn run(doc: &Document, _: &NoParams) -> Result<Change, CommandError> {
        let mut c = Change::builder(doc);
        for _ in doc.selected_paths(|_| true).0 {
            c.add(square(0.0, 0.0, 1.0));
        }
        Ok(c.finish())
    }
}

fn registry() -> Registry {
    let mut r = Registry::default();
    r.add::<Centres>().add::<Count>();
    r
}

/// Two squares and a line, all selected, the line a construction line.
fn job() -> Document {
    let line = Path::polyline(&[Point::new(0.0, 50.0), Point::new(100.0, 50.0)], false);
    let mut doc = document_with(vec![square(0.0, 0.0, 10.0), square(50.0, 0.0, 10.0), line]);
    construct(&mut doc, EntityId(3), true);
    doc
}

fn construct(doc: &mut Document, id: EntityId, on: bool) {
    let mut c = Change::builder(doc);
    c.set_construction(id, on);
    doc.apply(&c.finish()).unwrap();
}

fn select(doc: &mut Document, ids: &[EntityId]) {
    let mut c = Change::builder(doc);
    c.select(ids.iter().copied());
    doc.apply(&c.finish()).unwrap();
}

fn made(doc: &Document, id: &str) -> Result<Document, CommandError> {
    let change = registry()
        .get(id)
        .unwrap()
        .run_json(doc, &serde_json::json!({}))?;
    let mut after = doc.clone();
    after.apply(&change).unwrap();
    Ok(after)
}

#[test]
fn a_toolpath_leaves_selected_construction_lines_out() {
    let doc = job();
    let after = made(&doc, "centres").unwrap();
    let toolpath = after.toolpaths.values().next().unwrap();
    assert_eq!(toolpath.vectors, [EntityId(1), EntityId(2)]);
    assert_eq!(toolpath.moves.len(), 2, "nothing cut along the line");
}

#[test]
fn other_tools_use_construction_lines_like_any_vector() {
    let doc = job();
    let after = made(&doc, "count").unwrap();
    assert_eq!(
        after.vectors.len(),
        6,
        "one square for each of the three picked"
    );
}

#[test]
fn a_toolpath_given_only_construction_lines_says_why_it_has_nothing() {
    let mut doc = job();
    select(&mut doc, &[EntityId(3)]);
    let err = made(&doc, "centres").unwrap_err().to_string();
    assert!(err.contains("construction lines"), "{err}");
    assert!(err.contains("Make Real Line"), "{err}");
}

#[test]
fn turning_a_toolpaths_vector_into_a_construction_line_marks_it_out_of_date() {
    let mut doc = job();
    let mut history = History::default();
    let change = registry()
        .get("centres")
        .unwrap()
        .run_json(&doc, &serde_json::json!({}))
        .unwrap();
    history.apply(&mut doc, change).unwrap();
    let tp = *doc.toolpaths.keys().next().unwrap();
    assert!(!doc.toolpaths[&tp].stale);
    let mut c = Change::builder(&doc);
    c.set_construction(EntityId(2), true);
    let change = c.finish();
    history.apply(&mut doc, change).unwrap();
    assert!(doc.toolpaths[&tp].stale, "it cuts a construction line now");
    // calculated again, it cuts only the real square left
    let change = registry().recalculate(&doc, tp).unwrap();
    history.apply(&mut doc, change).unwrap();
    assert_eq!(doc.toolpaths[&tp].vectors, [EntityId(1)]);
    assert!(!doc.toolpaths[&tp].stale);
}

#[test]
fn recalculating_a_toolpath_whose_vectors_are_all_construction_lines_says_so() {
    let mut doc = job();
    select(&mut doc, &[EntityId(1)]);
    doc = made(&doc, "centres").unwrap();
    let tp = *doc.toolpaths.keys().next().unwrap();
    construct(&mut doc, EntityId(1), true);
    let err = registry().recalculate(&doc, tp).unwrap_err().to_string();
    assert!(err.contains("construction line"), "{err}");
}

#[test]
fn a_vector_selector_never_picks_construction_lines() {
    let doc = job();
    let picked = Selector {
        open_vectors: true,
        ..Selector::default()
    }
    .picked(&doc);
    assert!(picked.contains(&EntityId(1)));
    assert!(!picked.contains(&EntityId(3)));
}

#[test]
fn construction_lines_stay_construction_lines_on_a_sheet_put_away() {
    let mut doc = two_sheets();
    let id = *doc.vectors.keys().next().unwrap();
    construct(&mut doc, id, true);
    for sheet in [1, 0] {
        let mut c = Change::builder(&doc);
        c.activate_sheet(sheet);
        doc.apply(&c.finish()).unwrap();
    }
    assert!(doc.is_construction(id));
    assert_eq!(doc.construction.len(), 1);
}
