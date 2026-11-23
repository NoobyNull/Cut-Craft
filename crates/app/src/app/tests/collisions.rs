//! The checks on toolpaths: collisions of the holder and collet, the tool changer's slots, keep-out
//! zones and Analyse.

use super::*;

/// Rail and Ribbon have what Classic has beside its buttons (Matthew, 2026-11-17): the
/// checks before cutting, the favourites and the hovered tool's guidance.
#[test]
fn rail_and_ribbon_browsers_show_the_checks_before_cutting() {
    use crate::options::WindowLayout;
    for layout in [WindowLayout::Rail, WindowLayout::Ribbon] {
        let mut h = Harness::new();
        h.app.options.general.layout = layout;
        h.frame(vec![]);
        assert!(
            h.text_at("Checks").is_some(),
            "{layout:?}: the browser's tab"
        );
        // picked by clicking its tab, as a user would
        let tab = h.text_at("Checks").unwrap();
        h.press(tab, true, Modifiers::NONE);
        h.press(tab, false, Modifiers::NONE);
        h.frame(vec![]);
        assert_eq!(INSPECTOR_TABS[h.app.inspector_tab], "Checks", "{layout:?}");
        assert!(
            h.text_at("Make a toolpath (or tick one in the list) to check it here")
                .is_some(),
            "{layout:?}: the Analyse checks"
        );
    }
}

#[test]
fn a_toolpath_into_a_keep_out_zone_is_not_made_unless_clipping_is_turned_on() {
    use cut_craft_core::keep_out::KeepOutZone;
    use cut_craft_core::toolpath::MoveKind;
    let mut h = Harness::new();
    h.rectangle(10.0, 10.0, 50.0, 50.0);
    // a clamp over the rectangle's right-hand side
    let zone = cut_craft_core::testing::square(55.0, 20.0, 10.0);
    h.app.doc.keep_out = vec![KeepOutZone::new("Clamp 1", zone)];
    h.app.invoke(&h.ctx, "profile_toolpath");
    let params = h.app.form.as_ref().unwrap().params.clone();
    let refused = h.app.apply("profile_toolpath", &params).unwrap_err();
    assert!(
        refused.starts_with(super::toolpath_output::KEEP_OUT_REFUSED),
        "{refused}"
    );
    assert!(refused.contains("\"Clamp 1\""), "names the zone: {refused}");
    assert!(
        refused.contains("Clip toolpaths at keep-out zones"),
        "{refused}"
    );
    assert_eq!(h.app.doc.toolpaths.len(), 0, "nothing made");
    // clipping turned on in Options: made, the part under the clamp left uncut
    h.app.options.toolpaths.clip_at_keep_out_zones = true;
    assert!(h.app.sync_machine_keep_out());
    h.app.apply("profile_toolpath", &params).unwrap();
    let made = h.app.doc.toolpaths.values().next().expect("a toolpath");
    assert_eq!(
        h.app.zones_entered(made),
        [] as [String; 0],
        "{:?}",
        made.moves
    );
    let cuts = made.moves.iter().filter(|m| m.kind == MoveKind::Cut);
    assert!(
        cuts.clone().any(|m| m.to.x > 55.0),
        "still cut past the clamp"
    );
    assert!(cuts.clone().any(|m| m.to.x < 20.0));
    // no zones: made as it was, with no warning
    h.app.options.toolpaths.clip_at_keep_out_zones = false;
    h.app.sync_machine_keep_out();
    h.app.doc.keep_out.clear();
    h.app.apply("profile_toolpath", &params).unwrap();
    assert!(!h.app.status.message.contains("keep-out"));
}

#[test]
fn keep_out_zone_labels_turn_on_and_off_and_are_remembered() {
    let mut h = Harness::new();
    assert!(h.app.display.keep_out_labels, "shown to start with");
    assert_eq!(
        h.app.display.checked("show_keep_out_zone_labels"),
        Some(true)
    );
    h.app.invoke(&h.ctx, "show_keep_out_zone_labels");
    assert!(!h.app.display.keep_out_labels);
    assert!(
        !h.app.options.toolpaths.keep_out_labels,
        "kept with the options"
    );
    // over a zone the 2D view's menu names it and says where it is edited
    let job = crate::gui::keep_out_menu("Keep-out: Clamp 1", false, false);
    assert_eq!(job[0].name, "Keep-out: Clamp 1");
    assert!(!job[0].is_enabled);
    assert_eq!(job[1].action, "keep_out_zones");
    assert_eq!(job[2].name, "Show Keep-Out Zone Labels");
    let machine = crate::gui::keep_out_menu("Keep-out: Machine clamp 1", true, true);
    assert_eq!(machine[1].action, "machine_configuration");
    assert_eq!(machine[2].name, "Hide Keep-Out Zone Labels");
}

