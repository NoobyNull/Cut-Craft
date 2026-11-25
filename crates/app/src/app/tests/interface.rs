//! The window itself: menus, pages, side panels, tool buttons, options and layout.

use super::*;
use crate::options::WindowLayout;

#[test]
fn the_window_layout_menu_picks_and_ticks_a_layout() {
    let mut h = Harness::new();
    assert_eq!(h.app.layout_checked("layout_classic"), Some(true));
    assert_eq!(h.app.layout_checked("zoom_box"), None);
    assert!(h.app.available("layout_rail"));
    h.app.invoke(&h.ctx, "layout_rail");
    assert_eq!(h.app.options.general.layout, WindowLayout::Rail);
    assert_eq!(h.app.layout_checked("layout_rail"), Some(true));
    assert_eq!(h.app.layout_checked("layout_classic"), Some(false));
    h.frame(vec![]);
    assert!(h.text_at("Design").is_none(), "no Design page or side tab");
    assert!(h.text_at("Sheets").is_some(), "the browser's tabs");
}

#[test]
fn find_tool_lists_the_tools_whose_names_match() {
    let mut h = Harness::new();
    let names = |h: &Harness, text: &str| {
        h.app
            .search_matches(text)
            .into_iter()
            .map(|(_, label)| label)
            .collect::<Vec<_>>()
    };
    assert_eq!(
        names(&h, "draw circ"),
        vec!["Draw Circle", "Draw Circle from Edge"]
    );
    assert!(names(&h, "POCKET").contains(&"Pocket Toolpath".to_owned()));
    assert!(
        names(&h, "paste in place").is_empty(),
        "nothing to paste yet"
    );
    h.app.invoke(&h.ctx, "find_tool");
    assert_eq!(h.app.search.as_deref(), Some(""));
    // a new window is laid out unseen on its first frame
    h.frame(vec![]);
    h.frame(vec![]);
    assert!(h.text_at("Find Tool").is_some());
    h.frame(vec![Event::Key {
        key: Key::Escape,
        physical_key: None,
        pressed: true,
        repeat: false,
        modifiers: Modifiers::NONE,
    }]);
    assert!(h.app.search.is_none(), "Escape closes it");
}

#[test]
fn the_ribbons_favourites_tab_shows_each_pages_favourites() {
    let mut h = Harness::new();
    h.app.options.general.layout = crate::options::WindowLayout::Ribbon;
    h.app.ribbon_tab = RIBBON_TABS.iter().position(|&t| t == "Favourites").unwrap();
    h.frame(vec![]);
    assert!(
        h.text_at("No favourites yet: right-click a tool ▸ Add to Favourites")
            .is_some()
    );
    h.app.options.favourites.design = vec!["draw_circle".into()];
    h.app.options.favourites.toolpaths = vec!["pocket_toolpath".into(), "gone".into()];
    h.frame(vec![]);
    assert!(h.text_at("Design Favourites").is_some());
    assert!(h.text_at("Toolpath Favourites").is_some());
    assert_eq!(
        h.app
            .tool_model()
            .favourites(Page::Toolpaths, &h.app.options.favourites),
        ["pocket_toolpath"]
    );
}

#[test]
fn measuring_reports_distances_and_spans() {
    use cut_craft_core::document::Units;
    let m = measured(Point::ZERO, Point::new(3.0, 4.0), Units::Mm);
    assert_eq!(m, "Distance 5.000 mm, angle 53.13°, X 3.000 mm, Y 4.000 mm");
    let square = cut_craft_core::testing::square(0.0, 0.0, 10.0);
    let i = inspected(&square, Point::new(5.0, 0.0), 0.5, Units::Mm);
    assert_eq!(
        i,
        "Line, length 10.000 mm; vector perimeter 40.000 mm, area 100.000 mm²"
    );
    // the form's rows: the points, and the span's ends and the vector's span count
    let rows = measured_rows(Point::ZERO, Point::new(3.0, 4.0), Units::Mm);
    assert!(rows.contains(&("Second point".into(), "3.000 mm, 4.000 mm".into())));
    let rows = inspected_rows(&square, Point::new(5.0, 0.0), 0.5, Units::Mm);
    assert!(
        rows.contains(&("End".into(), "10.000 mm, 0.000 mm".into())),
        "{rows:?}"
    );
    assert!(rows.contains(&("Number of spans".into(), "4".into())));
    let circle = Path::circle(Point::new(5.0, 5.0), 2.0);
    let rows = inspected_rows(&circle, Point::new(7.0, 5.0), 0.5, Units::Mm);
    assert!(
        rows.contains(&("Radius".into(), "2.000 mm".into())),
        "{rows:?}"
    );
}

#[test]
fn times_read_as_hours_minutes_and_seconds() {
    assert_eq!(hms(0.0), "0:00:00");
    assert_eq!(hms(61.5), "1:01:30");
}

#[test]
fn trace_bitmap_takes_its_area_and_colours_from_the_2d_view() {
    use cut_craft_core::bitmap::Bitmap;
    use cut_craft_core::geometry::{Point, Rect};
    let mut h = Harness::new();
    let mut c = Change::builder(&h.app.doc);
    // a red left half and a blue right half on 100 x 50 mm
    let rgb = (0..4 * 2)
        .flat_map(|i| {
            if i % 4 < 2 {
                [200, 20, 20]
            } else {
                [20, 20, 200]
            }
        })
        .collect();
    c.add_bitmap(Bitmap {
        name: "b".into(),
        columns: 4,
        rows: 2,
        grey: vec![100; 8],
        rect: Rect::new(Point::new(0.0, 0.0), Point::new(100.0, 50.0)),
        depth: Vec::new(),
        rgb,
    });
    h.app.record(Ok(c.finish()), "added");
    h.app.invoke(&h.ctx, "trace_bitmap");
    h.frame(vec![]);
    let (from, to) = (h.at(10.0, 10.0), h.at(40.0, 40.0));
    h.drag(from, to);
    let area: Rect =
        serde_json::from_value(h.app.form.as_ref().unwrap().params["area"].clone()).unwrap();
    assert!(
        (area.min.x - 10.0).abs() < 1.0 && (area.max.y - 40.0).abs() < 1.0,
        "{area:?}"
    );
    // a click lets the area go; in colour mode the next one picks the colour clicked
    h.click(h.at(70.0, 25.0), Modifiers::NONE);
    assert!(h.app.form.as_ref().unwrap().params["area"].is_null());
    h.app.form.as_mut().unwrap().params["mode"] = "colors".into();
    // the blue starts traced (the red counts as the background): a click lets it go
    h.click(h.at(70.0, 25.0), Modifiers::NONE);
    assert_eq!(
        h.app.form.as_ref().unwrap().params["trace"],
        serde_json::json!([])
    );
    h.click(h.at(70.0, 25.0), Modifiers::NONE);
    assert_eq!(
        h.app.form.as_ref().unwrap().params["trace"],
        serde_json::json!(["#1414c8"])
    );
}

