use super::TurnRings;
use egui::{Pos2, pos2, vec2};

/// A square "ring" about `axis` round (0, 0), `half` points out each way.
fn square(half: f32) -> Vec<Pos2> {
    vec![
        pos2(-half, -half),
        pos2(half, -half),
        pos2(half, half),
        pos2(-half, half),
        pos2(-half, -half),
    ]
}

fn drawn() -> TurnRings {
    let mut rings = TurnRings::default();
    rings.add_ring("x", square(100.0));
    rings.add_ring("z", square(50.0));
    rings.set_sphere(pos2(0.0, 0.0), 10.0);
    rings
}

#[test]
fn the_sphere_is_found_before_any_ring() {
    assert_eq!(drawn().at(pos2(3.0, 0.0), 60.0), Some("xyz"));
}

#[test]
fn the_nearest_ring_within_reach_is_found() {
    let rings = drawn();
    assert_eq!(rings.at(pos2(95.0, 0.0), 8.0), Some("x"));
    assert_eq!(rings.at(pos2(55.0, 0.0), 8.0), Some("z"));
    assert_eq!(rings.at(pos2(75.0, 0.0), 8.0), None, "out of reach of both");
}

#[test]
fn nothing_is_found_once_cleared() {
    let mut rings = drawn();
    rings.clear();
    assert_eq!(rings.at(pos2(0.0, 0.0), 8.0), None);
    assert_eq!(rings.drawn(), (vec![], None));
}

#[test]
fn a_drag_holds_the_ring_it_started_on_until_let_go() {
    let mut rings = drawn();
    rings.press(Some(pos2(100.0, 20.0)), 8.0);
    assert_eq!(rings.held(), Some("x"));
    rings.let_go();
    assert_eq!(rings.held(), None);
    rings.press(Some(pos2(75.0, 0.0)), 8.0);
    assert_eq!(rings.held(), None, "started on no handle");
}

#[test]
fn the_grip_runs_along_the_ring_held_under_the_pointer() {
    let mut rings = drawn();
    rings.follow(Some(pos2(100.0, 0.0)));
    assert_eq!(rings.grip(), None, "nothing held");
    rings.press(Some(pos2(100.0, 0.0)), 8.0);
    rings.follow(Some(pos2(100.0, 0.0)));
    let (along, reach) = rings.grip().expect("held");
    assert_eq!(along, vec2(0.0, 1.0), "down the right-hand side");
    assert!(
        (reach - 100.0 * std::f32::consts::SQRT_2).abs() < 1e-3,
        "{reach}"
    );
    rings.follow(None);
    assert_eq!(rings.grip(), None, "no pointer");
}