#[test]
fn rapids_go_round_the_jobs_and_the_machines_keep_out_zones_and_save_refuses_a_cut_over_one() {
    use cut_craft_core::keep_out::crossed;
    use cut_craft_core::toolpath::MoveKind;
    let mut h = Harness::new();
    // two parts with a gap between them, cut by one profile toolpath
    h.rectangle(10.0, 10.0, 40.0, 40.0);
    h.rectangle(110.0, 10.0, 40.0, 40.0);
    h.app.invoke(&h.ctx, "select_all_vectors");
    h.app.invoke(&h.ctx, "profile_toolpath");
    let params = h.app.form.as_ref().unwrap().params.clone();
    h.app.apply("profile_toolpath", &params).unwrap();
    h.app.form = None;
    let moves = |app: &CutCraftApp| app.doc.toolpaths.values().next().unwrap().moves.clone();
    let rapids = |app: &CutCraftApp| {
        moves(app)
            .iter()
            .filter(|m| m.kind == MoveKind::Rapid)
            .count()
    };
    let straight = rapids(&h.app);
    // a clamp drawn across the gap, made a keep-out zone: the toolpath is made again round it
    h.rectangle(70.0, 0.0, 20.0, 60.0);
    let clamp = *h.app.doc.vectors.keys().next_back().unwrap();
    h.app.doc.selection = [clamp].into();
    let zone = serde_json::json!({ "action": "create", "clearance": 2.0 });
    h.app.apply("keep_out_zones", &zone).unwrap();
    assert!(!crossed(&h.app.doc.keep_out_zones(), &moves(&h.app)));
    assert!(rapids(&h.app) > straight, "round the clamp");
    // the same clamp fixed to the machine's bed instead: every job keeps out of it
    h.app
        .apply(
            "keep_out_zones",
            &serde_json::json!({ "action": "clear", "clearance": 0.0 }),
        )
        .unwrap();
    assert_eq!(rapids(&h.app), straight, "straight across again");
    h.app.machines.add();
    h.app.machines.active_mut().unwrap().keep_out = vec![cut_craft_machines::machine::Zone {
        x: 70.0,
        y: 0.0,
        width: 20.0,
        height: 60.0,
        name: String::new(),
        stands: 30.0,
    }];
    assert!(h.app.sync_machine_keep_out());
    assert_eq!(
        h.app.doc.machine_keep_out[0].height(),
        (30.0, false),
        "its height comes too"
    );
    h.app.keep_out_changed();
    assert!(!crossed(&h.app.doc.keep_out_zones(), &moves(&h.app)));
    assert!(rapids(&h.app) > straight, "round the machine's clamp");
    // a clamp over a part: the toolpath cannot keep out, so it is not saved
    h.app.machines.active_mut().unwrap().keep_out[0].x = 20.0;
    h.app.sync_machine_keep_out();
    h.app.keep_out_changed();
    assert!(
        h.app.status.message.contains("keep-out"),
        "{}",
        h.app.status.message
    );
    let post = h.app.output_post().unwrap();
    let form = SaveForm {
        mode: crate::files::SaveMode::VisibleToOneFile,
        group: false,
        post: None,
        add_side: false,
        changes: crate::files::ToolChanges::ToolChanger,
        job_sheet: true,
    };
    let path = std::env::temp_dir().join(format!("cut-craft-keep-out-{}.nc", std::process::id()));
    let refused = h.app.save_toolpaths_to(&form, &post, &path).unwrap_err();
    assert!(refused.contains("keep-out zone"), "{refused}");
    assert!(!path.exists(), "nothing written");
}

#[test]
fn the_tool_changer_window_loads_a_holder_with_select_tool() {
    let mut h = Harness::new();
    h.app.machines.add();
    h.app.machines.active_mut().unwrap().set_slot_count(3);
    h.app.invoke(&h.ctx, "tool_changer");
    h.frame(vec![]);
    h.frame(vec![]);
    // each holder: its tool, then Select… to pick one in the Tool Database
    let selects = h.texts_at("Select…");
    assert_eq!(selects.len(), 3, "a Select… per holder");
    h.click(selects[1], Modifiers::NONE);
    select_in_tool_database(&mut h, "6 mm Ball Nose");
    let machine = h.app.machines.active().unwrap();
    assert_eq!(machine.slots[1].as_deref(), Some("6 mm Ball Nose"));
    assert_eq!(
        h.app.tools.get("6 mm Ball Nose").unwrap().number,
        2,
        "loaded in holder 2, it is tool 2"
    );
    assert!(h.app.form.is_none(), "no form was touched");
}

#[test]
fn a_bit_with_no_shank_set_is_flagged_while_its_preview_plays() {
    let flagged = |shank: f64| {
        let mut h = Harness::new();
        h.rectangle(10.0, 10.0, 50.0, 50.0);
        h.app.invoke(&h.ctx, "profile_toolpath");
        let params = h.app.form.as_ref().unwrap().params.clone();
        h.app.apply("profile_toolpath", &params).unwrap();
        h.app.form = None;
        for t in h.app.doc.toolpaths.values_mut() {
            t.tool.shank_diameter = shank;
        }
        h.app.display.animate_preview = true;
        h.app.invoke(&h.ctx, "preview_all_toolpaths");
        h.frame(vec![]);
        h.frame(vec![]);
        h.texts()
            .iter()
            .any(|t| t.contains("no shank diameter set"))
    };
    assert!(flagged(0.0), "not set: said so");
    assert!(!flagged(6.35), "set: nothing to say");
}

#[test]
fn a_tool_changer_loads_tools_and_the_gcode_calls_their_slots() {
    let mut h = Harness::new();
    h.app.machines.add();
    h.app.machines.active_mut().unwrap().set_slot_count(5);
    // load the 3 mm end mill into slot 3 in the Tool Database, kept on OK
    h.app.invoke(&h.ctx, "tool_database");
    let manager = h.app.tool_manager.as_mut().unwrap();
    manager
        .changer
        .as_mut()
        .expect("the machine's tool changer")
        .load("3 mm End Mill", Some(3));
    // a new window is drawn unseen for a frame while egui sizes it
    h.frame(vec![]);
    h.frame(vec![]);
    assert!(h.text_at("Tool changer: My Machine").is_some());
    let ok = *h.texts_at("OK").last().unwrap();
    h.click(ok, Modifiers::NONE);
    assert_eq!(
        h.app.machines.active().unwrap().slot_of("3 mm End Mill"),
        Some(3)
    );
    assert_eq!(
        h.app.tools.get("3 mm End Mill").unwrap().number,
        3,
        "loaded in slot 3, it is tool 3"
    );
    // the toolpath form shows the magazine first: 0 by hand, then each holder
    h.rectangle(10.0, 10.0, 50.0, 50.0);
    h.app.invoke(&h.ctx, "profile_toolpath");
    h.app.pages.set_pinned(Page::Toolpaths, true);
    h.frame(vec![]);
    h.frame(vec![]);
    // Select… opens the Tool Database, the tool changer's holders over its tree
    h.click(
        h.text_at("Select…").expect("the form's tool"),
        Modifiers::NONE,
    );
    h.frame(vec![]);
    h.frame(vec![]);
    assert!(h.text_at("T1  (empty)").is_some());
    let holder = h
        .text_at("T3  3 mm End Mill")
        .expect("holder 3 and its tool");
    h.click(holder, Modifiers::NONE);
    h.frame(vec![]);
    let select = *h.texts_at("Select").last().expect("Select");
    h.click(select, Modifiers::NONE);
    h.frame(vec![]);
    assert!(
        h.text_at("T3  3 mm End Mill").is_some(),
        "the form names its holder"
    );
    let params = h.app.form.as_ref().unwrap().params.clone();
    h.app.apply("profile_toolpath", &params).unwrap();
    // the G-code calls slot 3
    let post = h.app.output_post().unwrap();
    let out = std::env::temp_dir().join(format!("cut-craft-atc-{}.nc", std::process::id()));
    files::save_through(&post, &h.app.slots(), &h.app.doc, &out, None).unwrap();
    let gcode = std::fs::read_to_string(&out).unwrap();
    assert!(gcode.contains("M6 T3"), "{gcode}");
    assert_eq!(
        params["tool"]["name"], "3 mm End Mill",
        "picked by its holder"
    );
    // a tool not in the magazine is changed by hand: T0
    h.app.machines.active_mut().unwrap().slots[2] = None;
    files::save_through(&post, &h.app.slots(), &h.app.doc, &out, None).unwrap();
    let gcode = std::fs::read_to_string(&out).unwrap();
    assert!(gcode.contains("M6 T0"), "{gcode}");
    let _ = std::fs::remove_file(out);
}

