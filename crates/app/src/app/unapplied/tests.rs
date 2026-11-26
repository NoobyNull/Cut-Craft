use super::Unapplied;

#[test]
fn changes_wait_until_taken() {
    let mut window = Unapplied::default();
    window.edited(&1, 2);
    assert!(window.dirty());
    assert_eq!(window.shown(&1), &2, "it shows the change");
    assert_eq!(window.take_changes(), Some(2));
    assert!(!window.dirty(), "Apply used them");
    assert_eq!(window.shown(&2), &2);
}

#[test]
fn editing_back_to_the_settings_as_they_are_leaves_nothing_waiting() {
    let mut window = Unapplied::default();
    window.edited(&1, 2);
    window.edited(&1, 1);
    assert!(!window.dirty());
}

#[test]
fn closing_throws_away_what_was_not_applied_and_the_question() {
    let mut window = Unapplied {
        open: true,
        ..Unapplied::default()
    };
    window.edited(&1, 2);
    window.closing = true;
    window.close();
    assert!(!window.open);
    assert!(!window.dirty());
    assert!(!window.closing);
}
