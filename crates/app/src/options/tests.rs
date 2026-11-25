#[test]
fn every_view_control_turns_the_other_way_when_inverted() {
    let plain = Controls::default();
    let up = egui::vec2(0.0, 50.0);
    let zoomed = plain.view_move(up, 1.0, 400.0).zoom;
    assert!(zoomed > 1.0, "the wheel zooms in: {zoomed}");
    let inverted = Controls {
        invert_wheel: true,
        ..Controls::default()
    };
    let back = inverted.view_move(up, 1.0, 400.0).zoom;
    assert!((back * zoomed - 1.0).abs() < 1e-5, "the other way: {back}");
    // a pinch zooms, or the other way
    assert!((plain.view_move(egui::Vec2::ZERO, 1.25, 400.0).zoom - 1.25).abs() < 1e-6);
    let pinch = Controls {
        invert_pinch: true,
        ..Controls::default()
    };
    assert!((pinch.view_move(egui::Vec2::ZERO, 1.25, 400.0).zoom - 0.8).abs() < 1e-6);
    // drags turning the 3D view, each way on its own
    let drag = egui::vec2(3.0, 4.0);
    assert_eq!(plain.turn(drag), drag);
    let x = Controls {
        invert_drag_x: true,
        ..Controls::default()
    };
    assert_eq!(x.turn(drag), egui::vec2(-3.0, 4.0));
    let y = Controls {
        invert_drag_y: true,
        ..Controls::default()
    };
    assert_eq!(y.turn(drag), egui::vec2(3.0, -4.0));
}

#[test]
fn in_trackpad_mode_two_fingers_pan_and_a_pinch_zooms() {
    let pad = Controls {
        trackpad: true,
        ..Controls::default()
    };
    let swipe = egui::vec2(12.0, -30.0);
    let moved = pad.view_move(swipe, 1.0, 400.0);
    assert_eq!(moved.pan, swipe, "the drawing follows the fingers");
    assert!(
        (moved.zoom - 1.0).abs() < f32::EPSILON,
        "two fingers never zoom"
    );
    assert!((pad.view_move(swipe, 0.5, 400.0).zoom - 0.5).abs() < f32::EPSILON);
    let against = Controls {
        invert_pan: true,
        ..pad
    };
    assert_eq!(against.view_move(swipe, 1.0, 400.0).pan, -swipe);
    // off, the wheel zooms and only a sideways scroll pans, as before
    let wheel = Controls::default().view_move(swipe, 1.0, 400.0);
    assert_eq!(wheel.pan, egui::vec2(12.0, 0.0));
    assert!(wheel.zoom < 1.0);
}

