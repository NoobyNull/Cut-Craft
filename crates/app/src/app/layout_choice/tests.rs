use super::*;

#[test]
fn asked_once_after_updating_but_not_with_first_time_setup_or_once_answered() {
    let mut options = Options::default();
    assert!(
        asks_after_update(&options, false),
        "an install that had Cut Craft before"
    );
    assert!(
        !asks_after_update(&options, true),
        "first-time setup asks it instead"
    );
    options.layout_asked = true;
    assert!(
        !asks_after_update(&options, false),
        "answered or closed once"
    );
}

#[test]
fn an_answer_sets_the_layout_and_closing_keeps_it_and_neither_asks_again() {
    let mut options = Options::default();
    options.general.layout = WindowLayout::Classic;
    answer(&mut options, None);
    assert_eq!(options.general.layout, WindowLayout::Classic);
    assert!(options.layout_asked);

    let mut options = Options::default();
    options.general.layout = WindowLayout::Classic;
    answer(&mut options, Some(WindowLayout::Rail));
    assert_eq!(options.general.layout, WindowLayout::Rail);
    assert!(options.layout_asked);
}

#[test]
fn every_layout_has_its_picture_in_the_help() {
    for layout in WindowLayout::ALL {
        assert!(
            cut_craft_help::picture(picture(layout)).is_some(),
            "{layout:?}"
        );
    }
}
