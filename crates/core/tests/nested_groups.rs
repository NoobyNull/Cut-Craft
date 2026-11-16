//! Groups inside groups: grouping keeps the groups it takes in (text stays text, part names
//! stay), and stepping inside a group picks single items in it.

use cut_craft_core::document::{Document, EntityId};
use cut_craft_core::testing::{document_with, square};
use cut_craft_core::{Change, History};

/// Four squares: 1 and 2 are the text "AB" (group 7), 3 is a border, 4 stands alone.
fn text_and_border() -> Document {
    let mut doc = document_with(vec![
        square(0.0, 0.0, 5.0),
        square(10.0, 0.0, 5.0),
        square(-10.0, -10.0, 40.0),
        square(100.0, 0.0, 5.0),
    ]);
    let mut c = Change::builder(&doc);
    c.set_group(EntityId(1), Some(7));
    c.set_group(EntityId(2), Some(7));
    c.set_text(7, Some(serde_json::json!({"text": "AB"})));
    c.set_part_name(7, Some("Sign".into()));
    doc.apply(&c.finish()).unwrap();
    doc
}

/// The text and the border grouped as group 9 (the text's group inside it).
fn grouped() -> Document {
    let mut doc = text_and_border();
    let mut c = Change::builder(&doc);
    c.set_group_parent(7, Some(9));
    c.set_group(EntityId(3), Some(9));
    doc.apply(&c.finish()).unwrap();
    doc
}

#[test]
fn a_group_inside_a_group_is_picked_whole_from_outside() {
    let doc = grouped();
    let mut members = doc.group_members(EntityId(1));
    members.sort();
    assert_eq!(members, [EntityId(1), EntityId(2), EntityId(3)]);
    assert_eq!(doc.top_group(EntityId(1)), Some(9));
    assert_eq!(doc.top_group(EntityId(3)), Some(9));
    assert_eq!(doc.top_group(EntityId(4)), None);
    assert_eq!(doc.group_chain(EntityId(1)), [7, 9]);
}

#[test]
fn text_inside_a_group_is_still_text_and_keeps_its_part_name() {
    let doc = grouped();
    assert_eq!(doc.text_group(EntityId(2)), Some(7));
    assert_eq!(doc.text_group(EntityId(3)), None);
    assert_eq!(doc.part_name(EntityId(1)), Some("Sign"));
}

#[test]
fn inside_a_group_a_click_picks_the_item_one_level_down() {
    let doc = grouped();
    // inside group 9: a letter brings the whole text, the border comes alone
    let mut text = doc.pick_inside(EntityId(1), Some(9)).unwrap();
    text.sort();
    assert_eq!(text, [EntityId(1), EntityId(2)]);
    assert_eq!(
        doc.pick_inside(EntityId(3), Some(9)).unwrap(),
        [EntityId(3)]
    );
    // inside the text itself, one letter
    assert_eq!(
        doc.pick_inside(EntityId(2), Some(7)).unwrap(),
        [EntityId(2)]
    );
    // something outside the group is not picked from inside it
    assert_eq!(doc.pick_inside(EntityId(4), Some(9)), None);
    // from outside every group, the whole group
    assert_eq!(doc.pick_inside(EntityId(3), None).unwrap().len(), 3);
}

#[test]
fn the_group_to_step_into_is_the_one_under_where_you_are() {
    let doc = grouped();
    assert_eq!(doc.group_to_enter(EntityId(1), None), Some(9));
    assert_eq!(doc.group_to_enter(EntityId(1), Some(9)), Some(7));
    assert_eq!(
        doc.group_to_enter(EntityId(3), Some(9)),
        None,
        "a lone item"
    );
    assert_eq!(doc.group_to_enter(EntityId(4), None), None);
}

#[test]
fn one_item_picked_inside_a_group_moves_alone() {
    let mut doc = grouped();
    let mut c = Change::builder(&doc);
    c.select([EntityId(3)]);
    doc.apply(&c.finish()).unwrap();
    assert_eq!(doc.selected_items(false), [vec![EntityId(3)]]);
    // the whole text picked inside the group is one item; the border is another
    let mut c = Change::builder(&doc);
    c.select([EntityId(1), EntityId(2), EntityId(3)]);
    doc.apply(&c.finish()).unwrap();
    assert_eq!(
        doc.selected_items(false),
        [vec![EntityId(1), EntityId(2), EntityId(3)]],
        "the whole group is one item"
    );
    let mut c = Change::builder(&doc);
    c.select([EntityId(1), EntityId(2), EntityId(4)]);
    doc.apply(&c.finish()).unwrap();
    assert_eq!(
        doc.selected_items(false),
        [vec![EntityId(1), EntityId(2)], vec![EntityId(4)]]
    );
}

#[test]
fn setting_a_groups_parent_undoes_and_saves() {
    let mut doc = text_and_border();
    let mut history = History::default();
    let mut c = Change::builder(&doc);
    c.set_group_parent(7, Some(9));
    c.set_group(EntityId(3), Some(9));
    let change = c.finish();
    history.apply(&mut doc, change).unwrap();
    let saved: Document = serde_json::from_str(&serde_json::to_string(&doc).unwrap()).unwrap();
    assert_eq!(saved.group_chain(EntityId(1)), [7, 9]);
    assert!(history.undo(&mut doc));
    assert_eq!(doc.group_chain(EntityId(1)), [7]);
    assert!(doc.group_parents.is_empty());
}

#[test]
fn a_new_group_number_is_not_one_a_parent_uses() {
    let doc = grouped();
    assert!(doc.next_group() > 9);
    let mut doc = text_and_border();
    doc.group_parents.insert(7, 30);
    assert!(doc.next_group() > 30);
}
