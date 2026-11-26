use super::PhotoDepth;

#[test]
fn high_detail_is_on_once_the_model_is_here_until_unticked() {
    let mut depth = PhotoDepth::default();
    assert!(!depth.high_detail(false), "no model, no High detail");
    assert!(depth.high_detail(true));
    depth.set_high_detail(false);
    assert!(!depth.high_detail(true), "the user's choice stays");
}

#[test]
fn a_finished_download_turns_high_detail_on() {
    let mut depth = PhotoDepth::default();
    depth.set_high_detail(false);
    depth.fetched(Err("offline".into()));
    assert_eq!(depth.failed(), Some("offline"));
    depth.fetched(Ok(()));
    assert_eq!(depth.failed(), None, "the old failure goes");
    assert!(depth.high_detail(true));
}

#[test]
fn a_download_that_cannot_start_says_why() {
    let mut depth = PhotoDepth::default();
    depth.fetch_started(Err("no add-ons folder".into()));
    assert_eq!(depth.failed(), Some("no add-ons folder"));
    assert_eq!(depth.fetching(), None);
}

#[test]
fn nothing_is_answered_while_nothing_runs() {
    let mut depth = PhotoDepth::default();
    assert!(!depth.estimating());
    assert!(depth.take_answer().is_none());
}
