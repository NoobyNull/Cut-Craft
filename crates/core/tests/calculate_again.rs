//! A toolpath form's Calculate pressed again: what it makes goes in place of what it made
//! last time, so the job never gets a second copy.

use cut_craft_core::Change;
use cut_craft_core::document::{Document, EntityId};
use cut_craft_core::toolpath::{Tool, Toolpath};

fn toolpath(name: &str, group: Option<&str>) -> Toolpath {
    Toolpath {
        name: name.into(),
        strategy: "pocket_toolpath".into(),
        params: serde_json::json!({ "name": name }),
        tool: Tool::default(),
        vectors: vec![],
        moves: vec![],
        visible: true,
        power: vec![],
        stale: false,
        group: group.map(str::to_owned),
        drill_bank: None,
        fallbacks: vec![],
    }
}

/// A job with toolpaths named `names`, the first in group "Signs".
fn job(names: &[&str]) -> (Document, Vec<EntityId>) {
    let mut doc = Document::default();
    let mut c = Change::builder(&doc);
    let ids = names
        .iter()
        .enumerate()
        .map(|(i, n)| c.add_toolpath(toolpath(n, (i == 0).then_some("Signs"))))
        .collect();
    doc.apply(&c.finish()).unwrap();
    (doc, ids)
}

fn made(doc: &Document, names: &[&str]) -> Change {
    let mut c = Change::builder(doc);
    for n in names {
        c.add_toolpath(toolpath(n, None));
    }
    c.finish()
}

#[test]
fn a_toolpath_made_again_takes_the_place_and_group_of_the_one_before() {
    let (mut doc, ids) = job(&["Pocket 1", "Profile"]);
    let change = made(&doc, &["Pocket 2"]).onto_toolpaths(&ids[..1], &doc);
    assert_eq!(change.toolpaths_made(), vec![ids[0]]);
    doc.apply(&change).unwrap();
    assert_eq!(doc.toolpaths.len(), 2, "replaced, not added");
    let first = &doc.toolpaths[&ids[0]];
    assert_eq!(first.name, "Pocket 2");
    assert_eq!(first.group.as_deref(), Some("Signs"), "stays in its group");
    assert_eq!(doc.toolpaths[&ids[1]].name, "Profile", "others untouched");
}

#[test]
fn more_made_than_before_adds_the_rest_and_fewer_takes_the_rest_away() {
    let (doc, ids) = job(&["Pocket", "Clearing"]);
    // a clearing bit added: three where there were two
    let mut more = doc.clone();
    more.apply(&made(&doc, &["A", "B", "C"]).onto_toolpaths(&ids, &doc))
        .unwrap();
    let names: Vec<_> = more.toolpaths.values().map(|t| t.name.as_str()).collect();
    assert_eq!(names, ["A", "B", "C"]);
    // the clearing bit taken out: one where there were two
    let mut fewer = doc.clone();
    fewer
        .apply(&made(&doc, &["Only"]).onto_toolpaths(&ids, &doc))
        .unwrap();
    assert_eq!(fewer.toolpaths.len(), 1);
    assert_eq!(fewer.toolpaths[&ids[0]].name, "Only");
}

#[test]
fn a_toolpath_deleted_since_is_not_put_back_in_place() {
    let (doc, ids) = job(&["Pocket"]);
    let gone = Document::default();
    let change = made(&gone, &["Again"]).onto_toolpaths(&ids, &gone);
    let mut fresh = Document::default();
    fresh.apply(&change).unwrap();
    assert_eq!(fresh.toolpaths.len(), 1, "made new");
    assert_eq!(doc.toolpaths.len(), 1);
}
