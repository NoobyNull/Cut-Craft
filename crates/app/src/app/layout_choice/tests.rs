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

#[test]
fn the_cards_are_sized_from_the_screen_so_the_window_keeps_its_size() {
    let row = 18.0;
    let wide = card_width(1600.0, 14.0, 8.0, row);
    assert_eq!(
        wide,
        card_width(1600.0, 14.0, 8.0, row),
        "the same every frame"
    );
    let all = 4.0 * (wide + 14.0) + 3.0 * 8.0;
    assert!(all <= 1600.0, "{all}");
    assert_eq!(
        card_width(4000.0, 14.0, 8.0, row),
        14.0 * row,
        "no wider than 14 rows"
    );
    assert!(
        card_width(800.0, 14.0, 8.0, row) < wide,
        "narrower in a narrow window"
    );
    assert_eq!(
        card_width(50.0, 14.0, 8.0, row),
        row,
        "never squeezed to nothing"
    );
}