#[test]
fn the_new_controls_start_off_and_old_settings_files_still_read() {
    let old: Controls = serde_json::from_str(r#"{"tilt": "pan_reversed"}"#).unwrap();
    assert_eq!(
        old,
        Controls {
            tilt: Tilt::PanReversed,
            ..Controls::default()
        }
    );
    assert!(!old.trackpad && !old.invert_drag_y && !old.invert_pinch);
}
use super::*;

#[test]
fn a_file_both_unreadable_and_tampered_says_both() {
    let unread = vec!["Display".to_owned()];
    let both = load_message(&unread, true).unwrap();
    assert!(both.contains("Display"), "{both}");
    assert!(both.contains(TAMPERED), "{both}");
    assert_eq!(load_message(&[], true).as_deref(), Some(TAMPERED));
    assert_eq!(load_message(&unread, false), Some(unread_message(&unread)));
    assert_eq!(load_message(&[], false), None);
}
#[test]
fn tilting_pans_as_set_and_reset_puts_back_cut_crafts_own_shortcuts() {
    assert!((Tilt::default().pan(-3.0) + 3.0).abs() < f32::EPSILON);
    assert!((Tilt::PanReversed.pan(-3.0) - 3.0).abs() < f32::EPSILON);
    assert!(Tilt::Nothing.pan(-3.0).abs() < f32::EPSILON);
    let mut options = Options::default();
    options.controls.tilt = Tilt::Nothing;
    (options.controls.shortcuts).insert("pan".to_owned(), vec!["Mouse Back".to_owned()]);
    let saved = serde_json::to_string(&options).unwrap();
    let read: Options = serde_json::from_str(&saved).unwrap();
    assert_eq!(
        read.controls, options.controls,
        "kept from one run to the next"
    );
    assert_eq!(options.reset("controls").controls, Controls::default());
    // options saved before there were any read as the defaults
    let mut old = serde_json::to_value(Options::default()).unwrap();
    old.as_object_mut().unwrap().remove("controls");
    let old: Options = serde_json::from_value(old).unwrap();
    assert_eq!(old.controls, Controls::default());
}

#[test]
fn a_mac_starts_following_the_desktops_light_or_dark() {
    use egui::os::OperatingSystem;
    assert_eq!(Theme::for_system(OperatingSystem::Mac), Theme::System);
    assert_eq!(Theme::for_system(OperatingSystem::Windows), Theme::Light);
    assert_eq!(Theme::for_system(OperatingSystem::Nix), Theme::Light);
}

#[test]
fn out_of_bounds_parts_are_five_percent_red_unless_the_options_say_otherwise() {
    let mut import = Import::default();
    assert_eq!(import.out_of_bounds_percent, 5);
    let c = import.out_of_bounds_colour();
    assert_eq!(c.to_srgba_unmultiplied(), [255, 0, 0, 13], "5% of 255");
    import.out_of_bounds_percent = 0;
    assert_eq!(import.out_of_bounds_colour().a(), 3, "never wholly hidden");
    import.out_of_bounds_percent = 200;
    assert_eq!(import.out_of_bounds_colour().a(), 255);
    // options saved before the setting existed
    let old: Import = serde_json::from_str(r#"{"preview_faces": 1000}"#).unwrap();
    assert_eq!(old.out_of_bounds_percent, 5);
}

#[test]
fn a_new_install_starts_in_ribbon_and_older_options_keep_classic() {
    assert_eq!(Options::default().general.layout, WindowLayout::Ribbon);
    let mut o = Options::default();
    o.general.layout = WindowLayout::Rail;
    let text = serde_json::to_string(&o).unwrap();
    assert!(text.contains("\"layout\":\"rail\""));
    let back: Options = serde_json::from_str(&text).unwrap();
    assert_eq!(back.general.layout, WindowLayout::Rail);
    // options saved before the setting existed had the Classic window
    let mut old: serde_json::Value = serde_json::to_value(&o).unwrap();
    old["general"].as_object_mut().unwrap().remove("layout");
    let old: Options = serde_json::from_value(old).unwrap();
    assert_eq!(old.general.layout, WindowLayout::Classic);
}

#[test]
fn an_older_file_read_setting_by_setting_still_keeps_classic() {
    let mut file = serde_json::to_value(Options::default()).unwrap();
    let general = file["general"].as_object_mut().unwrap();
    general.remove("layout");
    // a setting this version cannot read, so the file is read setting by setting
    general.insert("autosave_seconds".into(), "often".into());
    let read = Options::read_bytes(&serde_json::to_vec(&file).unwrap());
    assert_eq!(read.unread, ["general ▸ autosave_seconds"]);
    assert_eq!(read.options.general.layout, WindowLayout::Classic);
}

#[test]
fn layouts_are_listed_ribbon_rail_canvas_classic_last() {
    assert_eq!(
        WindowLayout::ALL,
        [
            WindowLayout::Ribbon,
            WindowLayout::Rail,
            WindowLayout::Canvas,
            WindowLayout::Classic
        ]
    );
    let schema = serde_json::to_string(&schemars::schema_for!(WindowLayout)).unwrap();
    let at = |name: &str| schema.find(name).unwrap();
    assert!(at("\"ribbon\"") < at("\"rail\"") && at("\"rail\"") < at("\"classic\""));
}

#[test]
fn the_layout_question_is_unasked_until_answered_and_then_kept() {
    let o = Options::default();
    assert!(!o.layout_asked);
    let text = serde_json::to_string(&o).unwrap();
    assert!(!text.contains("layout_asked"), "left out while false");
    let mut o = o;
    o.layout_asked = true;
    let back: Options = serde_json::from_str(&serde_json::to_string(&o).unwrap()).unwrap();
    assert!(back.layout_asked);
}

#[test]
fn the_defaults_leave_the_display_as_it_starts() {
    let mut d = Display::default();
    Options::default().apply_to(&mut d);
    assert_eq!(d, Display::default());
}

#[test]
fn options_set_what_the_views_draw() {
    let mut o = Options::default();
    o.view_3d.draw_origin = false;
    o.toolpaths.solid_2d_previews = true;
    o.toolpaths.solid_2d_preview_color = [1, 2, 3];
    let mut d = Display::default();
    o.apply_to(&mut d);
    assert!(!d.origin && d.solid_previews);
    assert_eq!(d.solid_colour, [1, 2, 3]);
}

#[test]
fn they_load_back_as_saved_and_fill_gaps_with_defaults() {
    let o: Options =
        serde_json::from_str(&serde_json::to_string(&Options::default()).unwrap()).unwrap();
    assert_eq!(o, Options::default());
    let older = r#"{ "view_3d": { "draw_origin": false, "draw_material_block": true,
            "color_shaded_view": true, "shaded_background": true, "light_follows_viewer": true },
            "toolpaths": { "auto_open_3d_view": true, "create_2d_previews": true,
            "solid_2d_previews": false }, "general": { "recent_file_list_size": 4 } }"#;
    let older: Options = serde_json::from_str(older).expect("options saved before preloading");
    assert!(!older.view_3d.draw_origin && older.general.preload_icons);
    assert!(
        older.updates.check_at_startup,
        "options saved before updates check for them"
    );
    assert_eq!(
        older.appearance.theme,
        Theme::default(),
        "the system's starting theme, as before themes"
    );
    assert_eq!(
        older.snapping,
        cut_craft_core::snap::SnapOptions::default(),
        "the standard snapping, as before it was kept"
    );
    // snapping is kept from one run to the next, rotate snap angles and all
    let mut snapped = Options::default();
    snapped.snapping.rotate_step = cut_craft_core::snap::RotateSnap::Ten;
    snapped.snapping.snap_to_grid = true;
    let back: Options = serde_json::from_str(&serde_json::to_string(&snapped).unwrap()).unwrap();
    assert_eq!(back.snapping, snapped.snapping);
    let fields =
        crate::form::fields(&serde_json::to_value(schemars::schema_for!(Options)).unwrap());
    let sections: Vec<&str> = fields
        .iter()
        .filter(|f| f.section)
        .map(|f| f.label.as_str())
        .collect();
    assert_eq!(
        sections,
        [
            "3D View Settings",
            "Toolpath Settings",
            "General Settings",
            "Performance",
            "Language",
            "Job Defaults",
            "Confirmation Choices",
            "Importing and Exporting",
            "Appearance",
            "Mouse and Shortcuts",
            "SpaceMouse",
            "Updates",
            "Bug Reports",
            "Web API",
            "Advanced"
        ]
    );
    // what the check keeps for itself is not a setting to change
    let updates = fields.iter().find(|f| f.label == "Updates").unwrap();
    let crate::form::Kind::Group(shown) = &updates.kind else {
        panic!("Updates is a group of settings")
    };
    let shown: Vec<&str> = shown.iter().map(|f| f.key.as_str()).collect();
    assert_eq!(shown, ["check_at_startup"]);
}

