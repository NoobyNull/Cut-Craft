//! Node editing, polylines, freehand, text, and joining open ends.

use super::*;

#[test]
fn grouping_text_says_nothing_in_red_and_keeps_it_text() {
    let mut h = Harness::new();
    let mut c = Change::builder(h.doc());
    let letter = c.add(Path::circle(Point::new(20.0, 20.0), 2.0));
    c.set_group(letter, Some(5));
    c.set_text(5, Some(serde_json::json!({"text": "o"})));
    let border = c.add(Path::circle(Point::new(20.0, 20.0), 15.0));
    c.select([letter, border]);
    h.given(&c.finish());
    h.frame(vec![]);
    h.invoke("group");
    h.frame(vec![]);
    assert_eq!(h.tool_note(), None, "nothing to warn about");
    assert!(
        !h.texts().iter().any(|t| t.contains("curves")),
        "no word of curves"
    );
    assert_eq!(h.doc().text_group(letter), Some(5), "still text");
    assert_eq!(h.doc().group_members(border).len(), 2, "one group");
}

#[test]
fn freehand_follows_the_whole_drag() {
    let mut h = Harness::new();
    h.invoke("draw_freehand");
    let points = [(10.0, 10.0), (20.0, 30.0), (30.0, 10.0), (40.0, 30.0)];
    h.frame(vec![Event::PointerMoved(h.at(10.0, 10.0))]);
    h.press(h.at(10.0, 10.0), true, Modifiers::NONE);
    for (x, y) in points {
        h.frame(vec![Event::PointerMoved(h.at(x, y))]);
    }
    h.press(h.at(40.0, 30.0), false, Modifiers::NONE);
    h.frame(vec![]);
    let drawn = h.doc().vectors.values().next().expect("a stroke");
    let b = drawn.bounds();
    assert!(
        b.size().y > 15.0,
        "it went up and down, not straight: {b:?}"
    );
}

#[test]
fn node_editing_drags_a_corner_and_offers_its_menu() {
    let mut h = Harness::new();
    h.rectangle(10.0, 10.0, 20.0, 10.0);
    let id = *h.doc().selection.iter().next().unwrap();
    h.invoke("node_edit");
    assert_eq!(h.view_tool(), Some(ViewTool::NodeEdit));
    // smart snapping would line the corner up with the other top corner
    h.given_options(|o| o.snapping.smart_snapping = false);
    let (from, to) = (h.at(30.0, 20.0), h.at(40.0, 25.0));
    h.drag(from, to);
    let corners = cut_craft_core::nodes::nodes(&h.doc().vectors[&id]);
    assert!(
        corners
            .iter()
            .any(|q| q.distance(Point::new(40.0, 25.0)) < 0.5),
        "the corner followed the drag: {corners:?}"
    );
    assert_eq!(h.doc().vectors.len(), 1, "a node drag selects nothing new");
    let target = h
        .node_target(Point::new(10.0, 10.0), 1.0)
        .expect("a node there");
    let menu = gui::node_menu(h.doc(), &target);
    // Smooth Point turns the corner's spans into Béziers; Node Position opens Move Node there
    let smooth = leaf(&menu, &["Smooth Point"]);
    assert!(smooth.action.contains("smooth_point"), "{}", smooth.action);
    let position = leaf(&menu, &["Node Position..."]);
    assert!(position.action.contains("move_node"), "{}", position.action);
    let delete = leaf(&menu, &["Delete Point"]);
    h.invoke(&delete.action);
    assert_eq!(cut_craft_core::nodes::nodes(&h.doc().vectors[&id]).len(), 3);
    h.invoke("node_edit");
    assert_eq!(h.view_tool(), None, "N toggles it off");
}

#[test]
fn a_span_made_a_bezier_shows_handles_that_drag_to_bend_it() {
    use cut_craft_core::geometry::Segment;
    let mut h = Harness::new();
    h.rectangle(10.0, 10.0, 20.0, 10.0);
    let id = *h.doc().selection.iter().next().unwrap();
    h.invoke("node_edit");
    h.given_options(|o| o.snapping.smart_snapping = false);
    h.given_options(|o| o.snapping.snap_to_grid = false);
    // right-click the bottom side, Convert Span to Bezier
    let target = h
        .node_target(Point::new(20.0, 10.0), 1.0)
        .expect("the bottom side");
    let menu = gui::node_menu(h.doc(), &target);
    let convert = leaf(&menu, &["Convert Span to Bezier"]);
    h.invoke(&convert.action);
    assert!(
        h.doc().selection.contains(&id),
        "still the one being edited"
    );
    let crate::view2d::Overlay::Nodes { handles, .. } = h.node_overlay(None, 1.0) else {
        panic!("the node overlay")
    };
    assert_eq!(
        handles.len(),
        2,
        "one at each end of the curve: {handles:?}"
    );
    // drag the handle nearest the left corner down: the side bends, the corners stay
    let (_, handle) = handles
        .iter()
        .copied()
        .min_by(|a, b| a.1.x.total_cmp(&b.1.x))
        .unwrap();
    let corners = cut_craft_core::nodes::nodes(&h.doc().vectors[&id]);
    let (from, to) = (h.at(handle.x, handle.y), h.at(handle.x, handle.y - 6.0));
    h.drag(from, to);
    let path = &h.doc().vectors[&id];
    assert_eq!(
        cut_craft_core::nodes::nodes(path),
        corners,
        "the corners stay"
    );
    let pulled = path.segments.iter().any(|s| {
        matches!(s, Segment::Cubic { c1, c2, .. }
            if c1.y < 7.0 || c2.y < 7.0)
    });
    assert!(
        pulled,
        "a handle went down with the drag: {:?}",
        path.segments
    );
}

