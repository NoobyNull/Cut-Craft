//! Flat on a locked face: a flat toolpath on a rotary job cut flat with A held still.

use super::*;
use cut_craft_core::document::{Along, Job};

/// A harness on a rotary job 300 mm along X, 50 across, with a 20 mm square drawn from 30
/// to 50 mm round the part selected and the Profile form open.
fn profile_on_a_rotary_job() -> Harness {
    let mut h = Harness::new();
    h.app.doc.job = Job::rotary(300.0, 50.0, Along::X);
    let mut c = Change::builder(&h.app.doc);
    let line = c.add(Path::polyline(
        &[
            Point::new(90.0, 30.0),
            Point::new(110.0, 30.0),
            Point::new(110.0, 50.0),
            Point::new(90.0, 50.0),
        ],
        true,
    ));
    h.app.record(Ok(c.finish()), "line");
    h.app.pages.set_pinned(Page::Toolpaths, true);
    h.app.invoke(&h.ctx, "profile_toolpath");
    h.app.doc.selection = [line].into_iter().collect();
    h
}

#[test]
fn a_profile_on_a_rotary_job_can_be_cut_flat_on_a_locked_face() {
    let mut h = profile_on_a_rotary_job();
    let ctx = egui::Context::default();
    let mut section = serde_json::json!({});
    let out = headless(ctx.run_ui(RawInput::default(), |ui| {
        egui::CentralPanel::default().show(ui, |ui| locked_face_section(ui, &mut section));
    }));
    assert!(
        out.shapes.iter().any(
            |c| matches!(&c.shape, egui::Shape::Text(t) if t.galley.text() == "Flat on a locked face")
        ),
        "offered on the form"
    );
    let mut params = h.app.form.as_ref().unwrap().params.clone();
    params[cut_craft_core::locked_face::KEY] = serde_json::json!({ "angle": null });
    h.app.apply("profile_toolpath", &params).unwrap();
    let (&id, made) = h.app.doc.toolpaths.iter().last().unwrap();
    assert_eq!(made.across.len(), made.moves.len(), "held across the face");
    // A stays where the square is drawn round the part: its middle, 40 mm round
    let radius = 25.0;
    let rounds: Vec<f64> = (made.moves.iter().zip(&made.across))
        .map(|(m, &a)| cut_craft_core::rotary_axis::held_at(radius, m.to.y, m.to.z, a).0)
        .collect();
    assert!(
        rounds.iter().all(|r| (r - 40.0).abs() < 1e-6),
        "A locked: {rounds:?}"
    );
    // recalculating keeps it on the face
    let again = h.app.registry.recalculate(&h.app.doc, id).unwrap();
    h.app.record(Ok(again), "recalculate");
    let kept = &h.app.doc.toolpaths[&id];
    assert_eq!(kept.across.len(), kept.moves.len(), "still on the face");
}