#[test]
fn job_defaults_start_blank_and_reset_puts_back_cut_crafts_own_job() {
    // options saved before the page existed: the new start, a blank job
    let mut file = serde_json::to_value(Options::default()).unwrap();
    file.as_object_mut().unwrap().remove(JOB_DEFAULTS);
    let old: Options = serde_json::from_value(file).unwrap();
    assert!(
        !old.job_defaults.classic_startup,
        "a blank job, not the start page"
    );
    let mut o = Options::default();
    o.job_defaults.classic_startup = true;
    o.default_job = Some(cut_craft_core::document::Job {
        width: 1220.0,
        ..Default::default()
    });
    let back = o.reset(JOB_DEFAULTS);
    assert!(!back.job_defaults.classic_startup);
    assert_eq!(back.default_job, None, "Cut Craft's own new job again");
    // another page's reset leaves them be
    let kept = o.reset("general");
    assert_eq!(kept.default_job, o.default_job);
    assert!(kept.job_defaults.classic_startup);
}

#[test]
fn saving_thins_each_toolpath_to_its_familys_tolerance() {
    use cut_craft_core::glam::DVec3;
    use cut_craft_core::toolpath::{Move, MoveKind, Toolpath};
    let line = |strategy: &str| Toolpath {
        name: strategy.into(),
        strategy: strategy.into(),
        params: serde_json::Value::Null,
        tool: cut_craft_core::toolpath::Tool::default(),
        vectors: vec![],
        moves: (0..=10)
            .map(|i| Move {
                kind: MoveKind::Cut,
                to: DVec3::new(f64::from(i), 0.0, -1.0),
            })
            .collect(),
        visible: true,
        power: vec![],
        stale: false,
        group: None,
        drill_bank: None,
        fallbacks: vec![],
        across: vec![],
    };
    let mut doc = cut_craft_core::document::Document::default();
    doc.toolpaths.insert(
        cut_craft_core::document::EntityId(1),
        line("profile_toolpath"),
    );
    doc.toolpaths.insert(
        cut_craft_core::document::EntityId(2),
        line("3d_finishing_toolpath"),
    );
    let mut t = Options::default().toolpaths;
    t.tolerance_3d = 0.0;
    let thinned = t.thinned(&doc);
    let count = |id| {
        thinned.toolpaths[&cut_craft_core::document::EntityId(id)]
            .moves
            .len()
    };
    assert_eq!(
        (count(1), count(2)),
        (2, 11),
        "2D thinned, 3D kept whole at 0"
    );
}

#[test]
fn a_saved_rotary_toolpath_held_off_the_axis_is_thinned_with_each_move_kept_where_it_is_held() {
    use cut_craft_core::Command;
    use cut_craft_core::component::{Combine, Component};
    use cut_craft_core::document::{Along, Document, Job};
    // a bump on a round blank, cut from four sides with A held still: the bit is held
    // off the axis, so each move has its own place across it
    let mut doc = Document {
        job: Job::rotary(60.0, 40.0, Along::X),
        ..Document::default()
    };
    let model = Component::from_fn("Bump", Combine::Add, doc.job.area(), 0.5, |p| {
        ((p.x - 30.0).abs() < 15.0).then_some(17.0)
    });
    let mut c = cut_craft_core::Change::builder(&doc);
    c.add_new_component(model);
    doc.apply(&c.finish()).unwrap();
    let p = cmd_rotary_machining_toolpath::Params {
        strategy: cmd_rotary_machining_toolpath::Strategy::Indexed,
        ..cmd_rotary_machining_toolpath::Params::default()
    };
    let change = cmd_rotary_machining_toolpath::RotaryMachiningToolpath::run(&doc, &p).unwrap();
    doc.apply(&change).unwrap();
    let toolpath = doc.toolpaths.values().next().unwrap();
    assert!(!toolpath.across.is_empty(), "held off the axis");
    // what Save Toolpaths writes (and the turning preview plays, thinned the same way):
    // fewer moves, each still held where it was calculated
    let saved = Options::default().toolpaths.thinned(&doc);
    let kept = saved.toolpaths.values().next().unwrap();
    assert_eq!(kept.moves.len(), kept.across.len());
    assert!(kept.moves.len() < toolpath.moves.len(), "thinned");
    let mut from = 0;
    for (m, across) in kept.moves.iter().zip(&kept.across) {
        let i = from
            + toolpath.moves[from..]
                .iter()
                .position(|o| o == m)
                .expect("a kept move is one of the toolpath's own");
        assert_eq!(
            *across, toolpath.across[i],
            "move {i} keeps its place across"
        );
        from = i + 1;
    }
}

#[test]
fn a_tolerance_is_never_more_than_a_tenth_of_a_millimetre() {
    use cut_craft_core::simplify::Family;
    let mut t = Options::default().toolpaths;
    t.tolerance_2d = 5.0;
    t.tolerance_3d = 1.0;
    t.tolerance_vcarve = 25.4;
    for family in [Family::TwoD, Family::ThreeD, Family::VCarve] {
        assert!((t.tolerance(family) - MAX_TOLERANCE).abs() < 1e-12);
    }
    let schema = serde_json::to_value(schemars::schema_for!(ToolpathSettings)).unwrap();
    for field in ["tolerance_2d", "tolerance_3d", "tolerance_vcarve"] {
        assert_eq!(schema["properties"][field]["maximum"], 0.1, "{field}");
    }
}

