//! Did You Know?: the tips, at start and from the Help menu, and the tips page.

use super::*;

#[test]
fn did_you_know_steps_through_the_tips() {
    let mut h = Harness::at_start();
    assert_eq!(h.tip(), None, "tests start without it");
    // it opens on a tip not seen yet: here, only the first
    h.given_options(|o| {
        o.seen_tips = super::tips::TIPS[1..]
            .iter()
            .map(super::tips::Tip::key)
            .collect();
    });
    h.choose_on_menu_bar("Help", &["Did You Know?..."]);
    assert_eq!(h.tip(), Some(0));
    h.frame(vec![]);
    h.frame(vec![]);
    assert!(
        h.text_at(&format!("Tip 1 of {}", super::tips::TIPS.len()))
            .is_some()
    );
    let next = h.text_at("Next ▶").unwrap();
    h.click(next, Modifiers::NONE);
    h.frame(vec![]);
    assert_eq!(h.tip(), Some(1));
    let previous = h.text_at("◀ Previous").unwrap();
    h.click(previous, Modifiers::NONE);
    h.frame(vec![]);
    assert_eq!(h.tip(), Some(0));
    // the tip's length moves the buttons
    let previous = h.text_at("◀ Previous").unwrap();
    h.click(previous, Modifiers::NONE);
    h.frame(vec![]);
    assert_eq!(
        h.tip(),
        Some(super::tips::TIPS.len() - 1),
        "round to the last"
    );
    let show = h.text_at("Show a tip when PowerWhittle starts").unwrap();
    h.click(show, Modifiers::NONE);
    h.frame(vec![]);
    assert!(!h.options().general.show_tips_at_startup, "turned off");
    let close = h.text_at("Close").unwrap();
    h.click(close, Modifiers::NONE);
    h.frame(vec![]);
    assert_eq!(h.tip(), None);
}

#[test]
fn each_start_shows_a_tip_not_seen_yet_unless_turned_off() {
    use super::tips::TIPS;
    let mut h = Harness::at_start();
    let mut shown = std::collections::HashSet::new();
    for _ in 0..TIPS.len() {
        h.close_tips();
        h.tip_at_startup();
        let n = h.tip().expect("a tip at start");
        h.frame(vec![]);
        assert!(shown.insert(n), "tip {n} came up twice before the rest");
    }
    // tests start in Classic, so the Canvas tip is among them
    assert_eq!(shown.len(), TIPS.len(), "every tip once");
    h.close_tips();
    h.tip_at_startup();
    assert!(h.tip().is_some(), "then round again");
    h.frame(vec![]);
    assert_eq!(h.options().seen_tips.len(), 1, "starting over");
    h.close_tips();
    h.given_options(|o| o.general.show_tips_at_startup = false);
    h.tip_at_startup();
    assert_eq!(h.tip(), None, "turned off");
    h.given_options(|o| o.general.show_tips_at_startup = true);
    h.open_first_run();
    h.tip_at_startup();
    assert_eq!(h.tip(), None, "not over first-time setup");
}

#[test]
fn paging_through_tips_counts_them_as_seen() {
    let mut h = Harness::at_start();
    h.open_tips(0);
    h.frame(vec![]);
    h.frame(vec![]);
    for _ in 0..3 {
        let next = h.text_at("Next ▶").unwrap();
        h.click(next, Modifiers::NONE);
        h.frame(vec![]);
    }
    let seen: Vec<u64> = super::tips::TIPS[..4]
        .iter()
        .map(super::tips::Tip::key)
        .collect();
    assert_eq!(h.options().seen_tips, seen);
}

/// Where the Canvas tip is in the list.
fn canvas_tip() -> usize {
    super::tips::TIPS
        .iter()
        .position(|t| t.tries == Some(crate::options::WindowLayout::Canvas))
        .expect("a tip offers Canvas")
}

#[test]
fn the_canvas_tip_switches_to_canvas_and_is_not_shown_in_canvas() {
    let mut h = Harness::at_start();
    let n = canvas_tip();
    assert!(super::tips::TIPS[n].weight > 1, "it comes up more often");
    h.open_tips(n);
    h.frame(vec![]);
    h.frame(vec![]);
    // the words name the button too: the button is under them
    let try_it = *h.texts_at("Try Canvas").last().expect("a button to try it");
    h.click(try_it, Modifiers::NONE);
    h.frame(vec![]);
    assert_eq!(
        h.options().general.layout,
        crate::options::WindowLayout::Canvas
    );
    assert_eq!(h.tip(), None, "the tip closes");
    // in Canvas: never at start, and Next and Previous step over it
    h.given_options(|o| o.seen_tips.clear());
    for _ in 0..super::tips::TIPS.len() {
        h.close_tips();
        h.tip_at_startup();
        assert_ne!(h.tip(), Some(n), "not offered in Canvas");
        h.frame(vec![]);
    }
    h.open_tips(n - 1);
    h.frame(vec![]);
    h.frame(vec![]);
    let next = h.text_at("Next ▶").unwrap();
    h.click(next, Modifiers::NONE);
    h.frame(vec![]);
    assert_eq!(h.tip(), Some(n + 1), "stepped over");
}

