use super::{List, RailList};

#[test]
fn one_list_is_open_at_a_time() {
    let mut rail = RailList::default();
    rail.toggle(List::Group(2));
    assert_eq!(rail.group(), Some(2));
    rail.toggle(List::Favourites);
    assert_eq!(rail.open, List::Favourites, "in place of the group");
    assert_eq!(rail.group(), None);
    rail.toggle(List::Gadgets);
    assert_eq!(rail.open, List::Gadgets);
}

#[test]
fn pressing_the_open_list_s_button_again_closes_it() {
    let mut rail = RailList::default();
    rail.toggle(List::Group(1));
    rail.toggle(List::Group(1));
    assert!(!rail.is_open());
    rail.toggle(List::Group(1));
    rail.toggle(List::Group(3));
    assert_eq!(rail.group(), Some(3), "another group opens in its place");
}

#[test]
fn a_pick_closes_the_list_unless_pinned() {
    let mut rail = RailList::default();
    rail.toggle(List::Favourites);
    rail.pinned = true;
    assert!(!rail.picked());
    assert!(rail.is_open(), "pinned");
    rail.pinned = false;
    assert!(rail.picked());
    assert!(!rail.is_open());
}

#[test]
fn the_gadgets_list_closes_once_nothing_is_installed() {
    let mut rail = RailList::default();
    rail.toggle(List::Favourites);
    rail.keep_gadgets(false);
    assert_eq!(rail.open, List::Favourites, "only the gadgets' list closes");
    rail.toggle(List::Gadgets);
    rail.keep_gadgets(true);
    assert_eq!(rail.open, List::Gadgets);
    rail.keep_gadgets(false);
    assert!(!rail.is_open());
}
