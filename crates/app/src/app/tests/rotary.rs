//! Rotary jobs: turning, wrapping and the cylinder.

use super::*;

#[test]
fn auto_rotate_turns_a_rotary_job_round_its_own_axis() {
    use cut_craft_core::document::{Along, Job};
    use cut_craft_core::glam::DVec3;
    for (along, axis) in [(Along::X, DVec3::X), (Along::Y, DVec3::Y)] {
        let mut h = Harness::new();
        h.app.doc.job = Job::rotary(300.0, 50.0, along);
        h.app.view = View::ThreeD;
        h.frame(vec![]);
        h.frame(vec![]);
        assert!(h.app.display.auto_wrapping, "drawn wrapped round the axis");
        let start = h.app.orbit;
        let button = h.text_at("Auto-rotate").unwrap();
        h.click(button, Modifiers::NONE);
        for _ in 0..5 {
            h.frame(vec![]);
        }
        let now = h.app.orbit;
        assert!(
            (now.project(axis).0 - start.project(axis).0).length() < 1e-6,
            "{along:?}: the cylinder's axis stays put"
        );
        assert!(
            now.towards_viewer().angle_between(start.towards_viewer()) > 0.05,
            "{along:?}: and the cylinder turns round it"
        );
        // drawn flat (View ▸ Auto Wrapping off), it spins about the vertical as a flat job
        h.app.display.auto_wrapping = false;
        let flat = h.app.orbit;
        h.frame(vec![]);
        h.frame(vec![]);
        assert!((h.app.orbit.pitch - flat.pitch).abs() < 1e-9 && h.app.orbit.yaw != flat.yaw);
    }
}

#[test]
fn an_imported_model_shows_whole_on_the_bed_sized_in_three_with_the_rotary_choices_hidden() {
    let mut h = Harness::new();
    let file = pyramid_stl("whole");
    h.app.import_file(&file);
    h.frame(vec![]);
    h.frame(vec![]);
    let form = h.app.form.as_ref().unwrap();
    // issue #20: the model whole while the form is open, its component left out of the view
    assert!(form.model.as_ref().is_some_and(|m| m.len() == 6));
    let k = form.drawn.unwrap();
    let shape = h.app.model_shape(MESH_CELLS);
    assert!(
        shape.heights.iter().all(|z| z.is_nan()),
        "the component is not drawn too"
    );
    assert!(
        h.app.doc.components[&k].visible,
        "and is only hidden while drawing"
    );
    // on the bed, and Model Size has the height
    let (lo, _) = h.app.doc.components[&k].height_range().unwrap();
    assert!((0.0..1.0).contains(&lo), "from the bed up: {lo}");
    let size = &h.app.form.as_ref().unwrap().params["size"];
    assert!((size["z"].as_f64().unwrap() - 5.0).abs() < 1e-6, "{size}");
    // the ratio locked, a height typed sizes all three
    let form = h.app.form.as_mut().unwrap();
    form.params["size"]["z"] = Value::from(10.0);
    form.dirty = true;
    h.frame(vec![]);
    let size = &h.app.form.as_ref().unwrap().params["size"];
    assert!((size["x"].as_f64().unwrap() - 40.0).abs() < 1e-6, "{size}");
    // the fit is offered in a flat job too, as a button done once; the model type only when
    // the options offer both kinds, and what the options are is never shown
    let over = h.text_at("Initial Orientation").unwrap();
    assert!(
        h.scroll_to(over, "Change size").is_some(),
        "the size's lock"
    );
    // the placing is on the same page, Center model left to the alignment buttons
    assert!(h.scroll_to(over, "Depth Below Top").is_some());
    assert!(h.text_at("Center model").is_none());
    // Scale to Fit Material sits in Model Size, just under the Change size tick that
    // unlocks it, before the next section starts
    let fit = h.scroll_to(over, "Scale to Fit Material").unwrap();
    let tick = h.text_at("Change size").unwrap();
    let next = h.text_at("Position relative to Modeling Plane").unwrap();
    assert!(
        tick.y < fit.y && fit.y < next.y,
        "beside Change size: {tick:?} {fit:?} {next:?}"
    );
    // greyed while the size is locked: a click there leaves the size alone
    let scale = |h: &Harness| h.app.form.as_ref().unwrap().params["size"]["scale"].clone();
    let before = scale(&h);
    h.click(fit, Modifiers::NONE);
    h.frame(vec![]);
    assert_eq!(scale(&h), before, "locked until Change size is ticked");
    // ticked, the same button scales the model to the material once
    h.app.form.as_mut().unwrap().params["size"]["change_size"] = Value::Bool(true);
    h.frame(vec![]);
    let fit = h.text_at("Scale to Fit Material").unwrap();
    h.click(fit, Modifiers::NONE);
    h.frame(vec![]);
    assert_ne!(scale(&h), before, "scaled to fit the material");
    for label in ["Imported model type", "Offered"] {
        assert_eq!(h.text_at(label), None, "{label} is not shown");
    }
    let _ = std::fs::remove_file(file);
}