#[test]
fn left_to_right_windows_and_right_to_left_crosses() {
    let mut h = Harness::new();
    h.rectangle(10.0, 10.0, 50.0, 50.0);
    h.rectangle(100.0, 10.0, 50.0, 50.0);
    h.drag(h.at(0.0, 0.0), h.at(120.0, 80.0));
    assert_eq!(
        h.app.doc.selection.len(),
        1,
        "only the first rectangle is fully inside"
    );
    h.drag(h.at(120.0, 80.0), h.at(0.0, 0.0));
    assert_eq!(
        h.app.doc.selection.len(),
        2,
        "crossing also takes the one it touches"
    );
}

#[test]
fn the_status_bar_says_what_the_tool_in_hand_does_next() {
    let mut h = Harness::new();
    let hint = |h: &mut Harness, at: egui::Pos2| {
        h.frame(vec![Event::PointerMoved(at)]);
        h.frame(vec![]);
        cut_craft_bug_report::hint::current(&h.ctx).unwrap_or_default()
    };
    let somewhere = h.at(100.0, 100.0);
    // an empty sheet: where to start
    let empty = hint(&mut h, somewhere);
    assert!(empty.starts_with("Nothing drawn yet"), "{empty}");
    h.rectangle(400.0, 300.0, 10.0, 10.0);
    h.app.doc.selection = cut_craft_core::selection::Selection::new();
    // selecting, with nothing selected
    let select = hint(&mut h, somewhere);
    assert!(
        select.starts_with("Click a vector to select it"),
        "{select}"
    );
    // a polyline: its first point, then the next
    h.app.invoke(&h.ctx, "draw_polyline");
    let first = hint(&mut h, somewhere);
    assert!(first.contains("click the first point"), "{first}");
    h.click(h.at(10.0, 10.0), Modifiers::NONE);
    let next = hint(&mut h, somewhere);
    assert!(next.starts_with("Click the next point"), "{next}");
    assert!(next.contains("click the first point to close it"), "{next}");
    // a dragged shape, and Shift-click to edit one
    h.app.form = None;
    h.app.polyline = crate::polyline::Draft::default();
    h.app.invoke(&h.ctx, "draw_circle");
    let circle = hint(&mut h, somewhere);
    assert!(
        circle.starts_with("Draw Circle: drag from the centre"),
        "{circle}"
    );
    assert!(
        circle.contains("Shift-click one this tool drew to edit it"),
        "{circle}"
    );
}

#[test]
fn every_command_has_a_line_saying_what_it_does() {
    let missing: Vec<&str> = crate::commands::registry()
        .iter()
        .map(cut_craft_core::DynCommand::id)
        .filter(|id| crate::gui::english_about(id).is_none_or(str::is_empty))
        .collect();
    assert!(missing.is_empty(), "no description for {missing:?}");
    assert!(
        crate::gui::about("pocket_toolpath")
            .unwrap()
            .starts_with("Pocket Toolpath: clears")
    );
}

#[test]
fn the_height_buttons_are_bigger_than_an_ordinary_button() {
    let (mut h, dome) = dome_from_above();
    h.click(dome, Modifiers::NONE);
    h.frame(vec![]);
    let at = |which: Handle3d| {
        h.app
            .handles_3d
            .iter()
            .find(|(k, _)| *k == which)
            .map(|(_, at)| *at)
            .unwrap()
    };
    let apart = (at(Handle3d::Height) - at(Handle3d::Total)).length();
    let button = h.ctx.global_style().spacing.interact_size.y;
    assert!(
        apart >= button * super::panels::HEIGHT_BUTTON,
        "{apart} apart, buttons {button}"
    );
}

#[test]
fn the_design_tab_opens_the_sub_tab_a_tool_run_elsewhere_lives_on() {
    let mut h = Harness::new();
    assert_eq!(h.app.design_subtab, 0, "Create is open at the start");
    h.rectangle(10.0, 10.0, 20.0, 10.0);
    h.app.invoke(&h.ctx, "align_left");
    assert_eq!(
        gui::DESIGN_SUBTABS[h.app.design_subtab].name,
        "Arrange",
        "an alignment from the Edit menu shows the Arrange sub-tab"
    );
    h.app.invoke(&h.ctx, "zoom_to_job");
    assert_eq!(
        h.app.design_subtab, 2,
        "a tool always shown changes nothing"
    );
    h.app.invoke(&h.ctx, "sculpt");
    assert_eq!(gui::DESIGN_SUBTABS[h.app.design_subtab].name, "Model");
}

#[test]
fn a_design_sub_tab_can_have_a_section_of_its_own() {
    let mut h = Harness::new();
    // tall enough for the buttons at their full size and what comes under them
    h.window = vec2(1280.0, 1600.0);
    h.app.pages.set_pinned(Page::Design, true);
    h.app.pages.open(Page::Design);
    h.frame(vec![]);
    assert!(h.text_at("Create Vectors").is_some(), "Create is open");
    assert!(h.text_at("Shape Creation").is_none(), "Model is a tab");
    h.app
        .invoke(&h.ctx, &format!("{}Import:section", gui::SUBTAB_PLACE));
    h.frame(vec![]);
    for group in ["Create Vectors", "Import"] {
        assert!(h.text_at(group).is_some(), "{group} shows");
    }
    // Model's section comes before Import's, which it pushes down the page
    h.app
        .invoke(&h.ctx, &format!("{}Model:section", gui::SUBTAB_PLACE));
    assert_eq!(h.app.options.design_sections, ["Model", "Import"]);
    h.frame(vec![]);
    for group in ["Create Vectors", "Shape Creation"] {
        assert!(h.text_at(group).is_some(), "{group} shows");
    }
    // a tool on a section leaves the row's tab as it was
    h.app.invoke(&h.ctx, "sculpt");
    assert_eq!(h.app.design_subtab, 0);
    // kept between runs
    let json = serde_json::to_string(&h.app.options).unwrap();
    let (back, _) = crate::options::Options::from_bytes(json.as_bytes());
    assert_eq!(back.design_sections, ["Model", "Import"]);
    // and all back in one row
    h.app.invoke(&h.ctx, &format!("{}*:row", gui::SUBTAB_PLACE));
    assert_eq!(h.app.options.design_sections, Vec::<String>::new());
    h.frame(vec![]);
    assert!(h.text_at("Shape Creation").is_none());
}

#[test]
fn the_view_buttons_are_the_view_actions() {
    for id in VIEW_BUTTONS {
        let icon = gui::built_in_icon(id).unwrap_or_default();
        assert!(icon_set().contains(icon), "{id}: {icon}");
    }
}