/// `o` as saved to a file, read back.
fn saved(o: &Options) -> Vec<u8> {
    static FILES: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    let n = FILES.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!("cc-options-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join(format!("{n}.json"));
    o.save_to(&path).unwrap();
    let bytes = std::fs::read(&path).unwrap();
    let _ = std::fs::remove_file(path);
    bytes
}

#[test]
fn a_setting_that_cannot_be_read_loses_only_itself_and_is_named() {
    let mut o = Options::default();
    o.controls
        .shortcuts
        .insert("pan".to_owned(), vec!["Mouse Back".to_owned()]);
    o.default_job = Some(cut_craft_core::document::Job {
        width: 300.0,
        ..Default::default()
    });
    o.appearance.system_title_bar = true;
    let mut file: serde_json::Value = serde_json::from_slice(&saved(&o)).unwrap();
    // a theme a newer version knows, and a section of the wrong kind
    file["appearance"]["theme"] = "neon".into();
    file["favourites"] = 7.into();
    let read = Options::read_bytes(&serde_json::to_vec(&file).unwrap());
    assert_eq!(read.options.controls, o.controls, "the shortcuts kept");
    assert_eq!(read.options.default_job, o.default_job);
    assert!(
        read.options.appearance.system_title_bar,
        "the rest of it kept"
    );
    assert_eq!(read.options.appearance.theme, Theme::default());
    assert_eq!(read.unread, ["appearance ▸ theme", "favourites"]);
    assert!(!read.tampered);
    // a file that reads whole names nothing
    assert_eq!(Options::read_bytes(&saved(&o)).unread, [] as [String; 0]);
    // and one that is not options at all names the lot
    assert_eq!(Options::read_bytes(b"{ not json").unread, ["everything"]);
}

#[test]
fn advanced_settings_saved_by_cut_craft_load_back_as_saved() {
    let mut o = Options::default();
    o.advanced.cores = 2;
    o.general.recent_file_list_size = 7;
    let (back, tampered) = Options::from_bytes(&saved(&o));
    assert!(!tampered);
    assert_eq!(back, o);
}

#[test]
fn advanced_settings_edited_outside_cut_craft_go_back_to_their_defaults() {
    let mut o = Options::default();
    o.advanced.cores = 2;
    let mut file: serde_json::Value = serde_json::from_slice(&saved(&o)).unwrap();
    file["advanced"]["cores"] = 64.into();
    // a normal setting edited by hand is kept
    file["general"]["recent_file_list_size"] = 9.into();
    let (back, tampered) = Options::from_bytes(&serde_json::to_vec(&file).unwrap());
    assert!(tampered);
    assert_eq!(back.advanced, Advanced::default());
    assert_eq!(back.general.recent_file_list_size, 9);
    // taking the check away does not get round it
    file.as_object_mut().unwrap().remove(SEAL);
    let (back, tampered) = Options::from_bytes(&serde_json::to_vec(&file).unwrap());
    assert!(tampered);
    assert_eq!(back.advanced, Advanced::default());
}

#[test]
fn a_seal_made_before_dxf_curve_detail_still_passes_and_the_detail_is_sealed() {
    // the seal an older Cut Craft wrote over { wayland_native, cores } alone
    #[derive(Serialize)]
    struct Before {
        wayland_native: bool,
        cores: u32,
    }
    let before = Before {
        wayland_native: false,
        cores: 2,
    };
    let seal = seal_of(&serde_json::to_vec(&before).unwrap());
    let mut file = serde_json::to_value(Options::default()).unwrap();
    file["advanced"] = serde_json::to_value(&before).unwrap();
    file[SEAL] = seal.into();
    let (back, tampered) = Options::from_bytes(&serde_json::to_vec(&file).unwrap());
    assert!(!tampered, "an upgrade is not an edit");
    assert_eq!(back.advanced.cores, 2);
    assert_eq!(back.advanced.dxf_spline_detail, 32);
    // changed in Cut Craft it loads back; changed by hand it goes back to 32
    let mut o = Options::default();
    o.advanced.dxf_spline_detail = 64;
    let (back, tampered) = Options::from_bytes(&saved(&o));
    assert!(!tampered);
    assert_eq!(back.advanced.dxf_spline_detail, 64);
    let mut file: serde_json::Value = serde_json::from_slice(&saved(&o)).unwrap();
    file["advanced"]["dxf_spline_detail"] = 200.into();
    let (back, tampered) = Options::from_bytes(&serde_json::to_vec(&file).unwrap());
    assert!(tampered);
    assert_eq!(back.advanced.dxf_spline_detail, 32);
}

#[test]
fn importing_settings_default_to_what_the_importers_used() {
    // what apply hands the importers; checked without calling it, since import tests
    // running beside this one read what it sets
    use cut_craft_core::import_settings::{self, PIXELS_PER_INCH, SPLINE_SAMPLES};
    let o = Options::default();
    assert_eq!(o.import.picture_pixels_per_inch, PIXELS_PER_INCH);
    assert_eq!(o.advanced.dxf_spline_detail, SPLINE_SAMPLES);
    assert_eq!(import_settings::pixels_per_inch(), PIXELS_PER_INCH);
    assert_eq!(import_settings::spline_samples(), SPLINE_SAMPLES);
    assert!((o.import.export_line_points - fmt_export_pdf::LINE).abs() < 1e-12);
    let old: Import = serde_json::from_str("{}").unwrap();
    assert!((old.export_line_points - fmt_export_pdf::LINE).abs() < 1e-12);
}

#[test]
fn library_limits_reach_the_library_and_are_sealed() {
    let mut o = Options::default();
    o.advanced.library_rescan_seconds = 60;
    o.advanced.library_most_files = 5000;
    o.advanced.preview_cache_days = 7;
    o.advanced.previews_on_card = 100;
    // what apply hands the Library, checked without setting it: Library tests beside
    // this one read it
    assert_eq!(o.advanced.library_limits(), (60, 5000, 7, 100));
    // out of range in the file: kept to the range
    o.advanced.library_most_files = 1;
    assert_eq!(o.advanced.library_limits().1, 1000);
    assert_eq!(
        Advanced::default().library_limits(),
        (5, crate::clipart::MOST, 60, 400)
    );
    // saved by Cut Craft they load back; edited by hand they go back to the defaults
    let mut o = Options::default();
    o.advanced.previews_on_card = 100;
    let (back, tampered) = Options::from_bytes(&saved(&o));
    assert!(!tampered);
    assert_eq!(back.advanced.previews_on_card, 100);
    let mut file: serde_json::Value = serde_json::from_slice(&saved(&o)).unwrap();
    file["advanced"]["library_rescan_seconds"] = 1.into();
    let (back, tampered) = Options::from_bytes(&serde_json::to_vec(&file).unwrap());
    assert!(tampered);
    assert_eq!(back.advanced, Advanced::default());
}

#[test]
fn gadget_limits_reach_gadgets_and_are_sealed() {
    use cut_craft_scripting::Limits;
    assert_eq!(
        Advanced::default().gadget_limits(),
        Limits::default(),
        "the fixed values they replace"
    );
    let mut o = Options::default();
    o.advanced.gadget_million_instructions = 1000;
    o.advanced.gadget_memory_mb = 256;
    o.advanced.gadget_most_commands = 50_000;
    // what apply hands gadgets, checked without setting it: gadget tests beside this one
    // read it
    assert_eq!(
        o.advanced.gadget_limits(),
        Limits {
            instructions: 1_000_000_000,
            memory: 256 << 20,
            commands: 50_000,
        }
    );
    // out of range in the file: kept to the range
    o.advanced.gadget_memory_mb = 1;
    assert_eq!(o.advanced.gadget_limits().memory, 8 << 20);
    // saved by Cut Craft they load back; edited by hand they go back to the defaults
    let mut o = Options::default();
    o.advanced.gadget_most_commands = 50_000;
    let (back, tampered) = Options::from_bytes(&saved(&o));
    assert!(!tampered);
    assert_eq!(back.advanced.gadget_most_commands, 50_000);
    let mut file: serde_json::Value = serde_json::from_slice(&saved(&o)).unwrap();
    file["advanced"]["gadget_memory_mb"] = 1024.into();
    let (back, tampered) = Options::from_bytes(&serde_json::to_vec(&file).unwrap());
    assert!(tampered);
    assert_eq!(back.advanced, Advanced::default());
}

#[test]
fn cutting_accuracy_reaches_the_toolpaths_in_mm_and_is_sealed() {
    use cut_craft_core::accuracy::{self, Accuracy};
    assert_eq!(
        (
            Advanced::default().curve_accuracy,
            Advanced::default().finishing_3d_accuracy
        ),
        (10, 2),
        "the fixed values they replace, in micrometres"
    );
    let mut o = Options::default();
    o.advanced.curve_accuracy = 50;
    o.advanced.roughing_3d_accuracy = 20;
    o.advanced.finishing_3d_accuracy = 5;
    o.advanced.rest_threshold = 100;
    o.advanced.nest_most_sheets = 200;
    o.advanced.nest_most_fillers = 10;
    // what apply hands the toolpaths, checked without setting it: toolpath tests beside
    // this one read it
    assert_eq!(
        o.advanced.accuracy(),
        Accuracy {
            curve: 0.05,
            roughing_3d: 0.02,
            finishing_3d: 0.005,
            rest: 0.1,
            nest_sheets: 200,
            nest_fillers: 10,
        }
    );
    assert_eq!(Advanced::default().accuracy(), Accuracy::default());
    assert_eq!(
        accuracy::get(),
        Accuracy::default(),
        "nothing here changed it"
    );
    // a gouging finish set by hand goes back to the safe default
    let mut file: serde_json::Value = serde_json::from_slice(&saved(&Options::default())).unwrap();
    file["advanced"]["finishing_3d_accuracy"] = 20.into();
    let (back, tampered) = Options::from_bytes(&serde_json::to_vec(&file).unwrap());
    assert!(tampered);
    assert_eq!(back.advanced.finishing_3d_accuracy, 2);
}

#[test]
fn options_saved_before_the_check_keep_their_wayland_choice() {
    let older = r#"{ "view_3d": { "draw_origin": true, "draw_material_block": true,
            "color_shaded_view": true, "shaded_background": true, "light_follows_viewer": true },
            "toolpaths": { "auto_open_3d_view": true, "create_2d_previews": true,
            "solid_2d_previews": false }, "general": { "recent_file_list_size": 4 },
            "advanced": { "wayland_native": true } }"#;
    let (back, tampered) = Options::from_bytes(older.as_bytes());
    assert!(!tampered, "nothing to warn about");
    assert!(back.advanced.wayland_native);
}