#[test]
fn orientate_3d_models_drags_turn_the_model_or_move_it_off_the_rotary_axis_when_asked() {
    use cut_craft_core::document::{Along, Job};
    let mut h = Harness::new();
    h.app.doc.job = Job::rotary(100.0, 30.0, Along::X);
    h.app.invoke(&h.ctx, "orientate_3d_model");
    let form = h.app.form.as_ref().unwrap();
    assert_eq!(
        form.params["rotary"], true,
        "the form knows the job is rotary"
    );
    assert_eq!(form.params["turn"]["mode"], "view");
    let (size, down, right) = (
        egui::vec2(400.0, 200.0),
        egui::vec2(0.0, 100.0),
        egui::vec2(100.0, 0.0),
    );
    // dragging the view turns the view, not the model
    assert!(
        !h.app
            .import_drag(Some(view3d::DragMode::Rotate), down, size)
    );
    h.app.form.as_mut().unwrap().params["turn"]["mode"] = Value::from("x");
    assert!(
        h.app
            .import_drag(Some(view3d::DragMode::Rotate), down, size)
    );
    let form = h.app.form.as_ref().unwrap();
    assert_eq!(
        form.params["turn"]["x"], 90.0,
        "half the view's height: a quarter turn"
    );
    assert!(form.dirty, "the model follows");
    // a pan moves the view unless Drag moves the axis is on
    assert!(!h.app.import_drag(Some(view3d::DragMode::Pan), right, size));
    h.app.form.as_mut().unwrap().params["axis"]["drag"] = Value::from(true);
    assert!(h.app.import_drag(Some(view3d::DragMode::Pan), right, size));
    let form = h.app.form.as_ref().unwrap();
    assert_eq!(
        form.params["axis"]["across"], 15.0,
        "a quarter of the view: half a diameter"
    );
    assert_eq!(form.params["axis"]["up"], 0.0);
}

#[test]
fn slow_frames_while_turning_draw_the_model_coarser_then_finely_once_still() {
    use crate::options::Detail;
    let mut h = turned_slowly(true, 20);
    let (level, moving) = h.app.detail_3d;
    assert!(moving);
    assert!(
        level < Detail::VeryHigh,
        "dropped from Very High: {level:?}"
    );
    assert!(h.app.moving_cells() < Detail::VeryHigh.cells());
    // a second still: back to the detail when still
    for _ in 0..10 {
        h.frame(vec![]);
    }
    assert_eq!(h.app.detail_3d, (Detail::VeryHigh, false));
}

#[test]
fn a_full_model_imported_into_a_rotary_job_is_drawn_as_it_really_is_not_wrapped_again() {
    use cut_craft_core::document::{Along, Job};
    let mut h = Harness::new();
    h.app.doc.job = Job::rotary(100.0, 30.0, Along::X);
    let file = pyramid_stl("rotary-real");
    h.app.import_file(&file);
    h.app.view = View::ThreeD;
    h.frame(vec![]);
    // what the import showed, turned round the axis
    let importing: Vec<_> = h
        .app
        .form
        .as_ref()
        .unwrap()
        .model
        .as_deref()
        .unwrap()
        .clone();
    h.app.form = None;
    h.frame(vec![]);
    let k = h.app.doc.components.values().next().unwrap().clone();
    assert!(!k.solid.is_empty(), "kept as its real 3D shape");
    // not wrapped round the cylinder again from its unrolled heights
    let shape = h.app.model_shape(64);
    assert!(
        shape.heights.iter().all(|h| h.is_nan()),
        "the wrapped surface leaves the full model out"
    );
    assert!(h.app.shows_turned(), "the job is shown round its axis");
    // drawn where the import showed it, to the micrometre
    let shown = h.app.rotary_models(None);
    assert_eq!(shown.len(), importing.len());
    for ((a, _), (b, _)) in shown.iter().zip(&importing) {
        for (p, q) in a.iter().zip(b) {
            assert!(p.distance(*q) < 1e-3, "{p} vs {q}");
        }
    }
    let _ = std::fs::remove_file(file);
}