#[test]
fn menu_items_that_reuse_existing_tools() {
    let mut h = Harness::new();
    h.rectangle(10.0, 10.0, 20.0, 20.0);
    h.app
        .apply("add_layer", &serde_json::json!({ "name": "Top" }))
        .unwrap();
    h.rectangle(50.0, 10.0, 20.0, 20.0);
    let on_top = *h.app.doc.selection.iter().next().unwrap();
    h.app.invoke(&h.ctx, "select_all_vectors");
    h.app.invoke(&h.ctx, "select_all_vectors_on_current_layer");
    assert_eq!(
        h.app.doc.selection,
        [on_top].into(),
        "Select Vectors on Layer, current layer"
    );
    h.app.invoke(&h.ctx, "job_size_and_position");
    assert_eq!(h.app.form.as_ref().map(|f| f.id), Some("job_setup"));
    h.app.invoke(&h.ctx, "show_toolpaths_tab");
    assert_ne!(h.app.pages.shown(Page::Toolpaths), Shown::Hidden);
    h.app.invoke(&h.ctx, "zoom_box");
    h.app.invoke(&h.ctx, "select");
    assert!(
        h.app.form.is_none() && h.app.view_tool.is_none(),
        "the Select tool"
    );
}

/// Controls that wait on a person or open other programs: the system's file dialogs, the
/// save-changes question (the smoke test's job has unsaved work), the file manager and
/// quitting. Running one in a test puts a real window on the desktop.
fn needs_a_person(app: &CutCraftApp, id: &str) -> bool {
    const ASKS: [&str; 30] = [
        "save_shaded_image",
        "import_toolpaths_file",
        // opens the browser
        "print_job_sheet",
        "add_clipart_folder",
        "segment_3d_model_file",
        "new",
        "open",
        "close",
        "save",
        "save_as",
        "increment_and_save",
        "open_application_data_folder",
        "import_vectors_bitmap",
        "import_cut_list",
        "import_component_3d_model",
        "save_as_template",
        "new_from_template",
        "save_selected_toolpath_as_template",
        "save_all_visible_toolpaths_as_template",
        "load_template",
        "print",
        "install_post_processor",
        "install_offline_package",
        // opens the browser
        "search_for_machine_online",
        "exit",
        "recent_files",
        "import_vectors",
        "export_vectors",
        "open_folder",
        "open_clipart_folder",
    ];
    ASKS.contains(&id)
        || app.registry.exporter(id).is_some()
        || id.starts_with(EXPORT_LEVEL)
        // a file dialog for the brush
        || id.starts_with(gui::SAVE_BRUSH)
}

/// Every enabled menu item and tool button, run through the app as a click would be on a
/// small job with vectors, a toolpath and a 3D shape: none may panic or be a dead end.
#[test]
fn every_enabled_control_works() {
    let mut h = Harness::new();
    h.app.doc.job.width = 100.0;
    h.app.doc.job.height = 100.0;
    h.rectangle(10.0, 10.0, 40.0, 40.0);
    h.rectangle(30.0, 30.0, 40.0, 40.0);
    h.app.invoke(&h.ctx, "select_all_vectors");
    for id in ["profile_toolpath", "create_shape"] {
        let params = h.app.registry.get(id).unwrap().initial_params(&h.app.doc);
        h.app.apply(id, &params).unwrap();
    }
    h.app.selected_toolpath = h.app.doc.toolpaths.keys().next().copied();
    h.frame(vec![]);
    let toolbar = &h.app.toolbar;
    let mut ids: Vec<String> = h
        .app
        .menus
        .items()
        .iter()
        .map(|i| i.action.clone())
        .collect();
    for group in toolbar
        .groups
        .iter()
        .chain(&toolbar.modeling_groups)
        .chain(&toolbar.toolpath_groups)
    {
        ids.extend(group.tools.iter().map(|&t| t.to_owned()));
    }
    ids.extend(VIEW_BUTTONS.iter().map(|id| (*id).to_owned()));
    // the right-click menus, on each thing they open on
    let doc = &h.app.doc;
    let mut popups = h.app.menus.selection_menu(doc);
    for layer in &doc.layers {
        popups.extend(gui::layer_menu(doc, layer));
    }
    for (&id, toolpath) in &doc.toolpaths {
        popups.extend(gui::toolpath_menu(doc, id, toolpath));
    }
    for (&id, component) in &doc.components {
        popups.extend(gui::component_menu(doc, id, component));
    }
    for (&id, level) in &doc.levels {
        popups.extend(gui::level_menu(doc, id, level));
    }
    for group in doc.components.values().filter_map(|k| k.group.as_ref()) {
        popups.extend(gui::group_menu(group));
    }
    let enabled = gui::leaves(&popups).into_iter().filter(|i| i.is_enabled);
    ids.extend(enabled.map(|i| i.action.clone()));
    ids.sort();
    ids.dedup();
    let start = (h.app.doc.clone(), h.app.selected_toolpath);
    let mut ran = 0;
    for id in ids {
        if !h.app.available(&id) || needs_a_person(&h.app, &id) {
            continue;
        }
        (h.app.doc, h.app.selected_toolpath) = start.clone();
        h.app.history = History::default();
        (h.app.form, h.app.view_tool) = (None, None);
        h.app.status.set_message("");
        h.app.invoke(&h.ctx, &id);
        // a tool with a form runs with what the form starts with
        if let Some(form) = h.app.form.take() {
            let outcome = h.app.apply(form.id, &form.params);
            if let Err(why) = outcome {
                assert!(!why.contains("not implemented"), "{id}: {why}");
            }
        }
        h.frame(vec![]);
        let said = &h.app.status.message;
        assert!(!said.contains("not implemented"), "{id}: {said}");
        ran += 1;
    }
    assert!(ran >= 92, "only {ran} controls ran");
}

#[test]
fn the_side_bar_lists_other_tools_like_this_with_their_icons() {
    let h = Harness::new();
    let group = h.app.help_group_of("draw_circle");
    assert!(
        group.contains(&"draw_circle") && group.len() > 2,
        "{group:?}"
    );
    let bar = h
        .app
        .help_sidebar("draw_circle", &h.app.help_page("draw_circle"));
    let others: Vec<String> = group
        .iter()
        .filter(|t| **t != "draw_circle")
        .map(|t| (*t).to_owned())
        .collect();
    assert_eq!(bar.like_this, others, "the rest of its group, in order");
    for t in &bar.like_this {
        assert!(h.app.help_icon(t).is_some(), "{t} shows its icon");
    }
    // pages that are no tool's have no group, but still their related pages
    assert_eq!(h.app.help_group_of("stepover").len(), 0);
    assert_ne!(side_bar(&h, "stepover").len(), 0);
}

#[test]
fn a_page_links_the_names_of_other_pages_in_its_words() {
    let h = Harness::new();
    let names = h.app.help_page("pocket_toolpath");
    assert!(names.contains("(help:stepover)"), "{names}");
    // a tool's own name never links to itself
    assert!(
        !h.app
            .help_page("draw_circle")
            .contains("(help:draw_circle)")
    );
}