#[test]
fn a_page_resets_to_its_defaults_alone() {
    let mut o = Options::default();
    o.general.recent_file_list_size = 9;
    o.advanced.cores = 3;
    o.favourites.design = vec!["draw_circle".into()];
    let general = o.reset("general");
    assert_eq!(general.general, Options::default().general);
    assert_eq!(general.advanced.cores, 3, "other pages kept");
    assert_eq!(general.favourites.design, ["draw_circle"]);
    assert_eq!(o.reset("advanced").advanced, Advanced::default());
    assert_eq!(o.reset("no_such_page"), o);
}

#[test]
fn new_settings_start_as_the_values_they_replace() {
    let o = Options::default();
    assert_eq!(
        f64::from(o.general.autosave_seconds),
        crate::app::AUTOSAVE_SECONDS
    );
    assert!((o.general.flyout_hold_seconds - crate::flyout::HOLD_SECONDS).abs() < 1e-9);
    assert_eq!(o.appearance.text_size, 115);
    assert!((o.appearance.text_scale() - 1.15).abs() < 1e-6);
    // options saved before these settings get the same
    let older = r#"{ "view_3d": { "draw_origin": true, "draw_material_block": true,
            "color_shaded_view": true, "shaded_background": true, "light_follows_viewer": true },
            "toolpaths": { "auto_open_3d_view": true, "create_2d_previews": true,
            "solid_2d_previews": false }, "general": { "recent_file_list_size": 4 } }"#;
    let (older, _) = Options::from_bytes(older.as_bytes());
    assert_eq!(older.general.autosave_seconds, o.general.autosave_seconds);
    assert!((older.general.flyout_hold_seconds - o.general.flyout_hold_seconds).abs() < 1e-9);
    assert_eq!(older.appearance.text_size, 115);
    // every tool shows, in its group's box, unless folded on purpose
    assert!(!o.general.fold_tool_groups);
    assert!(!older.general.fold_tool_groups);
}

