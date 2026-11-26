//! Rotary jobs in the 2D view: the bands past each seam, their ghosts, and clicks and snaps
//! on them.

use super::*;
use cut_craft_core::document::{Along, Job};

/// One turn of the 50 mm blank the tests use.
fn turn() -> f64 {
    std::f64::consts::PI * 50.0
}

/// A harness on a rotary job 300 mm along X, with a slanted line from just above the bottom
/// seam running out over the top one; the view fitted to it.
fn rotary_with_line() -> (Harness, EntityId) {
    let mut h = Harness::new();
    h.given_document(|d| d.job = Job::rotary(300.0, 50.0, Along::X));
    let mut c = Change::builder(h.doc());
    let line = c.add(Path::polyline(
        &[Point::new(60.0, 10.0), Point::new(120.0, turn() + 30.0)],
        false,
    ));
    h.make(c.finish(), "line");
    h.zoom_to_job();
    h.frame(vec![]);
    h.frame(vec![]);
    (h, line)
}

#[test]
fn zoom_to_fit_shows_a_rotary_job_s_bands_as_well_as_its_sheet() {
    // a fat blank, so one turn is much longer than the job: the bands set how far out it fits
    let mut h = Harness::new();
    h.given_document(|d| d.job = Job::rotary(100.0, 100.0, Along::X));
    h.zoom_to_job();
    h.frame(vec![]);
    let turn = std::f64::consts::PI * 100.0;
    let view = h.view_2d_area();
    for y in [-turn / 4.0, turn * 1.25] {
        assert!(
            view.contains(h.at(50.0, y)),
            "the band's edge at {y} is in view"
        );
    }
    // with the bands off, the sheet alone fills the view, as on a flat job: drawn larger
    let with_bands = h.zoom_2d();
    h.given_options(|o| o.appearance.rotary_wrap = crate::options::WrapBands::Off);
    h.zoom_to_job();
    h.frame(vec![]);
    let sheet_only = h.zoom_2d();
    assert!(
        sheet_only > with_bands * 1.3,
        "{sheet_only} against {with_bands}"
    );
}

#[test]
fn clicking_a_ghost_that_wraps_in_at_the_bottom_seam_selects_the_line_it_copies() {
    let (mut h, line) = rotary_with_line();
    // the line runs on over the top seam: that part's ghost comes in a turn lower, just
    // above the bottom seam, away from the real line
    let on_line = |y: f64| 60.0 + (y - 10.0) / (turn() + 20.0) * 60.0;
    let y = turn() + 15.0;
    let ghost = h.at(on_line(y), y - turn());
    h.click(ghost, Modifiers::NONE);
    assert_eq!(h.doc().selection, [line].into(), "the real line, a turn up");
    // a click on nothing lets it go; then, with the bands off, there is no ghost to click
    h.click(h.at(250.0, 100.0), Modifiers::NONE);
    assert!(h.doc().selection.is_empty());
    h.given_options(|o| o.appearance.rotary_wrap = crate::options::WrapBands::Off);
    h.frame(vec![]);
    h.click(ghost, Modifiers::NONE);
    assert!(
        h.doc().selection.is_empty(),
        "nothing drawn, nothing picked"
    );
}

#[test]
fn drawing_snaps_to_a_ghost_s_end_without_tying_the_point_to_the_real_line() {
    let (mut h, _) = rotary_with_line();
    h.invoke("draw_circle");
    h.frame(vec![]);
    let near = 2.0 / f64::from(h.zoom_2d());
    // the line's top end, a turn down: its ghost, just above the bottom seam
    let ghost_end = Point::new(120.0, 30.0);
    let drag = Drag {
        start: ghost_end + Point::new(near, -near),
        end: Point::new(200.0, 80.0),
        from_center: false,
        constrain: false,
    };
    let Some(Gesture::Dragging(d)) = h.snap_drag(drag) else {
        panic!("still a drag")
    };
    assert!(d.start.distance(ghost_end) < 1e-9, "onto the ghost's end");
}

#[test]
fn clicking_inside_a_ghost_of_a_closed_shape_selects_the_shape() {
    let (mut h, _) = rotary_with_line();
    // a circle across the bottom seam: its ghost reaches up over the top seam
    let mut c = Change::builder(h.doc());
    let circle = c.add(Path::circle(Point::new(250.0, 5.0), 20.0));
    h.make(c.finish(), "circle");
    h.frame(vec![]);
    // inside the ghost, away from its outline, in the band above the top seam
    h.click(h.at(250.0, turn() + 15.0), Modifiers::NONE);
    assert_eq!(h.doc().selection, [circle].into(), "the circle it copies");
}
