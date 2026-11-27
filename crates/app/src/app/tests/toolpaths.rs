//! Toolpath forms: making each kind of toolpath, its passes, tabs and settings.

use super::*;

#[test]
fn the_ribbon_layout_shows_a_tabs_groups() {
    let mut h = Harness::new();
    h.given_options(|o| o.general.layout = crate::options::WindowLayout::Ribbon);
    h.frame(vec![]);
    for name in ["Design", "Model", "Toolpaths", "Sheets"] {
        assert!(h.text_at(name).is_some(), "{name}");
    }
    // open, or folded into one button when the window is too narrow for every group
    assert!(
        h.text_at("Create Vectors")
            .or(h.text_at("Create Vectors ▾"))
            .is_some()
    );
    assert!(h.text_at("Cut Out and Clear").is_none());
    assert!(h.text_at("Cut Out and Clear ▾").is_none());
    h.show_ribbon_tab("Toolpaths");
    h.frame(vec![]);
    assert!(
        h.text_at("Cut Out and Clear")
            .or(h.text_at("Cut Out and Clear ▾"))
            .is_some()
    );
    h.invoke("pocket_toolpath");
    h.frame(vec![]);
    assert!(
        h.text_at("Pocket Toolpath").is_some(),
        "the form's heading in the inspector"
    );
}

