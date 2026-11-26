use super::*;

#[test]
fn a_kept_value_holds_only_for_what_it_was_made_from() {
    let mut kept = Kept::default();
    assert_eq!(kept.get(&1), None);
    assert_eq!(kept.keep(1, "one"), &"one");
    assert_eq!(kept.get(&1), Some(&"one"));
    assert_eq!(kept.get(&2), None, "made from something else");
    assert_eq!(kept.key(), Some(&1));
}

#[test]
fn keeping_another_puts_it_in_place_of_the_last() {
    let mut kept = Kept::default();
    kept.keep(1, "one");
    kept.keep(2, "two");
    assert_eq!((kept.get(&1), kept.get(&2)), (None, Some(&"two")));
}

#[test]
fn a_cleared_value_is_made_again() {
    let mut kept = Kept::default();
    kept.keep(1, "one");
    kept.clear();
    assert_eq!((kept.get(&1), kept.key()), (None, None));
}
