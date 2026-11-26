//! Previewing toolpaths: the preview, playing the cut, the uncut stock.

use super::*;

#[test]
fn a_model_past_the_preview_limit_is_drawn_as_its_surface_with_only_undercuts_over_it() {
    let mut h = Harness::new();
    h.given_options(|o| o.import.preview_faces = 1000);
    let file = pyramid_stl("limit");
    h.import(&file);
    h.frame(vec![]);
    assert!(h.import_drawn_whole() && h.importing_model().is_some_and(|m| m.len() == 6));
    let k = *h.doc().components.keys().next().unwrap();
    assert!(
        h.model_shape(MESH_CELLS).heights.iter().all(|z| z.is_nan()),
        "the whole model is drawn in the component's place"
    );
    // over the limit: the component shows, and the model keeps only its red faces (none)
    h.given_options(|o| o.import.preview_faces = 4);
    h.touch_form();
    h.frame(vec![]);
    assert!(!h.import_drawn_whole());
    assert_eq!(
        h.importing_model().map(<[_]>::len),
        Some(0),
        "a pyramid has no undercuts"
    );
    assert!(
        h.model_shape(MESH_CELLS)
            .heights
            .iter()
            .any(|z| !z.is_nan()),
        "the component is drawn instead"
    );
    assert!(h.doc().components[&k].visible);
    let _ = std::fs::remove_file(file);
}

#[test]
fn a_toolpath_preview_becomes_a_component() {
    let mut h = Harness::new();
    h.rectangle(10.0, 10.0, 50.0, 50.0);
    let params = h.opening_settings("pocket_toolpath");
    h.run("pocket_toolpath", &params).unwrap();
    assert!(!h.available("create_component_from_toolpath_preview"));
    h.invoke("preview_all_toolpaths");
    h.invoke("create_component_from_toolpath_preview");
    let made = h.doc().components.values().next().expect("a component");
    let thickness = h.doc().job.thickness;
    let (lo, hi) = made.height_range().unwrap();
    assert!((hi - thickness).abs() < 1e-9 && lo < hi, "{lo} {hi}");
}