#[test]
fn the_performance_page_limits_calculations_with_a_share_slider() {
    let mut h = Harness::new();
    h.app.invoke(&h.ctx, "options");
    h.frame(vec![]);
    h.frame(vec![]);
    let at = *h.texts_at("Performance").last().expect("a page of its own");
    h.click(at, Modifiers::NONE);
    h.frame(vec![]);
    for setting in [
        "Limit calculations so other programs stay smooth",
        "Share of the computer to use",
        "Preview detail",
        "Adjust detail to keep the view smooth",
    ] {
        assert!(h.text_at(setting).is_some(), "{setting} in {:?}", h.texts());
    }
}

#[test]
fn advanced_options_show_only_once_ticked_and_warn_first() {
    let mut h = Harness::new();
    h.app.invoke(&h.ctx, "options");
    h.frame(vec![]);
    h.frame(vec![]);
    let tick = h.text_at("Show advanced settings").expect("the tick box");
    h.click(tick, Modifiers::NONE);
    h.frame(vec![]);
    let at = *h.texts_at("Advanced").last().expect("listed once ticked");
    h.click(at, Modifiers::NONE);
    h.frame(vec![]);
    assert!(h.text_at("DXF curve detail (points per span)").is_some());
    assert!(
        h.text_at(crate::options::WARNING).is_some(),
        "a warning above them"
    );
    assert_eq!(
        h.text_at("Use Wayland directly (no file drag and drop)")
            .is_some(),
        crate::options::Advanced::wayland(),
        "the Wayland choice only on a Wayland desktop"
    );
}

#[test]
fn options_cancel_throws_away_changes_not_applied() {
    let mut h = options_with_a_change_not_applied();
    click_button(&mut h, "Cancel");
    assert!(!h.app.options_editor.open, "closed");
    assert_eq!(h.app.options.general.recent_file_list_size, 9, "as it was");
    assert!(!h.app.options_dirty());
    // opened again, it shows the options as they are
    h.app.invoke(&h.ctx, "options");
    h.frame(vec![]);
    h.frame(vec![]);
    assert!(h.text_at(super::options_apply::ALL_APPLIED).is_some());
}

#[test]
fn reset_to_defaults_puts_back_only_the_page_shown() {
    let mut h = Harness::new();
    h.app.options.appearance.theme = crate::options::Theme::Dark;
    h.app.options.general.recent_file_list_size = 9;
    h.app.invoke(&h.ctx, "options");
    h.frame(vec![]);
    h.frame(vec![]);
    let at = *h.texts_at("Appearance").last().expect("listed");
    h.click(at, Modifiers::NONE);
    h.frame(vec![]);
    let reset = h.text_at("Reset to defaults").expect("the page's reset");
    h.click(reset, Modifiers::NONE);
    h.frame(vec![]);
    click_button(&mut h, "Apply");
    // Light, or System on a Mac
    assert_eq!(
        h.app.options.appearance.theme,
        crate::options::Theme::default()
    );
    assert_eq!(
        h.app.options.general.recent_file_list_size, 9,
        "other pages kept"
    );
}

#[test]
fn the_window_title_sits_in_the_menu_bar_unless_the_desktop_s_is_chosen() {
    let mut h = Harness::new();
    let title = h.app.title.clone();
    let file = h.text_at("File").expect("the File menu");
    let at = h
        .text_at(&title)
        .expect("the title in Cut Craft's own title bar");
    assert!((at.y - file.y).abs() < 1.0, "in the menus' row");
    assert!(at.x > file.x, "after the menus");
    h.app.options.appearance.system_title_bar = true;
    h.frame(vec![]);
    assert_eq!(h.text_at(&title), None, "the desktop's title bar shows it");
}

#[test]
fn the_log_says_what_a_command_made() {
    let mut h = Harness::new();
    h.rectangle(10.0, 10.0, 40.0, 30.0);
    let vector = *h.app.doc.vectors.keys().next().unwrap();
    let step = cut_craft_core::toolpath::Move {
        kind: cut_craft_core::toolpath::MoveKind::Cut,
        to: cut_craft_core::glam::DVec3::ZERO,
    };
    let mut c = Change::builder(&h.app.doc);
    c.add_toolpath(cut_craft_core::toolpath::Toolpath {
        name: "Profile 1".into(),
        strategy: "test".into(),
        params: serde_json::Value::Null,
        tool: Tool::default(),
        vectors: vec![vector],
        moves: vec![step; 3],
        visible: true,
        power: vec![],
        stale: false,
        group: None,
        drill_bank: None,
        fallbacks: vec![],
        across: vec![],
    });
    assert_eq!(
        super::background::summary(&c.finish()),
        "done, changed 1 toolpath, 3 moves"
    );
    assert_eq!(
        super::background::summary(&Change::default()),
        "done, nothing changed"
    );
}

#[test]
fn the_design_tab_shows_its_favourites_or_all_its_tools() {
    let mut h = Harness::new();
    h.app.pages.set_pinned(Page::Design, true);
    h.app.pages.open(Page::Design);
    h.frame(vec![]);
    let first_group = h.app.toolbar.groups[0].name;
    assert!(
        h.text_at(first_group).is_some(),
        "all the tools to start with"
    );
    // the tab's menu: all tools ticked
    let menu = gui::favourites_menu(Page::Design, false);
    assert_eq!(menu[0].name, "Show Favourites");
    assert!(menu[1].is_checked);
    // right-clicked tools added, then one moved earlier
    let tool = |id: &str, what: &str| format!("{}design:{what}:{id}", gui::FAVOURITE);
    assert_eq!(
        gui::tool_menu(Page::Design, "draw_circle", false, false, false)[0].action,
        tool("draw_circle", "add")
    );
    h.app.invoke(&h.ctx, &tool("draw_circle", "add"));
    h.app.invoke(&h.ctx, &tool("draw_rectangle", "add"));
    h.app.invoke(&h.ctx, &tool("draw_rectangle", "earlier"));
    assert_eq!(
        h.app.options.favourites.design,
        ["draw_rectangle", "draw_circle"]
    );
    assert!(
        h.app.options.favourites.toolpaths.is_empty(),
        "each tab its own"
    );
    h.app
        .invoke(&h.ctx, &format!("{}design:true", gui::FAVOURITES_ONLY));
    h.frame(vec![]);
    assert!(
        !h.texts_at("Design ★").is_empty(),
        "the tab says it is filtered"
    );
    assert!(h.text_at("Favourites").is_some());
    assert!(h.text_at(first_group).is_none(), "only the favourites");
    // taken off them all, the way back is offered
    h.app.invoke(&h.ctx, &tool("draw_circle", "remove"));
    h.app.invoke(&h.ctx, &tool("draw_rectangle", "remove"));
    h.frame(vec![]);
    let back = h.text_at("Show All Tools").expect("a way back");
    h.click(back, Modifiers::NONE);
    h.frame(vec![]);
    assert!(!h.app.options.favourites.design_only);
    assert!(h.text_at(first_group).is_some());
}

