use cut_craft_core::document::{Document, EntityId};
use cut_craft_core::file::{load_job, save_job};
use cut_craft_core::geometry::Point;
use cut_craft_core::glam::DVec3;
use cut_craft_core::toolpath::{Move, MoveKind, Tool, Toolpath, contour_moves, pass_depths};
use cut_craft_core::{Change, History};

fn toolpath(name: &str) -> Toolpath {
    Toolpath {
        name: name.into(),
        strategy: "profile_toolpath".into(),
        params: serde_json::json!({ "depth": 3.0 }),
        tool: Tool::default(),
        vectors: vec![EntityId(1)],
        moves: vec![
            Move {
                kind: MoveKind::Rapid,
                to: DVec3::new(0.0, 0.0, 5.0),
            },
            Move {
                kind: MoveKind::Plunge,
                to: DVec3::new(0.0, 0.0, -3.0),
            },
            Move {
                kind: MoveKind::Cut,
                to: DVec3::new(10.0, 0.0, -3.0),
            },
        ],
        visible: true,
    }
}

#[test]
fn toolpaths_are_added_undone_and_ids_never_clash_with_vectors() {
    let mut doc = cut_craft_core::testing::sample_document();
    let mut history = History::default();
    let mut c = Change::builder(&doc);
    let id = c.add_toolpath(toolpath("Profile 1"));
    let change = c.finish();
    history.apply(&mut doc, change).unwrap();
    assert!(
        !doc.vectors.contains_key(&id),
        "toolpath ids are unique across the document"
    );
    assert_eq!(doc.toolpaths[&id].name, "Profile 1");
    assert_eq!(doc.next_id(), EntityId(id.0 + 1));
    history.undo(&mut doc);
    assert!(doc.toolpaths.is_empty());
}

#[test]
fn replacing_and_removing_toolpaths_round_trip() {
    let mut doc = Document::default();
    let mut c = Change::builder(&doc);
    let id = c.add_toolpath(toolpath("A"));
    doc.apply(&c.finish()).unwrap();
    let before = doc.clone();
    let mut c = Change::builder(&doc);
    c.replace_toolpath(id, toolpath("B"));
    let rename = c.finish();
    doc.apply(&rename).unwrap();
    assert_eq!(doc.toolpaths[&id].name, "B");
    let mut c = Change::builder(&doc);
    c.remove_toolpath(id);
    let remove = c.finish();
    doc.apply(&remove).unwrap();
    doc.apply(&remove.inverse()).unwrap();
    doc.apply(&rename.inverse()).unwrap();
    assert_eq!(doc, before);
}

#[test]
fn jobs_saved_before_toolpaths_existed_still_open() {
    let mut json: serde_json::Value =
        serde_json::from_slice(&save_job(&Document::default())).unwrap();
    json["document"]
        .as_object_mut()
        .unwrap()
        .remove("toolpaths");
    assert!(
        load_job(&serde_json::to_vec(&json).unwrap())
            .unwrap()
            .toolpaths
            .is_empty()
    );
}

#[test]
fn the_default_tool_is_a_6mm_end_mill() {
    let t = Tool::default();
    assert_eq!(t.diameter, 6.0);
    assert!(
        t.feed_rate > 0.0 && t.plunge_rate > 0.0 && t.pass_depth > 0.0 && t.spindle_speed > 0.0
    );
}

#[test]
fn passes_go_down_in_equal_steps_no_deeper_than_the_pass_depth() {
    let d = pass_depths(7.0, 3.0);
    assert_eq!(d.len(), 3);
    assert!((d[0] + 7.0 / 3.0).abs() < 1e-12 && (d[2] + 7.0).abs() < 1e-12);
    assert_eq!(pass_depths(3.0, 3.0), [-3.0]);
    assert_eq!(pass_depths(0.5, 3.0), [-0.5]);
}

fn square() -> Vec<Point> {
    vec![
        Point::new(0.0, 0.0),
        Point::new(10.0, 0.0),
        Point::new(10.0, 10.0),
        Point::new(0.0, 10.0),
    ]
}

#[test]
fn a_closed_contour_plunges_straight_to_each_next_pass() {
    let moves = contour_moves(&square(), true, &[-1.0, -2.0], 5.0);
    assert_eq!(
        moves[0],
        Move {
            kind: MoveKind::Rapid,
            to: DVec3::new(0.0, 0.0, 5.0)
        }
    );
    assert_eq!(
        moves[1],
        Move {
            kind: MoveKind::Plunge,
            to: DVec3::new(0.0, 0.0, -1.0)
        }
    );
    let kinds: Vec<MoveKind> = moves.iter().map(|m| m.kind).collect();
    assert_eq!(kinds.iter().filter(|k| **k == MoveKind::Plunge).count(), 2);
    assert_eq!(
        kinds.iter().filter(|k| **k == MoveKind::Cut).count(),
        8,
        "4 sides x 2 passes"
    );
    assert_eq!(
        *moves.last().unwrap(),
        Move {
            kind: MoveKind::Rapid,
            to: DVec3::new(0.0, 0.0, 5.0)
        },
        "ends retracted"
    );
}

#[test]
fn an_open_contour_retracts_and_returns_between_passes() {
    let line = [Point::new(0.0, 0.0), Point::new(10.0, 0.0)];
    let moves = contour_moves(&line, false, &[-1.0, -2.0], 5.0);
    let rapids_up = moves
        .iter()
        .filter(|m| m.kind == MoveKind::Rapid && (m.to.z - 5.0).abs() < 1e-12)
        .count();
    assert_eq!(rapids_up, 4, "approach, retract, return, final retract");
}

#[test]
fn the_job_has_a_clearance_height_that_old_files_default() {
    let mut json: serde_json::Value =
        serde_json::from_slice(&save_job(&Document::default())).unwrap();
    json["document"]["job"]
        .as_object_mut()
        .unwrap()
        .remove("clearance");
    assert_eq!(
        load_job(&serde_json::to_vec(&json).unwrap())
            .unwrap()
            .job
            .clearance,
        5.0
    );
}

#[test]
fn selected_paths_filters_the_selection() {
    let doc = cut_craft_core::testing::sample_document();
    let (ids, paths) = doc.selected_paths(|p| p.closed);
    assert_eq!(ids.len(), 3);
    assert!(paths.iter().all(|p| p.closed));
}

#[test]
fn cut_points_drop_the_repeated_closing_point() {
    let square = cut_craft_core::testing::square(0.0, 0.0, 10.0);
    let pts = cut_craft_core::toolpath::cut_points(&square);
    assert_eq!(pts.len(), 4);
    assert_ne!(pts.first(), pts.last());
}

#[test]
fn toolpaths_start_visible_and_old_ones_load_visible() {
    let t = toolpath("t");
    assert!(t.visible);
    let mut json = serde_json::to_value(&t).unwrap();
    json.as_object_mut().unwrap().remove("visible");
    let old: Toolpath = serde_json::from_value(json).unwrap();
    assert!(old.visible);
}
