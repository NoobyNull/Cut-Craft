//! Saving toolpaths: Save Toolpaths, View G-code, tiling, job sheets, job times and labels.

use super::*;

#[test]
fn a_preview_plays_as_long_as_the_toolpath_summary_says_the_machine_takes() {
    let h = playing_preview();
    let toolpath = h.app.doc.toolpaths.values().next().unwrap();
    let summary = toolpath.machining_minutes(h.app.summary.rapid) * 60.0;
    let length = h.app.playback.as_ref().unwrap().length();
    assert!(
        (length - summary).abs() < 1e-6,
        "{length} s, the summary {summary} s"
    );
}

#[test]
fn a_jump_gets_more_time_each_frame_than_playing_does() {
    let h = playing_preview();
    assert!(super::jump_budget(h.app.playback_budget) > h.app.playback_budget);
    assert_eq!(
        super::jump_budget(std::time::Duration::ZERO),
        std::time::Duration::ZERO
    );
}

#[test]
fn options_show_one_category_at_a_time() {
    let mut h = Harness::new();
    h.app.invoke(&h.ctx, "options");
    // a new window measures itself on its first frame
    h.frame(vec![]);
    h.frame(vec![]);
    assert!(
        h.text_at("Draw origin").is_some(),
        "the first category's settings"
    );
    assert!(h.text_at("Recent file list size").is_none(), "only its own");
    let at = *h.texts_at("Appearance").last().expect("listed");
    h.click(at, Modifiers::NONE);
    h.frame(vec![]);
    assert!(h.text_at("Theme").is_some());
    assert!(h.text_at("Draw origin").is_none());
    assert!(
        h.text_at("Advanced").is_none(),
        "Advanced lists only once asked for"
    );
    assert!(
        h.text_at("Bug reports go to").is_none(),
        "reports always go to Cut Craft's repository"
    );
}

#[test]
fn a_crash_last_time_opens_its_report_with_the_crash_log() {
    let mut h = Harness::new();
    h.app.options.bug_reports.crash_logs = crate::options::CrashLogs::Send;
    // no crash-report service yet: the report opens with the log ticked
    h.app.reporter.crash_service = None;
    let ctx = h.ctx.clone();
    h.app.offer_crash(&ctx, CRASH.into());
    h.frame(vec![]);
    h.frame(vec![]);
    assert!(h.app.reporter.is_open(), "Report a Bug opens by itself");
    assert!(
        h.text_at("Crash: index out of bounds").is_some(),
        "titled in words, not with the source path"
    );
    assert!(h.text_at("The crash log").is_some());
    assert!(
        h.text_at("May Cut Craft send these with your report?")
            .is_some()
    );
}

