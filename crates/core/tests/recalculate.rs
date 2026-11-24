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
        "The vectors Centres was made from have been deleted. Select new vectors, then edit \
         Centres and press Calculate."
    );
    let (doc, tp) = job();
    assert_eq!(
        Registry::default()
            .recalculate(&doc, tp)
            .unwrap_err()
            .to_string(),
        "Centres was made by `centres`, which is not available. Install the add-on that made \
         it, or delete this toolpath."
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
    let mut rotary = doc.toolpaths[&flat].clone();
    rotary.strategy = "rotary_machining_toolpath".into();
    let mut lathe = doc.toolpaths[&flat].clone();
    lathe.strategy = "rotary_lathe_toolpath".into();
    let mut c = Change::builder(&doc);
    let finishing = c.add_toolpath(finishing);
    let projected = c.add_toolpath(projected);
    let rotary = c.add_toolpath(rotary);
    let lathe = c.add_toolpath(lathe);
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
        doc.toolpaths[&rotary].stale,
        "4-Axis Rotary Machining cuts the model"
    );
    assert!(doc.toolpaths[&lathe].stale, "Rotary Lathe turns the model");
    assert!(
        !doc.toolpaths[&flat].stale,
        "a flat toolpath does not cut the model"
    );
}

#[test]
fn recalculating_each_says_how_far_it_has_got_and_stops_when_cancelled() {
    use cut_craft_core::progress::Progress;
    let (mut doc, first) = job();
    let mut c = Change::builder(&doc);
    c.select([EntityId(2)]);
    doc.apply(&c.finish()).unwrap();
    doc.apply(&Centres::run(&doc, &NoParams {}).unwrap())
        .unwrap();
    let ids: Vec<EntityId> = doc.toolpaths.keys().copied().collect();
    move_vector(&mut doc, EntityId(1), Point::new(0.0, 20.0));
    let progress = Progress::new();
    let change = progress
        .track(|| registry().recalculate_each(&doc, &ids))
        .unwrap();
    assert_eq!(progress.fraction(), Some(1.0));
    assert_eq!(progress.stage(), "Centres (2 of 2)", "named as it goes");
    doc.apply(&change).unwrap();
    assert_eq!(plunge(&doc, first), Point::new(5.0, 25.0));
    let stopped = Progress::new();
    stopped.cancel();
    assert!(
        stopped
            .track(|| registry().recalculate_each(&doc, &ids))
            .is_err(),
        "a cancelled recalculation makes nothing"
    );
}

#[test]
fn a_toolpath_following_a_vector_selector_is_stale_once_new_vectors_pass_it() {
    use cut_craft_core::geometry::Path;
    use cut_craft_core::selector::Selector;
    let (mut doc, tp) = job();
    let open_only = Selector {
        open_vectors: true,
        closed_vectors: false,
        ..Selector::default()
    };
    let mut toolpath = doc.toolpaths[&tp].clone();
    toolpath.params = serde_json::json!({ "selector": open_only });
    let mut c = Change::builder(&doc);
    c.replace_toolpath(tp, toolpath);
    doc.apply(&c.finish()).unwrap();
    let mut history = History::default();
    let add = |doc: &Document, path: Path| {
        let mut c = Change::builder(doc);
        c.add(path);
        c.finish()
    };
    let change = add(&doc, square(100.0, 0.0, 10.0));
    history.apply(&mut doc, change).unwrap();
    assert!(
        !doc.toolpaths[&tp].stale,
        "a closed vector does not pass it"
    );
    let line = Path::polyline(&[Point::new(0.0, 30.0), Point::new(20.0, 30.0)], false);
    let change = add(&doc, line);
    history.apply(&mut doc, change).unwrap();
    assert!(doc.toolpaths[&tp].stale, "an open one does");
    history.undo(&mut doc);
    assert!(!doc.toolpaths[&tp].stale, "undone with it");
}

#[test]
fn changing_the_material_or_where_the_model_sits_makes_the_toolpaths_stale() {
    let set = |doc: &Document, edit: &dyn Fn(&mut cut_craft_core::document::Job)| {
        let mut job = doc.job;
        edit(&mut job);
        let mut c = Change::builder(doc);
        c.set_job(job);
        c.finish()
    };
    let (doc, tp) = job();
    let thicker = |j: &mut cut_craft_core::document::Job| j.thickness += 6.0;
    let moved_model = |j: &mut cut_craft_core::document::Job| j.model_position.gap += 2.0;
    let recoloured = |j: &mut cut_craft_core::document::Job| j.colour = [1, 2, 3];
    for (what, edit, stale) in [
        (
            "the thickness",
            &thicker as &dyn Fn(&mut cut_craft_core::document::Job),
            true,
        ),
        ("the model's gap", &moved_model, true),
        ("only the colour", &recoloured, false),
    ] {
        let mut doc = doc.clone();
        let mut history = History::default();
        let change = set(&doc, edit);
        history.apply(&mut doc, change).unwrap();
        assert_eq!(doc.toolpaths[&tp].stale, stale, "changing {what}");
        if stale {
            history.undo(&mut doc);
            assert!(!doc.toolpaths[&tp].stale, "undone with {what}");
        }
    }
}