#[test]
fn sketch_carving_shows_the_areas_it_would_carve_while_its_form_is_open() {
    use cut_craft_core::bitmap::Bitmap;
    use cut_craft_core::geometry::{Point, Rect};
    let mut h = Harness::new();
    let mut c = Change::builder(h.doc());
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
    h.make(c.finish(), "added");
    h.invoke("sketch_carving_toolpath");
    h.frame(vec![]);
    let areas = h.sketch_areas().expect("found");
    assert!(!areas.is_empty(), "the stripe shows");
    assert!(
        areas.iter().all(|r| r.min.x >= 15.0 && r.max.x <= 30.0),
        "{areas:?}"
    );
    h.close_form();
    h.frame(vec![]);
    assert!(h.sketch_areas().is_none(), "gone with the form");
}

#[test]
fn forms_open_with_values_from_the_document() {
    let mut h = Harness::new();
    h.rectangle(10.0, 10.0, 50.0, 20.0);
    h.invoke("set_size");
    let params = h.form_settings();
    assert_eq!(
        (params["width"].as_f64(), params["height"].as_f64()),
        (Some(50.0), Some(20.0))
    );
}

#[test]
fn text_grouped_with_a_border_stays_editable_inside_the_group() {
    let mut h = Harness::new();
    let mut full = h.default_settings("draw_text");
    full["text"] = Value::from("HI");
    full["font"] = Value::from("Test Boxes");
    h.run("draw_text", &full).unwrap();
    h.close_form();
    let letters = h.doc().selection.clone();
    let letter = *letters.iter().next().unwrap();
    let box_ = h.doc().vectors[&letter].bounds();
    h.rectangle(box_.min.x - 200.0, box_.min.y - 200.0, 600.0, 600.0);
    h.invoke("select_all_vectors");
    h.invoke("group");
    assert!(h.tool_note().is_none(), "no warning: the text stays text");
    h.select([]);
    h.frame(vec![]);
    // the letters are still text: Edit Text works on the group
    h.invoke("select_all_vectors");
    assert!(h.available("edit_text"));
    h.select([]);
    h.frame(vec![]);
    // double-click steps in, a second double-click on the letters opens Edit Text
    let on_letter = h.at(box_.min.x, box_.center().y);
    h.double_click(on_letter);
    assert!(h.doc().inside_group.is_some());
    h.double_click(on_letter);
    assert_eq!(
        (h.form_id().unwrap(), h.form_settings()["text"].as_str()),
        ("draw_text", Some("HI"))
    );
    assert_eq!(
        h.doc().selection.len(),
        letters.len(),
        "the text alone is picked"
    );
    // the edited text is still in the group with its border
    let mut params = h.form_settings().clone();
    params["text"] = Value::from("HO");
    h.run("draw_text", &params).unwrap();
    let letter = *h.doc().selection.iter().next().unwrap();
    assert_eq!(
        h.doc().group_members(letter).len(),
        h.doc().vectors.len(),
        "one group still"
    );
    assert!(h.doc().text_group(letter).is_some(), "still text");
}

#[test]
fn the_join_icons_join_two_vectors_their_way() {
    let mut h = Harness::new();
    let line =
        |x0: f64, x1: f64| Path::polyline(&[Point::new(x0, 10.0), Point::new(x1, 10.0)], false);
    let change = Change::swap(h.doc(), [], vec![line(0.0, 10.0), line(20.0, 30.0)]);
    h.given(&change);
    for id in [
        "join_straight_line",
        "join_smooth_curve",
        "join_move_end_points",
    ] {
        assert!(h.available(id), "{id}");
    }
    h.invoke("join_smooth_curve");
    assert_eq!(h.doc().vectors.len(), 1, "{}", h.status_message());
    let edit = h
        .toolbar()
        .groups
        .iter()
        .find(|g| g.name == "Join, Combine and Trim")
        .unwrap();
    assert!(edit.tools.contains(&"join_move_end_points"));
}

#[test]
fn edit_text_opens_draw_text_on_the_selected_text() {
    let mut h = Harness::new();
    assert!(!h.available("edit_text"), "no text selected");
    let params = serde_json::json!({ "text": "HI", "font": "Test Boxes" });
    let mut full = h.default_settings("draw_text");
    for (k, v) in params.as_object().unwrap() {
        full[k] = v.clone();
    }
    h.run("draw_text", &full).unwrap();
    assert!(h.available("edit_text"));
    h.invoke("edit_text");
    assert_eq!(
        (h.form_id().unwrap(), h.form_settings()["text"].as_str()),
        ("draw_text", Some("HI"))
    );
}

#[test]
fn a_recent_file_that_is_there_but_will_not_open_stays_listed() {
    let path =
        std::env::temp_dir().join(format!("cut-craft-cut-short-{}.ccjob", std::process::id()));
    // cut short, as by a cloud sync still going
    let whole = cut_craft_core::file::save_job(&Document::default());
    std::fs::write(&path, &whole[..whole.len() / 2]).unwrap();
    let mut h = Harness::at_start();
    h.given_recent_files(vec![path.clone()]);
    h.invoke("open_recent:0");
    assert_eq!(h.screen(), Screen::Start);
    assert!(
        h.status_message().contains("damaged"),
        "{}",
        h.status_message()
    );
    assert_eq!(
        h.recent_shown(),
        vec![path.clone()],
        "kept to try again later"
    );
    std::fs::remove_file(&path).unwrap();
}