#[test]
fn pinned_pages_keep_one_width_whatever_they_show() {
    let mut h = both_pinned();
    h.rectangle(10.0, 10.0, 50.0, 50.0);
    let start = page_widths(&mut h, 1)[0];
    assert!(start[0].is_some() && start[1].is_some(), "both docked");
    let mut seen = vec![];
    for form in [
        "job_setup",
        "profile_toolpath",
        "pocket_toolpath",
        "draw_text",
        "offset_vectors",
    ] {
        h.app.invoke(&h.ctx, form);
        seen.extend(page_widths(&mut h, 8));
        h.app.form = None;
        seen.extend(page_widths(&mut h, 3));
    }
    for tab in 0..gui::DESIGN_SUBTABS.len() {
        h.app.design_subtab = tab;
        seen.extend(page_widths(&mut h, 3));
    }
    for widths in seen {
        assert_eq!(widths, start, "no page grows or shrinks with what it shows");
    }
}

#[test]
fn a_wide_window_widens_the_pages_to_keep_the_view_four_by_three() {
    let mut h = both_pinned();
    let narrow = page_widths(&mut h, 2)[1];
    h.window = vec2(2560.0, 1080.0);
    let wide = page_widths(&mut h, 3);
    let [Some(left), Some(right)] = wide[2] else {
        panic!("both docked")
    };
    assert_eq!(left, right, "the pages share the room evenly");
    assert!(left > narrow[0].unwrap(), "{left} from {narrow:?}");
    assert_eq!(wide[1], wide[2], "settled in the frame the window changed");
    let view = h.app.view_area.expect("the 2D view was drawn");
    let aspect = view.width() / view.height();
    assert!(
        (aspect - crate::pages::VIEW_ASPECT).abs() < 0.03,
        "{aspect} for {view:?}"
    );
}

#[test]
fn the_tool_changer_window_loads_holders_and_tools_carry_their_own_numbers() {
    let mut h = Harness::new();
    h.app.machines.machines.push(cut_craft_machines::Machine {
        name: "Router".into(),
        slots: vec![None; 4],
        ..cut_craft_machines::Machine::default()
    });
    h.app.machines.active = h.app.machines.machines.len() - 1;
    assert!(h.app.available("tool_changer"));
    h.app.invoke(&h.ctx, "tool_changer");
    assert!(h.app.tool_changer_window);
    h.frame(vec![]);
    h.frame(vec![]);
    assert!(h.text_at("Tool Changer").is_some(), "the window");
    assert!(h.text_at("Load by Tool Number").is_some());
    assert!(h.text_at("T4").is_some(), "a row per holder");
}

#[test]
fn tools_loaded_in_the_tool_changer_window_take_their_slot_numbers() {
    let mut h = Harness::new();
    h.app.machines.add();
    let machine = h.app.machines.active_mut().unwrap();
    machine.set_slot_count(4);
    machine.slots[1] = Some("6 mm Ball Nose".into());
    h.app.tool_changer_changed();
    assert_eq!(h.app.tools.get("6 mm Ball Nose").unwrap().number, 2);
    // and a tool loaded before is numbered when the Tool Database opens
    h.app.machines.active_mut().unwrap().slots[3] = Some("3 mm End Mill".into());
    h.app.invoke(&h.ctx, "tool_database");
    let draft = &h.app.tool_manager.as_ref().unwrap().draft;
    assert_eq!(draft.get("3 mm End Mill").unwrap().number, 4);
}

#[test]
fn keep_out_zones_and_shadows_in_the_3d_view_turn_on_and_off_from_the_view_menu() {
    let mut h = Harness::new();
    for (id, on) in [
        ("draw_keep_out_zones_in_3d_view", true),
        ("use_shadow_shading", false),
    ] {
        assert!(h.app.available(id), "{id}");
        assert_eq!(h.app.display.checked(id), Some(on), "{id} as it starts");
        h.app.invoke(&h.ctx, id);
        assert_eq!(h.app.display.checked(id), Some(!on), "{id} turned over");
    }
    assert!(h.app.display.shadows && !h.app.display.keep_out_3d);
}