#[test]
fn a_merged_or_array_copy_toolpath_is_stale_once_a_toolpath_it_copies_changes() {
    let (mut doc, tp) = job();
    // a toolpath made from tp's moves, as Merged and Array Copy are
    let mut copy = doc.toolpaths[&tp].clone();
    copy.name = "Copy".into();
    copy.vectors = vec![];
    copy.params = serde_json::json!({ "sources": [tp] });
    let mut c = Change::builder(&doc);
    c.add_toolpath(copy);
    let change = c.finish();
    doc.apply(&change).unwrap();
    let copy = *doc.toolpaths.keys().last().unwrap();
    let mut history = History::default();
    // shown or hidden, the source cuts the same: the copy is not out of date
    let mut hidden = doc.toolpaths[&tp].clone();
    hidden.visible = false;
    let mut c = Change::builder(&doc);
    c.replace_toolpath(tp, hidden);
    let change = c.finish();
    history.apply(&mut doc, change).unwrap();
    assert!(
        !doc.toolpaths[&copy].stale,
        "hiding a source changes no cut"
    );
    // the source's vector moves: the source and so the copy are out of date
    let mut c = Change::builder(&doc);
    c.replace(
        EntityId(1),
        doc.vectors[&EntityId(1)].transformed(&DAffine2::from_translation(Point::new(0.0, 9.0))),
    );
    let change = c.finish();
    history.apply(&mut doc, change).unwrap();
    assert!(doc.toolpaths[&tp].stale);
    assert!(doc.toolpaths[&copy].stale, "the copy follows its source");
    history.undo(&mut doc);
    assert!(!doc.toolpaths[&copy].stale, "undone with it");
    // the source recalculated with new moves: the copy still has the old ones
    let mut source = doc.toolpaths[&tp].clone();
    source.moves[0].to.x += 5.0;
    let mut c = Change::builder(&doc);
    c.replace_toolpath(tp, source);
    let change = c.finish();
    history.apply(&mut doc, change).unwrap();
    assert!(doc.toolpaths[&copy].stale, "a source cut differently");
    // a source deleted
    history.undo(&mut doc);
    let mut c = Change::builder(&doc);
    c.remove_toolpath(tp);
    let change = c.finish();
    history.apply(&mut doc, change).unwrap();
    assert!(doc.toolpaths[&copy].stale, "a source deleted");
}

/// Test strategy: makes nothing.
struct Nothing;

impl Command for Nothing {
    const ID: &'static str = "nothing";
    type Params = NoParams;

    fn meta() -> Meta {
        Meta {
            label: "Nothing",
            group: "Test",
            icon: "",
            shortcut: "",
        }
    }

    fn run(doc: &Document, _: &NoParams) -> Result<Change, CommandError> {
        Ok(Change::builder(doc).finish())
    }
}

#[test]
fn a_toolpath_that_makes_nothing_again_is_explained_in_plain_words() {
    let (mut doc, tp) = job();
    doc.toolpaths.get_mut(&tp).unwrap().strategy = Nothing::ID.into();
    let mut r = Registry::default();
    r.add::<Nothing>();
    let why = r.recalculate(&doc, tp).unwrap_err().to_string();
    assert!(!why.contains('`') && !why.contains("nothing"), "{why}");
    assert!(why.contains("Centres"), "names the toolpath: {why}");
}

#[derive(Clone, Default, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
struct CopiesParams {
    sources: Vec<EntityId>,
}

/// Test strategy: a toolpath copying its sources' moves, as Merged and Array Copy do.
struct Copies;

impl Command for Copies {
    const ID: &'static str = "copies";
    type Params = CopiesParams;

    fn meta() -> Meta {
        Meta {
            label: "Copies",
            group: "Test",
            icon: "",
            shortcut: "",
        }
    }

    fn run(doc: &Document, p: &CopiesParams) -> Result<Change, CommandError> {
        let moves = p
            .sources
            .iter()
            .filter_map(|s| doc.toolpaths.get(s))
            .flat_map(|t| t.moves.clone())
            .collect();
        let mut c = Change::builder(doc);
        c.add_toolpath(made_by::<Self>(
            p,
            "Copy".into(),
            Tool::default(),
            vec![],
            moves,
        ));
        Ok(c.finish())
    }
}

#[test]
fn a_copy_recalculated_while_its_source_is_out_of_date_is_not_left_with_old_moves() {
    let mut r = registry();
    r.add::<Copies>();
    let (mut doc, tp) = job();
    doc.apply(&Copies::run(&doc, &CopiesParams { sources: vec![tp] }).unwrap())
        .unwrap();
    let copy = *doc.toolpaths.keys().last().unwrap();
    let mut history = History::default();
    let mut c = Change::builder(&doc);
    c.replace(
        EntityId(1),
        doc.vectors[&EntityId(1)].transformed(&DAffine2::from_translation(Point::new(0.0, 9.0))),
    );
    let change = c.finish();
    history.apply(&mut doc, change).unwrap();
    assert!(doc.toolpaths[&tp].stale && doc.toolpaths[&copy].stale);
    // Edit > Calculate on the copy alone: it copies the old moves, so it stays out of date
    let change = r.edit(&doc, copy, None).unwrap();
    history.apply(&mut doc, change).unwrap();
    assert!(
        doc.toolpaths[&copy].stale,
        "its source is still out of date"
    );
    history.undo(&mut doc);
    // recalculating the copy (Recalculate Visible, with the source hidden) recalculates the
    // out-of-date source first, and copies its new moves
    for ids in [vec![copy], vec![copy, tp]] {
        let change = r.recalculate_each(&doc, &ids).unwrap();
        history.apply(&mut doc, change).unwrap();
        assert_eq!(plunge(&doc, tp), Point::new(5.0, 14.0));
        assert_eq!(plunge(&doc, copy), Point::new(5.0, 14.0), "{ids:?}");
        assert!(!doc.toolpaths[&tp].stale && !doc.toolpaths[&copy].stale);
        history.undo(&mut doc);
    }
}
