//! Calculating toolpaths: again, in place or as new, out of date, and progress.

use super::*;

fn cut_min_x(h: &Harness) -> f64 {
    cut_craft_core::testing::cut_bounds(h.doc().toolpaths.values().next().unwrap())
        .min
        .x
}

#[test]
fn toolpaths_follow_their_vectors_when_recalculated() {
    let mut h = Harness::new();
    h.rectangle(10.0, 10.0, 50.0, 50.0);
    h.invoke("profile_toolpath");
    let mut params = h.form_settings().clone();
    // straight down: a helix sits by the start point, which moves with the shape's place
    params["entry"] = serde_json::json!({ "mode": "plunge" });
    h.run("profile_toolpath", &params).unwrap();
    let before = cut_min_x(&h);
    let shift =
        serde_json::json!({ "relative": true, "x": 100.0, "y": 0.0, "anchor": "bottom_left" });
    h.run("move", &shift).unwrap();
    h.invoke("recalculate_all_toolpaths");
    assert!((cut_min_x(&h) - before - 100.0).abs() < 1e-6);
    h.invoke("undo");
    assert!(
        (cut_min_x(&h) - before).abs() < 1e-6,
        "recalculating is one undo step"
    );
}

#[test]
fn clearing_settings_are_the_jobs_an_edit_that_undoes_and_marks_toolpaths_out_of_date() {
    use cut_craft_core::entry::WayBack;
    let mut h = Harness::new();
    h.rectangle(10.0, 10.0, 50.0, 50.0);
    h.invoke("clearing_settings");
    let frames = |h: &mut Harness| {
        for _ in 0..3 {
            h.frame(vec![]);
        }
    };
    frames(&mut h);
    assert!(h.text_at("Light Step").is_some() && h.text_at("On the Way Back").is_some());
    h.click(
        h.text_at("Alternate").expect("the way back"),
        Modifiers::NONE,
    );
    frames(&mut h);
    assert_eq!(h.doc().job.clearing.way_back, WayBack::Alternate);
    // kept as the default for new jobs, then undone in this one
    h.click(
        h.text_at("Use as My Default").expect("the button"),
        Modifiers::NONE,
    );
    frames(&mut h);
    let default = |h: &Harness| h.options().default_job.map(|j| j.clearing.way_back);
    assert_eq!(default(&h), Some(WayBack::Alternate));
    h.invoke("undo");
    frames(&mut h);
    assert_eq!(h.doc().job.clearing.way_back, WayBack::KeepClimb);
    // an adaptive pocket, then the job back to the default: the pocket is out of date
    let mut params = h.opening_settings("pocket_toolpath");
    params["strategy"] = "adaptive".into();
    h.run("pocket_toolpath", &params).unwrap();
    frames(&mut h);
    h.click(
        h.text_at("Reset to My Defaults").expect("the button"),
        Modifiers::NONE,
    );
    frames(&mut h);
    assert_eq!(h.doc().job.clearing.way_back, WayBack::Alternate);
    assert!(h.doc().toolpaths.values().all(|t| t.stale), "out of date");
}

#[test]
fn a_toolpath_that_takes_a_while_shows_its_progress_then_lands_in_the_job() {
    let mut h = Harness::new();
    h.given_progress_shown_at_once();
    h.rectangle(10.0, 10.0, 50.0, 50.0);
    h.invoke("profile_toolpath");
    let params = h.form_settings().clone();
    h.apply_form("profile_toolpath", &params).unwrap();
    until_not_busy(&mut h);
    assert_eq!(h.doc().toolpaths.len(), 1, "made once the job was done");
    assert!(h.can_undo(), "as one undo, like any other");
}