#[test]
fn every_tool_on_the_tool_pages_has_an_icon() {
    let h = Harness::new();
    let toolbar = &h.app.toolbar;
    let missing: Vec<&str> = toolbar
        .groups
        .iter()
        .chain(&toolbar.modeling_groups)
        .chain(&toolbar.toolpath_groups)
        .flat_map(|g| g.tools.iter().copied())
        .filter(|id| h.app.registry.get(id).is_some() || gui::built_in_label(id).is_some())
        .filter(|id| h.app.icons.find(h.app.explicit_icon(id), id).is_none())
        .collect();
    assert!(
        missing.is_empty(),
        "drawn as text, with no icon: {missing:?}"
    );
}

#[test]
fn a_side_panel_never_takes_more_than_its_share_of_the_window() {
    // content that asks for a little more than the room it is given, every frame, as a
    // frame sized to `available_width` inside a resizable panel can
    let ctx = egui::Context::default();
    let window = Rect::from_min_size(Pos2::ZERO, vec2(1280.0, 800.0));
    let mut width = 0.0;
    for frame in 0..300 {
        let input = RawInput {
            screen_rect: Some(window),
            time: Some(f64::from(frame) * 0.1),
            ..Default::default()
        };
        headless(ctx.run_ui(input, |ui| {
            width = side_panel(egui::Panel::left("creeping"), ui.ctx())
                .show(ui, |ui| ui.set_min_width(ui.available_width() + 10.0))
                .response
                .rect
                .width();
        }));
    }
    assert!(
        width <= SIDE_PANEL_SHARE * window.width() + 1.0,
        "{width} of {}",
        window.width()
    );
}

#[test]
fn a_pinned_page_has_room_for_five_tool_buttons_side_by_side() {
    for text_scale in [1.0, 1.15, 1.5, 2.0] {
        // a tall window, where the pages are as narrow as they go
        let mut h = both_pinned();
        h.window = vec2(1600.0, 1400.0);
        readable(&h.ctx, text_scale);
        for _ in 0..3 {
            h.frame(vec![]);
        }
        let across = h.app.buttons_across.get();
        assert!(across >= 5.0, "{across} at {text_scale}× text");
    }
}

#[test]
fn unpinning_one_page_leaves_the_others_width_alone() {
    let mut h = both_pinned();
    let both = page_widths(&mut h, 2)[1];
    h.app.pages.set_pinned(Page::Design, false);
    let one = page_widths(&mut h, 2);
    assert_eq!(
        one[0],
        [None, both[1]],
        "no jump in the frame it was unpinned"
    );
    assert_eq!(one[1], one[0]);
}

#[test]
fn a_flown_out_page_is_as_wide_as_a_pinned_one() {
    let mut h = Harness::new();
    let docked = h.app.page_rects[0].expect("Design docked").width();
    h.app.pages.set_pinned(Page::Design, false);
    h.app.pages.open(Page::Design);
    h.frame(vec![]);
    let out = h.app.flyout_rect.expect("Design flown out").width();
    assert!(
        (out - docked).abs() <= 1.0,
        "{out} flown out, {docked} docked"
    );
    // nor does a form on it, or the frames after, push it wider
    h.rectangle(10.0, 10.0, 50.0, 50.0);
    for form in ["job_setup", "draw_text", "offset_vectors"] {
        h.app.invoke(&h.ctx, form);
        h.app.pages.open(Page::Design);
        for _ in 0..6 {
            h.frame(vec![]);
            let now = h.app.flyout_rect.expect("still out").width();
            assert_eq!(now, out, "{form}");
        }
        h.app.form = None;
    }
}

#[test]
fn the_status_bar_shows_the_version() {
    let mut h = Harness::new();
    h.frame(vec![]);
    let version = concat!("v", env!("CARGO_PKG_VERSION"));
    let at = h
        .text_at(version)
        .expect("the version is in the status bar");
    assert!(at.y > 700.0, "at the bottom: {at:?}");
}

#[test]
fn a_flyout_opens_from_its_arrow_or_a_held_press_and_shows_the_tool_picked_last() {
    let mut h = Harness::new();
    h.app.options.general.fold_tool_groups = true;
    h.app.pages.set_pinned(Page::Design, true);
    h.frame(vec![]);
    h.frame(vec![]);
    assert!(
        flyout_arrows(&h).len() >= 9,
        "the Drawing tab's flyouts show their arrows"
    );
    let arrow = flyout_arrow_naming(&mut h, "Draw Circle from Edge");
    // a click on the arrow opens the flyout, each tool by name, and runs nothing
    h.click(arrow, Modifiers::NONE);
    h.frame(vec![]);
    assert!(h.app.form.is_none(), "the arrow runs nothing");
    assert!(
        h.text_at("Draw Ellipse").is_some(),
        "the flyout names its tools"
    );
    let pick = h.text_at("Draw Circle from Edge").expect("listed");
    h.click(pick, Modifiers::NONE);
    let form = h.app.form.as_ref().expect("its form");
    assert_eq!(form.id, "draw_circle");
    assert_eq!(form.way, crate::flyout::DragWay::AcrossEdge);
    assert_eq!(
        h.app
            .options
            .flyout_faces
            .get("draw_circle")
            .map(String::as_str),
        Some("draw_circle_from_edge"),
        "the pick becomes the button's face"
    );
    // a press held on the face opens it too, and its release runs nothing
    h.app.form = None;
    h.frame(vec![]);
    h.frame(vec![]);
    let face = arrow - vec2(12.0, 12.0);
    h.frame(vec![Event::PointerMoved(face)]);
    h.press(face, true, Modifiers::NONE);
    for _ in 0..6 {
        h.frame(vec![]);
    }
    assert!(h.text_at("Draw Ellipse").is_some(), "held open");
    h.press(face, false, Modifiers::NONE);
    h.frame(vec![]);
    assert!(
        h.app.form.is_none(),
        "the release after a hold runs nothing"
    );
    // a plain click runs the face: the tool picked last
    h.frame(vec![key(Key::Escape)]);
    h.click(face, Modifiers::NONE);
    assert_eq!(
        h.app.form.as_ref().map(|f| (f.id, f.way)),
        Some(("draw_circle", crate::flyout::DragWay::AcrossEdge))
    );
}