#[test]
fn save_toolpaths_opens_its_form_and_saves_each_shown_toolpath_to_a_file_of_its_own() {
    use crate::files::SaveMode;
    let mut h = Harness::new();
    for (x, name) in [(10.0, "First"), (80.0, "Second")] {
        h.rectangle(x, 10.0, 50.0, 50.0);
        h.app.invoke(&h.ctx, "profile_toolpath");
        h.frame(vec![]);
        let mut params = h.app.form.as_ref().unwrap().params.clone();
        params["name"] = name.into();
        h.app.apply("profile_toolpath", &params).unwrap();
        h.app.form = None;
    }
    h.app.invoke(&h.ctx, "save_toolpaths");
    h.frame(vec![]);
    h.frame(vec![]);
    assert!(h.app.save_form.is_some());
    assert!(h.text_at("Visible toolpaths to multiple files").is_some());
    assert!(h.text_at("Save Toolpaths...").is_some());
    let dir = std::env::temp_dir().join(format!("cut-craft-save-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let form = SaveForm {
        mode: SaveMode::VisibleToMultipleFiles,
        group: false,
        post: None,
        add_side: false,
        changes: crate::files::ToolChanges::ToolChanger,
        job_sheet: true,
    };
    let post = h.app.output_post().unwrap();
    let done = h
        .app
        .save_toolpaths_to(&form, &post, &dir.join("job.nc"))
        .unwrap();
    assert!(done.starts_with("Saved 2 toolpath files"), "{done}");
    for name in ["job_1_First.nc", "job_2_Second.nc"] {
        let gcode = std::fs::read_to_string(dir.join(name)).unwrap();
        assert!(gcode.contains(&name[6..name.len() - 3]), "{gcode}");
    }
    // hidden toolpaths stay out of a one-file save
    let first = *h.app.doc.toolpaths.keys().next().unwrap();
    h.app.doc.toolpaths.get_mut(&first).unwrap().visible = false;
    let one = SaveForm {
        mode: SaveMode::VisibleToOneFile,
        ..form
    };
    h.app
        .save_toolpaths_to(&one, &post, &dir.join("one.nc"))
        .unwrap();
    let gcode = std::fs::read_to_string(dir.join("one.nc")).unwrap();
    assert!(
        gcode.contains("Second") && !gcode.contains("First"),
        "{gcode}"
    );
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn a_toolpath_with_a_vector_selector_takes_the_vectors_on_its_layers_each_time() {
    let mut h = Harness::new();
    let mut c = Change::builder(&h.app.doc);
    let cut = c.add_layer("Cut");
    c.set_current_layer(cut);
    h.app.doc.apply(&c.finish()).unwrap();
    h.rectangle(10.0, 10.0, 50.0, 50.0);
    h.app.invoke(&h.ctx, "profile_toolpath");
    // the section, drawn on its own (it is below the fold of the profile form): ticking its
    // box associates a selector
    let ctx = egui::Context::default();
    let mut section = serde_json::json!({});
    let out = headless(ctx.run_ui(RawInput::default(), |ui| {
        egui::CentralPanel::default().show(ui, |ui| {
            selector_section(ui, &mut section, &["Cut".to_owned()], Units::Mm);
        });
    }));
    assert!(out.shapes.iter().any(|c| matches!(&c.shape,
        egui::Shape::Text(t) if t.galley.text() == "Associate a Vector Selector with this toolpath")));
    let mut params = h.app.form.as_ref().unwrap().params.clone();
    params["selector"] = serde_json::to_value(cut_craft_core::selector::Selector {
        layers: cut_craft_core::selector::Layers::SelectedLayers,
        layer_names: vec!["Cut".into()],
        ..Default::default()
    })
    .unwrap();
    // nothing selected: the selector finds the rectangle on Cut
    h.app.doc.selection = cut_craft_core::selection::Selection::default();
    h.app.apply("profile_toolpath", &params).unwrap();
    let id = *h.app.doc.toolpaths.keys().next().unwrap();
    assert_eq!(h.app.doc.toolpaths[&id].vectors.len(), 1);
    // a new rectangle on Cut joins it when the toolpath is recalculated
    h.app.form = None;
    h.rectangle(80.0, 10.0, 30.0, 30.0);
    assert!(
        h.app.doc.toolpaths[&id].stale,
        "out of date once a new vector passes its selector"
    );
    h.app.invoke(&h.ctx, "recalculate_all_toolpaths");
    assert_eq!(
        h.app.doc.toolpaths[&id].vectors.len(),
        2,
        "{}",
        h.app.status.message
    );
    // and keeps its selector for the next time
    h.rectangle(130.0, 10.0, 30.0, 30.0);
    h.app.invoke(&h.ctx, "recalculate_all_toolpaths");
    assert_eq!(h.app.doc.toolpaths[&id].vectors.len(), 3);
}

#[test]
fn the_toolpath_summary_keeps_notes_per_toolpath_and_shows_the_rapid_rate_in_its_units() {
    let mut h = Harness::new();
    h.rectangle(10.0, 10.0, 50.0, 50.0);
    h.app.invoke(&h.ctx, "profile_toolpath");
    let params = h.app.form.as_ref().unwrap().params.clone();
    h.app.apply("profile_toolpath", &params).unwrap();
    h.app.form = None;
    let id = *h.app.doc.toolpaths.keys().next().unwrap();
    h.app.invoke(&h.ctx, "toolpath_summary");
    h.frame(vec![]);
    // the rapid rate is kept in mm/min, shown in the units picked
    h.app.summary.units = cut_craft_core::toolpath::RateUnits::InchPerMin;
    h.frame(vec![]);
    assert!(
        (h.app.summary.rapid - 5000.0).abs() < 1e-9,
        "the rate itself is unchanged"
    );
    // a toolpath's notes, written under the summary and applied
    let name = h.app.doc.toolpaths[&id].name.clone();
    // the summary's row (the toolpath list has the name too, further down)
    let row = *h.texts_at(&name).first().expect("the toolpath is listed");
    h.click(row, Modifiers::NONE);
    assert_eq!(h.app.summary.picked, Some(id));
    h.app.summary.notes = "Use the 1/4 in downcut".into();
    h.frame(vec![]);
    let apply = *h
        .texts_at("Save Notes")
        .last()
        .expect("the notes have Save Notes");
    h.click(apply, Modifiers::NONE);
    assert_eq!(
        h.app.doc.toolpaths[&id].summary_notes(),
        "Use the 1/4 in downcut"
    );
    // recalculating keeps them
    let change = h.app.registry.edit(&h.app.doc, id, None).unwrap();
    h.app.doc.apply(&change).unwrap();
    assert_eq!(
        h.app.doc.toolpaths[&id].summary_notes(),
        "Use the 1/4 in downcut"
    );
}

#[test]
fn a_double_clicked_tile_is_previewed_alone() {
    let mut h = Harness::new();
    h.app.display.animate_preview = false;
    h.app.options.snapping.smart_snapping = false;
    h.app.options.snapping.geometry_snapping = false;
    let area = h.app.doc.job.area();
    // a long thin rectangle right across the job
    let y = area.center().y;
    h.rectangle(area.min.x + 10.0, y - 5.0, area.size().x - 20.0, 10.0);
    h.app.invoke(&h.ctx, "profile_toolpath");
    let mut params = h.app.form.as_ref().unwrap().params.clone();
    params["cut_depth"] = serde_json::json!(2.0);
    h.app.apply("profile_toolpath", &params).unwrap();
    // two tiles side by side, then the right-hand one double-clicked
    h.app.invoke(&h.ctx, "toolpath_tiling");
    let form = h.app.form.as_mut().unwrap();
    form.params["enabled"] = serde_json::json!(true);
    form.params["tiling"]["width"] = serde_json::json!(area.size().x / 2.0);
    form.params["tiling"]["height"] = serde_json::json!(area.size().y);
    form.params["tiling"]["overlap"] = serde_json::json!(0.0);
    let params = form.params.clone();
    h.app.apply("toolpath_tiling", &params).unwrap();
    // the Toolpaths page docked beside the view, not flown out over it
    h.app.pages.set_pinned(Page::Toolpaths, true);
    h.frame(vec![]);
    h.frame(vec![]);
    let right = h.at(
        area.max.x - area.size().x / 4.0,
        area.min.y + area.size().y / 4.0,
    );
    h.double_click(right);
    assert_eq!(h.app.doc.tiling.map(|t| t.active), Some(2));
    h.app.invoke(&h.ctx, "preview_all_toolpaths");
    let m = h.app.simulation.as_ref().expect("previewed");
    let cut_beyond = |from_x: f64| {
        (0..m.rows).any(|r| {
            (0..m.columns).any(|c| m.centre(c, r).x > from_x && m.heights[r * m.columns + c] < -0.5)
        })
    };
    // the right-hand tile's cut, moved to the machine's corner: nothing on the right half
    assert!(
        !cut_beyond(area.center().x + 5.0),
        "the tile is cut at the corner"
    );
    assert!(cut_beyond(area.min.x), "it is cut");
}

#[test]
fn print_part_labels_shows_how_many_parts_get_a_label() {
    let mut h = Harness::new();
    h.rectangle(10.0, 10.0, 50.0, 50.0);
    h.app.invoke(&h.ctx, "part_labels");
    h.frame(vec![]);
    h.frame(vec![]);
    assert!(h.app.part_labels_window);
    assert!(
        h.text_at("1 part will get a label. Name parts with Edit ▸ Name Part.")
            .is_some()
    );
    assert!(h.text_at("Open Labels to Print").is_some());
    assert!(h.text_at("Label size").is_some());
}

#[test]
fn job_sheets_take_the_editor_s_style_and_name_their_side() {
    let mut h = Harness::new();
    h.app.invoke(&h.ctx, "job_sheet_editor");
    h.frame(vec![]);
    h.frame(vec![]);
    assert!(h.app.job_sheet_window);
    assert!(
        h.text_at("Job Setup Sheet Editor").is_some(),
        "the editor is open"
    );
    h.app.options.job_sheet.title = "Shop Sheet".into();
    h.app.doc.job.double_sided = Some(cut_craft_core::document::DoubleSided::default());
    let dir = std::env::temp_dir().join(format!("cut-craft-sheets-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    h.app.write_job_sheets(&dir.join("sign.html"));
    let page = std::fs::read_to_string(dir.join("sign_Top.html")).expect("the top side's sheet");
    assert!(page.contains("<h1>Shop Sheet</h1>"));
    assert_eq!(h.app.status.message, "Saved 1 job sheet");
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn quick_engraves_immediate_output_saves_the_toolpath_calculated_last() {
    use crate::files::SaveMode;
    let mut h = Harness::new();
    h.rectangle(10.0, 10.0, 50.0, 50.0);
    assert_eq!(h.app.immediate_output_toolpath(), None);
    h.app.invoke(&h.ctx, "quick_engrave_toolpath");
    let params = h.app.form.as_ref().unwrap().params.clone();
    h.app.apply("quick_engrave_toolpath", &params).unwrap();
    let id = h.app.immediate_output_toolpath().unwrap();
    assert_eq!(h.app.doc.toolpaths[&id].strategy, "quick_engrave_toolpath");
    // what Save Toolpath Now does, short of its file dialog
    let dir = std::env::temp_dir().join(format!("cut-craft-engrave-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    h.app.selected_toolpath = Some(id);
    let form = SaveForm {
        mode: SaveMode::Selected,
        group: false,
        post: None,
        add_side: false,
        changes: crate::files::ToolChanges::ToolChanger,
        job_sheet: true,
    };
    let post = h.app.output_post().unwrap();
    let done = h
        .app
        .save_toolpaths_to(&form, &post, &dir.join("engrave.nc"))
        .unwrap();
    assert!(done.starts_with("Saved"), "{done}");
    let gcode = std::fs::read_to_string(dir.join("engrave.nc")).unwrap();
    assert!(gcode.contains("Quick Engrave"), "{gcode}");
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn the_summary_allows_for_the_active_machines_acceleration_when_asked() {
    let mut h = Harness::new();
    h.rectangle(10.0, 10.0, 50.0, 50.0);
    h.app.invoke(&h.ctx, "profile_toolpath");
    let params = h.app.form.as_ref().unwrap().params.clone();
    h.app.apply("profile_toolpath", &params).unwrap();
    h.app.form = None;
    let t = h.app.doc.toolpaths.values().next().unwrap().clone();
    let plain = h.app.estimated_minutes(&t);
    h.app.summary.acceleration = true;
    assert!(
        (h.app.estimated_minutes(&t) - plain).abs() < 1e-12,
        "no machine: no allowance"
    );
    h.app.machines.machines.push(cut_craft_machines::Machine {
        name: "Slow".into(),
        acceleration: [20.0, 20.0, 20.0],
        ..cut_craft_machines::Machine::default()
    });
    h.app.machines.active = h.app.machines.machines.len() - 1;
    let allowing = h.app.estimated_minutes(&t);
    assert!(allowing > plain, "{allowing} vs {plain}");
    h.app.summary.scale = 2.0;
    assert!((h.app.estimated_minutes(&t) - 2.0 * allowing).abs() < 1e-9);
}

#[test]
fn the_job_time_counts_every_move_tool_changes_and_the_trip_home() {
    let mut h = Harness::new();
    assert!(
        h.app.summary.acceleration,
        "accelerations count unless told not to"
    );
    h.rectangle(10.0, 10.0, 50.0, 50.0);
    h.app.invoke(&h.ctx, "profile_toolpath");
    let params = h.app.form.as_ref().unwrap().params.clone();
    h.app.apply("profile_toolpath", &params).unwrap();
    h.app.form = None;
    let one: f64 = h
        .app
        .doc
        .toolpaths
        .values()
        .map(|t| h.app.estimated_minutes(t))
        .sum();
    let alone = cut_craft_core::cut_time::toolpath(
        h.app.doc.toolpaths.values().next().unwrap(),
        &h.app.motion(),
    )
    .total()
        / 60.0;
    assert!(
        one > alone,
        "getting there from home counts: {one} against {alone}"
    );
    assert!(h.app.job_minutes() > one, "so does the trip back home");
    // the same toolpath with another bit: a tool change by hand
    let mut other = h.app.doc.toolpaths.values().next().unwrap().clone();
    other.tool.name = "Another bit".into();
    let id = h.app.doc.next_id();
    h.app.doc.toolpaths.insert(id, other);
    let changed = h.app.estimated_minutes(&h.app.doc.toolpaths[&id]);
    assert!(
        changed > 2.0,
        "the 2 minute change by hand counts: {changed}"
    );
}

#[test]
fn the_window_items_tile_the_2d_and_3d_views_and_cascade_puts_them_back_in_tabs() {
    let mut h = Harness::new();
    for (id, split) in [
        ("tile_windows_horizontal", Some(Split::Stacked)),
        ("tile_windows_vertical", Some(Split::SideBySide)),
        ("cascade_windows", None),
    ] {
        assert!(h.app.available(id), "{id}");
        h.app.invoke(&h.ctx, id);
        assert_eq!(h.app.split, split, "{id}");
    }
}

#[test]
fn save_toolpaths_refuses_toolpaths_bigger_than_the_machine_unless_its_check_is_off() {
    let mut h = Harness::new();
    h.rectangle(10.0, 10.0, 150.0, 150.0);
    h.app.invoke(&h.ctx, "profile_toolpath");
    h.frame(vec![]);
    let params = h.app.form.as_ref().unwrap().params.clone();
    h.app.apply("profile_toolpath", &params).unwrap();
    h.app.form = None;
    h.app.machines.machines.push(cut_craft_machines::Machine {
        name: "Desktop".into(),
        width: 100.0,
        height: 100.0,
        ..cut_craft_machines::Machine::default()
    });
    h.app.machines.active = h.app.machines.machines.len() - 1;
    let form = SaveForm {
        mode: crate::files::SaveMode::VisibleToOneFile,
        group: false,
        post: None,
        add_side: false,
        changes: crate::files::ToolChanges::ToolChanger,
        job_sheet: true,
    };
    let post = h.app.output_post().unwrap();
    let path = std::env::temp_dir().join(format!("cut-craft-too-big-{}.nc", std::process::id()));
    let refused = h.app.save_toolpaths_to(&form, &post, &path).unwrap_err();
    assert!(
        refused.starts_with("Nothing saved: this could damage the machine.")
            && refused.contains("more than the bed of Desktop"),
        "{refused}"
    );
    assert!(!path.exists(), "nothing written");
    // only switching the check off for the machine lets it past, and the file says so
    h.app.machines.active_mut().unwrap().protect.bed = false;
    let found = h.app.preflight(
        &h.app.plan(&form, &post).unwrap(),
        crate::files::ToolChanges::ToolChanger,
    );
    assert!(
        !found
            .iter()
            .any(|f| f.check == cut_craft_core::check_list::Check::Bed),
        "{found:?}"
    );
    h.app.saving_checked = Some((h.app.history.revision(), h.app.machines.active));
    let written = h.app.write_toolpath_files(&form, &post, &path).unwrap();
    let text = std::fs::read_to_string(&written[0]).unwrap();
    assert!(
        text.contains("PROTECTION OFF: the checks for bed size were switched off for Desktop"),
        "{text}"
    );
    for file in written {
        let _ = std::fs::remove_file(file);
    }
}

#[test]
fn send_to_machine_puts_the_files_and_job_sheet_in_the_projects_own_folder() {
    use cut_craft_machines::send::{Folder, Store};
    let mut h = Harness::new();
    two_tools(&mut h);
    let card = std::env::temp_dir().join(format!("cut-craft-card-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&card);
    std::fs::create_dir_all(&card).unwrap();
    h.app.machines.add();
    let machine = h.app.machines.active_mut().unwrap();
    machine.name = "Shop router".into();
    machine.send_to.folder = card.display().to_string();
    h.app.session.path = Some(std::path::PathBuf::from("/jobs/Oak sign: front.ccjob"));
    let form = SaveForm {
        mode: crate::files::SaveMode::VisibleToOneFile,
        group: false,
        post: None,
        add_side: false,
        changes: crate::files::ToolChanges::ToolChanger,
        job_sheet: true,
    };
    let post = h.app.form_post(&form).unwrap();
    let store = Folder::new(&card);
    let never = |_: &[String]| panic!("nothing there to replace yet");
    let done = h.app.send_to(&form, &post, &store, &never).unwrap();
    assert_eq!(
        done,
        "Sent 2 files to Shop router, in its folder Oak sign_ front"
    );
    let folder = ["Oak sign_ front".to_owned()];
    let names: Vec<String> = store
        .list(&folder)
        .unwrap()
        .into_iter()
        .map(|i| i.name)
        .collect();
    assert_eq!(
        names,
        ["Oak sign_ front job sheet.html", "Oak sign_ front.nc"]
    );
    let sheet = std::fs::read_to_string(card.join(&folder[0]).join(&names[0])).unwrap();
    assert!(
        sheet.contains("Oak sign_ front.nc"),
        "the sheet lists the file sent"
    );
    // sending again asks before replacing what is there, and keeps it on No
    let no = |there: &[String]| {
        assert_eq!(there.len(), 2);
        None
    };
    let kept = h.app.send_to(&form, &post, &store, &no).unwrap_err();
    assert!(kept.contains("were kept"), "{kept}");
    // the project keeps its folder; another with the same name gets one of its own
    assert_eq!(
        h.app.machines.active().unwrap().send_to.projects["/jobs/Oak sign: front.ccjob"],
        "Oak sign_ front"
    );
    h.app.session.path = Some(std::path::PathBuf::from("/old/Oak sign_ front.ccjob"));
    let yes = |_: &[String]| Some(crate::options::EarlierGcode::Replace);
    let done = h.app.send_to(&form, &post, &store, &yes).unwrap();
    assert!(done.ends_with("in its folder Oak sign_ front 2"), "{done}");
    let _ = std::fs::remove_dir_all(card);
}

#[test]
fn sending_again_replaces_or_archives_tile_files_an_earlier_layout_left_on_the_machine() {
    use crate::options::EarlierGcode;
    use cut_craft_machines::send::{Folder, Store};
    let mut h = Harness::new();
    two_tools(&mut h);
    let card = std::env::temp_dir().join(format!("cut-craft-tiles-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&card);
    std::fs::create_dir_all(card.join("sign")).unwrap();
    h.app.machines.add();
    h.app.machines.active_mut().unwrap().send_to.folder = card.display().to_string();
    h.app.session.path = Some(std::path::PathBuf::from("/jobs/sign.ccjob"));
    let form = SaveForm {
        mode: crate::files::SaveMode::VisibleToOneFile,
        group: false,
        post: None,
        add_side: false,
        changes: crate::files::ToolChanges::ToolChanger,
        job_sheet: false,
    };
    let post = h.app.form_post(&form).unwrap();
    let ext = post.extension.clone();
    // an earlier send of three tiles; this one is not tiled, so all three are left over
    for n in 1..=3 {
        std::fs::write(card.join("sign").join(format!("sign_tile{n}.{ext}")), "old").unwrap();
    }
    std::fs::write(card.join("sign").join("notes.txt"), "mine").unwrap();
    let store = Folder::new(&card);
    let names = |store: &Folder, at: &[String]| -> Vec<String> {
        store
            .list(at)
            .unwrap()
            .into_iter()
            .map(|i| i.name)
            .collect()
    };
    let asked = std::cell::RefCell::new(vec![]);
    let keep = |there: &[String]| {
        asked.borrow_mut().extend_from_slice(there);
        None
    };
    let kept = h.app.send_to(&form, &post, &store, &keep).unwrap_err();
    assert!(kept.contains("were kept"), "{kept}");
    assert_eq!(
        *asked.borrow(),
        (1..=3)
            .map(|n| format!("sign_tile{n}.{ext}"))
            .collect::<Vec<_>>(),
        "the leftover tiles are asked about, not the user's own file"
    );
    // Archive: moved into a dated folder under Archive, then the new file sent
    let archive = |_: &[String]| Some(EarlierGcode::Archive);
    let done = h.app.send_to(&form, &post, &store, &archive).unwrap();
    assert!(
        done.contains("the earlier files are in sign/Archive/sign "),
        "{done}"
    );
    let at = ["sign".to_owned()];
    assert_eq!(
        names(&store, &at),
        [
            "Archive".to_owned(),
            "notes.txt".into(),
            format!("sign.{ext}")
        ]
    );
    let dated = names(&store, &["sign".into(), "Archive".into()]);
    assert_eq!(dated.len(), 1);
    assert_eq!(
        names(&store, &["sign".into(), "Archive".into(), dated[0].clone()]).len(),
        3
    );
    // Replace: an old tile left beside the new file is deleted
    std::fs::write(card.join("sign").join(format!("sign_tile2.{ext}")), "old").unwrap();
    let replace = |_: &[String]| Some(EarlierGcode::Replace);
    h.app.send_to(&form, &post, &store, &replace).unwrap();
    assert_eq!(
        names(&store, &at),
        [
            "Archive".to_owned(),
            "notes.txt".into(),
            format!("sign.{ext}")
        ]
    );
    let _ = std::fs::remove_dir_all(card);
}

#[test]
fn save_toolpaths_offers_send_to_machine_once_the_machine_has_a_files_folder() {
    let mut h = Harness::new();
    two_tools(&mut h);
    h.app.machines.add();
    h.app.invoke(&h.ctx, "save_toolpaths");
    h.frame(vec![]);
    h.frame(vec![]);
    assert!(h.text_at("Send to Machine").is_some(), "shown, greyed");
    assert!(h.text_at("Machine Files…").is_none());
    assert!(h.text_at("Save a job sheet with the files").is_some());
    h.app.machines.active_mut().unwrap().send_to.folder = "/media/card".into();
    h.frame(vec![]);
    h.frame(vec![]);
    assert!(h.text_at("Machine Files…").is_some());
}

#[test]
fn save_toolpaths_shows_what_could_damage_the_machine_and_will_not_save_while_it_is() {
    let mut h = Harness::new();
    two_tools(&mut h);
    h.app.machines.add();
    let machine = h.app.machines.active_mut().unwrap();
    machine.name = "Desktop".into();
    (machine.width, machine.height) = (60.0, 60.0);
    h.app.invoke(&h.ctx, "save_toolpaths");
    h.frame(vec![]);
    h.frame(vec![]);
    let shown = h
        .shapes
        .iter()
        .filter_map(|c| match &c.shape {
            egui::Shape::Text(t) => Some(t.galley.text().to_string()),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join("\n");
    assert!(shown.contains("Checks before cutting"), "{shown}");
    assert!(
        shown.contains("Could damage the machine: The toolpaths cover"),
        "{shown}"
    );
    assert!(
        shown.contains("Not saved or sent while this could damage"),
        "{shown}"
    );
    let checked = h.app.save_checks.clone();
    assert_eq!(checked.len(), 1);
    h.frame(vec![]);
    assert_eq!(
        h.app.save_checks.len(),
        1,
        "the checks are kept between frames, not run again"
    );
    assert!(h.app.save_checks[0].same(&checked[0]));
}

#[test]
fn first_time_setup_picks_the_machine_units_and_material_for_new_jobs() {
    use super::first_run::Maker;
    let mut h = Harness::at_start();
    assert!(h.app.first_run.is_none(), "tests start without it");
    h.choose(
        |app| {
            app.current_menus()
                .menus
                .iter()
                .find(|m| m.name == "Help")
                .unwrap()
                .items
                .clone()
        },
        &["Run First-Time Setup Again..."],
    );
    // a window sizes itself unseen on its first frame
    h.frame(vec![]);
    h.frame(vec![]);
    assert!(h.text_at("Welcome to Cut Craft").is_some());
    let setup = h.app.first_run.as_mut().unwrap();
    setup.pick_maker(Maker::Named("Sienci".into()));
    let longmill = setup
        .catalogue
        .iter()
        .position(|m| m.name == "Sienci LongMill MK2 30x30")
        .unwrap();
    setup.pick_model(longmill);
    h.frame(vec![]);
    h.frame(vec![]);
    let start = h.text_at("Start using Cut Craft").unwrap();
    h.click(start, Modifiers::NONE);
    h.frame(vec![]);
    assert!(h.app.first_run.is_none(), "closed");
    assert!(h.app.options.first_run_done);
    let machine = h.app.machines.active().expect("the machine set up");
    assert_eq!(machine.name, "Sienci LongMill MK2 30x30");
    assert_eq!(machine.post(), Some("Cut Craft GRBL Arcs (inch) (*.nc)"));
    assert_eq!(h.app.tools.groups[0].path, "Imperial Tools");
    // a new job starts as answered
    h.app.invoke(&h.ctx, "new");
    h.app.job_setup_ok();
    h.frame(vec![]);
    let job = h.app.doc.job;
    assert_eq!(job.units, Units::Inch);
    assert!(job.width <= 762.0 && job.height <= 762.0, "{job:?}");
    // asked again: the machine is not added twice
    h.app.open_first_run();
    let setup = h.app.first_run.as_mut().unwrap();
    setup.pick_maker(Maker::Named("Sienci".into()));
    setup.pick_model(longmill);
    let setup = h.app.first_run.take().unwrap();
    h.app.finish_first_run(&setup);
    assert_eq!(h.app.machines.machines.len(), 1);
}

#[test]
fn first_time_setup_again_starts_from_the_answers_and_corrects_my_machine() {
    use super::first_run::Maker;
    use crate::options::CrashLogs;
    use cut_craft_machines::Controller;
    let mut h = Harness::at_start();
    h.app.open_first_run();
    let setup = h.app.first_run.as_mut().unwrap();
    setup.pick_maker(Maker::NotListed);
    setup.bed = [500.0, 300.0];
    setup.controller = Controller::Mach3;
    setup.crash_logs = CrashLogs::Send;
    let setup = h.app.first_run.take().unwrap();
    h.app.finish_first_run(&setup);
    // asked again: it starts from what was answered, not from nothing
    h.app.open_first_run();
    let setup = h.app.first_run.as_mut().unwrap();
    assert_eq!(setup.maker, Maker::NotListed);
    assert_eq!(setup.bed, [500.0, 300.0]);
    assert_eq!(setup.controller, Controller::Mach3);
    assert_eq!(
        setup.crash_logs,
        CrashLogs::Send,
        "the crash-report answer kept"
    );
    // a corrected working area and controller change My Machine, not only pick it
    setup.bed = [800.0, 600.0];
    setup.controller = Controller::Grbl;
    let setup = h.app.first_run.take().unwrap();
    h.app.finish_first_run(&setup);
    assert_eq!(h.app.machines.machines.len(), 1, "not added twice");
    let machine = h.app.machines.active().unwrap();
    assert_eq!((machine.width, machine.height), (800.0, 600.0));
    assert_eq!(machine.post(), Some("Cut Craft GRBL Arcs (mm) (*.nc)"));
    assert_eq!(h.app.options.bug_reports.crash_logs, CrashLogs::Send);
}

#[test]
fn first_time_setup_again_keeps_the_posts_added_to_my_machine() {
    use super::first_run::Maker;
    use crate::options::CrashLogs;
    use cut_craft_machines::Controller;
    let mut h = Harness::at_start();
    h.app.open_first_run();
    let setup = h.app.first_run.as_mut().unwrap();
    setup.pick_maker(Maker::NotListed);
    setup.controller = Controller::Mach3;
    let setup = h.app.first_run.take().unwrap();
    h.app.finish_first_run(&setup);
    // a post added and made the default in Machine Configuration
    let extra = "Cut Craft GRBL Arcs (mm) (*.nc)".to_owned();
    let mine = h.app.machines.machines.first_mut().unwrap();
    mine.posts.push(extra.clone());
    mine.default_post = Some(extra.clone());
    // asked again only to change the crash-report answer
    h.app.open_first_run();
    let setup = h.app.first_run.as_mut().unwrap();
    setup.crash_logs = CrashLogs::Send;
    let setup = h.app.first_run.take().unwrap();
    h.app.finish_first_run(&setup);
    let machine = h.app.machines.active().unwrap();
    assert!(machine.posts.contains(&extra), "{:?}", machine.posts);
    assert_eq!(machine.post(), Some(extra.as_str()), "the default kept");
    // a new controller brings its posts and default, and still keeps the one added
    h.app.open_first_run();
    let setup = h.app.first_run.as_mut().unwrap();
    setup.controller = Controller::FluidNc;
    let setup = h.app.first_run.take().unwrap();
    h.app.finish_first_run(&setup);
    let machine = h.app.machines.active().unwrap();
    let (stock, default) = Controller::FluidNc.posts(false);
    assert_eq!(machine.default_post.as_deref(), Some(default.as_str()));
    assert!(stock.iter().all(|p| machine.posts.contains(p)));
    assert!(machine.posts.contains(&extra), "{:?}", machine.posts);
    let (mach3, _) = Controller::Mach3.posts(false);
    assert!(
        mach3
            .iter()
            .all(|p| !machine.posts.contains(p) || stock.contains(p)),
        "the old controller's own posts go: {:?}",
        machine.posts
    );
}

#[test]
fn first_time_setup_again_keeps_the_rest_of_the_default_job() {
    let mut h = Harness::at_start();
    h.app.open_first_run();
    let setup = h.app.first_run.take().unwrap();
    h.app.finish_first_run(&setup);
    // set in Job Setup's Set as Default, not asked about by first-time setup
    let mut kept = h.app.options.default_job.unwrap();
    kept.clearance = 12.5;
    kept.colour = [10, 20, 30];
    h.app.options.default_job = Some(kept);
    // asked again only to change the material's thickness
    h.app.open_first_run();
    let setup = h.app.first_run.as_mut().unwrap();
    setup.sheet[2] = 12.0;
    let setup = h.app.first_run.take().unwrap();
    h.app.finish_first_run(&setup);
    let job = h.app.options.default_job.unwrap();
    assert_eq!(job.thickness, 12.0);
    assert_eq!(job.clearance, 12.5, "the clearance kept");
    assert_eq!(job.colour, [10, 20, 30], "the colour kept");
}

#[test]
fn skipping_first_time_setup_keeps_todays_start() {
    let mut h = Harness::at_start();
    h.app.open_first_run();
    h.frame(vec![]);
    h.frame(vec![]);
    assert!(
        h.app.first_run.as_ref().is_some_and(|s| !s.ready()),
        "nothing picked yet"
    );
    let skip = h.text_at("Skip for now").unwrap();
    h.click(skip, Modifiers::NONE);
    h.frame(vec![]);
    assert!(h.app.first_run.is_none() && h.app.options.first_run_done);
    assert_eq!(
        h.app.machines.machines,
        [] as [cut_craft_machines::Machine; 0]
    );
    assert!(h.app.options.default_job.is_none());
    h.app.invoke(&h.ctx, "new");
    h.app.job_setup_ok();
    let (job, usual) = (h.app.doc.job, cut_craft_core::document::Job::default());
    assert_eq!(
        (job.width, job.height, job.thickness, job.units),
        (usual.width, usual.height, usual.thickness, usual.units)
    );
}

#[test]
fn the_toolpath_summary_s_notes_follow_their_toolpath_through_a_reorder() {
    let mut h = named_toolpaths();
    h.app.job_epoch += 1;
    h.frame(vec![]);
    h.app.summary.picked = Some(EntityId(1));
    h.app.summary.notes = "Use the downcut".into();
    h.app
        .apply(
            "arrange_toolpaths",
            &serde_json::json!({ "order": [2, 1, 3] }),
        )
        .unwrap();
    h.frame(vec![]);
    assert_eq!(h.app.doc.toolpaths[&EntityId(2)].name, "Pocket");
    assert_eq!(
        h.app.summary.picked,
        Some(EntityId(2)),
        "still the Pocket's"
    );
    h.app.invoke(&h.ctx, "undo");
    h.frame(vec![]);
    assert_eq!(h.app.summary.picked, Some(EntityId(1)));
}

#[test]
fn a_tile_and_its_overlap_bigger_than_the_bed_is_said_in_red() {
    let mut h = Harness::new();
    h.app.machines.machines.push(cut_craft_machines::Machine {
        name: "Small".into(),
        width: 300.0,
        height: 300.0,
        ..cut_craft_machines::Machine::default()
    });
    h.app.machines.active = h.app.machines.machines.len() - 1;
    h.app.invoke(&h.ctx, "toolpath_tiling");
    let form = h.app.form.as_mut().unwrap();
    form.params["enabled"] = serde_json::json!(true);
    form.params["tiling"]["width"] = serde_json::json!(300.0);
    form.params["tiling"]["height"] = serde_json::json!(300.0);
    form.params["tiling"]["overlap"] = serde_json::json!(3.0);
    h.frame(vec![]);
    h.frame(vec![]);
    let red = |h: &Harness| {
        h.texts()
            .iter()
            .any(|t| t.contains("bigger than Small's bed"))
    };
    assert!(red(&h), "the tile with its overlap is 306 across");
    let form = h.app.form.as_mut().unwrap();
    form.params["tiling"]["width"] = serde_json::json!(290.0);
    form.params["tiling"]["height"] = serde_json::json!(290.0);
    h.frame(vec![]);
    h.frame(vec![]);
    assert!(!red(&h));
}

#[test]
fn a_machine_with_no_bed_size_gets_no_tile_bed_warning() {
    let mut h = Harness::new();
    // the bed size was not given: 0 × 0, so there is nothing to check the tiles against
    h.app.machines.machines.push(cut_craft_machines::Machine {
        name: "Unsized".into(),
        ..cut_craft_machines::Machine::default()
    });
    h.app.machines.active = h.app.machines.machines.len() - 1;
    h.app.invoke(&h.ctx, "toolpath_tiling");
    let form = h.app.form.as_mut().unwrap();
    form.params["enabled"] = serde_json::json!(true);
    h.frame(vec![]);
    h.frame(vec![]);
    assert!(
        !h.texts()
            .iter()
            .any(|t| t.contains("bigger than Unsized's bed")),
        "an unsized bed cannot be too small"
    );
}

#[test]
fn a_tile_with_nothing_to_cut_gets_no_file() {
    let mut h = Harness::new();
    let area = h.app.doc.job.area();
    // a small square in the left half only
    h.rectangle(area.min.x + 20.0, area.min.y + 20.0, 40.0, 40.0);
    h.app.invoke(&h.ctx, "profile_toolpath");
    let params = h.app.form.as_ref().unwrap().params.clone();
    h.app.apply("profile_toolpath", &params).unwrap();
    h.app.form = None;
    h.app.doc.tiling = Some(cut_craft_core::tiling::Tiling {
        width: area.size().x / 2.0,
        height: area.size().y,
        ..Default::default()
    });
    let post = h.app.output_post().unwrap();
    let form = SaveForm {
        mode: crate::files::SaveMode::VisibleToOneFile,
        group: false,
        post: None,
        add_side: false,
        changes: crate::files::ToolChanges::ToolChanger,
        job_sheet: false,
    };
    let dir = std::env::temp_dir().join(format!("cut-craft-empty-tile-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let written = h
        .app
        .write_toolpath_files(&form, &post, &dir.join("sign.nc"))
        .unwrap();
    let names: Vec<String> = written.iter().map(|p| file_name(p)).collect();
    assert_eq!(
        names,
        ["sign_tile1.nc"],
        "the right-hand tile has nothing to cut"
    );
    assert!(!dir.join("sign_tile2.nc").exists());
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn each_tile_is_numbered_in_the_2d_view_as_its_file_is() {
    let mut h = Harness::new();
    let area = h.app.doc.job.area();
    h.app.doc.tiling = Some(cut_craft_core::tiling::Tiling {
        width: area.size().x / 2.0,
        height: area.size().y,
        ..Default::default()
    });
    h.frame(vec![]);
    h.frame(vec![]);
    let one = h.text_at("Tile 1").expect("the left tile is numbered");
    let two = h.text_at("Tile 2").expect("the right tile is numbered");
    assert!(one.x < two.x, "numbered from the left");
}

#[test]
fn save_toolpaths_says_tiling_is_on_and_how_many_tiles() {
    let mut h = named_toolpaths();
    h.app.invoke(&h.ctx, "save_toolpaths");
    h.frame(vec![]);
    h.frame(vec![]);
    assert!(!h.texts().iter().any(|t| t.starts_with("Tiling on")));
    h.app.save_form = None;
    let area = h.app.doc.job.area();
    let tiling = cut_craft_core::tiling::Tiling {
        width: area.size().x / 2.0,
        height: area.size().y,
        ..cut_craft_core::tiling::Tiling::default()
    };
    h.app.doc.tiling = Some(tiling);
    h.app.invoke(&h.ctx, "save_toolpaths");
    h.frame(vec![]);
    h.frame(vec![]);
    assert!(
        h.texts()
            .iter()
            .any(|t| t.starts_with("Tiling on: 2 tiles") && t.contains("_tile2")),
        "{:?}",
        h.texts()
    );
}

#[test]
fn view_gcode_shows_what_saving_would_write_without_saving_a_file() {
    // a 0.6.11 user: "Sometimes just need to quickly look at gcode and not need to save it"
    use super::panels::VIEW_GCODE;
    let mut h = ready_for_output();
    h.click(h.text_at(VIEW_GCODE).unwrap(), Modifiers::NONE);
    h.frame(vec![]);
    h.frame(vec![]);
    let view = h
        .app
        .gcode_view
        .as_ref()
        .expect("the G-code window is open");
    let (name, text) = &view.files.as_ref().unwrap()[0];
    assert_eq!(
        std::path::Path::new(name)
            .extension()
            .and_then(|e| e.to_str()),
        Some("nc"),
        "{name}"
    );
    assert!(text.lines().any(|l| l.starts_with("G1")), "{text}");
    // the same text the machine's file would hold, made at the same time
    let form = view.form.clone();
    let post = h.app.form_post(&form).unwrap();
    let path = std::path::PathBuf::from(name);
    let saved = h
        .app
        .toolpath_texts(&form, &post, &path, view.made)
        .unwrap();
    assert_eq!(&saved[0].1, text);
    // shown in the window, numbered, through the default post
    assert!(h.text_at(&post.name).is_some(), "says which post");
    assert!(
        h.texts().iter().any(|t| t.starts_with("G1")),
        "the lines are shown"
    );
    assert!(h.text_at("Copy All").is_some());
    assert!(h.text_at("Save Toolpaths…").is_some());
    // nothing written: viewing is not saving
    assert!(!path.exists());
}

#[test]
fn view_gcode_lists_what_the_checks_found_without_stopping_anything() {
    use super::panels::VIEW_GCODE;
    let mut h = ready_for_output();
    // out of date: a warning, shown in amber over the G-code
    h.app.doc.toolpaths.values_mut().next().unwrap().stale = true;
    h.click(h.text_at(VIEW_GCODE).unwrap(), Modifiers::NONE);
    for _ in 0..200 {
        h.frame(vec![]);
        if !h.app.checking() && h.texts().iter().any(|t| t.starts_with("⚠ ")) {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    let warned = h.texts();
    assert!(
        warned
            .iter()
            .any(|t| t.starts_with("⚠ ") && t.contains("out of date")),
        "{warned:?}"
    );
    assert!(h.text_at("What the checks before cutting found. It is your call what to change: viewing saves nothing.").is_some());
    // the G-code is still there to read and copy
    assert!(h.texts().iter().any(|t| t.starts_with("G1")), "{warned:?}");
    assert!(h.text_at("Copy All").is_some());
}

#[test]
fn view_gcode_lists_each_file_when_saving_would_make_several() {
    let mut h = ready_for_output();
    h.rectangle(70.0, 10.0, 20.0, 20.0);
    h.app.invoke(&h.ctx, "pocket_toolpath");
    let mut params = h.app.form.as_ref().unwrap().params.clone();
    // another tool, so separate files per tool make two
    params["tool"]["name"] = serde_json::json!("Another 3 mm end mill");
    params["tool"]["diameter"] = serde_json::json!(3.0);
    h.app.apply("pocket_toolpath", &params).unwrap();
    h.app.form = None;
    h.app.view_gcode(one_file_form());
    let view = h.app.gcode_view.as_ref().unwrap();
    let files = view.files.as_ref().unwrap();
    assert_eq!(files.len(), 2, "a file per tool");
    let first = files[0].0.clone();
    h.frame(vec![]);
    h.frame(vec![]);
    assert!(h.text_at(&first).is_some(), "the file shown is named");
}

#[test]
fn view_gcode_says_why_when_there_is_nothing_to_show() {
    let mut h = Harness::new();
    h.app.view_gcode(one_file_form());
    let view = h.app.gcode_view.as_ref().unwrap();
    assert!(view.files.is_err(), "no toolpaths: nothing to show");
    h.frame(vec![]);
    h.frame(vec![]);
    let said = view_error(&h);
    assert!(h.text_at(&said).is_some(), "the reason is in the window");
}

fn view_error(h: &Harness) -> String {
    h.app
        .gcode_view
        .as_ref()
        .unwrap()
        .files
        .as_ref()
        .unwrap_err()
        .clone()
}

#[test]
fn the_save_toolpaths_form_can_view_its_gcode_too() {
    let mut h = ready_for_output();
    h.app.invoke(&h.ctx, "save_toolpaths");
    h.frame(vec![]);
    h.frame(vec![]);
    let view = h
        .texts_at(super::panels::VIEW_GCODE)
        .into_iter()
        .max_by(|a, b| a.y.total_cmp(&b.y))
        .unwrap();
    h.click(view, Modifiers::NONE);
    h.frame(vec![]);
    assert!(h.app.gcode_view.is_some());
}

#[test]
fn save_toolpaths_window_steps_aside_while_the_save_asks_then_comes_back() {
    // a 0.6.11 user found the save dialog behind a window: Save Toolpaths is a window of its
    // own, in front of Cut Craft's, which the system's dialogs belong to
    let mut h = ready_for_output();
    h.app.invoke(&h.ctx, "save_toolpaths");
    let form = h.app.save_form.clone().unwrap();
    let post = h.app.form_post(&form).unwrap();
    h.app.save_checks = vec![super::SaveChecks {
        plan: h.app.plan(&form, &post).unwrap(),
        changes: form.changes,
        revision: h.app.history.revision(),
        machine: h.app.machines.active,
        found: Some(vec![cut_craft_core::preflight::Finding {
            severity: cut_craft_core::preflight::Severity::Warning,
            message: "Air cutting".to_owned(),
            check: cut_craft_core::preflight::Check::AirCutting,
        }]),
    }];
    h.frame(vec![]);
    h.frame(vec![]);
    // Matthew: each warning is acknowledged before saving; Save waits, greyed
    h.click(h.text_at("Save Toolpaths...").unwrap(), Modifiers::NONE);
    h.frame(vec![]);
    assert!(
        h.app.save_form.is_some(),
        "greyed until the warning is ticked"
    );
    assert!(h.app.save_after.is_none());
    h.click(h.text_at("⚠ Air cutting").unwrap(), Modifiers::NONE);
    h.frame(vec![]);
    assert!(h.app.acknowledged.contains("Air cutting"), "ticked as read");
    h.click(h.text_at("Save Toolpaths...").unwrap(), Modifiers::NONE);
    assert!(h.app.save_form.is_none(), "the form closes first");
    assert!(h.app.save_after.is_some(), "the save waits for it to go");
    assert!(
        h.app.save_anyway.is_none(),
        "nothing asked while it is still there"
    );
    // ticked in the form, the save does not ask again
    assert!(
        h.app.ready_to_save(&form, false).is_some(),
        "ticking was the asking"
    );
    // the question over the window still steps the form aside until answered
    h.app.save_after = None;
    h.app.form_back = Some(form.clone());
    h.app.ask_save_anyway(&form, air_cutting(), false);
    for _ in 0..3 {
        h.frame(vec![]);
    }
    assert!(h.app.save_anyway.is_some(), "then the save asks");
    assert!(h.app.save_form.is_none(), "with the form out of the way");
    h.frame(vec![]);
    h.click(h.text_at("Don't Save").unwrap(), Modifiers::NONE);
    h.frame(vec![]);
    assert!(h.app.save_form.is_some(), "the form is back once answered");
}

#[test]
fn another_job_closes_the_gcode_shown_and_any_save_waiting_on_the_last() {
    let mut h = ready_for_output();
    h.app.view_gcode(one_file_form());
    h.app
        .ask_save_anyway(&one_file_form(), air_cutting(), false);
    h.app.form_back = Some(one_file_form());
    h.app.reset_job();
    assert!(h.app.gcode_view.is_none(), "not the last job's G-code");
    assert!(
        h.app.save_anyway.is_none(),
        "its question is not this job's"
    );
    assert!(h.app.form_back.is_none());
}

#[test]
fn view_gcode_closes_when_saving_starts_so_it_cannot_cover_the_file_dialog() {
    use super::panels::VIEW_GCODE;
    let mut h = ready_for_output();
    h.click(h.text_at(VIEW_GCODE).unwrap(), Modifiers::NONE);
    h.frame(vec![]);
    h.frame(vec![]);
    assert!(h.app.gcode_view.is_some());
    // its Save Toolpaths… opens the form, and the G-code window gives way to it
    h.click(h.text_at("Save Toolpaths…").unwrap(), Modifiers::NONE);
    h.frame(vec![]);
    assert!(h.app.save_form.is_some());
    assert!(h.app.gcode_view.is_none(), "closed for the save");
    // opened again from the form, then Save pressed there: closed before the save runs
    h.app.view_gcode(one_file_form());
    let form = h.app.save_form.clone().unwrap();
    let post = h.app.form_post(&form).unwrap();
    h.app.save_checks = vec![super::SaveChecks {
        plan: h.app.plan(&form, &post).unwrap(),
        changes: form.changes,
        revision: h.app.history.revision(),
        machine: h.app.machines.active,
        found: Some(vec![cut_craft_core::preflight::Finding {
            severity: cut_craft_core::preflight::Severity::Warning,
            message: "Air cutting".to_owned(),
            check: cut_craft_core::preflight::Check::AirCutting,
        }]),
    }];
    h.app.save_after = Some(super::gcode_view::SaveAfter::new(form, false));
    for _ in 0..3 {
        h.frame(vec![]);
    }
    assert!(h.app.save_anyway.is_some(), "the save is asking");
    assert!(
        h.app.gcode_view.is_none(),
        "with the G-code window out of the way"
    );
}

#[test]
fn the_variable_explorer_searches_and_the_job_sheet_and_g_code_get_the_values() {
    let mut h = Harness::new();
    h.app.invoke(&h.ctx, "notes");
    h.frame(vec![]);
    h.app.notes_ui.search = "thick".into();
    h.frame(vec![]);
    h.frame(vec![]);
    assert!(h.text_at("Thickness").is_some());
    assert!(
        h.text_at("Job name").is_none(),
        "other variables are hidden while searching"
    );
    // every place notes go uses the same values
    h.app.doc.notes = "# Cut\n- **{{material.thickness}}** stock".into();
    let thick = super::notes_panel::value_now(&h.app, "material.thickness");
    let setup = h.app.job_setup();
    let sheet =
        fmt_create_job_sheet::sheet_for(&h.app.doc, &h.app.options.job_sheet, &setup).unwrap();
    assert!(
        sheet.contains(&format!("<li><strong>{thick}</strong> stock</li>")),
        "{sheet}"
    );
    let lines = cut_craft_core::notes::gcode_lines(&h.app.doc.notes, &h.app.note_variables());
    assert_eq!(lines, ["Cut".to_owned(), format!("- {thick} stock")]);
}

#[test]
fn the_notes_in_g_code_name_the_post_the_file_is_made_through() {
    let mut h = ready_for_output();
    h.app.doc.notes = "Post {{machine.post}}".into();
    h.app.doc.notes_in_gcode = true;
    let form = h.app.default_save_form();
    let other = h
        .app
        .posts
        .load("Cut Craft Mach3 Arcs (mm) (*.tap)")
        .unwrap();
    let default = h.app.form_post(&form).unwrap();
    assert_ne!(
        other.name, default.name,
        "a post other than the machine's own"
    );
    let path = std::path::PathBuf::from("job.tap");
    let made = chrono::Local::now().naive_local();
    let texts = h.app.toolpath_texts(&form, &other, &path, made).unwrap();
    let text = &texts[0].1;
    assert!(text.contains("Mach3"), "{text}");
    assert!(!text.contains("GRBL Arcs"), "{text}");
}

/// What Save Anyway asks with one warning, air cutting.
fn air_cutting() -> super::super::output::Asking {
    super::super::output::Asking {
        problems: vec!["Air cutting".to_owned()],
        ..Default::default()
    }
}