#[test]
fn a_slow_job_shows_a_progress_window_that_cancels_with_nothing_changed() {
    let mut h = Harness::new();
    h.given_progress_shown_at_once();
    h.rectangle(10.0, 10.0, 50.0, 50.0);
    let (go, wait) = std::sync::mpsc::channel::<()>();
    let (reported, said) = std::sync::mpsc::channel::<()>();
    let before = h.job_revision();
    h.start_slow_job("Calculating Slow Toolpath", move || {
        cut_craft_core::progress::report(0.4);
        reported.send(()).ok();
        wait.recv().ok();
    });
    assert!(h.busy(), "still running");
    // the job's thread may start late (Windows): the bar is drawn once it has reported
    said.recv_timeout(std::time::Duration::from_secs(30))
        .expect("the job reported its progress");
    h.frame(vec![]);
    h.frame(vec![]);
    assert!(h.text_at("Calculating Slow Toolpath…").is_some());
    assert!(h.text_at("40%").is_some());
    // keys do nothing under the window
    let undo = Event::Key {
        key: Key::Z,
        physical_key: None,
        pressed: true,
        repeat: false,
        modifiers: Modifiers::COMMAND,
    };
    h.frame_holding(Modifiers::COMMAND, vec![undo]);
    assert_eq!(h.job_revision(), before, "no undo behind the window");
    let cancel = h.text_at("Cancel").expect("a Cancel button");
    h.click(cancel, Modifiers::NONE);
    h.frame(vec![]);
    assert!(!h.busy(), "the window is gone at once");
    assert!(
        h.status_message().contains("cancelled"),
        "{}",
        h.status_message()
    );
    go.send(()).ok();
    h.frame(vec![]);
    assert_ne!(h.status_message(), "used", "what it made was let go");
}

#[test]
fn moving_what_a_toolpath_was_made_from_says_it_is_out_of_date() {
    let mut h = Harness::new();
    h.rectangle(10.0, 10.0, 40.0, 30.0);
    let vector = *h.doc().vectors.keys().next().unwrap();
    let mut c = Change::builder(h.doc());
    c.add_toolpath(cut_craft_core::toolpath::Toolpath {
        name: "Profile 1".into(),
        strategy: "test".into(),
        params: serde_json::Value::Null,
        tool: Tool::default(),
        vectors: vec![vector],
        moves: vec![],
        visible: true,
        power: vec![],
        stale: false,
        group: None,
        drill_bank: None,
        fallbacks: vec![],
        across: vec![],
    });
    h.make(c.finish(), "made");
    h.frame(vec![]);
    assert!(!h.status_message().contains("out of date"));
    h.select([vector]);
    h.frame(vec![]);
    let (from, to) = (h.at(30.0, 25.0), h.at(40.0, 25.0));
    h.drag(from, to);
    h.frame(vec![]);
    let message = h.status_message();
    assert!(
        message.contains("1 toolpath is now out of date") && message.contains("G-code"),
        "{message}"
    );
}

#[test]
fn plate_production_recalculates_toolpaths_on_placeholder_text_for_each_plates_text() {
    let mut h = Harness::new();
    let mut text = h.default_settings("draw_text");
    text["text"] = "!!name!!".into();
    text["font"] = "Test Boxes".into();
    text["height"] = 5.0.into();
    text["anchor"] = serde_json::json!({ "x": 10.0, "y": 10.0 });
    h.run("draw_text", &text).unwrap();
    h.close_form();
    h.invoke("profile_toolpath");
    let profile = h.form_settings().clone();
    h.run("profile_toolpath", &profile).unwrap();
    h.close_form();
    let template = h.doc().toolpaths.values().next().unwrap().clone();
    let file =
        std::env::temp_dir().join(format!("cut-craft-app-plates-{}.csv", std::process::id()));
    std::fs::write(&file, "name\nI\n").unwrap();
    let mut p = h.opening_settings("plate_production");
    p["data"]["file"] = file.display().to_string().into();
    h.run("plate_production", &p).unwrap();
    let plate = h
        .doc()
        .toolpaths
        .values()
        .find(|t| t.name.starts_with("S1 - "))
        .expect("the plates' toolpath, named after its sheet");
    assert!(!plate.stale, "recalculated straight away");
    // one letter box, not the eight of !!name!!: far fewer moves than the template's
    assert!(
        plate.moves.len() < template.moves.len(),
        "{} moves vs the template's {}",
        plate.moves.len(),
        template.moves.len()
    );
}