#[test]
fn run_and_form_items_apply_or_open_a_command_with_their_params() {
    let mut h = Harness::new();
    let add = gui::MenuItem::run::<cmd_add_layer::AddLayer>(
        "Add",
        &cmd_add_layer::Params { name: "Top".into() },
    );
    assert!(h.available(&add.action));
    h.invoke(&add.action);
    assert_eq!(h.doc().layers.last().map(|l| l.name.as_str()), Some("Top"));
    let mut rename: cmd_layer_properties::Params = h.doc().layers[0].clone().into();
    rename.name = "Renamed".into();
    let rename = gui::MenuItem::form::<cmd_layer_properties::LayerProperties>("Rename", &rename);
    h.invoke(&rename.action);
    assert_eq!(
        (h.form_id().unwrap(), h.form_settings()["name"].as_str()),
        ("layer_properties", Some("Renamed"))
    );
    assert!(!h.available("run:no_such_command:{}"));
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
    assert!(h.doc().bitmaps.is_empty());
    assert!(h.status_message().contains("job"), "{}", h.status_message());
    // in a job a picture is imported; dragging over says what a drop will do
    let mut h = Harness::new();
    h.drop_files(std::slice::from_ref(&picture), false);
    assert!(h.text_at("Drop to open or import").is_some());
    h.drop_files(&[picture], true);
    assert_eq!(h.doc().bitmaps.len(), 1, "imported");
    // a post-processor is installed
    h.given_posts_folder(dir.join("posts"));
    let post = dir.join("router.pp");
    std::fs::write(&post, cut_craft_post::BUILT_IN[0].1).unwrap();
    h.drop_files(&[post], true);
    assert!(
        h.posts()
            .entries()
            .iter()
            .any(cut_craft_machines::Entry::is_custom)
    );
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn the_new_version_reopens_the_job_that_was_open() {
    let mut h = Harness::new();
    let job = std::env::temp_dir().join(format!("cut-craft-reopen-{}.ccjob", std::process::id()));
    std::fs::write(&job, b"job").unwrap();
    h.given_job_path(job.clone());
    h.given_job_screen();
    assert_eq!(h.job_to_reopen(), Some(job.clone()));
    // the start page has no job open
    h.given_start_screen();
    assert_eq!(h.job_to_reopen(), None);
    // a job never saved, or whose file has gone, opens nothing
    h.given_job_screen();
    std::fs::remove_file(&job).unwrap();
    assert_eq!(h.job_to_reopen(), None);
    h.given_no_job_path();
    assert_eq!(h.job_to_reopen(), None);
}

#[test]
fn editing_placed_text_replaces_it() {
    let mut h = Harness::new();
    h.invoke("draw_text");
    h.edit_form(|p| p["font"] = "Test Boxes".into());
    let at = Point::new(20.0, 20.0);
    let click = Drag {
        start: at,
        end: at,
        from_center: false,
        constrain: false,
    };
    h.draw("draw_text", click);
    h.edit_form(|p| p["text"] = "HI".into());
    let params = h.form_settings().clone();
    h.apply_form("draw_text", &params).unwrap();
    assert_eq!(h.doc().texts.len(), 1, "the placeholder was replaced");
    assert_eq!(h.doc().texts.values().next().unwrap()["text"], "HI");
}

#[test]
fn options_changes_wait_for_apply_which_keeps_the_window_open() {
    let mut h = options_with_a_change_not_applied();
    assert_eq!(h.options().general.recent_file_list_size, 9, "not used yet");
    assert!(h.options_dirty());
    assert!(h.text_at(super::options_apply::NOT_APPLIED).is_some());
    click_button(&mut h, "Apply");
    assert_eq!(h.options().recent_count(), files::RECENT_COUNT, "used now");
    assert!(!h.options_dirty());
    assert!(h.options_window_open(), "still open");
    h.frame(vec![]);
    assert!(h.text_at(super::options_apply::ALL_APPLIED).is_some());
}

#[test]
fn node_editing_picks_nodes_with_a_box_and_a_drag_moves_them_together() {
    let mut h = Harness::new();
    h.rectangle(10.0, 10.0, 20.0, 10.0);
    let id = *h.doc().selection.iter().next().unwrap();
    h.invoke("node_edit");
    h.given_options(|o| o.snapping.smart_snapping = false);
    h.given_options(|o| o.snapping.geometry_snapping = false);
    // a box round the two right-hand corners picks them
    h.drag(h.at(25.0, 5.0), h.at(35.0, 25.0));
    assert_eq!(h.picked_nodes().len(), 2);
    assert_eq!(h.doc().selection.len(), 1, "the vector stays selected");
    // dragging one of them moves both 5 mm right
    h.drag(h.at(30.0, 10.0), h.at(35.0, 10.0));
    let corners = cut_craft_core::nodes::nodes(&h.doc().vectors[&id]);
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
    assert!(h.picked_nodes().is_empty());
}

#[test]
fn a_polyline_is_drawn_point_by_point_with_curves_and_its_keys() {
    use cut_craft_core::geometry::Segment;
    let mut h = Harness::new();
    h.given_options(|o| o.snapping.smart_snapping = false);
    h.given_options(|o| o.snapping.geometry_snapping = false);
    h.invoke("draw_polyline");
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
    assert_eq!(h.polyline_drawn().len(), 4);
    assert!(
        h.polyline_drawn()[3].distance(Point::new(40.0, 50.0)) < 1e-3,
        "{:?}",
        h.polyline_drawn()[3]
    );
    // Backspace takes it back, Tab closes and makes the polyline
    h.frame(vec![key(Key::Backspace)]);
    assert_eq!(h.polyline_drawn().len(), 3);
    h.frame(vec![key(Key::Tab)]);
    let path = h.doc().vectors.values().last().expect("drawn");
    assert!(path.closed);
    assert_eq!(path.segments.len(), 3);
    assert!(
        matches!(path.segments[1], Segment::Cubic { .. }),
        "the dragged span"
    );
    assert!(matches!(path.segments[0], Segment::Line { .. }));
    assert!(h.polyline_drawn().is_empty(), "ready for the next");
    assert_eq!(h.form_id(), Some("draw_polyline"));
    // S: curve mode, Space finishes an open one
    h.frame(vec![key(Key::S)]);
    assert!(h.polyline_curved());
    click(&mut h, 100.0, 100.0);
    click(&mut h, 150.0, 120.0);
    click(&mut h, 200.0, 100.0);
    h.frame(vec![key(Key::Space)]);
    let path = h.doc().vectors.values().last().unwrap();
    assert!(!path.closed);
    assert!(
        path.segments
            .iter()
            .all(|s| matches!(s, Segment::Cubic { .. }))
    );
    assert!(h.polyline_curved(), "the mode is kept");
}

#[test]
fn double_clicking_a_polylines_last_point_finishes_it_there() {
    let mut h = Harness::new();
    h.given_options(|o| o.snapping.smart_snapping = false);
    h.given_options(|o| o.snapping.geometry_snapping = false);
    h.invoke("draw_polyline");
    h.frame(vec![]);
    let (a, b) = (h.at(10.0, 10.0), h.at(60.0, 10.0));
    h.click(a, Modifiers::NONE);
    h.double_click(b);
    assert!(h.polyline_drawn().is_empty(), "finished");
    assert!(!h.polyline_curved(), "still straight lines");
    let path = h.doc().vectors.values().last().expect("drawn");
    assert!(!path.closed);
    assert_eq!(path.segments.len(), 1);
}

#[test]
fn edit_polyline_from_an_end_node_draws_on_and_extends_the_same_vector() {
    use cut_craft_core::nodes::NodeAt;
    use cut_craft_core::params::Xy;
    let mut h = Harness::new();
    h.given_options(|o| o.snapping.smart_snapping = false);
    h.given_options(|o| o.snapping.geometry_snapping = false);
    let line = serde_json::json!({
        "points": [{ "x": 10.0, "y": 10.0 }, { "x": 50.0, "y": 10.0 }], "closed": false
    });
    h.run("draw_polyline", &line).unwrap();
    let id = *h.doc().vectors.keys().next().unwrap();
    // the node menu offers it at either end, not in the middle of a closed shape
    let at = |x, y| NodeAt {
        vector: id,
        at: Xy::new(x, y),
        reach: 1.0,
    };
    let offered = |h: &Harness, x, y| {
        gui::node_menu(h.doc(), &at(x, y))
            .iter()
            .find(|i| i.name == "Edit Polyline")
            .map(|i| i.action.clone())
    };
    let action = offered(&h, 50.0, 10.0).expect("at the end");
    assert!(offered(&h, 10.0, 10.0).is_some_and(|a| a.ends_with(":start")));
    h.invoke(&action);
    h.frame(vec![]);
    let to = h.at(50.0, 40.0);
    h.click(to, Modifiers::NONE);
    h.frame(vec![key(Key::Space)]);
    assert_eq!(h.doc().vectors.len(), 1, "extended, not a new vector");
    let path = &h.doc().vectors[&id];
    assert_eq!(path.segments.len(), 2);
    assert!(path.segments[1].end().distance(Point::new(50.0, 40.0)) < 1e-3);
    // undo takes the extension back
    h.undo_step();
    assert_eq!(h.doc().vectors[&id].segments.len(), 1);
}

#[test]
fn with_create_fillets_open_a_click_rounds_a_corner_and_a_click_on_the_fillet_takes_it_out() {
    let mut h = Harness::new();
    h.rectangle(10.0, 10.0, 40.0, 30.0);
    h.invoke("create_fillets");
    assert!(h.filleting());
    h.frame(vec![]);
    let shape = |h: &Harness| {
        let path = h.doc().vectors.values().next().unwrap();
        let arcs = path
            .segments
            .iter()
            .filter(|s| matches!(s, cut_craft_core::geometry::Segment::Arc { .. }))
            .count();
        (path.segments.len(), arcs)
    };
    h.click(h.at(10.0, 10.0), Modifiers::NONE);
    assert_eq!(shape(&h), (5, 1), "one corner rounded");
    assert!(h.filleting(), "the form stays open");
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
    let change = Change::swap(h.doc(), [], paths);
    h.given(&change);
    h.invoke("join_open_ends");
    assert!(h.joining_ends());
    h.frame(vec![]);
    // a click away from any end picks nothing
    h.click(h.at(25.0, 10.0), Modifiers::NONE);
    assert!(h.join_end_picked().is_none());
    h.frame(vec![]);
    h.click(h.at(40.0, 10.0), Modifiers::NONE);
    assert!(h.join_end_picked().is_some(), "the first end is picked");
    h.frame(vec![]);
    h.click(h.at(45.0, 10.0), Modifiers::NONE);
    assert_eq!(h.doc().vectors.len(), 1, "joined into one");
    assert!(h.join_end_picked().is_none(), "ready for the next pair");
    assert!(h.joining_ends(), "the tool stays open");
    // its two ends clicked, the one vector closes
    h.frame(vec![]);
    h.click(h.at(10.0, 10.0), Modifiers::NONE);
    h.frame(vec![]);
    h.click(h.at(80.0, 10.0), Modifiers::NONE);
    assert!(h.doc().vectors.values().next().unwrap().closed);
}

#[test]
fn escape_lets_a_picked_open_end_go_and_keeps_join_open_ends_open() {
    let mut h = Harness::new();
    let path = Path::polyline(&[Point::new(10.0, 10.0), Point::new(40.0, 10.0)], false);
    let change = Change::swap(h.doc(), [], vec![path]);
    h.given(&change);
    h.invoke("join_open_ends");
    h.frame(vec![]);
    h.click(h.at(40.0, 10.0), Modifiers::NONE);
    assert!(h.join_end_picked().is_some());
    h.frame(vec![key(Key::Escape)]);
    assert!(h.join_end_picked().is_none(), "the end let go");
    assert!(h.joining_ends(), "the tool still open");
}

#[test]
fn the_right_click_menu_turns_vectors_into_construction_lines_and_back() {
    let mut h = Harness::new();
    h.rectangle(10.0, 10.0, 50.0, 30.0);
    h.choose_on_selection_menu(&["Make Construction Line"]);
    let id = *h.doc().vectors.keys().next().unwrap();
    assert!(h.doc().is_construction(id));
    // all of the selection is construction lines now: the item turns them back
    h.choose_on_selection_menu(&["Make Real Line"]);
    assert!(!h.doc().is_construction(id));
    h.invoke("undo");
    assert!(h.doc().is_construction(id), "undone, it is one again");
}

#[test]
fn hiding_construction_lines_lets_go_of_them_and_showing_brings_them_back() {
    let mut h = Harness::new();
    h.rectangle(10.0, 10.0, 50.0, 30.0);
    h.invoke("make_construction_line");
    let id = *h.doc().vectors.keys().next().unwrap();
    assert!(h.display().construction, "shown to start with");
    h.invoke("show_construction_lines");
    assert_eq!(h.display().checked("show_construction_lines"), Some(false));
    assert!(!h.doc().shown(id) && !h.doc().editable(id));
    assert!(
        h.doc().selection.is_empty(),
        "a hidden line is not left picked"
    );
    // a job opened while they are hidden keeps them hidden
    h.given_document(|d| d.hide_construction = false);
    h.frame(vec![]);
    assert!(h.doc().hide_construction);
    h.invoke("show_construction_lines");
    assert!(h.doc().shown(id));
}

#[test]
fn show_open_vector_ends_is_a_view_menu_switch() {
    let mut h = Harness::new();
    assert!(!h.display().open_ends, "off to start with");
    h.invoke("show_open_vector_ends");
    assert!(h.display().open_ends);
    assert_eq!(h.display().checked("show_open_vector_ends"), Some(true));
}

#[test]
fn freehands_stabilizing_circle_is_drawn_round_the_paths_end_while_dragging() {
    let mut h = Harness::new();
    h.invoke("draw_freehand");
    h.edit_form(|p| p["stabilize"] = Value::from(2.0));
    h.given_stroke(vec![Point::new(0.0, 0.0), Point::new(10.0, 0.0)]);
    let drag = cut_craft_core::Drag {
        start: Point::new(0.0, 0.0),
        end: Point::new(10.0, 0.0),
        from_center: false,
        constrain: false,
    };
    let paths = h.drawing_preview("draw_freehand", drag);
    assert_eq!(paths.len(), 2, "the path and the circle");
    let b = paths[1].bounds();
    assert!((b.center() - Point::new(8.0, 0.0)).length() < 1e-6, "{b:?}");
    assert!((b.size().x - 4.0).abs() < 1e-6, "{b:?}");
}

#[test]
fn a_click_between_two_letters_with_draw_text_open_pulls_them_together() {
    let mut h = Harness::new();
    h.invoke("draw_text");
    let mut params = h.form_settings().clone();
    params["text"] = "HI".into();
    params["font"] = "Test Boxes".into();
    params["height"] = 10.0.into();
    params["anchor"] = serde_json::json!({ "x": 10.0, "y": 10.0 });
    h.edit_form(|p| *p = params.clone());
    h.apply_form("draw_text", &params).unwrap();
    let width = |h: &Harness| {
        h.doc()
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
    h.text_click(Point::new(19.0, 15.0), 1.0, Modifiers::NONE);
    assert_eq!(
        h.form_settings()["spacing"]["kerning"][0],
        serde_json::json!([0, 0, -0.5])
    );
    assert!(
        (width(&h) - (before - 0.5)).abs() < 1e-6,
        "the letters followed at once: {}",
        width(&h)
    );
    // Shift and Ctrl push them apart by four steps; a click off the text places the anchor
    h.text_click(
        Point::new(18.5, 15.0),
        1.0,
        Modifiers::SHIFT | Modifiers::CTRL,
    );
    assert_eq!(
        h.form_settings()["spacing"]["kerning"][0],
        serde_json::json!([0, 0, 1.5])
    );
    h.text_click(Point::new(80.0, 80.0), 1.0, Modifiers::NONE);
    assert_eq!(
        h.form_settings()["anchor"],
        serde_json::json!({ "x": 80.0, "y": 80.0 })
    );
}

#[test]
fn dragging_the_green_handle_over_edited_text_bends_it_into_an_arc() {
    let mut h = Harness::new();
    h.invoke("draw_text");
    let mut params = h.form_settings().clone();
    params["text"] = "HIHIH".into();
    params["font"] = "Test Boxes".into();
    params["height"] = 10.0.into();
    params["anchor"] = serde_json::json!({ "x": 10.0, "y": 10.0 });
    h.edit_form(|p| *p = params.clone());
    h.apply_form("draw_text", &params).unwrap();
    h.frame(vec![]);
    let p: cmd_draw_text::Params = serde_json::from_value(h.form_settings().clone()).unwrap();
    let [top, _] = cmd_draw_text::arc_handles(&p).expect("handles on the edited text");
    let drag = Drag {
        start: top,
        end: top + Point::new(0.0, 5.0),
        from_center: false,
        constrain: false,
    };
    h.drawing_gesture(
        "draw_text",
        Some(view2d::Gesture::Dropped { drag, shift: false }),
        1.0,
        Modifiers::NONE,
    );
    let arc = h.form_value("arc").as_f64().unwrap();
    assert!(arc > 10.0, "bent up: {arc}");
    let low = h
        .doc()
        .vectors
        .values()
        .map(|v| v.bounds().min.y)
        .fold(f64::INFINITY, f64::min);
    assert!(low < 9.0, "the ends dropped: {low}");
    assert_eq!(h.doc().vectors.len(), 5, "the letters replaced");
}

#[test]
fn h_and_v_make_mirror_lines_and_the_arrows_move_picked_nodes_mirrored() {
    use cut_craft_core::geometry::Rect as JobRect;
    use cut_craft_core::nodes::nodes;
    let mut h = Harness::new();
    h.rectangle(10.0, 10.0, 20.0, 20.0);
    h.invoke("node_edit");
    assert_eq!(h.view_tool(), Some(ViewTool::NodeEdit));
    let id = *h.doc().vectors.keys().next().unwrap();
    // the two top nodes picked, as a box round them picks them
    let top = JobRect::new(Point::new(5.0, 25.0), Point::new(35.0, 35.0));
    h.pick_nodes_in(top);
    assert_eq!(h.picked_nodes().len(), 2);
    h.frame(vec![key(Key::H)]);
    assert_eq!(
        h.node_mirror(),
        [true, false],
        "H: a mirror line, not Mirror"
    );
    let step = h.options().snapping.grid_spacing;
    // the arrows move the two sides opposite ways: Left takes the left node left and the
    // right node right
    h.frame(vec![key(Key::ArrowLeft)]);
    let path = &h.doc().vectors[&id];
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
        h.picked_nodes()
            .iter()
            .all(|n| (n.at.x - 10.0 + step).abs() < 1e-9 || (n.at.x - 30.0 - step).abs() < 1e-9),
        "the picks follow"
    );
    // V adds the other line, H again takes its line off; the overlay draws what is on
    h.frame(vec![key(Key::V)]);
    h.frame(vec![key(Key::H)]);
    assert_eq!(h.node_mirror(), [false, true]);
    match h.node_overlay(None, 1.0) {
        crate::view2d::Overlay::Nodes { moving, .. } => assert_eq!(moving.len(), 1, "one line"),
        _ => panic!("the node overlay"),
    }
    // leaving node editing takes the lines away
    h.invoke("node_edit");
    assert_eq!(h.node_mirror(), [false, false]);
}

#[test]
fn picked_nodes_are_let_go_when_the_selection_changes_or_an_undo_moves_them() {
    use cut_craft_core::geometry::Rect as JobRect;
    let mut h = Harness::new();
    h.rectangle(10.0, 10.0, 20.0, 20.0);
    let first = *h.doc().vectors.keys().next().unwrap();
    h.rectangle(100.0, 10.0, 20.0, 20.0);
    let second = *h.doc().vectors.keys().last().unwrap();
    h.select([first]);
    h.invoke("node_edit");
    let top = JobRect::new(Point::new(5.0, 25.0), Point::new(35.0, 35.0));
    h.pick_nodes_in(top);
    assert_eq!(h.picked_nodes().len(), 2);
    // another vector picked: the arrows move it, not the first one's nodes
    h.select([second]);
    let before = h.doc().vectors[&first].clone();
    h.frame(vec![key(Key::ArrowLeft)]);
    assert_eq!(h.doc().vectors[&first], before, "the first one stays");
    assert!(h.picked_nodes().is_empty());
    // picked again, nudged, then undone: the picks are where no node is any more
    h.select([first]);
    h.pick_nodes_in(top);
    h.frame(vec![key(Key::ArrowLeft)]);
    h.invoke("undo");
    h.frame(vec![]);
    assert!(h.picked_nodes().is_empty(), "{:?}", h.picked_nodes());
    // and leaving node editing lets them go
    h.pick_nodes_in(top);
    h.invoke("node_edit");
    h.frame(vec![]);
    assert!(h.picked_nodes().is_empty());
}

#[test]
fn draw_text_lists_words_spelt_wrong_with_suggestions_and_keeps_added_words() {
    let mut h = Harness::new();
    let aff = "TRY esianrtolcdugmphbyfvkwz\nSFX S Y 1\nSFX S 0 s .\n";
    h.given_speller(cut_craft_core::spell::Speller::from_hunspell(
        aff,
        "2\nwood/S\nsign\n",
    ));
    h.invoke("draw_text");
    h.edit_form(|p| p["text"] = "Woood sign Zed".into());
    h.given_job_screen();
    h.pin(Page::Design);
    h.frame(vec![]);
    h.frame(vec![]);
    assert!(h.text_at("Spelling").is_some(), "the spelling section");
    assert!(h.text_at("Woood").is_some(), "the word spelt wrong");
    assert!(h.text_at("Wood").is_some(), "its suggestion");
    h.fix_spelling("Woood", "Wood");
    assert_eq!(h.form_value("text"), "Wood sign Zed");
    h.user_word("Zed", true);
    let s = h.speller();
    assert_eq!(s.misspelled("Wood sign Zed"), Vec::<String>::new());
    h.user_word("Zed", false);
    assert_eq!(h.speller().misspelled("Zed"), ["Zed"]);
}

#[test]
fn gadgets_ends_with_add_ons_which_opens_its_window() {
    let mut h = Harness::new();
    let gadgets = h.menu_items("Gadgets");
    let last = gadgets.last().unwrap();
    assert_eq!(
        (last.name.as_str(), last.action.as_str()),
        ("Add-ons...", "addons")
    );
    assert!(h.available("addons"));
    h.invoke("addons");
    assert!(h.addons_open());
}

#[test]
fn a_height_handle_drag_applies_its_height_at_most_ten_times_a_second() {
    let mut h = Harness::new();
    h.open_document(cut_craft_core::testing::domed());
    h.select_components([EntityId(1)]);
    h.given_orbit(view3d::Orbit::TOP);
    h.show_view(View::ThreeD);
    h.invoke("transform");
    h.frame(vec![]);
    h.frame(vec![]);
    let from = h
        .handles_3d()
        .iter()
        .find(|(k, _)| *k == Handle3d::Height)
        .map(|(_, at)| *at)
        .expect("the height handle");
    let quarter = h.fit_3d().height() / VIEW_ZOOM_QUARTERS;
    h.frame(vec![Event::PointerMoved(from)]);
    h.press(from, true, Modifiers::NONE);
    // ten moves a fiftieth of a second apart: a fifth of a second in all
    let mut revisions = std::collections::BTreeSet::new();
    for i in 1..=10u8 {
        h.time -= 0.08;
        let to = from - vec2(0.0, quarter * f32::from(i) / 10.0);
        h.frame(vec![Event::PointerMoved(to)]);
        revisions.insert(h.job_revision());
    }
    assert!(
        revisions.len() <= 3,
        "applied {} times in a fifth of a second",
        revisions.len()
    );
    let to = from - vec2(0.0, quarter);
    h.press(to, false, Modifiers::NONE);
    h.frame(vec![]);
    let height = h.doc().components[&EntityId(1)].shaping.shape_height;
    assert!(
        (height - 200.0).abs() < 5.0,
        "the last height is applied: {height}"
    );
}

#[test]
fn the_status_bar_stays_one_line_however_long_its_text() {
    let mut h = Harness::new();
    h.given_status("Ready");
    h.frame(vec![]);
    h.frame(vec![]);
    let version = concat!("v", env!("CARGO_PKG_VERSION"));
    let short = h.text_at(version).expect("the version");
    // a Library tile's hint: its name, then on a new line what can be done with it
    let long = format!(
        "0331. Flowers.STL\nDouble-click to add it to the project, or drag it onto a view. {}",
        "Click to pick it (Ctrl or Shift for more), right-click for more. ".repeat(6)
    );
    h.given_status(&long);
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

/// Every web link (Help > Community, the update window's links and download button, links
/// in help pages) asks the window to open it; the window only does so when eframe is built
/// with its "links" part. Without it each click was silently dropped (#609).
#[test]
fn the_window_is_built_to_open_web_links() {
    let manifest = include_str!("../../../../../Cargo.toml");
    let eframe = manifest
        .lines()
        .find(|l| l.trim_start().starts_with("eframe ="))
        .expect("the workspace names eframe");
    assert!(
        eframe.contains("\"links\""),
        "eframe needs the \"links\" feature, or no link opens a browser: {eframe}"
    );
}

#[test]
fn a_drawing_imports_along_the_selected_open_line_and_its_window_moves_it_below() {
    use cut_craft_core::geometry::{Path, Point, Rect};
    let mut h = Harness::new();
    // a straight open line 200 long, selected
    let mut c = Change::builder(h.doc());
    let line = c.add(Path::polyline(
        &[Point::new(0.0, 100.0), Point::new(200.0, 100.0)],
        false,
    ));
    c.select([line]);
    h.make(c.finish(), "line");
    assert!(super::line_to_import_along(h.doc()));
    let before = h.undo_steps();
    let svg = std::env::temp_dir().join(format!("cut-craft-along-{}.svg", std::process::id()));
    std::fs::write(
        &svg,
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="20mm" height="10mm" viewBox="0 0 20 10"><rect x="0" y="0" width="20" height="10"/></svg>"#,
    )
    .unwrap();
    h.import_into(&svg, line);
    h.frame(vec![]);
    h.frame(vec![]);
    let drawn = |h: &Harness| {
        h.doc()
            .vectors
            .iter()
            .filter(|(id, _)| **id != line)
            .map(|(_, p)| p.bounds())
            .reduce(Rect::union)
            .expect("the drawing came in")
    };
    let b = drawn(&h);
    // runs the line's length, proportions kept, standing on it
    assert!(
        (b.min.x).abs() < 1e-6 && (b.max.x - 200.0).abs() < 1e-6,
        "{b:?}"
    );
    assert!(
        (b.min.y - 100.0).abs() < 1e-6 && (b.max.y - 200.0).abs() < 1e-6,
        "{b:?}"
    );
    assert!(
        h.text_at("Import Along Line").is_some(),
        "its window is open"
    );
    h.fit_import_side(cut_craft_core::bend::Side::Below);
    let b = drawn(&h);
    assert!(
        (b.max.y - 100.0).abs() < 1e-6 && (b.min.y).abs() < 1e-6,
        "{b:?}"
    );
    assert_eq!(h.undo_steps(), before + 1, "still one step");
    let _ = std::fs::remove_file(svg);
}

/// Polygons, stars and ellipses open in their drawing tool as circles and rectangles do, and
/// a square opens in whichever of Draw Rectangle or Draw Polygon is picked; E picks Draw
/// Rectangle for it.
#[test]
fn polygons_stars_and_ellipses_open_in_their_drawing_tools() {
    let shapes = [
        (
            "draw_polygon",
            serde_json::json!({ "center": { "x": 40.0, "y": 40.0 }, "radius": 15.0, "sides": 5, "rotation": 10.0 }),
            "sides",
            5.0,
        ),
        (
            "draw_star",
            serde_json::json!({ "center": { "x": 40.0, "y": 40.0 }, "outer_radius": 15.0, "inner_radius": 6.0, "points": 6, "rotation": 0.0 }),
            "points",
            6.0,
        ),
        (
            "draw_ellipse",
            serde_json::json!({ "center": { "x": 40.0, "y": 40.0 }, "width": 30.0, "height": 12.0 }),
            "height",
            12.0,
        ),
    ];
    for (id, params, key, want) in shapes {
        let mut h = Harness::new();
        h.run(id, &params).unwrap();
        let shape = *h.doc().vectors.keys().next().unwrap();
        h.invoke(id);
        h.frame(vec![]);
        assert_eq!(h.form_drawn(), Some(shape), "{id} edits the selected shape");
        assert_eq!(h.form_settings()[key].as_f64(), Some(want), "{id}");
        assert!(h.text_at("Create").is_none(), "{id}");
        // changed on the form, the same shape changes
        h.edit_form(|p| p["center"]["x"] = Value::from(60.0));
        h.touch_form();
        h.frame(vec![]);
        assert_eq!(h.doc().vectors.len(), 1, "{id}: no second shape");
        let middle = h.doc().vectors.values().next().unwrap().bounds().center();
        assert!((middle.y - 40.0).abs() < 1.0, "{id}: {middle:?}");
        // E opens the same tool on it
        h.close_form();
        assert_eq!(h.object_to_edit().map(|(_, c, _)| c), Some(id));
    }
    // a square: Draw Polygon reads it as a four-sided polygon, E as the rectangle it is
    let mut h = Harness::new();
    h.rectangle(10.0, 10.0, 20.0, 20.0);
    assert_eq!(
        h.object_to_edit().map(|(_, c, _)| c),
        Some("draw_rectangle")
    );
    h.invoke("draw_polygon");
    h.frame(vec![]);
    assert!(h.form_drawn().is_some(), "Draw Polygon edits the square");
    assert_eq!(h.form_settings()["sides"].as_u64(), Some(4));
    assert_eq!(
        h.form_settings()["rotation"].as_f64(),
        Some(135.0),
        "a whole angle"
    );
}

#[test]
fn del_in_node_edit_deletes_the_picked_nodes_not_the_shape() {
    use cut_craft_core::geometry::Rect as JobRect;
    use cut_craft_core::nodes::nodes;
    let mut h = Harness::new();
    h.rectangle(10.0, 10.0, 20.0, 20.0);
    h.invoke("node_edit");
    let id = *h.doc().vectors.keys().next().unwrap();
    // the top right corner picked, as a box round it picks it
    let corner = JobRect::new(Point::new(25.0, 25.0), Point::new(35.0, 35.0));
    h.pick_nodes_in(corner);
    assert_eq!(h.picked_nodes().len(), 1);
    h.frame(vec![key(Key::Delete)]);
    let path = h.doc().vectors.get(&id).expect("the shape is kept");
    assert_eq!(nodes(path).len(), 3, "the picked corner went");
    assert!(h.picked_nodes().is_empty());
    // two more picked: the shape can lose no more than it can, and says so
    let all = JobRect::new(Point::new(0.0, 0.0), Point::new(40.0, 40.0));
    h.pick_nodes_in(all);
    h.frame(vec![key(Key::Delete)]);
    let path = h.doc().vectors.get(&id).expect("still kept");
    assert_eq!(nodes(path).len(), 3, "a closed shape keeps three nodes");
    assert!(
        h.status_message().contains("needs"),
        "{}",
        h.status_message()
    );
}