#[test]
fn a_rotary_model_along_y_is_drawn_with_its_axis_along_y() {
    use cut_craft_core::document::{Along, Job};
    let mut h = Harness::new();
    h.app.doc.job = Job::rotary(100.0, 30.0, Along::Y);
    let file = pyramid_stl("rotary-real-y");
    h.app.import_file(&file);
    h.frame(vec![]);
    let importing: Vec<_> = h
        .app
        .form
        .as_ref()
        .unwrap()
        .model
        .as_deref()
        .unwrap()
        .clone();
    h.app.form = None;
    let shown = h.app.rotary_models(None);
    for ((a, _), (b, _)) in shown.iter().zip(&importing) {
        for (p, q) in a.iter().zip(b) {
            assert!(p.distance(*q) < 1e-3, "{p} vs {q}");
        }
    }
    let _ = std::fs::remove_file(file);
}

#[test]
fn an_import_s_size_is_locked_and_set_once_whichever_way_it_is_turned() {
    use cut_craft_core::document::{Along, Job};
    let mut h = Harness::new();
    h.app.doc.job = Job::rotary(100.0, 30.0, Along::X);
    let file = pyramid_stl("size-once");
    h.app.import_file(&file);
    h.frame(vec![]);
    h.frame(vec![]);
    let form = h.app.form.as_ref().unwrap();
    assert_eq!(form.params["fit_material"], false, "not fitted unasked");
    assert_eq!(
        form.params["size"]["change_size"], false,
        "the size starts locked"
    );
    let shown = |h: &Harness| {
        h.app.form.as_ref().unwrap().params["size"]["x"]
            .as_f64()
            .unwrap()
    };
    // the 20 long pyramid at the file's size
    assert!((shown(&h) - 20.0).abs() < 0.01, "{}", shown(&h));
    // a length typed scales it once: twice as big
    let form = h.app.form.as_mut().unwrap();
    form.params["size"]["change_size"] = Value::Bool(true);
    form.params["size"]["x"] = Value::from(40.0);
    form.dirty = true;
    h.frame(vec![]);
    h.frame(vec![]);
    let scale = |h: &Harness| h.app.form.as_ref().unwrap().params["size"]["scale"].clone();
    assert_eq!(scale(&h), serde_json::json!([2.0, 2.0, 2.0]));
    // turned a quarter about Z: still twice as big, not sized again to 40 along X
    let form = h.app.form.as_mut().unwrap();
    form.params["turn"]["z"] = Value::from(90.0);
    form.dirty = true;
    h.frame(vec![]);
    h.frame(vec![]);
    assert_eq!(scale(&h), serde_json::json!([2.0, 2.0, 2.0]));
    // the pyramid's 10 wide side along the axis now: 20 long at twice
    assert!((shown(&h) - 20.0).abs() < 0.05, "{}", shown(&h));
    let _ = std::fs::remove_file(file);
}

/// Setup Material on a rotary job: every Shape choice shows whole, Board too (it was cut off
/// at the panel's edge), and the size is named for the shape chosen.
#[test]
fn setup_material_shows_every_shape_whole_and_names_the_size_for_it() {
    use cut_craft_core::document::{Along, Blank, Job, Rotary};
    for (blank, name) in [
        (Blank::Round, "Diameter (D)"),
        (Blank::Square, "Side (D)"),
        (Blank::Board { width: 150.0 }, "Thickness (D)"),
    ] {
        let mut h = Harness::new();
        let mut job = Job::rotary(300.0, 50.0, Along::X);
        job.rotary = job.rotary.map(|r| Rotary { blank, ..r });
        h.app.doc.job = job;
        // a system font a little wider than the tests' own, as on some desktops
        h.ctx.all_styles_mut(|style| {
            for font in style.text_styles.values_mut() {
                font.size *= 1.25;
            }
        });
        h.app.invoke(&h.ctx, "setup_material");
        for _ in 0..3 {
            h.frame(vec![]);
        }
        assert!(h.text_at(name).is_some(), "{blank:?}");
        for shape in ["Round", "Square", "Board"] {
            let (text, clip) = h
                .shapes
                .iter()
                .find_map(|c| match &c.shape {
                    egui::Shape::Text(t) if t.galley.text() == shape => Some((
                        egui::Rect::from_min_size(t.pos, t.galley.size()),
                        c.clip_rect,
                    )),
                    _ => None,
                })
                .unwrap_or_else(|| panic!("{shape} not shown"));
            assert!(
                clip.contains_rect(text.shrink(0.5)),
                "{shape} cut off: {text:?} in {clip:?}"
            );
        }
    }
}
