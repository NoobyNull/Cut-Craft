//! The app driven headlessly, one frame at a time, with synthetic input.

use super::*;
use eframe::egui::{Event, PointerButton, Pos2, RawInput, Rect, vec2};

/// Drives the real UI headlessly, one frame per call, with synthetic pointer input.
#[test]
fn cut_copy_and_paste_vectors() {
    let mut h = Harness::new();
    h.rectangle(10.0, 10.0, 20.0, 10.0);
    let before = h.app.doc.vectors.len();
    assert!(before > 0);
    assert!(!h.app.available("paste"));
    h.app.invoke(&h.ctx, "copy");
    h.app.invoke(&h.ctx, "paste");
    assert_eq!(h.app.doc.vectors.len(), before + h.app.clipboard.len());
    assert_eq!(
        h.app.doc.selection.len(),
        h.app.clipboard.len(),
        "the pasted copies are selected"
    );
    h.app.invoke(&h.ctx, "cut");
    assert_eq!(
        h.app.doc.vectors.len(),
        before,
        "cut takes the selection away"
    );
    h.app.invoke(&h.ctx, "undo");
    assert_eq!(h.app.doc.vectors.len(), before + h.app.clipboard.len());
}

#[test]
fn a_model_past_the_preview_limit_is_drawn_as_its_surface_with_only_undercuts_over_it() {
    let mut h = Harness::new();
    h.app.options.import.preview_faces = 1000;
    let file = pyramid_stl("limit");
    h.app.import_file(&file);
    h.frame(vec![]);
    let form = h.app.form.as_ref().unwrap();
    assert!(form.model_whole && form.model.as_ref().is_some_and(|m| m.len() == 6));
    let k = *h.app.doc.components.keys().next().unwrap();
    assert!(
        h.app
            .model_shape(MESH_CELLS)
            .heights
            .iter()
            .all(|z| z.is_nan()),
        "the whole model is drawn in the component's place"
    );
    // over the limit: the component shows, and the model keeps only its red faces (none)
    h.app.options.import.preview_faces = 4;
    h.app.form.as_mut().unwrap().dirty = true;
    h.frame(vec![]);
    let form = h.app.form.as_ref().unwrap();
    assert!(!form.model_whole);
    assert_eq!(
        form.model.as_ref().map(|m| m.len()),
        Some(0),
        "a pyramid has no undercuts"
    );
    assert!(
        h.app
            .model_shape(MESH_CELLS)
            .heights
            .iter()
            .any(|z| !z.is_nan()),
        "the component is drawn instead"
    );
    assert!(h.app.doc.components[&k].visible);
    let _ = std::fs::remove_file(file);
}

#[test]
fn about_lists_the_github_contributors() {
    let listed = super::actions::contributors();
    assert!(!listed.is_empty(), "scripts/contributors has been run");
    assert!(listed.iter().any(|(login, ..)| *login == "NoobyNull"));
    assert!(
        listed
            .iter()
            .all(|(_, commits, url)| commits.parse::<u32>().is_ok()
                && url.starts_with("https://github.com/")),
        "{listed:?}"
    );
    let mut h = Harness::new();
    h.app.info = Some(Info::About);
    h.frame(vec![]);
    h.frame(vec![]);
    assert!(h.text_at("Contributors on GitHub").is_some());
    assert!(h.text_at("NoobyNull").is_some());
}

#[test]
fn about_thanks_vectric_and_says_cut_craft_is_independent() {
    let mut h = Harness::new();
    h.app.info = Some(Info::About);
    h.frame(vec![]);
    h.frame(vec![]);
    assert!(h.text_at("Acknowledgements").is_some());
    let thanks = super::actions::ACKNOWLEDGEMENT;
    assert!(thanks.contains("Vectric Aspire"), "{thanks}");
    assert!(thanks.contains("not affiliated with"), "{thanks}");
    assert!(thanks.contains("trademarks"), "{thanks}");
}

#[test]
fn importing_models_offers_what_the_options_say() {
    use crate::options::ModelTypes;
    let mut h = Harness::new();
    h.app.invoke(&h.ctx, "orientate_3d_model");
    let form = h.app.form.as_ref().unwrap();
    assert_eq!(form.params["offered"], "full", "full alone, by default");
    assert_eq!(form.params["model_type"], "full");
    h.app.form = None;
    h.app.options.import.model_types = ModelTypes::Flat;
    h.app.invoke(&h.ctx, "orientate_3d_model");
    assert_eq!(h.app.form.as_ref().unwrap().params["model_type"], "flat");
    h.app.form = None;
    h.app.options.import.model_types = ModelTypes::Both;
    h.app.invoke(&h.ctx, "orientate_3d_model");
    let form = h.app.form.as_ref().unwrap();
    assert_eq!(form.params["offered"], "both");
    assert_eq!(
        form.params["model_type"], "full",
        "the choice starts on Full"
    );
    h.frame(vec![]);
    h.frame(vec![]);
    assert!(h.text_at("Full 3D model").is_some(), "the choice is shown");
}

#[test]
fn edit_units_switches_the_job_between_millimetres_and_inches() {
    use cut_craft_core::document::Units;
    let mut h = Harness::new();
    assert_eq!(h.app.units_checked("units_mm"), Some(true));
    assert_eq!(h.app.units_checked("zoom_box"), None);
    h.app.cursor = Some(Point::new(25.4, 50.8));
    assert_eq!(h.app.coordinates().map(|(_, unit)| unit), Some("mm"));
    h.app.invoke(&h.ctx, "units_inch");
    assert_eq!(h.app.doc.job.units, Units::Inch);
    assert_eq!(h.app.units_checked("units_inch"), Some(true));
    assert_eq!(h.app.units_checked("units_mm"), Some(false));
    let (figures, unit) = h.app.coordinates().unwrap();
    assert_eq!((figures.as_str(), unit), ("X    1.000  Y    2.000", "in"));
    h.frame(vec![]);
    assert!(h.text_at("in").is_some(), "the status bar's unit button");
    h.app.invoke(&h.ctx, "undo");
    assert_eq!(h.app.doc.job.units, Units::Mm, "undoable");
}

#[test]
fn a_tools_figures_read_in_the_tools_own_units() {
    let mut h = Harness::new();
    h.app.invoke(&h.ctx, "pocket_toolpath");
    h.app.form.as_mut().unwrap().params["tool"]["units"] = Value::from("Inch");
    // the Toolpaths page flies out unseen on its first frame
    h.frame(vec![]);
    h.frame(vec![]);
    assert!(h.text_at("Pocket Toolpath").is_some(), "the form is drawn");
    let line = h.shapes.iter().any(|clipped| {
        matches!(&clipped.shape, egui::Shape::Text(t)
            if t.galley.text().starts_with("0.236 in end mill"))
    });
    assert!(line, "a 6 mm end mill reads in inches in a mm job");
}

#[test]
fn the_window_layout_menu_picks_and_ticks_a_layout() {
    use crate::options::WindowLayout;
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
fn the_rail_layout_lists_a_groups_tools_by_name() {
    let mut h = Harness::new();
    h.app.options.general.layout = crate::options::WindowLayout::Rail;
    h.frame(vec![]);
    assert!(h.text_at("Draw Circle").is_none());
    let groups = h.app.all_groups();
    let create = groups
        .iter()
        .position(|g| g.name == "Create Vectors")
        .unwrap();
    h.app.rail_group = Some(create);
    h.frame(vec![]);
    let at = h.text_at("Draw Circle").expect("the group's tools by name");
    h.frame(vec![Event::PointerMoved(at)]);
    h.press(at, true, Modifiers::NONE);
    h.press(at, false, Modifiers::NONE);
    h.frame(vec![]);
    assert_eq!(h.app.form.as_ref().map(|f| f.id), Some("draw_circle"));
    assert!(
        h.app.rail_group.is_none(),
        "an unpinned list closes on a pick"
    );
    assert!(
        h.text_at("Draw Circle").is_some(),
        "the form in the inspector"
    );
    assert!(h.text_at("Apply").is_some());
}

#[test]
fn the_ribbon_layout_shows_a_tabs_groups() {
    let mut h = Harness::new();
    h.app.options.general.layout = crate::options::WindowLayout::Ribbon;
    h.frame(vec![]);
    for name in ["Design", "Model", "Toolpaths", "Create Vectors", "Sheets"] {
        assert!(h.text_at(name).is_some(), "{name}");
    }
    assert!(h.text_at("Toolpath Operations").is_none());
    h.app.ribbon_tab = 2;
    h.frame(vec![]);
    assert!(h.text_at("Toolpath Operations").is_some());
    h.app.invoke(&h.ctx, "pocket_toolpath");
    h.frame(vec![]);
    assert!(
        h.text_at("Pocket Toolpath").is_some(),
        "the form's heading in the inspector"
    );
}

#[test]
fn notes_starting_with_a_dot_open_with_the_job() {
    let dir = std::env::temp_dir().join(format!("cut-craft-notes-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let mut h = Harness::new();
    for (notes, opens) in [(".Read me", true), ("quiet", false)] {
        h.app.doc.notes = notes.into();
        let path = dir.join("notes.ccjob");
        h.app.save_to(&path);
        h.app.form = None;
        assert!(h.app.open_path(&path));
        assert_eq!(
            h.app.form.as_ref().map(|f| f.id) == Some("notes"),
            opens,
            "{notes}"
        );
    }
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn drawing_snaps_to_nearby_corners() {
    let mut h = Harness::new();
    h.rectangle(10.0, 10.0, 20.0, 10.0);
    h.app.invoke(&h.ctx, "draw_circle");
    h.app.invoke(&h.ctx, "snap_options");
    assert!(h.app.snap_window);
    h.frame(vec![]);
    let scale = h.app.view_transform.unwrap().scale;
    let near = 2.0 / f64::from(scale); // two pixels off the corner
    let drag = Drag {
        start: Point::new(30.0 + near, 20.0 - near),
        end: Point::new(50.0, 50.0),
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
    assert_eq!(d.start, Point::new(30.0, 20.0), "onto the corner");
    assert_eq!(d.end, Point::new(50.0, 50.0), "nothing near");
}

#[test]
fn a_drawn_circle_snaps_its_edge_square_onto_a_side() {
    let mut h = Harness::new();
    h.rectangle(10.0, 10.0, 20.0, 40.0);
    h.app.invoke(&h.ctx, "draw_circle");
    h.frame(vec![]);
    let scale = h.app.view_transform.unwrap().scale;
    let near = 2.0 / f64::from(scale); // two pixels off the side
    let drag = Drag {
        start: Point::new(50.0, 22.0),
        end: Point::new(30.0 + near, 22.0 + near),
        from_center: true,
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
    assert_eq!(d.end, Point::new(30.0, 22.0), "the circle touches the side");
}

#[test]
fn a_polyline_click_snaps_to_an_end_marked_and_named_under_a_crosshair() {
    let mut h = Harness::new();
    h.rectangle(10.0, 10.0, 20.0, 10.0);
    h.app.invoke(&h.ctx, "draw_polyline");
    h.frame(vec![]);
    let scale = f64::from(h.app.view_transform.unwrap().scale);
    let near = 2.0 / scale; // two pixels off the corner
    let corner = h.at(30.0 + near, 20.0 - near);
    // hovering: the status bar names the snap, and the pointer is a crosshair
    h.time += 0.1;
    let input = RawInput {
        screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(1280.0, 800.0))),
        time: Some(h.time),
        events: vec![Event::PointerMoved(corner)],
        ..Default::default()
    };
    let out = headless(h.ctx.run_ui(input, |ui| h.app.pass(ui)));
    assert_eq!(out.platform_output.cursor_icon, egui::CursorIcon::Crosshair);
    let hint = cut_craft_bug_report::hint::current(&h.ctx).unwrap_or_default();
    assert!(
        hint.starts_with("Draw Polyline: click the first point"),
        "{hint}"
    );
    assert!(hint.contains("snapped to End"), "{hint}");
    h.click(corner, Modifiers::NONE);
    assert_eq!(h.app.polyline.points, vec![Point::new(30.0, 20.0)]);
    // Shift places it freely
    let other = h.at(10.0 + near, 10.0 + near);
    h.click(other, Modifiers::SHIFT);
    let last = *h.app.polyline.points.last().unwrap();
    assert!(last.distance(Point::new(10.0, 10.0)) > 1e-6, "{last:?}");
}

/// Moves vector `id` by `by`, as the Move tool would.
fn move_vector(h: &mut Harness, id: EntityId, by: Point) {
    h.app.doc.selection = cut_craft_core::selection::Selection::new();
    h.app.doc.selection.insert(id);
    let change = Change::transform_selected(&h.app.doc, |_| {
        cut_craft_core::glam::DAffine2::from_translation(by)
    });
    h.app.record(change, "Moved");
}

#[test]
fn a_line_drawn_to_a_snapped_corner_stays_on_it_until_its_mark_is_deleted() {
    let mut h = Harness::new();
    h.rectangle(10.0, 10.0, 20.0, 10.0);
    let square = *h.app.doc.vectors.keys().next().unwrap();
    h.app.invoke(&h.ctx, "draw_polyline");
    h.frame(vec![]);
    let scale = f64::from(h.app.view_transform.unwrap().scale);
    let near = 2.0 / scale;
    h.click(h.at(60.0, 60.0), Modifiers::SHIFT);
    h.click(h.at(30.0 + near, 20.0 - near), Modifiers::NONE);
    h.app.finish_polyline(false);
    h.app.form = None;
    h.frame(vec![]);
    let line = *h.app.doc.vectors.keys().last().unwrap();
    assert_eq!(h.app.doc.constraints.len(), 1, "the snapped end is kept");
    move_vector(&mut h, square, Point::new(5.0, 5.0));
    let end = h.app.doc.vectors[&line].segments[0].end();
    assert_eq!(end, Point::new(35.0, 25.0), "the end followed the corner");
    // its mark picked and deleted: the line stays, the snap goes
    h.frame(vec![]);
    h.click(h.at(35.0, 25.0), Modifiers::NONE);
    assert_eq!(h.app.picked_constraint, Some(0));
    h.app.invoke(&h.ctx, "delete");
    assert_eq!(h.app.doc.constraints, []);
    assert_eq!(h.app.doc.vectors.len(), 2, "nothing deleted");
    move_vector(&mut h, square, Point::new(5.0, 5.0));
    let end = h.app.doc.vectors[&line].segments[0].end();
    assert_eq!(
        end,
        Point::new(35.0, 25.0),
        "free now: it stays where it was"
    );
}

#[test]
fn a_rectangle_drawn_from_a_snapped_corner_moves_whole_with_it() {
    let mut h = Harness::new();
    h.rectangle(10.0, 10.0, 20.0, 10.0);
    let first = *h.app.doc.vectors.keys().next().unwrap();
    h.app.invoke(&h.ctx, "draw_rectangle");
    h.frame(vec![]);
    let scale = f64::from(h.app.view_transform.unwrap().scale);
    let near = 2.0 / scale;
    h.drag(h.at(30.0 + near, 20.0 - near), h.at(45.123, 37.456));
    h.app.form = None;
    h.frame(vec![]);
    let second = *h.app.doc.vectors.keys().last().unwrap();
    assert_ne!(first, second);
    let before = h.app.doc.vectors[&second].bounds();
    assert_eq!(
        h.app.doc.constraints.len(),
        1,
        "{:?}",
        h.app.doc.constraints
    );
    move_vector(&mut h, first, Point::new(-4.0, 3.0));
    let after = h.app.doc.vectors[&second].bounds();
    assert!((after.size() - before.size()).length() < 1e-9, "same size");
    let moved = after.min - before.min;
    assert!(moved.distance(Point::new(-4.0, 3.0)) < 1e-9, "{moved:?}");
}

#[test]
fn a_polylines_next_point_snaps_tangent_to_a_circle() {
    let mut h = Harness::new();
    let circle = serde_json::json!({ "center": { "x": 50.0, "y": 70.0 }, "radius": 10.0 });
    h.app.apply("draw_circle", &circle).unwrap();
    h.app.form = None;
    h.app.options.snapping.job_center_and_corners = false;
    h.app.invoke(&h.ctx, "draw_polyline");
    h.frame(vec![]);
    h.app.polyline.click(Point::new(70.0, 70.0));
    let scale = h.app.view_transform.unwrap().scale;
    let near = Point::new(55.3, 61.6);
    let mut found = None;
    let ctx = h.ctx.clone();
    headless(ctx.run_ui(RawInput::default(), |ui| {
        egui::CentralPanel::default().show(ui, |ui| {
            found = h.app.snapping(ui, None, scale, Some(near)).1;
        });
    }));
    let hit = found.expect("a snap");
    assert_eq!(hit.kind, cut_craft_core::snap::SnapKind::Tangent);
    let tangent = Point::new(55.0, 70.0 - 75.0_f64.sqrt());
    assert!(hit.at.distance(tangent) < 1e-6, "{hit:?}");
}

#[test]
fn a_freehand_stroke_does_not_snap_square_onto_a_side() {
    let mut h = Harness::new();
    h.rectangle(10.0, 10.0, 20.0, 40.0);
    h.app.invoke(&h.ctx, "draw_freehand");
    h.frame(vec![]);
    let scale = h.app.view_transform.unwrap().scale;
    let near = 2.0 / f64::from(scale);
    let drag = Drag {
        start: Point::new(50.0, 22.0),
        end: Point::new(30.0 + near, 22.0 + near),
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
    assert_ne!(
        d.end,
        Point::new(30.0, 22.0),
        "a stroke's samples snap as points"
    );
}

#[test]
fn clicking_a_component_in_the_2d_view_selects_it() {
    let mut h = Harness::new();
    h.app.doc = cut_craft_core::testing::domed();
    h.frame(vec![]);
    h.click(h.at(20.0, 20.0), Modifiers::NONE);
    assert_eq!(
        h.app.doc.selected_components.len(),
        1,
        "the dome under the click"
    );
    h.click(h.at(2.0, 2.0), Modifiers::NONE);
    assert!(h.app.doc.selected_components.is_empty(), "off the dome");
}

#[test]
fn freehand_follows_the_whole_drag() {
    let mut h = Harness::new();
    h.app.invoke(&h.ctx, "draw_freehand");
    let points = [(10.0, 10.0), (20.0, 30.0), (30.0, 10.0), (40.0, 30.0)];
    h.frame(vec![Event::PointerMoved(h.at(10.0, 10.0))]);
    h.press(h.at(10.0, 10.0), true, Modifiers::NONE);
    for (x, y) in points {
        h.frame(vec![Event::PointerMoved(h.at(x, y))]);
    }
    h.press(h.at(40.0, 30.0), false, Modifiers::NONE);
    h.frame(vec![]);
    let drawn = h.app.doc.vectors.values().next().expect("a stroke");
    let b = drawn.bounds();
    assert!(
        b.size().y > 15.0,
        "it went up and down, not straight: {b:?}"
    );
}

#[test]
fn a_sculpting_drag_deposits_along_the_stroke() {
    let mut h = Harness::new();
    h.app.doc = cut_craft_core::testing::domed();
    h.frame(vec![]);
    let top = |h: &Harness| {
        h.app
            .doc
            .components
            .values()
            .next()
            .unwrap()
            .height_at(Point::new(20.0, 20.0))
    };
    let before = top(&h).unwrap();
    h.app.invoke(&h.ctx, "sculpt");
    h.app.form.as_mut().unwrap().params["tool"] = Value::from("deposit");
    h.frame(vec![Event::PointerMoved(h.at(16.0, 20.0))]);
    h.press(h.at(16.0, 20.0), true, Modifiers::NONE);
    for x in [18.0, 20.0, 22.0, 24.0] {
        h.frame(vec![Event::PointerMoved(h.at(x, 20.0))]);
    }
    h.press(h.at(24.0, 20.0), false, Modifiers::NONE);
    h.frame(vec![]);
    assert!(top(&h).unwrap() > before, "the stroke raised the dome");
    assert!(h.app.doc.vectors.is_empty(), "sculpting draws no vectors");
}

/// Vendor tool databases are not part of the repo: `CUT_CRAFT_VENDOR_FILES` names a
/// folder holding some to try them.
#[test]
fn a_vendor_tool_database_imports_when_one_is_given() {
    let mut h = Harness::new();
    let before = h.app.tools.tools.len();
    h.app
        .import_tools(std::path::Path::new("/nonexistent.vtdb"));
    assert_eq!(h.app.tools.tools.len(), before, "nothing added on failure");
    let Some(dir) = std::env::var_os("CUT_CRAFT_VENDOR_FILES") else {
        return;
    };
    let Some(file) = std::fs::read_dir(dir).ok().and_then(|d| {
        d.flatten()
            .map(|e| e.path())
            .find(|p| p.extension().is_some_and(|x| x == "vtdb"))
    }) else {
        return;
    };
    h.app.import_tools(&file);
    assert!(h.app.tools.tools.len() > before + 10, "{}", file.display());
    let name = file.file_stem().unwrap().to_string_lossy();
    assert!(
        h.app
            .tools
            .tools
            .iter()
            .any(|t| t.group.starts_with(&*name)),
        "in a group named after the file"
    );
}

#[test]
fn node_editing_drags_a_corner_and_offers_its_menu() {
    let mut h = Harness::new();
    h.rectangle(10.0, 10.0, 20.0, 10.0);
    let id = *h.app.doc.selection.iter().next().unwrap();
    h.app.invoke(&h.ctx, "node_edit");
    assert_eq!(h.app.view_tool, Some(ViewTool::NodeEdit));
    // smart snapping would line the corner up with the other top corner
    h.app.options.snapping.smart_snapping = false;
    let (from, to) = (h.at(30.0, 20.0), h.at(40.0, 25.0));
    h.drag(from, to);
    let corners = cut_craft_core::nodes::nodes(&h.app.doc.vectors[&id]);
    assert!(
        corners
            .iter()
            .any(|q| q.distance(Point::new(40.0, 25.0)) < 0.5),
        "the corner followed the drag: {corners:?}"
    );
    assert_eq!(
        h.app.doc.vectors.len(),
        1,
        "a node drag selects nothing new"
    );
    let target = h
        .app
        .node_target(Point::new(10.0, 10.0), 1.0)
        .expect("a node there");
    let menu = gui::node_menu(&h.app.doc, &target);
    // Smooth Point turns the corner's spans into Béziers; Node Position opens Move Node there
    let smooth = leaf(&menu, &["Smooth Point"]);
    assert!(smooth.action.contains("smooth_point"), "{}", smooth.action);
    let position = leaf(&menu, &["Node Position..."]);
    assert!(position.action.contains("move_node"), "{}", position.action);
    let delete = leaf(&menu, &["Delete Point"]);
    h.app.invoke(&h.ctx, &delete.action);
    assert_eq!(
        cut_craft_core::nodes::nodes(&h.app.doc.vectors[&id]).len(),
        3
    );
    h.app.invoke(&h.ctx, "node_edit");
    assert_eq!(h.app.view_tool, None, "N toggles it off");
}

#[test]
fn a_template_opens_as_a_new_unsaved_job() {
    let dir = std::env::temp_dir().join(format!("cut-craft-template-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("plate.ccjobt");
    let mut h = Harness::new();
    h.rectangle(10.0, 10.0, 20.0, 10.0);
    std::fs::write(&path, cut_craft_core::file::save_job(&h.app.doc)).unwrap();
    let mut fresh = Harness::new();
    assert!(fresh.app.open_path(&path));
    assert_eq!(fresh.app.doc.vectors.len(), h.app.doc.vectors.len());
    assert_eq!(fresh.app.session.path, None, "saving asks for a new name");
    assert!(!fresh.app.recent.contains(&path));
    std::fs::remove_dir_all(dir).unwrap();
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
fn the_measure_tool_shows_its_results_in_a_form() {
    let mut h = Harness::new();
    h.app.invoke(&h.ctx, "measure_inspect");
    h.frame(vec![]);
    h.frame(vec![]);
    assert!(h.text_at("Measure / Inspect").is_some());
    h.drag(h.at(10.0, 10.0), h.at(40.0, 50.0));
    h.frame(vec![]);
    assert!(h.text_at("Distance").is_some());
    assert!(h.text_at("50.000 mm").is_some(), "3-4-5 times 10");
}

#[test]
fn times_read_as_hours_minutes_and_seconds() {
    assert_eq!(hms(0.0), "0:00:00");
    assert_eq!(hms(61.5), "1:01:30");
}

#[test]
fn help_items_open_their_windows() {
    let mut h = Harness::new();
    for (id, info) in [
        ("about_cut_craft", Info::About),
        ("keyboard_shortcuts", Info::Shortcuts),
        ("third_party_licenses", Info::Licenses),
        ("toolpath_summary", Info::Summary),
    ] {
        h.app.invoke(&h.ctx, id);
        assert_eq!(h.app.info, Some(info));
    }
}

#[test]
fn a_toolpath_preview_becomes_a_component() {
    let mut h = Harness::new();
    h.rectangle(10.0, 10.0, 50.0, 50.0);
    let params = h
        .app
        .registry
        .get("pocket_toolpath")
        .unwrap()
        .initial_params(&h.app.doc);
    h.app.apply("pocket_toolpath", &params).unwrap();
    assert!(!h.app.available("create_component_from_toolpath_preview"));
    h.app.invoke(&h.ctx, "preview_all_toolpaths");
    h.app
        .invoke(&h.ctx, "create_component_from_toolpath_preview");
    let made = h.app.doc.components.values().next().expect("a component");
    let thickness = h.app.doc.job.thickness;
    let (lo, hi) = made.height_range().unwrap();
    assert!((hi - thickness).abs() < 1e-9 && lo < hi, "{lo} {hi}");
}

#[test]
fn clicking_a_picture_selects_it_and_delete_removes_it() {
    use cut_craft_core::bitmap::Bitmap;
    use cut_craft_core::geometry::{Point, Rect};
    let mut h = Harness::new();
    let mut c = Change::builder(&h.app.doc);
    let rect = Rect::new(Point::new(20.0, 20.0), Point::new(120.0, 120.0));
    let id = c.add_bitmap(Bitmap {
        name: "b".into(),
        columns: 1,
        rows: 1,
        grey: vec![0],
        rect,
        depth: Vec::new(),
        rgb: Vec::new(),
    });
    h.app.record(Ok(c.finish()), "added");
    h.frame(vec![]);
    let at = h.at(70.0, 70.0);
    h.click(at, Modifiers::NONE);
    assert_eq!(h.app.doc.selected_bitmaps, [id].into());
    h.click(h.at(5.0, 5.0), Modifiers::NONE);
    assert!(
        h.app.doc.selected_bitmaps.is_empty(),
        "clicking elsewhere lets it go"
    );
    h.click(at, Modifiers::NONE);
    h.app.invoke(&h.ctx, "delete");
    assert!(h.app.doc.bitmaps.is_empty());
}

#[test]
fn sketch_carving_shows_the_areas_it_would_carve_while_its_form_is_open() {
    use cut_craft_core::bitmap::Bitmap;
    use cut_craft_core::geometry::{Point, Rect};
    let mut h = Harness::new();
    let mut c = Change::builder(&h.app.doc);
    // a white picture with a dark stripe down its middle column
    let grey = (0..9 * 9)
        .map(|i| if i % 9 == 4 { 0 } else { 255 })
        .collect();
    c.add_bitmap(Bitmap {
        name: "b".into(),
        columns: 9,
        rows: 9,
        grey,
        rect: Rect::new(Point::new(0.0, 0.0), Point::new(45.0, 45.0)),
        depth: Vec::new(),
        rgb: Vec::new(),
    });
    h.app.record(Ok(c.finish()), "added");
    h.app.invoke(&h.ctx, "sketch_carving_toolpath");
    h.frame(vec![]);
    let areas = &h.app.sketch_areas.as_ref().expect("found").areas;
    assert!(!areas.is_empty(), "the stripe shows");
    assert!(
        areas.iter().all(|r| r.min.x >= 15.0 && r.max.x <= 30.0),
        "{areas:?}"
    );
    h.app.form = None;
    h.frame(vec![]);
    assert!(h.app.sketch_areas.is_none(), "gone with the form");
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

/// Trace Bitmap opens its trace window, whose preview traces off the window's thread and
/// follows the settings as they change; Apply in it traces into the job.
#[test]
fn trace_bitmap_previews_in_its_window_and_follows_the_settings() {
    use cut_craft_core::bitmap::Bitmap;
    use cut_craft_core::geometry::{Point, Rect};
    let mut h = Harness::new();
    let mut c = Change::builder(&h.app.doc);
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
    h.app.record(Ok(c.finish()), "added");
    h.app.invoke(&h.ctx, "trace_bitmap");
    let settle = |h: &mut Harness, want: usize| {
        for _ in 0..400 {
            h.frame(vec![]);
            let view = h.app.trace_view.as_ref().unwrap();
            let current = view.shown_params() == h.app.form.as_ref().map(|f| &f.params);
            if current && view.traced_colours() == Some(want) {
                return;
            }
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        panic!(
            "the preview never showed {want} colours: {:?}",
            h.app.trace_view.as_ref().unwrap().traced_colours()
        );
    };
    // black and white: the two squares are dark enough at the threshold
    settle(&mut h, 1);
    h.app.form.as_mut().unwrap().params["mode"] = "colors".into();
    settle(&mut h, 2);
    // untick live: a change waits for Update preview
    h.app.trace_view.as_mut().unwrap().live = false;
    h.app.form.as_mut().unwrap().params["trace"] = serde_json::json!(["#c81414"]);
    for _ in 0..30 {
        h.frame(vec![]);
    }
    assert_eq!(h.app.trace_view.as_ref().unwrap().traced_colours(), Some(2));
    // Quick Trace in the window picks settings to suit the picture: colours, as it has some
    h.app.form.as_mut().unwrap().params["mode"] = "black_and_white".into();
    h.frame(vec![]);
    let quick = h
        .text_at("Quick Trace")
        .expect("the window's Quick Trace button");
    h.click(quick, Modifiers::NONE);
    h.frame(vec![]);
    assert_eq!(h.app.form.as_ref().unwrap().params["mode"], "colors");
    // closing the form lets the window go
    h.app.form = None;
    h.frame(vec![]);
    assert!(h.app.trace_view.is_none());
}

/// Relief from Photo follows its settings once applied, as depth2stl's preview did: one
/// relief, made again as a setting changes, never a pile of them.
#[test]
fn relief_from_photo_follows_its_settings_as_one_relief() {
    use cut_craft_core::bitmap::Bitmap;
    use cut_craft_core::geometry::{Point, Rect};
    let mut h = Harness::new();
    let mut c = Change::builder(&h.app.doc);
    c.add_bitmap(Bitmap {
        name: "map".into(),
        columns: 4,
        rows: 4,
        grey: vec![0, 0, 0, 0, 0, 120, 255, 0, 0, 180, 220, 0, 0, 0, 0, 0],
        rect: Rect::new(Point::new(0.0, 0.0), Point::new(40.0, 40.0)),
        depth: Vec::new(),
        rgb: Vec::new(),
    });
    h.app.record(Ok(c.finish()), "added");
    h.app.invoke(&h.ctx, "relief_from_photo");
    h.frame(vec![]);
    let form = h.app.form.as_mut().unwrap();
    form.params["depth_from"] = "picture".into();
    form.params["max_height"] = 10.0.into();
    let params = form.params.clone();
    h.app.apply_form("relief_from_photo", &params).unwrap();
    assert_eq!(h.app.doc.components.len(), 1);
    let top = |h: &Harness| {
        let k = h.app.doc.components.values().next().unwrap();
        k.height_range().unwrap().1
    };
    let before = top(&h);
    let form = h.app.form.as_mut().unwrap();
    form.params["max_height"] = 20.0.into();
    form.dirty = true;
    h.frame(vec![]);
    assert_eq!(h.app.doc.components.len(), 1, "made again in place");
    assert!(top(&h) > before + 5.0, "{before} -> {}", top(&h));
}

/// Estimate Depth with a stand-in add-on (a shell script that answers with a fixed depth
/// map): the photo gets its depth, undoably, and Relief from Photo can then use it.
#[cfg(unix)]
#[test]
fn estimate_depth_gives_the_photo_its_depth_from_the_add_on() {
    use cut_craft_core::bitmap::Bitmap;
    use cut_craft_core::geometry::{Point, Rect};
    use std::os::unix::fs::PermissionsExt;
    let addons = std::env::temp_dir().join(format!("cutcraft-addons-test-{}", std::process::id()));
    let dir = addons.join(crate::depth::ADDON_ID);
    std::fs::create_dir_all(&dir).unwrap();
    let answer = dir.join("answer.png");
    image::ImageBuffer::<image::Luma<u16>, _>::from_raw(2, 1, vec![0_u16, 65_535])
        .unwrap()
        .save(&answer)
        .unwrap();
    let program = dir.join(crate::depth::program_name());
    std::fs::write(
        &program,
        format!("#!/bin/sh\ncp '{}' \"$3\"\n", answer.display()),
    )
    .unwrap();
    std::fs::set_permissions(&program, std::fs::Permissions::from_mode(0o755)).unwrap();
    std::fs::write(dir.join(crate::depth::WEIGHTS), b"").unwrap();

    let mut h = Harness::new();
    h.app.addons = crate::addons::Addons::new(Some(addons.clone()));
    h.app.addons.ask_to_run = |_| true;
    let mut c = Change::builder(&h.app.doc);
    c.add_bitmap(Bitmap {
        name: "photo".into(),
        columns: 2,
        rows: 1,
        grey: vec![30, 220],
        rect: Rect::new(Point::new(0.0, 0.0), Point::new(20.0, 10.0)),
        depth: Vec::new(),
        rgb: Vec::new(),
    });
    h.app.record(Ok(c.finish()), "added");
    h.app.invoke(&h.ctx, "relief_from_photo");
    h.frame(vec![]);
    let id = *h.app.doc.bitmaps.keys().next().unwrap();
    let installed = crate::depth::find(&addons).expect("installed");
    let photo = h.app.doc.bitmaps[&id].clone();
    h.app.depth_estimate =
        Some(crate::depth::Estimating::start(&installed, id, &photo, &h.ctx).unwrap());
    for _ in 0..200 {
        if h.app.depth_estimate.is_none() {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(20));
        h.frame(vec![]);
    }
    assert!(h.app.depth_estimate.is_none(), "the add-on answered");
    assert_eq!(h.app.doc.bitmaps[&id].depth, [0.0, 1.0]);
    assert!(h.app.history.undo(&mut h.app.doc));
    assert!(h.app.doc.bitmaps[&id].depth.is_empty(), "undone");
    // an answer for a picture that has changed since (another job's, sharing its id) is
    // turned away
    let mut other = photo.clone();
    other.grey = vec![90, 90];
    let mut c = Change::builder(&h.app.doc);
    c.replace_bitmap(id, other);
    h.app.record(Ok(c.finish()), "edited");
    h.app.depth_estimate =
        Some(crate::depth::Estimating::start(&installed, id, &photo, &h.ctx).unwrap());
    for _ in 0..200 {
        if h.app.depth_estimate.is_none() {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(20));
        h.frame(vec![]);
    }
    assert!(h.app.depth_estimate.is_none(), "the add-on answered");
    assert!(
        h.app.doc.bitmaps[&id].depth.is_empty(),
        "not given to another picture"
    );
    std::fs::remove_dir_all(addons).unwrap();
}

#[test]
fn clicking_one_of_a_group_picks_the_group() {
    let mut h = Harness::new();
    h.rectangle(10.0, 10.0, 20.0, 20.0);
    h.rectangle(60.0, 10.0, 20.0, 20.0);
    h.app.invoke(&h.ctx, "select_all_vectors");
    h.app.invoke(&h.ctx, "group");
    h.click(h.at(5.0, 5.0), Modifiers::NONE);
    assert!(h.app.doc.selection.is_empty());
    h.click(h.at(10.0, 20.0), Modifiers::NONE);
    assert_eq!(h.app.doc.selection.len(), 2, "the edge of one picks both");
    h.click(h.at(10.0, 20.0), Modifiers::SHIFT);
    assert!(
        h.app.doc.selection.is_empty(),
        "Shift-click takes the group away"
    );
    h.click(h.at(10.0, 20.0), Modifiers::NONE);
    h.app.invoke(&h.ctx, "ungroup");
    h.click(h.at(10.0, 20.0), Modifiers::NONE);
    assert_eq!(h.app.doc.selection.len(), 1, "ungrouped, one at a time");
}

/// A headless frame's output: no renderer takes its texture changes, so they are let go.
fn headless(mut out: egui::FullOutput) -> egui::FullOutput {
    out.textures_delta.clear();
    out
}

/// A file dropped on the window, as the windowing layer reports one.
#[derive(Debug)]
struct Dropped(std::path::PathBuf);

impl egui::DroppedFile for Dropped {
    fn path(&self) -> &std::path::Path {
        &self.0
    }

    fn bytes(&self) -> Result<Vec<u8>, String> {
        std::fs::read(&self.0).map_err(|e| e.to_string())
    }
}

struct Harness {
    ctx: egui::Context,
    app: CutCraftApp,
    time: f64,
    /// What the last frame drew.
    shapes: Vec<egui::epaint::ClippedShape>,
    /// What the last frame put on the system clipboard.
    copied: String,
    /// The last frame's request for the window's picture, if it made one.
    screenshot: Option<egui::ScreenshotCallback>,
    /// The window's size.
    window: egui::Vec2,
}

impl Harness {
    /// The app as it starts: the start page, no job open.
    fn at_start() -> Self {
        let mut h = Self {
            ctx: egui::Context::default(),
            app: CutCraftApp::new(),
            time: 0.0,
            shapes: vec![],
            copied: String::new(),
            screenshot: None,
            window: vec2(1280.0, 800.0),
        };
        // previews appear cut at once unless a test plays one
        h.app.display.animate_preview = false;
        // slow jobs finish in place, as quick ones do, unless a test waits on one
        h.app.wait_before_showing = std::time::Duration::from_mins(10);
        // no browser opens from a test
        h.app.reporter.browser = false;
        // nor a question about crossing vectors: the controls are clicked with whatever is
        // selected
        let checks = &mut h.app.options.confirmations;
        checks.toolpath_intersections = crate::options::IntersectionCheck::DontCheck;
        checks.radiused_offset_intersections = crate::options::IntersectionCheck::DontCheck;
        checks.sharp_offset_intersections = crate::options::IntersectionCheck::DontCheck;
        h.frame(vec![]);
        h.frame(vec![]);
        h
    }

    /// A new job made the way a user makes one: New, then OK in Job Setup.
    fn new() -> Self {
        let mut h = Self::at_start();
        h.app.invoke(&h.ctx, "new");
        h.app.job_setup_ok();
        h.frame(vec![]);
        h.frame(vec![]);
        h
    }

    fn frame(&mut self, events: Vec<Event>) {
        self.frame_holding(Modifiers::NONE, events);
    }

    /// One frame with `files` dropped on the window (or only dragged over it).
    fn drop_files(&mut self, files: &[std::path::PathBuf], dropped: bool) {
        self.time += 0.1;
        let input = RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(1280.0, 800.0))),
            time: Some(self.time),
            dropped_files: if dropped {
                files
                    .iter()
                    .map(|p| std::sync::Arc::new(Dropped(p.clone())) as egui::DroppedFileHandle)
                    .collect()
            } else {
                vec![]
            },
            hovered_files: if dropped {
                vec![]
            } else {
                files
                    .iter()
                    .map(|p| egui::HoveredFile {
                        path: Some(p.clone()),
                        ..Default::default()
                    })
                    .collect()
            },
            ..Default::default()
        };
        self.shapes = headless(self.ctx.run_ui(input, |ui| self.app.pass(ui))).shapes;
    }

    /// One frame with `modifiers` held, as the windowing layer reports them every frame.
    fn frame_holding(&mut self, modifiers: Modifiers, events: Vec<Event>) {
        self.time += 0.1;
        let mut events = events;
        events.insert(0, Event::ModifiersChanged(modifiers));
        let input = RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, self.window)),
            time: Some(self.time),
            events,
            ..Default::default()
        };
        let out = headless(self.ctx.run_ui(input, |ui| self.app.pass(ui)));
        self.shapes = out.shapes;
        self.copied = out
            .platform_output
            .commands
            .iter()
            .find_map(|c| match c {
                egui::OutputCommand::CopyText(text) => Some(text.clone()),
                _ => None,
            })
            .unwrap_or_default();
        self.screenshot = out
            .viewport_output
            .values()
            .flat_map(|v| &v.commands)
            .find_map(|c| match c {
                egui::ViewportCommand::Screenshot(callback) => Some(callback.clone()),
                _ => None,
            });
    }

    /// Every place the last frame drew `text`.
    fn texts_at(&self, text: &str) -> Vec<Pos2> {
        let drawn = self
            .shapes
            .iter()
            .filter_map(|clipped| match &clipped.shape {
                egui::Shape::Text(t) if t.galley.text() == text => {
                    Some(t.pos + t.galley.size() / 2.0)
                }
                _ => None,
            });
        drawn.collect()
    }

    /// The middle of `text` once the panel under `over` is wheeled down until it is in the
    /// window; `None` if it never is.
    fn scroll_to(&mut self, over: Pos2, text: &str) -> Option<Pos2> {
        for _ in 0..40 {
            if let Some(at) = self.text_at(text).filter(|at| at.y < 780.0) {
                return Some(at);
            }
            self.frame(vec![
                Event::PointerMoved(over),
                Event::MouseWheel {
                    unit: egui::MouseWheelUnit::Point,
                    delta: vec2(0.0, -100.0),
                    phase: egui::TouchPhase::Move,
                    source: egui::MouseWheelSource::Wheel,
                    modifiers: Modifiers::NONE,
                },
            ]);
            self.frame(vec![]);
        }
        None
    }

    /// The open form's heading `text`: the rightmost place it is drawn, as the Design
    /// page's tip box may name the tool too.
    fn form_heading(&self, text: &str) -> Pos2 {
        self.texts_at(text)
            .into_iter()
            .max_by(|a, b| a.x.total_cmp(&b.x))
            .unwrap_or_else(|| panic!("{text}"))
    }

    /// Every text the last frame drew.
    fn texts(&self) -> Vec<String> {
        (self.shapes.iter())
            .filter_map(|clipped| match &clipped.shape {
                egui::Shape::Text(t) => Some(t.galley.text().to_owned()),
                _ => None,
            })
            .collect()
    }

    /// The middle of `text` where the last frame drew it.
    fn text_at(&self, text: &str) -> Option<Pos2> {
        self.shapes.iter().find_map(|clipped| match &clipped.shape {
            egui::Shape::Text(t) if t.galley.text() == text => Some(t.pos + t.galley.size() / 2.0),
            _ => None,
        })
    }

    fn right_click(&mut self, at: Pos2) {
        self.frame(vec![Event::PointerMoved(at)]);
        for pressed in [true, false] {
            let button = Event::PointerButton {
                pos: at,
                button: PointerButton::Secondary,
                pressed,
                modifiers: Modifiers::NONE,
            };
            self.frame(vec![button]);
        }
        self.frame(vec![]);
    }

    /// Screen position of a job coordinate in the 2D view.
    fn at(&self, x: f64, y: f64) -> Pos2 {
        self.app
            .view_transform
            .expect("the 2D view has been drawn")
            .to_screen(Point::new(x, y))
    }

    fn press(&mut self, pos: Pos2, pressed: bool, modifiers: Modifiers) {
        let button = Event::PointerButton {
            pos,
            button: PointerButton::Primary,
            pressed,
            modifiers,
        };
        self.frame_holding(modifiers, vec![button]);
    }

    fn drag(&mut self, from: Pos2, to: Pos2) {
        self.frame(vec![Event::PointerMoved(from)]);
        self.press(from, true, Modifiers::NONE);
        self.frame(vec![Event::PointerMoved(from.lerp(to, 0.5))]);
        self.frame(vec![Event::PointerMoved(to)]);
        self.press(to, false, Modifiers::NONE);
        self.frame(vec![]);
    }

    /// A click with `modifiers` held for the whole gesture, as a held key would be.
    fn click(&mut self, at: Pos2, modifiers: Modifiers) {
        self.frame_holding(modifiers, vec![Event::PointerMoved(at)]);
        self.press(at, true, modifiers);
        self.press(at, false, modifiers);
        self.frame_holding(modifiers, vec![]);
    }

    fn double_click(&mut self, at: Pos2) {
        self.frame(vec![Event::PointerMoved(at)]);
        for pressed in [true, false, true, false] {
            self.press(at, pressed, Modifiers::NONE);
        }
        self.frame(vec![]);
    }

    /// Clicks the item at `path` (names, top down) of the right-click menu `menu` builds.
    fn choose(&mut self, menu: impl Fn(&CutCraftApp) -> Vec<gui::MenuItem>, path: &[&str]) {
        let items = menu(&self.app);
        let item = leaf(&items, path);
        assert!(
            item.is_enabled && self.app.available(&item.action),
            "{path:?} is greyed"
        );
        self.app.invoke(&self.ctx, &item.action);
    }

    /// Draws a rectangle through the command, then closes its form.
    fn rectangle(&mut self, x: f64, y: f64, width: f64, height: f64) {
        let params = serde_json::json!({ "x": x, "y": y, "width": width, "height": height, "corners": "square", "radius": 0.0 });
        self.app.apply("draw_rectangle", &params).unwrap();
        self.app.form = None;
        self.frame(vec![]);
    }
}

/// The item at `path` (names, top down) of a menu.
fn leaf<'a>(menu: &'a [gui::MenuItem], path: &[&str]) -> &'a gui::MenuItem {
    let (name, rest) = path.split_first().expect("a path to an item");
    let item = menu
        .iter()
        .find(|i| i.name == *name)
        .unwrap_or_else(|| panic!("no {name} in the menu"));
    if rest.is_empty() {
        item
    } else {
        leaf(&item.children, rest)
    }
}

fn near(a: Point, b: Point) -> bool {
    // one screen pixel is under a millimetre at this window size
    a.distance(b) < 2.0
}

#[test]
fn dragging_with_the_rectangle_tool_draws_that_rectangle() {
    let mut h = Harness::new();
    h.app.invoke(&h.ctx, "draw_rectangle");
    h.frame(vec![]);
    let (from, to) = (h.at(100.0, 100.0), h.at(300.0, 200.0));
    h.drag(from, to);
    assert_eq!(h.app.doc.vectors.len(), 1);
    let b = h.app.doc.vectors.values().next().unwrap().bounds();
    assert!(
        near(b.min, Point::new(100.0, 100.0)) && near(b.max, Point::new(300.0, 200.0)),
        "{b:?}"
    );
    assert!(h.app.history.can_undo());
    let width = h.app.form.as_ref().unwrap().params["width"]
        .as_f64()
        .unwrap();
    assert!(
        (width - 200.0).abs() < 2.0,
        "the form shows the dragged size, got {width}"
    );
}

#[test]
fn clicking_selects_without_adding_undo_steps() {
    let mut h = Harness::new();
    h.rectangle(10.0, 10.0, 50.0, 50.0);
    h.click(h.at(500.0, 300.0), Modifiers::NONE);
    assert!(
        h.app.doc.selection.is_empty(),
        "clicking empty space clears the selection"
    );
    h.click(h.at(60.0, 35.0), Modifiers::NONE);
    assert_eq!(
        h.app.doc.selection.len(),
        1,
        "clicking the right edge selects it"
    );
    assert!(h.app.history.undo(&mut h.app.doc));
    assert!(
        h.app.doc.vectors.is_empty(),
        "the last undo step is the draw, not a click"
    );
}

#[test]
fn clicking_inside_a_shape_selects_it_with_its_holes() {
    let mut h = Harness::new();
    h.rectangle(10.0, 10.0, 100.0, 100.0);
    h.rectangle(20.0, 20.0, 20.0, 20.0);
    h.rectangle(150.0, 10.0, 50.0, 50.0);
    h.click(h.at(80.0, 80.0), Modifiers::NONE);
    assert_eq!(
        h.app.doc.selection.len(),
        2,
        "the outline and its hole, not the shape beside them"
    );
    h.click(h.at(175.0, 35.0), Modifiers::SHIFT);
    assert_eq!(h.app.doc.selection.len(), 3, "Shift adds the shape beside");
    h.click(h.at(80.0, 80.0), Modifiers::SHIFT);
    assert_eq!(
        h.app.doc.selection.len(),
        1,
        "Shift again takes the part away"
    );
    h.click(h.at(130.0, 300.0), Modifiers::NONE);
    assert!(h.app.doc.selection.is_empty(), "outside every shape clears");
}

#[test]
fn select_vectors_inside_adds_a_selected_shapes_holes() {
    let mut h = Harness::new();
    h.rectangle(10.0, 10.0, 100.0, 100.0);
    h.rectangle(20.0, 20.0, 20.0, 20.0);
    h.rectangle(150.0, 10.0, 50.0, 50.0);
    h.click(h.at(110.0, 60.0), Modifiers::NONE);
    assert_eq!(h.app.doc.selection.len(), 1, "the outline's edge alone");
    let menu = |app: &CutCraftApp| app.menus.selection_menu(&app.doc);
    h.choose(menu, &["Select Vectors Inside"]);
    assert_eq!(h.app.doc.selection.len(), 2, "with its hole");
    h.app.invoke(&h.ctx, "unselect_all");
    h.app.invoke(&h.ctx, "select_vectors_inside");
    assert!(h.app.doc.selection.is_empty());
    assert!(
        h.app.status.message.contains("closed shape"),
        "{}",
        h.app.status.message
    );
}

#[test]
fn shift_click_adds_and_removes() {
    let mut h = Harness::new();
    h.rectangle(10.0, 10.0, 50.0, 50.0);
    h.rectangle(100.0, 10.0, 50.0, 50.0);
    h.click(h.at(10.0, 35.0), Modifiers::NONE);
    h.click(h.at(150.0, 35.0), Modifiers::SHIFT);
    assert_eq!(h.app.doc.selection.len(), 2);
    h.click(h.at(10.0, 35.0), Modifiers::SHIFT);
    assert_eq!(h.app.doc.selection.len(), 1);
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
fn forms_open_with_values_from_the_document() {
    let mut h = Harness::new();
    h.rectangle(10.0, 10.0, 50.0, 20.0);
    h.app.invoke(&h.ctx, "set_size");
    let params = &h.app.form.as_ref().unwrap().params;
    assert_eq!(
        (params["width"].as_f64(), params["height"].as_f64()),
        (Some(50.0), Some(20.0))
    );
}

fn cut_min_x(app: &CutCraftApp) -> f64 {
    cut_craft_core::testing::cut_bounds(app.doc.toolpaths.values().next().unwrap())
        .min
        .x
}

#[test]
fn guides_come_out_of_the_rulers_move_and_go_back_in() {
    let mut h = Harness::new();
    h.frame(vec![]);
    let area = h.app.view_area.unwrap();
    let ruler = h.ctx.global_style().spacing.interact_size.y / 2.0;
    let top_ruler = egui::pos2(area.center().x, area.min.y + ruler);
    h.drag(top_ruler, h.at(100.0, 80.0));
    assert_eq!(h.app.doc.guides.len(), 1, "a guide across the job");
    let g = h.app.doc.guides[0];
    assert!(!g.vertical && (g.at - 80.0).abs() < 0.5, "{g:?}");
    h.drag(h.at(150.0, 80.0), h.at(150.0, 60.0));
    assert!((h.app.doc.guides[0].at - 60.0).abs() < 0.5, "dragged along");
    h.app.invoke(&h.ctx, "lock_all_existing_guides");
    h.drag(h.at(150.0, 60.0), h.at(150.0, 40.0));
    assert!(
        (h.app.doc.guides[0].at - 60.0).abs() < 0.5,
        "a locked guide stays"
    );
    h.app.invoke(&h.ctx, "unlock_all_existing_guides");
    h.drag(h.at(150.0, 60.0), top_ruler);
    assert!(
        h.app.doc.guides.is_empty(),
        "back onto the ruler deletes it"
    );
    h.app.doc.guides = vec![cut_craft_core::document::Guide::vertical(5.0)];
    h.app.invoke(&h.ctx, "delete_all_guides");
    assert_eq!(h.app.doc.guides, [] as [cut_craft_core::document::Guide; 0]);
}

#[test]
fn transform_mode_scales_by_a_corner_and_rotates_after_a_second_click() {
    let mut h = Harness::new();
    h.rectangle(10.0, 10.0, 20.0, 10.0);
    h.app.invoke(&h.ctx, "transform");
    assert_eq!(h.app.view_tool, Some(ViewTool::Transform));
    let (from, to) = (h.at(30.0, 20.0), h.at(50.0, 30.0));
    h.drag(from, to);
    let b = h.app.doc.selection_bounds().unwrap();
    assert!(
        (b.max.x - 50.0).abs() < 0.5 && (b.max.y - 30.0).abs() < 0.5,
        "{b:?}"
    );
    assert!((b.min.x - 10.0).abs() < 1e-6, "the opposite corner stays");
    assert!(!h.app.rotating);
    let middle = h.at(b.center().x, b.center().y);
    h.click(middle, Modifiers::NONE);
    assert!(
        h.app.rotating,
        "a second click on the middle turns rotation on"
    );
}

#[test]
fn a_selection_always_has_its_handles_to_scale_and_rotate_by() {
    let mut h = Harness::new();
    h.rectangle(10.0, 10.0, 20.0, 10.0);
    // no tool in hand: plain selecting
    h.app.form = None;
    h.frame(vec![]);
    assert_eq!(h.app.view_tool, None);
    assert!(h.app.drawing_tool().is_none());
    // a side handle stretches one way only
    h.drag(h.at(30.0, 15.0), h.at(40.0, 15.0));
    let b = h.app.doc.selection_bounds().unwrap();
    assert!(
        (b.max.x - 40.0).abs() < 0.5 && (b.max.y - 20.0).abs() < 1e-6,
        "{b:?}"
    );
    // a corner freely, each axis on its own
    h.drag(h.at(40.0, 20.0), h.at(50.0, 30.0));
    let b = h.app.doc.selection_bounds().unwrap();
    assert!(
        (b.max.x - 50.0).abs() < 0.5 && (b.max.y - 30.0).abs() < 0.5,
        "{b:?}"
    );
    // with Shift in proportion
    let (from, to) = (h.at(50.0, 30.0), h.at(70.0, 32.0));
    h.frame(vec![Event::PointerMoved(from)]);
    h.press(from, true, Modifiers::SHIFT);
    h.frame_holding(Modifiers::SHIFT, vec![Event::PointerMoved(to)]);
    h.press(to, false, Modifiers::SHIFT);
    let b = h.app.doc.selection_bounds().unwrap();
    let size = b.size();
    assert!(
        size.x > 41.0 && (size.x / size.y - 2.0).abs() < 0.01,
        "{b:?}"
    );
    // the rotation handles are there too, a little beyond the corners
    let row = h.ctx.global_style().spacing.interact_size.y;
    let out = 2.5 * row / 2.0;
    let corner = h.at(b.max.x, b.max.y);
    let handle = corner + egui::vec2(out, -out);
    let centre = h.at(b.center().x, b.center().y);
    // over it, the status bar says the keys
    h.frame(vec![Event::PointerMoved(handle)]);
    let hint = cut_craft_bug_report::hint::current(&h.ctx).unwrap_or_default();
    assert!(
        hint.contains("Hold Shift to snap to 22.5°, or Ctrl to 1°"),
        "{hint}"
    );
    // a quarter turn about the middle
    let spun = centre + (handle - centre).rot90();
    h.drag(handle, spun);
    let turned = h.app.doc.selection_bounds().unwrap().size();
    assert!(
        (turned.x - size.y).abs() < 0.5 && (turned.y - size.x).abs() < 0.5,
        "{size:?} turned to {turned:?}"
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
        .filter(|id| crate::gui::about(id).is_none_or(str::is_empty))
        .collect();
    assert!(missing.is_empty(), "no description for {missing:?}");
    assert!(
        crate::gui::about("pocket_toolpath")
            .unwrap()
            .starts_with("Pocket Toolpath: clears")
    );
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
        refused.starts_with(super::actions::KEEP_OUT_REFUSED),
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
    }];
    assert!(h.app.sync_machine_keep_out());
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
    assert!(refused.contains("keep-out zones"), "{refused}");
    assert!(!path.exists(), "nothing written");
}

#[test]
fn toolpaths_follow_their_vectors_when_recalculated() {
    let mut h = Harness::new();
    h.rectangle(10.0, 10.0, 50.0, 50.0);
    h.app.invoke(&h.ctx, "profile_toolpath");
    let params = h.app.form.as_ref().unwrap().params.clone();
    h.app.apply("profile_toolpath", &params).unwrap();
    let before = cut_min_x(&h.app);
    let shift =
        serde_json::json!({ "relative": true, "x": 100.0, "y": 0.0, "anchor": "bottom_left" });
    h.app.apply("move", &shift).unwrap();
    h.app.invoke(&h.ctx, "recalculate_all_toolpaths");
    assert!((cut_min_x(&h.app) - before - 100.0).abs() < 1e-6);
    h.app.invoke(&h.ctx, "undo");
    assert!(
        (cut_min_x(&h.app) - before).abs() < 1e-6,
        "recalculating is one undo step"
    );
}

#[test]
fn hiding_a_layer_from_the_panel_is_undoable_and_stops_picking() {
    let mut h = Harness::new();
    h.app.design_tab = DESIGN_TABS.iter().position(|&t| t == "Layers").unwrap();
    h.rectangle(10.0, 10.0, 50.0, 50.0);
    h.app.layer_action(&h.ctx, LayerId(1), "toggle_visible");
    h.frame(vec![]);
    h.click(h.at(60.0, 35.0), Modifiers::NONE);
    assert!(h.app.doc.selection.is_empty());
    h.app.invoke(&h.ctx, "undo");
    assert!(h.app.doc.layers[0].visible);
}

#[test]
fn choosing_a_layer_makes_it_current_without_an_undo_step() {
    let mut h = Harness::new();
    h.app
        .apply("add_layer", &serde_json::json!({ "name": "Top" }))
        .unwrap();
    h.app.layer_action(&h.ctx, LayerId(1), "set_current_layer");
    assert_eq!(h.app.doc.current_layer, LayerId(1));
    h.app.invoke(&h.ctx, "undo");
    assert_eq!(
        h.app.doc.layers.len(),
        1,
        "the last undo step added the layer"
    );
}

#[test]
fn the_layer_menu_acts_on_the_layer_clicked() {
    let mut h = Harness::new();
    h.rectangle(10.0, 10.0, 50.0, 50.0);
    h.app
        .apply("add_layer", &serde_json::json!({ "name": "Top" }))
        .unwrap();
    let layer = |i: usize| move |app: &CutCraftApp| gui::layer_menu(&app.doc, &app.doc.layers[i]);
    h.choose(layer(1), &["Move Selection Here"]);
    assert!(
        h.app
            .doc
            .vectors
            .keys()
            .all(|&id| h.app.doc.layer_of(id) == LayerId(2))
    );
    h.choose(layer(0), &["Activate"]);
    assert_eq!(h.app.doc.current_layer, LayerId(1));
    h.choose(layer(0), &["Show", "Only This"]);
    let visible: Vec<bool> = h.app.doc.layers.iter().map(|l| l.visible).collect();
    assert_eq!(visible, [true, false]);
    h.choose(layer(0), &["Lock"]);
    let locked = layer(0)(&h.app);
    assert!(!leaf(&locked, &["Lock"]).is_enabled && leaf(&locked, &["Unlock"]).is_enabled);
    h.choose(layer(1), &["Rename"]);
    let form = h.app.form.take().unwrap();
    assert_eq!(
        (form.id, form.params["name"].as_str()),
        ("layer_properties", Some("Top"))
    );
    h.choose(layer(0), &["Delete", "Empty"]);
    let names: Vec<&str> = h.app.doc.layers.iter().map(|l| l.name.as_str()).collect();
    assert_eq!(names, ["Top"], "the first layer had nothing left on it");
}

#[test]
fn the_toolpath_menu_acts_on_the_toolpath_clicked() {
    let mut h = Harness::new();
    h.rectangle(10.0, 10.0, 50.0, 50.0);
    for _ in 0..2 {
        let params = h.app.registry.get("profile_toolpath").unwrap();
        let params = params.initial_params(&h.app.doc);
        h.app.apply("profile_toolpath", &params).unwrap();
    }
    let toolpath = |i: usize| {
        move |app: &CutCraftApp| {
            let (&id, t) = app.doc.toolpaths.iter().nth(i).unwrap();
            gui::toolpath_menu(&app.doc, id, t)
        }
    };
    let min_x = |app: &CutCraftApp| -> Vec<f64> {
        let bounds = app.doc.toolpaths.values();
        bounds
            .map(|t| cut_craft_core::testing::cut_bounds(t).min.x)
            .collect()
    };
    h.choose(toolpath(0), &["Hide", "This"]);
    let visible: Vec<bool> = h.app.doc.toolpaths.values().map(|t| t.visible).collect();
    assert_eq!(visible, [false, true]);
    let before = min_x(&h.app);
    let shift =
        serde_json::json!({ "relative": true, "x": 100.0, "y": 0.0, "anchor": "bottom_left" });
    h.app.apply("move", &shift).unwrap();
    h.choose(toolpath(1), &["Recalculate", "Visible"]);
    let after = min_x(&h.app);
    assert!(
        (after[0] - before[0]).abs() < 1e-6 && (after[1] - before[1] - 100.0).abs() < 1e-6,
        "only the shown toolpath follows its vectors: {before:?} → {after:?}"
    );
    let second = *h.app.doc.toolpaths.keys().nth(1).unwrap();
    h.choose(toolpath(1), &["Rename"]);
    let form = h.app.form.take().unwrap();
    assert_eq!(form.id, "toolpath_properties");
    assert_eq!(form.params["toolpath"], serde_json::json!(second));
    h.choose(toolpath(1), &["Delete", "Invisible"]);
    assert_eq!(
        h.app.doc.toolpaths.keys().copied().collect::<Vec<_>>(),
        [second]
    );
}

#[test]
fn the_level_menu_mirrors_and_components_move_between_levels() {
    use cut_craft_core::component::Mirror;
    let mut h = Harness::new();
    h.app.doc = cut_craft_core::testing::domed();
    let params = h.app.registry.get("insert_level").unwrap();
    let params = params.initial_params(&h.app.doc);
    h.app.apply("insert_level", &params).unwrap();
    let level_menu = |app: &CutCraftApp| {
        let (&id, level) = app.doc.levels.iter().next().unwrap();
        gui::level_menu(&app.doc, id, level)
    };
    h.choose(level_menu, &["Mirror Mode", "Left to Right"]);
    assert_eq!(h.app.doc.levels[&0].mirror, Mirror::LeftToRight);
    assert!(
        !leaf(&level_menu(&h.app), &["Wrapping"]).is_enabled,
        "flat job"
    );
    h.app.doc.selected_components = [EntityId(1)].into();
    let menu = |app: &CutCraftApp| {
        let (&id, component) = app.doc.components.iter().next().unwrap();
        gui::component_menu(&app.doc, id, component)
    };
    h.choose(menu, &["Move to Level", "Level 2"]);
    assert_eq!(h.app.doc.components[&EntityId(1)].level, 1);
    assert!(leaf(&menu(&h.app), &["Move to Level", "Level 2"]).is_checked);
}

#[test]
fn the_level_menu_selects_its_components_and_exports_them() {
    let mut h = Harness::new();
    h.app.doc = cut_craft_core::testing::two_components();
    let level_menu = |app: &CutCraftApp| {
        let (&id, level) = app.doc.levels.iter().next().unwrap();
        gui::level_menu(&app.doc, id, level)
    };
    h.choose(level_menu, &["Select Components"]);
    let all: Vec<EntityId> = h.app.doc.components.keys().copied().collect();
    assert_eq!(
        h.app
            .doc
            .selected_components
            .iter()
            .copied()
            .collect::<Vec<_>>(),
        all
    );
    let menu = level_menu(&h.app);
    let export = leaf(&menu, &["Export as 3D Clipart"]);
    assert!(export.is_enabled && h.app.available(&export.action));
}

#[test]
fn the_component_menu_acts_on_the_component_clicked() {
    use cut_craft_core::component::Combine;
    let mut h = Harness::new();
    h.rectangle(10.0, 10.0, 50.0, 50.0);
    let params = h.app.registry.get("create_shape").unwrap();
    let params = params.initial_params(&h.app.doc);
    h.app.apply("create_shape", &params).unwrap();
    let menu = |app: &CutCraftApp| {
        let (&id, component) = app.doc.components.iter().next().unwrap();
        gui::component_menu(&app.doc, id, component)
    };
    let component = |app: &CutCraftApp| app.doc.components.values().next().unwrap().clone();
    assert!(leaf(&menu(&h.app), &["Combine Mode", "Add"]).is_checked);
    assert_eq!(
        leaf(&menu(&h.app), &["Combine Mode", "Add"])
            .icon
            .as_deref(),
        Some("modeling/combine_add.png"),
        "Aspire's combine icons"
    );
    h.choose(menu, &["Combine Mode", "Subtract"]);
    assert_eq!(component(&h.app).combine, Combine::Subtract);
    assert!(leaf(&menu(&h.app), &["Combine Mode", "Subtract"]).is_checked);
    h.choose(menu, &["Hide", "This"]);
    assert!(!component(&h.app).visible);
    h.choose(menu, &["Properties"]);
    let form = h.app.form.take().unwrap();
    assert_eq!(
        (form.id, form.params["combine"].as_str()),
        ("component_properties", Some("subtract"))
    );
    h.choose(menu, &["Delete"]);
    assert!(h.app.doc.components.is_empty());
}

#[test]
fn the_selection_menu_runs_edit_tools_on_the_selection() {
    let mut h = Harness::new();
    h.rectangle(10.0, 10.0, 40.0, 40.0);
    h.rectangle(30.0, 30.0, 40.0, 40.0);
    h.app
        .apply("add_layer", &serde_json::json!({ "name": "Top" }))
        .unwrap();
    h.app.invoke(&h.ctx, "select_all_vectors");
    let menu = |app: &CutCraftApp| app.menus.selection_menu(&app.doc);
    h.choose(menu, &["Move to Layer", "Top"]);
    let doc = &h.app.doc;
    assert!(doc.vectors.keys().all(|&id| doc.layer_of(id) == LayerId(2)));
    h.choose(menu, &["Weld"]);
    assert_eq!(h.app.doc.vectors.len(), 1);
    h.choose(menu, &["Undo"]);
    assert_eq!(h.app.doc.vectors.len(), 2);
    h.choose(menu, &["Align", "Left"]);
    let left: Vec<f64> = h
        .app
        .doc
        .vectors
        .values()
        .map(|v| v.bounds().min.x)
        .collect();
    assert!((left[0] - left[1]).abs() < 1e-9, "{left:?}");
    h.choose(menu, &["Cut"]);
    assert!(h.app.doc.vectors.is_empty(), "Cut takes the selection away");
}

#[test]
fn right_clicking_the_view_opens_the_selection_menu() {
    let mut h = Harness::new();
    h.rectangle(10.0, 10.0, 40.0, 40.0);
    h.rectangle(30.0, 30.0, 40.0, 40.0);
    h.app.invoke(&h.ctx, "select_all_vectors");
    h.right_click(h.at(90.0, 90.0));
    let weld = h.text_at("Weld").expect("the selection menu is open");
    h.click(weld, Modifiers::NONE);
    assert_eq!(h.app.doc.vectors.len(), 1, "{}", h.app.status.message);
}

#[test]
fn right_clicking_a_toolpath_selects_it_and_opens_its_menu() {
    let mut h = Harness::new();
    h.rectangle(10.0, 10.0, 50.0, 50.0);
    let params = h.app.registry.get("profile_toolpath").unwrap();
    let params = params.initial_params(&h.app.doc);
    h.app.apply("profile_toolpath", &params).unwrap();
    h.app.invoke(&h.ctx, "machining_layout");
    // the toolpath list's section settles its height over a frame or two
    h.frame(vec![]);
    h.frame(vec![]);
    let (&id, toolpath) = h.app.doc.toolpaths.iter().next().unwrap();
    let row = h
        .text_at(&toolpath.name.clone())
        .expect("the toolpath is listed");
    h.right_click(row);
    assert_eq!(h.app.selected_toolpath, Some(id));
    assert!(h.text_at("Recalculate").is_some(), "its menu is open");
}

#[test]
fn toolpath_forms_take_their_tool_from_the_database_and_follow_its_edits() {
    let mut h = Harness::new();
    h.app.invoke(&h.ctx, "pocket_toolpath");
    h.app.choose_tool("3 mm End Mill");
    h.frame(vec![]);
    h.frame(vec![]);
    assert!(h.text_at("Edit…").is_some());
    assert!(
        h.text_at("Number of Flutes").is_none(),
        "the tool's settings are not edited in the form"
    );
    assert_eq!(h.app.form.as_ref().unwrap().params["tool"]["diameter"], 3.0);
    h.app.edit_tool(&h.ctx, Some("3 mm End Mill".into()));
    let manager = h
        .app
        .tool_manager
        .as_mut()
        .expect("the Tool Database opens");
    let Some(cut_craft_tool_database::Pick::Tool(i)) = manager.pick else {
        panic!("on the form's tool")
    };
    assert_eq!(manager.draft.tools[i].name, "3 mm End Mill");
    manager.draft.tools[i].pass_depth = 1.25;
    let edited = manager.draft.clone();
    h.app.tools = edited;
    h.app.refresh_form_tool();
    assert_eq!(
        h.app.form.as_ref().unwrap().params["tool"]["pass_depth"],
        1.25,
        "saving the database updates the open form"
    );
}

#[test]
fn a_tool_chosen_for_a_toolpath_cuts_with_the_active_material() {
    let mut h = Harness::new();
    let mut tool = h.app.tools.get("3 mm End Mill").unwrap().clone();
    tool.set_cutting(cut_craft_core::toolpath::CuttingData {
        material: "Oak".into(),
        feed_rate: 450.0,
        ..tool.cutting("")
    });
    h.app.tools.put(tool);
    h.app.tools.material = "Oak".into();
    h.app.invoke(&h.ctx, "pocket_toolpath");
    h.app.choose_tool("3 mm End Mill");
    assert_eq!(
        h.app.form.as_ref().unwrap().params["tool"]["feed_rate"],
        450.0
    );
}

/// A 20 × 10 mm, 5 mm high pyramid as an ASCII STL, in a file of its own.
fn pyramid_stl(name: &str) -> std::path::PathBuf {
    use std::fmt::Write as _;
    let base = [
        [0.0, 0.0, 0.0],
        [20.0, 0.0, 0.0],
        [20.0, 10.0, 0.0],
        [0.0, 10.0, 0.0],
    ];
    let top = [10.0, 5.0, 5.0];
    let mut faces: Vec<[[f64; 3]; 3]> = (0..4).map(|i| [base[i], base[(i + 1) % 4], top]).collect();
    faces.extend([[base[0], base[2], base[1]], [base[0], base[3], base[2]]]);
    let mut text = String::from("solid pyramid\n");
    for face in faces {
        text += "facet normal 0 0 0\n outer loop\n";
        for v in face {
            let _ = writeln!(text, "  vertex {} {} {}", v[0], v[1], v[2]);
        }
        text += " endloop\nendfacet\n";
    }
    text += "endsolid pyramid\n";
    let path = std::env::temp_dir().join(format!("cut-craft-{name}-{}.stl", std::process::id()));
    std::fs::write(&path, text).unwrap();
    path
}

#[test]
fn an_imported_model_shows_at_once_and_follows_its_form() {
    let mut h = Harness::new();
    let file = pyramid_stl("live-import");
    h.app.import_file(&file);
    assert_eq!(
        h.app.doc.components.len(),
        1,
        "issue #3: the model is in the scene at once"
    );
    let size = &h.app.form.as_ref().unwrap().params["size"];
    assert!(
        (size["x"].as_f64().unwrap() - 20.0).abs() < 1.0,
        "Model Size shows its size: {size}"
    );
    // moving it in the form moves the same model
    let form = h.app.form.as_mut().unwrap();
    form.params["placing"]["center"] = Value::Bool(false);
    form.params["placing"]["position"] = serde_json::json!({ "x": 50.0, "y": 60.0 });
    form.dirty = true;
    h.frame(vec![]);
    assert_eq!(h.app.doc.components.len(), 1, "moved, not added");
    let k = h.app.doc.components.values().next().unwrap();
    let o = h.app.doc.job.area().min;
    assert!(
        (k.origin.x - 50.0).abs() < 1.0 && (k.origin.y - 60.0).abs() < 1.0,
        "at {:?} of {o:?}",
        k.origin
    );
    // and sizing it sizes it
    let form = h.app.form.as_mut().unwrap();
    form.params["size"]["x"] = Value::from(40.0);
    form.dirty = true;
    h.frame(vec![]);
    let k = h.app.doc.components.values().next().unwrap();
    let width = k.cell * f64::from(u32::try_from(k.columns).unwrap());
    assert!((width - 40.0).abs() < 2.0 * k.cell, "{} columns", k.columns);
    let _ = std::fs::remove_file(file);
}

#[test]
fn a_model_imported_past_the_material_edge_shows_that_part_as_a_red_ghost() {
    let mut h = Harness::new();
    let file = pyramid_stl("past-edge");
    h.app.import_file(&file);
    h.frame(vec![]);
    let form = h.app.form.as_ref().unwrap();
    assert!(
        form.beyond.as_ref().is_some_and(|b| b.is_empty()),
        "centred, nothing is past the edge"
    );
    // half of the 20 mm pyramid past the material's right edge
    let area = h.app.doc.job.area();
    let form = h.app.form.as_mut().unwrap();
    form.params["placing"]["center"] = Value::Bool(false);
    form.params["placing"]["position"] =
        serde_json::json!({ "x": area.max.x - 10.0, "y": area.min.y + 10.0 });
    form.dirty = true;
    h.frame(vec![]);
    let form = h.app.form.as_ref().unwrap();
    let beyond = form.beyond.as_ref().unwrap();
    assert!(!beyond.is_empty(), "the part past the edge is kept to show");
    assert!(
        beyond
            .iter()
            .flat_map(|(t, _)| t)
            .all(|v| v.x >= area.max.x - 1e-6),
        "only what is past it"
    );
    let model = form.model.as_ref().unwrap();
    assert!(
        model
            .iter()
            .flat_map(|(t, _)| t)
            .all(|v| v.x <= area.max.x + 1e-6),
        "the model drawn as usual stops at the edge"
    );
    // past the preview limit the component shows over the material, the ghost still past it
    h.app.options.import.preview_faces = 4;
    h.app.form.as_mut().unwrap().dirty = true;
    h.frame(vec![]);
    let form = h.app.form.as_ref().unwrap();
    assert!(!form.model_whole);
    assert!(
        form.beyond
            .as_ref()
            .is_some_and(|b| !b.is_empty() && b.len() <= 4),
        "the ghost keeps to the limit too: {:?}",
        form.beyond.as_ref().map(|b| b.len())
    );
    let _ = std::fs::remove_file(file);
}

#[test]
fn importing_a_model_shows_orientation_and_position_on_one_page() {
    let mut h = Harness::new();
    let file = pyramid_stl("pages");
    h.app.import_file(&file);
    h.frame(vec![]);
    h.frame(vec![]);
    let over = h.text_at("Initial Orientation").unwrap();
    for text in ["Depth Below Top", "Update Undercuts", "Import"] {
        assert!(h.scroll_to(over, text).is_some(), "{text}");
    }
    assert!(h.text_at("< Back").is_none(), "one page");
    let form = h.app.form.as_ref().unwrap();
    assert_eq!(
        form.params["placing"]["depth_below_top"], 5.0,
        "the whole model to start with, as its height"
    );
    let tall = |app: &CutCraftApp| {
        let k = app.doc.components.values().next().unwrap();
        k.height_range().unwrap().1
    };
    // the apex falls between cells
    let cell = h.app.doc.job.model_cell();
    assert!((tall(&h.app) - 5.0).abs() < 2.0 * cell, "{}", tall(&h.app));
    // the plane halfway down the model keeps its top half
    let form = h.app.form.as_mut().unwrap();
    form.params["placing"]["depth_below_top"] = Value::from(2.5);
    form.dirty = true;
    h.frame(vec![]);
    assert!((tall(&h.app) - 2.5).abs() < 2.0 * cell, "{}", tall(&h.app));
    let import = h.scroll_to(over, "Import").unwrap();
    h.click(import, Modifiers::NONE);
    assert!(h.app.form.is_none(), "Import closes the form");
    assert_eq!(h.app.doc.components.len(), 1, "keeping the model");
    let _ = std::fs::remove_file(file);
}

#[test]
fn the_import_form_lifts_aligns_and_follows_a_drag_as_it_goes() {
    let mut h = Harness::new();
    let file = pyramid_stl("align");
    h.app.import_file(&file);
    h.frame(vec![]);
    h.frame(vec![]);
    // centred, the corner it gives is shown, for the numbers to move it from there
    let (area, thickness) = (h.app.doc.job.area(), h.app.doc.job.thickness);
    let placing = |app: &CutCraftApp| app.form.as_ref().unwrap().params["placing"].clone();
    let p = placing(&h.app);
    assert_eq!(p["center"], false);
    let x0 = p["position"]["x"].as_f64().unwrap();
    assert!((x0 - (area.center().x - 10.0)).abs() < 0.01, "{p}");
    let top = |app: &CutCraftApp| {
        let k = app.doc.components.values().next().unwrap();
        k.height_range().unwrap().1
    };
    let cell = h.app.doc.job.model_cell();
    // a depth past the model's 5 mm lifts it off the plane, and stays
    assert!(thickness > 8.0, "{thickness}");
    let form = h.app.form.as_mut().unwrap();
    form.params["placing"]["depth_below_top"] = Value::from(8.0);
    form.dirty = true;
    h.frame(vec![]);
    assert_eq!(placing(&h.app)["depth_below_top"], 8.0, "not set back");
    assert!((top(&h.app) - 8.0).abs() < 2.0 * cell, "{}", top(&h.app));
    // Height ▸ Middle puts it half way up the material
    let over = h.text_at("Orientate 3D Model").unwrap();
    let middle = h.scroll_to(over, "Middle").expect("the height buttons");
    h.click(middle, Modifiers::NONE);
    h.frame(vec![]);
    let half = f64::midpoint(5.0, thickness);
    let depth = placing(&h.app)["depth_below_top"].as_f64().unwrap();
    assert!((depth - half).abs() < 0.01, "{depth}");
    assert!((top(&h.app) - half).abs() < 2.0 * cell, "{}", top(&h.app));
    // a dragged number moves the model while the button is held, the drag one undo
    let origin = |app: &CutCraftApp| app.doc.components.values().next().unwrap().origin.x;
    let undo_from = h.app.history.undo_depth();
    let held = h.text_at("Align in Material").unwrap();
    h.press(held, true, Modifiers::NONE);
    for step in 1..=3 {
        let x = x0 + 5.0 * f64::from(step);
        let form = h.app.form.as_mut().unwrap();
        form.params["placing"]["position"]["x"] = Value::from(x);
        form.dirty = true;
        h.frame(vec![]);
        assert!(
            (origin(&h.app) - x).abs() < cell,
            "moved as it goes: {step}"
        );
    }
    h.press(held, false, Modifiers::NONE);
    h.frame(vec![]);
    assert_eq!(h.app.history.undo_depth(), undo_from + 1, "one undo");
    assert_eq!(h.app.doc.components.len(), 1);
    assert!(h.app.history.undo(&mut h.app.doc));
    assert!((origin(&h.app) - x0).abs() < cell, "back where it was");
    // every section's numbers so: Interactive Rotation turns it as the drag goes
    let wide = |app: &CutCraftApp| {
        let k = app.doc.components.values().next().unwrap();
        k.cell * f64::from(u32::try_from(k.columns).unwrap())
    };
    assert!((wide(&h.app) - 20.0).abs() < 2.0 * cell);
    h.press(held, true, Modifiers::NONE);
    let form = h.app.form.as_mut().unwrap();
    form.params["turn"]["z"] = Value::from(90.0);
    form.dirty = true;
    h.frame(vec![]);
    assert!(
        (wide(&h.app) - 10.0).abs() < 2.0 * cell,
        "turned while held"
    );
    h.press(held, false, Modifiers::NONE);
    let _ = std::fs::remove_file(file);
}

#[test]
fn a_drawn_shape_changes_with_its_form_until_it_is_let_go() {
    use cut_craft_core::geometry::Segment;
    let arcs = |app: &CutCraftApp| {
        let only: Vec<_> = app.doc.vectors.values().collect();
        assert_eq!(only.len(), 1, "one shape, not a second on top");
        only[0]
            .segments
            .iter()
            .filter(|s| matches!(s, Segment::Arc { .. }))
            .count()
    };
    let mut h = Harness::new();
    h.app.invoke(&h.ctx, "draw_rectangle");
    h.frame(vec![]);
    let (from, to) = (h.at(100.0, 100.0), h.at(200.0, 160.0));
    h.drag(from, to);
    assert_eq!(arcs(&h.app), 0, "drawn square");
    // issue #2: choosing rounded corners afterwards changes the rectangle just drawn, as
    // soon as it is chosen (issue #3), and Apply has nothing more to do
    let rounded = h.text_at("Rounded").expect("the corner choices");
    h.click(rounded, Modifiers::NONE);
    assert_eq!(arcs(&h.app), 4, "the same rectangle, rounded");
    let params = h.app.form.as_ref().unwrap().params.clone();
    h.app.apply_form("draw_rectangle", &params).unwrap();
    assert_eq!(arcs(&h.app), 4);
    let rect = h.app.doc.vectors.values().next().unwrap().bounds();
    // drawn from screen pixels: the same to a hundredth of a millimetre
    assert!((rect.size().x - 100.0).abs() < 1e-2, "same size: {rect:?}");
    h.app.invoke(&h.ctx, "undo");
    assert_eq!(arcs(&h.app), 0, "one undo puts the square back");
    // once let go, Apply draws a new shape
    h.app.invoke(&h.ctx, "redo");
    h.app.doc.selection.clear();
    h.app.apply_form("draw_rectangle", &params).unwrap();
    assert_eq!(h.app.doc.vectors.len(), 2);
}

#[test]
fn right_clicking_anywhere_offers_a_bug_report_saying_where() {
    let mut h = Harness::new();
    h.rectangle(10.0, 10.0, 50.0, 50.0);
    // the 2D view's own menu carries the item
    let canvas = h.at(80.0, 80.0);
    h.right_click(canvas);
    assert!(
        h.text_at("Report a Bug Here…").is_some(),
        "in the 2D view's menu"
    );
    h.frame(vec![Event::Key {
        key: egui::Key::Escape,
        physical_key: None,
        pressed: true,
        repeat: false,
        modifiers: Modifiers::NONE,
    }]);
    h.frame(vec![]);
    // the status bar has no menu of its own: the small one shows
    let status = h.ctx.content_rect().left_bottom() + egui::vec2(5.0, -5.0);
    h.right_click(status);
    // a new menu is drawn unseen for a frame while egui sizes it
    h.frame(vec![]);
    let item = h.text_at("Report a Bug Here…").expect("the small menu");
    h.click(item, Modifiers::NONE);
    assert!(h.app.reporter.is_open(), "the form opens");
    assert!(h.text_at("Report a Bug").is_some());
    let state = h.app.bug_state();
    assert!(
        state.contains("Cut Craft") && state.contains("1 vectors"),
        "{state}"
    );
}

#[test]
fn the_tool_database_window_keeps_edits_only_on_ok() {
    let mut h = Harness::new();
    h.app.invoke(&h.ctx, "tool_database");
    h.frame(vec![]);
    let count = h.app.tools.tools.len();
    h.app.tool_manager.as_mut().unwrap().draft.tools.remove(0);
    h.frame(vec![]);
    assert_eq!(h.app.tools.tools.len(), count, "not until OK");
    let ok = *h.texts_at("OK").last().expect("the window has OK");
    h.click(ok, Modifiers::NONE);
    assert!(h.app.tool_manager.is_none());
    assert_eq!(h.app.tools.tools.len(), count - 1);
    h.app.invoke(&h.ctx, "tool_database");
    h.app.tool_manager.as_mut().unwrap().draft.tools.clear();
    h.frame(vec![]);
    let cancel = *h.texts_at("Cancel").last().expect("and Cancel");
    h.click(cancel, Modifiers::NONE);
    assert_eq!(h.app.tools.tools.len(), count - 1, "Cancel drops the edits");
}

/// Picks `name` in the open Select Tool window and clicks its Select.
fn select_in_tool_database(h: &mut Harness, name: &str) {
    h.frame(vec![]);
    h.frame(vec![]);
    let manager = h.app.tool_manager.as_mut().expect("Select Tool");
    assert!(manager.selecting, "the Tool Database opens to pick in");
    let at = manager
        .draft
        .tools
        .iter()
        .position(|t| t.name == name)
        .unwrap();
    manager.pick = Some(cut_craft_tool_database::Pick::Tool(at));
    h.frame(vec![]);
    h.frame(vec![]);
    let select = *h.texts_at("Select").last().expect("Select, in place of OK");
    h.click(select, Modifiers::NONE);
    h.frame(vec![]);
    assert!(h.app.tool_manager.is_none(), "closed");
}

#[test]
fn three_d_finishing_picks_its_tools_in_the_tool_database() {
    let mut h = Harness::new();
    h.app.invoke(&h.ctx, "3d_finishing_toolpath");
    h.app.pages.set_pinned(Page::Toolpaths, true);
    h.frame(vec![]);
    h.frame(vec![]);
    let first = h.app.form.as_ref().unwrap().params["tools"][0]["name"].clone();
    // no drop-down of tools: each has Select… and Edit…, and Add a tool… opens the database
    assert!(h.text_at("Select…").is_some(), "the first tool's Select…");
    let add = h.text_at("Add a tool…").expect("an Add button");
    h.click(add, Modifiers::NONE);
    select_in_tool_database(&mut h, "6 mm Ball Nose");
    let form = h.app.form.as_ref().unwrap();
    assert_eq!(form.params["tools"][0]["name"], first, "kept");
    assert_eq!(
        form.params["tools"][1]["name"], "6 mm Ball Nose",
        "added after it"
    );
    let ball = h.app.tools.get("6 mm Ball Nose").unwrap();
    assert_eq!(
        form.params["tools"][1]["feed_rate"],
        serde_json::json!(ball.feed_rate),
        "its own feeds and speeds come with it"
    );
    // the first tool's Select… puts another in its place
    let select = *h.texts_at("Select…").first().unwrap();
    h.click(select, Modifiers::NONE);
    select_in_tool_database(&mut h, "3 mm End Mill");
    let form = h.app.form.as_ref().unwrap();
    assert_eq!(form.params["tools"][0]["name"], "3 mm End Mill");
    assert_eq!(form.params["tools"][1]["name"], "6 mm Ball Nose");
    assert_eq!(form.params["tools"].as_array().unwrap().len(), 2);
}

#[test]
fn rest_machining_picks_its_earlier_tools_in_the_tool_database() {
    let mut h = Harness::new();
    h.rectangle(10.0, 10.0, 50.0, 50.0);
    h.app.invoke(&h.ctx, "profile_toolpath");
    h.app.pages.set_pinned(Page::Toolpaths, true);
    h.app.choose_tool("3 mm End Mill");
    h.frame(vec![]);
    // what Rest Machining's Add a tool… asks for: Select Tool, for its list
    h.app
        .ask_listed(&h.ctx, ToolList::Rest, ListAsk::Select(ListSlot::New));
    select_in_tool_database(&mut h, "6 mm End Mill");
    let form = h.app.form.as_ref().unwrap();
    assert_eq!(form.params["rest"]["after"][0]["name"], "6 mm End Mill");
    assert_eq!(
        form.params["tool"]["name"], "3 mm End Mill",
        "the form's own tool is left as it was"
    );
    // and its Select… replaces that one
    h.app
        .ask_listed(&h.ctx, ToolList::Rest, ListAsk::Select(ListSlot::At(0)));
    select_in_tool_database(&mut h, "6 mm Ball Nose");
    let after = &h.app.form.as_ref().unwrap().params["rest"]["after"];
    assert_eq!(after.as_array().unwrap().len(), 1);
    assert_eq!(after[0]["name"], "6 mm Ball Nose");
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
fn the_tool_database_window_shows_the_chip_load() {
    let mut h = Harness::new();
    h.app.tools = ToolDatabase {
        tools: vec![Tool::default()],
        ..ToolDatabase::default()
    };
    h.app.invoke(&h.ctx, "tool_database");
    h.app.tool_manager.as_mut().unwrap().pick = Some(cut_craft_tool_database::Pick::Tool(0));
    h.frame(vec![]);
    h.frame(vec![]);
    // the details scroll: down to the feeds
    let details = h.text_at("Tool Info").expect("the tool's details");
    for _ in 0..3 {
        h.frame(vec![
            Event::PointerMoved(details),
            Event::MouseWheel {
                unit: egui::MouseWheelUnit::Point,
                delta: vec2(0.0, -400.0),
                phase: egui::TouchPhase::Move,
                source: egui::MouseWheelSource::Wheel,
                modifiers: Modifiers::NONE,
            },
        ]);
    }
    h.frame(vec![]);
    // 1000 mm/min over 18000 rpm x 2 flutes
    assert!(
        h.text_at("Chip load: 0.028 mm").is_some(),
        "the window shows the feed per tooth"
    );
}

#[test]
fn previewing_all_toolpaths_is_previewing_each_in_turn() {
    let mut h = Harness::new();
    h.rectangle(10.0, 10.0, 50.0, 50.0);
    for id in ["drilling_toolpath", "profile_toolpath"] {
        h.app.invoke(&h.ctx, id);
        let params = h.app.form.as_ref().unwrap().params.clone();
        h.app.apply(id, &params).unwrap();
    }
    h.app.invoke(&h.ctx, "preview_all_toolpaths");
    assert_eq!(h.app.view, View::ThreeD, "previewing shows the 3D view");
    h.frame(vec![]);
    let all = h.app.simulation.clone().expect("a preview");
    h.app.invoke(&h.ctx, "reset_preview");
    assert!(h.app.simulation.is_none());
    for id in h.app.doc.toolpaths.keys().copied().collect::<Vec<_>>() {
        h.app.selected_toolpath = Some(id);
        h.app.invoke(&h.ctx, "preview_toolpath");
    }
    assert_eq!(h.app.simulation, Some(all));
}

#[test]
fn the_3d_view_shows_the_model_until_there_is_a_preview() {
    let mut h = Harness::new();
    h.app.design_tab = DESIGN_TABS.iter().position(|&t| t == "Design").unwrap();
    assert!(
        h.app
            .toolbar
            .modeling_groups
            .iter()
            .any(|g| g.tools.contains(&"create_shape_round"))
    );
    h.rectangle(10.0, 10.0, 50.0, 50.0);
    let shape = h.app.registry.get("create_shape").unwrap().default_params();
    h.app.apply("create_shape", &shape).unwrap();
    h.app.view = View::ThreeD;
    h.frame(vec![]);
    let raised = |app: &mut CutCraftApp| app.surface_3d().unwrap().heights.iter().any(|&z| z > 0.0);
    assert!(raised(&mut h.app), "the model");
    let profile = h
        .app
        .registry
        .get("profile_toolpath")
        .unwrap()
        .default_params();
    h.app.apply("profile_toolpath", &profile).unwrap();
    h.app.invoke(&h.ctx, "preview_all_toolpaths");
    h.frame(vec![]);
    assert!(!raised(&mut h.app), "the preview, cut down from the top");
}

#[test]
fn a_double_click_in_the_3d_view_picks_the_component_under_it() {
    let mut h = Harness::new();
    h.app.doc = cut_craft_core::testing::domed();
    h.app.orbit = view3d::Orbit::TOP;
    let fit = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(400.0, 400.0));
    let surface = h.app.surface_3d().unwrap().into_owned();
    let screen = view3d::screen_of(&surface, h.app.orbit, h.app.framing, fit, None);
    let local = |x: f64, y: f64| {
        let z = surface
            .at(cut_craft_core::geometry::Point::new(x, y))
            .unwrap();
        cut_craft_core::glam::DVec3::new(x - surface.origin.x, y - surface.origin.y, z)
    };
    assert_eq!(
        h.app.component_at_3d(fit, screen(local(20.0, 20.0))),
        Some(EntityId(1))
    );
    assert_eq!(
        h.app.component_at_3d(fit, screen(local(2.0, 2.0))),
        None,
        "off the dome"
    );
}

#[test]
fn right_clicking_a_component_in_the_3d_view_picks_it_and_opens_its_menu() {
    let mut h = Harness::new();
    h.app.doc = cut_craft_core::testing::domed();
    h.app.orbit = view3d::Orbit::TOP;
    h.app.view = View::ThreeD;
    h.frame(vec![]);
    h.frame(vec![]);
    let fit = h.app.fit_3d.expect("the 3D view has been drawn");
    let surface = h.app.surface_3d().unwrap().into_owned();
    let screen = view3d::screen_of(&surface, h.app.orbit, h.app.framing, fit, None);
    let z = surface.at(Point::new(20.0, 20.0)).unwrap();
    let top = screen(cut_craft_core::glam::DVec3::new(
        20.0 - surface.origin.x,
        20.0 - surface.origin.y,
        z,
    ));
    h.right_click(top);
    assert!(h.app.doc.selected_components.contains(&EntityId(1)));
    assert_eq!(h.app.under_3d, vec![EntityId(1)]);
    assert!(h.text_at("dome").is_some(), "the menu, headed by its name");
    let menu = |app: &CutCraftApp| app.menus.view_3d_menu(&app.doc, &app.under_3d);
    assert!(
        leaf(&menu(&h.app), &["dome"]).is_checked,
        "listed as selected"
    );
    assert!(
        !leaf(&menu(&h.app), &["Node Edit Mode"]).is_enabled,
        "a vector item, greyed for a component"
    );
    assert!(leaf(&menu(&h.app), &["Component", "Combine Mode", "Add"]).is_checked);
    h.choose(menu, &["Component", "Create Vector Boundary"]);
    assert_eq!(h.app.doc.vectors.len(), 1, "shape to vector outline");
    h.choose(menu, &["Copy"]);
    h.choose(menu, &["Paste"]);
    assert_eq!(h.app.doc.components.len(), 2, "components paste too");
    h.app.doc.selected_components = [EntityId(1)].into();
    h.choose(menu, &["Delete"]);
    assert_eq!(h.app.doc.components.len(), 1);
}

/// The domed document in the 3D view, seen at a slant, with drawing tool `tool` open, and
/// where a point on the model's surface shows on screen.
fn drawing_on_the_dome(tool: &str) -> (Harness, impl Fn(f64, f64) -> Pos2, f64) {
    let mut h = Harness::new();
    h.app.doc = cut_craft_core::testing::domed();
    h.app.view = View::ThreeD;
    h.app.invoke(&h.ctx, tool);
    h.frame(vec![]);
    h.frame(vec![]);
    let fit = h.app.fit_3d.expect("the 3D view has been drawn");
    let surface = h.app.surface_3d().unwrap().into_owned();
    let camera = view3d::Camera::new(&surface, h.app.orbit, h.app.framing, fit, None);
    let cell = surface.cell;
    let on_model = move |x: f64, y: f64| {
        let z = surface.at(Point::new(x, y)).unwrap();
        camera.screen(cut_craft_core::glam::DVec3::new(
            x - surface.origin.x,
            y - surface.origin.y,
            z,
        ))
    };
    (h, on_model, cell)
}

#[test]
fn a_drag_with_a_drawing_tool_in_the_3d_view_draws_where_it_meets_the_model() {
    let (mut h, on_model, cell) = drawing_on_the_dome("draw_rectangle");
    let turned = h.app.orbit;
    // on the dome's top, well up off the material's top plane
    h.drag(on_model(18.0, 18.0), on_model(23.0, 21.0));
    assert_eq!(
        h.app.orbit, turned,
        "the left button draws, it does not turn the view"
    );
    assert_eq!(
        h.app.doc.vectors.len(),
        1,
        "{:?}",
        h.app.form.as_ref().map(|f| &f.error)
    );
    let b = h.app.doc.vectors.values().next().unwrap().bounds();
    let close = |a: Point, b: Point| a.distance(b) < 1.5 * cell;
    assert!(
        close(b.min, Point::new(18.0, 18.0)) && close(b.max, Point::new(23.0, 21.0)),
        "{b:?}, cells of {cell}"
    );
    assert!(h.app.history.can_undo());
    // the right button still turns it
    let at = on_model(20.0, 20.0);
    h.frame(vec![Event::PointerMoved(at)]);
    let right = |pos, pressed| Event::PointerButton {
        pos,
        button: PointerButton::Secondary,
        pressed,
        modifiers: Modifiers::NONE,
    };
    h.frame(vec![right(at, true)]);
    h.frame(vec![Event::PointerMoved(at + vec2(40.0, 0.0))]);
    h.frame(vec![Event::PointerMoved(at + vec2(80.0, 0.0))]);
    h.frame(vec![right(at + vec2(80.0, 0.0), false)]);
    assert_ne!(h.app.orbit, turned);
    assert_eq!(h.app.doc.vectors.len(), 1, "turning draws nothing");
}

#[test]
fn drawing_in_the_3d_view_meets_the_model_as_finely_as_the_settled_view_draws_it() {
    let (mut h, _, _) = drawing_on_the_dome("draw_rectangle");
    let fine = h.app.fine_surface();
    let hit = h.app.drawing_surface().expect("a model to draw on");
    assert_eq!((hit.columns, hit.rows), (fine.columns, fine.rows));
    // cells off the model are NaN, so compare bit for bit
    let bits = |m: &Material| m.heights.iter().map(|h| h.to_bits()).collect::<Vec<_>>();
    assert_eq!(bits(&hit), bits(&fine));
    let again = h.app.drawing_surface().unwrap();
    assert!(
        std::sync::Arc::ptr_eq(&hit, &again),
        "kept while nothing changes"
    );
    h.app.doc.selected_components = [EntityId(1)].into();
    h.app.invoke(&h.ctx, "delete");
    assert!(h.app.drawing_surface().is_none(), "nothing left to draw on");
}

#[test]
fn polyline_clicks_in_the_3d_view_place_its_points_on_the_model() {
    let (mut h, on_model, cell) = drawing_on_the_dome("draw_polyline");
    let corners = [(17.0, 18.0), (23.0, 18.0), (20.0, 23.0)];
    for (x, y) in corners {
        h.click(on_model(x, y), Modifiers::NONE);
    }
    // the first point again closes it
    h.click(on_model(17.0, 18.0), Modifiers::NONE);
    assert_eq!(h.app.doc.vectors.len(), 1);
    let path = h.app.doc.vectors.values().next().unwrap();
    assert!(path.closed);
    let b = path.bounds();
    let close = |a: f64, b: f64| (a - b).abs() < 1.5 * cell;
    assert!(
        close(b.min.x, 17.0)
            && close(b.max.x, 23.0)
            && close(b.min.y, 18.0)
            && close(b.max.y, 23.0),
        "{b:?}"
    );
}

#[test]
fn the_3d_view_tells_how_to_draw_and_turn_while_a_drawing_tool_is_open() {
    let (mut h, on_model, _) = drawing_on_the_dome("draw_circle");
    h.frame(vec![Event::PointerMoved(on_model(20.0, 20.0))]);
    h.frame(vec![]);
    let hint = cut_craft_bug_report::hint::current(&h.ctx).unwrap_or_default();
    assert!(
        hint.starts_with("Draw Circle: drag from the centre"),
        "{hint}"
    );
    assert!(hint.ends_with("right-drag turns the view"), "{hint}");
}

#[test]
fn the_3d_views_transform_handles_move_scale_and_turn_the_selection() {
    use cut_craft_core::handles::Handle;
    let mut h = Harness::new();
    h.app.doc = cut_craft_core::testing::domed();
    h.app.doc.selected_components = [EntityId(1)].into();
    h.app.orbit = view3d::Orbit::TOP;
    h.app.view = View::ThreeD;
    h.app.invoke(&h.ctx, "transform");
    h.frame(vec![]);
    h.frame(vec![]);
    let at = |h: &Harness, which: Handle| {
        h.app
            .handles_3d
            .iter()
            .find(|(k, _)| *k == Handle3d::Transform(which))
            .map(|(_, at)| *at)
            .expect("the transform handles are drawn over the selection")
    };
    let extent = |h: &Harness| h.app.doc.components[&EntityId(1)].extent();
    let before = extent(&h);
    let corner = Handle::Scale { x: 1, y: 1 };
    // the middle moves: dragged onto the top-right corner handle, the component's centre
    // lands where that corner was
    h.drag(at(&h, Handle::Move), at(&h, corner));
    let moved = extent(&h);
    assert!((moved.center() - before.max).length() < 0.5, "{moved:?}");
    h.app.invoke(&h.ctx, "undo");
    assert_eq!(extent(&h), before, "one undo");
    h.frame(vec![]);
    // a corner scales about the opposite corner keeping the shape: dragged half as far
    // again from the middle, the box is half as big again
    let (middle, from) = (at(&h, Handle::Move), at(&h, corner));
    h.drag(from, from + (from - middle));
    let scaled = extent(&h);
    assert!((scaled.min - before.min).length() < 0.5, "{scaled:?}");
    let grown = before.min + before.size() * 1.5;
    assert!((scaled.max - grown).length() < 0.5, "{scaled:?}");
    h.app.invoke(&h.ctx, "undo");
    h.frame(vec![]);
    // the rotation arrow with Shift held turns in 22.5° steps: dragged about 20° round the
    // middle, the box turns 22.5°, so its bounds grow by cos 22.5° + sin 22.5°
    let (middle, from) = (at(&h, Handle::Move), at(&h, Handle::Rotate { x: 0, y: 1 }));
    let to = middle + egui::emath::Rot2::from_angle(20f32.to_radians()) * (from - middle);
    let shift = Modifiers::SHIFT;
    h.frame_holding(shift, vec![Event::PointerMoved(from)]);
    h.press(from, true, shift);
    h.frame_holding(shift, vec![Event::PointerMoved(to)]);
    h.press(to, false, shift);
    h.frame_holding(shift, vec![]);
    let turned = extent(&h);
    let (sin, cos) = 22.5f64.to_radians().sin_cos();
    let wide = before.size().x * (cos + sin);
    assert!(
        (turned.size().x - wide).abs() < 0.3,
        "{turned:?}, not {wide} wide"
    );
    assert!(
        (turned.center() - before.center()).length() < 0.5,
        "about the centre"
    );
}

#[test]
fn the_rotate_snap_angle_comes_from_snapping_options() {
    use cut_craft_core::handles::Handle;
    let mut h = Harness::new();
    h.app.doc = cut_craft_core::testing::domed();
    h.app.doc.selected_components = [EntityId(1)].into();
    h.app.orbit = view3d::Orbit::TOP;
    h.app.view = View::ThreeD;
    h.app.options.snapping.rotate_step = cut_craft_core::snap::RotateSnap::Ten;
    h.app.invoke(&h.ctx, "transform");
    h.frame(vec![]);
    h.frame(vec![]);
    let at = |h: &Harness, which: Handle| {
        h.app
            .handles_3d
            .iter()
            .find(|(k, _)| *k == Handle3d::Transform(which))
            .map(|(_, at)| *at)
            .expect("the transform handles are drawn over the selection")
    };
    let before = h.app.doc.components[&EntityId(1)].extent();
    let (middle, from) = (at(&h, Handle::Move), at(&h, Handle::Rotate { x: 0, y: 1 }));
    // over the arrow, the status bar names the chosen angle
    h.frame(vec![Event::PointerMoved(from)]);
    let hint = cut_craft_bug_report::hint::current(&h.ctx).unwrap_or_default();
    assert!(
        hint.contains("Hold Shift to snap to 10°, or Ctrl to 1°"),
        "{hint}"
    );
    // dragged about 27° round the middle with Shift held, the box turns 30°, so its bounds
    // grow by cos 30° + sin 30°
    let to = middle + egui::emath::Rot2::from_angle(27f32.to_radians()) * (from - middle);
    let shift = Modifiers::SHIFT;
    h.frame_holding(shift, vec![Event::PointerMoved(from)]);
    h.press(from, true, shift);
    h.frame_holding(shift, vec![Event::PointerMoved(to)]);
    h.press(to, false, shift);
    h.frame_holding(shift, vec![]);
    let turned = h.app.doc.components[&EntityId(1)].extent();
    let (sin, cos) = 30f64.to_radians().sin_cos();
    let wide = before.size().x * (cos + sin);
    assert!(
        (turned.size().x - wide).abs() < 0.3,
        "{turned:?}, not {wide} wide"
    );
}

#[test]
fn a_3d_transform_drag_let_go_above_the_horizon_changes_nothing_and_ends() {
    use cut_craft_core::handles::Handle;
    let mut h = Harness::new();
    h.app.doc = cut_craft_core::testing::domed();
    h.app.doc.selected_components = [EntityId(1)].into();
    // nearly side on in perspective: the top of the view is sky, not material
    h.app.orbit = view3d::Orbit {
        pitch: 80f64.to_radians(),
        ..view3d::Orbit::FRONT
    };
    h.app.framing.perspective = true;
    h.app.view = View::ThreeD;
    h.app.invoke(&h.ctx, "transform");
    h.frame(vec![]);
    h.frame(vec![]);
    let middle = h
        .app
        .handles_3d
        .iter()
        .find(|(k, _)| *k == Handle3d::Transform(Handle::Move))
        .map(|(_, at)| *at)
        .expect("the move handle is drawn");
    let sky = h
        .app
        .fit_3d
        .expect("the 3D view has been drawn")
        .center_top()
        + vec2(0.0, 5.0);
    let before = h.app.doc.components[&EntityId(1)].extent();
    h.drag(middle, sky);
    assert!(h.app.drag_3d.is_none(), "letting go ends the drag");
    assert_eq!(
        h.app.doc.components[&EntityId(1)].extent(),
        before,
        "let go over the sky, nothing moves"
    );
    // moving and clicking afterwards does not pick the abandoned drag back up
    h.click(middle + vec2(0.0, 30.0), Modifiers::NONE);
    assert_eq!(h.app.doc.components[&EntityId(1)].extent(), before);
}

/// The 3D view in Transform mode over the domed fixture, its component selected.
fn transform_3d(orbit: view3d::Orbit, perspective: bool) -> Harness {
    let mut h = Harness::new();
    h.app.doc = cut_craft_core::testing::domed();
    h.app.doc.selected_components = [EntityId(1)].into();
    h.app.orbit = orbit;
    h.app.framing.perspective = perspective;
    h.app.view = View::ThreeD;
    h.app.invoke(&h.ctx, "transform");
    h.frame(vec![]);
    h.frame(vec![]);
    h
}

/// Where the 3D view drew `which`.
fn handle_3d_at(h: &Harness, which: Handle3d) -> Pos2 {
    h.app
        .handles_3d
        .iter()
        .find(|(k, _)| *k == which)
        .map(|(_, at)| *at)
        .expect("the handle is drawn")
}

fn escape() -> Event {
    Event::Key {
        key: egui::Key::Escape,
        physical_key: None,
        pressed: true,
        repeat: false,
        modifiers: Modifiers::NONE,
    }
}

#[test]
fn escape_cancels_a_3d_handle_drag_and_stays_in_transform_mode() {
    use cut_craft_core::handles::Handle;
    let mut h = transform_3d(view3d::Orbit::TOP, false);
    let extent = |h: &Harness| h.app.doc.components[&EntityId(1)].extent();
    let before = extent(&h);
    let depth = h.app.history.undo_depth();
    // a move: Escape mid-drag, then the button let go
    let from = handle_3d_at(&h, Handle3d::Transform(Handle::Move));
    let to = from + vec2(60.0, 0.0);
    h.frame(vec![Event::PointerMoved(from)]);
    h.press(from, true, Modifiers::NONE);
    h.frame(vec![Event::PointerMoved(to)]);
    h.frame(vec![escape()]);
    h.press(to, false, Modifiers::NONE);
    h.frame(vec![]);
    assert_eq!(extent(&h), before, "nothing moved");
    assert_eq!(h.app.history.undo_depth(), depth, "nothing to undo");
    assert_eq!(
        h.app.view_tool,
        Some(ViewTool::Transform),
        "still transforming"
    );
    assert!(h.app.drag_3d.is_none());
    // a height: set as it is dragged, so Escape takes it back
    let height = |h: &Harness| h.app.doc.components[&EntityId(1)].shaping.shape_height;
    let start = height(&h);
    let from = handle_3d_at(&h, Handle3d::Height);
    let up = from - vec2(0.0, 80.0);
    h.frame(vec![Event::PointerMoved(from)]);
    h.press(from, true, Modifiers::NONE);
    h.frame(vec![Event::PointerMoved(up)]);
    h.frame(vec![Event::PointerMoved(up)]);
    assert!(height(&h) > start * 1.2, "follows the drag");
    h.frame(vec![escape()]);
    h.press(up, false, Modifiers::NONE);
    h.frame(vec![]);
    assert!((height(&h) - start).abs() < 1e-9, "back to {start}");
    assert_eq!(h.app.history.undo_depth(), depth);
    assert_eq!(h.app.view_tool, Some(ViewTool::Transform));
}

#[test]
fn the_3d_views_transform_handles_move_a_selection_of_vectors_only() {
    use cut_craft_core::handles::Handle;
    let mut h = transform_3d(view3d::Orbit::TOP, false);
    h.app.doc.selected_components.clear();
    h.rectangle(2.0, 2.0, 6.0, 6.0);
    let id = *h
        .app
        .doc
        .selection
        .iter()
        .next()
        .expect("the rectangle is selected");
    h.app.view = View::ThreeD;
    h.app.view_tool = Some(ViewTool::Transform);
    h.frame(vec![]);
    h.frame(vec![]);
    assert!(
        !h.app.handles_3d.iter().any(|(k, _)| *k == Handle3d::Height),
        "a vector has no component icons"
    );
    let before = h.app.doc.vectors[&id].bounds();
    let from = handle_3d_at(&h, Handle3d::Transform(Handle::Move));
    let corner = handle_3d_at(&h, Handle3d::Transform(Handle::Scale { x: 1, y: 1 }));
    h.drag(from, corner);
    let after = h.app.doc.vectors[&id].bounds();
    assert!((after.center() - before.max).length() < 0.5, "{after:?}");
}

#[test]
fn with_nothing_selected_the_3d_transform_mode_says_how_to_start() {
    let mut h = transform_3d(view3d::Orbit::TOP, false);
    h.app.doc.selected_components.clear();
    let middle = h.app.fit_3d.expect("the 3D view has been drawn").center();
    h.frame(vec![Event::PointerMoved(middle)]);
    h.frame(vec![]);
    assert_eq!(
        h.app.handles_3d,
        Vec::<(Handle3d, Pos2)>::new(),
        "no selection, no handles"
    );
    let hint = cut_craft_bug_report::hint::current(&h.ctx).unwrap_or_default();
    assert!(hint.contains("select it"), "{hint}");
}

#[test]
fn the_3d_transform_handles_sit_on_top_of_a_raised_component() {
    use cut_craft_core::glam::DVec3;
    use cut_craft_core::handles::Handle;
    // nearly side on in perspective, where a box on the material's top would sit well
    // below the dome
    let low = view3d::Orbit {
        pitch: 70f64.to_radians(),
        ..view3d::Orbit::FRONT
    };
    let mut h = transform_3d(low, true);
    let fit = h.app.fit_3d.unwrap();
    let surface = h.app.surface_3d().unwrap().into_owned();
    let top = surface.at(Point::new(20.0, 20.0)).unwrap();
    assert!(top > 1.0, "the dome stands up: {top}");
    let camera = view3d::Camera::new(&surface, h.app.orbit, h.app.framing, fit, None);
    let at = |z: f64| {
        camera.screen(DVec3::new(
            20.0 - surface.origin.x,
            20.0 - surface.origin.y,
            z,
        ))
    };
    let cross = handle_3d_at(&h, Handle3d::Transform(Handle::Move));
    assert!(
        (cross - at(top)).length() < 2.0,
        "{cross:?} at the dome's top {:?}, not its foot {:?}",
        at(top),
        at(0.0)
    );
    // and a drag on that plane still moves by what the pointer moved over
    let before = h.app.doc.components[&EntityId(1)].extent();
    let corner = handle_3d_at(&h, Handle3d::Transform(Handle::Scale { x: 1, y: 1 }));
    h.drag(cross, corner);
    let moved = h.app.doc.components[&EntityId(1)].extent();
    assert!((moved.center() - before.max).length() < 0.5, "{moved:?}");
}

#[test]
fn the_3d_views_handles_scale_the_selected_component_and_open_its_settings() {
    use cut_craft_core::component::Combine;
    let mut h = Harness::new();
    h.app.doc = cut_craft_core::testing::domed();
    h.app.doc.selected_components = [EntityId(1)].into();
    h.app.orbit = view3d::Orbit::TOP;
    h.app.view = View::ThreeD;
    h.frame(vec![]);
    h.frame(vec![]);
    assert!(h.app.handles_3d.is_empty(), "not until Transform mode");
    // a double-click on the selected component enters it, as Aspire's second click does
    let fit = h.app.fit_3d.unwrap();
    let surface = h.app.surface_3d().unwrap().into_owned();
    let screen = view3d::screen_of(&surface, h.app.orbit, h.app.framing, fit, None);
    let z = surface.at(Point::new(20.0, 20.0)).unwrap();
    h.double_click(screen(cut_craft_core::glam::DVec3::new(
        20.0 - surface.origin.x,
        20.0 - surface.origin.y,
        z,
    )));
    assert_eq!(h.app.view_tool, Some(ViewTool::Transform));
    h.frame(vec![]);
    let at = |h: &Harness, which: Handle3d| {
        h.app
            .handles_3d
            .iter()
            .find(|(k, _)| *k == which)
            .map(|(_, at)| *at)
            .expect("the handles are drawn over the selected component")
    };
    let dome = |h: &Harness| h.app.doc.components[&EntityId(1)].clone();
    // dragging the height handle up a quarter of the view doubles the shape height; the
    // component follows the drag, and the whole drag is one undo
    let quarter = h.app.fit_3d.unwrap().height() / VIEW_ZOOM_QUARTERS;
    let depth = h.app.history.undo_depth();
    let from = at(&h, Handle3d::Height);
    let halfway = from - vec2(0.0, quarter / 2.0);
    h.frame(vec![Event::PointerMoved(from)]);
    h.press(from, true, Modifiers::NONE);
    h.frame(vec![Event::PointerMoved(halfway)]);
    h.frame(vec![Event::PointerMoved(halfway)]);
    let height = dome(&h).shaping.shape_height;
    assert!((height - 141.0).abs() < 5.0, "follows the drag: {height}");
    h.frame(vec![Event::PointerMoved(from - vec2(0.0, quarter))]);
    h.press(from - vec2(0.0, quarter), false, Modifiers::NONE);
    h.frame(vec![]);
    let height = dome(&h).shaping.shape_height;
    assert!((height - 200.0).abs() < 5.0, "{height}");
    assert_eq!(
        h.app.history.undo_depth(),
        depth + 1,
        "one undo for the drag"
    );
    assert!(h.app.drag_base.is_none(), "let go");
    h.app.invoke(&h.ctx, "undo");
    assert_eq!(dome(&h).shaping.shape_height, 100.0, "undoable");
    // the view is fitted to the model again, so the handles move
    h.frame(vec![]);
    // the base handle raises it on a solid bottom
    let from = at(&h, Handle3d::Base);
    h.drag(from, from - vec2(0.0, quarter));
    let base = dome(&h).shaping.base_height;
    assert!((base - h.app.doc.job.thickness).abs() < 0.5, "{base}");
    // the total height handle scales shape and base together
    h.frame(vec![]);
    let before = dome(&h);
    let from = at(&h, Handle3d::Total);
    h.drag(from, from - vec2(0.0, quarter));
    let after = dome(&h);
    assert!(
        (after.shaping.shape_height - 200.0).abs() < 5.0,
        "{after:?}"
    );
    assert!((after.shaping.base_height - 2.0 * before.shaping.base_height).abs() < 0.5);
    // the settings handle opens the popup, which works through Component Properties
    let settings = at(&h, Handle3d::Settings);
    h.click(settings, Modifiers::NONE);
    h.frame(vec![]);
    let subtract = h
        .app
        .settings_3d
        .as_ref()
        .expect("the settings popup")
        .radios
        .iter()
        .find(|(m, _)| *m == Combine::Subtract)
        .map(|(_, at)| *at)
        .expect("the combine radios, drawn as icons");
    h.click(subtract, Modifiers::NONE);
    h.frame(vec![]);
    assert_eq!(dome(&h).combine, Combine::Subtract);
    let fade = h.text_at("Fade").unwrap();
    h.click(fade, Modifiers::NONE);
    h.frame(vec![]);
    assert!(dome(&h).shaping.fade.is_some());
    h.app.invoke(&h.ctx, "undo");
    assert!(dome(&h).shaping.fade.is_none(), "one undo each");
    let close = h.text_at("Close").unwrap();
    h.click(close, Modifiers::NONE);
    h.frame(vec![]);
    assert!(h.text_at("Fade").is_none(), "closed");
}

#[test]
fn a_dragged_value_in_the_3d_settings_changes_the_component_as_it_goes() {
    let mut h = Harness::new();
    h.app.doc = cut_craft_core::testing::domed();
    h.app.doc.selected_components = [EntityId(1)].into();
    h.app.orbit = view3d::Orbit::TOP;
    h.app.view = View::ThreeD;
    h.app.invoke(&h.ctx, "transform");
    h.frame(vec![]);
    h.frame(vec![]);
    let settings = h
        .app
        .handles_3d
        .iter()
        .find(|(k, _)| *k == Handle3d::Settings)
        .map(|(_, at)| *at)
        .expect("the editing icons");
    h.click(settings, Modifiers::NONE);
    h.frame(vec![]);
    let tilt = h.text_at("Tilt").expect("the settings popup");
    h.click(tilt, Modifiers::NONE);
    h.frame(vec![]);
    let angle = |h: &Harness| {
        let k = &h.app.doc.components[&EntityId(1)];
        k.shaping.tilt.map(|t| t.angle)
    };
    assert!(angle(&h).is_some(), "tilted");
    let undo_from = h.app.history.undo_depth();
    let held = h.text_at("Combine with other components ...").unwrap();
    h.press(held, true, Modifiers::NONE);
    for step in 1..=3 {
        let to = 10.0 * f64::from(step);
        let s = h.app.settings_3d.as_mut().unwrap();
        s.params.tilt.angle = to;
        s.dirty = true;
        h.frame(vec![]);
        assert_eq!(angle(&h), Some(to), "follows the drag: {step}");
    }
    h.press(held, false, Modifiers::NONE);
    h.frame(vec![]);
    assert_eq!(h.app.history.undo_depth(), undo_from + 1, "one undo");
    h.app.invoke(&h.ctx, "undo");
    assert_ne!(angle(&h), Some(30.0), "the drag undone at once");
}

#[test]
fn set_in_the_3d_settings_places_the_fade_anchors_by_two_clicks() {
    let mut h = Harness::new();
    h.app.doc = cut_craft_core::testing::domed();
    h.app.doc.selected_components = [EntityId(1)].into();
    h.app.orbit = view3d::Orbit::TOP;
    h.app.view = View::ThreeD;
    h.app.invoke(&h.ctx, "transform");
    h.frame(vec![]);
    h.frame(vec![]);
    let settings = h
        .app
        .handles_3d
        .iter()
        .find(|(k, _)| *k == Handle3d::Settings)
        .map(|(_, at)| *at)
        .expect("the editing icons");
    h.click(settings, Modifiers::NONE);
    h.frame(vec![]);
    let fade = h.text_at("Fade").expect("the settings popup");
    h.click(fade, Modifiers::NONE);
    h.frame(vec![]);
    let set = h.texts_at("Set")[0];
    h.click(set, Modifiers::NONE);
    h.frame(vec![]);
    let anchors = |h: &Harness| h.app.settings_3d.as_ref().unwrap().anchors;
    assert_eq!(anchors(&h), Some((Anchor::Fade, None)));
    assert_eq!(h.app.status.message, Anchor::Fade.prompt(false));
    // two clicks on the dome, left then right of its middle
    let fit = h.app.fit_3d.unwrap();
    let surface = h.app.surface_3d().unwrap().into_owned();
    let screen = view3d::screen_of(&surface, h.app.orbit, h.app.framing, fit, None);
    let on_dome = |x: f64| {
        let z = surface.at(Point::new(x, 20.0)).unwrap();
        screen(cut_craft_core::glam::DVec3::new(
            x - surface.origin.x,
            20.0 - surface.origin.y,
            z,
        ))
    };
    h.click(on_dome(16.0), Modifiers::NONE);
    h.frame(vec![]);
    assert!(matches!(anchors(&h), Some((Anchor::Fade, Some(_)))));
    assert_eq!(h.app.status.message, Anchor::Fade.prompt(true));
    h.click(on_dome(24.0), Modifiers::NONE);
    h.frame(vec![]);
    h.frame(vec![]);
    let fade = h.app.doc.components[&EntityId(1)]
        .shaping
        .fade
        .expect("faded between the clicks");
    assert!(
        (fade.from.x - 16.0).abs() < 0.5 && (fade.to.x - 24.0).abs() < 0.5,
        "{fade:?}"
    );
    assert_eq!(anchors(&h), None, "placed");
}

#[test]
fn the_window_is_titled_cut_craft() {
    let h = Harness::new();
    assert_eq!(h.app.title, "Untitled - Cut Craft");
}

#[test]
fn the_align_objects_group_spaces_by_pick_order() {
    let mut h = Harness::new();
    h.rectangle(100.0, 10.0, 10.0, 10.0);
    let first = *h.app.doc.selection.iter().next().unwrap();
    h.rectangle(10.0, 10.0, 10.0, 10.0);
    h.rectangle(50.0, 10.0, 10.0, 10.0);
    let all: Vec<EntityId> = h.app.doc.vectors.keys().copied().collect();
    h.app.doc.selection = all.into_iter().collect();
    for id in [
        "left_to_right",
        "right_to_left",
        "top_to_bottom",
        "bottom_to_top",
    ] {
        assert!(h.app.available(id), "{id}");
    }
    h.app.invoke(&h.ctx, "left_to_right");
    let left = h.app.doc.vectors[&first].bounds().min.x;
    assert!(
        (left - 10.0).abs() < 1e-6,
        "the first picked goes leftmost: {left}"
    );
    let group = h
        .app
        .toolbar
        .groups
        .iter()
        .find(|g| g.name == "Align Objects")
        .unwrap();
    assert!(group.tools.contains(&"left_to_right") && group.tools.contains(&"align_left"));
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
        h.app.invoke(&h.ctx, id);
        let form = h.app.form.as_ref().expect("Create Shape's form");
        assert_eq!(
            (form.id, form.params["profile"].as_str()),
            ("create_shape", Some(profile))
        );
    }
}

#[test]
fn edit_toolpath_reopens_its_form_and_replaces_it() {
    let mut h = Harness::new();
    h.rectangle(10.0, 10.0, 50.0, 50.0);
    h.app.invoke(&h.ctx, "profile_toolpath");
    let params = h.app.form.as_ref().unwrap().params.clone();
    h.app.apply("profile_toolpath", &params).unwrap();
    h.app.form = None;
    let id = *h.app.doc.toolpaths.keys().next().unwrap();
    h.app.selected_toolpath = Some(id);
    h.app.doc.selection = cut_craft_core::selection::Selection::new();
    h.app.invoke(&h.ctx, "edit_toolpath");
    let form = h.app.form.as_mut().expect("the toolpath's form");
    assert_eq!((form.id, form.editing), ("profile_toolpath", Some(id)));
    form.params["name"] = Value::from("Edited");
    let params = form.params.clone();
    h.app.apply_form("profile_toolpath", &params).unwrap();
    assert_eq!(h.app.doc.toolpaths.len(), 1, "replaced, not added");
    assert_eq!(h.app.doc.toolpaths[&id].name, "Edited");
}

#[test]
fn a_drill_bank_toolpath_drills_with_the_machines_bank_and_says_when_there_is_none() {
    use cut_craft_core::drill_bank::DrillBank;
    use cut_craft_core::geometry::{Path, Point};
    let mut h = Harness::new();
    let mut c = cut_craft_core::Change::builder(&h.app.doc);
    let holes: Vec<_> = (0..3)
        .map(|i| {
            c.add(Path::circle(
                Point::new(20.0 + 32.0 * f64::from(i), 20.0),
                2.5,
            ))
        })
        .collect();
    c.select(holes);
    h.app.doc.apply(&c.finish()).unwrap();
    // no machine: the form opens and the toolpath says where the bank is set up
    h.app.invoke(&h.ctx, "drill_bank_toolpath");
    let params = h.app.form.as_ref().expect("its form").params.clone();
    let err = h.app.apply("drill_bank_toolpath", &params).unwrap_err();
    assert!(err.contains("Machine Configuration"), "{err}");
    // a machine with a bank of three: one plunge drills all three
    h.app.machines.add();
    h.app.machines.active_mut().unwrap().drill_bank = Some(DrillBank::new(3, 0, 32.0, 5.0));
    h.app.invoke(&h.ctx, "drill_bank_toolpath");
    let params = h.app.form.as_ref().unwrap().params.clone();
    h.app.apply("drill_bank_toolpath", &params).unwrap();
    h.app.form = None;
    let (&id, toolpath) = h.app.doc.toolpaths.iter().next().unwrap();
    assert_eq!(toolpath.drill_bank.as_ref().unwrap().hits.len(), 1);
    // editing it again picks up the bank as the machine has it now
    h.app.machines.active_mut().unwrap().drill_bank = Some(DrillBank::new(2, 0, 32.0, 5.0));
    h.app.selected_toolpath = Some(id);
    h.app.invoke(&h.ctx, "edit_toolpath");
    let form = h.app.form.as_ref().expect("the toolpath's form");
    assert_eq!(
        form.params["bank"]["horizontal"].as_array().unwrap().len(),
        2
    );
    // Cut Craft's own posts have no drill bank blocks: saving says so
    let post = h.app.output_post().unwrap();
    let out = std::env::temp_dir().join(format!("cut-craft-bank-{}.nc", std::process::id()));
    let err = files::save_through(&post, &h.app.slots(), &h.app.doc, &out, None).unwrap_err();
    assert!(err.contains("SETUP_DRILL_BANK"), "{err}");
    let _ = std::fs::remove_file(out);
}

#[test]
fn the_file_and_component_level_icons_have_their_tools() {
    let mut h = Harness::new();
    for id in [
        "import_vectors",
        "export_vectors",
        "open_folder",
        "open_clipart_folder",
        "component_levels",
    ] {
        assert!(h.app.available(id), "{id}");
    }
    let groups = &h.app.toolbar.groups;
    let file = groups.iter().find(|g| g.name == "File Operations").unwrap();
    assert!(file.tools.contains(&"export_vectors") && file.tools.contains(&"open_folder"));
    h.app.invoke(&h.ctx, "component_levels");
    assert_eq!(DESIGN_TABS[h.app.design_tab], "Components");
    // with no library folder there is nothing to open, and it says so
    h.app.invoke(&h.ctx, "open_clipart_folder");
    assert!(
        h.app.status.message.contains("Library first"),
        "{}",
        h.app.status.message
    );
}

#[test]
fn a_collapsed_level_hides_its_components() {
    let mut h = Harness::new();
    h.app.doc = cut_craft_core::testing::domed();
    h.app.pages.set_pinned(Page::Design, true);
    h.app.design_tab = DESIGN_TABS.iter().position(|&t| t == "Components").unwrap();
    h.frame(vec![]);
    let k = h.app.doc.components.values().next().unwrap();
    let name = format!("{} ({:?})", k.name, k.combine);
    assert!(h.text_at(&name).is_some(), "{name} listed");
    let level = *h.app.doc.levels.keys().next().unwrap();
    h.app.collapsed_levels.insert(level);
    h.frame(vec![]);
    assert!(
        h.text_at(&name).is_none(),
        "hidden under its collapsed level"
    );
}

#[test]
fn the_join_icons_join_two_vectors_their_way() {
    let mut h = Harness::new();
    let line =
        |x0: f64, x1: f64| Path::polyline(&[Point::new(x0, 10.0), Point::new(x1, 10.0)], false);
    let change = Change::swap(&h.app.doc, [], vec![line(0.0, 10.0), line(20.0, 30.0)]);
    h.app.doc.apply(&change).unwrap();
    for id in [
        "join_straight_line",
        "join_smooth_curve",
        "join_move_end_points",
    ] {
        assert!(h.app.available(id), "{id}");
    }
    h.app.invoke(&h.ctx, "join_smooth_curve");
    assert_eq!(h.app.doc.vectors.len(), 1, "{}", h.app.status.message);
    let edit = h
        .app
        .toolbar
        .groups
        .iter()
        .find(|g| g.name == "Join, Combine and Trim")
        .unwrap();
    assert!(edit.tools.contains(&"join_move_end_points"));
}

#[test]
fn edit_text_opens_draw_text_on_the_selected_text() {
    let mut h = Harness::new();
    assert!(!h.app.available("edit_text"), "no text selected");
    let params = serde_json::json!({ "text": "HI", "font": "Test Boxes" });
    let mut full = h.app.registry.get("draw_text").unwrap().default_params();
    for (k, v) in params.as_object().unwrap() {
        full[k] = v.clone();
    }
    h.app.apply("draw_text", &full).unwrap();
    assert!(h.app.available("edit_text"));
    h.app.invoke(&h.ctx, "edit_text");
    let form = h.app.form.as_ref().expect("Draw Text's form");
    assert_eq!(
        (form.id, form.params["text"].as_str()),
        ("draw_text", Some("HI"))
    );
}

#[test]
fn a_new_job_is_set_up_over_the_2d_view() {
    let mut h = Harness::at_start();
    h.app.view = View::ThreeD;
    h.app.invoke(&h.ctx, "new");
    assert!(
        h.app.view == View::TwoD,
        "the material shows while it is set up"
    );
}

#[test]
fn it_starts_on_the_start_page_with_the_no_job_menus() {
    let h = Harness::at_start();
    assert_eq!(h.app.screen, Screen::Start);
    assert_eq!(h.app.title, "Cut Craft", "no job, so no file name");
    assert_eq!(
        h.app.menu_names(),
        ["File", "Edit", "Machine", "Toolpaths", "Gadgets", "Help"]
    );
    assert!(
        h.app.view_transform.is_none(),
        "no 2D view before there is a job"
    );
}

#[test]
fn create_a_new_file_goes_through_job_setup_to_the_job() {
    let mut h = Harness::at_start();
    h.app.invoke(&h.ctx, "new");
    h.frame(vec![]);
    assert_eq!(h.app.screen, Screen::NewJob);
    let form = h.app.form.as_mut().expect("the Job Setup form");
    assert_eq!(form.id, "job_setup");
    form.params["size"]["width"] = 300.0.into();
    assert!(
        (h.app.shown_job().width - 300.0).abs() < 1e-9,
        "the 2D view shows the job being set up"
    );
    h.app.job_setup_ok();
    h.frame(vec![]);
    assert_eq!(h.app.screen, Screen::Job);
    assert!(h.app.form.is_none());
    assert!((h.app.doc.job.width - 300.0).abs() < 1e-9);
    assert!(
        !h.app.history.can_undo(),
        "a new job starts with nothing to undo"
    );
    assert_eq!(
        h.app.status.current_tool, "Select",
        "Job Setup is done with"
    );
    assert!(h.app.menu_names().contains(&"Model"));
    assert!(h.app.view_transform.is_some());
}

#[test]
fn the_start_page_says_whats_new_and_links_out() {
    let h = Harness::at_start();
    let heading = format!("What's new in v{}", env!("CARGO_PKG_VERSION"));
    assert!(h.text_at(&heading).is_some(), "{heading}");
    let first = WHATS_NEW.lines().find(|l| !l.trim().is_empty()).unwrap();
    assert!(
        h.text_at(first.trim_start_matches("- ")).is_some(),
        "{first}"
    );
    assert!(h.text_at("•").is_some(), "each change is bulleted");
    // the links themselves are in Help ▸ Community (see
    // the_community_links_are_in_help_not_on_the_start_page)
    for (_, url, _) in LINKS {
        assert!(url.starts_with("https://"), "{url}");
    }
}

#[test]
fn cancelling_job_setup_goes_back_to_the_start_page() {
    let mut h = Harness::at_start();
    h.app.invoke(&h.ctx, "new");
    h.app.job_setup_cancel();
    h.frame(vec![]);
    assert_eq!(h.app.screen, Screen::Start);
    assert!(h.app.form.is_none());
}

/// The icons built into the program.
fn icon_set() -> std::collections::HashSet<String> {
    crate::icons::built_in::BUILT_IN
        .iter()
        .map(|(key, _)| (*key).to_owned())
        .collect()
}

#[test]
fn start_page_and_form_pictures_are_in_the_icon_set() {
    let known = icon_set();
    for (_, _, icon) in STARTUP_TASKS {
        assert!(known.contains(icon), "{icon}");
    }
    assert!(known.contains(RECENT_FILE_ICON));
    // every picture these forms can show, whatever is chosen
    let forms = [
        "job_setup",
        "orientate_3d_model",
        "draw_rectangle",
        "draw_circle",
        "draw_polygon",
        "draw_star",
    ];
    let fields: Vec<form::Field> = forms
        .iter()
        .flat_map(|id| form::fields(&commands::registry().get(id).unwrap().schema()))
        .collect();
    let mut pictures = 0;
    for f in &fields {
        let Some(template) = &f.icon else { continue };
        let choices = |kind: &form::Kind| match kind {
            form::Kind::Choice(options, _) => options.iter().map(|o| o.value.clone()).collect(),
            _ => vec![],
        };
        let values: Vec<Value> = match &f.kind {
            form::Kind::Choice(..) => choices(&f.kind).into_iter().map(Value::from).collect(),
            form::Kind::Group(inner) => {
                let each: Vec<Value> = inner
                    .iter()
                    .flat_map(|i| {
                        choices(&i.kind)
                            .into_iter()
                            .map(|c| serde_json::json!({ i.key.as_str(): c }))
                    })
                    .collect();
                // a section of numbers only has the one picture
                if each.is_empty() {
                    vec![Value::Null]
                } else {
                    each
                }
            }
            _ => vec![Value::Null],
        };
        for value in values {
            let path = form::icon_path(template, &value);
            if !path.contains('{') {
                assert!(known.contains(&path), "{} shows {path}", f.key);
                pictures += 1;
            }
        }
    }
    assert!(
        pictures >= 9 + 3 + 12,
        "Job Setup's, Orientate 3D Model's and the shape tools' pictures: {pictures}"
    );
}

#[test]
fn closing_the_job_goes_back_to_the_start_page() {
    let mut h = Harness::new();
    h.app.invoke(&h.ctx, "close");
    h.frame(vec![]);
    assert_eq!(h.app.screen, Screen::Start);
}

#[test]
fn saved_and_opened_jobs_are_listed_as_recent_files() {
    let mut h = Harness::new();
    let dir = std::env::temp_dir().join(format!("cut-craft-recent-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    h.app.recent_path = Some(dir.join("recent.json"));
    let job = dir.join("first.ccjob");
    h.app.save_to(&job);
    assert_eq!(h.app.recent, std::slice::from_ref(&job));
    assert_eq!(
        files::load_recent(&dir.join("recent.json")),
        std::slice::from_ref(&job)
    );
    h.app.invoke(&h.ctx, "close");
    h.app.invoke(&h.ctx, "open_recent:0");
    assert_eq!(h.app.screen, Screen::Job);
    assert_eq!(h.app.session.path.as_deref(), Some(job.as_path()));
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn a_recent_file_that_has_gone_is_dropped() {
    let mut h = Harness::at_start();
    h.app.recent = vec![std::env::temp_dir().join("cut-craft-gone.ccjob")];
    h.app.invoke(&h.ctx, "open_recent:0");
    assert_eq!(h.app.screen, Screen::Start);
    assert_eq!(h.app.recent, Vec::<std::path::PathBuf>::new());
    assert!(h.app.status.message.contains("cut-craft-gone.ccjob"));
}

/// The middle of the 2D view on screen.
fn view_middle(h: &Harness) -> Pos2 {
    h.app
        .view_area
        .expect("the 2D view has been drawn")
        .center()
}

#[test]
fn the_wheel_zooms_about_the_cursor() {
    let mut h = Harness::new();
    let at = view_middle(&h) + vec2(40.0, -30.0);
    let (before, under) = {
        let t = h.app.view_transform.unwrap();
        (t.scale, t.to_job(at))
    };
    h.frame(vec![Event::PointerMoved(at)]);
    h.frame(vec![Event::MouseWheel {
        unit: egui::MouseWheelUnit::Point,
        delta: vec2(0.0, 120.0),
        phase: egui::TouchPhase::Move,
        source: egui::MouseWheelSource::Wheel,
        modifiers: Modifiers::NONE,
    }]);
    h.frame(vec![]);
    let t = h.app.view_transform.unwrap();
    assert!(t.scale > before, "scrolling up zooms in");
    assert!(t.to_job(at).distance(under) < 0.01, "about the cursor");
}

#[test]
fn the_middle_button_and_the_pan_tool_pan() {
    let mut h = Harness::new();
    let p = Point::new(100.0, 100.0);
    let before = h.app.view_transform.unwrap().to_screen(p);
    let from = view_middle(&h);
    h.frame(vec![Event::PointerMoved(from)]);
    let middle = |pressed: bool| Event::PointerButton {
        pos: from + vec2(40.0, 10.0) * f32::from(u8::from(!pressed)),
        button: PointerButton::Middle,
        pressed,
        modifiers: Modifiers::NONE,
    };
    h.frame(vec![middle(true)]);
    h.frame(vec![Event::PointerMoved(from + vec2(20.0, 5.0))]);
    h.frame(vec![Event::PointerMoved(from + vec2(40.0, 10.0))]);
    h.frame(vec![middle(false)]);
    h.frame(vec![]);
    let after = h.app.view_transform.unwrap().to_screen(p);
    assert!(
        (after - before - vec2(40.0, 10.0)).length() < 1.0,
        "{before:?} → {after:?}"
    );
    // the Pan tool does the same with the left button, and draws nothing
    h.app.invoke(&h.ctx, "pan");
    let before = h.app.view_transform.unwrap().to_screen(p);
    h.drag(from, from + vec2(-30.0, 0.0));
    let after = h.app.view_transform.unwrap().to_screen(p);
    assert!(
        (after - before - vec2(-30.0, 0.0)).length() < 1.0,
        "{before:?} → {after:?}"
    );
    assert!(h.app.doc.vectors.is_empty() && h.app.doc.selection.is_empty());
}

#[test]
fn zoom_box_zoom_to_selection_and_zoom_to_job() {
    let mut h = Harness::new();
    let fitted = h.app.view_transform.unwrap().scale;
    h.app.invoke(&h.ctx, "zoom_box");
    h.drag(h.at(100.0, 100.0), h.at(200.0, 150.0));
    let boxed = h.app.view_transform.unwrap();
    assert!(boxed.scale > 3.0 * fitted, "the box fills the view");
    assert!(h.app.doc.selection.is_empty(), "zooming selects nothing");
    let area = h.app.view_area.unwrap();
    assert!(area.contains(boxed.to_screen(Point::new(150.0, 125.0))));
    h.rectangle(400.0, 300.0, 20.0, 10.0);
    h.app.invoke(&h.ctx, "zoom_selected");
    h.frame(vec![]);
    let t = h.app.view_transform.unwrap();
    let (a, b) = (
        t.to_screen(Point::new(400.0, 300.0)),
        t.to_screen(Point::new(420.0, 310.0)),
    );
    assert!(area.contains(a) && area.contains(b));
    assert!(
        (b.x - a.x) > area.width() / 2.0,
        "the selection fills the view"
    );
    h.app.invoke(&h.ctx, "zoom_to_job");
    h.frame(vec![]);
    assert!((h.app.view_transform.unwrap().scale - fitted).abs() < 1e-3);
}

#[test]
fn the_side_tabs_are_aspire_12s_with_drawing_and_modeling_on_design() {
    assert_eq!(
        DESIGN_TABS,
        ["Design", "Sheets", "Layers", "Components", "Library"]
    );
    let h = Harness::new();
    let names: Vec<&str> = h
        .app
        .design_groups("Design")
        .iter()
        .map(|g| g.name)
        .collect();
    assert!(
        names.contains(&"Create Vectors") && names.contains(&"Shape Creation"),
        "{names:?}"
    );
    assert!(h.app.design_groups("Components").is_empty());
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
fn the_clipart_library_button_opens_the_library_tab() {
    let mut h = Harness::new();
    assert!(h.app.available("clipart_library"));
    // picked from a flown-out Design page, the page stays out on the Library tab
    h.app.pages.set_pinned(Page::Design, false);
    h.app.pages.open(Page::Design);
    h.app.pick(&h.ctx, Page::Design, "clipart_library");
    assert_eq!(DESIGN_TABS[h.app.design_tab], "Library");
    assert_ne!(h.app.pages.shown(Page::Design), Shown::Hidden);
    assert_eq!(h.app.tool_label("import_cut_list"), "Import Cut List");
}

#[test]
fn a_tool_that_works_on_a_selection_says_what_to_select_first() {
    let h = Harness::new();
    let tip = h
        .app
        .tool_tip("pocket_toolpath", gui::about("pocket_toolpath").unwrap());
    assert_eq!(tip.label, "Pocket Toolpath");
    assert_eq!(
        tip.next.as_deref(),
        Some("Select the closed vectors to pocket"),
        "the toolpath's own words"
    );
    assert!(
        !tip.about.starts_with("Pocket Toolpath"),
        "the description loses the label the box shows over it: {}",
        tip.about
    );
    let tip = h
        .app
        .tool_tip("align_left", gui::about("align_left").unwrap());
    assert_eq!(
        tip.next.as_deref(),
        Some(guide::SELECT_FIRST),
        "a tool that says only that nothing is selected"
    );
    let tip = h
        .app
        .tool_tip("draw_circle", gui::about("draw_circle").unwrap());
    assert_eq!(
        tip.next.as_deref(),
        Some("Drag from the centre out to the radius; Shift-click one this tool drew to edit it"),
        "a drawing tool says how to draw with it"
    );
    assert_eq!(
        tip.status(),
        "Draw Circle: Drag from the centre out to the radius; Shift-click one this tool drew to \
         edit it",
        "the status bar's line"
    );
    let tip = h.app.tool_tip("undo", "Undo");
    assert_eq!(tip.next, None, "a tool that just runs has no next step");
    assert_eq!(tip.status(), "Undo");
}

#[test]
fn every_tool_on_the_pages_can_be_asked_its_first_step() {
    let h = Harness::new();
    let mut with_step = 0;
    for group in h
        .app
        .toolbar
        .all_design_groups()
        .iter()
        .chain(&h.app.toolbar.toolpath_groups)
    {
        for id in group.tools {
            if h.app.first_step_of(id).is_some() {
                with_step += 1;
            }
        }
    }
    assert!(with_step >= 40, "{with_step} tools say what to select");
    for id in ["weld_vectors", "two_rail_sweep", "profile_toolpath", "move"] {
        assert!(h.app.first_step_of(id).is_some(), "{id}");
    }
    for id in ["draw_rectangle", "job_setup", "zoom_to_job", "undo"] {
        assert_eq!(h.app.first_step_of(id), None, "{id}");
    }
}

#[test]
fn a_form_opens_saying_what_to_select_until_something_is() {
    let mut h = Harness::new();
    h.app.invoke(&h.ctx, "pocket_toolpath");
    assert_eq!(
        h.app.form_guidance().as_deref(),
        Some("Select the closed vectors to pocket")
    );
    assert_eq!(
        h.app.next_step().as_deref(),
        Some("Select the closed vectors to pocket"),
        "the status bar's step in the view says the same"
    );
    h.app.pages.set_pinned(Page::Toolpaths, true);
    h.frame(vec![]);
    assert!(h.text_at("Next:").is_some(), "the form shows it at the top");
    assert!(h.text_at("Select the closed vectors to pocket").is_some());
    h.rectangle(10.0, 10.0, 20.0, 10.0);
    h.app.invoke(&h.ctx, "pocket_toolpath");
    assert_eq!(
        h.app.form_guidance(),
        None,
        "with a selection, the settings are the next step"
    );
    h.app.invoke(&h.ctx, "draw_circle");
    assert_eq!(
        h.app.form_guidance().as_deref(),
        Some("Drag from the centre out to the radius; Shift-click one this tool drew to edit it"),
        "a drawing tool's form says how to draw"
    );
    h.app.invoke(&h.ctx, "job_setup");
    assert_eq!(h.app.form_guidance(), None, "a settings form has no step");
}

#[test]
fn the_design_page_shows_the_hovered_tools_guidance_under_its_icons() {
    let mut h = Harness::new();
    h.app.pages.set_pinned(Page::Design, true);
    h.frame(vec![]);
    h.frame(vec![]);
    // nothing hovered: the tool in hand, Select, and its next step
    assert!(
        h.text_at("Select").is_some(),
        "the tip box names the tool in hand"
    );
    assert!(h.text_at("Next:").is_some());
    assert!(
        h.texts_at("Next:").len() == 1,
        "one box: {:?}",
        h.texts_at("Next:")
    );
    // hovering Draw Circle's button: its description, its next step, and the status bar's line
    h.app.options.general.fold_tool_groups = true;
    h.frame(vec![]);
    flyout_arrow_naming(&mut h, "Draw Circle");
    let hovered = guide::hovered(&h.ctx).expect("a hovered tool");
    assert_eq!(hovered.label, "Draw Circle");
    assert!(
        hovered
            .next
            .as_deref()
            .unwrap()
            .starts_with("Drag from the centre"),
        "{hovered:?}"
    );
    assert!(h.text_at("Draw Circle").is_some(), "the box shows its name");
    let hint = cut_craft_bug_report::hint::current(&h.ctx).unwrap_or_default();
    assert!(
        hint.starts_with("Draw Circle: Drag from the centre"),
        "{hint}"
    );
}

#[test]
fn the_rails_tool_rows_and_a_flyouts_list_carry_the_guidance_too() {
    let mut h = Harness::new();
    h.app.options.general.layout = crate::options::WindowLayout::Rail;
    h.frame(vec![]);
    let groups = h.app.all_groups();
    h.app.rail_group = groups.iter().position(|g| g.name == "Create Vectors");
    h.frame(vec![]);
    // the Rail's list of Create Vectors: its rows name each tool
    let row = h.text_at("Draw Rectangle").expect("a tool row");
    h.frame(vec![Event::PointerMoved(row)]);
    h.frame(vec![]);
    let hovered = guide::hovered(&h.ctx).expect("the row's tip");
    assert_eq!(hovered.label, "Draw Rectangle");
    assert!(
        hovered
            .next
            .as_deref()
            .unwrap()
            .starts_with("Drag from one corner"),
        "{hovered:?}"
    );
    let hint = cut_craft_bug_report::hint::current(&h.ctx).unwrap_or_default();
    assert!(
        hint.starts_with("Draw Rectangle: Drag from one corner"),
        "{hint}"
    );
}

#[test]
fn the_components_tab_is_the_component_list() {
    let mut h = Harness::new();
    h.app.pages = Pages::design_layout();
    h.app.design_tab = DESIGN_TABS.iter().position(|&t| t == "Components").unwrap();
    h.frame(vec![]);
    assert!(h.text_at("No components yet").is_some());
}

#[test]
fn the_view_buttons_sit_at_the_top_right_of_the_2d_view_starting_with_pan() {
    let mut h = Harness::new();
    // text buttons, as where the icons are missing
    h.app.icons = Icons::load(std::path::Path::new("/no/icons"));
    h.frame(vec![]);
    h.frame(vec![]);
    let area = h.app.view_area.unwrap();
    let over_view = |at: &Pos2| area.contains(*at);
    let pan = *h
        .texts_at("Pan")
        .iter()
        .find(|p| over_view(p))
        .expect("Pan over the view");
    let selection = h.text_at("Zoom to Selection").unwrap();
    assert!(
        pan.x > area.center().x && selection.x > pan.x,
        "on the right, Pan first: {pan:?} {selection:?} in {area:?}"
    );
    assert!(pan.y < area.center().y, "along the top: {pan:?}");
    h.click(pan, Modifiers::NONE);
    assert_eq!(h.app.view_tool, Some(ViewTool::Pan));
}

#[test]
fn the_view_buttons_are_the_view_actions() {
    for (id, _, icon) in VIEW_BUTTONS {
        assert!(icon_set().contains(icon), "{icon}");
        assert!(BUILT_INS.contains(&id), "{id}");
    }
}

fn key(key: Key) -> Event {
    Event::Key {
        key,
        physical_key: None,
        pressed: true,
        repeat: false,
        modifiers: Modifiers::NONE,
    }
}

#[test]
fn tab_moves_a_visible_focus_through_the_controls() {
    let mut h = Harness::new();
    h.frame(vec![]);
    assert_eq!(focus_ring(&h.ctx), None, "nothing focused yet");
    h.frame(vec![key(Key::Tab)]);
    h.frame(vec![]);
    let first = focus_ring(&h.ctx).expect("Tab focuses a control, ringed");
    h.frame(vec![key(Key::Tab)]);
    h.frame(vec![]);
    let second = focus_ring(&h.ctx).expect("the next control");
    assert_ne!(first, second, "Tab moves on");
}

#[test]
fn a_preview_plays_pauses_scrubs_and_keeps_what_is_cut() {
    let mut h = Harness::new();
    h.rectangle(10.0, 10.0, 50.0, 50.0);
    h.app.invoke(&h.ctx, "profile_toolpath");
    let params = h.app.form.as_ref().unwrap().params.clone();
    h.app.apply("profile_toolpath", &params).unwrap();
    h.app.form = None;
    h.app.display.animate_preview = true;
    h.app.invoke(&h.ctx, "preview_all_toolpaths");
    assert!(h.app.playing && h.app.playback.is_some(), "it plays");
    assert_eq!(h.app.view, View::ThreeD);
    h.frame(vec![]);
    h.frame(vec![]);
    let along = h.app.playback.as_ref().unwrap().at();
    assert!(along > 0.0, "the tool moves on each frame");
    assert!(h.text_at("Pause").is_some() && h.text_at("Run to Retract").is_some());
    h.click(h.text_at("Pause").unwrap(), Modifiers::NONE);
    assert!(!h.app.playing);
    let paused = h.app.playback.as_ref().unwrap().at();
    h.frame(vec![]);
    assert_eq!(
        h.app.playback.as_ref().unwrap().at(),
        paused,
        "paused stays put"
    );
    // the Progress slider, dragged right, moves the tool on
    let progress = h.text_at("Progress").expect("a Progress slider");
    let from = progress + egui::vec2(80.0, 0.0);
    h.drag(from, from + egui::vec2(200.0, 0.0));
    let on = h.app.playback.as_ref().unwrap().at();
    assert!(on > paused, "dragged on: {paused} to {on}");
    assert!(!h.app.playing, "a drag pauses it");
    h.click(h.text_at("Stop").unwrap(), Modifiers::NONE);
    assert!(
        h.app.playback.is_none() && h.app.simulation.is_some(),
        "kept as far as it went"
    );
}

/// A profile toolpath around a 40 mm square, previewed playing.
fn playing_preview() -> Harness {
    let mut h = Harness::new();
    h.rectangle(10.0, 10.0, 50.0, 50.0);
    h.app.invoke(&h.ctx, "profile_toolpath");
    let params = h.app.form.as_ref().unwrap().params.clone();
    h.app.apply("profile_toolpath", &params).unwrap();
    h.app.form = None;
    h.app.display.animate_preview = true;
    h.app.invoke(&h.ctx, "preview_all_toolpaths");
    assert!(h.app.playing && h.app.playback.is_some(), "it plays");
    h.frame(vec![]);
    h
}

#[test]
fn escape_stops_a_playing_preview_keeping_what_is_cut_and_leaves_the_form_open() {
    let mut h = playing_preview();
    h.app.invoke(&h.ctx, "profile_toolpath");
    h.frame(vec![]);
    h.frame(vec![key(Key::Escape)]);
    assert!(
        h.app.playback.is_none() && h.app.simulation.is_some(),
        "stopped, kept as far as it went"
    );
    assert!(
        h.app.form.is_some(),
        "the one Escape only stopped the preview"
    );
    h.frame(vec![key(Key::Escape)]);
    assert!(h.app.form.is_none(), "the next closes the form as ever");
}

#[test]
fn a_preview_can_be_stopped_from_the_2d_view_too() {
    let mut h = playing_preview();
    h.app.view = View::TwoD;
    h.frame(vec![]);
    let along = h.app.playback.as_ref().unwrap().at();
    h.frame(vec![]);
    assert!(
        h.app.playback.as_ref().unwrap().at() > along,
        "it plays on in the 2D view"
    );
    h.click(
        h.text_at("Stop").expect("Stop over the 2D view"),
        Modifiers::NONE,
    );
    assert!(h.app.playback.is_none() && h.app.simulation.is_some());
}

#[test]
fn a_playing_preview_says_how_long_it_has_left() {
    let mut h = playing_preview();
    h.app.playback_speed = 1.0;
    h.frame(vec![]);
    let note = |h: &Harness| {
        h.texts()
            .into_iter()
            .find(|t| t.starts_with("Machine time left"))
            .expect("the time the machine takes")
    };
    assert!(!note(&h).contains(" at ×"), "{}", note(&h));
    h.app.playback_speed = 10.0;
    h.frame(vec![]);
    assert!(note(&h).contains(" at ×10"), "{}", note(&h));
}

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
fn the_preview_controls_stay_after_stop_to_scrub_back_until_reset() {
    let mut h = playing_preview();
    h.click(h.text_at("Stop").unwrap(), Modifiers::NONE);
    assert!(
        h.app.playback.is_none() && h.app.simulation.is_some(),
        "stopped"
    );
    assert!(h.text_at("Progress").is_some(), "the slider is still there");
    assert!(h.text_at("Close up").is_some(), "and the fine one");
    // the slider dragged back: playing it again from there, paused
    let progress = h.text_at("Progress").unwrap();
    let from = progress + egui::vec2(200.0, 0.0);
    h.drag(from, from - egui::vec2(100.0, 0.0));
    assert!(
        h.app.playback.is_some() && !h.app.playing,
        "scrubbing again"
    );
    h.app.invoke(&h.ctx, "reset_preview");
    h.frame(vec![]);
    assert!(h.text_at("Progress").is_none(), "gone with the preview");
}

#[test]
fn the_preview_controls_stay_once_it_has_played_to_the_end() {
    let mut h = playing_preview();
    h.app.playback_speed = 1000.0;
    for _ in 0..600 {
        if h.app.playback.is_none() {
            break;
        }
        h.frame(vec![]);
    }
    assert!(
        h.app.playback.is_none() && h.app.simulation.is_some(),
        "played through"
    );
    h.frame(vec![]);
    assert!(h.text_at("Progress").is_some(), "still there to scrub back");
    h.click(
        h.text_at("Close").expect("Close hides them"),
        Modifiers::NONE,
    );
    assert!(h.text_at("Progress").is_none() && h.app.simulation.is_some());
}

#[test]
fn a_long_jump_is_cut_a_little_each_frame_and_stop_ends_it() {
    let mut h = playing_preview();
    // a step a frame, as a toolpath of millions of moves would go
    h.app.playback_budget = std::time::Duration::ZERO;
    h.click(h.text_at("Pause").unwrap(), Modifiers::NONE);
    let from = h.app.playback.as_ref().unwrap().at();
    h.click(h.text_at("Run to Retract").unwrap(), Modifiers::NONE);
    let goal = h.app.playback_goal.expect("on its way there");
    let at = h.app.playback.as_ref().unwrap().at();
    assert!(at > from && at < goal, "part way: {from} to {at} of {goal}");
    assert!(
        h.texts().iter().any(|t| t.contains("Esc")),
        "says how to stop it"
    );
    h.click(h.text_at("Stop").unwrap(), Modifiers::NONE);
    assert!(h.app.playback.is_none() && h.app.playback_goal.is_none());
}

#[test]
fn escape_cancels_a_slow_job() {
    let mut h = Harness::new();
    h.app.wait_before_showing = std::time::Duration::ZERO;
    let (_go, wait) = std::sync::mpsc::channel::<()>();
    h.app.in_background(
        "Previewing Slow Toolpath",
        move || {
            wait.recv().ok();
        },
        |app, ()| app.status.set_message("used"),
    );
    h.frame(vec![]);
    h.frame(vec![key(Key::Escape)]);
    h.frame(vec![]);
    assert!(h.app.busy.is_none(), "cancelled");
}

#[test]
fn any_toolpath_rest_machines_after_the_tools_named_in_its_form() {
    let mut h = Harness::new();
    h.rectangle(10.0, 10.0, 40.0, 40.0);
    h.app.invoke(&h.ctx, "pocket_toolpath");
    // the section, drawn on its own (it is below the fold of the pocket form)
    let ctx = egui::Context::default();
    let mut section = serde_json::json!({});
    let out = headless(ctx.run_ui(RawInput::default(), |ui| {
        egui::CentralPanel::default()
            .show(ui, |ui| rest_section(ui, &mut section, &h.app.tools, &[]));
    }));
    let drawn = |text: &str| {
        out.shapes
            .iter()
            .any(|c| matches!(&c.shape, egui::Shape::Text(t) if t.galley.text() == text))
    };
    assert!(drawn("Rest Machining") && drawn("Add a tool…"));
    let mut params = h.app.form.as_ref().unwrap().params.clone();
    h.app.apply("pocket_toolpath", &params).unwrap();
    let whole = h.app.doc.toolpaths.values().last().unwrap().moves.len();
    let mut big = h.app.form.as_ref().unwrap().params["tool"].clone();
    big["diameter"] = Value::from(params["tool"]["diameter"].as_f64().unwrap() * 4.0);
    params["rest"] = serde_json::json!({ "after": [big], "minimum_detail": 0.0, "overlap": 0.0 });
    h.app.apply("pocket_toolpath", &params).unwrap();
    let rest = h.app.doc.toolpaths.values().last().unwrap();
    assert!(
        rest.moves.len() < whole,
        "{} of {whole} moves",
        rest.moves.len()
    );
    assert!(
        cut_craft_core::rest::Rest::of(&rest.params).is_some(),
        "kept to recalculate"
    );
}

#[test]
fn the_arrow_keys_nudge_the_selection() {
    let mut h = Harness::new();
    h.rectangle(10.0, 10.0, 20.0, 20.0);
    let square = h.app.options.snapping.grid_spacing;
    let at = |h: &Harness| h.app.doc.selection_bounds().unwrap().min;
    let before = at(&h);
    h.frame(vec![key(Key::ArrowRight)]);
    h.frame(vec![key(Key::ArrowUp)]);
    let moved = at(&h) - before;
    assert!(
        (moved.x - square).abs() < 1e-9 && (moved.y - square).abs() < 1e-9,
        "{moved:?}"
    );
    h.frame(vec![key(Key::ArrowLeft), key(Key::ArrowDown)]);
    assert!((at(&h) - before).length() < 1e-9, "back where it was");
    // Ctrl alone: Snapping Options' Fixed Nudge Distance
    h.app.options.snapping.fixed_nudge = 0.25;
    for (modifiers, share) in [
        (Modifiers::SHIFT, 0.5),
        (Modifiers::SHIFT | Modifiers::COMMAND, 0.1),
        (Modifiers::COMMAND, 0.25 / square),
    ] {
        let from = at(&h);
        let press = Event::Key {
            key: Key::ArrowRight,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers,
        };
        h.frame_holding(modifiers, vec![press]);
        let moved = (at(&h) - from).x;
        assert!(
            (moved - square * share).abs() < 1e-9,
            "{modifiers:?}: {moved}"
        );
    }
}

#[test]
fn f11_designs_f12_machines_and_switch_layout_swaps() {
    let mut h = Harness::new();
    assert_eq!(h.app.pages, Pages::design_layout());
    h.frame(vec![key(Key::F12)]);
    assert_eq!(h.app.pages, Pages::machining_layout());
    h.frame(vec![key(Key::F11)]);
    assert_eq!(h.app.pages, Pages::design_layout());
    h.app.invoke(&h.ctx, "interface_layout");
    assert_eq!(
        h.app.pages,
        Pages::machining_layout(),
        "the Switch Layout button"
    );
}

#[test]
fn a_tool_from_a_flown_out_page_hides_it_unless_it_opens_a_form() {
    let mut h = Harness::new();
    h.app.pages = Pages::machining_layout();
    h.app.pages.open(Page::Design);
    h.app.pick(&h.ctx, Page::Design, "draw_rectangle");
    assert_eq!(
        h.app.pages.shown(Page::Design),
        Shown::Flyout,
        "its form shows there"
    );
    h.app.form = None;
    h.app.pick(&h.ctx, Page::Design, "select_all_vectors");
    assert_eq!(h.app.pages.shown(Page::Design), Shown::Hidden);
}

#[test]
fn a_tools_form_takes_the_place_of_the_page_as_in_aspire() {
    let mut h = Harness::new();
    h.app.pages.set_pinned(Page::Design, true);
    h.app.pages.set_pinned(Page::Toolpaths, true);
    h.frame(vec![]);
    assert!(h.text_at("Create Vectors").is_some());
    assert!(h.text_at("Toolpath List").is_some());
    h.app.invoke(&h.ctx, "draw_circle");
    h.frame(vec![]);
    assert!(h.text_at("Create Vectors").is_none(), "design tools hidden");
    assert!(h.text_at("Toolpath List").is_some(), "the other page stays");
    h.app.invoke(&h.ctx, "profile_toolpath");
    h.frame(vec![]);
    assert!(
        h.text_at("Create Vectors").is_some(),
        "back once its form went"
    );
    assert!(h.text_at("Toolpath List").is_none(), "toolpath page hidden");
}

#[test]
fn the_sheets_tab_lists_each_sheets_size_and_activates_one() {
    use cut_craft_core::document::Units;
    let mut h = Harness::new();
    h.app.pages.set_pinned(Page::Design, true);
    h.app.design_tab = 1;
    h.app.doc = cut_craft_core::testing::two_sheets();
    // the list's grid measures its columns on the first frame
    h.frame(vec![]);
    h.frame(vec![]);
    assert!(h.text_at("Sheets List").is_some());
    assert!(h.text_at("Sheet 2").is_some());
    assert!(
        h.text_at(&sheet_length(300.0, Units::Mm)).is_some(),
        "Sheet 2's X"
    );
    assert!(
        h.text_at(&sheet_length(12.0, Units::Mm)).is_some(),
        "Sheet 2's Z"
    );
    assert!(h.text_at("Add New Sheet...").is_some());
    let at = h.text_at("Sheet 2").unwrap();
    h.click(at, Modifiers::NONE);
    assert_eq!(h.app.doc.active_sheet, 1, "clicking a sheet activates it");
    assert_eq!(h.app.doc.job.width, 300.0);
}

#[test]
fn the_clipart_tab_shows_a_library_folder_and_imports_what_is_double_clicked() {
    let dir = std::env::temp_dir().join(format!("cut-craft-clipart-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("flower.bmp"), crate::icons::tiny_bmp()).unwrap();
    std::fs::write(dir.join("notes.txt"), "not clipart").unwrap();
    let mut h = Harness::new();
    h.app.pages.set_pinned(Page::Design, true);
    h.app.design_tab = 4;
    h.app.clipart.folders = vec![dir.clone()];
    h.frame(vec![]);
    let importable = ["bmp".to_owned()].into();
    h.app.clipart.read_now(&importable);
    h.frame(vec![]);
    let folder = dir.file_name().unwrap().to_string_lossy().into_owned();
    assert!(
        h.text_at(&format!("{folder} (1)")).is_some(),
        "the library lists the folder, with how many files are in it"
    );
    assert!(
        h.text_at("notes.txt").is_none(),
        "only what can be imported"
    );
    let at = h
        .text_at("flower.bmp")
        .expect("the picture is in the browser");
    h.double_click(at);
    assert_eq!(h.app.doc.bitmaps.len(), 1, "double-clicking imports it");
    // dragged onto the 2D view, it goes where it is dropped
    let to = h.at(120.0, 90.0);
    h.drag(at, to);
    assert_eq!(h.app.doc.bitmaps.len(), 2, "dropping imports another");
    let dropped = h.app.doc.bitmaps.values().last().unwrap().rect;
    assert!(
        dropped
            .center()
            .distance(h.app.view_transform.unwrap().to_job(to))
            < 1e-6,
        "centred where dropped: {dropped:?}"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn the_top_bar_splits_the_views_and_toggles_snapping() {
    let mut h = Harness::new();
    h.app.doc = cut_craft_core::testing::two_sheets();
    h.frame(vec![]);
    for name in ["Sheet 1", "Layer 1", "Level 1"] {
        assert!(h.text_at(name).is_some(), "{name} in the top bar");
    }
    h.app.invoke(&h.ctx, "split_side_by_side");
    assert_eq!(h.app.split, Some(Split::SideBySide));
    h.frame(vec![]);
    h.app.invoke(&h.ctx, "split_side_by_side");
    assert_eq!(h.app.split, None, "a second click joins them again");
    let before = h.app.options.snapping.clone();
    for id in [
        "toggle_geometry_snapping",
        "toggle_smart_snapping",
        "toggle_grid_snapping",
    ] {
        assert!(h.app.available(id), "{id}");
        h.app.invoke(&h.ctx, id);
    }
    let s = &h.app.options.snapping;
    assert_eq!(
        (s.geometry_snapping, s.smart_snapping, s.snap_to_grid),
        (
            !before.geometry_snapping,
            !before.smart_snapping,
            !before.snap_to_grid
        )
    );
}

#[test]
fn toolpath_forms_open_in_the_toolpaths_page() {
    let mut h = Harness::new();
    h.app.invoke(&h.ctx, "profile_toolpath");
    assert_eq!(h.app.form.as_ref().unwrap().page, Page::Toolpaths);
    assert_eq!(
        h.app.pages.shown(Page::Toolpaths),
        Shown::Flyout,
        "brought out to show its form"
    );
    h.app.invoke(&h.ctx, "draw_circle");
    assert_eq!(h.app.form.as_ref().unwrap().page, Page::Design);
}

#[test]
fn clicking_the_view_puts_a_flown_out_page_away() {
    let mut h = Harness::new();
    h.app.pages.open(Page::Toolpaths);
    h.frame(vec![]);
    h.click(h.at(20.0, 200.0), Modifiers::NONE);
    assert_eq!(h.app.pages.shown(Page::Toolpaths), Shown::Hidden);
    assert_eq!(h.app.pages.shown(Page::Design), Shown::Docked);
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

#[test]
fn run_and_form_items_apply_or_open_a_command_with_their_params() {
    let mut h = Harness::new();
    let add = gui::MenuItem::run::<cmd_add_layer::AddLayer>(
        "Add",
        &cmd_add_layer::Params { name: "Top".into() },
    );
    assert!(h.app.available(&add.action));
    h.app.invoke(&h.ctx, &add.action);
    assert_eq!(
        h.app.doc.layers.last().map(|l| l.name.as_str()),
        Some("Top")
    );
    let mut rename: cmd_layer_properties::Params = h.app.doc.layers[0].clone().into();
    rename.name = "Renamed".into();
    let rename = gui::MenuItem::form::<cmd_layer_properties::LayerProperties>("Rename", &rename);
    h.app.invoke(&h.ctx, &rename.action);
    let form = h.app.form.as_ref().unwrap();
    assert_eq!(
        (form.id, form.params["name"].as_str()),
        ("layer_properties", Some("Renamed"))
    );
    assert!(!h.app.available("run:no_such_command:{}"));
}

#[test]
fn the_model_resolution_is_changed_from_the_model_menu() {
    use cut_craft_core::document::Resolution;
    let mut h = Harness::new();
    assert_eq!(h.app.doc.job.resolution, Resolution::Standard);
    assert_eq!(
        h.app.resolution_checked("model_resolution_standard"),
        Some(true)
    );
    assert!(h.app.available("model_resolution_highest"));
    let standard = h.app.doc.job.model_cell();
    h.app.invoke(&h.ctx, "model_resolution_highest");
    assert_eq!(h.app.doc.job.resolution, Resolution::Highest);
    assert!(
        h.app.doc.job.model_cell() < standard / 2.0,
        "finer cells for new models"
    );
    assert_eq!(
        h.app.resolution_checked("model_resolution_highest"),
        Some(true)
    );
    assert_eq!(
        h.app.resolution_checked("model_resolution_standard"),
        Some(false)
    );
    assert!(
        h.app.status.message.contains("Highest"),
        "{}",
        h.app.status.message
    );
    h.app.invoke(&h.ctx, "undo");
    assert_eq!(h.app.doc.job.resolution, Resolution::Standard, "undone");
    assert_eq!(h.app.resolution_checked("preview_quality_1"), None);
}

#[test]
fn a_still_3d_view_draws_the_model_at_its_own_detail() {
    use cut_craft_core::component::{Combine, Component};
    let mut h = Harness::new();
    let o = h.app.doc.job.area().min;
    let rect = cut_craft_core::geometry::Rect::new(
        o + Point::new(150.0, 100.0),
        o + Point::new(450.0, 250.0),
    );
    let cell = h.app.doc.job.model_cell();
    let mut c = Change::builder(&h.app.doc);
    c.add_component(Component::from_fn(
        "relief",
        Combine::Add,
        rect,
        cell,
        |p| Some((p.x * 0.5).sin() + 2.0),
    ));
    h.app.record(Ok(c.finish()), "added");
    h.app.view = View::ThreeD;
    h.frame(vec![]);
    // a moment after anything moved, the view settles and is drawn finely
    h.time += 1.0;
    h.frame(vec![]);
    let fine = h
        .app
        .fine_3d
        .as_ref()
        .map(|(_, mesh)| mesh.indices.len() / 3);
    let (key, _) = h.app.fine_3d.as_ref().unwrap();
    let quick = view3d::mesh(
        &cut_craft_core::component::model_shape(&h.app.doc, MESH_CELLS),
        key.orbit,
        key.framing,
        &key.look,
        key.fit,
    );
    let quick = quick.indices.len() / 3;
    // issue: the mermaid had no detail, drawn at 2.4 mm on a 600 mm job
    assert!(
        fine.is_some_and(|t| t > 4 * quick),
        "fine {fine:?} against {quick} quick triangles"
    );
    let kept = h.app.fine_3d.as_ref().map(|(key, _)| key.clone());
    h.time += 1.0;
    h.frame(vec![]);
    assert_eq!(
        h.app.fine_3d.as_ref().map(|(key, _)| key.clone()),
        kept,
        "kept, not rebuilt"
    );
    // an edit to the model makes it again
    let mut c = Change::builder(&h.app.doc);
    c.add_component(Component::from_fn("dome", Combine::Add, rect, cell, |_| {
        Some(4.0)
    }));
    h.app.record(Ok(c.finish()), "added");
    h.time += 1.0;
    h.frame(vec![]);
    assert_ne!(h.app.fine_3d.as_ref().map(|(key, _)| key.clone()), kept);
}

#[test]
fn dragging_a_model_in_the_2d_view_moves_it() {
    use cut_craft_core::component::{Combine, Component};
    let mut h = Harness::new();
    let o = h.app.doc.job.area().min;
    let rect = cut_craft_core::geometry::Rect::new(
        o + Point::new(100.0, 100.0),
        o + Point::new(160.0, 140.0),
    );
    let mut c = Change::builder(&h.app.doc);
    c.add_component(Component::from_fn("model", Combine::Add, rect, 1.0, |_| {
        Some(2.0)
    }));
    h.app.record(Ok(c.finish()), "added");
    h.frame(vec![]);
    // issue #4: pressed on the model (not yet picked) and dragged, it is picked and moves
    let (from, to) = (h.at(130.0, 120.0), h.at(180.0, 140.0));
    h.drag(from, to);
    let k = h.app.doc.components.values().next().unwrap();
    let moved = k.extent().min - rect.min;
    assert!(
        (moved.x - 50.0).abs() < 1.0 && (moved.y - 20.0).abs() < 1.0,
        "moved by {moved:?}"
    );
    assert_eq!(h.app.doc.selected_components.len(), 1, "and picked");
    h.app.invoke(&h.ctx, "undo");
    let k = h.app.doc.components.values().next().unwrap();
    assert!(
        (k.extent().min - rect.min).length() < 1e-6,
        "one undo puts it back"
    );
    // a vector selected is dragged by pressing inside its box
    h.rectangle(10.0, 10.0, 40.0, 30.0);
    let before = h.app.doc.selection_bounds().unwrap();
    let (from, to) = (h.at(30.0, 25.0), h.at(40.0, 25.0));
    h.drag(from, to);
    let after = h.app.doc.selection_bounds().unwrap();
    assert!(
        (after.min.x - before.min.x - 10.0).abs() < 0.5,
        "{before:?} → {after:?}"
    );
    // a drag from empty space still selects by window
    h.app.doc.selection.clear();
    h.app.doc.selected_components.clear();
    let (from, to) = (h.at(5.0, 5.0), h.at(90.0, 80.0));
    h.drag(from, to);
    assert_eq!(
        h.app.doc.selection.len(),
        1,
        "the window picked the rectangle"
    );
}

#[test]
fn files_dropped_on_the_window_open_import_or_install() {
    let dir = std::env::temp_dir().join(format!("cut-craft-drop-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let picture = dir.join("photo.png");
    let mut png = std::io::Cursor::new(vec![]);
    image::GrayImage::from_pixel(4, 4, image::Luma([128]))
        .write_to(&mut png, image::ImageFormat::Png)
        .unwrap();
    std::fs::write(&picture, png.into_inner()).unwrap();
    // on the start page there is no job to import into
    let mut h = Harness::at_start();
    h.drop_files(std::slice::from_ref(&picture), true);
    assert!(h.app.doc.bitmaps.is_empty());
    assert!(
        h.app.status.message.contains("job"),
        "{}",
        h.app.status.message
    );
    // in a job a picture is imported; dragging over says what a drop will do
    let mut h = Harness::new();
    h.drop_files(std::slice::from_ref(&picture), false);
    assert!(h.text_at("Drop to open or import").is_some());
    h.drop_files(&[picture], true);
    assert_eq!(h.app.doc.bitmaps.len(), 1, "imported");
    // a post-processor is installed
    h.app.posts = cut_craft_machines::Library::new(Some(dir.join("posts")));
    let post = dir.join("router.pp");
    std::fs::write(&post, cut_craft_post::BUILT_IN[0].1).unwrap();
    h.drop_files(&[post], true);
    assert!(
        h.app
            .posts
            .entries()
            .iter()
            .any(cut_craft_machines::Entry::is_custom)
    );
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn an_installed_post_is_kept_and_the_machine_saves_through_it() {
    let mut h = Harness::new();
    let dir = std::env::temp_dir().join(format!("cut-craft-app-posts-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    h.app.posts = cut_craft_machines::Library::new(Some(dir.join("posts")));
    let file = dir.join("mine.pp");
    std::fs::create_dir_all(&dir).unwrap();
    let text = cut_craft_post::BUILT_IN[0]
        .1
        .replace("Cut Craft FluidNC ATC (mm)", "My Router (mm)");
    std::fs::write(&file, text).unwrap();
    h.app.install_post(&file);
    assert_eq!(h.app.machines.machines.len(), 1, "a machine to use it");
    assert_eq!(h.app.output_post().unwrap().name, "My Router (mm) (*.nc)");
    std::fs::remove_file(&file).unwrap();
    assert!(
        h.app.output_post().is_ok(),
        "kept in the custom posts, not read from where it was"
    );
    // the Machine menu opens its windows; the online ones stay greyed
    for id in [
        "machine_configuration",
        "post_processor_management",
        "add_machine",
    ] {
        assert!(h.app.available(id), "{id}");
        h.app.invoke(&h.ctx, id);
        h.frame(vec![]);
    }
    assert!(h.app.machine_window.is_some() && h.app.post_window.is_some());
    assert!(h.text_at("Associated Post-Processors").is_some());
    // Cut Craft's own posts come with Cut Craft: updating them is an update check
    assert!(h.app.available("update_post_processor_database"));
    let _ = std::fs::remove_dir_all(dir);
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
fn toolpaths_save_through_the_default_fluidnc_post_until_one_is_installed() {
    use cut_craft_core::document::{Along, Job};
    let mut h = Harness::new();
    assert_eq!(
        h.app.output_post().unwrap().name,
        "Cut Craft FluidNC ATC (mm) (*.nc)"
    );
    h.app.doc.job = Job::rotary(300.0, 50.0, Along::X);
    assert_eq!(
        h.app.output_post().unwrap().name,
        "Cut Craft FluidNC ATC Wrap A G93 (mm) (*.nc)"
    );
}

/// Controls that wait on a person or open other programs: the system's file dialogs, the
/// save-changes question (the smoke test's job has unsaved work), the file manager and
/// quitting. Running one in a test puts a real window on the desktop.
fn needs_a_person(app: &CutCraftApp, id: &str) -> bool {
    const ASKS: [&str; 31] = [
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
        "print_preview",
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
    ids.extend(VIEW_BUTTONS.iter().map(|(id, ..)| (*id).to_owned()));
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
fn the_model_menu_runs_the_component_tools_it_names() {
    let h = Harness::new();
    let items = h.app.menus.items();
    for (label, id) in [
        ("Create Textured Area Component", "texture_area"),
        (
            "Create Component with Draft from Visible Model",
            "add_draft",
        ),
        ("Create an Offset Model", "offset_model"),
        ("Slice Model", "slice"),
        ("Split Selected Component", "split"),
        ("Emboss Component", "emboss"),
    ] {
        let item = items
            .iter()
            .find(|i| i.name == label)
            .unwrap_or_else(|| panic!("{label} is in the menu"));
        assert_eq!(item.action, id, "{label}");
        assert!(h.app.available(id), "{label} runs its tool");
    }
}

#[test]
fn preview_visible_leaves_hidden_toolpaths_out() {
    let mut h = Harness::new();
    h.rectangle(10.0, 10.0, 50.0, 50.0);
    for id in ["drilling_toolpath", "profile_toolpath"] {
        let params = h.app.registry.get(id).unwrap().initial_params(&h.app.doc);
        h.app.apply(id, &params).unwrap();
    }
    let profile = *h.app.doc.toolpaths.keys().last().unwrap();
    h.app.selected_toolpath = Some(profile);
    h.app.invoke(&h.ctx, "preview_toolpath");
    let only_profile = h.app.simulation.clone();
    let drilling = *h.app.doc.toolpaths.keys().next().unwrap();
    h.app.doc.toolpaths.get_mut(&drilling).unwrap().visible = false;
    h.app.invoke(&h.ctx, "preview_visible_toolpaths");
    assert_eq!(h.app.simulation, only_profile);
}

#[test]
fn deleting_the_selected_toolpath_is_undoable() {
    let mut h = Harness::new();
    h.rectangle(10.0, 10.0, 50.0, 50.0);
    h.app.invoke(&h.ctx, "drilling_toolpath");
    let params = h.app.form.as_ref().unwrap().params.clone();
    h.app.apply("drilling_toolpath", &params).unwrap();
    h.app.selected_toolpath = h.app.doc.toolpaths.keys().next().copied();
    h.app.invoke(&h.ctx, "delete_toolpath");
    assert!(h.app.doc.toolpaths.is_empty());
    h.app.invoke(&h.ctx, "undo");
    assert_eq!(h.app.doc.toolpaths.len(), 1);
}

#[test]
fn unsaved_work_is_kept_for_recovery_and_opens_after_a_crash() {
    let dir = std::env::temp_dir().join(format!("cut-craft-recovery-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let recovery = dir.join("recovery.ccjob");
    let autosaved = |h: &mut Harness| {
        h.app.recovery_path = Some(recovery.clone());
        h.rectangle(10.0, 10.0, 20.0, 20.0);
        h.frame(vec![]);
        assert!(!recovery.exists(), "not at once");
        h.time += AUTOSAVE_SECONDS;
        h.frame(vec![]);
        let writing = h.app.autosave.take().expect("a copy is being written");
        writing.join().unwrap().unwrap();
        assert!(recovery.exists());
    };
    // a longer interval in the options waits longer
    let mut h = Harness::new();
    h.app.options.general.autosave_seconds = 120;
    h.app.recovery_path = Some(dir.join("later.ccjob"));
    h.rectangle(10.0, 10.0, 20.0, 20.0);
    h.frame(vec![]);
    h.time += AUTOSAVE_SECONDS;
    h.frame(vec![]);
    assert!(h.app.autosave.is_none(), "not after the default interval");
    h.time += 120.0;
    h.frame(vec![]);
    h.app
        .autosave
        .take()
        .expect("written once the options' interval is up")
        .join()
        .unwrap()
        .unwrap();
    let _ = std::fs::remove_file(dir.join("later.ccjob"));
    // closing the job on purpose leaves nothing to recover
    let mut h = Harness::new();
    autosaved(&mut h);
    h.app.session.discard_changes(&h.app.history);
    h.app.invoke(&h.ctx, "close");
    assert!(!recovery.exists());
    // a run that ends badly leaves the copy, and the next run opens it
    let mut h = Harness::new();
    autosaved(&mut h);
    drop(h);
    let mut next = Harness::at_start();
    next.app.recovery_path = Some(recovery.clone());
    assert!(next.app.open_recovery());
    assert_eq!(next.app.screen, Screen::Job);
    assert_eq!(next.app.doc.vectors.len(), 1);
    assert!(
        next.app.session.is_dirty(&next.app.history),
        "unsaved until saved"
    );
    assert!(next.app.status.message.contains("Recovered"));
    next.app.save_to(&dir.join("kept.ccjob"));
    assert!(!recovery.exists(), "saved: nothing to recover");
    assert!(!next.app.open_recovery());
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn ctrl_x_c_and_v_arrive_as_clipboard_events_and_work_across_windows() {
    let mut h = Harness::new();
    h.rectangle(10.0, 10.0, 20.0, 20.0);
    h.frame(vec![Event::Copy]);
    assert_eq!(h.app.clipboard.len(), 1);
    assert!(
        h.copied.starts_with('['),
        "the system clipboard gets the vectors: {}",
        h.copied
    );
    // what another window copied pastes here
    let circle = vec![Path::circle(Point::new(50.0, 50.0), 5.0)];
    h.frame(vec![Event::Paste(serde_json::to_string(&circle).unwrap())]);
    assert_eq!(h.app.doc.vectors.len(), 2);
    assert_eq!(h.app.clipboard, circle);
    // text from elsewhere pastes nothing
    h.frame(vec![Event::Paste("hello".into())]);
    assert_eq!(h.app.doc.vectors.len(), 2);
    // the pasted circle is selected: Ctrl+X takes it
    h.frame(vec![Event::Cut]);
    assert_eq!(h.app.doc.vectors.len(), 1);
    assert_eq!(h.app.clipboard, circle);
}

#[test]
fn crossing_vectors_go_to_the_vector_validator_or_on_as_options_say() {
    use crate::options::IntersectionCheck;
    use cut_craft_core::geometry::{Path, Point};
    let mut h = Harness::new();
    let bow_tie = Path::polyline(
        &[
            Point::new(10.0, 10.0),
            Point::new(50.0, 50.0),
            Point::new(50.0, 10.0),
            Point::new(10.0, 50.0),
        ],
        true,
    );
    let mut c = Change::builder(&h.app.doc);
    let id = c.add(bow_tie);
    c.select(vec![id]);
    let change = c.finish();
    h.app.history.apply(&mut h.app.doc, change).unwrap();
    let params = h
        .app
        .registry
        .get("pocket_toolpath")
        .unwrap()
        .initial_params(&h.app.doc);
    h.app.options.confirmations.toolpath_intersections = IntersectionCheck::OpenVectorValidator;
    let why = h.app.apply("pocket_toolpath", &params).unwrap_err();
    assert!(why.contains("cross"), "{why}");
    assert!(h.app.doc.toolpaths.is_empty());
    assert_eq!(h.app.form.as_ref().map(|f| f.id), Some("vector_validator"));
    assert!(h.app.doc.selection.contains(&id));
    h.app.options.confirmations.toolpath_intersections = IntersectionCheck::DontCheck;
    h.app.apply("pocket_toolpath", &params).unwrap();
    assert_eq!(h.app.doc.toolpaths.len(), 1);
}

#[test]
fn tabs_are_placed_and_removed_by_clicking_the_vector() {
    let mut h = Harness::new();
    h.rectangle(10.0, 10.0, 50.0, 50.0);
    h.click(h.at(60.0, 35.0), Modifiers::NONE);
    h.app.invoke(&h.ctx, "profile_toolpath");
    {
        let params = &mut h.app.form.as_mut().unwrap().params;
        params["tabs"] = "flat".into();
        params["tab"]["spacing"] = "placed".into();
    }
    h.frame(vec![]);
    h.click(h.at(60.0, 30.0), Modifiers::NONE);
    let tabs = |h: &Harness| -> Vec<Point> {
        serde_json::from_value(h.app.form.as_ref().unwrap().params["tab_points"].clone())
            .unwrap_or_default()
    };
    let placed = tabs(&h);
    assert_eq!(placed.len(), 1, "{}", h.app.status.message);
    assert!(near(placed[0], Point::new(60.0, 30.0)), "{placed:?}");
    assert_eq!(
        h.app.doc.selection.len(),
        1,
        "the click placed a tab, not a selection"
    );
    // a drag from the tab moves it along the vector
    h.app.options.snapping.smart_snapping = false;
    h.drag(h.at(60.0, 30.0), h.at(60.5, 45.0));
    let moved = tabs(&h);
    assert_eq!(moved.len(), 1, "{}", h.app.status.message);
    assert!(near(moved[0], Point::new(60.0, 45.0)), "{moved:?}");
    let params = h.app.form.as_ref().unwrap().params.clone();
    h.app.apply("profile_toolpath", &params).unwrap();
    assert_eq!(h.app.doc.toolpaths.len(), 1);
    h.click(h.at(60.0, 45.0), Modifiers::NONE);
    assert!(tabs(&h).is_empty(), "a second click takes it off");
}

/// A new job with a shape on it, in the 3D view.
fn shape_in_3d() -> Harness {
    let mut h = Harness::new();
    h.rectangle(10.0, 10.0, 50.0, 50.0);
    let shape = h.app.registry.get("create_shape").unwrap().default_params();
    h.app.apply("create_shape", &shape).unwrap();
    h.app.view = View::ThreeD;
    h.frame(vec![]);
    h.frame(vec![]);
    h
}

#[test]
fn the_view_cube_looks_from_the_face_edge_or_corner_clicked() {
    use cut_craft_core::glam::DVec3;
    let mut h = shape_in_3d();
    let looks_from = |h: &Harness, from: DVec3| {
        let towards = h.app.orbit.towards_viewer();
        assert!(
            towards.abs_diff_eq(from.normalize(), 1e-9),
            "{towards} not {from}"
        );
    };
    let cube = h.app.view_cube.expect("the 3D view draws its view cube");
    let framing = h.app.framing;
    // the right face's middle, with its name on it
    let right = cube.screen(h.app.orbit, DVec3::X);
    assert!(
        h.texts_at("Right")
            .iter()
            .any(|at| at.distance(right) < 1.0),
        "named"
    );
    h.click(right, Modifiers::NONE);
    looks_from(&h, DVec3::X);
    assert_eq!(h.app.framing, framing, "turned, not refitted");
    // face on now: its top corner at the back, then its top edge
    let corner = cube.screen(h.app.orbit, DVec3::new(1.0, 0.8, 0.8));
    h.click(corner, Modifiers::NONE);
    looks_from(&h, DVec3::new(1.0, 1.0, 1.0));
    let edge = cube.screen(h.app.orbit, DVec3::new(0.8, 0.0, 1.0));
    h.click(edge, Modifiers::NONE);
    looks_from(&h, DVec3::new(1.0, 0.0, 1.0));
    // a double-click looks from the other side, and the view behind takes no clicks
    let selected = h.app.doc.selected_components.clone();
    h.double_click(cube.screen(h.app.orbit, DVec3::new(1.0, 0.0, 1.0)));
    looks_from(&h, DVec3::new(-1.0, 0.0, -1.0));
    assert_eq!(h.app.doc.selected_components, selected);
    assert_eq!(h.app.framing, framing);
}

#[test]
fn the_3d_view_turns_to_perspective_and_tilts_underneath() {
    let mut h = shape_in_3d();
    let start = h.app.orbit;
    let button = h.text_at("Perspective").expect("the perspective toggle");
    h.click(button, Modifiers::NONE);
    assert!(h.app.framing.perspective);
    h.click(h.text_at("Fit").unwrap(), Modifiers::NONE);
    assert!(h.app.framing.perspective, "Fit keeps the projection");
    // a long drag down, below the cube, tilts the view past side on to under the job
    let cube = h.app.view_cube.unwrap().rect();
    let from = cube.left_bottom() + vec2(-cube.width(), cube.height());
    h.drag(from, from + vec2(0.0, 300.0));
    let pitch = h.app.orbit.pitch;
    assert!(
        pitch > std::f64::consts::FRAC_PI_2 && pitch > start.pitch,
        "{pitch}"
    );
    assert!(h.app.orbit.towards_viewer().z < 0.0, "from underneath");
    h.click(button, Modifiers::NONE);
    assert!(!h.app.framing.perspective, "back to orthographic");
}

#[test]
fn a_vector_behind_the_model_is_not_drawn_over_it() {
    // a tall block on the right half; an outline on the floor left of it
    use cut_craft_core::component::{Combine, Component};
    let mut h = Harness::new();
    let job = h.app.doc.job;
    let o = job.area().min;
    let wall = cut_craft_core::geometry::Rect::new(
        o + Point::new(job.width / 2.0, 0.0),
        o + Point::new(job.width, job.height),
    );
    let mut c = Change::builder(&h.app.doc);
    c.add_component(Component::from_fn(
        "wall",
        Combine::Add,
        wall,
        job.model_cell(),
        |_| Some(job.width / 2.0),
    ));
    h.app.record(Ok(c.finish()), "added");
    h.rectangle(
        job.width / 8.0,
        job.height / 4.0,
        job.width / 8.0,
        job.height / 2.0,
    );
    h.app.view = View::ThreeD;
    // lines drawn in the vectors' colour
    let lines = |h: &mut Harness, orbit, vectors| {
        h.app.orbit = orbit;
        h.app.display.vectors_3d = vectors;
        h.frame(vec![]);
        let stroke = h
            .ctx
            .global_style()
            .visuals
            .widgets
            .noninteractive
            .fg_stroke;
        let colour = egui::epaint::ColorMode::Solid(stroke.color);
        let drawn = h
            .shapes
            .iter()
            .filter(|c| matches!(&c.shape, egui::Shape::Path(p) if p.stroke.color == colour));
        drawn.count()
    };
    let top = view3d::Orbit::TOP;
    assert!(
        lines(&mut h, top, true) > lines(&mut h, top, false),
        "from above it shows"
    );
    let low_right = view3d::Orbit::facing(cut_craft_core::glam::DVec3::new(1.0, 0.0, 0.1));
    assert_eq!(
        lines(&mut h, low_right, true),
        lines(&mut h, low_right, false),
        "from low on the right the block hides it"
    );
}

#[test]
fn importing_a_model_while_setting_up_a_new_job_keeps_the_job_s_pages() {
    // issue #17: New, Import Component / 3D Model, then Close left the new-job screen bare
    let mut h = Harness::at_start();
    h.app.invoke(&h.ctx, "new");
    assert_eq!(h.app.screen, Screen::NewJob);
    let file = pyramid_stl("new-job-import");
    h.app.import_file(&file);
    assert_eq!(h.app.screen, Screen::Job, "the job is set up as it stood");
    assert_eq!(
        h.app.form.as_ref().map(|f| f.id),
        Some("orientate_3d_model")
    );
    h.app.form = None;
    h.frame(vec![]);
    assert!(
        h.text_at("Design").is_some(),
        "the Design page's tab is back once the form closes"
    );
    let _ = std::fs::remove_file(file);
}

#[test]
fn another_design_tab_closes_the_design_form() {
    let mut h = Harness::new();
    h.app.pages.set_pinned(Page::Design, true);
    h.app.start_command("document_variables");
    assert!(h.app.form.is_some());
    h.app.set_design_tab(h.app.design_tab);
    assert!(h.app.form.is_some(), "the same tab keeps it");
    let layers = DESIGN_TABS.iter().position(|&t| t == "Layers").unwrap();
    h.app.set_design_tab(layers);
    assert!(h.app.form.is_none(), "the Layers tab shows, not the form");
}

#[test]
fn toolpaths_materials_opens_the_material_library() {
    let mut h = Harness::new();
    let item = h
        .app
        .menus
        .items()
        .into_iter()
        .find(|i| i.name == "Materials...")
        .map(|i| i.action.clone())
        .expect("Toolpaths ▸ Materials...");
    h.app.invoke(&h.ctx, &item);
    assert_eq!(h.app.form.as_ref().map(|f| f.id), Some("materials"));
    assert_eq!(
        h.app.form.as_ref().map(|f| f.page),
        Some(Page::Toolpaths),
        "beside Material Setup"
    );
    h.frame(vec![]);
    assert!(
        h.text_at("Rest Machining").is_none(),
        "it cuts nothing, so nothing to rest machine"
    );
    let walnut = serde_json::json!({ "material": "walnut" });
    h.app.apply("materials", &walnut).unwrap();
    assert_ne!(
        h.app.doc.job.colour,
        cut_craft_core::document::Job::default().colour
    );
}

#[test]
fn materials_shows_its_form_in_a_job_and_waits_for_one_on_the_start_page() {
    let mut h = Harness::new();
    h.frame(vec![]);
    h.app.invoke(&h.ctx, "materials");
    h.frame(vec![]);
    h.frame(vec![]);
    assert!(
        h.text_at("Pine").is_some(),
        "the Toolpaths page flies out with it"
    );
    let start = Harness::at_start();
    assert!(!start.app.available("materials"), "no job to colour");
    assert!(
        !start.app.available("wrapped_fluting_layout"),
        "nor for a gadget"
    );
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
    // the fit is offered in a flat job too; the model type only when the options offer
    // both kinds, and what the options are is never shown
    assert!(h.text_at("Scale model to fit material").is_some());
    for label in ["Imported model type", "Offered"] {
        assert_eq!(h.text_at(label), None, "{label} is not shown");
    }
    // the placing is on the same page, Center model left to the alignment buttons
    assert!(h.text_at("Depth Below Top").is_some());
    assert!(h.text_at("Center model").is_none());
    let _ = std::fs::remove_file(file);
}

#[test]
#[expect(clippy::cast_precision_loss)]
fn a_model_imports_again_from_its_own_file_while_it_is_there_a_changed_one_with_a_warning() {
    let mut h = Harness::new();
    let file = pyramid_stl("again");
    h.app.import_file(&file);
    let form = h.app.form.as_mut().unwrap();
    form.params["orientation"]["rotation"] = Value::from(90.0);
    form.params["size"]["x"] = Value::from(50.0);
    form.dirty = true;
    h.frame(vec![]);
    h.app.form = None;
    let (&id, k) = h.app.doc.components.iter().next().unwrap();
    let width = |k: &cut_craft_core::component::Component| k.cell * k.columns as f64;
    assert!((width(k) - 50.0).abs() <= k.cell, "{}", width(k));
    let source = k.source.clone().expect("the import remembers its file");
    assert_eq!(source.path, file);
    let again = format!("{}{}", gui::REIMPORT_MODEL, id.0);
    let menu = gui::component_menu(&h.app.doc, id, k);
    assert_eq!(menu[0].action, again, "first on the model's menu");
    assert!(h.app.available(&again));
    // the form again with the settings it had, its Apply replacing the model
    h.app.invoke(&h.ctx, &again);
    let form = h.app.form.as_ref().expect("Orientate 3D Model again");
    assert_eq!(form.params["orientation"]["rotation"], 90.0);
    assert_eq!(form.params["size"]["x"], 50.0, "at the size it was made");
    assert_eq!(form.drawn, Some(id));
    // most often to scale it again
    let form = h.app.form.as_mut().unwrap();
    form.params["orientation"]["rotation"] = Value::from(0.0);
    form.params["size"]["x"] = Value::from(25.0);
    form.dirty = true;
    h.frame(vec![]);
    assert_eq!(h.app.doc.components.len(), 1, "replaced, not added");
    let k = h.app.doc.components.values().next().unwrap();
    assert!((width(k) - 25.0).abs() <= k.cell, "{}", width(k));
    h.app.form = None;
    // changed, it is imported again as it is now, with a warning
    let mut changed = std::fs::read(&file).unwrap();
    changed.extend(b"\n");
    std::fs::write(&file, changed).unwrap();
    h.app.invoke(&h.ctx, &again);
    assert!(h.app.form.is_some(), "from the changed file");
    assert!(
        h.app.status.message.contains("has changed"),
        "{}",
        h.app.status.message
    );
    h.app.form = None;
    // gone, it is not offered
    std::fs::remove_file(&file).unwrap();
    assert!(!h.app.available(&again));
}

#[test]
fn editing_placed_text_replaces_it() {
    let mut h = Harness::new();
    h.app.invoke(&h.ctx, "draw_text");
    h.app.form.as_mut().unwrap().params["font"] = "Test Boxes".into();
    let at = Point::new(20.0, 20.0);
    let click = Drag {
        start: at,
        end: at,
        from_center: false,
        constrain: false,
    };
    h.app.draw("draw_text", click);
    let form = h.app.form.as_mut().unwrap();
    form.params["text"] = "HI".into();
    let params = form.params.clone();
    h.app.apply_form("draw_text", &params).unwrap();
    assert_eq!(h.app.doc.texts.len(), 1, "the placeholder was replaced");
    assert_eq!(h.app.doc.texts.values().next().unwrap()["text"], "HI");
}

#[test]
fn help_contents_links_to_the_pages_and_their_tools() {
    let mut h = Harness::new();
    h.app.pages.set_pinned(Page::Design, true);
    h.app.invoke(&h.ctx, "help_contents");
    assert_eq!(h.app.info, Some(Info::Help("contents")));
    // a new window measures itself on its first frame
    h.frame(vec![]);
    h.frame(vec![]);
    // the window's link, drawn over the Design page's own title and side tab
    let at = *h
        .texts_at("Design")
        .last()
        .expect("Contents links the Design page");
    h.click(at, Modifiers::NONE);
    h.frame(vec![]);
    assert_eq!(h.app.info, Some(Info::Help("design")));
    let at = *h
        .texts_at("Job Setup")
        .last()
        .expect("its tools are listed");
    h.click(at, Modifiers::NONE);
    h.frame(vec![]);
    assert_eq!(h.app.info, Some(Info::Help("job_setup")));
    let vars = h.app.help_markdown("document_variables");
    assert!(
        vars.contains("{depth}"),
        "a tool's help is its doc comments"
    );
    assert!(vars.contains("**Variables**"), "and its settings'");
}

#[test]
fn a_tools_help_page_falls_back_to_its_own_description() {
    let h = Harness::new();
    // Profile's params carry no doc comment of their own; its crate says what it does
    let profile = h.app.help_markdown("profile_toolpath");
    assert!(!profile.contains("No help written"), "{profile}");
    assert!(
        profile.contains("Cuts along the selected vectors, outside, inside or on the line"),
        "{profile}"
    );
    assert!(
        !profile.contains("Profile Toolpath: cuts"),
        "the heading names it already: {profile}"
    );
}

#[test]
fn a_pages_question_mark_opens_its_help() {
    let mut h = Harness::new();
    h.app.pages.set_pinned(Page::Design, true);
    h.frame(vec![]);
    let at = h.text_at("?").expect("the page's ? button");
    h.click(at, Modifiers::NONE);
    assert_eq!(h.app.info, Some(Info::Help("design")));
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
    assert!(h.text_at("Processor cores to use (0 = all)").is_some());
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
    assert_eq!(h.app.options.appearance.theme, crate::options::Theme::Light);
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
fn an_offline_package_installs_its_machines_once() {
    let mut h = Harness::new();
    let package =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../machines/genmitsu.json");
    h.app.install_package(&package);
    let installed = h.app.machines.machines.len();
    assert!(installed > 0, "the package's machines are added");
    assert!(
        h.app
            .machines
            .machines
            .iter()
            .all(|m| m.name.starts_with("Genmitsu"))
    );
    h.app.install_package(&package);
    assert_eq!(
        h.app.machines.machines.len(),
        installed,
        "installing again adds none"
    );
    h.app.install_package(&package.with_file_name("README.md"));
    assert_eq!(
        h.app.machines.machines.len(),
        installed,
        "not a package: nothing added"
    );
    for id in ["install_offline_package", "search_for_machine_online"] {
        assert!(h.app.available(id), "{id}");
    }
}

#[test]
fn rotary_jobs_need_a_4th_axis_and_save_through_the_post_wrapping_their_side() {
    use cut_craft_core::document::{Along, Job};
    let mut h = Harness::new();
    let package =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../machines/genmitsu.json");
    h.app.install_package(&package);
    let wraps = |app: &CutCraftApp| app.output_post().map(|p| p.wraps().map(str::to_owned));
    assert_eq!(wraps(&h.app), Ok(None), "a flat job: the flat default");
    h.app.doc.job = Job::rotary(300.0, 50.0, Along::X);
    let why = wraps(&h.app).unwrap_err();
    assert!(why.contains("4th axis"), "{why}");
    let names: Vec<String> = h.app.posts.entries().into_iter().map(|e| e.name).collect();
    let machine = h.app.machines.active_mut().unwrap();
    machine.rotary = true;
    machine.add_rotary_posts(&names);
    assert_eq!(
        wraps(&h.app),
        Ok(Some("Y".into())),
        "Y goes round a cylinder along X"
    );
    h.app.doc.job = Job::rotary(300.0, 50.0, Along::Y);
    assert_eq!(wraps(&h.app), Ok(Some("X".into())));
    h.app.doc.job = Job::default();
    assert_eq!(wraps(&h.app), Ok(None), "flat jobs keep the flat default");
}

#[test]
fn set_as_default_in_job_setup_is_what_new_jobs_start_with() {
    let mut h = Harness::at_start();
    h.app.invoke(&h.ctx, "new");
    h.frame(vec![]);
    let form = h.app.form.as_mut().expect("the Job Setup form");
    form.params["size"]["width"] = 300.0.into();
    form.params["size"]["thickness"] = 12.0.into();
    let button = h
        .text_at("Set as Default")
        .expect("a Set as Default button");
    h.click(button, Modifiers::NONE);
    h.frame(vec![]);
    let saved = h.app.options.default_job.expect("kept in the options");
    assert!((saved.width - 300.0).abs() < 1e-9);
    h.app.job_setup_cancel();
    h.app.invoke(&h.ctx, "new");
    h.frame(vec![]);
    let form = h.app.form.as_ref().expect("Job Setup again");
    assert_eq!(form.params["size"]["width"], 300.0);
    assert_eq!(form.params["size"]["thickness"], 12.0);
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

const CRASH: &str = "Cut Craft 0.3.0 on linux crashed: panicked at crates/app/src/view3d.rs:9:9:\n\
                     index out of bounds\n\nBacktrace:\n...";

#[test]
fn a_crash_log_goes_to_the_service_by_itself_when_allowed() {
    #[expect(
        clippy::unnecessary_wraps,
        reason = "a post that can fail, as the real one"
    )]
    fn accepted(url: &str, body: &str) -> Result<(), String> {
        assert_eq!(url, "https://crash.example/v1/crash");
        assert!(
            body.contains("\"title\":\"Crash: index out of bounds\""),
            "{body}"
        );
        Ok(())
    }
    fn refused(_: &str, _: &str) -> Result<(), String> {
        Err("the service is down".into())
    }
    for (post, sent) in [
        (accepted as fn(&str, &str) -> Result<(), String>, true),
        (refused, false),
    ] {
        let mut h = Harness::new();
        h.app.options.bug_reports.crash_logs = crate::options::CrashLogs::Send;
        h.app.reporter.crash_service = Some("https://crash.example/v1/crash".into());
        h.app.reporter.post = post;
        let ctx = h.ctx.clone();
        h.app.offer_crash(&ctx, CRASH.into());
        assert!(
            h.app.options.bug_reports.install_id.is_some(),
            "numbered once"
        );
        for _ in 0..200 {
            h.frame(vec![]);
            if h.app.status.message.contains("crash report was sent") || h.app.reporter.is_open() {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        h.frame(vec![]);
        assert_eq!(
            h.app.status.message.contains("crash report was sent"),
            sent,
            "{}",
            h.app.status.message
        );
        assert_eq!(
            h.app.reporter.is_open(),
            !sent,
            "not sent: the form instead"
        );
    }
}

#[test]
fn a_crash_is_not_offered_when_crash_logs_are_not_to_be_sent() {
    let mut h = Harness::new();
    h.app.options.bug_reports.crash_logs = crate::options::CrashLogs::DontSend;
    let ctx = h.ctx.clone();
    h.app.offer_crash(&ctx, CRASH.into());
    h.frame(vec![]);
    h.frame(vec![]);
    assert!(!h.app.reporter.is_open(), "no report");
    assert!(
        h.app
            .status
            .message
            .starts_with("Cut Craft closed with an error last time."),
        "{}",
        h.app.status.message
    );
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
fn ctrl_shift_v_pastes_in_place() {
    let mut h = Harness::new();
    h.rectangle(10.0, 10.0, 20.0, 20.0);
    h.frame(vec![Event::Copy]);
    let copied = h.app.clipboard.clone();
    let press = |pressed| Event::Key {
        key: Key::V,
        physical_key: None,
        pressed,
        repeat: false,
        modifiers: Modifiers::COMMAND | Modifiers::SHIFT,
    };
    h.frame(vec![press(true), press(false)]);
    assert_eq!(h.app.doc.vectors.len(), 2, "a second copy");
    let pasted = h.app.doc.vectors.values().last().unwrap();
    assert_eq!(pasted, &copied[0], "where the first one is");
}

#[test]
fn a_moving_3d_view_keeps_full_detail_unless_simplifying_is_on() {
    let moving = |simplify: bool| moved_ago(simplify, 0.0, None);
    assert!(moving(false), "full detail while it turns, by default");
    assert!(!moving(true), "coarser while it turns, when asked");
    // full detail once the view has been still as long as the options say
    assert!(moved_ago(true, 0.3, None), "after the usual quarter second");
    assert!(
        !moved_ago(true, 0.3, Some(1.0)),
        "not yet, when told to wait longer"
    );
    assert!(moved_ago(true, 1.1, Some(1.0)));
}

/// Whether the 3D view draws the model at full detail `ago` seconds after it last turned,
/// simplifying while moving or not, and waiting `settle` seconds when given.
fn moved_ago(simplify: bool, ago: f64, settle: Option<f64>) -> bool {
    use cut_craft_core::component::{Combine, Component};
    {
        let mut h = Harness::new();
        h.app.options.view_3d.simplify_while_moving = simplify;
        if let Some(settle) = settle {
            h.app.options.view_3d.settle_seconds = settle;
        }
        let o = h.app.doc.job.area().min;
        let rect = cut_craft_core::geometry::Rect::new(
            o + Point::new(150.0, 100.0),
            o + Point::new(450.0, 250.0),
        );
        let cell = h.app.doc.job.model_cell();
        let mut c = Change::builder(&h.app.doc);
        c.add_component(Component::from_fn(
            "relief",
            Combine::Add,
            rect,
            cell,
            |p| Some((p.x * 0.5).sin() + 2.0),
        ));
        h.app.record(Ok(c.finish()), "added");
        h.app.view = View::ThreeD;
        h.app.moved_3d = h.time - ago;
        h.frame(vec![]);
        h.app.fine_3d.is_some()
    }
}

#[test]
fn a_space_mouse_event_after_a_pause_is_not_the_pause_long() {
    use crate::spacemouse::{Input, LONGEST_STEP, moved};
    let mut h = Harness::new();
    h.rectangle(10.0, 10.0, 20.0, 10.0);
    h.app.view = View::ThreeD;
    h.frame(vec![]);
    let (tx, rx) = std::sync::mpsc::channel();
    h.app.space_mouse = Some(rx);
    let before = (h.app.orbit, h.app.framing);
    // a light twist and push, the first event after half a minute still
    let motion = ([0, 0, 40], [0, 35, 0]);
    tx.send(Input::Motion {
        translate: motion.0,
        rotate: motion.1,
        period: 30_000,
    })
    .unwrap();
    h.frame(vec![]);
    let settings = &h.app.options.space_mouse;
    let rotation = h.app.options.view_3d.rotation.view();
    let (orbit, framing) = moved(before, motion, LONGEST_STEP, settings, rotation);
    assert!(
        (h.app.orbit.yaw - orbit.yaw).abs() < 1e-9 && h.app.framing == framing,
        "{:?} against {orbit:?}",
        h.app.orbit
    );
    assert!(h.app.orbit != before.0, "it still turned");
}

#[test]
fn importing_a_model_shows_the_3d_view() {
    let mut h = Harness::new();
    h.app.view = View::TwoD;
    let file = pyramid_stl("to-3d");
    h.app.import_file(&file);
    assert!(h.app.view == View::ThreeD, "placed in 3D, shown there");
    // and again from its file
    let form = h.app.form.as_mut().unwrap();
    form.dirty = true;
    h.frame(vec![]);
    h.app.form = None;
    h.app.view = View::TwoD;
    let id = *h.app.doc.components.keys().next().unwrap();
    h.app
        .invoke(&h.ctx, &format!("{}{}", gui::REIMPORT_MODEL, id.0));
    assert!(h.app.view == View::ThreeD, "a re-import too");
}

#[test]
fn cancel_takes_back_what_a_tool_did_and_closes_it() {
    let mut h = Harness::new();
    // a tool that has done nothing yet has nothing to cancel
    h.app.start_command("draw_circle");
    h.frame(vec![]);
    h.frame(vec![]);
    assert!(h.text_at("Cancel").is_some());
    let undoable = h.app.history.undo_depth();
    h.app.form = None;
    // Import 3D Model puts the model in the job at once
    let file = pyramid_stl("cancel");
    h.app.import_file(&file);
    h.frame(vec![]);
    h.frame(vec![]);
    assert_eq!(
        h.app.doc.components.len(),
        1,
        "placed while the form is open"
    );
    let over = h.text_at("Initial Orientation").unwrap();
    let cancel = h.scroll_to(over, "Cancel").expect("a Cancel button");
    h.click(cancel, Modifiers::NONE);
    h.frame(vec![]);
    assert!(h.app.form.is_none(), "closed");
    assert!(h.app.doc.components.is_empty(), "the model taken back");
    assert_eq!(h.app.history.undo_depth(), undoable);
    assert!(h.app.history.can_redo(), "and it can be redone");
    let _ = std::fs::remove_file(file);
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

/// How wide each docked page was drawn over `frames` frames, Design then Toolpaths.
fn page_widths(h: &mut Harness, frames: usize) -> Vec<[Option<f32>; 2]> {
    (0..frames)
        .map(|_| {
            h.frame(vec![]);
            h.app.page_rects.map(|r| r.map(|r| r.width()))
        })
        .collect()
}

fn both_pinned() -> Harness {
    let mut h = Harness::new();
    h.app.pages.set_pinned(Page::Toolpaths, true);
    h.frame(vec![]);
    h
}

#[test]
fn a_page_whose_contents_ask_for_more_room_keeps_its_place() {
    // a field asking for a little more than the page has, as a long form in a big font does
    let ctx = egui::Context::default();
    let window = Rect::from_min_size(Pos2::ZERO, vec2(1280.0, 800.0));
    for frame in 0..20 {
        let input = RawInput {
            screen_rect: Some(window),
            time: Some(f64::from(frame) * 0.1),
            ..Default::default()
        };
        let mut rects = vec![];
        headless(ctx.run_ui(input, |ui| {
            for panel in [egui::Panel::left("l"), egui::Panel::right("r")] {
                rects.push(show_page(panel, 300.0, ui, |ui| {
                    ui.set_min_width(ui.available_width() + 40.0);
                }));
            }
            rects.push(ui.available_rect_before_wrap());
        }));
        assert_eq!(
            rects[0].x_range(),
            egui::Rangef::new(0.0, 300.0),
            "left page"
        );
        assert_eq!(
            rects[1].x_range(),
            egui::Rangef::new(980.0, 1280.0),
            "right page"
        );
        assert_eq!(
            rects[2].x_range(),
            egui::Rangef::new(300.0, 980.0),
            "the view between"
        );
    }
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
fn job_setup_for_a_new_job_is_as_wide_as_the_design_page_after() {
    let mut h = Harness::at_start();
    h.app.invoke(&h.ctx, "new");
    let setup = page_widths(&mut h, 3);
    h.app.job_setup_ok();
    let job = page_widths(&mut h, 2);
    assert_eq!(setup[2][0], job[1][0], "no jump when the job is made");
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
fn the_toolpath_list_shows_hides_edits_and_reorders_its_rows() {
    let mut h = Harness::new();
    h.app.doc = cut_craft_core::testing::three_toolpaths();
    let mut c = Change::builder(&h.app.doc);
    for (id, name) in [(1, "Pocket"), (2, "Drill"), (3, "Cut Out")] {
        let mut t = h.app.doc.toolpaths[&EntityId(id)].clone();
        t.name = name.into();
        c.replace_toolpath(EntityId(id), t);
    }
    h.app.doc.apply(&c.finish()).unwrap();
    h.app.screen = Screen::Job;
    h.app.pages.set_pinned(Page::Toolpaths, true);
    h.frame(vec![]);
    h.frame(vec![]);
    let names = |h: &Harness| -> Vec<String> {
        h.app
            .doc
            .toolpaths
            .values()
            .map(|t| t.name.clone())
            .collect()
    };
    // the tick left of a name hides it
    let drill = h.text_at("Drill").unwrap();
    h.click(drill - egui::vec2(24.0, 0.0), Modifiers::NONE);
    assert!(!h.app.doc.toolpaths[&EntityId(2)].visible);
    // a click selects, the down arrow moves it below the next one
    h.click(drill, Modifiers::NONE);
    assert_eq!(h.app.selected_toolpath, Some(EntityId(2)));
    let down = h.text_at("Move Down").unwrap();
    h.click(down, Modifiers::NONE);
    assert_eq!(names(&h), ["Pocket", "Cut Out", "Drill"]);
    assert_eq!(
        h.app.selected_toolpath,
        Some(EntityId(3)),
        "it stays selected"
    );
    assert!(
        !h.app.doc.toolpaths[&EntityId(3)].visible,
        "hidden moves with it"
    );
    // a double-click opens its form
    h.frame(vec![]);
    let pocket = h.text_at("Pocket").unwrap();
    h.double_click(pocket);
    h.frame(vec![]);
    assert_eq!(h.app.selected_toolpath, Some(EntityId(1)));
    assert!(h.app.form.is_some(), "the toolpath's form is open");
}

#[test]
fn a_double_click_on_the_preview_takes_away_the_piece_cut_free_under_it() {
    use cut_craft_core::glam::DVec3;
    use cut_craft_core::toolpath::{Move, MoveKind, Tool, Toolpath};
    let mut h = Harness::new();
    h.app.doc.job.width = 100.0;
    h.app.doc.job.height = 100.0;
    let job = h.app.doc.job;
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
    });
    h.app.simulation = Some(m);
    h.app.orbit = view3d::Orbit::TOP;
    let fit = egui::Rect::from_min_size(egui::pos2(0.0, 0.0), egui::vec2(400.0, 400.0));
    // the middle of the view is the part: it goes, the waste round it stays
    assert!(h.app.remove_preview_piece(fit, fit.center()));
    let sim = h.app.simulation.as_ref().unwrap();
    let at = |x, y| sim.at(cut_craft_core::geometry::Point::new(x, y)).unwrap();
    assert!(at(50.0, 50.0).is_nan());
    assert_eq!(at(5.0, 5.0), 0.0);
    assert!(
        !h.app.remove_preview_piece(fit, fit.center()),
        "already gone"
    );
}

#[test]
fn preview_visible_leaves_hidden_toolpaths_out_and_undo_last_steps_back_one_preview() {
    let mut h = Harness::new();
    h.app.display.animate_preview = false;
    h.rectangle(10.0, 10.0, 50.0, 50.0);
    for id in ["drilling_toolpath", "profile_toolpath"] {
        h.app.invoke(&h.ctx, id);
        let params = h.app.form.as_ref().unwrap().params.clone();
        h.app.apply(id, &params).unwrap();
    }
    let ids: Vec<EntityId> = h.app.doc.toolpaths.keys().copied().collect();
    h.app.invoke(&h.ctx, "preview_all_toolpaths");
    let all = h.app.simulation.clone().unwrap();
    // hide the profile: Preview Visible cuts the drilling only, Preview All both
    h.app.doc.toolpaths.get_mut(&ids[1]).unwrap().visible = false;
    h.app.invoke(&h.ctx, "preview_visible_toolpaths");
    let visible = h.app.simulation.clone().unwrap();
    assert_ne!(visible, all);
    h.app.invoke(&h.ctx, "preview_all_toolpaths");
    assert_eq!(h.app.simulation.as_ref(), Some(&all));
    // undo goes back to before the last preview, once
    h.app.invoke(&h.ctx, "undo_last_preview");
    assert_eq!(h.app.simulation.as_ref(), Some(&visible));
    assert!(!h.app.available("undo_last_preview"));
    h.app.selected_toolpath = Some(ids[1]);
    h.app.invoke(&h.ctx, "preview_toolpath");
    assert_eq!(h.app.simulation.as_ref(), Some(&all));
    h.app.invoke(&h.ctx, "undo_last_preview");
    assert_eq!(h.app.simulation.as_ref(), Some(&visible));
}

#[test]
fn node_editing_picks_nodes_with_a_box_and_a_drag_moves_them_together() {
    let mut h = Harness::new();
    h.rectangle(10.0, 10.0, 20.0, 10.0);
    let id = *h.app.doc.selection.iter().next().unwrap();
    h.app.invoke(&h.ctx, "node_edit");
    h.app.options.snapping.smart_snapping = false;
    h.app.options.snapping.geometry_snapping = false;
    // a box round the two right-hand corners picks them
    h.drag(h.at(25.0, 5.0), h.at(35.0, 25.0));
    assert_eq!(h.app.picked_nodes.len(), 2);
    assert_eq!(h.app.doc.selection.len(), 1, "the vector stays selected");
    // dragging one of them moves both 5 mm right
    h.drag(h.at(30.0, 10.0), h.at(35.0, 10.0));
    let corners = cut_craft_core::nodes::nodes(&h.app.doc.vectors[&id]);
    for corner in [Point::new(35.0, 10.0), Point::new(35.0, 20.0)] {
        assert!(
            corners.iter().any(|q| q.distance(corner) < 0.5),
            "{corner} in {corners:?}"
        );
    }
    assert!(
        corners
            .iter()
            .any(|q| q.distance(Point::new(10.0, 10.0)) < 0.5)
    );
    // a box with no nodes in it goes back to selecting vectors
    h.drag(h.at(100.0, 100.0), h.at(120.0, 120.0));
    assert!(h.app.picked_nodes.is_empty());
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
fn the_vector_selector_offers_only_circles_and_their_size() {
    use cut_craft_core::selector::{Circles, ClosedKind, Selector};
    let texts = |section: &mut Value| -> Vec<String> {
        let ctx = egui::Context::default();
        let out = headless(ctx.run_ui(RawInput::default(), |ui| {
            egui::CentralPanel::default().show(ui, |ui| {
                selector_section(ui, section, &[], Units::Inch);
            });
        }));
        out.shapes
            .iter()
            .filter_map(|c| match &c.shape {
                egui::Shape::Text(t) => Some(t.galley.text().to_owned()),
                _ => None,
            })
            .collect()
    };
    let mut section = serde_json::json!({ "selector": Selector::default() });
    let shown = texts(&mut section);
    assert!(shown.iter().any(|t| t == "Only circles"), "{shown:?}");
    assert!(
        !shown.iter().any(|t| t == "Tolerance"),
        "no size until circles of one are asked for"
    );
    section["selector"] = serde_json::to_value(Selector {
        closed_kind: ClosedKind::OnlyCircles,
        circles: Circles::SelectCirclesMatching,
        diameter: 25.4,
        ..Selector::default()
    })
    .unwrap();
    let shown = texts(&mut section);
    assert!(
        shown.iter().any(|t| t == "Circles of this diameter"),
        "{shown:?}"
    );
    assert!(
        shown.iter().any(|t| t == "1.000"),
        "the diameter in the job's units: {shown:?}"
    );
}

#[test]
fn the_preview_paints_machined_areas_in_the_materials_a_fill_or_each_toolpaths_colour() {
    let mut h = Harness::new();
    h.app.display.animate_preview = false;
    h.rectangle(10.0, 10.0, 50.0, 50.0);
    for id in ["drilling_toolpath", "profile_toolpath"] {
        h.app.invoke(&h.ctx, id);
        let params = h.app.form.as_ref().unwrap().params.clone();
        h.app.apply(id, &params).unwrap();
    }
    let ids: Vec<EntityId> = h.app.doc.toolpaths.keys().copied().collect();
    h.app.invoke(&h.ctx, "preview_all_toolpaths");
    let paints = |h: &mut Harness| -> std::collections::BTreeSet<u32> {
        h.app.surface_3d().unwrap().paint.iter().copied().collect()
    };
    let packed = |[r, g, b]: [u8; 3]| u32::from_be_bytes([1, r, g, b]);
    assert!(
        paints(&mut h).is_empty(),
        "the material's colour paints nothing"
    );
    // Toolpaths ▸ Materials: one fill colour over every machined area
    let mut p = h
        .app
        .registry
        .get("materials")
        .unwrap()
        .initial_params(&h.app.doc);
    p["machined_area"] = "global_fill_color".into();
    p["fill_colour"] = serde_json::json!([0, 0, 255]);
    h.app.apply("materials", &p).unwrap();
    assert_eq!(paints(&mut h), [0, packed([0, 0, 255])].into());
    // each toolpath's own colour: the drilling given red, the profile with no fill
    p["machined_area"] = "toolpath_color".into();
    h.app.apply("materials", &p).unwrap();
    h.frame(vec![]);
    let colour = |id: EntityId, preview: serde_json::Value| serde_json::json!({ "toolpath": id, "name": "t", "visible": true, "preview": preview });
    h.app
        .apply(
            "toolpath_properties",
            &colour(ids[0], serde_json::json!({ "colour": [255, 0, 0] })),
        )
        .unwrap();
    h.app
        .apply("toolpath_properties", &colour(ids[1], "no_fill".into()))
        .unwrap();
    assert_eq!(paints(&mut h), [0, packed([255, 0, 0])].into());
    assert!(
        h.app.doc.toolpaths[&ids[1]].preview_colour(1).is_none(),
        "no fill: the profile shows the material"
    );
}

/// Add to Project with several picked adds them one after another: a model through its
/// Orientate 3D Model form, the next once that form is closed; a file gone meanwhile is
/// reported, not a crash.
#[test]
fn library_adds_several_in_turn() {
    let mut h = Harness::new();
    let dir = std::env::temp_dir().join(format!("cut-craft-library-add-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let square = r#"<svg xmlns="http://www.w3.org/2000/svg" width="20mm" height="20mm" viewBox="0 0 20 20"><rect x="0" y="0" width="20" height="20"/></svg>"#;
    let (first, last) = (dir.join("first.svg"), dir.join("last.svg"));
    std::fs::write(&first, square).unwrap();
    std::fs::write(&last, square).unwrap();
    let model = pyramid_stl("library-add");
    h.app
        .add_from_library(vec![first, model, last, dir.join("gone.svg")], None);
    assert_eq!(h.app.doc.vectors.len(), 1, "the first is in");
    assert_eq!(
        h.app.form.as_ref().map(|f| f.id),
        Some("orientate_3d_model"),
        "the model waits on its form"
    );
    assert_eq!(h.app.import_queue.len(), 2, "the rest wait their turn");
    h.app.form = None; // the form closed
    h.frame(vec![]);
    assert_eq!(h.app.doc.vectors.len(), 2, "then the next comes in");
    assert!(h.app.import_queue.is_empty());
    assert!(
        h.app.status.message.contains("isn't there"),
        "{}",
        h.app.status.message
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn clipart_dropped_on_a_view_goes_there_and_import_to_level_puts_it_on_a_level() {
    use crate::clipart::ToLevel;
    use cut_craft_core::geometry::Point;
    let mut h = Harness::new();
    // a 20 mm square drawn as SVG, dropped on the 2D view at (100, 80)
    let svg = std::env::temp_dir().join(format!("cut-craft-drop-{}.svg", std::process::id()));
    std::fs::write(
        &svg,
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="20mm" height="20mm" viewBox="0 0 20 20"><rect x="0" y="0" width="20" height="20"/></svg>"#,
    )
    .unwrap();
    h.app
        .import_clipart(&svg, Some(Point::new(100.0, 80.0)), None);
    let bounds = h
        .app
        .doc
        .vectors
        .values()
        .map(cut_craft_core::geometry::Path::bounds)
        .reduce(cut_craft_core::geometry::Rect::union)
        .expect("the square came in");
    assert!(
        bounds.center().distance(Point::new(100.0, 80.0)) < 1e-6,
        "centred where dropped: {bounds:?}"
    );
    h.app.history.undo(&mut h.app.doc);
    assert!(h.app.doc.vectors.is_empty(), "one undo takes the drop away");
    let _ = std::fs::remove_file(svg);
    // a model to a new level named after it, placed where dropped as it is made
    let file = pyramid_stl("to-level");
    let levels = h.app.doc.levels.len();
    h.app
        .import_clipart(&file, Some(Point::new(150.0, 100.0)), Some(ToLevel::New));
    assert_eq!(h.app.doc.levels.len(), levels + 1);
    let (&new, level) = h.app.doc.levels.iter().next_back().unwrap();
    assert!(
        level.name.starts_with("cut-craft-to-level"),
        "{}",
        level.name
    );
    let k = h
        .app
        .doc
        .components
        .values()
        .next()
        .expect("the model is in");
    assert_eq!(k.level, new, "on the new level");
    assert!(
        k.extent().center().distance(Point::new(150.0, 100.0)) < 1.0,
        "centred where dropped: {:?}",
        k.extent()
    );
    // moving it in the form keeps it on the level
    let form = h.app.form.as_mut().unwrap();
    form.params["placing"]["position"] = serde_json::json!({ "x": 50.0, "y": 60.0 });
    form.dirty = true;
    h.frame(vec![]);
    assert_eq!(h.app.doc.components.values().next().unwrap().level, new);
    // and to an existing level
    h.app.form = None;
    h.app
        .import_clipart(&file, None, Some(ToLevel::Existing(0)));
    assert_ne!(new, 0, "the job starts with level 0");
    assert_eq!(h.app.doc.level_components(0).len(), 1, "the second model");
    assert_eq!(h.app.doc.level_components(new).len(), 1, "the first stays");
    let _ = std::fs::remove_file(file);
}

#[test]
fn a_polyline_is_drawn_point_by_point_with_curves_and_its_keys() {
    use cut_craft_core::geometry::Segment;
    let mut h = Harness::new();
    h.app.options.snapping.smart_snapping = false;
    h.app.options.snapping.geometry_snapping = false;
    h.app.invoke(&h.ctx, "draw_polyline");
    h.frame(vec![]);
    // clicks make straight spans, a drag a curved one
    let click = |h: &mut Harness, x, y| {
        let at = h.at(x, y);
        h.click(at, Modifiers::NONE);
    };
    click(&mut h, 10.0, 10.0);
    click(&mut h, 60.0, 10.0);
    let (from, to) = (h.at(60.0, 50.0), h.at(70.0, 50.0));
    h.drag(from, to);
    // a length typed and Enter: 20 along the pointer, here straight left
    let left = h.at(0.0, 50.0);
    h.frame(vec![Event::PointerMoved(left), Event::Text("20".into())]);
    h.frame(vec![key(Key::Enter)]);
    assert_eq!(h.app.polyline.points.len(), 4);
    assert!(
        h.app.polyline.points[3].distance(Point::new(40.0, 50.0)) < 1e-3,
        "{:?}",
        h.app.polyline.points[3]
    );
    // Backspace takes it back, Tab closes and makes the polyline
    h.frame(vec![key(Key::Backspace)]);
    assert_eq!(h.app.polyline.points.len(), 3);
    h.frame(vec![key(Key::Tab)]);
    let path = h.app.doc.vectors.values().last().expect("drawn");
    assert!(path.closed);
    assert_eq!(path.segments.len(), 3);
    assert!(
        matches!(path.segments[1], Segment::Cubic { .. }),
        "the dragged span"
    );
    assert!(matches!(path.segments[0], Segment::Line { .. }));
    assert!(h.app.polyline.is_empty(), "ready for the next");
    assert_eq!(h.app.form.as_ref().map(|f| f.id), Some("draw_polyline"));
    // S: curve mode, Space finishes an open one
    h.frame(vec![key(Key::S)]);
    assert!(h.app.polyline.curved);
    click(&mut h, 100.0, 100.0);
    click(&mut h, 150.0, 120.0);
    click(&mut h, 200.0, 100.0);
    h.frame(vec![key(Key::Space)]);
    let path = h.app.doc.vectors.values().last().unwrap();
    assert!(!path.closed);
    assert!(
        path.segments
            .iter()
            .all(|s| matches!(s, Segment::Cubic { .. }))
    );
    assert!(h.app.polyline.curved, "the mode is kept");
}

#[test]
fn edit_polyline_from_an_end_node_draws_on_and_extends_the_same_vector() {
    use cut_craft_core::nodes::NodeAt;
    use cut_craft_core::params::Xy;
    let mut h = Harness::new();
    h.app.options.snapping.smart_snapping = false;
    h.app.options.snapping.geometry_snapping = false;
    let line = serde_json::json!({
        "points": [{ "x": 10.0, "y": 10.0 }, { "x": 50.0, "y": 10.0 }], "closed": false
    });
    h.app.apply("draw_polyline", &line).unwrap();
    let id = *h.app.doc.vectors.keys().next().unwrap();
    // the node menu offers it at either end, not in the middle of a closed shape
    let at = |x, y| NodeAt {
        vector: id,
        at: Xy::new(x, y),
        reach: 1.0,
    };
    let offered = |h: &Harness, x, y| {
        gui::node_menu(&h.app.doc, &at(x, y))
            .iter()
            .find(|i| i.name == "Edit Polyline")
            .map(|i| i.action.clone())
    };
    let action = offered(&h, 50.0, 10.0).expect("at the end");
    assert!(offered(&h, 10.0, 10.0).is_some_and(|a| a.ends_with(":start")));
    h.app.invoke(&h.ctx, &action);
    h.frame(vec![]);
    let to = h.at(50.0, 40.0);
    h.click(to, Modifiers::NONE);
    h.frame(vec![key(Key::Space)]);
    assert_eq!(h.app.doc.vectors.len(), 1, "extended, not a new vector");
    let path = &h.app.doc.vectors[&id];
    assert_eq!(path.segments.len(), 2);
    assert!(path.segments[1].end().distance(Point::new(50.0, 40.0)) < 1e-3);
    // undo takes the extension back
    h.app.history.undo(&mut h.app.doc);
    assert_eq!(h.app.doc.vectors[&id].segments.len(), 1);
}

#[test]
fn sculpting_works_in_the_3d_view_with_its_keys_and_wheel() {
    let mut h = Harness::new();
    h.app.doc = cut_craft_core::testing::domed();
    h.app.view = View::ThreeD;
    h.frame(vec![]);
    let total = |h: &Harness| -> f64 {
        let k = h.app.doc.components.values().next().unwrap();
        k.heights.iter().filter(|z| !z.is_nan()).sum()
    };
    let before = total(&h);
    h.app.invoke(&h.ctx, "sculpt");
    h.frame(vec![]);
    // 3 picks Deposit; the wheel grows the brush, Shift and the wheel its strength
    let mid = h.app.view_3d_area.expect("the 3D view is drawn").center();
    let wheel = |modifiers| Event::MouseWheel {
        unit: egui::MouseWheelUnit::Line,
        delta: egui::vec2(0.0, 1.0),
        modifiers,
        phase: egui::TouchPhase::Move,
        source: egui::MouseWheelSource::Wheel,
    };
    h.frame(vec![
        Event::PointerMoved(mid),
        key(Key::Num3),
        wheel(Modifiers::NONE),
    ]);
    let params = &h.app.form.as_ref().unwrap().params;
    assert_eq!(params["tool"], "deposit");
    assert!(
        (params["diameter"].as_f64().unwrap() - 11.0).abs() < 1e-9,
        "{params}"
    );
    h.frame_holding(Modifiers::SHIFT, vec![wheel(Modifiers::SHIFT)]);
    let strength = h.app.form.as_ref().unwrap().params["strength"]
        .as_f64()
        .unwrap();
    assert!((strength - 22.0).abs() < 1e-9, "{strength}");
    let orbit = h.app.orbit;
    // a left drag over the model sculpts it, and does not turn the view
    h.press(mid, true, Modifiers::NONE);
    for dx in [5.0, 10.0, 15.0, 20.0] {
        h.frame(vec![Event::PointerMoved(mid + egui::vec2(dx, 0.0))]);
    }
    h.press(mid + egui::vec2(20.0, 0.0), false, Modifiers::NONE);
    h.frame(vec![]);
    assert!(total(&h) > before, "the stroke deposited");
    assert_eq!(h.app.orbit, orbit, "the view stayed put");
}

#[test]
fn a_double_sided_preview_cuts_both_sides_and_turns_over_with_the_job() {
    let mut h = Harness::new();
    h.app.display.animate_preview = false;
    h.app.doc.job.double_sided = Some(cut_craft_core::document::DoubleSided::default());
    // a pocket on this side, a pocket on the other
    let pocket = |h: &mut Harness| {
        h.rectangle(10.0, 10.0, 40.0, 40.0);
        h.app.invoke(&h.ctx, "pocket_toolpath");
        let params = h.app.form.as_ref().unwrap().params.clone();
        h.app.apply("pocket_toolpath", &params).unwrap();
        h.app.form = None;
    };
    pocket(&mut h);
    h.app.apply("swap_sides", &serde_json::json!({})).unwrap();
    pocket(&mut h);
    h.app.apply("swap_sides", &serde_json::json!({})).unwrap();
    assert_eq!(h.app.doc.other_side.toolpaths.len(), 1);
    h.app.invoke(&h.ctx, "preview_all_toolpaths");
    let preview = h.app.simulation.as_ref().expect("previewed");
    assert!(
        preview.other_side.is_some(),
        "the other side is previewed too"
    );
    // the 3D view's surface has the other side's pocket on its underside
    let surface = h.app.surface_3d().unwrap().into_owned();
    let bottom = surface.bottom;
    assert!(
        surface.under.iter().any(|&z| z > bottom + 1e-6),
        "the underside is cut"
    );
    // turned over, the other side's cuts are on top and this side's underneath
    let before = h.app.simulation.clone().unwrap();
    h.app.apply("swap_sides", &serde_json::json!({})).unwrap();
    let after = h.app.simulation.as_ref().expect("still previewed");
    assert_eq!(
        after.other_side.as_deref().map(|m| &m.heights),
        Some(&before.heights),
        "the side that was up is now underneath"
    );
    h.frame(vec![]);
}

#[test]
fn save_shaded_image_saves_the_3d_view_s_part_of_the_window() {
    let mut h = Harness::new();
    h.app.doc = cut_craft_core::testing::domed();
    assert!(h.app.available("save_shaded_image"));
    let path = std::env::temp_dir().join(format!("cut-craft-shaded-{}.png", std::process::id()));
    h.app.save_shaded_image(path.clone());
    h.frame(vec![]);
    assert_eq!(h.app.view, View::ThreeD, "the 3D view is shown");
    let asked = h
        .screenshot
        .take()
        .expect("the window's picture is asked for");
    // the window's picture, 1280 x 800, red inside the 3D view and blue outside it
    let area = h.app.view_3d_area.unwrap();
    let mut picture = egui::ColorImage::filled([1280, 800], egui::Color32::BLUE);
    for y in 0..800 {
        for x in 0..1280 {
            #[expect(clippy::cast_precision_loss, reason = "pixel coordinates")]
            if area.contains(egui::pos2(x as f32 + 0.5, y as f32 + 0.5)) {
                picture.pixels[y * 1280 + x] = egui::Color32::RED;
            }
        }
    }
    asked.complete(std::sync::Arc::new(picture));
    h.frame(vec![]);
    assert!(
        h.app.status.message.starts_with("Saved the shaded image"),
        "{}",
        h.app.status.message
    );
    let saved = image::open(&path).unwrap().to_rgba8();
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "pixels"
    )]
    let size = (area.width().round() as u32, area.height().round() as u32);
    assert_eq!(saved.dimensions(), size);
    assert!(
        saved.pixels().all(|p| p.0 == [255, 0, 0, 255]),
        "only the 3D view"
    );
    let _ = std::fs::remove_file(path);
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
    let apply = *h.texts_at("Apply").last().expect("the notes have Apply");
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
fn a_form_tool_takes_its_shape_from_the_selected_vector() {
    let mut h = Harness::new();
    let profile = serde_json::json!({ "points": [
        { "x": 10.0, "y": 10.0 }, { "x": 14.0, "y": 10.0 }, { "x": 16.0, "y": 13.0 }
    ], "closed": false });
    h.app.apply("draw_polyline", &profile).unwrap();
    h.app.form = None;
    h.app.tools = ToolDatabase {
        tools: vec![Tool {
            shape: cut_craft_core::toolpath::ToolShape::FormTool,
            ..Tool::default()
        }],
        ..ToolDatabase::default()
    };
    h.app.invoke(&h.ctx, "tool_database");
    h.app.tool_manager.as_mut().unwrap().pick = Some(cut_craft_tool_database::Pick::Tool(0));
    h.frame(vec![]);
    h.frame(vec![]);
    let button = h
        .text_at("Use Selected Vector")
        .expect("a form tool offers it");
    h.click(button, Modifiers::NONE);
    let tool = &h.app.tool_manager.as_ref().unwrap().draft.tools[0];
    assert_eq!(tool.form.len(), 3);
    assert!((tool.diameter - 12.0).abs() < 1e-9);
}

#[test]
fn e_and_shift_click_edit_a_shape_in_the_tool_that_drew_it() {
    let mut h = Harness::new();
    h.app.options.snapping.smart_snapping = false;
    h.app.options.snapping.geometry_snapping = false;
    h.rectangle(10.0, 10.0, 20.0, 20.0);
    let rect = *h.app.doc.selection.iter().next().unwrap();
    assert!(h.app.available("edit_object"));
    // E opens Draw Rectangle on it; a change to the form changes that rectangle
    h.frame(vec![key(Key::E)]);
    assert_eq!(h.app.form.as_ref().map(|f| f.id), Some("draw_rectangle"));
    let form = h.app.form.as_mut().unwrap();
    assert_eq!(form.params["width"], 20.0);
    form.params["width"] = serde_json::json!(40.0);
    form.dirty = true;
    h.frame(vec![]);
    assert_eq!(h.app.doc.vectors.len(), 1, "changed, not drawn again");
    assert!((h.app.doc.vectors[&rect].bounds().size().x - 40.0).abs() < 1e-9);
    // a second rectangle drawn elsewhere, then Shift-click edits the first one
    let second = serde_json::json!({ "x": 100.0, "y": 100.0, "width": 10.0, "height": 10.0,
        "corners": "rounded", "radius": 2.0 });
    h.app.form = None;
    h.app.apply("draw_rectangle", &second).unwrap();
    h.app.invoke(&h.ctx, "draw_rectangle");
    h.frame(vec![]);
    let edge = h.at(10.0, 20.0);
    h.click(edge, Modifiers::SHIFT);
    let form = h.app.form.as_ref().unwrap();
    assert_eq!(form.params["width"], 40.0, "the first rectangle's settings");
    assert_eq!(
        h.app.doc.selection.iter().copied().collect::<Vec<_>>(),
        [rect]
    );
    // a circle is not Draw Rectangle's
    h.app.form = None;
    h.app
        .apply(
            "draw_circle",
            &serde_json::json!({ "center": { "x": 200.0, "y": 50.0 }, "radius": 5.0 }),
        )
        .unwrap();
    h.frame(vec![key(Key::E)]);
    assert_eq!(h.app.form.as_ref().map(|f| f.id), Some("draw_circle"));
}

#[test]
fn e_on_a_shape_component_opens_create_shape_on_it() {
    let mut h = Harness::new();
    h.rectangle(10.0, 10.0, 40.0, 40.0);
    let mut params = h.app.registry.get("create_shape").unwrap().default_params();
    params["profile"] = "angular".into();
    h.app.apply("create_shape", &params).unwrap();
    let id = *h.app.doc.components.keys().next().unwrap();
    h.app.doc.selection = cut_craft_core::selection::Selection::default();
    h.app.doc.selected_components = [id].into();
    assert!(h.app.available("edit_object"));
    h.frame(vec![key(Key::E)]);
    let form = h.app.form.as_mut().expect("Create Shape opened");
    assert_eq!(form.id, "create_shape");
    assert_eq!(form.params["profile"], "angular");
    let mut edited = form.params.clone();
    edited["profile"] = "flat".into();
    edited["height"] = 3.0.into();
    h.app.apply_form("create_shape", &edited).unwrap();
    assert_eq!(h.app.doc.components.len(), 1, "remade, not added");
    let top = h.app.doc.components[&id]
        .height_at(Point::new(30.0, 30.0))
        .unwrap();
    assert!((top - 3.0).abs() < 1e-9, "{top}");
}

#[test]
fn sheets_take_selected_vectors_and_toolpaths_from_their_menus() {
    let mut h = Harness::new();
    h.app.doc = cut_craft_core::testing::two_sheets();
    let menu = gui::sheet_menu(&h.app.doc, 1);
    let item = |name: &str| menu.iter().find(|m| m.name == name).unwrap();
    assert!(item("Copy Selected Vectors Here").is_enabled);
    let selected = h.app.doc.selection.len();
    let action = item("Move Selected Vectors Here").action.clone();
    h.app.invoke(&h.ctx, &action);
    assert_eq!(h.app.doc.sheets[1].objects.vectors.len(), selected);
    // the active sheet's own menu cannot send to itself
    let own = gui::sheet_menu(&h.app.doc, 0);
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
fn export_as_stl_opens_its_form_and_keeps_its_settings() {
    let mut h = Harness::new();
    h.app.invoke(&h.ctx, "export_as_stl");
    h.frame(vec![]);
    h.frame(vec![]);
    assert!(h.app.stl_window);
    for text in [
        "Export Model as STL",
        "Triangulate",
        "Back face of triangulation",
    ] {
        assert!(h.text_at(text).is_some(), "{text}");
    }
    assert_eq!(h.app.options.stl, fmt_export_as_stl::Settings::default());
}

#[test]
fn a_cut_list_imports_as_parts_and_nest_parts_opens_to_move_them() {
    let mut h = Harness::new();
    let dir = std::env::temp_dir().join(format!("cut-craft-cut-list-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let file = dir.join("kitchen.csv");
    std::fs::write(&file, "Name,Length,Width,Qty\nSide,72,56,2\n").unwrap();
    h.app.import_cut_list(&file);
    assert_eq!(h.app.doc.vectors.len(), 2);
    assert_eq!(h.app.doc.part_names.values().next().unwrap(), "Side");
    let form = h.app.form.as_ref().expect("Nest Parts opens next");
    assert_eq!(form.id, "nest_parts");
    assert_eq!(form.params["remove_original_parts"], true);
    std::fs::remove_dir_all(&dir).ok();
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
fn the_layers_list_arrows_move_the_active_layer_and_merge_visible_is_offered() {
    let mut h = Harness::new();
    h.app.doc = cut_craft_core::testing::two_layers();
    let first = h.app.doc.layers[0].id;
    h.app.doc.current_layer = first;
    h.app.pages.set_pinned(Page::Design, true);
    h.app.design_tab = DESIGN_TABS.iter().position(|&t| t == "Layers").unwrap();
    h.frame(vec![]);
    h.frame(vec![]);
    let up = *h
        .texts_at("Move Up")
        .first()
        .expect("the arrows are over the list");
    h.click(up, Modifiers::NONE);
    assert_eq!(
        h.app.doc.layers.last().map(|l| l.id),
        Some(first),
        "now at the top"
    );
    let menu = gui::layer_menu(&h.app.doc, &h.app.doc.layers[0]);
    let merge = leaf(&menu, &["Merge Visible"]);
    assert!(merge.is_enabled, "another layer is visible");
    // layers with nothing on this sheet can be left out of the list
    let empty = h
        .app
        .doc
        .layers
        .iter()
        .find(|l| h.app.doc.vectors_on(l.id).next().is_none() && l.id != first)
        .map(|l| l.name.clone());
    if let Some(name) = empty {
        h.app.layers_on_sheet = true;
        h.frame(vec![]);
        assert!(h.text_at(&name).is_none(), "{name} is left out");
    }
}

#[test]
fn a_texture_toolpaths_texture_saves_to_a_file_and_loads_into_another_form() {
    let dir = std::env::temp_dir().join(format!("cut-craft-texture-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("scallops.vtexture");
    let mut h = Harness::new();
    assert_eq!(
        h.app.save_texture(&path).unwrap_err(),
        "no Texture Toolpath form is open"
    );
    h.app.invoke(&h.ctx, "texture_toolpath");
    h.app.form.as_mut().unwrap().params["stepover"] = Value::from(7.5);
    h.app.save_texture(&path).unwrap();
    let text = std::fs::read_to_string(&path).unwrap();
    assert!(text.contains("\r\nStepover: 7.50000 0\r\n"), "{text}");
    h.app.form = None;
    h.app.invoke(&h.ctx, "texture_toolpath");
    h.app.form.as_mut().unwrap().params["name"] = Value::from("Lid");
    h.app.load_texture(&path).unwrap();
    let form = h.app.form.as_ref().unwrap();
    assert_eq!(form.params["stepover"], 7.5);
    assert_eq!(form.params["name"], "Lid", "the form keeps its own name");
    assert!(h.app.status.message.contains("Loaded texture"));
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn toolpath_groups_are_listed_picked_previewed_saved_as_one_and_joined_by_new_toolpaths() {
    let mut h = Harness::new();
    h.app.doc = cut_craft_core::testing::three_toolpaths();
    h.app.screen = Screen::Job;
    h.app.pages.set_pinned(Page::Toolpaths, true);
    // Group Visible, with the third toolpath hidden
    let hide = serde_json::json!({ "toolpath": 3, "name": "Profile", "visible": false });
    h.app.apply("toolpath_properties", &hide).unwrap();
    let shown: Vec<u64> = h
        .app
        .doc
        .toolpaths
        .iter()
        .filter(|(_, t)| t.visible)
        .map(|(id, _)| id.0)
        .collect();
    let group =
        serde_json::json!({ "toolpaths": shown, "group": h.app.doc.next_toolpath_group_name() });
    h.app.apply("group_toolpaths", &group).unwrap();
    assert_eq!(h.app.doc.toolpath_groups, ["Group 1"]);
    h.frame(vec![]);
    h.frame(vec![]);
    // the group's row is drawn; a click picks the group instead of a toolpath
    h.app.selected_toolpath = Some(EntityId(3));
    let row = h.text_at("Group 1").unwrap();
    h.click(row, Modifiers::NONE);
    assert_eq!(h.app.selected_group.as_deref(), Some("Group 1"));
    assert_eq!(h.app.selected_toolpath, None);
    // Save Toolpaths' Selected mode saves the group as one file
    let plan = files::save_plan(
        &h.app.doc,
        files::SaveMode::Selected,
        None,
        Some("Group 1"),
        false,
    );
    assert_eq!(plan.unwrap(), [vec![EntityId(1), EntityId(2)]]);
    // a toolpath made while the group is picked joins it
    h.rectangle(10.0, 10.0, 50.0, 50.0);
    h.app.invoke(&h.ctx, "drilling_toolpath");
    let params = h.app.form.as_ref().unwrap().params.clone();
    h.app.apply("drilling_toolpath", &params).unwrap();
    let newest = h.app.doc.toolpaths.values().last().unwrap();
    assert_eq!(newest.group.as_deref(), Some("Group 1"));
    // Preview with the group picked previews its toolpaths
    h.app.display.animate_preview = false;
    assert!(h.app.available("preview_toolpath"));
    h.app.invoke(&h.ctx, "preview_toolpath");
    assert!(h.app.simulation.is_some());
    // ungrouping keeps the toolpaths
    let ungroup = serde_json::json!({ "group": "Group 1", "delete_toolpaths": false });
    h.app.apply("ungroup_toolpaths", &ungroup).unwrap();
    assert_eq!(h.app.doc.toolpath_groups, Vec::<String>::new());
    assert!(h.app.doc.toolpaths.values().all(|t| t.group.is_none()));
}

#[test]
fn with_create_fillets_open_a_click_rounds_a_corner_and_a_click_on_the_fillet_takes_it_out() {
    let mut h = Harness::new();
    h.rectangle(10.0, 10.0, 40.0, 30.0);
    h.app.invoke(&h.ctx, "create_fillets");
    assert!(h.app.filleting());
    h.frame(vec![]);
    let shape = |h: &Harness| {
        let path = h.app.doc.vectors.values().next().unwrap();
        let arcs = path
            .segments
            .iter()
            .filter(|s| matches!(s, cut_craft_core::geometry::Segment::Arc { .. }))
            .count();
        (path.segments.len(), arcs)
    };
    h.click(h.at(10.0, 10.0), Modifiers::NONE);
    assert_eq!(shape(&h), (5, 1), "one corner rounded");
    assert!(h.app.filleting(), "the form stays open");
    // the fillet's middle: a click there takes it out
    let mid = 10.0 + 3.0 - 3.0 * std::f64::consts::FRAC_1_SQRT_2;
    h.frame(vec![]);
    h.click(h.at(mid, mid), Modifiers::NONE);
    assert_eq!(shape(&h), (4, 0), "the sharp corner is back");
}

#[test]
fn with_join_open_ends_open_two_clicks_on_ends_join_the_vectors() {
    let mut h = Harness::new();
    let line = |a: (f64, f64), b: (f64, f64)| {
        Path::polyline(&[Point::new(a.0, a.1), Point::new(b.0, b.1)], false)
    };
    let paths = vec![
        line((10.0, 10.0), (40.0, 10.0)),
        line((45.0, 10.0), (80.0, 10.0)),
    ];
    let change = Change::swap(&h.app.doc, [], paths);
    h.app.doc.apply(&change).unwrap();
    h.app.invoke(&h.ctx, "join_open_ends");
    assert!(h.app.joining_ends());
    h.frame(vec![]);
    // a click away from any end picks nothing
    h.click(h.at(25.0, 10.0), Modifiers::NONE);
    assert!(h.app.join_ends_first().is_none());
    h.frame(vec![]);
    h.click(h.at(40.0, 10.0), Modifiers::NONE);
    assert!(h.app.join_ends_first().is_some(), "the first end is picked");
    h.frame(vec![]);
    h.click(h.at(45.0, 10.0), Modifiers::NONE);
    assert_eq!(h.app.doc.vectors.len(), 1, "joined into one");
    assert!(h.app.join_ends_first().is_none(), "ready for the next pair");
    assert!(h.app.joining_ends(), "the tool stays open");
    // its two ends clicked, the one vector closes
    h.frame(vec![]);
    h.click(h.at(10.0, 10.0), Modifiers::NONE);
    h.frame(vec![]);
    h.click(h.at(80.0, 10.0), Modifiers::NONE);
    assert!(h.app.doc.vectors.values().next().unwrap().closed);
}

#[test]
fn escape_lets_a_picked_open_end_go_and_keeps_join_open_ends_open() {
    let mut h = Harness::new();
    let path = Path::polyline(&[Point::new(10.0, 10.0), Point::new(40.0, 10.0)], false);
    let change = Change::swap(&h.app.doc, [], vec![path]);
    h.app.doc.apply(&change).unwrap();
    h.app.invoke(&h.ctx, "join_open_ends");
    h.frame(vec![]);
    h.click(h.at(40.0, 10.0), Modifiers::NONE);
    assert!(h.app.join_ends_first().is_some());
    h.frame(vec![key(Key::Escape)]);
    assert!(h.app.join_ends_first().is_none(), "the end let go");
    assert!(h.app.joining_ends(), "the tool still open");
}

#[test]
fn show_open_vector_ends_is_a_view_menu_switch() {
    let mut h = Harness::new();
    assert!(!h.app.display.open_ends, "off to start with");
    h.app.invoke(&h.ctx, "show_open_vector_ends");
    assert!(h.app.display.open_ends);
    assert_eq!(h.app.display.checked("show_open_vector_ends"), Some(true));
}

#[test]
fn freehands_stabilizing_circle_is_drawn_round_the_paths_end_while_dragging() {
    let mut h = Harness::new();
    h.app.invoke(&h.ctx, "draw_freehand");
    h.app.form.as_mut().unwrap().params["stabilize"] = Value::from(2.0);
    h.app.stroke = vec![Point::new(0.0, 0.0), Point::new(10.0, 0.0)];
    let drag = cut_craft_core::Drag {
        start: Point::new(0.0, 0.0),
        end: Point::new(10.0, 0.0),
        from_center: false,
        constrain: false,
    };
    let paths = h.app.preview("draw_freehand", drag);
    assert_eq!(paths.len(), 2, "the path and the circle");
    let b = paths[1].bounds();
    assert!((b.center() - Point::new(8.0, 0.0)).length() < 1e-6, "{b:?}");
    assert!((b.size().x - 4.0).abs() < 1e-6, "{b:?}");
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
fn orientate_3d_models_rings_turn_the_model_about_their_axis_and_the_sphere_about_all() {
    let mut h = Harness::new();
    let file = pyramid_stl("rings");
    h.app.import_file(&file);
    h.app.view = View::ThreeD;
    h.frame(vec![]);
    h.frame(vec![]);
    let (rings, sphere) = h.app.rings_3d.clone();
    let axes: Vec<&str> = rings.iter().map(|(a, _)| *a).collect();
    assert_eq!(axes, ["x", "y", "z"], "red, green and blue rings");
    let (centre, _) = sphere.expect("the grey sphere");
    let turn = |h: &Harness| h.app.form.as_ref().unwrap().params["turn"].clone();
    assert_eq!(
        turn(&h)["mode"],
        "view",
        "drags turn the view unless a handle is taken"
    );
    // a quarter of the way round the red ring, dragged up: a turn about X only
    let on_x = rings[0].1[rings[0].1.len() / 4];
    h.drag(on_x, on_x - vec2(0.0, 60.0));
    let t = turn(&h);
    assert!(t["x"].as_f64().unwrap().abs() > 1.0, "{t}");
    assert_eq!((t["y"].as_f64(), t["z"].as_f64()), (Some(0.0), Some(0.0)));
    assert!(h.app.turn_ring.is_none(), "let go");
    // the sphere, dragged across and up: about all three (X and Z here)
    let before = turn(&h);
    h.drag(centre, centre + vec2(60.0, -60.0));
    let t = turn(&h);
    assert!(t["z"].as_f64() != before["z"].as_f64() && t["x"].as_f64() != before["x"].as_f64());
    // anywhere else turns the view, not the model
    let orbit = h.app.orbit;
    let corner = h.app.fit_3d.unwrap().left_bottom() + vec2(10.0, -10.0);
    let before = turn(&h);
    h.drag(corner, corner + vec2(60.0, 0.0));
    assert_eq!(turn(&h), before);
    assert_ne!(h.app.orbit, orbit, "the view turned");
    let _ = std::fs::remove_file(file);
}

#[test]
fn a_click_between_two_letters_with_draw_text_open_pulls_them_together() {
    let mut h = Harness::new();
    h.app.invoke(&h.ctx, "draw_text");
    let mut params = h.app.form.as_ref().unwrap().params.clone();
    params["text"] = "HI".into();
    params["font"] = "Test Boxes".into();
    params["height"] = 10.0.into();
    params["anchor"] = serde_json::json!({ "x": 10.0, "y": 10.0 });
    h.app.form.as_mut().unwrap().params = params.clone();
    h.app.apply_form("draw_text", &params).unwrap();
    let width = |h: &Harness| {
        h.app
            .doc
            .vectors
            .values()
            .map(cut_craft_core::geometry::Path::bounds)
            .reduce(cut_craft_core::geometry::Rect::union)
            .unwrap()
            .size()
            .x
    };
    let before = width(&h);
    h.frame(vec![]);
    // between the H and the I, at the gap's middle
    h.app
        .text_click(Point::new(19.0, 15.0), 1.0, Modifiers::NONE);
    let form = h.app.form.as_ref().unwrap();
    assert_eq!(
        form.params["spacing"]["kerning"][0],
        serde_json::json!([0, 0, -0.5])
    );
    assert!(
        (width(&h) - (before - 0.5)).abs() < 1e-6,
        "the letters followed at once: {}",
        width(&h)
    );
    // Shift and Ctrl push them apart by four steps; a click off the text places the anchor
    h.app.text_click(
        Point::new(18.5, 15.0),
        1.0,
        Modifiers::SHIFT | Modifiers::CTRL,
    );
    let form = h.app.form.as_ref().unwrap();
    assert_eq!(
        form.params["spacing"]["kerning"][0],
        serde_json::json!([0, 0, 1.5])
    );
    h.app
        .text_click(Point::new(80.0, 80.0), 1.0, Modifiers::NONE);
    let form = h.app.form.as_ref().unwrap();
    assert_eq!(
        form.params["anchor"],
        serde_json::json!({ "x": 80.0, "y": 80.0 })
    );
}

#[test]
fn dragging_the_green_handle_over_edited_text_bends_it_into_an_arc() {
    let mut h = Harness::new();
    h.app.invoke(&h.ctx, "draw_text");
    let mut params = h.app.form.as_ref().unwrap().params.clone();
    params["text"] = "HIHIH".into();
    params["font"] = "Test Boxes".into();
    params["height"] = 10.0.into();
    params["anchor"] = serde_json::json!({ "x": 10.0, "y": 10.0 });
    h.app.form.as_mut().unwrap().params = params.clone();
    h.app.apply_form("draw_text", &params).unwrap();
    h.frame(vec![]);
    let p: cmd_draw_text::Params =
        serde_json::from_value(h.app.form.as_ref().unwrap().params.clone()).unwrap();
    let [top, _] = cmd_draw_text::arc_handles(&p).expect("handles on the edited text");
    let drag = Drag {
        start: top,
        end: top + Point::new(0.0, 5.0),
        from_center: false,
        constrain: false,
    };
    h.app.drawing_gesture(
        "draw_text",
        Some(view2d::Gesture::Dropped { drag, shift: false }),
        1.0,
        Modifiers::NONE,
    );
    let arc = h.app.form.as_ref().unwrap().params["arc"].as_f64().unwrap();
    assert!(arc > 10.0, "bent up: {arc}");
    let low = h
        .app
        .doc
        .vectors
        .values()
        .map(|v| v.bounds().min.y)
        .fold(f64::INFINITY, f64::min);
    assert!(low < 9.0, "the ends dropped: {low}");
    assert_eq!(h.app.doc.vectors.len(), 5, "the letters replaced");
}

#[test]
fn h_and_v_make_mirror_lines_and_the_arrows_move_picked_nodes_mirrored() {
    use cut_craft_core::geometry::Rect as JobRect;
    use cut_craft_core::nodes::nodes;
    let mut h = Harness::new();
    h.rectangle(10.0, 10.0, 20.0, 20.0);
    h.app.invoke(&h.ctx, "node_edit");
    assert_eq!(h.app.view_tool, Some(ViewTool::NodeEdit));
    let id = *h.app.doc.vectors.keys().next().unwrap();
    // the two top nodes picked, as a box round them picks them
    let top = JobRect::new(Point::new(5.0, 25.0), Point::new(35.0, 35.0));
    h.app.picked_nodes = h.app.nodes_in(top);
    assert_eq!(h.app.picked_nodes.len(), 2);
    h.frame(vec![key(Key::H)]);
    assert_eq!(
        h.app.node_mirror,
        [true, false],
        "H: a mirror line, not Mirror"
    );
    let step = h.app.options.snapping.grid_spacing;
    // the arrows move the two sides opposite ways: Left takes the left node left and the
    // right node right
    h.frame(vec![key(Key::ArrowLeft)]);
    let path = &h.app.doc.vectors[&id];
    let xs_above = |y: f64| -> Vec<f64> {
        nodes(path)
            .iter()
            .filter(|p| if y > 0.0 { p.y > 20.0 } else { p.y < 20.0 })
            .map(|p| p.x)
            .collect()
    };
    let mut top = xs_above(1.0);
    top.sort_by(f64::total_cmp);
    // the top widened by a step each way
    assert!(
        (top[0] - (10.0 - step)).abs() < 1e-9 && (top[1] - (30.0 + step)).abs() < 1e-9,
        "{top:?}"
    );
    let mut bottom = xs_above(-1.0);
    bottom.sort_by(f64::total_cmp);
    assert_eq!(bottom, [10.0, 30.0], "the rest of the shape stayed");
    assert!(
        h.app
            .picked_nodes
            .iter()
            .all(|n| (n.at.x - 10.0 + step).abs() < 1e-9 || (n.at.x - 30.0 - step).abs() < 1e-9),
        "the picks follow"
    );
    // V adds the other line, H again takes its line off; the overlay draws what is on
    h.frame(vec![key(Key::V)]);
    h.frame(vec![key(Key::H)]);
    assert_eq!(h.app.node_mirror, [false, true]);
    match h.app.node_overlay(None, 1.0) {
        crate::view2d::Overlay::Nodes { moving, .. } => assert_eq!(moving.len(), 1, "one line"),
        _ => panic!("the node overlay"),
    }
    // leaving node editing takes the lines away
    h.app.invoke(&h.ctx, "node_edit");
    assert_eq!(h.app.node_mirror, [false, false]);
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
fn a_shift_click_on_a_dimension_with_its_form_open_edits_it() {
    let mut h = Harness::new();
    let mut params = serde_json::to_value(cmd_dimensions::Params::default()).unwrap();
    params["font"] = "Test Boxes".into();
    params["custom_text"] = "W".into();
    h.app.apply("dimensions", &params).unwrap();
    let group = *h.app.doc.texts.keys().next().unwrap();
    h.app.invoke(&h.ctx, "dimensions");
    // the dimension line runs from (10, 20) to (60, 20): a Shift-click on it
    h.app
        .shift_click_shape("dimensions", Point::new(35.0, 20.0), 1.0);
    let form = h.app.form.as_ref().unwrap();
    assert_eq!(form.params["replaces"], serde_json::json!(group));
    assert_eq!(form.params["custom_text"], "W");
    assert!(form.drawn.is_some(), "the shape it edits");
}

#[test]
fn guides_are_inserted_from_the_menu_and_their_properties_turn_lock_and_add_parallels() {
    let mut h = Harness::new();
    h.app.menu_at = Some(Point::new(40.0, 30.0));
    for insert in ["guide:insert:v", "guide:insert:h", "guide:insert:a"] {
        assert!(h.app.available(insert));
        h.app.invoke(&h.ctx, insert);
    }
    let guides = h.app.doc.guides.clone();
    assert_eq!(guides.len(), 3);
    assert!(guides[0].vertical && (guides[0].at - 40.0).abs() < 1e-9);
    assert!(!guides[1].vertical && (guides[1].at - 30.0).abs() < 1e-9);
    let angled = guides[2];
    assert!((angled.angle() - 45.0).abs() < 1e-9, "{angled:?}");
    assert!(
        angled.distance(Point::new(40.0, 30.0)) < 1e-9,
        "through where the menu opened"
    );
    assert_eq!(
        h.app.guide_props.as_ref().map(|p| p.index),
        Some(2),
        "its properties open"
    );
    // Guide Properties on the first: a new position and angle, locked
    h.app.guide_props = Some(GuideProps {
        index: 0,
        position: 50.0,
        angle: 60.0,
        locked: true,
        new_at: 10.0,
        relative: true,
        count: 2,
    });
    h.app.guide_props_apply();
    let g = h.app.doc.guides[0];
    assert!(
        g.locked && (g.at - 50.0).abs() < 1e-9 && (g.angle() - 60.0).abs() < 1e-9,
        "{g:?}"
    );
    // two parallels, 10 and 20 to its left
    h.app.guide_props_create();
    assert_eq!(h.app.doc.guides.len(), 5);
    for (k, i) in [(1.0, 3), (2.0, 4)] {
        let made = h.app.doc.guides[i];
        let want = g.parallel(10.0 * k);
        assert!((made.at - want.at).abs() < 1e-9 && (made.angle() - want.angle()).abs() < 1e-9);
        assert!(!made.locked);
    }
    h.app.guide_props_delete();
    assert_eq!(h.app.doc.guides.len(), 4);
    assert!(h.app.guide_props.is_none(), "the window closed");
    // the menu's lock toggles
    assert!(!h.app.doc.guides[0].locked);
    h.app.invoke(&h.ctx, "guide:lock:0");
    assert!(h.app.doc.guides[0].locked);
    let menu = gui::guide_menu(0, true);
    assert_eq!(menu[1].name, "Unlock Guide");
}

#[test]
fn a_component_saved_as_a_sculpting_brush_is_pressed_into_the_model() {
    let dir = std::env::temp_dir().join(format!("cut-craft-brush-app-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("dome.ccclip");
    let mut h = Harness::new();
    h.app.doc = cut_craft_core::testing::domed();
    let id = EntityId(1);
    let action = format!("{}{}", gui::SAVE_BRUSH, id.0);
    let menu = gui::component_menu(&h.app.doc, id, &h.app.doc.components[&id]);
    assert!(
        menu.iter().any(|m| m.action == action),
        "on the component's menu"
    );
    assert!(h.app.available(&action));
    h.app.save_brush_to(id, &path).unwrap();
    let top = |h: &Harness| {
        h.app.doc.components[&id]
            .height_at(Point::new(20.0, 20.0))
            .unwrap()
    };
    let before = top(&h);
    h.app.invoke(&h.ctx, "sculpt");
    let mut params = h.app.form.as_ref().unwrap().params.clone();
    params["tool"] = "deposit".into();
    params["strength"] = 100.0.into();
    params["brush"] = path.display().to_string().into();
    params["points"] = serde_json::json!([{ "x": 20.0, "y": 20.0 }, { "x": 20.0, "y": 20.0 }]);
    h.app.apply("sculpt", &params).unwrap();
    // the dome's top is the brush's highest: pressed in its full depth, a fifth of 10 mm
    assert!(
        (top(&h) - before - 2.0).abs() < 0.2,
        "{} from {before}",
        top(&h)
    );
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn draw_text_lists_words_spelt_wrong_with_suggestions_and_keeps_added_words() {
    let mut h = Harness::new();
    let aff = "TRY esianrtolcdugmphbyfvkwz\nSFX S Y 1\nSFX S 0 s .\n";
    h.app.speller = Some(cut_craft_core::spell::Speller::from_hunspell(
        aff,
        "2\nwood/S\nsign\n",
    ));
    h.app.speller_looked = true;
    h.app.invoke(&h.ctx, "draw_text");
    h.app.form.as_mut().unwrap().params["text"] = "Woood sign Zed".into();
    h.app.screen = Screen::Job;
    h.app.pages.set_pinned(Page::Design, true);
    h.frame(vec![]);
    h.frame(vec![]);
    assert!(h.text_at("Spelling").is_some(), "the spelling section");
    assert!(h.text_at("Woood").is_some(), "the word spelt wrong");
    assert!(h.text_at("Wood").is_some(), "its suggestion");
    h.app.fix_spelling("Woood", "Wood");
    assert_eq!(h.app.form.as_ref().unwrap().params["text"], "Wood sign Zed");
    h.app.user_word("Zed", true);
    let s = h.app.speller.as_ref().unwrap();
    assert_eq!(s.misspelled("Wood sign Zed"), Vec::<String>::new());
    h.app.user_word("Zed", false);
    assert_eq!(h.app.speller.as_ref().unwrap().misspelled("Zed"), ["Zed"]);
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
fn the_toolpath_lists_divider_stays_where_it_is_dragged() {
    let mut h = Harness::new();
    h.app.doc = cut_craft_core::testing::three_toolpaths();
    h.app.screen = Screen::Job;
    h.app.pages.set_pinned(Page::Toolpaths, true);
    h.frame(vec![]);
    h.frame(vec![]);
    let before = h.text_at("Toolpath List").expect("the list's heading");
    // the divider, a little over the heading, dragged up 150
    let from = before - egui::vec2(0.0, 12.0);
    h.frame(vec![Event::PointerMoved(from)]);
    h.press(from, true, Modifiers::NONE);
    for k in 1..=10_u8 {
        h.frame(vec![Event::PointerMoved(
            from - egui::vec2(0.0, 15.0 * f32::from(k)),
        )]);
    }
    h.press(from - egui::vec2(0.0, 150.0), false, Modifiers::NONE);
    for _ in 0..4 {
        h.frame(vec![]);
    }
    let after = h.text_at("Toolpath List").unwrap();
    assert!(
        (after.y - (before.y - 150.0)).abs() < 4.0,
        "dragged up it stays: {before:?} then {after:?}"
    );
}

#[test]
fn edit_passes_shows_each_pass_and_a_click_adds_one() {
    let mut h = Harness::new();
    h.rectangle(10.0, 10.0, 50.0, 50.0);
    h.app.invoke(&h.ctx, "profile_toolpath");
    h.app.form.as_mut().unwrap().params["depth"] = serde_json::json!(6.0);
    h.app.form.as_mut().unwrap().params["tool"]["pass_depth"] = serde_json::json!(3.0);
    h.frame(vec![]);
    h.frame(vec![]);
    let passes = |h: &Harness| {
        let tool = &h.app.form.as_ref().unwrap().params["tool"];
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
    h.app.invoke(&h.ctx, "profile_toolpath");
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
    let params = h.app.form.as_ref().unwrap().params.clone();
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
    assert_eq!(h.app.form.as_ref().unwrap().params["tabs"], "triangular");
    h.click(h.text_at("Delete All Tabs").unwrap(), Modifiers::NONE);
    assert_eq!(
        h.app.form.as_ref().unwrap().params["tab_points"],
        serde_json::json!([])
    );
}

#[test]
fn the_cutting_window_sets_how_a_pocket_cuts_and_draws_it() {
    let mut h = Harness::new();
    h.rectangle(10.0, 10.0, 50.0, 50.0);
    h.app.invoke(&h.ctx, "pocket_toolpath");
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
    let params = |h: &Harness| h.app.form.as_ref().unwrap().params.clone();
    for (choice, key, value) in [
        ("Both directions", "ways", "both"),
        ("From the outline in", "rings", "outline_in"),
        ("Trochoidal", "motion", "trochoidal"),
    ] {
        h.click(h.text_at(choice).expect(choice), Modifiers::NONE);
        h.frame(vec![]);
        assert_eq!(params(&h)[key], value, "{choice}");
    }
    // trochoidal: the loops' settings appear, their turn set apart
    h.frame(vec![]);
    h.click(h.text_at("Clockwise").expect("the loops"), Modifiers::NONE);
    h.frame(vec![]);
    assert_eq!(params(&h)["loops"]["turn"], "clockwise");
    assert!(
        h.text_at("In loops turning clockwise: each loop cuts conventional")
            .is_some()
    );
}

/// The middles of the corner arrows the last frame drew on flyout buttons.
fn flyout_arrows(h: &Harness) -> Vec<Pos2> {
    h.shapes
        .iter()
        .filter_map(|clipped| match &clipped.shape {
            egui::Shape::Path(p) if p.points.len() == 3 && p.closed => Some(
                p.points
                    .iter()
                    .fold(Pos2::ZERO, |sum, q| sum + q.to_vec2() / 3.0),
            ),
            _ => None,
        })
        .collect()
}

/// The corner arrow of the flyout button whose tip names `tool` (its face or one it holds).
fn flyout_arrow_naming(h: &mut Harness, tool: &str) -> Pos2 {
    for arrow in flyout_arrows(h) {
        let over = arrow - vec2(12.0, 12.0);
        h.frame(vec![Event::PointerMoved(over)]);
        h.frame(vec![]);
        if guide::hovered(&h.ctx).is_some_and(|t| t.about.contains(tool)) {
            return arrow;
        }
    }
    panic!("no flyout button names {tool}");
}

#[test]
fn every_tool_shows_in_a_labelled_box_unless_groups_are_folded() {
    let mut h = Harness::new();
    h.app.pages.set_pinned(Page::Design, true);
    h.frame(vec![]);
    h.frame(vec![]);
    // spread out by default: no button hides others, each group of ways captioned
    let spread = flyout_arrows(&h).len();
    for caption in ["Lines and Curves", "Circles and Ellipses", "Text"] {
        assert!(h.text_at(caption).is_some(), "{caption} box");
    }
    // the box's buttons run their own tool: Draw Circle from Edge is the second
    let circles = h.text_at("Circles and Ellipses").expect("caption");
    let mut found = None;
    for step in 0..12_u8 {
        let at = circles + vec2(-50.0 + 10.0 * f32::from(step), 30.0);
        h.frame(vec![Event::PointerMoved(at)]);
        h.frame(vec![]);
        if guide::hovered(&h.ctx).is_some_and(|t| t.label == "Draw Circle from Edge") {
            found = Some(at);
            break;
        }
    }
    let at = found.expect("Draw Circle from Edge has a button of its own");
    h.click(at, Modifiers::NONE);
    assert_eq!(
        h.app.form.as_ref().map(|f| (f.id, f.way)),
        Some(("draw_circle", crate::flyout::DragWay::AcrossEdge))
    );
    // a tool's right-click menu folds the groups back into flyouts, kept in the options
    let fold = gui::tool_menu(Page::Design, "draw_circle", false, false, false)
        .into_iter()
        .find(|m| m.name == "Fold Tool Groups into One Button")
        .expect("the menu offers folding");
    h.app.form = None;
    h.app.invoke(&h.ctx, &fold.action);
    assert!(h.app.options.general.fold_tool_groups);
    h.frame(vec![]);
    h.frame(vec![]);
    // Create's five flyouts (lines, rectangles, circles, polygons, text) and Zoom
    assert!(
        flyout_arrows(&h).len() >= spread + 6,
        "folded into flyouts: {} arrows, {spread} spread out",
        flyout_arrows(&h).len()
    );
    assert!(h.text_at("Circles and Ellipses").is_none());
    let unfold = gui::tool_menu(Page::Design, "draw_circle", false, false, true)
        .into_iter()
        .find(|m| m.name == "Fold Tool Groups into One Button")
        .expect("still offered");
    assert!(unfold.is_checked);
    h.app.invoke(&h.ctx, &unfold.action);
    assert!(!h.app.options.general.fold_tool_groups);
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

#[test]
fn draw_circle_from_edge_takes_the_drag_as_the_diameter() {
    let mut h = Harness::new();
    h.app.invoke(&h.ctx, "draw_circle_from_edge");
    h.frame(vec![]);
    assert_eq!(h.app.status.current_tool, "Draw Circle from Edge");
    h.drag(h.at(20.0, 50.0), h.at(60.0, 50.0));
    let b = h.app.doc.vectors.values().last().expect("drawn").bounds();
    assert!(
        (b.min.x - 20.0).abs() < 0.5 && (b.max.x - 60.0).abs() < 0.5,
        "{b:?}"
    );
    assert!(
        (b.min.y - 30.0).abs() < 0.5 && (b.max.y - 70.0).abs() < 0.5,
        "{b:?}"
    );
}

#[test]
fn draw_rectangle_from_center_drags_from_the_middle_out() {
    let mut h = Harness::new();
    h.app.invoke(&h.ctx, "draw_rectangle_from_center");
    h.frame(vec![]);
    h.drag(h.at(50.0, 50.0), h.at(70.0, 60.0));
    let b = h.app.doc.vectors.values().last().expect("drawn").bounds();
    assert!(
        (b.min.x - 30.0).abs() < 0.5 && (b.max.x - 70.0).abs() < 0.5,
        "{b:?}"
    );
    assert!(
        (b.min.y - 40.0).abs() < 0.5 && (b.max.y - 60.0).abs() < 0.5,
        "{b:?}"
    );
}

#[test]
fn favourites_list_each_tool_on_its_own_even_from_one_flyout() {
    let mut h = Harness::new();
    h.app.pages.set_pinned(Page::Design, true);
    h.app.options.favourites.design = vec!["draw_circle".into(), "draw_ellipse".into()];
    h.app.options.favourites.design_only = true;
    h.frame(vec![]);
    h.frame(vec![]);
    assert!(h.text_at("Favourites").is_some());
    for arrow in flyout_arrows(&h) {
        h.frame(vec![Event::PointerMoved(arrow - vec2(12.0, 12.0))]);
        h.frame(vec![]);
        let hint = cut_craft_bug_report::hint::current(&h.ctx).unwrap_or_default();
        assert!(
            !hint.contains("corner arrow"),
            "a flyout among favourites: {hint}"
        );
    }
}

#[test]
fn the_gadgets_menu_runs_a_lua_gadget_as_one_undo_step() {
    let mut h = Harness::new();
    let gadgets = &h
        .app
        .menus
        .menus
        .iter()
        .find(|m| m.name == "Gadgets")
        .unwrap()
        .items;
    let hole_pattern = leaf(gadgets, &["Hole Pattern"]).action.clone();
    assert!(gadgets.iter().any(|i| i.name == "My Gadgets"));
    assert!(h.app.available(&hole_pattern));
    h.app.invoke(&h.ctx, &hole_pattern);
    let mut params = h
        .app
        .form
        .as_ref()
        .expect("its settings form")
        .params
        .clone();
    assert_eq!(params["count"], 6, "the gadget's own default");
    params["count"] = serde_json::json!(8);
    let before = h.app.doc.vectors.len();
    h.app.apply_form(&hole_pattern, &params).unwrap();
    assert_eq!(h.app.doc.vectors.len(), before + 8);
    h.app.form = None;
    h.app.invoke(&h.ctx, "undo");
    assert_eq!(h.app.doc.vectors.len(), before, "one Undo takes all 8 away");
}

#[test]
fn an_unsigned_gadget_warns_once_before_it_runs() {
    let mut h = Harness::new();
    h.app.unsigned_gadgets = [("gadget_hole_pattern".to_owned(), "Hole Pattern".to_owned())].into();
    // the user says no: nothing opens, and they are asked again next time
    h.app.unsigned_gadget_ok = |_| false;
    h.app.invoke(&h.ctx, "gadget_hole_pattern");
    assert!(h.app.form.is_none(), "cancelled: no form");
    // the user says yes: the form opens, and the gadget is not asked about again this session
    h.app.unsigned_gadget_ok = |label| label == "Hole Pattern";
    h.app.invoke(&h.ctx, "gadget_hole_pattern");
    assert!(h.app.form.is_some());
    h.app.form = None;
    h.app.unsigned_gadget_ok = |_| panic!("asked twice");
    h.app.invoke(&h.ctx, "gadget_hole_pattern");
    assert!(h.app.form.is_some());
}

#[test]
fn built_in_gadgets_need_no_warning() {
    let mut h = Harness::new();
    h.app.unsigned_gadget_ok = |_| panic!("a built-in gadget is signed with the app");
    h.app.invoke(&h.ctx, "gadget_drill_grid");
    assert!(h.app.form.is_some());
}

#[test]
fn the_unsigned_warning_says_who_is_responsible() {
    let text = super::actions::unsigned_warning("Star Burst");
    assert!(text.contains("Star Burst"));
    assert!(text.contains("not responsible"));
    assert!(text.contains("Undo"));
}

/// The harness on a Mac, as egui sees the system.
fn on_a_mac() -> Harness {
    let mut h = Harness::new();
    h.ctx.set_os(egui::os::OperatingSystem::Mac);
    h.frame(vec![]);
    h.frame(vec![]);
    h
}

#[test]
fn on_a_mac_the_menus_leave_room_for_the_traffic_lights() {
    let h = Harness::new();
    let file = h.text_at("File").expect("the File menu");
    assert!(
        file.x < super::mac::TRAFFIC_LIGHTS,
        "Windows and Linux: at the left"
    );
    let mut mac = on_a_mac();
    let file = mac.text_at("File").expect("the File menu");
    assert!(
        file.x > super::mac::TRAFFIC_LIGHTS,
        "after the window's buttons"
    );
    let title = mac.app.title.clone();
    let at = mac.text_at(&title).expect("the title in the menus' row");
    assert!((at.y - file.y).abs() < 1.0);
    // with the desktop's title bar, the menus start at the left again
    mac.app.title_bar_at_start = true;
    mac.frame(vec![]);
    let file = mac.text_at("File").expect("the File menu");
    assert!(file.x < super::mac::TRAFFIC_LIGHTS);
}

#[test]
fn on_a_mac_the_delete_key_deletes_the_selection() {
    for mac in [false, true] {
        let mut h = if mac { on_a_mac() } else { Harness::new() };
        h.rectangle(10.0, 10.0, 20.0, 10.0);
        let before = h.app.doc.vectors.len();
        assert!(!h.app.doc.selection.is_empty(), "the rectangle is selected");
        h.frame(vec![key(Key::Backspace)]);
        h.frame(vec![]);
        let after = h.app.doc.vectors.len();
        if mac {
            assert!(after < before, "the Mac's delete key is egui's Backspace");
        } else {
            assert_eq!(after, before, "Backspace isn't Delete on Windows or Linux");
        }
    }
}

#[test]
fn on_a_mac_menus_show_shortcuts_with_the_command_key() {
    for mac in [false, true] {
        let mut h = if mac { on_a_mac() } else { Harness::new() };
        let edit = h.text_at("Edit").expect("the Edit menu");
        h.click(edit, Modifiers::NONE);
        h.frame(vec![]);
        let (shown, hidden) = if mac {
            ("⌘Z", "Ctrl+Z")
        } else {
            ("Ctrl+Z", "⌘Z")
        };
        assert!(h.text_at(shown).is_some(), "{shown}");
        assert!(h.text_at(hidden).is_none(), "{hidden}");
    }
}

#[test]
fn toolpath_forms_save_their_settings_and_load_them_back() {
    let mut h = Harness::new();
    let dir = std::env::temp_dir().join(format!("cut-craft-form-settings-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("profile.ccsettings");
    h.app.invoke(&h.ctx, "profile_toolpath");
    let form = h.app.form.as_mut().expect("open");
    let key = form
        .params
        .as_object()
        .unwrap()
        .iter()
        .find(|(_, v)| v.is_f64())
        .map(|(k, _)| k.clone())
        .expect("a number setting");
    form.params[&key] = serde_json::json!(7.25);
    h.app.save_form_settings(&path);
    assert!(h.app.status.message.starts_with("Saved the settings"));

    h.app.form.as_mut().unwrap().params[&key] = serde_json::json!(1.0);
    h.app.load_form_settings(&path);
    let form = h.app.form.as_ref().unwrap();
    assert_eq!(form.params[&key], serde_json::json!(7.25));
    assert!(form.dirty, "the loaded settings are yet to be applied");

    h.app.form = None;
    h.app.invoke(&h.ctx, "pocket_toolpath");
    let before = h.app.form.as_ref().unwrap().params.clone();
    h.app.load_form_settings(&path);
    assert_eq!(h.app.form.as_ref().unwrap().params, before, "left alone");
    assert!(
        h.app.status.message.starts_with("Could not load"),
        "{}",
        h.app.status.message
    );
    std::fs::remove_dir_all(&dir).ok();
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

#[test]
fn object_properties_opens_the_form_for_what_is_selected() {
    let mut h = Harness::new();
    assert!(!h.app.available("object_properties"), "nothing selected");
    h.rectangle(10.0, 10.0, 20.0, 10.0);
    assert!(!h.app.doc.selection.is_empty());
    assert!(h.app.available("object_properties"));
    h.app.invoke(&h.ctx, "object_properties");
    let form = h.app.form.as_ref().expect("a form opens").id;
    assert!(
        form == "draw_rectangle" || form == "set_size",
        "the rectangle's own tool, or its position and size: {form}"
    );
}

#[test]
fn delete_layer_is_on_the_layer_menu_once_there_is_another_layer() {
    let mut h = Harness::new();
    let layer = |h: &Harness, i: usize| h.app.doc.layers[i].clone();
    let only = gui::layer_menu(&h.app.doc, &layer(&h, 0));
    let item = only
        .iter()
        .find(|i| i.name == "Delete Layer")
        .expect("listed");
    assert!(!item.is_enabled, "the last layer stays");
    h.app
        .apply("add_layer", &serde_json::json!({ "name": "Top" }))
        .unwrap();
    let top = layer(&h, h.app.doc.layers.len() - 1);
    let menu = gui::layer_menu(&h.app.doc, &top);
    let item = menu.iter().find(|i| i.name == "Delete Layer").unwrap();
    assert!(item.is_enabled);
    let action = item.action.clone();
    h.app.invoke(&h.ctx, &action);
    assert!(h.app.doc.layers.iter().all(|l| l.id != top.id), "deleted");
}

#[test]
fn import_toolpaths_opens_its_form_on_the_file_and_adds_its_toolpaths() {
    let mut h = Harness::new();
    let dir = std::env::temp_dir().join(format!("cut-craft-import-app-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let file = dir.join("nameplate.nc");
    std::fs::write(&file, "G21 G90\nG0 X10 Y10 Z5\nG1 Z-1 F600\nX40\n").unwrap();
    h.app.import_toolpaths_from(&file);
    let form = h.app.form.as_ref().expect("its form");
    assert_eq!(form.id, "import_toolpaths");
    assert_eq!(form.params["name"], "nameplate");
    let params = form.params.clone();
    h.app.apply("import_toolpaths", &params).unwrap();
    let toolpath = h.app.doc.toolpaths.values().next().expect("added");
    assert_eq!(toolpath.name, "nameplate");
    assert_eq!(toolpath.tool.feed_rate, 600.0, "the file's feed rate");
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn update_post_processor_database_says_what_it_found() {
    let entries = h_posts();
    let message = super::actions::post_database_message(&entries);
    assert!(
        message.contains(&format!("{} post-processors", entries.len())),
        "{message}"
    );
    assert!(message.contains("newer Cut Craft"), "{message}");
}

fn h_posts() -> Vec<cut_craft_machines::Entry> {
    cut_craft_machines::Library::new(None).entries()
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
    assert!((setup.rapid - h.app.summary.rapid).abs() < 1e-9);
    assert!(h.app.available("print_job_sheet"));
    let _ = std::fs::remove_dir_all(&dir);
}

/// Frames until the slow job running now is done and its answer used.
fn until_not_busy(h: &mut Harness) {
    for _ in 0..2000 {
        if h.app.busy.is_none() {
            return;
        }
        std::thread::sleep(std::time::Duration::from_millis(5));
        h.frame(vec![]);
    }
    panic!("the job never finished");
}

#[test]
fn a_toolpath_that_takes_a_while_shows_its_progress_then_lands_in_the_job() {
    let mut h = Harness::new();
    h.app.wait_before_showing = std::time::Duration::ZERO;
    h.rectangle(10.0, 10.0, 50.0, 50.0);
    h.app.invoke(&h.ctx, "profile_toolpath");
    let params = h.app.form.as_ref().unwrap().params.clone();
    h.app.apply_form("profile_toolpath", &params).unwrap();
    until_not_busy(&mut h);
    assert_eq!(h.app.doc.toolpaths.len(), 1, "made once the job was done");
    assert!(h.app.history.can_undo(), "as one undo, like any other");
}

#[test]
fn a_slow_job_shows_a_progress_window_that_cancels_with_nothing_changed() {
    let mut h = Harness::new();
    h.app.wait_before_showing = std::time::Duration::ZERO;
    h.rectangle(10.0, 10.0, 50.0, 50.0);
    let (go, wait) = std::sync::mpsc::channel::<()>();
    let (reported, said) = std::sync::mpsc::channel::<()>();
    let before = h.app.history.revision();
    h.app.in_background(
        "Calculating Slow Toolpath",
        move || {
            cut_craft_core::progress::report(0.4);
            reported.send(()).ok();
            wait.recv().ok();
        },
        |app, ()| app.status.set_message("used"),
    );
    assert!(h.app.busy.is_some(), "still running");
    // the job's thread may start late (Windows): the bar is drawn once it has reported
    said.recv().unwrap();
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
    assert_eq!(
        h.app.history.revision(),
        before,
        "no undo behind the window"
    );
    let cancel = h.text_at("Cancel").expect("a Cancel button");
    h.click(cancel, Modifiers::NONE);
    h.frame(vec![]);
    assert!(h.app.busy.is_none(), "the window is gone at once");
    assert!(
        h.app.status.message.contains("cancelled"),
        "{}",
        h.app.status.message
    );
    go.send(()).ok();
    h.frame(vec![]);
    assert_ne!(h.app.status.message, "used", "what it made was let go");
}

#[test]
fn what_a_slow_job_made_is_not_used_if_the_job_changed_meanwhile() {
    let mut h = Harness::new();
    h.app.wait_before_showing = std::time::Duration::ZERO;
    h.rectangle(10.0, 10.0, 50.0, 50.0);
    let (go, wait) = std::sync::mpsc::channel::<()>();
    h.app.in_background(
        "Calculating",
        move || {
            wait.recv().ok();
        },
        |app, ()| app.status.set_message("used"),
    );
    h.app.invoke(&h.ctx, "undo");
    go.send(()).unwrap();
    until_not_busy(&mut h);
    assert!(
        h.app.status.message.contains("was not used"),
        "{}",
        h.app.status.message
    );
}

#[test]
fn a_preview_cut_off_the_windows_thread_becomes_the_preview() {
    let mut h = Harness::new();
    h.app.wait_before_showing = std::time::Duration::ZERO;
    h.rectangle(10.0, 10.0, 50.0, 50.0);
    h.app.invoke(&h.ctx, "profile_toolpath");
    let params = h.app.form.as_ref().unwrap().params.clone();
    h.app.apply_form("profile_toolpath", &params).unwrap();
    until_not_busy(&mut h);
    h.app.preview_all(false);
    until_not_busy(&mut h);
    let cut = h.app.simulation.as_ref().expect("a preview");
    assert!(cut.heights.iter().any(|&z| z < 0.0), "the profile is cut");
    assert!(h.app.status.message.starts_with("Previewed"));
}

#[test]
fn a_double_sided_preview_cuts_the_other_side_off_the_windows_thread_too() {
    let mut h = Harness::new();
    h.app.doc.job.double_sided = Some(cut_craft_core::document::DoubleSided::default());
    let pocket = |h: &mut Harness| {
        h.rectangle(10.0, 10.0, 40.0, 40.0);
        h.app.invoke(&h.ctx, "pocket_toolpath");
        let params = h.app.form.as_ref().unwrap().params.clone();
        h.app.apply("pocket_toolpath", &params).unwrap();
        h.app.form = None;
    };
    pocket(&mut h);
    h.app.apply("swap_sides", &serde_json::json!({})).unwrap();
    pocket(&mut h);
    h.app.apply("swap_sides", &serde_json::json!({})).unwrap();
    h.app.wait_before_showing = std::time::Duration::ZERO;
    h.app.invoke(&h.ctx, "preview_all_toolpaths");
    until_not_busy(&mut h);
    let preview = h.app.simulation.as_ref().expect("previewed");
    assert!(
        preview.other_side.is_some(),
        "the other side is previewed too"
    );
}

#[test]
fn save_toolpaths_refuses_toolpaths_bigger_than_the_machine_and_says_how_to_fix_it() {
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
        refused.starts_with("Not saved:") && refused.contains("more than the bed of Desktop"),
        "{refused}"
    );
    assert!(!path.exists(), "nothing written");
}

/// Two profile toolpaths, First with a 6 mm end mill and Second with a 3 mm one.
fn two_tools(h: &mut Harness) {
    for (x, name, diameter) in [(10.0, "First", 6.0), (80.0, "Second", 3.0)] {
        h.rectangle(x, 10.0, 50.0, 50.0);
        h.app.invoke(&h.ctx, "profile_toolpath");
        h.frame(vec![]);
        let mut params = h.app.form.as_ref().unwrap().params.clone();
        params["name"] = name.into();
        params["tool"]["diameter"] = diameter.into();
        params["tool"]["name"] = format!("{diameter} mm End Mill").into();
        h.app.apply("profile_toolpath", &params).unwrap();
        h.app.form = None;
    }
}

#[test]
fn a_post_that_cannot_change_tools_saves_a_file_for_each_tool_named_after_it() {
    let mut h = Harness::new();
    two_tools(&mut h);
    let plain = cut_craft_post::Post::parse(
        "POST_NAME = \"Plain (*.nc)\"\nFILE_EXTENSION = \"nc\"\nUNITS = \"MM\"\n\
         begin HEADER\n\"G21\"\nbegin RAPID_MOVE\n\"G0 [X] [Y] [Z]\"\n\
         begin FEED_MOVE\n\"G1 [X] [Y] [Z]\"\n",
    )
    .unwrap();
    let form = SaveForm {
        mode: crate::files::SaveMode::VisibleToOneFile,
        group: false,
        post: None,
        add_side: false,
        changes: crate::files::ToolChanges::ToolChanger,
        job_sheet: true,
    };
    let dir = std::env::temp_dir().join(format!("cut-craft-per-tool-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let done = h
        .app
        .save_toolpaths_to(&form, &plain, &dir.join("sign.nc"))
        .unwrap();
    assert!(done.starts_with("Saved 2 toolpath files"), "{done}");
    for name in ["sign_1_6_mm_End_Mill.nc", "sign_2_3_mm_End_Mill.nc"] {
        assert!(dir.join(name).exists(), "{name}");
    }
    let _ = std::fs::remove_dir_all(dir);
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
        false
    };
    let kept = h.app.send_to(&form, &post, &store, &no).unwrap_err();
    assert!(kept.contains("were kept"), "{kept}");
    // the project keeps its folder; another with the same name gets one of its own
    assert_eq!(
        h.app.machines.active().unwrap().send_to.projects["/jobs/Oak sign: front.ccjob"],
        "Oak sign_ front"
    );
    h.app.session.path = Some(std::path::PathBuf::from("/old/Oak sign_ front.ccjob"));
    let yes = |_: &[String]| true;
    let done = h.app.send_to(&form, &post, &store, &yes).unwrap();
    assert!(done.ends_with("in its folder Oak sign_ front 2"), "{done}");
    let _ = std::fs::remove_dir_all(card);
}

#[test]
fn nothing_is_sent_while_the_machine_runs_a_job() {
    use cut_craft_machines::send::{Folder, Item, Store};
    /// A card on a machine that is cutting.
    struct Cutting(Folder);
    impl Store for Cutting {
        fn place(&self) -> String {
            self.0.place()
        }
        fn busy(&self) -> Option<String> {
            Some("The machine is running a job".into())
        }
        fn list(&self, at: &[String]) -> Result<Vec<Item>, String> {
            self.0.list(at)
        }
        fn upload(&self, at: &[String], name: &str, bytes: &[u8]) -> Result<(), String> {
            self.0.upload(at, name, bytes)
        }
        fn exists(&self, at: &[String], name: &str) -> Result<bool, String> {
            self.0.exists(at, name)
        }
        fn rename(&self, at: &[String], from: &str, to: &str) -> Result<(), String> {
            self.0.rename(at, from, to)
        }
        fn delete(&self, at: &[String], name: &str) -> Result<(), String> {
            self.0.delete(at, name)
        }
        fn make_folder(&self, at: &[String], name: &str) -> Result<(), String> {
            self.0.make_folder(at, name)
        }
    }
    let mut h = Harness::new();
    two_tools(&mut h);
    let card = std::env::temp_dir().join(format!("cut-craft-cutting-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&card);
    std::fs::create_dir_all(card.join("Sign")).unwrap();
    h.app.machines.add();
    h.app.machines.active_mut().unwrap().send_to.folder = card.display().to_string();
    let form = SaveForm {
        mode: crate::files::SaveMode::VisibleToOneFile,
        group: false,
        post: None,
        add_side: false,
        changes: crate::files::ToolChanges::ToolChanger,
        job_sheet: false,
    };
    let post = h.app.form_post(&form).unwrap();
    let store = Cutting(Folder::new(&card));
    let never = |_: &[String]| panic!("not asked");
    let refused = h.app.send_to(&form, &post, &store, &never).unwrap_err();
    assert!(
        refused.starts_with("Not sent: The machine is running a job"),
        "{refused}"
    );
    let mut files =
        super::send::MachineFiles::new("Router", Box::new(Cutting(Folder::new(&card))), false);
    files.delete("Sign");
    assert!(card.join("Sign").is_dir(), "kept");
    assert_eq!(
        files.message,
        Err("The machine is running a job".to_owned())
    );
    let _ = std::fs::remove_dir_all(card);
}

#[test]
fn machine_files_uploads_renames_makes_folders_and_deletes_only_on_the_second_ask() {
    let card = std::env::temp_dir().join(format!("cut-craft-files-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&card);
    std::fs::create_dir_all(card.join("Sign")).unwrap();
    let local = card.with_extension("local.nc");
    std::fs::write(&local, "G0 X0").unwrap();
    let store = Box::new(cut_craft_machines::send::Folder::new(&card));
    let mut files = super::send::MachineFiles::new("Router", store, false);
    files.open_folder("Sign");
    files.upload(std::slice::from_ref(&local));
    assert_eq!(files.message, Ok("Uploaded 1 file".to_owned()));
    let uploaded = crate::app::file_name(&local);
    files.rename(&uploaded, "Front: v2.nc");
    assert!(
        card.join("Sign/Front_ v2.nc").exists(),
        "{:?}",
        files.message
    );
    files.make_folder("Old jobs");
    assert!(card.join("Sign/Old jobs").is_dir());
    files.delete("Old jobs");
    let names: Vec<String> = files
        .items
        .clone()
        .unwrap()
        .into_iter()
        .map(|i| i.name)
        .collect();
    assert_eq!(names, ["Front_ v2.nc"]);
    let _ = std::fs::remove_dir_all(card);
    let _ = std::fs::remove_file(local);
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
fn a_machine_without_a_files_folder_says_where_to_set_it() {
    let mut h = Harness::new();
    h.app.machines.add();
    h.app.invoke(&h.ctx, "machine_files");
    assert!(h.app.machine_files.is_none());
    assert_eq!(h.app.status.message, super::send::NO_MACHINE_FOLDER);
}

#[test]
fn save_toolpaths_shows_what_must_be_fixed_and_will_not_save_until_it_is() {
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
    assert!(shown.contains("Must fix: The toolpaths cover"), "{shown}");
    let checked = h.app.save_checks.as_ref().unwrap().revision;
    h.frame(vec![]);
    assert_eq!(
        h.app.save_checks.as_ref().unwrap().revision,
        checked,
        "the checks are kept between frames"
    );
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
fn gadgets_ends_with_add_ons_which_opens_its_window() {
    let mut h = Harness::new();
    let gadgets = &h
        .app
        .menus
        .menus
        .iter()
        .find(|m| m.name == "Gadgets")
        .unwrap()
        .items;
    let last = gadgets.last().unwrap();
    assert_eq!(
        (last.name.as_str(), last.action.as_str()),
        ("Add-ons...", "addons")
    );
    assert!(h.app.available("addons"));
    h.app.invoke(&h.ctx, "addons");
    assert!(h.app.addons.open);
}

/// The screen boxes of the pictures the last frame drew in full (models; one being dragged
/// is also drawn faint where it was).
fn textured_meshes(h: &Harness) -> Vec<Rect> {
    h.shapes
        .iter()
        .filter_map(|c| match &c.shape {
            egui::Shape::Mesh(m)
                if m.texture_id != egui::TextureId::default()
                    && m.vertices.iter().all(|v| v.color == egui::Color32::WHITE) =>
            {
                let mut r = Rect::NOTHING;
                for v in &m.vertices {
                    r.extend_with(v.pos);
                }
                Some(r)
            }
            _ => None,
        })
        .collect()
}

/// A 60 × 40 mm model at (100, 100) on a new job, picked and drawn.
fn picked_model(h: &mut Harness) -> cut_craft_core::geometry::Rect {
    use cut_craft_core::component::{Combine, Component};
    let o = h.app.doc.job.area().min;
    let rect = cut_craft_core::geometry::Rect::new(
        o + Point::new(100.0, 100.0),
        o + Point::new(160.0, 140.0),
    );
    let mut c = Change::builder(&h.app.doc);
    let id = c.add_component(Component::from_fn("model", Combine::Add, rect, 1.0, |p| {
        Some(1.0 + (p.x - rect.min.x) / 10.0)
    }));
    h.app.record(Ok(c.finish()), "added");
    h.app.doc.selected_components = [id].into();
    h.frame(vec![]);
    rect
}

#[test]
fn a_model_follows_a_2d_handle_drag_as_it_goes_and_changes_only_when_let_go() {
    let mut h = Harness::new();
    let rect = picked_model(&mut h);
    let o = h.app.doc.job.area().min;
    let at = |h: &Harness, p: Point| h.at(p.x - o.x, p.y - o.y);
    let (corner, far) = (at(&h, rect.max), at(&h, rect.max + Point::new(30.0, 20.0)));
    let before = h.app.doc.components.values().next().unwrap().extent();
    h.frame(vec![Event::PointerMoved(corner)]);
    h.press(corner, true, Modifiers::NONE);
    h.frame(vec![Event::PointerMoved(corner.lerp(far, 0.5))]);
    h.frame(vec![Event::PointerMoved(far)]);
    // mid-drag: the model's picture is drawn stretched to the pointer, the model unchanged
    let drawn = textured_meshes(&h);
    assert_eq!(drawn.len(), 1, "the model drawn as it will be: {drawn:?}");
    assert!(
        (drawn[0].right() - far.x).abs() < 2.0 && (drawn[0].top() - far.y).abs() < 2.0,
        "{drawn:?} reaches the pointer at {far:?}"
    );
    assert_eq!(
        h.app.doc.components.values().next().unwrap().extent(),
        before,
        "nothing changes until let go"
    );
    h.press(far, false, Modifiers::NONE);
    h.frame(vec![]);
    let drawn = textured_meshes(&h);
    assert_eq!(drawn.len(), 1, "drawn once, where it now is: {drawn:?}");
    let after = h.app.doc.components.values().next().unwrap().extent();
    assert!(
        (after.max - (rect.max + Point::new(30.0, 20.0))).length() < 1.5,
        "{after:?}"
    );
}

#[test]
fn a_model_dragged_in_the_2d_view_is_drawn_where_it_is_going() {
    let mut h = Harness::new();
    let rect = picked_model(&mut h);
    let o = h.app.doc.job.area().min;
    let at = |h: &Harness, p: Point| h.at(p.x - o.x, p.y - o.y);
    let (from, to) = (
        at(&h, rect.center()),
        at(&h, rect.center() + Point::new(40.0, 0.0)),
    );
    h.frame(vec![Event::PointerMoved(from)]);
    h.press(from, true, Modifiers::NONE);
    h.frame(vec![Event::PointerMoved(from.lerp(to, 0.5))]);
    h.frame(vec![Event::PointerMoved(to)]);
    let drawn = textured_meshes(&h);
    assert_eq!(drawn.len(), 1, "{drawn:?}");
    assert!(
        (drawn[0].center().x - to.x).abs() < 2.0,
        "{drawn:?} at {to:?}"
    );
    h.press(to, false, Modifiers::NONE);
    h.frame(vec![]);
}

#[test]
fn editing_a_model_again_and_again_keeps_one_picture_of_it() {
    let mut h = Harness::new();
    picked_model(&mut h);
    let textures = |h: &Harness| h.ctx.tex_manager().read().num_allocated();
    let before = textures(&h);
    for _ in 0..4 {
        let change = Change::transform_selected(&h.app.doc, |_| {
            cut_craft_core::glam::DAffine2::from_scale(cut_craft_core::glam::DVec2::splat(1.1))
        });
        h.app.record(change, "scaled");
        h.frame(vec![]);
    }
    assert_eq!(textures(&h), before, "the old pictures are let go");
}

#[test]
fn the_3d_views_quick_surface_is_kept_until_the_model_changes() {
    let mut h = Harness::new();
    h.app.doc = cut_craft_core::testing::domed();
    h.app.view = View::ThreeD;
    h.frame(vec![]);
    h.frame(vec![]);
    let kept = h.app.rough_3d.as_ref().map(|(key, _)| *key);
    assert!(kept.is_some(), "made once");
    h.frame(vec![]);
    assert_eq!(h.app.rough_3d.as_ref().map(|(key, _)| *key), kept, "kept");
    h.app.doc.selected_components = [EntityId(1)].into();
    let change = Change::transform_selected(&h.app.doc, |_| {
        cut_craft_core::glam::DAffine2::from_translation(cut_craft_core::glam::DVec2::X)
    });
    h.app.record(change, "moved");
    h.frame(vec![]);
    assert_ne!(
        h.app.rough_3d.as_ref().map(|(key, _)| *key),
        kept,
        "made again"
    );
}

#[test]
fn the_detail_while_moving_is_two_steps_below_the_detail_when_still_unless_picked() {
    use crate::options::{Detail, MovingDetail};
    let mut v = crate::options::Options::default().view_3d;
    assert_eq!(v.still_detail, Detail::VeryHigh);
    assert_eq!(v.moving_detail, MovingDetail::TwoStepsLower);
    assert_eq!(
        v.moving_cells(),
        Detail::Medium.cells(),
        "Very High drops to Medium"
    );
    assert_eq!(v.still_cells(), Detail::VeryHigh.cells());
    v.still_detail = Detail::Medium;
    assert_eq!(v.moving_cells(), Detail::Low.cells(), "never below Low");
    v.moving_detail = MovingDetail::High;
    assert_eq!(
        v.moving_cells(),
        Detail::Medium.cells(),
        "never finer while moving than when still"
    );
    v.still_detail = Detail::VeryHigh;
    assert_eq!(v.moving_cells(), Detail::High.cells());
    assert!(Detail::Low.cells() < Detail::Medium.cells());
    assert!(Detail::High.cells() < Detail::VeryHigh.cells());
}

#[test]
fn a_height_handle_drag_applies_its_height_at_most_ten_times_a_second() {
    let mut h = Harness::new();
    h.app.doc = cut_craft_core::testing::domed();
    h.app.doc.selected_components = [EntityId(1)].into();
    h.app.orbit = view3d::Orbit::TOP;
    h.app.view = View::ThreeD;
    h.app.invoke(&h.ctx, "transform");
    h.frame(vec![]);
    h.frame(vec![]);
    let from = h
        .app
        .handles_3d
        .iter()
        .find(|(k, _)| *k == Handle3d::Height)
        .map(|(_, at)| *at)
        .expect("the height handle");
    let quarter = h.app.fit_3d.unwrap().height() / VIEW_ZOOM_QUARTERS;
    h.frame(vec![Event::PointerMoved(from)]);
    h.press(from, true, Modifiers::NONE);
    // ten moves a fiftieth of a second apart: a fifth of a second in all
    let mut revisions = std::collections::BTreeSet::new();
    for i in 1..=10u8 {
        h.time -= 0.08;
        let to = from - vec2(0.0, quarter * f32::from(i) / 10.0);
        h.frame(vec![Event::PointerMoved(to)]);
        revisions.insert(h.app.history.revision());
    }
    assert!(
        revisions.len() <= 3,
        "applied {} times in a fifth of a second",
        revisions.len()
    );
    let to = from - vec2(0.0, quarter);
    h.press(to, false, Modifiers::NONE);
    h.frame(vec![]);
    let height = h.app.doc.components[&EntityId(1)].shaping.shape_height;
    assert!(
        (height - 200.0).abs() < 5.0,
        "the last height is applied: {height}"
    );
}

#[test]
fn moving_what_a_toolpath_was_made_from_says_it_is_out_of_date() {
    let mut h = Harness::new();
    h.rectangle(10.0, 10.0, 40.0, 30.0);
    let vector = *h.app.doc.vectors.keys().next().unwrap();
    let mut c = Change::builder(&h.app.doc);
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
    });
    h.app.record(Ok(c.finish()), "made");
    h.frame(vec![]);
    assert!(!h.app.status.message.contains("out of date"));
    h.app.doc.selection = [vector].into();
    h.frame(vec![]);
    let (from, to) = (h.at(30.0, 25.0), h.at(40.0, 25.0));
    h.drag(from, to);
    h.frame(vec![]);
    let message = &h.app.status.message;
    assert!(
        message.contains("1 toolpath is now out of date") && message.contains("G-code"),
        "{message}"
    );
}

#[test]
fn the_surface_drawn_on_is_made_again_when_the_detail_when_still_changes() {
    let mut h = Harness::new();
    h.app.doc = cut_craft_core::testing::domed();
    let first = h.app.drawing_surface().expect("a model to draw on");
    let again = h.app.drawing_surface().unwrap();
    assert!(std::sync::Arc::ptr_eq(&first, &again), "kept");
    h.app.options.view_3d.still_detail = crate::options::Detail::Low;
    let changed = h.app.drawing_surface().unwrap();
    assert!(!std::sync::Arc::ptr_eq(&first, &changed), "made again");
}

/// The status bar's text that contains `part`: its middle and how many rows it was laid in.
fn status_text(h: &Harness, part: &str) -> Option<(Pos2, usize)> {
    h.shapes.iter().find_map(|clipped| match &clipped.shape {
        egui::Shape::Text(t) if t.galley.text().contains(part) && t.pos.y > 700.0 => {
            Some((t.pos + t.galley.size() / 2.0, t.galley.rows.len()))
        }
        _ => None,
    })
}

#[test]
fn the_status_bar_stays_one_line_however_long_its_text() {
    let mut h = Harness::new();
    h.app.status.set_message("Ready");
    h.frame(vec![]);
    h.frame(vec![]);
    let version = concat!("v", env!("CARGO_PKG_VERSION"));
    let short = h.text_at(version).expect("the version");
    // a Library tile's hint: its name, then on a new line what can be done with it
    let long = format!(
        "0331. Flowers.STL\nDouble-click to add it to the project, or drag it onto a view. {}",
        "Click to pick it (Ctrl or Shift for more), right-click for more. ".repeat(6)
    );
    h.app.status.set_message(&long);
    h.frame(vec![]);
    h.frame(vec![]);
    let (_, rows) = status_text(&h, "Flowers.STL").expect("the status text");
    assert_eq!(rows, 1, "one row");
    let after = h.text_at(version).expect("the version");
    assert!(
        (after.y - short.y).abs() < 0.5,
        "the bar keeps its height: {short:?} then {after:?}"
    );
}

#[test]
fn the_status_bar_keeps_its_hint_while_the_pointer_is_on_it() {
    let mut h = Harness::new();
    h.app.status.set_message("Ready");
    h.frame(vec![]);
    let hint = "0331. Flowers.STL\nDouble-click to add it to the project";
    cut_craft_bug_report::hint::hint(&h.ctx, hint);
    h.frame(vec![]);
    let (at, _) = status_text(&h, "Flowers.STL").expect("the hint is shown");
    // moving onto the bar to read it: nothing sets a hint now, yet it stays
    h.frame(vec![Event::PointerMoved(at)]);
    h.frame(vec![]);
    h.frame(vec![]);
    assert!(
        status_text(&h, "Flowers.STL").is_some(),
        "the hint stays while the pointer is on the bar"
    );
    // and goes once the pointer leaves
    h.frame(vec![Event::PointerMoved(egui::pos2(640.0, 300.0))]);
    h.frame(vec![]);
    assert!(status_text(&h, "Flowers.STL").is_none(), "gone after");
}

#[test]
fn the_view_follows_the_frame_rate_by_default_aiming_for_thirty_a_second() {
    let mut v = crate::options::Options::default().view_3d;
    assert!(v.adjust_to_frame_rate);
    assert!((v.fps_wanted() - crate::options::TARGET_FPS).abs() < 1e-9);
    assert!((crate::options::TARGET_FPS - 30.0).abs() < 1e-9);
    assert!(!v.show_frame_rate, "the status bar note is opt-in");
    v.target_fps = 500.0;
    assert!((v.fps_wanted() - 120.0).abs() < 1e-9);
    v.target_fps = 0.0;
    assert!((v.fps_wanted() - 10.0).abs() < 1e-9);
}

/// A harness with a domed model in the 3D view, turned every frame for `frames` frames
/// (the harness's frames are a tenth of a second apart: ten a second, slower than wanted).
fn turned_slowly(adjust: bool, frames: usize) -> Harness {
    let mut h = Harness::new();
    h.app.options.view_3d.adjust_to_frame_rate = adjust;
    h.app.doc = cut_craft_core::testing::domed();
    h.app.view = View::ThreeD;
    for _ in 0..frames {
        h.app.moved_3d = h.time + 0.1;
        h.frame(vec![]);
    }
    h
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
fn with_the_frame_rate_setting_off_moving_keeps_options_own_detail() {
    let h = turned_slowly(false, 20);
    assert_eq!(h.app.moving_cells(), h.app.options.view_3d.moving_cells());
    assert_eq!(h.app.turning_cells(), h.app.options.view_3d.still_cells());
}

#[test]
fn the_status_bar_shows_the_3d_detail_and_frame_rate_when_asked() {
    let mut h = turned_slowly(true, 3);
    assert!(status_text(&h, "3D detail").is_none(), "off unless asked");
    h.app.options.view_3d.show_frame_rate = true;
    for _ in 0..10 {
        h.app.moved_3d = h.time + 0.1;
        h.frame(vec![]);
    }
    let note = h.app.frame_rate_note();
    assert!(note.contains("frames/s"), "{note}");
    assert!(status_text(&h, "3D detail").is_some(), "{note}");
}

#[test]
fn coarser_surfaces_are_made_from_the_fine_one_on_another_thread_and_kept() {
    use cut_craft_core::simulation::Material;
    let job = cut_craft_core::document::Job::default();
    let fine = Material::new(&job, 120);
    let key = super::GeometryKey {
        revision: 1,
        job,
        wrap: None,
        sides: true,
        importing: false,
        cells: 120,
    };
    let mut kept = super::SurfaceGpu::new(key);
    assert_eq!(fine.columns.max(fine.rows), 120);
    kept.levels
        .push(super::level_of(&fine, 120, None, true, true));
    kept.fine = Some(std::sync::Arc::new(fine));
    let ctx = egui::Context::default();
    kept.make(30, true, &ctx);
    kept.make(30, true, &ctx);
    assert!(kept.making.len() <= 1, "made once however often asked");
    assert_eq!(
        kept.nearest(30).map(|l| l.cells),
        Some(120),
        "the fine one meanwhile"
    );
    while !kept.making.is_empty() {
        std::thread::yield_now();
        kept.collect();
    }
    let coarse = kept.at(30).expect("kept once made");
    assert!(coarse.solid.is_some(), "with its walls");
    assert!(coarse.surface.triangles() < kept.at(120).expect("fine").surface.triangles());
}

/// The opening splash shows Matthew's CC logo, which carries the name itself: it is a real,
/// square picture big enough to stay sharp at the splash's size on a large screen.
#[test]
fn splash_logo_is_a_square_picture() {
    let logo = image::load_from_memory(SPLASH_LOGO).expect("the splash logo decodes");
    assert_eq!(
        logo.width(),
        logo.height(),
        "square, so the splash can size it by one edge"
    );
    assert!(logo.width() >= 512, "sharp at a third of a large screen");
}

/// A rectangle selected and the Profile Toolpath form open on it.
fn profile_form() -> Harness {
    let mut h = Harness::new();
    h.rectangle(10.0, 10.0, 50.0, 50.0);
    h.app.invoke(&h.ctx, "profile_toolpath");
    for _ in 0..3 {
        h.frame(vec![]);
    }
    h
}

#[test]
fn the_profile_form_shows_which_side_it_cuts_just_under_the_depth() {
    let mut h = profile_form();
    let y = |h: &Harness, text: &str| h.text_at(text).unwrap_or_else(|| panic!("{text}")).y;
    let side = y(&h, crate::form::WHICH_SIDE);
    assert!(y(&h, "Cut Depth") < side, "under the depth");
    assert!(
        side < y(&h, "Machine Vectors…"),
        "before the cutting details"
    );
    for choice in ["Outside", "Inside", "On the Line"] {
        assert!(
            y(&h, choice) > side,
            "{choice} is a picture under the heading"
        );
    }
    assert!(h.text_at(super::cutting::side_said("outside")).is_some());
    // the picture over Inside picks it, and the sentence says what that makes
    let inside = h.text_at("Inside").unwrap();
    h.click(inside - egui::vec2(0.0, 25.0), Modifiers::NONE);
    h.frame(vec![]);
    assert_eq!(h.app.form.as_ref().unwrap().params["side"], "inside");
    assert!(h.text_at(super::cutting::side_said("inside")).is_some());
}

#[test]
fn a_cut_right_through_with_no_tabs_is_warned_of_with_add_tabs() {
    let mut h = profile_form();
    let thickness = h.app.doc.job.thickness;
    assert!(
        h.text_at("Cut right through").is_none(),
        "no tick: a cut past the material is warned of on Calculate"
    );
    assert!(
        h.text_at(super::tabs::LOOSE).is_none(),
        "6 mm deep holds the part"
    );
    h.app.form.as_mut().unwrap().params["depth"] = thickness.into();
    h.frame(vec![]);
    h.frame(vec![]);
    let params = |h: &Harness| h.app.form.as_ref().unwrap().params.clone();
    let over = h.form_heading("Profile Toolpath") + egui::vec2(0.0, 200.0);
    let add = scroll_above_foot(&mut h, over, "Add Tabs");
    assert!(
        h.text_at(super::tabs::LOOSE).is_some(),
        "right through, no tabs"
    );
    // made like that, the status bar warns too
    h.app.apply("profile_toolpath", &params(&h)).unwrap();
    assert!(
        h.app.status.message.contains("comes loose"),
        "{}",
        h.app.status.message
    );
    // Add Tabs puts tabs on, and the warning goes
    h.click(add, Modifiers::NONE);
    h.frame(vec![]);
    assert_eq!(h.app.form.as_ref().unwrap().params["tabs"], "flat");
    assert!(h.text_at(super::tabs::LOOSE).is_none());
}

#[test]
fn a_toolpath_made_with_cut_right_through_reads_as_the_materials_thickness() {
    let mut h = profile_form();
    let thickness = h.app.doc.job.thickness;
    h.app.form.as_mut().unwrap().params["through"] = true.into();
    h.frame(vec![]);
    h.frame(vec![]);
    let params = &h.app.form.as_ref().unwrap().params;
    assert_eq!(params["depth"].as_f64(), Some(thickness));
    assert_eq!(params["through"], false, "an ordinary depth to change");
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
        let apply = h.text_at("Apply").or_else(|| h.text_at("Calculate"));
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
fn a_long_forms_apply_button_and_error_stay_in_sight() {
    let mut h = profile_form();
    let apply = h.text_at("Apply").expect("Apply");
    assert!(apply.y < 800.0, "Apply is in the window: {apply:?}");
    assert!(
        h.text_at("Add ramps to toolpath").is_none(),
        "the form runs on out of sight, under the pinned buttons"
    );
    h.app.form.as_mut().unwrap().error = Some("select the vectors to profile".into());
    h.frame(vec![]);
    h.frame(vec![]);
    let error = h
        .text_at("select the vectors to profile")
        .expect("the error");
    let apply = h.text_at("Apply").unwrap();
    assert!(
        error.y < apply.y && error.y > 600.0,
        "{error:?} over {apply:?}"
    );
}

/// Wheels the form under `over` down until `text` sits clear of the buttons pinned at its
/// foot, and gives where it is.
fn scroll_above_foot(h: &mut Harness, over: Pos2, text: &str) -> Pos2 {
    let apply = h.text_at("Apply").expect("the form's buttons");
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
fn the_toolpath_list_ends_in_preview_then_a_labelled_save_button() {
    use super::panels::{SAVE_FOR_MACHINE, SAVE_NEEDS};
    let mut h = Harness::new();
    h.app.pages.set_pinned(Page::Toolpaths, true);
    h.frame(vec![]);
    h.frame(vec![]);
    assert!(h.text_at("Make a toolpath: pick how to cut").is_some());
    assert!(h.text_at(SAVE_FOR_MACHINE).is_some());
    assert!(h.text_at(SAVE_NEEDS).is_some(), "says what to do first");
    h.click(h.text_at(SAVE_FOR_MACHINE).unwrap(), Modifiers::NONE);
    assert!(h.app.save_form.is_none(), "nothing to save yet");
    h.rectangle(10.0, 10.0, 50.0, 50.0);
    h.app.invoke(&h.ctx, "profile_toolpath");
    let params = h.app.form.as_ref().unwrap().params.clone();
    h.app.apply("profile_toolpath", &params).unwrap();
    h.app.form = None;
    h.frame(vec![]);
    h.frame(vec![]);
    assert!(h.text_at(SAVE_NEEDS).is_none());
    let y = |h: &Harness, text: &str| h.text_at(text).unwrap_or_else(|| panic!("{text}")).y;
    assert!(y(&h, "Toolpath List") < y(&h, "Preview: watch it cut"));
    assert!(y(&h, "Preview: watch it cut") < y(&h, SAVE_FOR_MACHINE));
    h.click(h.text_at(SAVE_FOR_MACHINE).unwrap(), Modifiers::NONE);
    h.frame(vec![]);
    assert!(h.app.save_form.is_some(), "the same Save Toolpaths form");
}

#[test]
fn an_empty_sheet_says_where_to_start_until_something_is_drawn() {
    let mut h = Harness::new();
    let hint = format!(
        "{}\n{}",
        super::views::EMPTY_SHEET_TITLE,
        super::views::EMPTY_SHEET_HINT
    );
    let title = hint.as_str();
    assert!(h.text_at(title).is_some(), "shown on a new job");
    h.app.invoke(&h.ctx, "draw_rectangle");
    h.frame(vec![]);
    assert!(h.text_at(title).is_none(), "not over a tool in hand");
    h.app.form = None;
    h.frame(vec![]);
    assert!(h.text_at(title).is_some());
    h.rectangle(10.0, 10.0, 20.0, 10.0);
    assert!(h.text_at(title).is_none(), "gone with the first shape");
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
    // the starter tools give no flute length, so how deep they reach goes unchecked: a
    // note on every job, not a fault of the sample's
    let faults: Vec<_> = found
        .iter()
        .filter(|f| !f.message.contains("has no flute length set"))
        .collect();
    assert_eq!(faults, [] as [&cut_craft_core::preflight::Finding; 0]);
    assert!(
        !h.app.session.is_dirty(&h.app.history),
        "closing an untouched sample asks nothing"
    );
    assert!(h.app.session.path.is_none(), "saving asks where");
}

#[test]
fn the_community_links_are_in_help_not_on_the_start_page() {
    let h = Harness::at_start();
    assert!(h.text_at("Facebook").is_none());
    assert!(h.text_at("Cut Craft on GitHub").is_some());
    let help = h
        .app
        .current_menus()
        .menus
        .iter()
        .find(|m| m.name == "Help")
        .unwrap()
        .items
        .clone();
    let community = leaf(&help, &["Community"]);
    assert_eq!(community.children.len(), super::LINKS.len());
    assert!(
        community
            .children
            .iter()
            .all(|i| h.app.available(&i.action))
    );
}

#[test]
fn plate_production_recalculates_toolpaths_on_placeholder_text_for_each_plates_text() {
    let mut h = Harness::new();
    let mut text = h.app.registry.get("draw_text").unwrap().default_params();
    text["text"] = "!!name!!".into();
    text["font"] = "Test Boxes".into();
    text["height"] = 5.0.into();
    text["anchor"] = serde_json::json!({ "x": 10.0, "y": 10.0 });
    h.app.apply("draw_text", &text).unwrap();
    h.app.form = None;
    h.app.invoke(&h.ctx, "profile_toolpath");
    let profile = h.app.form.as_ref().unwrap().params.clone();
    h.app.apply("profile_toolpath", &profile).unwrap();
    h.app.form = None;
    let template = h.app.doc.toolpaths.values().next().unwrap().clone();
    let file =
        std::env::temp_dir().join(format!("cut-craft-app-plates-{}.csv", std::process::id()));
    std::fs::write(&file, "name\nI\n").unwrap();
    let mut p = h
        .app
        .registry
        .get("plate_production")
        .unwrap()
        .initial_params(&h.app.doc);
    p["data"]["file"] = file.display().to_string().into();
    h.app.apply("plate_production", &p).unwrap();
    let plate = h
        .app
        .doc
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
        if rel == "app/mac.rs" || rel.ends_with("tests.rs") {
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
        "no glyph in egui's fonts, so these draw as empty boxes; draw an icon \
         (scripts/make_icons.py) or pick a character they have:\n{}",
        missing.join("\n")
    );
}

/// The push-pin is drawn, loose and pinned, not left to a font's 📌.
#[test]
fn the_pin_buttons_are_drawn_icons() {
    let h = Harness::new();
    for icon in [super::panels::PIN_ICON, super::panels::PINNED_ICON] {
        assert!(
            h.app.icons.find(icon, "").is_some(),
            "{icon} is in assets/icons"
        );
    }
}

#[test]
fn an_embellishment_follows_its_settings_in_place_and_one_undo_takes_it_away() {
    let mut h = Harness::new();
    h.rectangle(10.0, 10.0, 160.0, 120.0);
    let shape: Vec<EntityId> = h.app.doc.vectors.keys().copied().collect();
    h.app.doc.selection = shape.iter().copied().collect();
    h.app.invoke(&h.ctx, "embellish");
    h.frame(vec![]);
    let mut params = h.app.form.as_ref().expect("the form").params.clone();
    h.app.form.as_mut().unwrap().params = params.clone();
    h.app.apply_form("embellish", &params).unwrap();
    let pieces = |h: &Harness| -> std::collections::BTreeSet<EntityId> {
        h.app
            .doc
            .vectors
            .keys()
            .filter(|id| !shape.contains(id))
            .copied()
            .collect()
    };
    let first = pieces(&h);
    assert!(first.len() > 20, "ornament made");
    assert_eq!(h.app.doc.components.len(), 1, "and its carving");
    let outlines = |h: &Harness| -> Vec<String> {
        let mut all: Vec<String> = pieces(h)
            .iter()
            .map(|id| format!("{:?}", h.app.doc.vectors[id]))
            .collect();
        all.sort();
        all
    };
    let first_outlines = outlines(&h);
    let undo_depth = h.app.history.undo_depth();
    // a new design takes the place of the first, live, rather than adding another
    params["seed"] = 99.into();
    let form = h.app.form.as_mut().unwrap();
    form.params = params.clone();
    form.dirty = true;
    h.frame(vec![]);
    assert!(
        !pieces(&h).is_empty() && outlines(&h) != first_outlines,
        "made again"
    );
    assert_eq!(h.app.doc.components.len(), 1, "one carving still");
    assert_eq!(
        h.app.history.undo_depth(),
        undo_depth,
        "in place of the first"
    );
    // Apply with the same settings has nothing more to do
    let second = pieces(&h);
    h.app.apply_form("embellish", &params).unwrap();
    assert_eq!(pieces(&h), second);
    // one undo leaves the shape as it was
    h.app.invoke(&h.ctx, "undo");
    assert!(pieces(&h).is_empty());
    assert!(h.app.doc.components.is_empty());
}

#[test]
fn a_form_tool_reads_in_one_line() {
    let tool = Tool {
        diameter: 6.35,
        pass_depth: 3.0,
        ..Tool::default()
    };
    assert_eq!(
        tool_line(&tool, Some(2.0)),
        "6.35 mm end mill, 3 mm a pass, 2 passes"
    );
    assert_eq!(
        tool_line(&tool, Some(1.0)),
        "6.35 mm end mill, 3 mm a pass, 1 pass"
    );
    let v = Tool {
        shape: cut_craft_core::toolpath::ToolShape::VBit,
        angle: 60.0,
        diameter: 12.7,
        pass_depth: 2.0,
        ..Tool::default()
    };
    assert_eq!(
        tool_line(&v, None),
        "60° V-bit, 12.7 mm across, 2 mm a pass"
    );
    let slots = [None, Some("Badger".to_owned())];
    assert_eq!(held_name("Badger", &slots), "T2  Badger");
    assert_eq!(held_name("Other", &slots), "Other");
}

#[test]
fn the_tabs_read_in_one_line_on_the_form() {
    use cut_craft_core::entry::{TabPlacement, TabSettings, TabSpacing, Tabs};
    let tab = TabSettings::default();
    let mm = Units::Mm;
    assert_eq!(tabs::summary(Tabs::None, tab, true, 0, mm), None);
    assert_eq!(
        tabs::summary(Tabs::Flat, tab, true, 0, mm).unwrap(),
        "12.7 mm long, 6.35 mm thick (from the tool), about one every 150 mm, at least 3 a shape"
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
    h.app.invoke(&h.ctx, "profile_toolpath");
    h.app.pages.set_pinned(Page::Toolpaths, true);
    h.app.choose_tool("3 mm End Mill");
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
    // Select… opens the Tool Database to pick in, on the form's tool
    h.click(h.text_at("Select…").unwrap(), Modifiers::NONE);
    h.frame(vec![]);
    h.frame(vec![]);
    let manager = h.app.tool_manager.as_mut().expect("Select Tool");
    assert!(manager.selecting);
    assert_eq!(manager.shown_tool(), Some("3 mm End Mill"));
    let other = manager
        .draft
        .tools
        .iter()
        .position(|t| t.name == "6 mm Ball Nose")
        .unwrap();
    manager.pick = Some(cut_craft_tool_database::Pick::Tool(other));
    h.frame(vec![]);
    h.frame(vec![]);
    assert!(h.text_at("Select Tool").is_some());
    let select = *h.texts_at("Select").last().expect("Select, in place of OK");
    h.click(select, Modifiers::NONE);
    h.frame(vec![]);
    assert!(h.app.tool_manager.is_none(), "closed");
    let form = h.app.form.as_ref().unwrap();
    let ball = h.app.tools.get("6 mm Ball Nose").unwrap();
    assert_eq!(form.params["tool"]["name"], "6 mm Ball Nose");
    assert_eq!(
        form.params["tool"]["feed_rate"],
        serde_json::json!(ball.feed_rate),
        "its own feeds and speeds come with it"
    );
    // 2D tabs: the form says what they will be
    h.app.form.as_mut().unwrap().params["tabs"] = "flat".into();
    h.frame(vec![]);
    h.frame(vec![]);
    assert!(
        h.app.form.as_ref().unwrap().params["tab"]["spacing"] == "automatic",
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

#[test]
fn pocket_clears_with_bigger_bits_picked_in_the_tool_database() {
    let mut h = Harness::new();
    h.rectangle(10.0, 10.0, 50.0, 50.0);
    h.click(h.at(60.0, 35.0), Modifiers::NONE);
    h.app.invoke(&h.ctx, "pocket_toolpath");
    h.app.pages.set_pinned(Page::Toolpaths, true);
    h.app.choose_tool("3 mm End Mill");
    h.frame(vec![]);
    h.frame(vec![]);
    // one picker on the form: no separate Rest Machining list beside clearing
    assert!(
        h.text_at("Rest Machining").is_none(),
        "clearing does its job"
    );
    assert!(
        h.text_at("Add a tool…").is_none(),
        "no menus of tools on the form"
    );
    let tick = h
        .text_at("Clear with bigger bits first")
        .expect("clearing, off to start with");
    assert_eq!(
        h.app.form.as_ref().unwrap().params["clearance_tools"],
        serde_json::json!([])
    );
    // ticking it opens the Tool Database to pick the clearing bit in
    h.click(tick, Modifiers::NONE);
    h.frame(vec![]);
    h.frame(vec![]);
    let manager = h.app.tool_manager.as_mut().expect("Select Tool");
    assert!(manager.selecting);
    let big = manager
        .draft
        .tools
        .iter()
        .position(|t| t.name == "12 mm End Mill")
        .unwrap();
    manager.pick = Some(cut_craft_tool_database::Pick::Tool(big));
    h.frame(vec![]);
    h.frame(vec![]);
    let select = *h.texts_at("Select").last().expect("Select, in place of OK");
    h.click(select, Modifiers::NONE);
    h.frame(vec![]);
    h.frame(vec![]);
    let form = h.app.form.as_ref().unwrap();
    assert_eq!(
        form.params["tool"]["name"], "3 mm End Mill",
        "the finishing bit stays"
    );
    assert_eq!(form.params["clearance_tools"][0]["name"], "12 mm End Mill");
    assert_eq!(
        form.params["clearance_tools"][0]["feed_rate"],
        serde_json::json!(h.app.tools.get("12 mm End Mill").unwrap().feed_rate),
        "with its own feeds and speeds"
    );
    // another, in between: kept largest first
    h.click(
        h.text_at("Add a smaller clearing bit…")
            .expect("another bit"),
        Modifiers::NONE,
    );
    h.frame(vec![]);
    h.frame(vec![]);
    let manager = h.app.tool_manager.as_mut().expect("Select Tool");
    let middle = manager
        .draft
        .tools
        .iter()
        .position(|t| t.name == "6 mm End Mill")
        .unwrap();
    manager.pick = Some(cut_craft_tool_database::Pick::Tool(middle));
    h.frame(vec![]);
    h.frame(vec![]);
    h.click(*h.texts_at("Select").last().unwrap(), Modifiers::NONE);
    h.frame(vec![]);
    h.frame(vec![]);
    let names: Vec<&str> = h.app.form.as_ref().unwrap().params["clearance_tools"]
        .as_array()
        .unwrap()
        .iter()
        .map(|t| t["name"].as_str().unwrap())
        .collect();
    assert_eq!(names, ["12 mm End Mill", "6 mm End Mill"]);
}

#[test]
fn pocket_says_on_the_form_when_a_clearing_bit_is_not_bigger_than_the_finishing_bit() {
    let mut h = Harness::new();
    h.rectangle(10.0, 10.0, 50.0, 50.0);
    h.click(h.at(60.0, 35.0), Modifiers::NONE);
    h.app.invoke(&h.ctx, "pocket_toolpath");
    h.app.pages.set_pinned(Page::Toolpaths, true);
    h.app.choose_tool("6 mm End Mill");
    let small = serde_json::to_value(h.app.tools.get("3 mm End Mill").unwrap()).unwrap();
    h.app.form.as_mut().unwrap().params["clearance_tools"] = serde_json::json!([small]);
    h.frame(vec![]);
    h.frame(vec![]);
    let said = h.shapes.iter().any(|clipped| {
        matches!(&clipped.shape, egui::Shape::Text(t)
            if t.galley.text().contains("must be bigger than the finishing bit"))
    });
    assert!(said, "the problem shows before Calculate");
}

#[test]
fn an_older_pocket_with_a_rest_list_still_shows_it() {
    let mut h = Harness::new();
    h.rectangle(10.0, 10.0, 50.0, 50.0);
    h.click(h.at(60.0, 35.0), Modifiers::NONE);
    h.app.invoke(&h.ctx, "pocket_toolpath");
    h.app.pages.set_pinned(Page::Toolpaths, true);
    let big = serde_json::to_value(h.app.tools.get("12 mm End Mill").unwrap()).unwrap();
    h.app.form.as_mut().unwrap().params["rest"] =
        serde_json::json!({ "after": [big], "minimum_detail": 0.0, "overlap": 0.0 });
    h.frame(vec![]);
    h.frame(vec![]);
    assert!(
        h.text_at("Rest Machining").is_some(),
        "kept, so it can be changed"
    );
}

/// Tools that were only in a menu get a button too (Matthew, 2026-11-10): the left page's
/// for what changes the shape to cut, the right page's for how it is cut and sent to the
/// machine, each where Aspire's help puts its icon. Buttons already there stay put.
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
            "job_setup",
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
    starts(
        "Toolpath Templates",
        &["array_copy_toolpath", "merged_toolpath"],
    );
    starts(
        "Output",
        &[
            "save_toolpaths",
            "toolpath_summary",
            "tool_database",
            "keep_out_zones",
            "toolpath_tiling",
        ],
    );
}

#[test]
fn a_fill_follows_its_settings_in_place_and_one_undo_takes_it_away() {
    let mut h = Harness::new();
    h.rectangle(10.0, 10.0, 100.0, 100.0);
    let square: Vec<EntityId> = h.app.doc.vectors.keys().copied().collect();
    h.app.doc.selection = square.iter().copied().collect();
    h.app.invoke(&h.ctx, "vector_texture");
    h.frame(vec![]);
    let mut params = h.app.form.as_ref().expect("the form").params.clone();
    params["style"] = "snowflakes".into();
    h.app.form.as_mut().unwrap().params = params.clone();
    h.app.apply_form("vector_texture", &params).unwrap();
    let fill = |h: &Harness| -> std::collections::BTreeSet<EntityId> {
        h.app
            .doc
            .vectors
            .keys()
            .filter(|id| !square.contains(id))
            .copied()
            .collect()
    };
    let first = fill(&h);
    assert!(first.len() > 3, "snowflakes made");
    let shapes = |h: &Harness| -> Vec<String> {
        let mut all: Vec<String> = fill(h)
            .iter()
            .map(|id| format!("{:?}", h.app.doc.vectors[id]))
            .collect();
        all.sort();
        all
    };
    let first_shapes = shapes(&h);
    let undo_depth = h.app.history.undo_depth();
    // a new variation takes the place of the fill, live, rather than adding another
    params["seed"] = 99.into();
    let form = h.app.form.as_mut().unwrap();
    form.params = params.clone();
    form.dirty = true;
    h.frame(vec![]);
    let second = fill(&h);
    assert!(
        !second.is_empty() && shapes(&h) != first_shapes,
        "made again"
    );
    assert_eq!(
        h.app.history.undo_depth(),
        undo_depth,
        "in place of the first"
    );
    // and Apply with the same settings has nothing more to do
    h.app.apply_form("vector_texture", &params).unwrap();
    assert_eq!(fill(&h), second);
    // one undo leaves the square as it was
    h.app.invoke(&h.ctx, "undo");
    assert!(fill(&h).is_empty());
    assert_eq!(h.app.doc.vectors.len(), 1);
    // with another shape picked, Apply fills it too and keeps the first fill
    h.app.invoke(&h.ctx, "redo");
    h.rectangle(120.0, 10.0, 80.0, 80.0);
    let kept = fill(&h);
    h.app.invoke(&h.ctx, "vector_texture");
    h.frame(vec![]);
    h.app.form.as_mut().unwrap().params = params.clone();
    h.app.apply_form("vector_texture", &params).unwrap();
    assert!(fill(&h).is_superset(&kept), "the first fill stays");
}

#[test]
fn a_decorative_relief_follows_its_settings_as_one_component() {
    let mut h = Harness::new();
    h.rectangle(10.0, 10.0, 150.0, 150.0);
    let square: Vec<EntityId> = h.app.doc.vectors.keys().copied().collect();
    h.app.doc.selection = square.iter().copied().collect();
    h.app.invoke(&h.ctx, "decorative_relief");
    h.frame(vec![]);
    let mut params = h.app.form.as_ref().expect("the form").params.clone();
    h.app.apply_form("decorative_relief", &params).unwrap();
    assert_eq!(h.app.doc.components.len(), 1, "one relief made");
    let first = h
        .app
        .doc
        .components
        .values()
        .next()
        .unwrap()
        .heights
        .clone();
    let undo_depth = h.app.history.undo_depth();
    // a new variation carves another in its place, live
    params["seed"] = 7.into();
    let form = h.app.form.as_mut().unwrap();
    form.params = params.clone();
    form.dirty = true;
    h.frame(vec![]);
    assert_eq!(h.app.doc.components.len(), 1, "in place of the first");
    let second = &h.app.doc.components.values().next().unwrap().heights;
    assert!(!cut_craft_core::component::same_heights(&first, second));
    assert_eq!(h.app.history.undo_depth(), undo_depth);
    h.app.invoke(&h.ctx, "undo");
    assert!(h.app.doc.components.is_empty());
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
    h.app.display.previews_2d = true;
    let (width, height) = (h.app.doc.job.width, h.app.doc.job.height);
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
    let mut c = Change::builder(&h.app.doc);
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
    });
    h.app.record(Ok(c.finish()), "made");
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