#[test]
fn text_size_stays_readable_whatever_the_file_says() {
    let mut a = Appearance {
        text_size: 5,
        ..Appearance::default()
    };
    assert!((a.text_scale() - 0.8).abs() < 1e-6, "no smaller than 80%");
    a.text_size = 900;
    assert!((a.text_scale() - 2.0).abs() < 1e-6, "no larger than 200%");
}

#[test]
fn the_3d_view_settings_start_as_the_values_they_replace() {
    let v = Options::default().view_3d;
    assert!((v.pick_reach() - crate::view3d::PICK_REACH).abs() < 1e-6);
    assert!((v.field_of_view() - crate::view3d::FIELD_OF_VIEW).abs() < 1e-9);
    assert!((v.settle() - crate::app::SETTLE_SECONDS).abs() < 1e-9);
    let wild = View3d {
        click_reach: 500.0,
        perspective_angle: 179.0,
        settle_seconds: 0.0,
        ..v
    };
    assert!((wild.pick_reach() - 20.0).abs() < 1e-6);
    assert!((wild.field_of_view() - 90_f64.to_radians()).abs() < 1e-9);
    assert!((wild.settle() - 0.05).abs() < 1e-9);
}

#[test]
fn the_toolpath_settings_start_as_the_values_they_replace() {
    let o = Options::default();
    assert!((o.toolpaths.preview_speed() - crate::app::PLAYBACK_SPEED).abs() < 1e-9);
    assert!(
        (o.toolpaths.preview_speed() - 10.0).abs() < 1e-9,
        "previews start ten times faster than the machine"
    );
    assert!((o.confirmations.left() - cut_craft_core::checks::LEFT).abs() < 1e-9);
    for older in [
        r#"{ "cutting_checks": "ask" }"#,
        r#"{ "cutting_checks": "save_and_tell" }"#,
    ] {
        let older: Confirmations = serde_json::from_str(older).unwrap();
        assert_eq!(
            older,
            Confirmations::default(),
            "older options, with the Save and tell choice since taken out, still load"
        );
    }
    let wild = ToolpathSettings {
        preview_multiplier: 1e9,
        ..o.toolpaths
    };
    assert!((wild.preview_speed() - 1000.0).abs() < 1e-9);
}

#[test]
fn a_bit_change_takes_two_minutes_by_hand_and_a_quarter_by_tool_changer() {
    let o = Options::default();
    assert!((o.toolpaths.change_by_hand - 2.0).abs() < 1e-9);
    assert!((o.toolpaths.change_by_changer - 0.25).abs() < 1e-9);
}

#[test]
fn keep_out_labels_show_and_clipping_is_off_until_turned_on() {
    let o = Options::default();
    assert!(o.toolpaths.keep_out_labels);
    assert!(
        !o.toolpaths.clip_at_keep_out_zones,
        "clipping is off by default"
    );
    // options saved before these settings get the same
    let mut saved = serde_json::to_value(&o.toolpaths).unwrap();
    let fields = saved.as_object_mut().unwrap();
    fields.remove("keep_out_labels");
    fields.remove("clip_at_keep_out_zones");
    fields.remove("change_by_hand");
    fields.remove("change_by_changer");
    fields.remove("file_name_underscores");
    fields.remove("bit_in_file_names");
    fields.remove("advanced_rotary");
    let older: ToolpathSettings = serde_json::from_value(saved).unwrap();
    assert_eq!(older, o.toolpaths);
}

/// WCAG's contrast ratio between two colours, 1 to 21.
fn contrast_of(a: [u8; 3], b: [u8; 3]) -> f64 {
    contrast(a, b)
}

#[test]
fn the_2d_grid_stands_out_from_the_sheet_in_both_themes() {
    let a = Appearance::default();
    for dark in [false, true] {
        let c = a.canvas(dark);
        assert!(
            contrast_of(c.grid_color, c.sheet_color) >= 3.0,
            "dark {dark}: grid {:?} on sheet {:?}",
            c.grid_color,
            c.sheet_color
        );
    }
    // the sheet looks as it always has: each theme's own field colour
    let field = |v: egui::Visuals| rgb(v.widgets.inactive.bg_fill);
    assert_eq!(a.canvas(false).sheet_color, field(egui::Visuals::light()));
    assert_eq!(a.canvas(true).sheet_color, field(egui::Visuals::dark()));
    assert_ne!(a.canvas(false), a.canvas(true), "each theme its own");
}