#[test]
fn an_edit_that_leaves_toolpaths_out_of_date_asks_to_recalculate_them() {
    let mut h = pocketed();
    assert!(h.text_at("Toolpaths out of date").is_none());
    move_first_vector(&mut h);
    h.frame(vec![]);
    h.frame(vec![]);
    assert!(h.text_at("Toolpaths out of date").is_some(), "asked");
    let yes = h.text_at("Recalculate All").expect("the button");
    h.click(yes, Modifiers::NONE);
    h.frame(vec![]);
    assert!(h.text_at("Toolpaths out of date").is_none(), "answered");
    assert!(h.doc().toolpaths.values().all(|t| !t.stale), "recalculated");
}

#[test]
fn the_recalculate_question_counts_the_toolpaths() {
    assert!(form_apply::recalculate_question(1).starts_with("1 toolpath is now out of date"));
    assert!(form_apply::recalculate_question(3).starts_with("3 toolpaths are now out of date"));
}

#[test]
fn toolpath_forms_calculate_and_selection_tools_apply() {
    let mut h = profile_form();
    assert!(h.text_at("Calculate").is_some());
    assert!(h.text_at("Apply").is_none());
    h.close_form();
    h.invoke("offset_vectors");
    h.frame(vec![]);
    assert!(h.text_at("Apply").is_some());
    h.close_form();
    h.invoke("circular_copy");
    h.frame(vec![]);
    assert!(h.text_at("Create").is_some());
}

/// A Profile toolpath round a 40 mm square, its form still open after Calculate.
fn calculated_profile() -> (Harness, EntityId) {
    let mut h = Harness::new();
    h.rectangle(10.0, 10.0, 50.0, 50.0);
    h.invoke("profile_toolpath");
    let params = h.form_settings().clone();
    h.apply_form("profile_toolpath", &params).unwrap();
    assert_eq!(h.doc().toolpaths.len(), 1);
    let id = *h.doc().toolpaths.keys().next().unwrap();
    (h, id)
}

fn press_calculate(h: &mut Harness) {
    let params = h.form_settings().clone();
    h.apply_form("profile_toolpath", &params).unwrap();
}

#[test]
fn calculate_again_with_nothing_changed_keeps_the_toolpath_and_says_it_is_up_to_date() {
    let (mut h, id) = calculated_profile();
    let (depth, revision) = (h.undo_steps(), h.job_revision());
    press_calculate(&mut h);
    assert_eq!(h.doc().toolpaths.len(), 1, "no second copy");
    assert!(h.doc().toolpaths.contains_key(&id));
    assert_eq!(h.undo_steps(), depth, "nothing to undo");
    assert_eq!(h.job_revision(), revision);
    let note = h.form_note().expect("said on the form");
    assert!(note.starts_with("Already up to date"), "{note}");
}

#[test]
fn calculate_after_a_change_of_settings_updates_the_toolpath_in_place() {
    let (mut h, id) = calculated_profile();
    h.set_form_value("/name", Value::from("Outside cut"));
    assert_eq!(h.form_note(), None, "the note goes once a setting changes");
    press_calculate(&mut h);
    assert_eq!(h.doc().toolpaths.len(), 1, "updated, not added");
    assert_eq!(h.doc().toolpaths[&id].name, "Outside cut", "in its place");
    // and pressed again: up to date with the new settings
    let depth = h.undo_steps();
    press_calculate(&mut h);
    assert_eq!(h.undo_steps(), depth);
    // one undo takes the update back, the toolpath staying
    h.invoke("undo");
    assert_eq!(h.doc().toolpaths.len(), 1);
    assert_ne!(h.doc().toolpaths[&id].name, "Outside cut");
}