/// Every non-ASCII character in the app's strings, with where it first appears: the text in
/// `"…"` literals of `src`, comments left out. `app/mac.rs` is left out too: its key symbols
/// (⌘ ⌥ ⌫ …) are only shown on a Mac, in the Mac's own font, which has them.
fn label_characters() -> std::collections::BTreeMap<char, String> {
    fn walk(dir: &std::path::Path, files: &mut Vec<std::path::PathBuf>) {
        for entry in std::fs::read_dir(dir).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                walk(&path, files);
            } else if path.extension().is_some_and(|e| e == "rs") {
                files.push(path);
            }
        }
    }
    let src = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut files = Vec::new();
    walk(&src, &mut files);
    files.sort();
    let mut found = std::collections::BTreeMap::new();
    for file in files {
        let rel = file
            .strip_prefix(&src)
            .unwrap()
            .to_string_lossy()
            .replace('\\', "/");
        if rel == "app/mac.rs" || rel.ends_with("tests.rs") || rel.contains("/tests/") {
            continue;
        }
        let text = std::fs::read_to_string(&file).unwrap();
        // inside a string literal, which may run over several lines
        let mut in_string = false;
        for (n, line) in text.lines().enumerate() {
            let mut chars = line.chars().peekable();
            while let Some(c) = chars.next() {
                if in_string {
                    match c {
                        '\\' => {
                            chars.next();
                        }
                        '"' => in_string = false,
                        c if !c.is_ascii() => {
                            found.entry(c).or_insert_with(|| format!("{rel}:{}", n + 1));
                        }
                        _ => {}
                    }
                } else if c == '"' {
                    in_string = true;
                } else if c == '\'' {
                    // a char literal such as '"', or a lifetime: step over a quoted quote
                    if chars.peek() == Some(&'"') {
                        chars.next();
                    }
                } else if c == '/' && chars.peek() == Some(&'/') {
                    break;
                }
            }
        }
    }
    found
}

#[test]
fn label_characters_are_found_in_strings_only() {
    let found = label_characters();
    assert!(found.contains_key(&'…'), "{found:?}");
    assert!(found.contains_key(&'▸'), "{found:?}");
    assert!(
        !found.contains_key(&'⌘'),
        "the Mac's key symbols are its own font's"
    );
}

/// Every non-ASCII character the app writes has a glyph in egui's own fonts, which go with the
/// program everywhere: a system font that lacks one then still draws it, not an empty box
/// (the 📌 pin button once did, on Linux and Windows).
#[test]
fn every_label_character_has_a_glyph() {
    let ctx = egui::Context::default();
    ctx.set_fonts(super::ui_fonts(None));
    let _ = headless(ctx.run_ui(RawInput::default(), |_| {}));
    let font = egui::FontId::proportional(14.0);
    let missing: Vec<String> = label_characters()
        .into_iter()
        .filter(|&(c, _)| !ctx.fonts_mut(|f| f.has_glyph(&font, c)))
        .map(|(c, at)| format!("{c} (U+{:04X}) at {at}", u32::from(c)))
        .collect();
    assert!(
        missing.is_empty(),
        "no glyph in egui's fonts, so these draw as empty boxes; use an icon \
         (assets/icons) or pick a character they have:\n{}",
        missing.join("\n")
    );
}

/// Every character the language packs in the source use has a glyph in Cut Craft's fonts,
/// so a pack never shows empty boxes (a language whose letters the fonts lack needs a font
/// added first).
#[test]
fn every_language_pack_character_has_a_glyph() {
    let ctx = egui::Context::default();
    ctx.set_fonts(super::ui_fonts(None));
    let _ = headless(ctx.run_ui(RawInput::default(), |_| {}));
    let font = egui::FontId::proportional(14.0);
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../language-packs");
    let mut missing = std::collections::BTreeSet::new();
    for pack in std::fs::read_dir(&root).unwrap().flatten() {
        for file in std::fs::read_dir(pack.path())
            .into_iter()
            .flatten()
            .flatten()
        {
            if file.path().extension().is_none_or(|x| x != "ftl") {
                continue;
            }
            let text = std::fs::read_to_string(file.path()).unwrap();
            for c in text.chars().filter(|c| !c.is_control()) {
                if !ctx.fonts_mut(|f| f.has_glyph(&font, c)) {
                    missing.insert(format!(
                        "{c} (U+{:04X}) in {}",
                        u32::from(c),
                        file.path().display()
                    ));
                }
            }
        }
    }
    assert!(
        missing.is_empty(),
        "no glyph for:\n{}",
        missing.into_iter().collect::<Vec<_>>().join("\n")
    );
}

#[test]
fn switching_units_and_back_leaves_every_tool_figure_as_it_was() {
    let mut h = Harness::new();
    h.rectangle(10.0, 10.0, 50.0, 50.0);
    h.app.invoke(&h.ctx, "profile_toolpath");
    h.app.pages.set_pinned(Page::Toolpaths, true);
    h.app.choose_tool("3 mm End Mill");
    h.frame(vec![]);
    let before = h.app.form.as_ref().unwrap().params.clone();
    let database = serde_json::to_value(&h.app.tools).unwrap();
    for units in ["units_inch", "units_mm", "units_inch", "units_mm"] {
        h.app.set_units(units);
        h.frame(vec![]);
        h.frame(vec![]);
    }
    assert_eq!(
        h.app.form.as_ref().unwrap().params["tool"],
        before["tool"],
        "kept in mm, only shown in inches"
    );
    assert_eq!(serde_json::to_value(&h.app.tools).unwrap(), database);
}

/// Tools that were only in a menu get a button too (Matthew, 2026-11-10): the left page's
/// for what changes the shape to cut, the right page's for how it is cut and sent to the
/// machine, each where the help puts its icon. Buttons already there stay put.
#[test]
fn menu_only_tools_have_a_button_on_their_side() {
    let h = Harness::new();
    let toolbar = &h.app.toolbar;
    let model = toolbar.subtab_groups(
        gui::DESIGN_SUBTABS
            .iter()
            .position(|t| t.name == "Model")
            .unwrap(),
    );
    let on = |groups: &[gui::ToolGroup], id: &str| groups.iter().any(|g| g.tools.contains(&id));
    for id in [
        "reset_inside_selected_vectors",
        "reset_outside_selected_vectors",
        "component_boundary",
        "cross_section",
        "create_component_from_visible_model",
        "export_as_stl",
        "export_as_grayscale_bitmap",
        "export_as_3d_clipart",
    ] {
        assert!(on(&model, id), "{id} on the Model sub-tab");
    }
    let design = toolbar.all_design_groups();
    for id in ["notes", "document_variables"] {
        assert!(on(&design, id), "{id} on the Design page");
    }
    for id in [
        "load_template",
        "save_selected_toolpath_as_template",
        "save_all_visible_toolpaths_as_template",
        "create_job_sheet",
        "print_job_sheet",
        "part_labels",
        "cut_list",
    ] {
        assert!(
            on(&toolbar.toolpath_groups, id),
            "{id} on the Toolpaths page"
        );
        assert!(
            !on(&design, id),
            "{id} is how it is cut, not on the Design page"
        );
    }
    // each says what it is in words, not its id
    for (id, label) in [
        ("export_as_stl", "Export Model as STL"),
        (
            "export_as_grayscale_bitmap",
            "Export Model as Grayscale Picture",
        ),
        ("export_as_3d_clipart", "Export Model as 3D Clipart"),
        ("load_template", "Load Toolpath Template"),
        ("create_job_sheet", "Save Job Sheet"),
        ("cut_list", "Save Cut List"),
    ] {
        assert_eq!(h.app.tool_label(id), label);
        assert!(gui::about(id).is_some(), "{id} has a tooltip");
    }
}

