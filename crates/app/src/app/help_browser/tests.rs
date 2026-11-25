use super::*;

fn page(topic: &'static str, title: &str, md: &str) -> (&'static str, String, String) {
    (topic, title.to_owned(), md.to_owned())
}

#[test]
fn a_clicked_page_can_be_gone_back_to_and_forward_again() {
    let mut help = HelpBrowser::default();
    assert_eq!(help.can_step(), (false, false));
    assert_eq!(help.go("contents", None, Some("pocket")), Some("pocket"));
    assert_eq!(help.can_step(), (true, false));
    assert_eq!(help.go("pocket", Some(Step::Back), None), Some("contents"));
    assert_eq!(help.can_step(), (false, true));
    assert_eq!(
        help.go("contents", Some(Step::Forward), None),
        Some("pocket")
    );
}

#[test]
fn clicking_the_page_shown_keeps_no_history() {
    let mut help = HelpBrowser::default();
    assert_eq!(help.go("pocket", None, Some("pocket")), Some("pocket"));
    assert_eq!(help.go("pocket", None, None), None);
    assert_eq!(help.can_step(), (false, false));
}

#[test]
fn a_search_finds_the_page_saying_the_words_with_its_line() {
    let mut help = HelpBrowser::default();
    assert!(help.needs_pages());
    help.index_pages(vec![
        page(
            "pocket",
            "Pocket",
            "# Pocket\nClears the inside of a shape to a depth.",
        ),
        page(
            "profile",
            "Profile",
            "# Profile\nCuts round the outline, with tabs.",
        ),
    ]);
    assert!(!help.needs_pages());
    let found = help.results("tabs");
    assert_eq!(found.first().map(|(t, ..)| *t), Some("profile"));
    assert!(found[0].2.contains("tabs"), "{found:?}");
}

#[test]
fn new_pages_are_ranked_afresh() {
    let mut help = HelpBrowser::default();
    help.index_pages(vec![page("pocket", "Pocket", "Clears the inside.")]);
    assert_eq!(help.results("outline"), vec![]);
    help.index_pages(vec![page("profile", "Profile", "Cuts round the outline.")]);
    assert_eq!(help.results("outline").len(), 1);
}
