use super::SavingChecks;

fn problems(of: &[&str]) -> Vec<String> {
    of.iter().map(|p| (*p).to_owned()).collect()
}

#[test]
fn a_save_anyway_answer_is_handed_to_the_next_save_only() {
    let mut saving = SavingChecks::default();
    saving.accept(problems(&["Air cutting"]));
    assert_eq!(saving.start(), problems(&["Air cutting"]));
    assert!(saving.start().is_empty(), "only for that save");
}

#[test]
fn a_forgotten_answer_is_not_handed_over() {
    let mut saving = SavingChecks::default();
    saving.accept(problems(&["Too deep"]));
    saving.forget();
    assert_eq!(saving.accepted(), none());
    assert_eq!(saving.start(), none());
}

#[test]
fn the_errors_gone_ahead_with_last_until_the_next_save_starts() {
    let mut saving = SavingChecks::default();
    saving.go(problems(&["Too deep"]), (3, 0));
    assert_eq!(saving.errors(), problems(&["Too deep"]));
    saving.start();
    assert_eq!(saving.errors(), none());
}

#[test]
fn a_change_after_the_checks_stops_the_save() {
    let mut saving = SavingChecks::default();
    assert!(saving.still_as_checked((1, 0)).is_ok(), "not checked yet");
    saving.go(Vec::new(), (3, 0));
    assert!(saving.still_as_checked((3, 0)).is_ok());
    assert!(saving.still_as_checked((4, 0)).is_err(), "the job changed");
    assert!(saving.still_as_checked((3, 1)).is_err(), "another machine");
    saving.start();
    assert!(
        saving.still_as_checked((4, 0)).is_ok(),
        "a new save checks again"
    );
}

fn none() -> Vec<String> {
    Vec::new()
}