#[test]
fn dont_ask_again_stops_the_question_but_not_the_mark() {
    let mut h = pocketed();
    move_first_vector(&mut h);
    h.frame(vec![]);
    h.frame(vec![]);
    let silence = h.text_at("Don't ask again").expect("the tick box");
    h.click(silence, Modifiers::NONE);
    let no = h.text_at("Not Now").expect("the button");
    h.click(no, Modifiers::NONE);
    h.frame(vec![]);
    assert!(!h.app.options.confirmations.ask_to_recalculate);
    move_first_vector(&mut h);
    h.frame(vec![]);
    h.frame(vec![]);
    assert!(
        h.text_at("Toolpaths out of date").is_none(),
        "not asked again"
    );
    assert!(
        h.app.doc.toolpaths.values().all(|t| t.stale),
        "still marked out of date"
    );
}

#[test]
fn cancel_brings_nothing_in() {
    let mut h = Harness::new();
    h.app.import_file(&unitless_dxf("units-cancel"));
    h.frame(vec![]);
    h.frame(vec![]);
    h.click(*h.texts_at("Cancel").last().unwrap(), Modifiers::NONE);
    h.frame(vec![]);
    assert!(h.app.doc.vectors.is_empty());
    assert!(
        h.text_at("This drawing doesn't say what units it was made in")
            .is_none()
    );
}

#[test]
fn a_page_that_names_its_icon_shows_it_beside_its_title_and_in_lists() {
    let h = Harness::new();
    let named: Vec<&str> = cut_craft_help::pages()
        .filter(|p| p.icon.is_some())
        .map(|p| p.id)
        .collect();
    assert!(named.contains(&"number_fields"), "{named:?}");
    for id in named {
        assert!(h.app.help_icon(id).is_some(), "{id}: its icon is not found");
        let uri = h
            .app
            .help_icon_uri(id, super::help::PAGE_ICON)
            .unwrap_or_default();
        let page = h.app.help_markdown(id);
        let title = page
            .lines()
            .find(|l| l.starts_with("# "))
            .unwrap_or_default();
        assert!(title.contains(&uri), "{id}: no icon by its title: {title}");
        assert!(title.ends_with(&h.app.help_title(id)), "{id}: {title}");
    }
}

/// Tools used only on the canvas have nothing to press: Vector Trim's Apply used to trim
/// at 0,0.
#[test]
fn canvas_only_tools_have_no_commit_button() {
    let mut h = Harness::new();
    h.rectangle(-10.0, -10.0, 20.0, 20.0);
    let before = h.app.doc.vectors.len();
    for id in ["interactive_vector_trim", "draw_freehand"] {
        h.app.invoke(&h.ctx, id);
        h.frame(vec![]);
        for label in ["Apply", "Create", "Calculate"] {
            assert!(h.text_at(label).is_none(), "{id}: {label}");
        }
        assert!(h.text_at("Close").is_some(), "{id}");
        assert!(
            h.text_at(&commit::live_note(cut_craft_core::Form::Gesture, true))
                .is_some(),
            "{id}"
        );
        h.app.form = None;
    }
    assert_eq!(h.app.doc.vectors.len(), before, "nothing trimmed or drawn");
}

/// A tool page that settles, with icons drawn and the app's own look at 100% text: the Edit
/// tab, in a window where its Join and Close box just fits six buttons across.
fn edit_tab_with_icons() -> Harness {
    let mut h = Harness::new();
    egui_extras::install_image_loaders(&h.ctx);
    h.window = vec2(1700.0, 800.0);
    readable(&h.ctx, 1.0);
    h.app.design_subtab = gui::DESIGN_SUBTABS
        .iter()
        .position(|t| t.name == "Edit")
        .expect("an Edit tab");
    for _ in 0..4 {
        h.frame(vec![]);
    }
    // the icons decode on a background thread: one that lands mid-test grows its button and
    // moves the page, so wait until every picture is in (a slow machine took long enough)
    for _ in 0..500 {
        if !h.ctx.has_pending_images() {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(10));
        h.frame(vec![]);
    }
    assert!(
        !h.ctx.has_pending_images(),
        "the icons never finished loading"
    );
    for _ in 0..2 {
        h.frame(vec![]);
    }
    h
}

/// Where the Design page's tip box starts: the first text on the page under the Transform
/// Objects heading, as its buttons are pictures.
fn tip_box_top(h: &Harness, page: Rect) -> Option<i32> {
    let heading = h.text_at("Transform Objects")?.y;
    let tops = h.shapes.iter().filter_map(|clipped| match &clipped.shape {
        egui::Shape::Text(t) if page.contains(t.pos) && t.pos.y > heading + 5.0 => Some(t.pos.y),
        _ => None,
    });
    // whole points: the tip's own first line moves nothing
    #[expect(clippy::cast_possible_truncation, reason = "a screen position")]
    tops.reduce(f32::min).map(|y| y.round() as i32)
}

#[test]
fn hovering_a_tool_button_never_rewraps_the_page() {
    // a hovered button's border is drawn a fraction of a point thicker: when that tipped a
    // full row's last button onto the next row, the pointer left it, it came back, and the
    // page jumped between the two layouts for as long as the pointer stayed
    let mut h = edit_tab_with_icons();
    let top = h
        .text_at("Join and Close")
        .expect("the Join and Close box")
        .y;
    // down to the Transform Objects row too, the last on the tab
    let bottom = h.text_at("Transform Objects").expect("its heading").y + 70.0;
    let page = h.app.page_rects[0].expect("the Design page");
    // the tip box's top edge, under the last row of buttons, moves if any row rewraps
    let settled = |h: &Harness| {
        (
            h.text_at("Combine Shapes"),
            h.text_at("Transform Objects"),
            tip_box_top(h, page),
        )
    };
    let still = settled(&h);
    let mut y = top;
    while y < bottom {
        let mut x = page.left();
        while x < page.right() {
            let at = Pos2::new(x, y);
            for frame in 0..5 {
                h.frame(vec![Event::PointerMoved(at)]);
                // the first frame may still show the button the pointer left
                if frame > 0 {
                    assert_eq!(
                        settled(&h),
                        still,
                        "the page moved with the pointer at {at:?}"
                    );
                }
            }
            x += 12.0;
        }
        y += 12.0;
    }
}

