use super::ToolpathPick;
use cut_craft_core::document::EntityId;

const A: EntityId = EntityId(1);
const B: EntityId = EntityId(2);
const C: EntityId = EntityId(3);

fn picked(pick: &ToolpathPick) -> Vec<EntityId> {
    [A, B, C]
        .into_iter()
        .filter(|&id| pick.is_picked(id))
        .collect()
}

#[test]
fn nothing_is_picked_at_first() {
    let pick = ToolpathPick::default();
    assert_eq!(pick.picked(), None);
    assert_eq!(picked(&pick), vec![]);
    assert!(!pick.del_deletes_it());
}

#[test]
fn ctrl_clicks_add_and_take_away() {
    let mut pick = ToolpathPick::default();
    pick.ctrl_click(A);
    pick.ctrl_click(B);
    pick.ctrl_click(C);
    assert_eq!(pick.picked(), Some(A));
    assert_eq!(picked(&pick), vec![A, B, C]);
    pick.ctrl_click(B);
    assert_eq!(picked(&pick), vec![A, C]);
    // unpicking the first hands its place to the next
    pick.ctrl_click(A);
    assert_eq!(pick.picked(), Some(C));
    assert_eq!(picked(&pick), vec![C]);
}

#[test]
fn a_click_picks_one_alone_and_a_second_click_unpicks_it() {
    let mut pick = ToolpathPick::default();
    pick.ctrl_click(A);
    pick.ctrl_click(B);
    pick.click(B, false);
    assert_eq!(pick.picked(), Some(B));
    assert_eq!(picked(&pick), vec![B]);
    pick.click(B, true);
    assert_eq!(pick.picked(), None);
    // with only one picked, a click on another picks that one
    pick.only(Some(A));
    pick.click(C, true);
    assert_eq!(picked(&pick), vec![C]);
}

#[test]
fn also_adds_without_taking_away() {
    let mut pick = ToolpathPick::default();
    pick.also(A);
    pick.also(B);
    pick.also(A);
    assert_eq!(pick.picked(), Some(A));
    assert_eq!(picked(&pick), vec![A, B]);
}

#[test]
fn picks_follow_their_toolpaths_to_new_places() {
    let mut pick = ToolpathPick::default();
    pick.only(Some(A));
    pick.also(B);
    // A went to C's place, B is gone
    let moved = pick.moved(|id| (id == A).then_some(C));
    assert_eq!(moved.picked(), Some(C));
    assert_eq!(picked(&moved), vec![C]);
}

#[test]
fn del_deletes_the_toolpath_only_just_after_a_row_was_clicked() {
    let mut pick = ToolpathPick::default();
    pick.list_clicked(true);
    assert!(!pick.del_deletes_it(), "nothing picked");
    pick.only(Some(A));
    assert!(pick.del_deletes_it());
    pick.list_clicked(false);
    assert!(!pick.del_deletes_it(), "something else was pressed");
}

#[test]
fn taking_the_picked_toolpath_leaves_the_rest() {
    let mut pick = ToolpathPick::default();
    pick.only(Some(A));
    pick.also(B);
    assert_eq!(pick.take(), Some(A));
    assert_eq!(pick.picked(), None);
    assert_eq!(picked(&pick), vec![B]);
}