/// Print Job Sheet's page names the job's file, the active machine, its post and its tool
/// changer's slots.
#[test]
fn the_job_sheet_names_the_file_machine_post_and_slots() {
    let mut h = Harness::new();
    let dir = std::env::temp_dir().join(format!("cut-craft-job-sheet-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    // not saved: no file of its own, so a copy goes to the recovery file
    h.app.recovery_path = Some(dir.join("recovery.ccjob"));
    let setup = h.app.job_setup();
    assert_eq!(setup.job_file, "");
    assert_eq!(
        setup.job_copy,
        dir.join("recovery.ccjob").display().to_string()
    );
    assert!(dir.join("recovery.ccjob").is_file());
    let job = dir.join("Sign.ccraft");
    std::fs::write(&job, b"job").unwrap();
    h.app.session.path = Some(job.clone());
    h.app.machines.machines.push(cut_craft_machines::Machine {
        name: "Shop CNC".into(),
        posts: vec!["FluidNC".into()],
        slots: vec![None, Some("6 mm End Mill".into())],
        ..cut_craft_machines::Machine::default()
    });
    h.app.machines.active = h.app.machines.machines.len() - 1;
    let setup = h.app.job_setup();
    assert_eq!(
        (
            setup.job_file.as_str(),
            setup.machine.as_str(),
            setup.post.as_str()
        ),
        (job.display().to_string().as_str(), "Shop CNC", "FluidNC")
    );
    assert_eq!(setup.job_copy, "", "saved: no copy needed");
    assert_eq!(setup.slots.len(), 2);
    assert!((setup.motion.rapid - h.app.summary.rapid).abs() < 1e-9);
    assert!(h.app.available("print_job_sheet"));
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn tool_changes_go_to_files_of_their_own_the_tool_changer_or_a_pause() {
    use crate::files::{SaveMode, ToolChanges};
    let mut h = Harness::new();
    two_tools(&mut h);
    let dir = std::env::temp_dir().join(format!("cut-craft-changes-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let save = |h: &Harness, changes, name: &str| {
        let form = SaveForm {
            mode: SaveMode::VisibleToOneFile,
            group: false,
            post: None,
            add_side: false,
            changes,
            job_sheet: true,
        };
        let post = h.app.form_post(&form).unwrap();
        h.app
            .save_toolpaths_to(&form, &post, &dir.join(name))
            .unwrap()
    };
    // Cut Craft's own post has a tool changer: separate files still splits by tool
    let done = save(&h, ToolChanges::SeparateFiles, "apart.nc");
    assert!(done.starts_with("Saved 2 toolpath files"), "{done}");
    assert!(dir.join("apart_1_6_mm_End_Mill.nc").exists());
    let gcode = std::fs::read_to_string(dir.join("apart_2_3_mm_End_Mill.nc")).unwrap();
    assert!(!gcode.contains("M6"), "no tool change in a file: {gcode}");
    // the tool changer: one file, M6
    save(&h, ToolChanges::ToolChanger, "atc.nc");
    let gcode = std::fs::read_to_string(dir.join("atc.nc")).unwrap();
    assert!(gcode.contains("M6"), "{gcode}");
    // a pause: one file, M0 and no M6
    save(&h, ToolChanges::Pause, "pause.nc");
    let gcode = std::fs::read_to_string(dir.join("pause.nc")).unwrap();
    assert!(gcode.contains("\nM0") && !gcode.contains("M6"), "{gcode}");
    assert!(gcode.contains("Change to"), "{gcode}");
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn a_machine_with_a_tool_changer_uses_it_by_default_and_one_without_saves_files_per_tool() {
    let mut h = Harness::new();
    h.app.machines.add();
    h.app.invoke(&h.ctx, "save_toolpaths");
    assert_eq!(
        h.app.save_form.as_ref().unwrap().changes,
        crate::files::ToolChanges::SeparateFiles
    );
    h.app.machines.active_mut().unwrap().set_slot_count(4);
    h.app.invoke(&h.ctx, "save_toolpaths");
    assert_eq!(
        h.app.save_form.as_ref().unwrap().changes,
        crate::files::ToolChanges::ToolChanger
    );
}

#[test]
fn a_cut_deeper_than_the_material_asks_first() {
    use super::depth::too_deep;
    let thickness = 18.0;
    let deep = serde_json::json!({ "depth": 20.0 });
    assert_eq!(too_deep("profile_toolpath", &deep, thickness), Some(20.0));
    let exact = serde_json::json!({ "depth": 18.0 });
    assert_eq!(
        too_deep("profile_toolpath", &exact, thickness),
        None,
        "right through, not past"
    );
    let projected = serde_json::json!({ "depth": 20.0, "project_onto_model": true });
    assert_eq!(too_deep("profile_toolpath", &projected, thickness), None);
    assert_eq!(too_deep("pocket_toolpath", &deep, thickness), None);
    assert!(
        super::depth::warning(20.0, 18.0, Units::Mm)
            .contains("20 mm deep, 2 mm past the bottom of the 18 mm thick material")
    );

    let mut h = profile_form();
    let thickness = h.app.doc.job.thickness;
    let calculate = |h: &mut Harness| {
        let apply = h.text_at("Calculate");
        h.click(apply.expect("the form's button"), Modifiers::NONE);
        h.frame(vec![]);
        h.frame(vec![]);
    };
    // OK: made as set
    h.app.form.as_mut().unwrap().params["depth"] = (thickness + 2.0).into();
    h.frame(vec![]);
    calculate(&mut h);
    assert!(h.app.deep_cut.is_some(), "held for the warning");
    assert!(h.text_at("Warning: deeper than the material").is_some());
    assert!(h.app.doc.toolpaths.is_empty(), "nothing made yet");
    let ok = *h.texts_at("OK").last().unwrap();
    h.click(ok, Modifiers::NONE);
    h.frame(vec![]);
    assert!(h.app.deep_cut.is_none());
    for _ in 0..500 {
        if !h.app.doc.toolpaths.is_empty() && h.app.busy.is_none() {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(10));
        h.frame(vec![]);
    }
    assert_eq!(h.app.doc.toolpaths.len(), 1, "made as it was set");
    assert_eq!(
        h.app.form.as_ref().unwrap().params["depth"].as_f64(),
        Some(thickness + 2.0)
    );
    // Cancel: back to the form, the depth corrected
    calculate(&mut h);
    assert!(h.app.deep_cut.is_some());
    let cancel = *h.texts_at("Cancel").last().unwrap();
    h.click(cancel, Modifiers::NONE);
    h.frame(vec![]);
    assert!(h.app.deep_cut.is_none());
    assert_eq!(
        h.app.form.as_ref().unwrap().params["depth"].as_f64(),
        Some(thickness)
    );
    assert!(h.app.form.is_some(), "still on the form");
}

#[test]
fn the_sample_project_opens_with_toolpaths_that_pass_the_checks() {
    let mut h = Harness::at_start();
    assert!(
        h.text_at("Try a sample project").is_some(),
        "on the start page"
    );
    let at = h.text_at("Try a sample project").unwrap();
    h.click(at, Modifiers::NONE);
    h.frame(vec![]);
    assert_eq!(h.app.screen, Screen::Job);
    let names: Vec<&str> = h
        .app
        .doc
        .toolpaths
        .values()
        .map(|t| t.name.as_str())
        .collect();
    assert_eq!(names, [super::sample::CARVE, super::sample::CUT_OUT]);
    let toolpaths: Vec<_> = h.app.doc.toolpaths.values().collect();
    let found = cut_craft_core::preflight::check(&h.app.doc, &toolpaths, &h.app.limits());
    // where to zero the bit is asked of every job whose Material Setup was never opened
    let faults: Vec<_> = found
        .iter()
        .filter(|f| f.check != cut_craft_core::preflight::Check::Zeroing)
        .collect();
    assert_eq!(faults, [] as [&cut_craft_core::preflight::Finding; 0]);
    assert!(
        !h.app.session.is_dirty(&h.app.history),
        "closing an untouched sample asks nothing"
    );
    assert!(h.app.session.path.is_none(), "saving asks where");
}

#[test]
fn the_pocket_form_says_when_the_bit_cannot_reach_everything() {
    let mut h = Harness::new();
    // a slot 2 mm wide: a 3 mm bit can't get into it at all
    h.rectangle(10.0, 10.0, 2.0, 50.0);
    h.click(h.at(10.0, 35.0), Modifiers::NONE);
    h.app.invoke(&h.ctx, "pocket_toolpath");
    h.app.pages.set_pinned(Page::Toolpaths, true);
    h.app.choose_tool("3 mm End Mill");
    h.frame(vec![]);
    h.frame(vec![]);
    let line = h.app.form_reach().expect("a reach line under the tool");
    assert!(
        matches!(line, cut_craft_core::reach::Line::Short(_)),
        "{line:?}"
    );
    assert!(h.text_at(line.text()).is_some(), "shown: {line:?}");
    assert!(line.text().contains("too wide for 1 spot"), "{line:?}");
    // a wider shape fits; the starter bit has no flute length, so it reaches any depth
    h.app.form = None;
    h.rectangle(30.0, 10.0, 40.0, 50.0);
    h.click(h.at(70.0, 35.0), Modifiers::NONE);
    h.app.invoke(&h.ctx, "pocket_toolpath");
    h.app.choose_tool("3 mm End Mill");
    h.frame(vec![]);
    let line = h.app.form_reach().expect("a reach line");
    assert_eq!(line.text(), "Reaches everything", "{line:?}");
}

/// Adding buttons moves none of those already there: each group keeps its tools in their
/// order, the new ones after them.
#[test]
fn the_buttons_already_there_keep_their_places() {
    let toolbar = gui::Toolbar::new();
    let starts = |name: &str, tools: &[&str]| {
        let all: Vec<gui::ToolGroup> = toolbar
            .all_design_groups()
            .into_iter()
            .chain(toolbar.toolpath_groups.iter().copied())
            .collect();
        let group = all.iter().find(|g| g.name == name).unwrap();
        assert_eq!(&group.tools[..tools.len()], tools, "{name}");
    };
    starts(
        "File Operations",
        &[
            "new",
            "open",
            "save",
            "export_vectors",
            "paste_in_place",
            "open_folder",
            "setup_material",
        ],
    );
    starts(
        "Shape Editing",
        &[
            "sculpt",
            "smooth_selected_components",
            "emboss",
            "distort_object",
            "offset_model",
            "slice",
            "add_draft",
            "scale_model_z_height",
            "replace_below",
            "split",
            "bake_components",
            "component_levels",
        ],
    );
    // the Toolpaths page was regrouped on purpose (Matthew, 2026-11-13); its new groups
    // keep their places from here on
    starts(
        "Job and Tools",
        &[
            "material_setup",
            "materials",
            "tool_database",
            "keep_out_zones",
        ],
    );
    starts(
        "From Other Toolpaths",
        &["array_copy_toolpath", "merged_toolpath"],
    );
}

#[test]
fn the_chamfer_form_says_when_it_goes_deeper_than_the_v() {
    let mut h = Harness::new();
    h.rectangle(10.0, 10.0, 40.0, 40.0);
    h.click(h.at(10.0, 30.0), Modifiers::NONE);
    h.app.invoke(&h.ctx, "chamfer_toolpath");
    h.app.pages.set_pinned(Page::Toolpaths, true);
    h.app.choose_tool("90° V-Bit");
    h.frame(vec![]);
    // 3 wide with a 90° V-bit is 3 deep: well inside its 6.35 mm V
    let line = h.app.form_reach().expect("a reach line under the tool");
    assert!(
        matches!(line, cut_craft_core::reach::Line::Reaches(_)),
        "{line:?}"
    );
    // from 5 below the surface it is 8 deep, past the V
    h.app.form.as_mut().unwrap().params["start_depth"] = 5.0.into();
    h.frame(vec![]);
    let line = h.app.form_reach().expect("a reach line");
    assert!(
        matches!(line, cut_craft_core::reach::Line::Short(_)),
        "{line:?}"
    );
    assert!(line.text().contains("the cut is 8.0 mm"), "{line:?}");
}

#[test]
fn analyse_checks_off_the_window_thread_so_the_window_keeps_drawing() {
    // Matthew's lock-up (v0.6.10): Analyse ran the checks before cutting on a 3D finish of
    // 12.7 million moves inside the frame, and the window (and with the card busy, his
    // desktop) froze for over ten minutes. They now run with their progress over the window.
    let mut h = analysing();
    h.app.wait_before_showing = std::time::Duration::ZERO;
    h.frame(vec![]);
    let running = h.app.checking();
    let done = h.app.save_checks.first().is_some_and(|c| c.found.is_some());
    assert!(running || done, "started");
    if running {
        assert!(
            h.text_at("Checking the toolpaths: a big 3D toolpath takes a little while")
                .is_some(),
            "says it is checking"
        );
    }
    let until = std::time::Instant::now() + std::time::Duration::from_mins(1);
    while h.app.checking() && std::time::Instant::now() < until {
        h.frame(vec![]);
    }
    h.frame(vec![]);
    assert!(!h.app.checking(), "finished");
    assert!(h.app.save_checks[0].found.is_some(), "kept");
    assert!(
        h.text_at("✔ Every toolpath matches what it was made from")
            .is_some(),
        "the rest of the tab is shown once checked"
    );
    h.frame(vec![]);
    assert!(!h.app.checking(), "not run again while nothing changes");
}

#[test]
fn cancelled_checks_say_so_and_check_again_runs_them() {
    let mut h = analysing();
    h.frame(vec![]);
    h.frame(vec![]);
    // as Cancel leaves them: the plan known, nothing found yet, nothing running
    h.app.save_checks[0].found = None;
    h.frame(vec![]);
    assert!(
        h.text_at("The checks were cancelled, so nothing has been checked yet")
            .is_some()
    );
    assert!(h.app.busy.is_none(), "not started again by itself");
    h.click(h.text_at("Check Again").unwrap(), Modifiers::NONE);
    h.frame(vec![]);
    h.frame(vec![]);
    assert!(h.app.save_checks[0].found.is_some(), "checked again");
    assert!(h.text_at("Check Again").is_none());
}

#[test]
fn checks_waiting_on_other_work_say_so_rather_than_cancelled() {
    let mut h = analysing();
    h.frame(vec![]);
    h.frame(vec![]);
    h.app.wait_before_showing = std::time::Duration::ZERO;
    let (go, wait) = std::sync::mpsc::channel::<()>();
    h.app.in_background(
        "Calculating Slow Toolpath",
        "test:slow",
        move || {
            wait.recv_timeout(std::time::Duration::from_secs(30)).ok();
        },
        |_, ()| {},
    );
    // the job changed: the checks must run again, once the slow work is done
    h.app.save_checks.clear();
    h.frame(vec![]);
    assert!(
        h.text_at("The checks start once the work running now is done")
            .is_some(),
        "waiting, not cancelled"
    );
    assert!(h.text_at("Check Again").is_none(), "nothing to run again");
    go.send(()).ok();
    until_not_busy(&mut h);
}

#[test]
fn save_toolpaths_waits_for_cancelled_checks_rather_than_saving_unchecked() {
    let mut h = analysing();
    let form = SaveForm {
        mode: crate::files::SaveMode::VisibleToOneFile,
        group: false,
        post: None,
        add_side: false,
        changes: crate::files::ToolChanges::SeparateFiles,
        job_sheet: false,
    };
    let post = h.app.form_post(&form).unwrap();
    // the form's checks, as Cancel leaves them
    h.app.save_checks = vec![super::SaveChecks {
        plan: h.app.plan(&form, &post).unwrap(),
        changes: form.changes,
        revision: h.app.history.revision(),
        machine: h.app.machines.active,
        found: None,
    }];
    assert!(
        h.app.ready_to_save(&form, false).is_none(),
        "not saved unchecked"
    );
    assert!(
        h.app.status.message.contains("were cancelled"),
        "{}",
        h.app.status.message
    );
    assert_eq!(h.app.save_checks.len(), 1, "not started again by itself");
}

#[test]
fn changing_the_tool_changer_checks_the_toolpaths_again() {
    // the checks are kept per plan, job and machine; a holder or slot change on the same
    // machine must not keep the results worked out for the slots before it
    use crate::files::ToolChanges;
    let mut h = Harness::new();
    two_tools(&mut h);
    let plan = vec![h.app.doc.toolpaths.keys().copied().collect::<Vec<_>>()];
    h.app.machines.add();
    h.app.machines.active_mut().unwrap().set_slot_count(4);
    let warned = |h: &mut Harness| {
        h.app
            .save_checks(&plan, ToolChanges::ToolChanger)
            .expect("checked")
            .iter()
            .any(|f| f.message.contains("no tool changer"))
    };
    assert!(!warned(&mut h));
    h.app.machines.active_mut().unwrap().set_slot_count(0);
    h.app.tool_changer_changed();
    assert!(warned(&mut h), "checked again for the slots now set");
}

#[test]
fn loading_bits_in_the_tool_database_checks_the_toolpaths_again() {
    // the Tool Database's OK puts bits in the tool changer's slots; the checks made with
    // the slots empty must not still say they are missing
    use crate::files::ToolChanges;
    let mut h = Harness::new();
    two_tools(&mut h);
    let plan = vec![h.app.doc.toolpaths.keys().copied().collect::<Vec<_>>()];
    h.app.machines.add();
    h.app.machines.active_mut().unwrap().set_slot_count(5);
    let unloaded = |h: &mut Harness| {
        h.app
            .save_checks(&plan, ToolChanges::ToolChanger)
            .expect("checked")
            .iter()
            .filter(|f| f.message.contains("not loaded in the tool changer"))
            .count()
    };
    assert_eq!(unloaded(&mut h), 2, "neither bit is loaded yet");
    h.app.invoke(&h.ctx, "tool_database");
    let changer = h
        .app
        .tool_manager
        .as_mut()
        .unwrap()
        .changer
        .as_mut()
        .unwrap();
    changer.load("6 mm End Mill", Some(1));
    changer.load("3 mm End Mill", Some(2));
    h.frame(vec![]);
    h.frame(vec![]);
    let ok = *h.texts_at("OK").last().unwrap();
    h.click(ok, Modifiers::NONE);
    assert_eq!(
        h.app.machines.active().unwrap().slot_of("3 mm End Mill"),
        Some(2)
    );
    assert_eq!(unloaded(&mut h), 0, "checked again for the bits now loaded");
}

#[test]
fn a_check_started_before_the_machine_changed_never_fills_the_checks_after() {
    // Machine Configuration's Apply forgets the checks; one still running for the old
    // limits must not hand its findings to the checks asked for after
    use crate::files::ToolChanges;
    let mut h = Harness::new();
    two_tools(&mut h);
    let plan = vec![h.app.doc.toolpaths.keys().copied().collect::<Vec<_>>()];
    h.app.machines.add();
    h.app.wait_before_showing = std::time::Duration::ZERO;
    // started with no slots, so it would warn that there is no tool changer
    let _ = h.app.save_checks(&plan, ToolChanges::ToolChanger);
    h.app.machines.active_mut().unwrap().set_slot_count(4);
    h.app.forget_save_checks();
    let stale = h
        .app
        .save_checks(&plan, ToolChanges::ToolChanger)
        .is_some_and(|found| found.iter().any(|f| f.message.contains("no tool changer")));
    assert!(!stale, "the old check's findings were thrown away");
    h.app.wait_before_showing = std::time::Duration::from_mins(10);
    if let Some(running) = h.app.busy.take() {
        running.finish(&mut h.app);
    }
    let found = h
        .app
        .save_checks(&plan, ToolChanges::ToolChanger)
        .expect("checked");
    assert!(!found.iter().any(|f| f.message.contains("no tool changer")));
}

#[test]
fn the_keep_out_check_at_saving_checks_the_moves_that_are_saved() {
    use cut_craft_core::glam::DVec3;
    use cut_craft_core::keep_out::KeepOutZone;
    use cut_craft_core::toolpath::{Move, MoveKind};
    let mut h = Harness::new();
    h.app.doc = cut_craft_core::testing::with_toolpath();
    let id = EntityId(1);
    let toolpath = h.app.doc.toolpaths.get_mut(&id).unwrap();
    toolpath.tool.diameter = 6.0;
    let at = |kind, x: f64, y: f64| Move {
        kind,
        to: DVec3::new(x, y, -1.0),
    };
    // a cut bowed 0.09 mm away from a clamp: thinned, it runs straight and 0.05 mm into it
    toolpath.moves = vec![
        at(MoveKind::Plunge, 0.0, 0.0),
        at(MoveKind::Cut, 5.0, 0.09),
        at(MoveKind::Cut, 10.0, 0.0),
    ];
    let clamp = cut_craft_core::testing::square(4.9, -3.5, 0.55);
    h.app.doc.keep_out = vec![KeepOutZone::new("Clamp 1", clamp)];
    h.app.options.toolpaths.tolerance_2d = 0.1;
    let plan = vec![vec![id]];
    assert_eq!(h.app.not_kept_out(&h.app.doc, &plan), [] as [String; 0]);
    // the checks before cutting check the moves as saved, thinned: Must fix
    let found = h
        .app
        .preflight(&plan, crate::files::ToolChanges::SeparateFiles);
    let keep_out = found
        .iter()
        .find(|f| f.check == cut_craft_core::check_list::Check::KeepOut)
        .expect("the thinned cut enters the clamp's zone");
    assert_eq!(
        keep_out.severity,
        cut_craft_core::preflight::Severity::Error
    );
    assert!(keep_out.message.contains("Clamp 1"), "{}", keep_out.message);
}

#[test]
fn a_previewed_toolpath_s_cuts_keep_its_colour_through_a_reorder() {
    let mut h = named_toolpaths();
    h.app.job_epoch += 1;
    h.frame(vec![]);
    // the Pocket (id 1) was previewed: its cells carry its mark
    let mark = h.app.preview_mark(EntityId(1));
    let pocket = h.app.preview_fills()[&mark];
    assert_eq!(pocket, h.app.doc.toolpaths[&EntityId(1)].preview_colour(0));
    h.app
        .apply(
            "arrange_toolpaths",
            &serde_json::json!({ "order": [2, 1, 3] }),
        )
        .unwrap();
    h.frame(vec![]);
    let now = h.app.doc.toolpaths[&EntityId(2)].preview_colour(1);
    assert_eq!(h.app.preview_fills()[&mark], now, "the Pocket's swatch");
    assert_eq!(
        h.app.preview_mark(EntityId(2)),
        mark,
        "previewing the Pocket again marks its cells the same"
    );
}

#[test]
fn a_laser_cutting_into_a_keep_out_zone_is_not_told_to_turn_on_clipping() {
    use cut_craft_core::toolpath::{Move, MoveKind};
    let mut h = Harness::new();
    h.app.doc = cut_craft_core::testing::with_toolpath();
    h.app.doc.clip_keep_out = false;
    h.app.doc.keep_out = vec![cut_craft_core::keep_out::KeepOutZone::new(
        "Clamp",
        cut_craft_core::testing::square(40.0, -10.0, 20.0),
    )];
    let mut t = h.app.doc.toolpaths.values().next().unwrap().clone();
    let at = |kind, x: f64, z: f64| Move {
        kind,
        to: cut_craft_core::glam::DVec3::new(x, 0.0, z),
    };
    // a line burned right across the clamp
    t.moves = vec![
        at(MoveKind::Rapid, 0.0, 5.0),
        at(MoveKind::Plunge, 0.0, 0.0),
        at(MoveKind::Cut, 100.0, 0.0),
    ];
    t.power = vec![0.0, 0.0, 0.8];
    let mut c = Change::builder(&h.app.doc);
    c.add_toolpath(t.clone());
    let why = h.app.keep_out_refusal(&c.finish()).expect("refused");
    assert!(!why.contains("Clip toolpaths"), "{why}");
    assert!(why.contains("laser"), "{why}");
    // a router's toolpath is still told it can be clipped
    t.power.clear();
    let mut c = Change::builder(&h.app.doc);
    c.add_toolpath(t);
    let why = h.app.keep_out_refusal(&c.finish()).expect("refused");
    assert!(why.contains("Clip toolpaths"), "{why}");
}

/// A Profile round a rectangle with the bit sticking out only 2 mm below its collet nut, so
/// the nut sinks into the wood round the cut.
fn collet_too_low() -> Harness {
    let mut h = Harness::new();
    h.app.display.animate_preview = false;
    h.app.pages.set_pinned(Page::Toolpaths, true);
    h.rectangle(10.0, 10.0, 50.0, 50.0);
    h.app.invoke(&h.ctx, "profile_toolpath");
    let params = h.app.form.as_ref().unwrap().params.clone();
    h.app.apply("profile_toolpath", &params).unwrap();
    h.app.form = None;
    for toolpath in h.app.doc.toolpaths.values_mut() {
        toolpath.tool.flute_length = 1.0;
        toolpath.tool.stickout = 2.0;
    }
    h
}

#[test]
fn a_preview_marks_the_collet_hitting_the_wood_in_the_list_and_in_red() {
    // Matthew 2026-11-17: the uncut stock the preview keeps is what collisions are checked
    // against
    let mut h = collet_too_low();
    h.app.invoke(&h.ctx, "preview_all_toolpaths");
    h.frame(vec![]);
    let id = *h.app.doc.toolpaths.keys().next().unwrap();
    let collided = h.app.collisions.get(&id).expect("the nut hits");
    assert!(
        collided
            .sentence
            .contains("the collet nut would hit the material"),
        "{}",
        collided.sentence
    );
    assert!(
        h.app
            .status
            .message
            .contains("⚠ The collet or spindle would hit")
    );
    // red in the list
    h.frame(vec![]);
    let name = h.app.doc.toolpaths[&id].name.clone();
    assert!(h.text_at(&format!("⚠ {name}")).is_some());
    // and the wood in the way red in the 3D view
    let surface = h.app.surface_3d().unwrap().into_owned();
    let [r, g, b, _] = super::panels::COLLISION_RED.to_array();
    let red = u32::from_be_bytes([1, r, g, b]);
    assert!(surface.paint.contains(&red));
    // recalculated, what was found no longer holds
    h.app.doc.toolpaths.get_mut(&id).unwrap().moves.pop();
    assert_eq!(h.app.current_collisions().count(), 0);
}

#[test]
fn the_collet_nut_coming_down_over_a_clamp_is_marked_after_a_preview() {
    // Matthew 2026-11-17: keep-out zones get a height, so the collet is checked against
    // clamps too
    let mut h = Harness::new();
    h.app.display.animate_preview = false;
    h.app.pages.set_pinned(Page::Toolpaths, true);
    h.rectangle(10.0, 10.0, 50.0, 50.0);
    h.app.invoke(&h.ctx, "profile_toolpath");
    let params = h.app.form.as_ref().unwrap().params.clone();
    h.app.apply("profile_toolpath", &params).unwrap();
    h.app.form = None;
    for toolpath in h.app.doc.toolpaths.values_mut() {
        // the bit reaches the whole cut, so only the clamp is in the nut's way
        toolpath.tool.flute_length = 60.0;
        toolpath.tool.stickout = 60.0;
    }
    let deepest = h
        .app
        .doc
        .toolpaths
        .values()
        .next()
        .unwrap()
        .moves
        .iter()
        .map(|m| m.to.z)
        .fold(0.0, f64::min);
    // a clamp just outside the profile, standing taller than the nut is above the deepest
    // pass
    let o = h.app.doc.job.area().min;
    let at = |x: f64, y: f64| cut_craft_core::geometry::Point::new(o.x + x, o.y + y);
    let clamp = cut_craft_core::geometry::Path::polyline(
        &[
            at(64.0, 20.0),
            at(74.0, 20.0),
            at(74.0, 30.0),
            at(64.0, 30.0),
        ],
        true,
    );
    h.app.doc.keep_out = vec![
        cut_craft_core::keep_out::KeepOutZone::new("Clamp 1", clamp)
            .standing(60.0 + deepest + 10.0),
    ];
    h.app.invoke(&h.ctx, "preview_all_toolpaths");
    h.frame(vec![]);
    let id = *h.app.doc.toolpaths.keys().next().unwrap();
    let collided = h
        .app
        .collisions
        .get(&id)
        .expect("the nut comes over the clamp");
    assert!(
        collided.sentence.contains("below the top of Clamp 1"),
        "{}",
        collided.sentence
    );
}

#[test]
fn checks_asked_for_while_other_work_runs_wait_their_turn_without_holding_up_the_window() {
    // Save Toolpaths and the Analyse tab can both ask for checks in one frame; the second
    // must not wait on the first on the window's thread, nor be kept as started
    use crate::files::ToolChanges;
    let mut h = Harness::new();
    two_tools(&mut h);
    let plan = vec![h.app.doc.toolpaths.keys().copied().collect::<Vec<_>>()];
    h.app.wait_before_showing = std::time::Duration::ZERO;
    let (go, wait) = std::sync::mpsc::channel::<()>();
    h.app.in_background(
        "Calculating Slow Toolpath",
        "test:slow",
        move || {
            wait.recv_timeout(std::time::Duration::from_secs(3)).ok();
        },
        |_, ()| {},
    );
    assert!(h.app.busy.is_some(), "the slow work is running");
    let asked = std::time::Instant::now();
    assert!(
        h.app.save_checks(&plan, ToolChanges::ToolChanger).is_none(),
        "not checked yet"
    );
    assert!(
        asked.elapsed() < std::time::Duration::from_secs(2),
        "the window kept drawing"
    );
    assert!(h.app.save_checks.is_empty(), "not counted as started");
    go.send(()).ok();
    until_not_busy(&mut h);
    // asked for again (the next frame): checked now
    let _ = h.app.save_checks(&plan, ToolChanges::ToolChanger);
    until_not_busy(&mut h);
    assert!(
        h.app.save_checks(&plan, ToolChanges::ToolChanger).is_some(),
        "checked once the worker was free"
    );
}