#[test]
fn the_old_grid_colour_is_dropped_unless_it_was_chosen() {
    let saved = |grid: [u8; 3]| {
        let mut file = serde_json::to_value(Options::default()).unwrap();
        file["appearance"] = serde_json::json!({ "theme": "dark", "grid_color": grid });
        Options::from_bytes(file.to_string().as_bytes()).0
    };
    // the old default, the light theme's faint text, all but hid the grid in the dark
    let old = saved(old_grid_color());
    assert_eq!(old.appearance.theme, Theme::Dark);
    assert_eq!(old.appearance.light_2d, CanvasColours::light());
    assert_eq!(old.appearance.dark_2d, CanvasColours::dark());
    // one picked on purpose is kept, for the light theme it was picked in
    let picked = saved([200, 0, 0]);
    assert_eq!(picked.appearance.light_2d.grid_color, [200, 0, 0]);
    assert_eq!(picked.appearance.dark_2d, CanvasColours::dark());
    let file = serde_json::to_value(&picked).unwrap();
    assert!(
        file["appearance"].get("grid_color").is_none(),
        "not written back"
    );
}

#[test]
fn reset_puts_the_2d_colours_back() {
    let mut o = Options::default();
    o.appearance.dark_2d.sheet_color = [1, 2, 3];
    o.appearance.light_2d.grid_color = [4, 5, 6];
    let reset = o.reset("appearance");
    assert_eq!(reset.appearance.dark_2d, CanvasColours::dark());
    assert_eq!(reset.appearance.light_2d, CanvasColours::light());
}

#[test]
fn standard_line_colours_are_the_theme_s_own() {
    // what the 2D view always drew: vectors in the text colour, toolpaths in the warning
    // colour, rapid moves in the error colour
    let a = Appearance::default();
    assert_eq!(a.line_colors, LinePalette::Standard);
    for (dark, v) in [
        (false, egui::Visuals::light()),
        (true, egui::Visuals::dark()),
    ] {
        let lines = a.lines(dark);
        assert_eq!(lines.vector, rgb(v.text_color()), "dark {dark}");
        assert_eq!(lines.toolpath, rgb(v.warn_fg_color), "dark {dark}");
        assert_eq!(lines.rapid, rgb(v.error_fg_color), "dark {dark}");
    }
}

#[test]
fn colour_blind_lines_differ_in_brightness_not_only_hue() {
    // issue from 0.6.11: a colour-blind user could not tell vectors from toolpaths,
    // the dark theme's grey vectors and orange toolpaths being as bright as each other
    let a = Appearance {
        line_colors: LinePalette::ColorBlind,
        ..Appearance::default()
    };
    for dark in [false, true] {
        let (lines, sheet) = (a.lines(dark), a.canvas(dark).sheet_color);
        let pairs = [
            ("vector and toolpath", lines.vector, lines.toolpath, 3.0),
            ("vector and sheet", lines.vector, sheet, 4.5),
            ("toolpath and sheet", lines.toolpath, sheet, 2.5),
            ("rapid and sheet", lines.rapid, sheet, 2.0),
        ];
        for (what, a, b, least) in pairs {
            assert!(
                contrast_of(a, b) >= least,
                "dark {dark}: {what} {a:?} {b:?} only {:.2}",
                contrast_of(a, b)
            );
        }
    }
    // the standard dark theme's are the pair the report was about
    let standard = Appearance::default().lines(true);
    assert!(contrast_of(standard.vector, standard.toolpath) < 2.0);
}

#[test]
fn my_own_line_colours_are_used_only_once_chosen() {
    let mut a = Appearance::default();
    a.light_2d.toolpath_color = [1, 2, 3];
    a.dark_2d.vector_color = [4, 5, 6];
    a.dark_2d.rapid_color = [7, 8, 9];
    assert_ne!(a.lines(false).toolpath, [1, 2, 3], "still standard");
    a.line_colors = LinePalette::ColorBlind;
    assert_ne!(
        a.lines(false).toolpath,
        [1, 2, 3],
        "still the colour-blind set"
    );
    a.line_colors = LinePalette::MyOwn;
    assert_eq!(a.lines(false).toolpath, [1, 2, 3]);
    assert_eq!(a.lines(true).vector, [4, 5, 6]);
    assert_eq!(a.lines(true).rapid, [7, 8, 9]);
    // my own start as the standard ones, so picking it changes nothing until edited
    let fresh = Appearance {
        line_colors: LinePalette::MyOwn,
        ..Appearance::default()
    };
    assert_eq!(fresh.lines(true), Appearance::default().lines(true));
    assert_eq!(fresh.lines(false), Appearance::default().lines(false));
}

