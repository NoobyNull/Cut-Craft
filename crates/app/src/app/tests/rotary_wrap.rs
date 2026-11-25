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
    h.app.doc.job = Job::rotary(300.0, 50.0, Along::X);
    let mut c = Change::builder(&h.app.doc);
    let line = c.add(Path::polyline(
        &[Point::new(60.0, 10.0), Point::new(120.0, turn() + 30.0)],
        false,
    ));
    h.app.record(Ok(c.finish()), "line");
    h.app.camera = None;
    h.frame(vec![]);
    h.frame(vec![]);
    (h, line)
}

#[test]
fn zoom_to_fit_shows_a_rotary_job_s_bands_as_well_as_its_sheet() {
    // a fat blank, so one turn is much longer than the job: the bands set how far out it fits
    let mut h = Harness::new();
    h.app.doc.job = Job::rotary(100.0, 100.0, Along::X);
    h.app.camera = None;
    h.frame(vec![]);
    let turn = std::f64::consts::PI * 100.0;
    let view = h.app.view_area.expect("the 2D view was drawn");
    for y in [-turn / 4.0, turn * 1.25] {
        assert!(
            view.contains(h.at(50.0, y)),
            "the band's edge at {y} is in view"
        );
    }
    // with the bands off, the sheet alone fills the view, as on a flat job: drawn larger
    let with_bands = h.app.view_transform.unwrap().scale;
    h.app.options.appearance.rotary_wrap = crate::options::WrapBands::Off;
    h.app.camera = None;
    h.frame(vec![]);
    let sheet_only = h.app.view_transform.unwrap().scale;
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
    assert_eq!(
        h.app.doc.selection,
        [line].into(),
        "the real line, a turn up"
    );
    // a click on nothing lets it go; then, with the bands off, there is no ghost to click
    h.click(h.at(250.0, 100.0), Modifiers::NONE);
    assert!(h.app.doc.selection.is_empty());
    h.app.options.appearance.rotary_wrap = crate::options::WrapBands::Off;
    h.frame(vec![]);
    h.click(ghost, Modifiers::NONE);
    assert!(
        h.app.doc.selection.is_empty(),
        "nothing drawn, nothing picked"
    );
}

#[test]
fn drawing_snaps_to_a_ghost_s_end_without_tying_the_point_to_the_real_line() {
    let (mut h, _) = rotary_with_line();
    h.app.invoke(&h.ctx, "draw_circle");
    h.frame(vec![]);
    let scale = h.app.view_transform.unwrap().scale;
    let near = 2.0 / f64::from(scale);
    // the line's top end, a turn down: its ghost, just above the bottom seam
    let ghost_end = Point::new(120.0, 30.0);
    let drag = Drag {
        start: ghost_end + Point::new(near, -near),
        end: Point::new(200.0, 80.0),
        from_center: false,
        constrain: false,
    };
    let mut snapped = None;
    let ctx = h.ctx.clone();
    headless(ctx.run_ui(RawInput::default(), |ui| {
        egui::CentralPanel::default().show(ui, |ui| {
            snapped = h.app.snapped(ui, Some(Gesture::Dragging(drag)), scale);
        });
    }));
    let Some(Gesture::Dragging(d)) = snapped else {
        panic!("still a drag")
    };
    assert!(d.start.distance(ghost_end) < 1e-9, "onto the ghost's end");
}
