//! Moving, transforming, aligning, cutting, copying, pasting, deleting and undo.

use super::*;

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
fn set_size_with_link_xy_works_out_the_other_box_from_the_one_typed() {
    let mut h = Harness::new();
    h.rectangle(10.0, 10.0, 50.0, 20.0);
    h.app.invoke(&h.ctx, "set_size");
    h.frame(vec![]);
    h.app.form.as_mut().unwrap().params["height"] = 40.0.into();
    h.frame(vec![]);
    let params = &h.app.form.as_ref().unwrap().params;
    assert_eq!(
        params["width"].as_f64(),
        Some(100.0),
        "the height sets the width"
    );
    h.app.form.as_mut().unwrap().params["width"] = 25.0.into();
    h.frame(vec![]);
    let params = &h.app.form.as_ref().unwrap().params;
    assert_eq!(
        params["height"].as_f64(),
        Some(10.0),
        "the width sets the height"
    );
    h.app.form.as_mut().unwrap().params["keep_proportions"] = false.into();
    h.app.form.as_mut().unwrap().params["height"] = 33.0.into();
    h.frame(vec![]);
    let params = &h.app.form.as_ref().unwrap().params;
    assert_eq!(
        params["width"].as_f64(),
        Some(25.0),
        "unlinked, each stands alone"
    );
}

#[test]
fn set_size_items_one_by_one_with_link_xy_goes_by_the_box_typed() {
    let mut h = Harness::new();
    h.rectangle(10.0, 10.0, 50.0, 20.0);
    h.rectangle(100.0, 10.0, 10.0, 40.0);
    h.app.doc.selection = h.app.doc.vectors.keys().copied().collect();
    h.app.invoke(&h.ctx, "set_size");
    h.frame(vec![]);
    let form = h.app.form.as_mut().unwrap();
    form.params["mode"] = "scale_items_individually".into();
    form.params["height"] = 30.0.into();
    h.frame(vec![]);
    let params = &h.app.form.as_ref().unwrap().params;
    assert_eq!(
        params["by_height"].as_bool(),
        Some(true),
        "sized by the height"
    );
    assert_eq!(params["height"].as_f64(), Some(30.0));
    h.app.form.as_mut().unwrap().params["width"] = 12.0.into();
    h.frame(vec![]);
    let params = &h.app.form.as_ref().unwrap().params;
    assert_eq!(
        params["by_height"].as_bool(),
        Some(false),
        "now by the width"
    );
    assert_eq!(params["width"].as_f64(), Some(12.0));
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
fn the_window_is_titled_cut_craft() {
    let h = Harness::new();
    assert_eq!(h.app.title, "Untitled - Cut Craft");
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
fn auto_rotates_speed_is_kept_with_the_options() {
    let o = crate::options::Options::default().view_3d;
    assert!(
        (o.spin_speed() - 20.0).abs() < 1e-9,
        "a slow turn to start with"
    );
    let wild = crate::options::View3d {
        spin_speed: 1000.0,
        ..o.clone()
    };
    assert!((wild.spin_speed() - 90.0).abs() < 1e-9);
    let slow = crate::options::View3d {
        spin_speed: 0.0,
        ..o
    };
    assert!((slow.spin_speed() - 2.0).abs() < 1e-9);
}

#[test]
fn a_copy_pasted_on_its_original_shows_its_common_lines_in_red() {
    let mut h = Harness::new();
    h.rectangle(10.0, 10.0, 20.0, 10.0);
    let red = |h: &mut Harness| {
        h.frame(vec![]);
        let colour = egui::epaint::ColorMode::Solid(h.ctx.global_style().visuals.error_fg_color);
        h.shapes
            .iter()
            .filter(|c| matches!(&c.shape, egui::Shape::Path(p) if p.stroke.color == colour))
            .count()
    };
    let before = red(&mut h);
    h.app.invoke(&h.ctx, "copy");
    h.app.invoke(&h.ctx, "paste");
    // pasted in place, on top of the original: each of both rectangles' four sides
    assert_eq!(red(&mut h), before + 8);
    h.app.invoke(&h.ctx, "undo");
    assert_eq!(red(&mut h), before, "gone with the copy");
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
fn the_other_units_can_be_chosen_and_set_size_opened() {
    let mut h = Harness::new();
    h.app.import_file(&unitless_dxf("units-other"));
    h.frame(vec![]);
    h.frame(vec![]);
    h.click(
        h.text_at("Millimetres: 6.0 mm by 6.0 mm (too small to see)")
            .unwrap(),
        Modifiers::NONE,
    );
    h.click(h.text_at("Zoom and Set Size…").unwrap(), Modifiers::NONE);
    h.frame(vec![]);
    assert!((drawn_width(&h) - 6.0).abs() < 1e-6, "{}", drawn_width(&h));
    assert_eq!(h.app.form.as_ref().map(|f| f.id), Some("set_size"));
}

#[test]
fn ctrl_alt_s_saves_a_numbered_copy_and_leaves_the_file_as_it_was() {
    let mut h = Harness::new();
    let dir = std::env::temp_dir().join(format!("cut-craft-increment-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let job = dir.join("Sign.ccjob");
    std::fs::write(&job, b"earlier").unwrap();
    h.app.session.path = Some(job.clone());
    let held = Modifiers::COMMAND | Modifiers::ALT;
    h.frame_holding(held, vec![key_with(Key::S, held)]);
    assert!(
        std::fs::read(&job).unwrap() == b"earlier",
        "the earlier file is kept, not saved over"
    );
    assert!(dir.join("Sign_1.ccjob").is_file(), "a numbered copy");
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn cut_craft_opens_to_a_blank_job_unless_classic_startup_flow_is_ticked() {
    let mut h = Harness::at_start();
    h.app.options.default_job = Some(cut_craft_core::document::Job {
        width: 1220.0,
        height: 610.0,
        ..Default::default()
    });
    h.app.open_at_start(None);
    h.frame(vec![]);
    h.frame(vec![]);
    assert_eq!(h.app.screen, Screen::Job);
    assert!(h.app.form.is_none(), "nothing to fill in first");
    assert_eq!((h.app.doc.job.width, h.app.doc.job.height), (1220.0, 610.0));
    assert!(
        !h.app.session.is_dirty(&h.app.history),
        "nothing to save yet"
    );
    assert!(
        h.text_at(views::EMPTY_SHEET_SAMPLE).is_some(),
        "ready to draw on, the sample a click away"
    );
    // the old start, as a setting
    let mut h = Harness::at_start();
    h.app.options.job_defaults.classic_startup = true;
    h.app.open_at_start(None);
    assert_eq!(h.app.screen, Screen::Start);
}