#[test]
fn a_greyed_item_says_what_it_waits_for_not_that_it_is_missing() {
    let h = Harness::new();
    for (id, said) in [
        ("undo", "Nothing to undo"),
        ("copy", "Select"),
        ("cut", "Select"),
        ("paste", "copy something first"),
        ("delete_toolpath", "Pick a toolpath"),
        ("recalculate_all_toolpaths", "No toolpaths yet"),
    ] {
        assert!(!h.app.available(id), "{id}");
        let why = h.app.unavailable_why(id);
        assert!(why.contains(said), "{id}: {why}");
        assert!(!why.to_lowercase().contains("not implemented"), "{id}");
    }
    assert!(
        h.app
            .unavailable_why("no_such_tool")
            .contains("not implemented yet")
    );
    let start = Harness::at_start();
    assert!(
        start
            .app
            .unavailable_why("close")
            .contains("No job is open")
    );
}

#[test]
fn a_language_pack_picked_in_options_is_used_once_applied() {
    let addons = std::env::temp_dir().join(format!("cut-craft-language-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&addons);
    let ru = addons.join("language-ru");
    std::fs::create_dir_all(&ru).unwrap();
    std::fs::write(
        ru.join(cut_craft_updater::addon::MANIFEST),
        "id = \"language-ru\"\nname = \"Русский\"\nkind = \"language\"\nversion = \"0.1.0\"\n\
         language = \"ru\"",
    )
    .unwrap();
    std::fs::write(
        ru.join(cut_craft_updater::addon::RECORD),
        "{\"repo\":{\"owner\":\"NoobyNull\",\"name\":\"CutCraft-Translations\"},\"tag\":\
         \"v1.0.0\",\"version\":\"1.0.0\",\"signed\":false}",
    )
    .unwrap();
    std::fs::write(ru.join("menus.ftl"), "menu-file = Файл\n").unwrap();
    let mut h = Harness::new();
    h.app.addons = crate::addons::Addons::new(Some(addons.clone()));
    h.app.options_editor.open_at(crate::options::LANGUAGE);
    h.frame(vec![]);
    h.frame(vec![]);
    // listed with how much it covers, and marked as not official (unsigned)
    assert!(h.text_at("Русский").is_some(), "{:?}", h.texts());
    assert!(h.text_at("⚠ not official").is_some());
    click_button(&mut h, "Русский");
    assert_eq!(h.app.edited_options().language.pack, "language-ru");
    assert_eq!(
        cut_craft_i18n::tr("menu-file"),
        "File",
        "not used before Apply"
    );
    h.apply_options();
    assert_eq!(h.app.options.language.pack, "language-ru");
    assert_eq!(cut_craft_i18n::tr("menu-file"), "Файл");
    // what the pack lacks stays English
    assert_eq!(cut_craft_i18n::tr("menu-edit"), "Edit");
    // back to English, and a pack that has gone leaves English and says so
    h.app.use_language("");
    assert_eq!(cut_craft_i18n::tr("menu-file"), "File");
    std::fs::remove_dir_all(&addons).unwrap();
    h.app.options.language.pack = "language-ru".to_owned();
    h.app.use_language("language-ru");
    assert_eq!(cut_craft_i18n::tr("menu-file"), "File");
    assert_eq!(h.app.options.language.pack, "", "Options says English too");
    assert!(
        h.app.status.message.contains("not installed"),
        "{}",
        h.app.status.message
    );
}

#[test]
fn while_a_calculation_runs_the_status_bar_says_how_briskly_and_opens_performance() {
    let mut h = Harness::new();
    h.frame(vec![]);
    let paces = ["Crawl", "Walk", "Run", "Sprint"];
    assert!(
        paces.iter().all(|p| h.text_at(p).is_none()),
        "nothing while nothing calculates"
    );
    let stop = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    let busy = std::sync::Arc::clone(&stop);
    let job = cut_craft_core::work::background("a test calculation", move || {
        let mut x = 0u64;
        while !busy.load(std::sync::atomic::Ordering::Relaxed) {
            x = std::hint::black_box(x.wrapping_mul(31).wrapping_add(7));
        }
    })
    .unwrap();
    while cut_craft_core::work::running() == 0 {
        std::thread::yield_now();
    }
    h.frame(vec![]);
    std::thread::sleep(cut_craft_core::work::Gauge::EVERY + std::time::Duration::from_millis(50));
    h.frame(vec![]);
    let shown: Vec<_> = paces.iter().filter_map(|p| h.text_at(p)).collect();
    assert_eq!(
        shown.len(),
        1,
        "one pace once a second has passed: {:?}",
        h.texts()
    );
    h.click(shown[0], Modifiers::NONE);
    h.frame(vec![]);
    h.frame(vec![]);
    assert!(
        h.text_at("Share of the computer to use").is_some(),
        "Options ▸ Performance"
    );
    stop.store(true, std::sync::atomic::Ordering::Relaxed);
    job.join().unwrap();
    h.frame(vec![]);
    assert!(
        paces.iter().all(|p| h.text_at(p).is_none()),
        "gone once it ends"
    );
}

#[test]
fn interface_layout_opens_the_layout_picker_and_use_this_layout_switches() {
    let mut h = Harness::new();
    assert_eq!(h.app.options.general.layout, WindowLayout::Classic);
    h.app.invoke(&h.ctx, "interface_layout");
    // a window is sized on its first frame and drawn on the next
    h.frame(vec![]);
    h.frame(vec![]);
    for name in [
        "Window Layout",
        "Ribbon",
        "Rail",
        "Classic",
        "Use This Layout",
        "Cancel",
    ] {
        assert!(h.text_at(name).is_some(), "{name}: {:?}", h.texts());
    }
    click_button(&mut h, "Rail");
    click_button(&mut h, "Use This Layout");
    assert_eq!(h.app.options.general.layout, WindowLayout::Rail);
    assert!(h.app.layout_question.is_none());
}

#[test]
fn cancel_on_the_layout_picker_changes_nothing() {
    let mut h = Harness::new();
    h.app.invoke(&h.ctx, "interface_layout");
    // a window is sized on its first frame and drawn on the next
    h.frame(vec![]);
    h.frame(vec![]);
    click_button(&mut h, "Ribbon");
    click_button(&mut h, "Cancel");
    assert_eq!(h.app.options.general.layout, WindowLayout::Classic);
    assert!(h.app.layout_question.is_none());
    assert!(!h.app.options.layout_asked);
}

#[test]
fn after_updating_the_question_offers_ribbon_and_keep_leaves_the_layout() {
    let mut h = Harness::new();
    h.app.ask_layout_after_update();
    h.frame(vec![]);
    h.frame(vec![]);
    assert_eq!(h.app.layout_question.unwrap().chosen, WindowLayout::Ribbon);
    click_button(&mut h, "Keep Classic");
    assert_eq!(h.app.options.general.layout, WindowLayout::Classic);
    assert!(h.app.options.layout_asked);
}
