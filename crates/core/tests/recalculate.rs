use cut_craft_core::document::{Document, EntityId};
use cut_craft_core::geometry::Point;
use cut_craft_core::glam::{DAffine2, DVec3};
use cut_craft_core::testing::{document_with, square, two_components, with_toolpath};
use cut_craft_core::toolpath::{Move, MoveKind, Tool, made_by};
use cut_craft_core::{Change, Command, CommandError, History, Meta, NoParams, Registry};

/// Test strategy: one plunge at the centre of each selected vector.
struct Centres;

impl Command for Centres {
    const ID: &'static str = "centres";
    type Params = NoParams;

    fn meta() -> Meta {
        Meta {
            label: "Centres",
            group: "Test",
            icon: "",
            shortcut: "",
        }
    }

    fn run(doc: &Document, p: &NoParams) -> Result<Change, CommandError> {
        let (ids, paths) = doc.selected_paths(|_| true);
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

fn registry() -> Registry {
    let mut r = Registry::default();
    r.add::<Centres>();
    r
}

/// Two squares, a toolpath made from the first one only.
fn job() -> (Document, EntityId) {
    let mut doc = document_with(vec![square(0.0, 0.0, 10.0), square(50.0, 0.0, 10.0)]);
    let mut c = Change::builder(&doc);
    c.select([EntityId(1)]);
    doc.apply(&c.finish()).unwrap();
    doc.apply(&Centres::run(&doc, &NoParams {}).unwrap())
        .unwrap();
    let id = *doc.toolpaths.keys().next().unwrap();
    (doc, id)
}

fn move_vector(doc: &mut Document, id: EntityId, by: Point) {
    let mut c = Change::builder(doc);
    let moved = doc.vectors[&id].transformed(&DAffine2::from_translation(by));
    c.replace(id, moved);
    doc.apply(&c.finish()).unwrap();
}

fn plunge(doc: &Document, id: EntityId) -> Point {
    doc.toolpaths[&id].moves[0].to.truncate()
}

#[test]
fn recalculating_follows_the_vectors_it_was_made_from_whatever_is_selected() {
    let (mut doc, tp) = job();
    move_vector(&mut doc, EntityId(1), Point::new(100.0, 0.0));
    let mut c = Change::builder(&doc);
    c.select([EntityId(2)]);
    doc.apply(&c.finish()).unwrap();
    let change = registry().recalculate(&doc, tp).unwrap();
    doc.apply(&change).unwrap();
    assert_eq!(plunge(&doc, tp), Point::new(105.0, 5.0));
    assert_eq!(doc.toolpaths.len(), 1, "replaced in place, same id");
    doc.apply(&change.inverse()).unwrap();
    assert_eq!(plunge(&doc, tp), Point::new(5.0, 5.0), "and undoable");
}

#[test]
fn nothing_to_recalculate_when_nothing_moved() {
    let (doc, tp) = job();
    assert!(registry().recalculate(&doc, tp).unwrap().is_empty());
}

#[test]
fn recalculate_all_is_recalculate_for_each_toolpath() {
    let (mut doc, first) = job();
    let mut c = Change::builder(&doc);
    c.select([EntityId(2)]);
    doc.apply(&c.finish()).unwrap();
    doc.apply(&Centres::run(&doc, &NoParams {}).unwrap())
        .unwrap();
    let second = *doc.toolpaths.keys().last().unwrap();
    move_vector(&mut doc, EntityId(1), Point::new(0.0, 20.0));
    move_vector(&mut doc, EntityId(2), Point::new(0.0, 30.0));
    let change = registry().recalculate_all(&doc).unwrap();
    doc.apply(&change).unwrap();
    assert_eq!(plunge(&doc, first), Point::new(5.0, 25.0));
    assert_eq!(plunge(&doc, second), Point::new(55.0, 35.0));
}

#[test]
fn deleted_source_vectors_and_unknown_strategies_are_explained() {
    let (mut doc, tp) = job();
    let mut c = Change::builder(&doc);
    c.remove(EntityId(1));
    doc.apply(&c.finish()).unwrap();
    assert_eq!(
        registry().recalculate(&doc, tp).unwrap_err().to_string(),
        "the vectors Centres was made from have been deleted"
    );
    let (doc, tp) = job();
    assert_eq!(
        Registry::default()
            .recalculate(&doc, tp)
            .unwrap_err()
            .to_string(),
        "Centres was made by `centres`, which is not available"
    );
}

#[test]
fn a_toolpath_made_from_no_vectors_recalculates() {
    // as 3D, rotary and picture toolpaths are: Centres with nothing selected makes no moves
    let mut doc = Document::default();
    doc.apply(&Centres::run(&doc, &NoParams {}).unwrap())
        .unwrap();
    let id = *doc.toolpaths.keys().next().unwrap();
    assert!(registry().recalculate(&doc, id).is_ok());
}

#[test]
fn a_toolpath_is_stale_once_its_own_vectors_change_until_recalculated() {
    let (mut doc, tp) = job();
    let mut history = History::default();
    let moved = |doc: &Document, id: EntityId| {
        let mut c = Change::builder(doc);
        c.replace(
            id,
            doc.vectors[&id].transformed(&DAffine2::from_translation(Point::new(0.0, 9.0))),
        );
        c.finish()
    };
    let change = moved(&doc, EntityId(2));
    history.apply(&mut doc, change).unwrap();
    assert!(!doc.toolpaths[&tp].stale, "not made from the other square");
    let change = moved(&doc, EntityId(1));
    history.apply(&mut doc, change).unwrap();
    assert!(doc.toolpaths[&tp].stale);
    let change = registry().recalculate(&doc, tp).unwrap();
    history.apply(&mut doc, change).unwrap();
    assert!(!doc.toolpaths[&tp].stale, "fresh once recalculated");
    history.undo(&mut doc);
    assert!(
        doc.toolpaths[&tp].stale,
        "and stale again when that is undone"
    );
    history.undo(&mut doc);
    assert!(
        !doc.toolpaths[&tp].stale,
        "undoing the move undoes the mark"
    );
}

#[test]
fn changing_the_model_makes_only_the_toolpaths_that_cut_it_stale() {
    let mut doc = with_toolpath();
    let flat = EntityId(1);
    let mut finishing = doc.toolpaths[&flat].clone();
    finishing.strategy = "3d_finishing_toolpath".into();
    let mut projected = doc.toolpaths[&flat].clone();
    projected.params = serde_json::json!({ "project_onto_model": true });
    let mut c = Change::builder(&doc);
    let finishing = c.add_toolpath(finishing);
    let projected = c.add_toolpath(projected);
    let change = c.finish();
    doc.apply(&change).unwrap();
    let component = two_components().components.values().next().unwrap().clone();
    let mut history = History::default();
    let mut c = Change::builder(&doc);
    c.add_component(component);
    let change = c.finish();
    history.apply(&mut doc, change).unwrap();
    assert!(doc.toolpaths[&finishing].stale);
    assert!(doc.toolpaths[&projected].stale);
    assert!(
        !doc.toolpaths[&flat].stale,
        "a flat toolpath does not cut the model"
    );
}