#[test]
fn calculate_after_undo_makes_the_toolpath_again_rather_than_calling_it_up_to_date() {
    let (mut h, id) = calculated_profile();
    h.set_form_value("/name", Value::from("Outside cut"));
    press_calculate(&mut h);
    h.invoke("undo");
    press_calculate(&mut h);
    assert_eq!(h.doc().toolpaths.len(), 1);
    assert_eq!(h.doc().toolpaths[&id].name, "Outside cut");
    // the first Calculate undone too: the toolpath is made anew
    h.invoke("undo");
    h.invoke("undo");
    assert!(h.doc().toolpaths.is_empty());
    press_calculate(&mut h);
    assert_eq!(h.doc().toolpaths.len(), 1);
}

#[test]
fn calculate_after_the_vectors_changed_works_the_toolpath_out_again_in_place() {
    let (mut h, id) = calculated_profile();
    // the square moved: the toolpath is out of date
    let moved = cut_craft_core::Change::transform_selected(h.doc(), |_| {
        cut_craft_core::glam::DAffine2::from_translation((5.0, 0.0).into())
    })
    .unwrap();
    h.make(moved, "move");
    assert!(h.doc().toolpaths[&id].stale);
    press_calculate(&mut h);
    assert_eq!(h.doc().toolpaths.len(), 1);
    assert!(!h.doc().toolpaths[&id].stale, "worked out again");
    assert_eq!(h.form_note(), None, "not called up to date");
}

#[test]
fn calculate_as_new_adds_another_toolpath_and_follows_that_one() {
    let (mut h, id) = calculated_profile();
    h.calculate_as_new();
    assert_eq!(h.doc().toolpaths.len(), 2, "a second one, asked for");
    let new = *h.doc().toolpaths.keys().last().unwrap();
    assert_ne!(new, id);
    h.set_form_value("/name", Value::from("Second"));
    press_calculate(&mut h);
    assert_eq!(h.doc().toolpaths.len(), 2);
    assert_eq!(
        h.doc().toolpaths[&new].name,
        "Second",
        "the new one updated"
    );
    assert_ne!(h.doc().toolpaths[&id].name, "Second");
}

#[test]
fn calculate_with_other_vectors_picked_makes_a_new_toolpath() {
    let (mut h, _) = calculated_profile();
    h.close_form();
    h.rectangle(80.0, 10.0, 20.0, 20.0);
    h.invoke("profile_toolpath");
    // the form opened again remembers nothing: a toolpath for the new square
    press_calculate(&mut h);
    assert_eq!(h.doc().toolpaths.len(), 2);
    // the same form, other vectors picked: another, the first kept
    let squares: Vec<EntityId> = h.doc().vectors.keys().copied().collect();
    h.select([squares[0]]);
    press_calculate(&mut h);
    assert_eq!(h.doc().toolpaths.len(), 3);
}

#[test]
fn calculate_with_nothing_picked_updates_the_toolpath_from_its_own_vectors() {
    let (mut h, id) = calculated_profile();
    h.select([]);
    h.set_form_value("/name", Value::from("Kept"));
    press_calculate(&mut h);
    assert_eq!(h.doc().toolpaths.len(), 1);
    assert_eq!(h.doc().toolpaths[&id].name, "Kept");
    assert_ne!(h.doc().toolpaths[&id].vectors, Vec::new());
}

#[test]
fn edit_toolpath_calculate_with_nothing_changed_says_it_is_up_to_date() {
    let (mut h, id) = calculated_profile();
    h.close_form();
    h.select_toolpath(id);
    h.invoke("edit_toolpath");
    let depth = h.undo_steps();
    press_calculate(&mut h);
    assert_eq!(h.undo_steps(), depth, "not made again");
    assert!(
        h.form_note()
            .is_some_and(|n| n.starts_with("Already up to date"))
    );
}