#[test]
fn options_saved_before_line_colours_keep_their_2d_colours() {
    // 0.6.11 saved each theme's sheet and grid colours, nothing more
    let mut file = serde_json::to_value(Options::default()).unwrap();
    file["appearance"]["dark_2d"] =
        serde_json::json!({ "sheet_color": [9, 9, 9], "grid_color": [200, 200, 200] });
    file["appearance"]["light_2d"] =
        serde_json::json!({ "sheet_color": [250, 250, 250], "grid_color": [1, 1, 1] });
    file["appearance"]
        .as_object_mut()
        .unwrap()
        .remove("line_colors");
    let read = Options::read_bytes(file.to_string().as_bytes());
    assert!(read.unread.is_empty(), "{:?}", read.unread);
    let a = read.options.appearance;
    assert_eq!(a.line_colors, LinePalette::Standard);
    assert_eq!(a.dark_2d.sheet_color, [9, 9, 9]);
    assert_eq!(a.light_2d.grid_color, [1, 1, 1]);
    // each theme's line colours start as that theme's own
    let (dark, light) = (CanvasColours::dark(), CanvasColours::light());
    assert_eq!(a.dark_2d.vector_color, dark.vector_color);
    assert_eq!(a.dark_2d.toolpath_color, dark.toolpath_color);
    assert_eq!(a.light_2d.rapid_color, light.rapid_color);
    assert_ne!(dark.vector_color, light.vector_color);
}

#[test]
fn my_own_line_colours_show_only_once_chosen() {
    let schema = serde_json::to_value(schemars::schema_for!(Options)).unwrap();
    let fields = crate::form::fields(&schema);
    let appearance = fields.iter().find(|f| f.key == "appearance").unwrap();
    let crate::form::Kind::Group(inner) = &appearance.kind else {
        panic!("a section");
    };
    let palette = inner.iter().find(|f| f.key == "line_colors");
    assert!(palette.is_some(), "the choice is on the Appearance page");
    let light = inner.iter().find(|f| f.key == "light_2d").unwrap();
    let crate::form::Kind::Group(colours) = &light.kind else {
        panic!("a section");
    };
    let toolpath = colours.iter().find(|f| f.key == "toolpath_color").unwrap();
    let sheet = colours.iter().find(|f| f.key == "sheet_color").unwrap();
    let mut options = Options::default();
    let shown = |f: &crate::form::Field, o: &Options| {
        let values = serde_json::to_value(o).unwrap();
        f.when.as_ref().is_none_or(|w| w.shown(&values))
    };
    assert!(!shown(toolpath, &options));
    assert!(shown(sheet, &options), "the sheet's colour always shows");
    options.appearance.line_colors = LinePalette::MyOwn;
    assert!(shown(toolpath, &options));
}

#[test]
fn a_colour_that_contrasts_is_found_for_any_sheet() {
    for sheet in [
        [0, 0, 0],
        [255, 255, 255],
        [128, 128, 128],
        [60, 60, 60],
        [200, 30, 30],
    ] {
        for toward in [[0, 0, 0], [255, 255, 255], [128, 128, 128]] {
            let grid = contrasting(sheet, toward);
            assert!(
                contrast_of(grid, sheet) >= 3.0,
                "{sheet:?} toward {toward:?}: {grid:?}"
            );
        }
    }
}

#[test]
fn calculations_are_limited_to_80_percent_unless_options_say_otherwise() {
    let o = Options::default();
    assert!(o.performance.limit_calculations);
    assert_eq!(o.performance.share, 80);
    assert_eq!(
        o.performance.limit(),
        cut_craft_core::work::Limit::default()
    );
    let mut off = o.performance.clone();
    off.limit_calculations = false;
    assert!(!off.limit().on, "unticked: no limit at all");
    off.share = 0;
    off.limit_calculations = true;
    assert_eq!(off.limit().share, 1, "kept to the slider's range");
}

#[test]
fn options_saved_before_the_performance_page_keep_their_detail_and_cores() {
    // an older file: detail settings on the 3D View page, cores among the advanced ones
    #[derive(Serialize)]
    struct Before {
        wayland_native: bool,
        cores: u32,
    }
    let all = cut_craft_core::work::all_cores();
    for (cores, wanted) in [
        (0, cut_craft_core::work::Limit::default()),
        (1, cut_craft_core::work::Limit::from_old_cores(1, all)),
    ] {
        let before = Before {
            wayland_native: false,
            cores,
        };
        let mut file = serde_json::to_value(Options::default()).unwrap();
        file.as_object_mut().unwrap().remove("performance");
        let view = file["view_3d"].as_object_mut().unwrap();
        view.insert("preview_detail".into(), 70.into());
        view.insert("adjust_to_frame_rate".into(), false.into());
        view.insert("target_fps".into(), 45.0.into());
        view.insert("still_detail".into(), "low".into());
        view.insert("show_frame_rate".into(), true.into());
        file["advanced"] = serde_json::to_value(&before).unwrap();
        file[SEAL] = seal_of(&serde_json::to_vec(&before).unwrap()).into();
        let (back, tampered) = Options::from_bytes(&serde_json::to_vec(&file).unwrap());
        assert!(!tampered, "an upgrade is not an edit");
        let p = &back.performance;
        assert_eq!(p.preview_detail, 70);
        assert!(!p.adjust_to_frame_rate);
        assert_eq!(p.target_fps, 45.0);
        assert_eq!(p.still_detail, Detail::Low);
        assert!(p.show_frame_rate);
        assert_eq!(p.limit(), wanted, "{cores} cores");
        assert_eq!(
            p.limit().cores_of(all),
            if cores == 0 { wanted.cores_of(all) } else { 1 }
        );
    }
}

#[test]
fn the_performance_page_once_saved_is_not_carried_over_again() {
    let mut o = Options::default();
    o.performance.share = 30;
    o.performance.preview_detail = 10;
    let mut file: serde_json::Value = serde_json::from_slice(&saved(&o)).unwrap();
    // an old setting left in the file by hand changes nothing
    file["view_3d"]["preview_detail"] = 90.into();
    let (back, _) = Options::from_bytes(&serde_json::to_vec(&file).unwrap());
    assert_eq!(back.performance, o.performance);
}