/// Trace Bitmap opens its trace window, whose preview traces off the window's thread and
/// follows the settings as they change; Apply in it traces into the job.
#[test]
fn trace_bitmap_previews_in_its_window_and_follows_the_settings() {
    use cut_craft_core::bitmap::Bitmap;
    use cut_craft_core::geometry::{Point, Rect};
    let mut h = Harness::new();
    let mut c = Change::builder(h.doc());
    // white round a red square and a blue one
    let (w, rows) = (40, 20);
    let mut rgb = vec![];
    for r in 0..rows {
        for col in 0..w {
            let inside = (5..15).contains(&r);
            rgb.extend(match col {
                5..15 if inside => [200, 20, 20],
                25..35 if inside => [20, 20, 200],
                _ => [250, 250, 250],
            });
        }
    }
    c.add_bitmap(Bitmap {
        name: "b".into(),
        columns: w,
        rows,
        grey: rgb.chunks(3).map(|p| p[1]).collect(),
        rect: Rect::new(Point::new(0.0, 0.0), Point::new(80.0, 40.0)),
        depth: Vec::new(),
        rgb,
    });
    h.make(c.finish(), "added");
    h.invoke("trace_bitmap");
    let settle = |h: &mut Harness, want: usize| {
        for _ in 0..400 {
            h.frame(vec![]);
            if h.trace_up_to_date() && h.traced_colours() == Some(want) {
                return;
            }
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        panic!(
            "the preview never showed {want} colours: {:?}",
            h.traced_colours()
        );
    };
    // black and white: the two squares are dark enough at the threshold
    settle(&mut h, 1);
    h.edit_form(|p| p["mode"] = "colors".into());
    settle(&mut h, 2);
    // untick live: a change waits for Update preview
    h.untick_live_trace();
    h.edit_form(|p| p["trace"] = serde_json::json!(["#c81414"]));
    for _ in 0..30 {
        h.frame(vec![]);
    }
    assert_eq!(h.traced_colours(), Some(2));
    // Quick Trace in the window picks settings to suit the picture: colours, as it has some
    h.edit_form(|p| p["mode"] = "black_and_white".into());
    h.frame(vec![]);
    let quick = h
        .text_at("Quick Trace")
        .expect("the window's Quick Trace button");
    h.click(quick, Modifiers::NONE);
    h.frame(vec![]);
    assert_eq!(h.form_value("mode"), "colors");
    // closing the form lets the window go
    h.close_form();
    h.frame(vec![]);
    assert!(!h.trace_window_open());
}

#[test]
fn previewing_all_toolpaths_is_previewing_each_in_turn() {
    let mut h = Harness::new();
    h.rectangle(10.0, 10.0, 50.0, 50.0);
    for id in ["drilling_toolpath", "profile_toolpath"] {
        h.invoke(id);
        let params = h.form_settings().clone();
        h.run(id, &params).unwrap();
    }
    h.invoke("preview_all_toolpaths");
    assert_eq!(*h.view(), View::ThreeD, "previewing shows the 3D view");
    h.frame(vec![]);
    let all = h.previewed().cloned().expect("a preview");
    h.invoke("reset_preview");
    assert!(h.previewed().is_none());
    for id in h.doc().toolpaths.keys().copied().collect::<Vec<_>>() {
        h.select_toolpath(id);
        h.invoke("preview_toolpath");
    }
    assert_eq!(h.previewed().cloned(), Some(all));
}

#[test]
fn undo_last_preview_brings_back_a_reset_preview() {
    let mut h = Harness::new();
    h.given_animated_preview(false);
    h.rectangle(10.0, 10.0, 50.0, 50.0);
    h.invoke("profile_toolpath");
    let params = h.form_settings().clone();
    h.run("profile_toolpath", &params).unwrap();
    h.close_form();
    h.invoke("preview_all_toolpaths");
    h.frame(vec![]);
    let previewed = h.previewed().cloned().expect("a preview");
    h.invoke("reset_preview");
    assert!(h.previewed().is_none(), "reset");
    assert!(h.available("undo_last_preview"), "the reset can be undone");
    h.invoke("undo_last_preview");
    assert_eq!(h.previewed().cloned(), Some(previewed));
}

#[test]
fn the_3d_view_shows_the_model_until_there_is_a_preview() {
    let mut h = Harness::new();
    h.show_design_tab("Design");
    assert!(
        h.toolbar()
            .modeling_groups
            .iter()
            .any(|g| g.tools.contains(&"create_shape_round"))
    );
    h.rectangle(10.0, 10.0, 50.0, 50.0);
    let shape = h.default_settings("create_shape");
    h.run("create_shape", &shape).unwrap();
    h.show_view(View::ThreeD);
    h.frame(vec![]);
    // off the model: cut down to the model's base before a preview (the model in the
    // material where Material Setup puts it), the material's uncut top in a preview that
    // does not cut there
    let off = job_corner(&h) + Point::new(1.0, 1.0);
    let raised = |h: &mut Harness| h.surface_3d().unwrap().at(off).unwrap() < 0.0;
    assert!(raised(&mut h), "the model");
    let profile = h.default_settings("profile_toolpath");
    h.run("profile_toolpath", &profile).unwrap();
    h.invoke("preview_all_toolpaths");
    h.frame(vec![]);
    assert!(!raised(&mut h), "the preview, cut down from the top");
}

#[test]
fn a_preview_plays_pauses_scrubs_and_keeps_what_is_cut() {
    let mut h = Harness::new();
    h.rectangle(10.0, 10.0, 50.0, 50.0);
    h.invoke("profile_toolpath");
    let params = h.form_settings().clone();
    h.run("profile_toolpath", &params).unwrap();
    h.close_form();
    h.given_animated_preview(true);
    h.invoke("preview_all_toolpaths");
    assert!(h.playing() && h.playback().is_some(), "it plays");
    assert_eq!(*h.view(), View::ThreeD);
    h.frame(vec![]);
    h.frame(vec![]);
    let along = h.playback().unwrap().at();
    assert!(along > 0.0, "the tool moves on each frame");
    assert!(h.text_at("Pause").is_some() && h.text_at("Run to Retract").is_some());
    h.click(h.text_at("Pause").unwrap(), Modifiers::NONE);
    assert!(!h.playing());
    let paused = h.playback().unwrap().at();
    h.frame(vec![]);
    assert_eq!(h.playback().unwrap().at(), paused, "paused stays put");
    // the Progress slider, dragged right, moves the tool on
    let progress = h.text_at("Progress").expect("a Progress slider");
    let from = progress + egui::vec2(80.0, 0.0);
    h.drag(from, from + egui::vec2(200.0, 0.0));
    let on = h.playback().unwrap().at();
    assert!(on > paused, "dragged on: {paused} to {on}");
    assert!(!h.playing(), "a drag pauses it");
    h.click(h.text_at("Stop").unwrap(), Modifiers::NONE);
    assert!(
        h.playback().is_none() && h.previewed().is_some(),
        "kept as far as it went"
    );
}

#[test]
fn escape_stops_a_playing_preview_keeping_what_is_cut_and_leaves_the_form_open() {
    let mut h = playing_preview();
    h.invoke("profile_toolpath");
    h.frame(vec![]);
    h.frame(vec![key(Key::Escape)]);
    assert!(
        h.playback().is_none() && h.previewed().is_some(),
        "stopped, kept as far as it went"
    );
    assert!(
        h.form_id().is_some(),
        "the one Escape only stopped the preview"
    );
    h.frame(vec![key(Key::Escape)]);
    assert!(h.form_id().is_none(), "the next closes the form as ever");
}

#[test]
fn a_preview_can_be_stopped_from_the_2d_view_too() {
    let mut h = playing_preview();
    h.show_view(View::TwoD);
    h.frame(vec![]);
    let along = h.playback().unwrap().at();
    h.frame(vec![]);
    assert!(
        h.playback().unwrap().at() > along,
        "it plays on in the 2D view"
    );
    h.click(
        h.text_at("Stop").expect("Stop over the 2D view"),
        Modifiers::NONE,
    );
    assert!(h.playback().is_none() && h.previewed().is_some());
}

#[test]
fn a_playing_preview_says_how_long_it_has_left() {
    let mut h = playing_preview();
    h.set_playback_speed(1.0);
    h.frame(vec![]);
    let note = |h: &Harness| {
        h.texts()
            .into_iter()
            .find(|t| t.starts_with("Machine time left"))
            .expect("the time the machine takes")
    };
    assert!(!note(&h).contains(" at ×"), "{}", note(&h));
    h.set_playback_speed(10.0);
    h.frame(vec![]);
    assert!(note(&h).contains(" at ×10"), "{}", note(&h));
}

#[test]
fn the_preview_controls_stay_after_stop_to_scrub_back_until_reset() {
    let mut h = playing_preview();
    h.click(h.text_at("Stop").unwrap(), Modifiers::NONE);
    assert!(h.playback().is_none() && h.previewed().is_some(), "stopped");
    assert!(h.text_at("Progress").is_some(), "the slider is still there");
    assert!(h.text_at("Close up").is_some(), "and the fine one");
    // the slider dragged back: playing it again from there, paused
    let progress = h.text_at("Progress").unwrap();
    let from = progress + egui::vec2(200.0, 0.0);
    h.drag(from, from - egui::vec2(100.0, 0.0));
    assert!(h.playback().is_some() && !h.playing(), "scrubbing again");
    h.invoke("reset_preview");
    h.frame(vec![]);
    assert!(h.text_at("Progress").is_none(), "gone with the preview");
}

#[test]
fn the_preview_controls_stay_once_it_has_played_to_the_end() {
    let mut h = playing_preview();
    h.set_playback_speed(1000.0);
    for _ in 0..600 {
        if h.playback().is_none() {
            break;
        }
        h.frame(vec![]);
    }
    assert!(
        h.playback().is_none() && h.previewed().is_some(),
        "played through"
    );
    h.frame(vec![]);
    assert!(h.text_at("Progress").is_some(), "still there to scrub back");
    h.click(
        h.text_at("Close").expect("Close hides them"),
        Modifiers::NONE,
    );
    assert!(h.text_at("Progress").is_none() && h.previewed().is_some());
}

#[test]
fn a_paused_preview_is_not_rebuilt_for_the_3d_view_every_frame() {
    let mut h = playing_preview();
    h.click(h.text_at("Pause").unwrap(), Modifiers::NONE);
    h.frame(vec![]);
    let built = h.playback_builds();
    for _ in 0..3 {
        h.frame(vec![]);
    }
    assert_eq!(h.playback_builds(), built, "kept while it shows the same");
    h.click(h.text_at("Single Step").unwrap(), Modifiers::NONE);
    h.frame(vec![]);
    assert!(h.playback_builds() > built, "made again once it cut on");
}

#[test]
fn a_playing_preview_is_drawn_at_full_detail_and_redrawn_a_few_times_a_second() {
    let mut h = playing_preview();
    h.given_options(|o| o.performance.adjust_to_frame_rate = false);
    // a test's frames are slow: the redraws are kept well apart to see them kept
    h.given_playback_redraw(std::time::Duration::from_secs(3600));
    h.frame(vec![]);
    h.frame(vec![]);
    let (_, cells) = h.playback_drawn().expect("drawn");
    assert_eq!(
        cells,
        h.options().performance.still_cells(),
        "as sharp as when still, not the moving detail"
    );
    // frames straight after one another: the cut so far is kept, not made again each one
    let built = h.playback_builds();
    for _ in 0..5 {
        h.frame(vec![]);
    }
    assert!(
        h.playback_builds() <= built + 1,
        "{built} then {}",
        h.playback_builds()
    );
    // once it gets where it was sent, that is drawn at once
    h.click(h.text_at("Pause").unwrap(), Modifiers::NONE);
    h.click(h.text_at("Single Step").unwrap(), Modifiers::NONE);
    h.frame(vec![]);
    let shown = h.playback_drawn().unwrap().0;
    assert_eq!(shown, h.playback().unwrap().revision());
}

#[test]
fn a_long_jump_is_cut_a_little_each_frame_and_stop_ends_it() {
    let mut h = playing_preview();
    // a step a frame, as a toolpath of millions of moves would go
    h.given_playback_budget(std::time::Duration::ZERO);
    h.click(h.text_at("Pause").unwrap(), Modifiers::NONE);
    let from = h.playback().unwrap().at();
    h.click(h.text_at("Run to Retract").unwrap(), Modifiers::NONE);
    let goal = h.playback_goal().expect("on its way there");
    let at = h.playback().unwrap().at();
    assert!(at > from && at < goal, "part way: {from} to {at} of {goal}");
    assert!(
        h.texts().iter().any(|t| t.contains("Esc")),
        "says how to stop it"
    );
    h.click(h.text_at("Stop").unwrap(), Modifiers::NONE);
    assert!(h.playback().is_none() && h.playback_goal().is_none());
}

#[test]
fn preview_visible_leaves_hidden_toolpaths_out() {
    let mut h = Harness::new();
    h.rectangle(10.0, 10.0, 50.0, 50.0);
    for id in ["drilling_toolpath", "profile_toolpath"] {
        let params = h.opening_settings(id);
        h.run(id, &params).unwrap();
    }
    let profile = *h.doc().toolpaths.keys().last().unwrap();
    h.select_toolpath(profile);
    h.invoke("preview_toolpath");
    let only_profile = h.previewed().cloned();
    let drilling = *h.doc().toolpaths.keys().next().unwrap();
    h.given_document(|d| d.toolpaths.get_mut(&drilling).unwrap().visible = false);
    h.invoke("preview_visible_toolpaths");
    assert_eq!(h.previewed().cloned(), only_profile);
}

#[test]
fn a_double_click_on_the_preview_takes_away_the_piece_cut_free_under_it() {
    use cut_craft_core::glam::DVec3;
    use cut_craft_core::toolpath::{Move, MoveKind, Tool, Toolpath};
    let mut h = Harness::new();
    h.given_document(|d| {
        d.job.width = 100.0;
        d.job.height = 100.0;
    });
    let job = h.doc().job;
    let mut m = cut_craft_core::simulation::Material::new(&job, 200);
    // a ring cut right through round the middle
    let ring = [
        (30.0, 30.0),
        (70.0, 30.0),
        (70.0, 70.0),
        (30.0, 70.0),
        (30.0, 30.0),
    ];
    m.cut(&Toolpath {
        name: "cut out".into(),
        strategy: "test".into(),
        params: serde_json::Value::Null,
        tool: Tool::default(),
        vectors: vec![],
        moves: ring
            .iter()
            .map(|&(x, y)| Move {
                kind: MoveKind::Cut,
                to: DVec3::new(x, y, -job.thickness - 1.0),
            })
            .collect(),
        visible: true,
        power: vec![],
        stale: false,
        group: None,
        drill_bank: None,
        fallbacks: vec![],
        across: vec![],
    });
    h.given_previewed(Some(m));
    h.given_orbit(view3d::Orbit::TOP);
    let fit = egui::Rect::from_min_size(egui::pos2(0.0, 0.0), egui::vec2(400.0, 400.0));
    // the middle of the view is the part: it goes, the waste round it stays
    assert!(h.remove_preview_piece(fit, fit.center()));
    let sim = h.previewed().unwrap();
    let at = |x, y| sim.at(cut_craft_core::geometry::Point::new(x, y)).unwrap();
    assert!(at(50.0, 50.0).is_nan());
    assert_eq!(at(5.0, 5.0), 0.0);
    assert!(
        h.status_message().contains("Undo Last Preview"),
        "says how to bring it back: {}",
        h.status_message()
    );
    assert!(!h.remove_preview_piece(fit, fit.center()), "already gone");
}

#[test]
fn double_clicking_an_uncut_preview_leaves_the_block() {
    let mut h = Harness::new();
    let job = h.doc().job;
    h.given_previewed(Some(cut_craft_core::simulation::Material::new(&job, 200)));
    h.given_orbit(view3d::Orbit::TOP);
    let fit = egui::Rect::from_min_size(egui::pos2(0.0, 0.0), egui::vec2(400.0, 400.0));
    assert!(
        !h.remove_preview_piece(fit, fit.center()),
        "nothing cut free"
    );
    let sim = h.previewed().unwrap();
    assert!(sim.heights.iter().all(|h| !h.is_nan()), "the block stays");
    assert!(!h.preview_before_kept());
}

#[test]
fn preview_visible_leaves_hidden_toolpaths_out_and_undo_last_steps_back_one_preview() {
    let mut h = Harness::new();
    h.given_animated_preview(false);
    h.rectangle(10.0, 10.0, 50.0, 50.0);
    for id in ["drilling_toolpath", "profile_toolpath"] {
        h.invoke(id);
        let params = h.form_settings().clone();
        h.run(id, &params).unwrap();
    }
    let ids: Vec<EntityId> = h.doc().toolpaths.keys().copied().collect();
    h.invoke("preview_all_toolpaths");
    let all = h.previewed().cloned().unwrap();
    // hide the profile: Preview Visible cuts the drilling only, Preview All both
    h.given_document(|d| d.toolpaths.get_mut(&ids[1]).unwrap().visible = false);
    h.invoke("preview_visible_toolpaths");
    let visible = h.previewed().cloned().unwrap();
    assert_ne!(visible, all);
    h.invoke("preview_all_toolpaths");
    assert_eq!(h.previewed(), Some(&all));
    // undo goes back to before the last preview, once
    h.invoke("undo_last_preview");
    assert_eq!(h.previewed(), Some(&visible));
    assert!(!h.available("undo_last_preview"));
    h.select_toolpath(ids[1]);
    h.invoke("preview_toolpath");
    assert_eq!(h.previewed(), Some(&all));
    h.invoke("undo_last_preview");
    assert_eq!(h.previewed(), Some(&visible));
}

#[test]
fn a_double_sided_preview_cuts_both_sides_and_turns_over_with_the_job() {
    let mut h = Harness::new();
    h.given_animated_preview(false);
    h.given_document(|d| {
        d.job.double_sided = Some(cut_craft_core::document::DoubleSided::default());
    });
    // a pocket on this side, a pocket on the other
    let pocket = |h: &mut Harness| {
        h.rectangle(10.0, 10.0, 40.0, 40.0);
        h.invoke("pocket_toolpath");
        let params = h.form_settings().clone();
        h.run("pocket_toolpath", &params).unwrap();
        h.close_form();
    };
    pocket(&mut h);
    h.run("swap_sides", &serde_json::json!({})).unwrap();
    pocket(&mut h);
    h.run("swap_sides", &serde_json::json!({})).unwrap();
    assert_eq!(h.doc().other_side.toolpaths.len(), 1);
    h.invoke("preview_all_toolpaths");
    let preview = h.previewed().expect("previewed");
    assert!(
        preview.other_side.is_some(),
        "the other side is previewed too"
    );
    // the 3D view's surface has the other side's pocket on its underside
    let surface = h.surface_3d().unwrap();
    let bottom = surface.bottom;
    assert!(
        surface.under.iter().any(|&z| z > bottom + 1e-6),
        "the underside is cut"
    );
    // turned over, the other side's cuts are on top and this side's underneath
    let before = h.previewed().cloned().unwrap();
    h.run("swap_sides", &serde_json::json!({})).unwrap();
    let after = h.previewed().expect("still previewed");
    assert_eq!(
        after.other_side.as_deref().map(|m| &m.heights),
        Some(&before.heights),
        "the side that was up is now underneath"
    );
    h.frame(vec![]);
}

#[test]
fn undoing_swap_sides_turns_the_preview_back_and_redo_turns_it_over_again() {
    let mut h = Harness::new();
    h.given_animated_preview(false);
    h.given_document(|d| {
        d.job.double_sided = Some(cut_craft_core::document::DoubleSided::default());
    });
    h.rectangle(10.0, 10.0, 40.0, 40.0);
    h.invoke("pocket_toolpath");
    let params = h.form_settings().clone();
    h.run("pocket_toolpath", &params).unwrap();
    h.close_form();
    h.invoke("preview_all_toolpaths");
    let up = h.previewed().cloned().expect("previewed").heights;
    assert!(up.iter().any(|&z| z < 0.0), "the pocket is cut");
    // only this side has toolpaths: turned over, its preview is kept underneath
    h.run("swap_sides", &serde_json::json!({})).unwrap();
    h.frame(vec![]);
    let turned = h.previewed().expect("the preview is kept");
    assert!(
        turned.heights.iter().all(|&z| z == 0.0),
        "nothing cut on this side"
    );
    assert_eq!(
        turned.other_side.as_deref().map(|m| &m.heights),
        Some(&up),
        "the side that was up is underneath"
    );
    h.invoke("undo");
    h.frame(vec![]);
    assert!(!h.doc().bottom_side);
    assert_eq!(
        h.previewed().map(|m| &m.heights),
        Some(&up),
        "undone, the side that was up is up again"
    );
    h.invoke("redo");
    h.frame(vec![]);
    let again = h.previewed().expect("still previewed");
    assert_eq!(again.other_side.as_deref().map(|m| &m.heights), Some(&up));
}

#[test]
fn toolpath_groups_are_listed_picked_previewed_saved_as_one_and_joined_by_new_toolpaths() {
    let mut h = Harness::new();
    h.open_document(cut_craft_core::testing::three_toolpaths());
    h.given_job_screen();
    h.pin(Page::Toolpaths);
    // Group Visible, with the third toolpath hidden
    let hide = serde_json::json!({ "toolpath": 3, "name": "Profile", "visible": false });
    h.run("toolpath_properties", &hide).unwrap();
    let shown: Vec<u64> = h
        .doc()
        .toolpaths
        .iter()
        .filter(|(_, t)| t.visible)
        .map(|(id, _)| id.0)
        .collect();
    let group =
        serde_json::json!({ "toolpaths": shown, "group": h.doc().next_toolpath_group_name() });
    h.run("group_toolpaths", &group).unwrap();
    assert_eq!(h.doc().toolpath_groups, ["Group 1"]);
    h.frame(vec![]);
    h.frame(vec![]);
    // the group's row is drawn; a click picks the group instead of a toolpath
    h.select_toolpath(EntityId(3));
    let row = h.text_at("Group 1").unwrap();
    h.click(row, Modifiers::NONE);
    assert_eq!(h.selected_toolpath_group(), Some("Group 1"));
    assert_eq!(h.selected_toolpath(), None);
    // Save Toolpaths' Selected mode saves the group as one file
    let plan = files::save_plan(
        h.doc(),
        files::SaveMode::Selected,
        None,
        Some("Group 1"),
        false,
    );
    assert_eq!(plan.unwrap(), [vec![EntityId(1), EntityId(2)]]);
    // a toolpath made while the group is picked joins it
    h.rectangle(10.0, 10.0, 50.0, 50.0);
    h.invoke("drilling_toolpath");
    let params = h.form_settings().clone();
    h.run("drilling_toolpath", &params).unwrap();
    let newest = h.doc().toolpaths.values().last().unwrap();
    assert_eq!(newest.group.as_deref(), Some("Group 1"));
    // Preview with the group picked previews its toolpaths
    h.given_animated_preview(false);
    assert!(h.available("preview_toolpath"));
    h.invoke("preview_toolpath");
    assert!(h.previewed().is_some());
    // ungrouping keeps the toolpaths
    let ungroup = serde_json::json!({ "group": "Group 1", "delete_toolpaths": false });
    h.run("ungroup_toolpaths", &ungroup).unwrap();
    assert_eq!(h.doc().toolpath_groups, Vec::<String>::new());
    assert!(h.doc().toolpaths.values().all(|t| t.group.is_none()));
}

#[test]
fn a_preview_cut_off_the_windows_thread_becomes_the_preview() {
    let mut h = Harness::new();
    h.given_progress_shown_at_once();
    h.rectangle(10.0, 10.0, 50.0, 50.0);
    h.invoke("profile_toolpath");
    let params = h.form_settings().clone();
    h.apply_form("profile_toolpath", &params).unwrap();
    until_not_busy(&mut h);
    h.preview_all(false);
    until_not_busy(&mut h);
    let cut = h.previewed().expect("a preview");
    assert!(cut.heights.iter().any(|&z| z < 0.0), "the profile is cut");
    assert!(h.status_message().starts_with("Previewed"));
}

#[test]
fn previewing_several_toolpaths_off_the_windows_thread_says_how_many() {
    let mut h = Harness::new();
    h.given_progress_shown_at_once();
    h.given_animated_preview(false);
    h.rectangle(10.0, 10.0, 50.0, 50.0);
    for id in ["profile_toolpath", "pocket_toolpath"] {
        h.invoke(id);
        let params = h.form_settings().clone();
        h.apply_form(id, &params).unwrap();
        until_not_busy(&mut h);
        h.close_form();
    }
    h.preview_all(false);
    until_not_busy(&mut h);
    assert_eq!(h.status_message(), "Previewed 2 toolpaths");
}

#[test]
fn a_double_sided_preview_cuts_the_other_side_off_the_windows_thread_too() {
    let mut h = Harness::new();
    h.given_document(|d| {
        d.job.double_sided = Some(cut_craft_core::document::DoubleSided::default());
    });
    let pocket = |h: &mut Harness| {
        h.rectangle(10.0, 10.0, 40.0, 40.0);
        h.invoke("pocket_toolpath");
        let params = h.form_settings().clone();
        h.run("pocket_toolpath", &params).unwrap();
        h.close_form();
    };
    pocket(&mut h);
    h.run("swap_sides", &serde_json::json!({})).unwrap();
    pocket(&mut h);
    h.run("swap_sides", &serde_json::json!({})).unwrap();
    h.given_progress_shown_at_once();
    h.invoke("preview_all_toolpaths");
    until_not_busy(&mut h);
    let preview = h.previewed().expect("previewed");
    assert!(
        preview.other_side.is_some(),
        "the other side is previewed too"
    );
}

#[test]
fn a_component_from_the_toolpath_preview_goes_on_the_top_shown_level_and_says_so() {
    use cut_craft_core::component::Level;
    let mut h = Harness::new();
    // the first level hidden, a second one shown above it
    h.given_document(|d| {
        d.levels.get_mut(&0).unwrap().visible = false;
        d.levels.insert(
            1,
            Level {
                name: "Top".into(),
                ..Level::default()
            },
        );
    });
    let job = h.doc().job;
    h.given_previewed(Some(cut_craft_core::simulation::Material::new(&job, 50)));
    h.component_from_preview();
    assert_eq!(h.doc().components.values().last().unwrap().level, 1);
    assert!(
        h.status_message().contains("on level Top"),
        "{}",
        h.status_message()
    );
}

#[test]
fn a_new_job_drops_the_old_jobs_preview_playback_and_its_undo() {
    // playing, then a new job
    let mut h = playing_preview();
    h.reset_job();
    assert!(h.playback().is_none() && !h.playback_kept());
    assert!(!h.playing() && h.playback_goal().is_none());
    assert!(!h.available("undo_last_preview"), "no old preview to undo");
    h.frame(vec![]);
    h.frame(vec![]);
    assert!(h.previewed().is_none(), "the old job's cuts not played in");
    assert!(h.text_at("Progress").is_none(), "no old controls");
    // stopped, then a new job
    let mut h = playing_preview();
    h.click(h.text_at("Stop").unwrap(), Modifiers::NONE);
    assert!(h.available("reset_preview"));
    h.reset_job();
    h.frame(vec![]);
    assert!(!h.playback_kept() && h.text_at("Progress").is_none());
}

#[test]
fn reset_preview_is_there_while_a_stopped_previews_controls_are() {
    let mut h = playing_preview();
    h.click(h.text_at("Stop").unwrap(), Modifiers::NONE);
    h.given_previewed(None);
    assert!(h.playback_kept());
    assert!(h.available("reset_preview"), "its controls can be cleared");
}

#[test]
fn a_change_to_the_job_closes_a_stopped_previews_controls() {
    let mut h = playing_preview();
    h.click(h.text_at("Stop").unwrap(), Modifiers::NONE);
    assert!(h.text_at("Progress").is_some(), "kept to scrub");
    let cut = h.previewed().cloned();
    h.rectangle(60.0, 60.0, 10.0, 10.0);
    h.frame(vec![]);
    assert!(!h.playback_kept(), "its old moves not played again");
    assert!(h.text_at("Progress").is_none(), "no old controls");
    assert_eq!(
        h.previewed().map(|m| &m.heights),
        cut.as_ref().map(|m| &m.heights),
        "what was cut stays shown"
    );
}

#[test]
fn a_change_to_the_job_stops_a_playing_preview_keeping_what_is_cut() {
    let mut h = playing_preview();
    h.rectangle(60.0, 60.0, 10.0, 10.0);
    h.frame(vec![]);
    assert!(h.playback().is_none() && !h.playback_kept());
    assert!(h.previewed().is_some(), "kept as far as it went");
    assert!(h.text_at("Progress").is_none());
}

#[test]
fn a_job_setup_change_starts_the_preview_again_on_the_new_block() {
    let mut h = playing_preview();
    h.click(h.text_at("Stop").unwrap(), Modifiers::NONE);
    assert!(h.previewed().is_some());
    h.given_document(|d| d.job.thickness += 7.0);
    h.frame(vec![]);
    assert!(h.previewed().is_none(), "the old block is not kept");
    assert!(!h.available("undo_last_preview"), "nor brought back");
    assert!(
        h.status_message().contains("Job Setup"),
        "{}",
        h.status_message()
    );
    // straight after a change, before a frame: Preview cuts the new block
    let mut h = playing_preview();
    h.click(h.text_at("Stop").unwrap(), Modifiers::NONE);
    h.given_document(|d| d.job.width += 20.0);
    let id = *h.doc().toolpaths.keys().next().unwrap();
    h.preview_toolpath(id);
    let fresh = h.fresh_material();
    let start = h.playback().expect("it plays").material();
    assert_eq!((start.columns, start.origin), (fresh.columns, fresh.origin));
}

#[test]
fn undo_last_preview_after_swap_sides_brings_the_preview_back_turned_over_with_the_job() {
    let mut h = Harness::new();
    h.given_animated_preview(false);
    h.given_document(|d| {
        d.job.double_sided = Some(cut_craft_core::document::DoubleSided::default());
    });
    h.rectangle(10.0, 10.0, 40.0, 40.0);
    h.invoke("pocket_toolpath");
    let params = h.form_settings().clone();
    h.run("pocket_toolpath", &params).unwrap();
    h.close_form();
    h.invoke("preview_all_toolpaths");
    let up = h.previewed().cloned().expect("previewed").heights;
    assert!(up.iter().any(|&z| z < 0.0), "the pocket is cut");
    h.invoke("reset_preview");
    h.run("swap_sides", &serde_json::json!({})).unwrap();
    h.frame(vec![]);
    assert!(h.doc().bottom_side);
    h.invoke("undo_last_preview");
    h.frame(vec![]);
    let back = h.previewed().expect("brought back");
    assert!(
        back.heights.iter().all(|&z| z == 0.0),
        "the job is on its other side, so that side is up, uncut"
    );
    assert_eq!(
        back.other_side.as_deref().map(|m| &m.heights),
        Some(&up),
        "the side the preview was made on is underneath"
    );
    // swapped back, the preview is up again
    h.invoke("undo");
    h.frame(vec![]);
    assert!(!h.doc().bottom_side);
    assert_eq!(h.previewed().map(|m| &m.heights), Some(&up));
}

#[test]
fn a_small_part_on_a_big_sheet_is_previewed_finely_where_it_is_cut() {
    let mut h = Harness::new();
    h.given_animated_preview(false);
    h.rectangle(10.0, 10.0, 50.0, 50.0);
    h.invoke("profile_toolpath");
    let params = h.form_settings().clone();
    h.run("profile_toolpath", &params).unwrap();
    h.close_form();
    h.invoke("preview_all_toolpaths");
    h.frame(vec![]);
    let job = h.doc().job;
    let m = h.previewed().cloned().expect("a preview");
    let area = m.area();
    assert!(
        area.max.x - area.min.x < job.width / 2.0 && area.max.y - area.min.y < job.height / 2.0,
        "round the part, not the whole sheet: {area:?}"
    );
    assert!(
        (m.cell - crate::preview_detail::finest(50)).abs() < 1e-9,
        "{}",
        m.cell
    );
    assert!(m.lies_on(&job));
    // the 3D view still shows the whole sheet round it
    let shown = h.fine_surface();
    assert!((shown.origin - job.area().min).length() < 1e-9);
    let across = shown.area().max.x - shown.area().min.x;
    assert!((across - job.width).abs() <= shown.cell);
    // Toolpaths ▸ Preview Simulation Quality ▸ Extremely High: the slider to 80%, the
    // preview worked out again finer, its tick on that pick
    h.invoke("preview_quality_8");
    let finer = h.previewed().cloned().expect("worked out again");
    assert_eq!(h.display().detail, 80);
    assert!(finer.cell < m.cell, "{} then {}", m.cell, finer.cell);
    assert_eq!(h.display().checked("preview_quality_8"), Some(true));
    assert_eq!(h.display().checked("preview_quality_1"), Some(false));
    // Reset Preview: the slider back to Options' default
    h.invoke("reset_preview");
    assert_eq!(h.display().detail, h.options().performance.preview_detail);
}

#[test]
fn the_preview_tab_has_the_detail_slider_always_to_hand() {
    let mut h = named_toolpaths();
    // the Toolpaths page's Preview tab
    let tab = h
        .text_at("Preview")
        .unwrap_or_else(|| panic!("{:?}", h.texts()));
    h.click(tab, Modifiers::NONE);
    h.frame(vec![]);
    for word in ["Faster", "More detail", "50%"] {
        assert!(
            h.text_at(word).is_some(),
            "{word} beside the slider: {:?}",
            h.texts()
        );
    }
}