#[test]
fn every_tip_says_something_and_its_more_is_a_help_page() {
    let h = Harness::new();
    let topics = h.all_help_topics();
    let mut seen = std::collections::HashSet::new();
    for tip in super::tips::TIPS {
        assert!(tip.text.split_whitespace().count() >= 10, "{}", tip.text);
        assert!(seen.insert(tip.text), "said twice: {}", tip.text);
        assert!(
            topics.contains(&tip.topic),
            "{} is not a help page: {}",
            tip.topic,
            tip.text
        );
        for name in ["Aspire", "Vectric"] {
            assert!(!tip.text.contains(name), "{name} named: {}", tip.text);
        }
    }
    let page = h.help_markdown("tips");
    assert!(page.contains(super::tips::TIPS[0].text));
}

#[test]
fn every_tip_tells_more_on_a_page_of_its_own_not_help_contents() {
    // Tell Me More… on Help Contents leaves a beginner to hunt for what the tip meant
    let vague: Vec<&str> = super::tips::TIPS
        .iter()
        .filter(|t| t.topic == "contents")
        .map(|t| t.text)
        .collect();
    assert!(
        vague.is_empty(),
        "point these at their own page:\n{vague:#?}"
    );
}

#[test]
fn there_are_well_over_a_hundred_tips_on_every_subject() {
    use super::tips::{Area, TIPS};
    assert!(TIPS.len() >= 129, "only {} tips", TIPS.len());
    for area in Area::ALL {
        let n = TIPS.iter().filter(|t| t.area == area).count();
        assert!(n >= 5, "{area:?} has only {n} tips");
    }
}

#[test]
fn the_tips_page_sorts_them_by_subject_and_links_each_to_its_help() {
    let h = Harness::new();
    let page = h.help_markdown("tips");
    let mut at = 0;
    for area in super::tips::Area::ALL {
        let heading = format!("\n## {}\n", area.heading());
        let found = page[at..]
            .find(&heading)
            .unwrap_or_else(|| panic!("{heading:?} missing, or out of order"));
        at += found + heading.len();
    }
    for tip in super::tips::TIPS {
        let line = format!("- {} [Tell me more](help:{})", tip.text, tip.topic);
        assert!(page.contains(&line), "not listed with its link: {line}");
    }
}

#[test]
fn every_tip_has_an_illustration_of_its_own_or_its_tools_icon() {
    use super::tips::Picture;
    let h = Harness::new();
    let icons = icon_set();
    let mut seen = std::collections::HashSet::new();
    let mut missing = vec![];
    for tip in super::tips::TIPS {
        match tip.picture {
            Picture::Scene(picture) => {
                assert!(
                    picture.starts_with("tips/")
                        && std::path::Path::new(picture)
                            .extension()
                            .is_some_and(|e| e == "svg"),
                    "{picture}: a drawn illustration from assets/icons/tips, sharp at any size"
                );
                assert!(icons.contains(picture), "{picture} is not drawn");
                assert!(seen.insert(picture), "{picture} illustrates two tips");
            }
            Picture::Icon(id) => {
                let Some(key) = h.tip_icon(id) else {
                    missing.push(id);
                    continue;
                };
                assert!(!key.starts_with("tips/"), "{key}: an icon, not a scene");
            }
        }
    }
    let unused: Vec<_> = icons
        .iter()
        .filter(|k| k.starts_with("tips/") && !seen.contains(k.as_str()))
        .collect();
    assert!(unused.is_empty(), "illustrations no tip shows: {unused:?}");
    assert!(missing.is_empty(), "no icon for these tips: {missing:?}");
}

#[test]
fn a_tip_about_a_tool_shows_that_tools_icon() {
    use super::tips::{Picture, TIPS};
    let h = Harness::new();
    let tip = TIPS
        .iter()
        .find(|t| t.picture == Picture::Icon("pocket_toolpath"))
        .expect("a tip about pockets");
    assert_eq!(
        h.tip_picture(tip),
        Some("toolpaths/pocket_toolpath.png"),
        "the same picture as the button and its help page"
    );
    assert_eq!(h.tip_picture(&TIPS[0]), Some("tips/sample_project.svg"));
}

#[test]
fn the_tips_window_shows_the_illustration_above_the_words() {
    let mut h = Harness::at_start();
    h.open_tips(0);
    h.frame(vec![]);
    h.frame(vec![]);
    let starting = |start: &str| {
        h.shapes.iter().find_map(|clipped| match &clipped.shape {
            egui::Shape::Text(t) if t.galley.text().starts_with(start) => Some(t.pos),
            _ => None,
        })
    };
    let words = starting("New to PowerWhittle?").expect("the first tip's words");
    let tip = starting("Tip 1 of").expect("the count");
    assert!(
        words.y - tip.y > 6.0 * 18.0,
        "room for the picture between the count and the words: {tip:?} {words:?}"
    );
}
