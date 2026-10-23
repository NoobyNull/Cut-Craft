use cut_craft_core::document::{Document, EntityId};
use cut_craft_core::geometry::Point;
use cut_craft_core::glam::{DAffine2, DVec3};
use cut_craft_core::testing::{document_with, square};
use cut_craft_core::toolpath::{Move, MoveKind, Tool, made_by};
use cut_craft_core::{Change, Command, CommandError, Meta, NoParams, Registry};

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