#[test]
fn a_template_opens_as_a_new_unsaved_job() {
    let dir = std::env::temp_dir().join(format!("cut-craft-template-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("plate.ccjobt");
    let mut h = Harness::new();
    h.rectangle(10.0, 10.0, 20.0, 10.0);
    std::fs::write(&path, cut_craft_core::file::save_job(h.doc())).unwrap();
    let mut fresh = Harness::new();
    assert!(fresh.open(&path));
    assert_eq!(fresh.doc().vectors.len(), h.doc().vectors.len());
    assert_eq!(fresh.job_path(), None, "saving asks for a new name");
    assert!(!fresh.recent_shown().contains(&path));
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn vcarve_shades_the_area_it_would_carve_while_its_form_is_open() {
    let mut h = Harness::new();
    h.rectangle(10.0, 10.0, 20.0, 10.0);
    h.invoke("select_all_vectors");
    h.invoke("vcarve_engrave_toolpath");
    h.frame(vec![]);
    let area = |h: &Harness| -> f64 {
        let found = h.carve_area().expect("found");
        found
            .iter()
            .map(|[a, b, c]| ((b.x - a.x) * (c.y - a.y) - (c.x - a.x) * (b.y - a.y)).abs() / 2.0)
            .sum()
    };
    let fill = area(&h);
    assert!(
        (fill - 200.0).abs() < 1e-6,
        "Fill shades the whole shape: {fill}"
    );
    // Outside: a band beside the edge, outside the rectangle
    h.edit_form(|p| p["place"] = "outside".into());
    h.frame(vec![]);
    let outside = area(&h);
    assert!(outside > 0.0 && (outside - fill).abs() > 1.0, "{outside}");
    assert!(
        h.carve_area()
            .expect("found")
            .iter()
            .flatten()
            .any(|p| p.x < 10.0),
        "outside the edge"
    );
    h.close_form();
    h.frame(vec![]);
    assert!(h.carve_area().is_none(), "gone with the form");
}

#[test]
fn the_toolpath_menu_acts_on_the_toolpath_clicked() {
    let mut h = Harness::new();
    h.rectangle(10.0, 10.0, 50.0, 50.0);
    for _ in 0..2 {
        let mut params = h.opening_settings("profile_toolpath");
        // straight down: a helix sits by the start point, which moves with the shape's place
        params["entry"] = serde_json::json!({ "mode": "plunge" });
        h.run("profile_toolpath", &params).unwrap();
    }
    let min_x = |h: &Harness| -> Vec<f64> {
        let bounds = h.doc().toolpaths.values();
        bounds
            .map(|t| cut_craft_core::testing::cut_bounds(t).min.x)
            .collect()
    };
    h.choose_on_toolpath(0, &["Hide", "This"]);
    let visible: Vec<bool> = h.doc().toolpaths.values().map(|t| t.visible).collect();
    assert_eq!(visible, [false, true]);
    let before = min_x(&h);
    let shift =
        serde_json::json!({ "relative": true, "x": 100.0, "y": 0.0, "anchor": "bottom_left" });
    h.run("move", &shift).unwrap();
    h.choose_on_toolpath(1, &["Recalculate", "Visible"]);
    let after = min_x(&h);
    assert!(
        (after[0] - before[0]).abs() < 1e-6 && (after[1] - before[1] - 100.0).abs() < 1e-6,
        "only the shown toolpath follows its vectors: {before:?} → {after:?}"
    );
    let second = *h.doc().toolpaths.keys().nth(1).unwrap();
    h.choose_on_toolpath(1, &["Rename"]);
    let (id, settings) = h.take_form().unwrap();
    assert_eq!(id, "toolpath_properties");
    assert_eq!(settings["toolpath"], serde_json::json!(second));
    h.choose_on_toolpath(1, &["Delete", "Invisible"]);
    assert_eq!(
        h.doc().toolpaths.keys().copied().collect::<Vec<_>>(),
        [second]
    );
}

#[test]
fn right_clicking_a_toolpath_selects_it_and_opens_its_menu() {
    let mut h = Harness::new();
    h.rectangle(10.0, 10.0, 50.0, 50.0);
    let params = h.opening_settings("profile_toolpath");
    h.run("profile_toolpath", &params).unwrap();
    h.invoke("machining_layout");
    // the toolpath list's section settles its height over a frame or two
    h.frame(vec![]);
    h.frame(vec![]);
    let (&id, toolpath) = h.doc().toolpaths.iter().next().unwrap();
    let row = h
        .text_at(&toolpath.name.clone())
        .expect("the toolpath is listed");
    h.right_click(row);
    assert_eq!(h.selected_toolpath(), Some(id));
    assert!(h.text_at("Recalculate").is_some(), "its menu is open");
}

#[test]
fn the_create_shape_icons_open_it_with_their_profile() {
    let mut h = Harness::new();
    for (id, profile) in [
        ("create_shape_round", "round"),
        ("create_shape_angled", "angular"),
        ("create_shape_smooth", "smooth"),
        ("create_shape_flat", "flat"),
    ] {
        h.invoke(id);
        assert_eq!(
            (h.form_id(), h.form_value("profile").as_str()),
            (Some("create_shape"), Some(profile))
        );
    }
}

#[test]
fn edit_toolpath_reopens_its_form_and_replaces_it() {
    let mut h = Harness::new();
    h.rectangle(10.0, 10.0, 50.0, 50.0);
    h.invoke("profile_toolpath");
    let params = h.form_settings().clone();
    h.run("profile_toolpath", &params).unwrap();
    h.close_form();
    let id = *h.doc().toolpaths.keys().next().unwrap();
    h.select_toolpath(id);
    h.select([]);
    h.invoke("edit_toolpath");
    assert_eq!(
        (h.form_id(), h.form_editing()),
        (Some("profile_toolpath"), Some(id))
    );
    h.edit_form(|p| p["name"] = Value::from("Edited"));
    let params = h.form_settings().clone();
    h.apply_form("profile_toolpath", &params).unwrap();
    assert_eq!(h.doc().toolpaths.len(), 1, "replaced, not added");
    assert_eq!(h.doc().toolpaths[&id].name, "Edited");
}

#[test]
fn the_side_tabs_put_drawing_and_modeling_on_design() {
    assert_eq!(
        DESIGN_TABS,
        ["Design", "Sheets", "Layers", "Components", "Library"]
    );
    let h = Harness::new();
    let names: Vec<&str> = h.design_groups("Design").iter().map(|g| g.name).collect();
    assert!(
        names.contains(&"Create Vectors") && names.contains(&"Shape Creation"),
        "{names:?}"
    );
    assert!(h.design_groups("Components").is_empty());
}

#[test]
fn toolpath_forms_open_in_the_toolpaths_page() {
    let mut h = Harness::new();
    h.invoke("profile_toolpath");
    assert_eq!(h.form_page(), Page::Toolpaths);
    assert_eq!(
        h.page_shown(Page::Toolpaths),
        Shown::Flyout,
        "brought out to show its form"
    );
    h.invoke("draw_circle");
    assert_eq!(h.form_page(), Page::Design);
}

#[test]
fn deleting_the_selected_toolpath_is_undoable() {
    let mut h = Harness::new();
    h.rectangle(10.0, 10.0, 50.0, 50.0);
    h.invoke("drilling_toolpath");
    let params = h.form_settings().clone();
    h.run("drilling_toolpath", &params).unwrap();
    let id = *h.doc().toolpaths.keys().next().unwrap();
    h.select_toolpath(id);
    h.invoke("delete_toolpath");
    assert!(h.doc().toolpaths.is_empty());
    h.invoke("undo");
    assert_eq!(h.doc().toolpaths.len(), 1);
}

#[test]
fn tabs_are_placed_and_removed_by_clicking_the_vector() {
    let mut h = Harness::new();
    h.rectangle(10.0, 10.0, 50.0, 50.0);
    h.click(h.at(60.0, 35.0), Modifiers::NONE);
    h.invoke("profile_toolpath");
    h.edit_form(|params| {
        params["tabs"] = "flat".into();
        params["tab"]["spacing"] = "placed".into();
    });
    h.frame(vec![]);
    h.click(h.at(60.0, 30.0), Modifiers::NONE);
    let tabs = |h: &Harness| -> Vec<Point> {
        serde_json::from_value(h.form_settings()["tab_points"].clone()).unwrap_or_default()
    };
    let placed = tabs(&h);
    assert_eq!(placed.len(), 1, "{}", h.status_message());
    assert!(near(placed[0], Point::new(60.0, 30.0)), "{placed:?}");
    assert_eq!(
        h.doc().selection.len(),
        1,
        "the click placed a tab, not a selection"
    );
    // a drag from the tab moves it along the vector
    h.given_options(|o| o.snapping.smart_snapping = false);
    h.drag(h.at(60.0, 30.0), h.at(60.5, 45.0));
    let moved = tabs(&h);
    assert_eq!(moved.len(), 1, "{}", h.status_message());
    assert!(near(moved[0], Point::new(60.0, 45.0)), "{moved:?}");
    let params = h.form_settings().clone();
    h.run("profile_toolpath", &params).unwrap();
    assert_eq!(h.doc().toolpaths.len(), 1);
    h.click(h.at(60.0, 45.0), Modifiers::NONE);
    assert!(tabs(&h).is_empty(), "a second click takes it off");
}

#[test]
fn a_side_bar_entry_opens_its_page() {
    let mut h = Harness::new();
    h.open_info(Info::Help("pocket_toolpath"));
    h.frame(vec![]);
    h.frame(vec![]);
    let label = h.help_title("profile_toolpath");
    let at = *h
        .texts_at(&label)
        .last()
        .expect("Profile listed beside Pocket");
    h.click(at, Modifiers::NONE);
    h.frame(vec![]);
    assert_eq!(h.info(), Some(Info::Help("profile_toolpath")));
}

#[test]
fn set_as_default_in_job_setup_keeps_the_clearing_default() {
    use cut_craft_core::entry::WayBack;
    let mut h = Harness::at_classic_start();
    let mut mine = cut_craft_core::document::Job::default();
    mine.clearing.way_back = WayBack::Alternate;
    h.given_options(|o| o.default_job = Some(mine));
    h.invoke("new");
    h.frame(vec![]);
    assert!(h.form_id().is_some(), "the Job Setup form");
    h.edit_form(|p| p["size"]["width"] = 300.0.into());
    let button = h
        .text_at("Set as Default")
        .expect("a Set as Default button");
    h.click(button, Modifiers::NONE);
    h.frame(vec![]);
    let saved = h.options().default_job.expect("kept in the options");
    assert!((saved.width - 300.0).abs() < 1e-9);
    assert_eq!(
        saved.clearing.way_back,
        WayBack::Alternate,
        "Clearing Settings' own default is left as it was"
    );
}

#[test]
fn sheets_take_selected_vectors_and_toolpaths_from_their_menus() {
    let mut h = Harness::new();
    h.open_document(cut_craft_core::testing::two_sheets());
    let menu = gui::sheet_menu(h.doc(), 1);
    let item = |name: &str| menu.iter().find(|m| m.name == name).unwrap();
    assert!(item("Copy Selected Vectors Here").is_enabled);
    let selected = h.doc().selection.len();
    let action = item("Move Selected Vectors Here").action.clone();
    h.invoke(&action);
    assert_eq!(h.doc().sheets[1].objects.vectors.len(), selected);
    // the active sheet's own menu cannot send to itself
    let own = gui::sheet_menu(h.doc(), 0);
    assert!(
        !own.iter()
            .find(|m| m.name == "Copy Selected Vectors Here")
            .unwrap()
            .is_enabled
    );
    // a toolpath's menu offers the other sheet
    let mut doc = cut_craft_core::testing::with_toolpath();
    doc.sheets = cut_craft_core::testing::two_sheets().sheets;
    let (&id, t) = doc.toolpaths.iter().next().unwrap();
    let menu = gui::toolpath_menu(&doc, id, t);
    let copy = menu.iter().find(|m| m.name == "Copy to Sheet").unwrap();
    let names: Vec<&str> = copy.children.iter().map(|m| m.name.as_str()).collect();
    assert_eq!(names, ["Sheet 2"]);
}

#[test]
fn a_texture_toolpaths_texture_saves_to_a_file_and_loads_into_another_form() {
    let dir = std::env::temp_dir().join(format!("cut-craft-texture-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("scallops.vtexture");
    let mut h = Harness::new();
    assert_eq!(
        h.save_texture(&path).unwrap_err(),
        "no Texture Toolpath form is open"
    );
    h.invoke("texture_toolpath");
    h.edit_form(|p| p["stepover"] = Value::from(7.5));
    h.save_texture(&path).unwrap();
    let text = std::fs::read_to_string(&path).unwrap();
    assert!(text.contains("\r\nStepover: 7.50000 0\r\n"), "{text}");
    h.close_form();
    h.invoke("texture_toolpath");
    h.edit_form(|p| p["name"] = Value::from("Lid"));
    h.load_texture(&path).unwrap();
    assert_eq!(h.form_value("stepover"), 7.5);
    assert_eq!(h.form_value("name"), "Lid", "the form keeps its own name");
    assert!(h.status_message().contains("Loaded texture"));
    std::fs::remove_dir_all(dir).unwrap();
}

/// Load Texture replaces the texture's own settings; what the form holds besides them, such as
/// its Vector Selector, stays.
#[test]
fn load_texture_keeps_the_forms_vector_selector() {
    let dir = fresh_dir("texture-selector");
    let path = dir.join("scallops.vtexture");
    let mut h = Harness::new();
    h.invoke("texture_toolpath");
    h.save_texture(&path).unwrap();
    let selector = serde_json::json!({ "open": false, "closed": true });
    h.edit_form(|p| p["selector"] = selector.clone());
    h.load_texture(&path).unwrap();
    std::fs::remove_dir_all(dir).unwrap();
    assert_eq!(h.form_value("selector"), &selector);
}

#[test]
fn a_profile_never_cuts_a_selected_construction_line() {
    let mut h = Harness::new();
    h.rectangle(10.0, 10.0, 50.0, 30.0);
    h.invoke("make_construction_line");
    let settings = h.default_settings("profile_toolpath");
    let err = h.run("profile_toolpath", &settings).unwrap_err();
    assert!(err.contains("construction lines"), "{err}");
    assert!(h.doc().toolpaths.is_empty());
}

#[test]
fn v_carve_fill_has_no_edit_passes_for_a_depth_it_does_not_cut() {
    // Fill works its depth out from each shape's width: the hidden Groove depth is not it
    let mut h = Harness::new();
    h.rectangle(10.0, 10.0, 50.0, 50.0);
    h.invoke("vcarve_engrave_toolpath");
    for _ in 0..5 {
        h.frame(vec![]);
    }
    assert_eq!(h.form_settings()["place"], "fill");
    assert!(
        h.text_at("Edit Passes…").is_none(),
        "no passes window for Fill"
    );
    h.edit_form(|p| p["place"] = "inside".into());
    for _ in 0..3 {
        h.frame(vec![]);
    }
    assert!(
        h.text_at("Edit Passes…").is_some(),
        "a groove has its depth"
    );
}

#[test]
fn fluting_offers_edit_passes_for_its_flute_depth() {
    // fluting steps down a pass at a time, so its passes are set as any other cut's
    let mut h = Harness::new();
    h.rectangle(10.0, 10.0, 50.0, 50.0);
    h.invoke("fluting_toolpath");
    for _ in 0..5 {
        h.frame(vec![]);
    }
    assert!(h.text_at("Edit Passes…").is_some());
    let params = h.form_settings();
    assert_eq!(passes::cut_depth(params), params["flute_depth"].as_f64());
}

#[test]
fn edit_passes_shows_each_pass_and_a_click_adds_one() {
    let mut h = Harness::new();
    h.rectangle(10.0, 10.0, 50.0, 50.0);
    h.invoke("profile_toolpath");
    h.edit_form(|p| p["depth"] = serde_json::json!(6.0));
    h.edit_form(|p| p["tool"]["pass_depth"] = serde_json::json!(3.0));
    h.frame(vec![]);
    h.frame(vec![]);
    let passes = |h: &Harness| {
        let tool = &h.form_settings()["tool"];
        serde_json::from_value::<Tool>(tool.clone())
            .unwrap()
            .pass_depths(6.0)
    };
    assert_eq!(passes(&h), [-3.0, -6.0]);
    // the form's panel settles to its width
    for _ in 0..5 {
        h.frame(vec![]);
    }
    h.click(
        h.text_at("Edit Passes…").expect("the button"),
        Modifiers::NONE,
    );
    h.frame(vec![]);
    assert!(
        h.text_at("Cut Depth 6 mm in 2 passes").is_some(),
        "the window"
    );
    // the picture labels each pass by its depth; a click high in the material adds one
    let top = h.text_at("Top").expect("the picture");
    let half = h.text_at("3 mm").expect("the first pass");
    h.click(
        egui::pos2(top.x - 40.0, f32::midpoint(top.y, half.y)),
        Modifiers::NONE,
    );
    h.frame(vec![]);
    let set = passes(&h);
    assert_eq!(set.len(), 3, "{set:?}");
    assert!(
        set[0] > -3.0 && set[0] < 0.0 && (set[1] + 3.0).abs() < 1e-9,
        "{set:?}"
    );
    // a click on a pass takes it away
    h.click(egui::pos2(top.x - 40.0, half.y), Modifiers::NONE);
    h.frame(vec![]);
    assert_eq!(passes(&h).len(), 2);
    // a thin last pass
    h.click(h.text_at("Last Pass Thickness").unwrap(), Modifiers::NONE);
    h.frame(vec![]);
    let set = passes(&h);
    assert!(
        (set[set.len() - 2] + 5.4).abs() < 1e-9,
        "a 0.6 mm last pass: {set:?}"
    );
}

#[test]
fn edit_tabs_adds_tabs_by_number_off_the_corners_and_sets_2d_or_3d() {
    let mut h = Harness::new();
    h.rectangle(10.0, 10.0, 50.0, 50.0);
    h.click(h.at(60.0, 35.0), Modifiers::NONE);
    h.invoke("profile_toolpath");
    h.frame(vec![]);
    h.frame(vec![]);
    // down the form to its Tabs, the button at their foot (the form is on the Toolpaths page,
    // to the right; the Design page's tip box names the tool in hand too)
    let over = h
        .texts_at("Profile Toolpath")
        .into_iter()
        .max_by(|a, b| a.x.total_cmp(&b.x))
        .expect("the form");
    for _ in 0..30 {
        if h.text_at("Edit Tabs…").is_some_and(|b| b.y < 700.0) {
            break;
        }
        h.frame(vec![Event::PointerMoved(over + egui::vec2(0.0, 200.0))]);
        h.frame(vec![Event::MouseWheel {
            unit: egui::MouseWheelUnit::Point,
            delta: egui::vec2(0.0, -60.0),
            phase: egui::TouchPhase::Move,
            source: egui::MouseWheelSource::Wheel,
            modifiers: Modifiers::NONE,
        }]);
    }
    // the scroll eases to a stop
    for _ in 0..30 {
        h.frame(vec![]);
    }
    h.click(
        h.text_at("Edit Tabs…").expect("the button"),
        Modifiers::NONE,
    );
    h.frame(vec![]);
    h.frame(vec![]);
    // automatic to start with, as the form says; by number here
    h.click(h.text_at("Number").expect("the window"), Modifiers::NONE);
    h.click(h.text_at("Not on corners").unwrap(), Modifiers::NONE);
    h.click(h.text_at("Add Tabs").unwrap(), Modifiers::NONE);
    h.frame(vec![]);
    let params = h.form_settings().clone();
    let tabs: Vec<Point> = serde_json::from_value(params["tab_points"].clone()).unwrap();
    assert_eq!(
        tabs.len(),
        4,
        "the form's number, on the selected rectangle"
    );
    let corners = [(10.0, 10.0), (60.0, 10.0), (60.0, 60.0), (10.0, 60.0)];
    for t in &tabs {
        assert!(
            corners
                .iter()
                .all(|&(x, y)| t.distance(Point::new(x, y)) > 5.0),
            "{tabs:?}"
        );
    }
    assert_eq!(params["tabs"], "flat");
    assert_eq!(params["tab"]["spacing"], "placed", "to move by hand after");
    assert!(h.text_at("4 tabs placed").is_some());
    h.click(h.text_at("3D Tabs").unwrap(), Modifiers::NONE);
    assert_eq!(h.form_settings()["tabs"], "triangular");
    h.click(h.text_at("Delete All Tabs").unwrap(), Modifiers::NONE);
    assert_eq!(h.form_settings()["tab_points"], serde_json::json!([]));
}

#[test]
fn the_cutting_window_sets_how_a_pocket_cuts_and_draws_it() {
    let mut h = Harness::new();
    h.rectangle(10.0, 10.0, 50.0, 50.0);
    h.invoke("pocket_toolpath");
    h.frame(vec![]);
    h.frame(vec![]);
    // down the form to its Clear Pocket, the picture and Cutting… at its foot; the form's
    // heading, not the Design page's tip naming the tool
    let over = h.form_heading("Pocket Toolpath");
    for _ in 0..30 {
        if h.text_at("Cutting…").is_some_and(|b| b.y < 700.0) {
            break;
        }
        h.frame(vec![Event::PointerMoved(over + egui::vec2(0.0, 200.0))]);
        h.frame(vec![Event::MouseWheel {
            unit: egui::MouseWheelUnit::Point,
            delta: egui::vec2(0.0, -60.0),
            phase: egui::TouchPhase::Move,
            source: egui::MouseWheelSource::Wheel,
            modifiers: Modifiers::NONE,
        }]);
    }
    for _ in 0..30 {
        h.frame(vec![]);
    }
    assert!(
        h.text_at("Pass 1: climb, anticlockwise round the pocket")
            .is_some(),
        "the form says what the choices make"
    );
    h.click(h.text_at("Cutting…").expect("the button"), Modifiers::NONE);
    h.frame(vec![]);
    h.frame(vec![]);
    let params = |h: &Harness| h.form_settings().clone();
    for (choice, key, value) in [
        ("Both directions", "ways", "both"),
        ("From the outline in", "rings", "outline_in"),
    ] {
        h.click(h.text_at(choice).expect(choice), Modifiers::NONE);
        h.frame(vec![]);
        assert_eq!(params(&h)[key], value, "{choice}");
    }
    // a pocket clears with a light bite by Adaptive, not in loops
    assert!(h.text_at("Trochoidal").is_none());
    h.edit_form(|p| p["strategy"] = "adaptive".into());
    // adaptive: the job's Clearing Settings, a button away, set its bite and the loops
    for _ in 0..3 {
        h.frame(vec![]);
    }
    h.click(
        h.text_at("Clearing Settings…").expect("the button"),
        Modifiers::NONE,
    );
    for _ in 0..3 {
        h.frame(vec![]);
    }
    assert!(h.clearing_window_open());
    h.click(h.text_at("Clockwise").expect("the loops"), Modifiers::NONE);
    for _ in 0..3 {
        h.frame(vec![]);
    }
    assert_eq!(
        h.doc().job.clearing.loops.turn,
        cut_craft_core::entry::Turn::Clockwise
    );
}

#[test]
fn toolpath_forms_save_their_settings_and_load_them_back() {
    let mut h = Harness::new();
    let dir = std::env::temp_dir().join(format!("cut-craft-form-settings-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("profile.ccsettings");
    h.invoke("profile_toolpath");
    let key = h
        .form_settings()
        .as_object()
        .unwrap()
        .iter()
        .find(|(_, v)| v.is_f64())
        .map(|(k, _)| k.clone())
        .expect("a number setting");
    h.edit_form(|p| p[&key] = serde_json::json!(7.25));
    h.save_form_settings(&path);
    assert!(h.status_message().starts_with("Saved the settings"));

    h.edit_form(|p| p[&key] = serde_json::json!(1.0));
    h.load_form_settings(&path);
    assert_eq!(h.form_value(&key), &serde_json::json!(7.25));
    assert!(h.form_dirty(), "the loaded settings are yet to be applied");

    h.close_form();
    h.invoke("pocket_toolpath");
    let before = h.form_settings().clone();
    h.load_form_settings(&path);
    assert_eq!(h.form_settings(), &before, "left alone");
    assert!(
        h.status_message().starts_with("Could not load"),
        "{}",
        h.status_message()
    );
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn import_toolpaths_opens_its_form_on_the_file_and_adds_its_toolpaths() {
    let mut h = Harness::new();
    let dir = std::env::temp_dir().join(format!("cut-craft-import-app-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let file = dir.join("nameplate.nc");
    std::fs::write(&file, "G21 G90\nG0 X10 Y10 Z5\nG1 Z-1 F600\nX40\n").unwrap();
    h.import_toolpaths_from(&file);
    assert_eq!(h.form_id(), Some("import_toolpaths"), "its form");
    assert_eq!(h.form_value("name"), "nameplate");
    let params = h.form_settings().clone();
    h.run("import_toolpaths", &params).unwrap();
    let toolpath = h.doc().toolpaths.values().next().expect("added");
    assert_eq!(toolpath.name, "nameplate");
    assert_eq!(toolpath.tool.feed_rate, 600.0, "the file's feed rate");
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn the_profile_form_shows_which_side_it_cuts_just_under_the_depth() {
    let mut h = profile_form();
    let y = |h: &Harness, text: &str| h.text_at(text).unwrap_or_else(|| panic!("{text}")).y;
    let side = y(&h, crate::form::WHICH_SIDE);
    assert!(y(&h, "Cut Depth") < side, "under the depth");
    assert!(y(&h, "Bit") < y(&h, "Cut Depth"), "the bit first");
    // the cutting details further down, scrolled out of sight in a short window
    assert!(
        h.text_at("Machine Vectors…").is_none_or(|p| side < p.y),
        "before the cutting details"
    );
    for choice in ["Outside", "Inside", "On the Line"] {
        assert!(
            y(&h, choice) > side,
            "{choice} is a picture under the heading"
        );
    }
    assert!(h.text_at(&super::cutting::side_said("outside")).is_some());
    // the picture over Inside picks it, and the sentence says what that makes
    let inside = h.text_at("Inside").unwrap();
    h.click(inside - egui::vec2(0.0, 25.0), Modifiers::NONE);
    h.frame(vec![]);
    assert_eq!(h.form_settings()["side"], "inside");
    assert!(h.text_at(&super::cutting::side_said("inside")).is_some());
}

#[test]
fn a_cut_right_through_with_no_tabs_is_warned_of_with_add_tabs() {
    let mut h = profile_form();
    let thickness = h.doc().job.thickness;
    assert!(
        h.text_at("Cut right through").is_none(),
        "no tick: a cut past the material is warned of on Calculate"
    );
    assert!(
        h.text_at(&super::tabs::loose()).is_none(),
        "6 mm deep holds the part"
    );
    h.edit_form(|p| p["depth"] = thickness.into());
    h.frame(vec![]);
    h.frame(vec![]);
    let params = |h: &Harness| h.form_settings().clone();
    let over = h.form_heading("Profile Toolpath") + egui::vec2(0.0, 200.0);
    let add = scroll_above_foot(&mut h, over, "Add Tabs");
    assert!(
        h.text_at(&super::tabs::loose()).is_some(),
        "right through, no tabs"
    );
    // made like that, the status bar warns too
    h.run("profile_toolpath", &params(&h)).unwrap();
    assert!(
        h.status_message().contains("comes loose"),
        "{}",
        h.status_message()
    );
    // Add Tabs puts tabs on, and the warning goes
    h.click(add, Modifiers::NONE);
    h.frame(vec![]);
    assert_eq!(h.form_settings()["tabs"], "flat");
    assert!(h.text_at(&super::tabs::loose()).is_none());
}

#[test]
fn add_tabs_with_nothing_selected_and_delete_all_tabs_leave_the_tab_settings_alone() {
    let mut h = profile_form();
    h.select([]);
    let params = |h: &Harness| h.form_settings().clone();
    assert_eq!(params(&h)["tabs"], "none");
    h.ctx
        .data_mut(|d| d.insert_temp(egui::Id::unique("edit_tabs"), true));
    h.frame(vec![]);
    h.frame(vec![]);
    h.click(h.text_at("Add Tabs").expect("the window"), Modifiers::NONE);
    h.frame(vec![]);
    assert_eq!(params(&h)["tabs"], "none", "nothing added: tabs stay off");
    assert_eq!(params(&h)["tab"]["spacing"], "automatic");
    assert!(
        h.status_message().contains("select the vectors"),
        "{}",
        h.status_message()
    );
    // tabs placed by hand, then turned off: deleting them leaves them off
    h.edit_form(|p| {
        p["tab"]["spacing"] = "placed".into();
        p["tab_points"] = serde_json::json!([[10.0, 30.0]]);
    });
    h.frame(vec![]);
    h.click(h.text_at("Delete All Tabs").unwrap(), Modifiers::NONE);
    h.frame(vec![]);
    assert_eq!(params(&h)["tab_points"], serde_json::json!([]));
    assert_eq!(params(&h)["tabs"], "none", "still off");
    assert_eq!(params(&h)["tab"]["spacing"], "placed");
    assert!(
        h.status_message().contains("click the vectors"),
        "says how to place more: {}",
        h.status_message()
    );
}

/// Wheels the form under `over` down until `text` sits clear of the buttons pinned at its
/// foot, and gives where it is.
fn scroll_above_foot(h: &mut Harness, over: Pos2, text: &str) -> Pos2 {
    let apply = h.text_at("Close").expect("the form's buttons");
    for _ in 0..40 {
        if let Some(at) = h.text_at(text).filter(|at| at.y < apply.y - 80.0) {
            return at;
        }
        h.frame(vec![
            Event::PointerMoved(over),
            Event::MouseWheel {
                unit: egui::MouseWheelUnit::Point,
                delta: vec2(0.0, -60.0),
                phase: egui::TouchPhase::Move,
                source: egui::MouseWheelSource::Wheel,
                modifiers: Modifiers::NONE,
            },
        ]);
        h.frame(vec![]);
    }
    panic!("{text} never scrolled into sight");
}

#[test]
fn the_tabs_read_in_one_line_on_the_form() {
    use cut_craft_core::entry::{TabPlacement, TabSettings, TabSpacing, Tabs};
    let tab = TabSettings::default();
    let mm = Units::Mm;
    assert_eq!(tabs::summary(Tabs::None, tab, true, 0, mm), None);
    assert_eq!(
        tabs::summary(Tabs::Flat, tab, true, 0, mm).unwrap(),
        "12.7 mm long, 6.35 mm thick (from the bit), about one every 150 mm, 3 or more a shape (fewer if small)"
    );
    let by_number = TabSettings {
        spacing: TabSpacing::ConstantNumber,
        placement: TabPlacement::AvoidCorners,
        ..tab
    };
    assert_eq!(
        tabs::summary(Tabs::Triangular, by_number, false, 0, mm).unwrap(),
        "12.7 mm long, 6.35 mm thick, 4 a shape, off the corners"
    );
    let placed = TabSettings {
        spacing: TabSpacing::Placed,
        ..tab
    };
    assert!(
        tabs::summary(Tabs::Flat, placed, false, 5, mm)
            .unwrap()
            .ends_with("5 placed by hand")
    );
}

#[test]
fn the_profile_form_keeps_tab_and_tool_settings_in_their_windows() {
    let mut h = Harness::new();
    h.rectangle(10.0, 10.0, 50.0, 50.0);
    h.click(h.at(60.0, 35.0), Modifiers::NONE);
    h.invoke("profile_toolpath");
    h.pin(Page::Toolpaths);
    h.choose_tool("3 mm End Mill");
    h.frame(vec![]);
    h.frame(vec![]);
    // the form: the tool's name and one line, the tab choice, and the buttons
    assert!(h.text_at("3 mm End Mill").is_some());
    assert!(h.text_at("Select…").is_some());
    for hidden in [
        "Clearing stepover",
        "Feed rate",
        "Tab Size",
        "Minimum Number",
        "Constant distance",
    ] {
        assert!(h.text_at(hidden).is_none(), "{hidden} is in a window");
    }
    // Select… opens the Bit Library to pick in, on the form's tool
    h.click(h.text_at("Select…").unwrap(), Modifiers::NONE);
    h.frame(vec![]);
    h.frame(vec![]);
    h.in_bit_library(|manager| {
        assert!(manager.selecting, "Select Bit");
        assert_eq!(manager.shown_tool(), Some("3 mm End Mill"));
        let other = manager
            .draft
            .tools
            .iter()
            .position(|t| t.name == "6 mm Ball Nose")
            .unwrap();
        manager.pick = Some(cut_craft_tool_database::Pick::Tool(other));
    });
    h.frame(vec![]);
    h.frame(vec![]);
    assert!(h.text_at("Select Bit").is_some());
    let select = *h.texts_at("Select").last().expect("Select, in place of OK");
    h.click(select, Modifiers::NONE);
    h.frame(vec![]);
    assert!(h.bit_library().is_none(), "closed");
    let ball = h.tool_database().get("6 mm Ball Nose").unwrap();
    assert_eq!(h.form_value("tool")["name"], "6 mm Ball Nose");
    assert_eq!(
        h.form_value("tool")["feed_rate"],
        serde_json::json!(ball.feed_rate),
        "its own feeds and speeds come with it"
    );
    // 2D tabs: the form says what they will be
    h.edit_form(|p| p["tabs"] = "flat".into());
    h.frame(vec![]);
    h.frame(vec![]);
    assert!(
        h.form_settings()["tab"]["spacing"] == "automatic",
        "kept with the toolpath"
    );
    let over = h.form_heading("Profile Toolpath") + egui::vec2(0.0, 200.0);
    scroll_above_foot(&mut h, over, "Edit Tabs…");
    let summary = h.shapes.iter().any(|clipped| {
        matches!(&clipped.shape, egui::Shape::Text(t)
            if t.galley.text().contains("about one every 150 mm"))
    });
    assert!(summary, "the tabs in one line");
}

/// Matthew's 3D finishing toolpath (6.8 million tiny cuts) crashed the app as it was shown:
/// a line for each cut was more than the graphics card's 2 GB buffer takes. A frame showing
/// a toolpath that dense stays small.
#[test]
#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_precision_loss,
    reason = "small counts"
)]
fn a_toolpath_of_millions_of_cuts_is_drawn_in_a_frame_the_graphics_card_takes() {
    use cut_craft_core::glam::DVec3;
    use cut_craft_core::toolpath::{Move, MoveKind};
    let mut h = Harness::new();
    h.given_2d_previews(true);
    let (width, height) = (h.doc().job.width, h.doc().job.height);
    let (rows, step) = (400_usize, 0.02);
    let along = (width / step) as usize;
    let mut moves = Vec::with_capacity(rows * along);
    for row in 0..rows {
        let across = height * row as f64 / rows as f64;
        for step_along in 0..along {
            let at = if row % 2 == 0 {
                step_along
            } else {
                along - 1 - step_along
            };
            // over a bumpy model, as a finishing pass is
            let x = at as f64 * step;
            let depth = -1.0 - (x * 0.7).sin() * (across * 0.3).cos();
            moves.push(Move {
                kind: MoveKind::Cut,
                to: DVec3::new(x, across, depth),
            });
        }
    }
    assert!(moves.len() > 2_000_000, "{} cuts", moves.len());
    let mut c = Change::builder(h.doc());
    c.add_toolpath(cut_craft_core::toolpath::Toolpath {
        name: "3D Finishing".into(),
        strategy: "3d_finishing_toolpath".into(),
        params: serde_json::Value::Null,
        tool: Tool::default(),
        vectors: vec![],
        moves,
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
    let shapes = std::mem::take(&mut h.shapes);
    let vertices: usize = h
        .ctx
        .tessellate(shapes, 1.0)
        .iter()
        .map(|p| match &p.primitive {
            egui::epaint::Primitive::Mesh(mesh) => mesh.vertices.len(),
            egui::epaint::Primitive::Callback(_) => 0,
        })
        .sum();
    // egui's vertices are 20 bytes: 100 MB at most, the card's limit is 2 GB
    assert!(vertices < 5_000_000, "{vertices} vertices");
}

#[test]
fn the_status_bar_and_a_pocket_form_count_the_selected_vectors() {
    let mut h = Harness::new();
    h.rectangle(10.0, 10.0, 20.0, 10.0);
    h.rectangle(40.0, 10.0, 20.0, 10.0);
    let line = serde_json::json!({ "points": [
        { "x": 10.0, "y": 40.0 }, { "x": 60.0, "y": 40.0 }
    ], "closed": false });
    h.run("draw_polyline", &line).unwrap();
    h.close_form();
    h.invoke("select_all_vectors");
    h.frame(vec![]);
    let count = "3 vectors selected: 2 closed, 1 open";
    let at = h.text_at(count).expect("the status bar counts them");
    assert!(at.y > 700.0, "in the status bar: {at:?}");
    h.invoke("pocket_toolpath");
    // the Toolpaths page flies out unseen on its first frame
    h.frame(vec![]);
    h.frame(vec![]);
    let on_form = h
        .shapes
        .iter()
        .filter(
            |clipped| matches!(&clipped.shape, egui::Shape::Text(t) if t.galley.text() == count),
        )
        .count();
    assert_eq!(on_form, 2, "on the form too, as well as the status bar");
    assert!(
        h.text_at("1 open vector will be left out: Pocket Toolpath only cuts closed shapes, and it is not closed")
            .is_some(),
        "the form says what Pocket will leave out"
    );
}

#[test]
fn an_on_the_line_profile_leaves_no_open_vector_out() {
    assert!(form_panel::closed_only_toolpath(
        "pocket_toolpath",
        &serde_json::json!({})
    ));
    assert!(form_panel::closed_only_toolpath(
        "profile_toolpath",
        &serde_json::json!({ "side": "outside" })
    ));
    assert!(!form_panel::closed_only_toolpath(
        "profile_toolpath",
        &serde_json::json!({ "side": "on" })
    ));
    assert!(!form_panel::closed_only_toolpath(
        "quick_engrave_toolpath",
        &serde_json::json!({})
    ));
    // a texture fills closed vectors, but follows any vector used as the pattern
    assert!(form_panel::closed_only_toolpath(
        "texture_toolpath",
        &serde_json::json!({ "use_selected_vectors_as_pattern": false })
    ));
    assert!(!form_panel::closed_only_toolpath(
        "texture_toolpath",
        &serde_json::json!({ "use_selected_vectors_as_pattern": true })
    ));
}

#[test]
fn picked_toolpaths_group_together_and_the_group_moves_as_one() {
    let mut h = named_toolpaths();
    // Pocket, then Cut Out with Ctrl held: both picked
    h.click(h.text_at("Pocket").unwrap(), Modifiers::NONE);
    h.click(h.text_at("Cut Out").unwrap(), Modifiers::COMMAND);
    assert_eq!(h.picked_toolpaths(), [EntityId(1), EntityId(3)]);
    h.click(h.text_at("Group").unwrap(), Modifiers::NONE);
    h.frame(vec![]);
    // grouped toolpaths cut one after another, where the first of them did
    assert_eq!(toolpath_names(&h), ["Pocket", "Cut Out", "Drill"]);
    let groups: Vec<Option<&str>> = h
        .doc()
        .toolpaths
        .values()
        .map(|t| t.group.as_deref())
        .collect();
    assert_eq!(groups, [Some("Group 1"), Some("Group 1"), None]);
    assert_eq!(
        h.picked_toolpaths(),
        [EntityId(1), EntityId(2)],
        "still picked in their new places"
    );
    // one step undoes it
    h.invoke("undo");
    assert_eq!(toolpath_names(&h), ["Pocket", "Drill", "Cut Out"]);
    h.invoke("redo");
    h.frame(vec![]);
    // the group's name picks it; Move Down takes both its toolpaths past Drill
    h.click(h.text_at("Group 1").unwrap(), Modifiers::NONE);
    assert_eq!(h.selected_toolpath_group(), Some("Group 1"));
    h.click(h.text_at("Move Down").unwrap(), Modifiers::NONE);
    assert_eq!(toolpath_names(&h), ["Drill", "Pocket", "Cut Out"]);
    // Ungroup on the picked group takes it apart, keeping its toolpaths
    h.click(h.text_at("Ungroup").unwrap(), Modifiers::NONE);
    assert!(h.doc().toolpaths.values().all(|t| t.group.is_none()));
    assert_eq!(toolpath_names(&h), ["Drill", "Pocket", "Cut Out"]);
}

#[test]
fn the_pockets_clearing_settings_have_their_picture() {
    fn find<'a>(fields: &'a [form::Field], key: &str) -> Option<&'a form::Field> {
        fields.iter().find_map(|f| {
            if f.key == key {
                return Some(f);
            }
            match &f.kind {
                form::Kind::Group(inner) => find(inner, key),
                _ => None,
            }
        })
    }
    let pocket = commands::registry()
        .get("pocket_toolpath")
        .expect("pocket")
        .schema();
    let fields = form::fields(&pocket);
    let edge = find(&fields, "edge_allowance").expect("Leave for Edge Pass");
    assert_eq!(edge.diagram.as_deref(), Some("diagrams/edge_pass.png"));
    assert_eq!(edge.dim.as_deref(), Some("A"));
}

#[test]
fn a_drill_picked_for_a_profile_is_warned_of_in_red_at_the_foot_of_its_form() {
    let mut h = Harness::new();
    h.rectangle(10.0, 10.0, 50.0, 50.0);
    let warning = |name: &str| {
        format!(
            "{name} is a drill, which cuts only straight down. Moved sideways it can snap or \
             tear the work: pick an end mill for this toolpath, or use the drill in a \
             Drilling toolpath"
        )
    };
    // a drill brought in from a vendor's tool library
    h.given_bits(|bits| {
        let mut drill = bits.tools[0].clone();
        drill.name = "Drill 6mm".into();
        drill.drill_only = true;
        bits.tools.push(drill);
    });
    for (id, warned) in [("profile_toolpath", true), ("drilling_toolpath", false)] {
        h.invoke(id);
        h.choose_tool("Drill 6mm");
        h.frame(vec![]);
        h.frame(vec![]);
        assert_eq!(h.text_at(&warning("Drill 6mm")).is_some(), warned, "{id}");
        h.close_form();
    }
    // an end mill: nothing said
    h.invoke("profile_toolpath");
    h.frame(vec![]);
    let name = h.form_settings()["tool"]["name"]
        .as_str()
        .unwrap()
        .to_owned();
    assert!(h.text_at(&warning(&name)).is_none());
}

#[test]
fn on_a_mac_quick_engrave_has_no_cmd_q() {
    let h = Harness::new();
    h.ctx.set_os(egui::os::OperatingSystem::Mac);
    let quit = shortcut::parse_trigger("Ctrl+Q").unwrap();
    assert!(
        !h.shortcuts_in_use().iter().any(|(_, t)| *t == quit),
        "⌘Q is the Mac's Quit"
    );
    h.ctx.set_os(egui::os::OperatingSystem::Windows);
    assert!(
        h.shortcuts_in_use()
            .contains(&("quick_engrave_toolpath".to_owned(), quit)),
        "Ctrl+Q elsewhere"
    );
}

#[test]
fn a_laser_reads_as_its_kerf_and_speed_in_the_jobs_units() {
    use cut_craft_core::document::Units;
    assert_eq!(
        tool_line(&Tool::laser(), Some(2.0), Units::Mm),
        "Laser, 0.15 mm kerf, 1500 mm/min"
    );
    assert_eq!(
        tool_line(&Tool::laser(), None, Units::Inch),
        "Laser, 0.006 in kerf, 59.055 in/min"
    );
}

#[test]
fn the_2d_view_draws_vectors_and_toolpaths_in_the_colours_picked() {
    // a colour-blind user could not tell vectors from toolpaths: Options ▸ Appearance
    let mut h = Harness::new();
    h.rectangle(10.0, 10.0, 50.0, 50.0);
    let params = h.opening_settings("profile_toolpath");
    h.run("profile_toolpath", &params).unwrap();
    h.given_no_toolpath_picked();
    h.select([]);
    h.show_view(View::TwoD);
    let (vector, toolpath) = ([1, 2, 3], [4, 5, 6]);
    h.given_options(|o| {
        let look = &mut o.appearance;
        look.line_colors = crate::options::LinePalette::MyOwn;
        for theme in [&mut look.light_2d, &mut look.dark_2d] {
            theme.vector_color = vector;
            theme.toolpath_color = toolpath;
        }
    });
    h.frame(vec![]);
    let drawn_in = |h: &Harness, [r, g, b]: [u8; 3]| {
        let colour = egui::epaint::ColorMode::Solid(egui::Color32::from_rgb(r, g, b));
        h.shapes
            .iter()
            .filter(|c| matches!(&c.shape, egui::Shape::Path(p) if p.stroke.color == colour))
            .count()
    };
    assert!(
        drawn_in(&h, vector) > 0,
        "the rectangle in the vector colour"
    );
    assert!(
        drawn_in(&h, toolpath) > 0,
        "its profile in the toolpath colour"
    );
}

#[test]
fn a_new_vcarve_carves_as_the_options_say() {
    use cmd_vcarve_engrave_toolpath::{Direction, Style};
    let mut h = Harness::new();
    assert_eq!(h.options().toolpaths.vcarve_style, Style::StayDown);
    h.invoke("vcarve_engrave_toolpath");
    assert_eq!(h.form_value("style"), "stay_down");
    h.close_form();
    h.given_options(|o| {
        o.toolpaths.vcarve_style = Style::Lift;
        o.toolpaths.vcarve_direction = Direction::DepthOut;
    });
    h.invoke("vcarve_engrave_toolpath");
    assert_eq!(h.form_value("style"), "lift");
    assert_eq!(h.form_value("direction"), "depth_out");
}
