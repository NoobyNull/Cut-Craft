use super::*;

const WINDOW: Rect = Rect {
    min: pos2(0.0, 0.0),
    max: pos2(1000.0, 800.0),
};

fn names(menu: &[MenuItem]) -> Vec<&str> {
    menu.iter().map(|item| item.name.as_str()).collect()
}

fn act(place: &mut Place, item: &MenuItem) -> bool {
    let what = item.action.strip_prefix("bar:").expect("a bar action");
    apply(place, what, true)
}

fn find<'a>(menu: &'a [MenuItem], name: &str) -> &'a MenuItem {
    menu.iter()
        .find(|item| item.name == name)
        .unwrap_or_else(|| panic!("no {name} in {:?}", names(menu)))
}

#[test]
fn a_new_bar_floats_and_runs_across() {
    let place = Place::default();
    assert!(matches!(place, Place::Floating(f) if f.offset.is_none() && !f.locked));
    assert!(place.runs_across());
}

#[test]
fn a_bar_snaps_to_each_side_and_runs_along_it() {
    let mut place = Place::default();
    for (side, across) in [
        ("Snap to Top", true),
        ("Snap to Bottom", true),
        ("Snap to Left", false),
        ("Snap to Right", false),
    ] {
        let menu = menu(&place, "bar:", None);
        assert!(act(&mut place, find(&menu, side)), "{side}");
        assert_eq!(place.runs_across(), across, "{side}");
        let menu = super::menu(&place, "bar:", None);
        assert!(find(&menu, side).is_checked, "{side} ticked");
        assert!(!find(&menu, "Float").is_checked);
    }
}

#[test]
fn only_a_floating_bar_locks_and_turns() {
    let snapped = Place::Snapped { side: Side::Top };
    let menu = menu(&snapped, "bar:", None);
    assert!(!names(&menu).contains(&"Lock Position"));
    assert!(!names(&menu).contains(&"Turn"));
    let mut place = snapped;
    assert!(
        !apply(&mut place, "turn", true),
        "a snapped bar runs along its side"
    );
    let menu = super::menu(&place, "bar:", None);
    assert!(act(&mut place, find(&menu, "Float")));
    let menu = super::menu(&place, "bar:", None);
    assert!(find(&menu, "Float").is_checked);
    assert!(act(&mut place, find(&menu, "Turn")));
    assert!(!place.runs_across(), "turned to run down");
    let menu = super::menu(&place, "bar:", None);
    assert!(act(&mut place, find(&menu, "Lock Position")));
    let menu = super::menu(&place, "bar:", None);
    assert!(find(&menu, "Lock Position").is_checked);
}

#[test]
fn a_bar_with_a_home_detaches_from_it_and_attaches_back() {
    let mut place = Place::Layout;
    let menu = menu(&place, "bar:", Some(Home::Header));
    assert_eq!(menu.last().unwrap().name, "Detach");
    assert!(act(&mut place, menu.last().unwrap()));
    assert!(matches!(place, Place::Floating(_)), "detached, it floats");
    let menu = super::menu(&place, "bar:", Some(Home::Header));
    assert_eq!(menu.last().unwrap().name, "Attach to Header");
    assert!(act(&mut place, menu.last().unwrap()));
    assert_eq!(place, Place::Layout);
    // each layout names its own home
    let snapped = Place::Snapped { side: Side::Top };
    for (home, name) in [
        (Home::Rail, "Attach to Rail"),
        (Home::DesignTab, "Attach to Design Tab"),
    ] {
        assert_eq!(menu_last(&snapped, home), name);
    }
    // a bar with no home (the Canvas palette) is never put in the layout
    assert!(!names(&super::menu(&snapped, "bar:", None)).contains(&"Detach"));
    let mut palette = snapped;
    assert!(!apply(&mut palette, "attach", false));
}

fn menu_last(place: &Place, home: Home) -> String {
    menu(place, "bar:", Some(home)).last().unwrap().name.clone()
}

#[test]
fn a_floating_bar_starts_where_its_bar_starts_until_it_is_moved() {
    let floating = Floating::default();
    let size = vec2(200.0, 40.0);
    assert_eq!(
        floating.top_left(size, WINDOW, pos2(10.0, 60.0)),
        pos2(10.0, 60.0)
    );
}

#[test]
fn a_dropped_bar_keeps_its_distance_from_the_nearest_corner_as_the_window_changes() {
    let size = vec2(200.0, 40.0);
    let start = pos2(10.0, 60.0);
    let bigger = Rect::from_min_max(pos2(0.0, 0.0), pos2(1400.0, 1000.0));
    for (at, then) in [
        // top left stays put
        (pos2(30.0, 50.0), pos2(30.0, 50.0)),
        // top right keeps its distance from the right
        (pos2(700.0, 50.0), pos2(1100.0, 50.0)),
        // bottom left keeps its distance from the foot
        (pos2(30.0, 700.0), pos2(30.0, 900.0)),
        // bottom right from both
        (pos2(700.0, 700.0), pos2(1100.0, 900.0)),
    ] {
        let mut floating = Floating::default();
        assert!(floating.drop_at(at, size, WINDOW));
        assert_eq!(
            floating.top_left(size, WINDOW, start),
            at,
            "where it was left"
        );
        assert_eq!(floating.top_left(size, bigger, start), then, "{at:?}");
    }
}

#[test]
fn a_floating_bar_stays_inside_a_window_made_smaller() {
    let size = vec2(200.0, 40.0);
    let mut floating = Floating::default();
    floating.drop_at(pos2(30.0, 50.0), size, WINDOW);
    let tiny = Rect::from_min_max(pos2(0.0, 0.0), pos2(100.0, 30.0));
    let at = floating.top_left(size, tiny, pos2(0.0, 0.0));
    assert_eq!(at, pos2(0.0, 0.0), "pushed back in from the far edges");
}

#[test]
fn a_locked_bar_is_not_moved_by_a_drag() {
    let size = vec2(200.0, 40.0);
    let mut floating = Floating::default();
    floating.drop_at(pos2(30.0, 50.0), size, WINDOW);
    floating.locked = true;
    assert!(!floating.drop_at(pos2(500.0, 500.0), size, WINDOW));
    assert_eq!(
        floating.top_left(size, WINDOW, pos2(0.0, 0.0)),
        pos2(30.0, 50.0)
    );
}

#[test]
fn bars_made_together_start_one_under_another() {
    assert_eq!(start(pos2(10.0, 60.0), 0, 40.0), pos2(10.0, 60.0));
    assert_eq!(start(pos2(10.0, 60.0), 2, 40.0), pos2(10.0, 140.0));
}

#[test]
fn a_place_is_kept_and_read_back() {
    let mut floating = Floating::default();
    floating.drop_at(pos2(700.0, 50.0), vec2(200.0, 40.0), WINDOW);
    floating.locked = true;
    for place in [
        Place::Layout,
        Place::Snapped { side: Side::Right },
        Place::Floating(floating),
    ] {
        let json = serde_json::to_string(&place).unwrap();
        assert_eq!(
            serde_json::from_str::<Place>(&json).unwrap(),
            place,
            "{json}"
        );
    }
}

#[test]
fn a_bar_floated_off_the_left_or_right_keeps_running_down() {
    for (side, down) in [
        (Side::Left, true),
        (Side::Right, true),
        (Side::Top, false),
        (Side::Bottom, false),
    ] {
        let mut place = Place::Snapped { side };
        assert!(apply(&mut place, "float", false));
        assert_eq!(!place.runs_across(), down, "{side:?}");
    }
    assert!(Place::column().is_column() && !Place::column().runs_across());
}
