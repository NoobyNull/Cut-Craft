//! Announcements: at start in place of the tip, once a day, Don't show this again, and
//! Help ▸ Announcements.

use super::*;

const HEADS_UP: &str = "Cut Craft is becoming PowerWhittle";

#[expect(
    clippy::unnecessary_wraps,
    reason = "it stands in for GitHub, which can fail"
)]
fn open_day() -> Result<String, String> {
    Ok(
        "How to write one.\n---\nid: open-day\ntitle: Workshop open day\n\
        from: 2026-12-05\nuntil: 2026-12-06\n---\nCome and **see the machines**.\n"
            .to_owned(),
    )
}

fn offline() -> Result<String, String> {
    Err("Could not reach GitHub".to_owned())
}

#[test]
fn a_note_from_github_opens_at_the_first_start_of_its_days_in_place_of_the_tip() {
    let mut h = Harness::at_start();
    h.given_announcements("2026-12-05", open_day);
    h.start_cut_craft();
    assert_eq!(
        h.announcements_shown(),
        Some(vec![HEADS_UP.to_owned(), "Workshop open day".to_owned()]),
        "the bundled one, then GitHub's"
    );
    assert_eq!(h.tip(), None, "no tip as well");
    assert!(h.text_at("Workshop open day").is_some());
    // started again the same day: the tip, not the announcements
    h.start_cut_craft();
    assert_eq!(h.announcements_shown(), None);
    assert!(h.tip().is_some(), "the tip once GitHub has answered");
    // the next day it comes back
    h.given_announcements("2026-12-06", open_day);
    h.start_cut_craft();
    assert_eq!(h.announcements_shown().map(|t| t.len()), Some(2));
    // and after its days only the bundled one is left, already seen... until it, too, ends
    h.given_announcements("2026-12-07", open_day);
    h.start_cut_craft();
    assert_eq!(h.announcements_shown(), Some(vec![HEADS_UP.to_owned()]));
    h.given_announcements("2027-01-01", open_day);
    h.start_cut_craft();
    assert_eq!(
        h.announcements_shown(),
        None,
        "past every announcement's days"
    );
}

#[test]
fn dont_show_this_again_stops_one_and_help_still_shows_it() {
    let mut h = Harness::at_start();
    h.given_announcements("2026-12-05", open_day);
    h.start_cut_craft();
    // tick Don't show this again under GitHub's note (the second)
    let mut boxes = h.texts_at("Don't show this again");
    assert_eq!(boxes.len(), 2, "one under each");
    let second = boxes.pop().unwrap();
    h.click(second, Modifiers::NONE);
    h.frame(vec![]);
    h.given_announcements("2026-12-06", open_day);
    h.start_cut_craft();
    assert_eq!(h.announcements_shown(), Some(vec![HEADS_UP.to_owned()]));
    // Help ▸ Announcements lists every current one
    h.choose_on_menu_bar("Help", &["Announcements..."]);
    h.until_github_answers();
    assert_eq!(
        h.announcements_shown(),
        Some(vec![HEADS_UP.to_owned(), "Workshop open day".to_owned()])
    );
}

#[test]
fn offline_or_turned_off_shows_only_what_came_with_the_version_or_nothing() {
    let mut h = Harness::at_start();
    h.given_announcements("2026-12-05", offline);
    h.start_cut_craft();
    assert_eq!(h.announcements_shown(), Some(vec![HEADS_UP.to_owned()]));
    h.choose_on_menu_bar("Help", &["Announcements..."]);
    h.until_github_answers();
    assert!(
        h.text_at("GitHub could not be reached, so only the announcements that came with this version are shown.")
            .is_some()
    );
    let mut h = Harness::at_start();
    h.given_announcements("2026-12-05", open_day);
    h.given_options(|o| o.updates.show_announcements = false);
    h.start_cut_craft();
    assert_eq!(h.announcements_shown(), None);
    assert!(h.tip().is_some(), "the tip as before");
}

#[test]
fn not_over_first_time_setup() {
    let mut h = Harness::at_start();
    h.given_announcements("2026-12-05", open_day);
    h.open_first_run();
    h.start_cut_craft();
    assert_eq!(h.announcements_shown(), None);
}
