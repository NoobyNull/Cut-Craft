//! Snapping, guides, rulers, measuring and dimensions.

use super::*;

#[test]
fn drawing_snaps_to_nearby_corners() {
    let mut h = Harness::new();
    h.rectangle(10.0, 10.0, 20.0, 10.0);
    h.app.invoke(&h.ctx, "draw_circle");
    h.app.invoke(&h.ctx, "snap_options");
    assert!(h.app.snap.open);
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
fn a_click_that_draws_nothing_does_not_tie_a_typed_in_circle_to_its_snap() {
    let mut h = Harness::new();
    h.rectangle(10.0, 10.0, 20.0, 10.0);
    h.app.invoke(&h.ctx, "draw_circle");
    h.frame(vec![]);
    // a plain click on the rectangle's corner: a circle needs a drag, so nothing is drawn
    h.click(h.at(30.0, 20.0), Modifiers::NONE);
    assert_eq!(h.app.doc.vectors.len(), 1);
    let circle = serde_json::json!({ "center": { "x": 200.0, "y": 150.0 }, "radius": 20.0 });
    h.app.apply("draw_circle", &circle).unwrap();
    assert_eq!(h.app.doc.vectors.len(), 2);
    assert_eq!(h.app.doc.constraints, [], "not tied to the far corner");
}

#[test]
fn a_star_drawn_from_a_snapped_circle_centre_follows_the_circle() {
    let mut h = Harness::new();
    let circle = serde_json::json!({ "center": { "x": 50.0, "y": 50.0 }, "radius": 10.0 });
    h.app.apply("draw_circle", &circle).unwrap();
    h.app.form = None;
    let circle = *h.app.doc.vectors.keys().next().unwrap();
    h.app.invoke(&h.ctx, "draw_star");
    h.frame(vec![]);
    let near = 2.0 / f64::from(h.app.view_transform.unwrap().scale);
    // a five-point star: its middle is not the middle of its box
    h.drag(h.at(50.0 + near, 50.0 - near), h.at(50.0, 80.0));
    h.app.form = None;
    h.frame(vec![]);
    assert_eq!(h.app.doc.vectors.len(), 2, "a star is drawn");
    let star = *h.app.doc.vectors.keys().last().unwrap();
    assert_eq!(
        h.app.doc.constraints.len(),
        1,
        "tied to the circle's centre"
    );
    let middle = |h: &Harness| {
        let points = cut_craft_core::nodes::nodes(&h.app.doc.vectors[&star]);
        let sum = points.iter().fold(Point::new(0.0, 0.0), |a, &b| a + b);
        #[expect(clippy::cast_precision_loss, reason = "a star's ten nodes")]
        let count = points.len() as f64;
        sum / count
    };
    assert!(middle(&h).distance(Point::new(50.0, 50.0)) < 1e-6);
    move_vector(&mut h, circle, Point::new(5.0, 5.0));
    assert!(
        middle(&h).distance(Point::new(55.0, 55.0)) < 1e-6,
        "the star followed the circle: {:?}",
        middle(&h)
    );
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
    assert_eq!(h.app.picked_snap(), Some(0));
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
fn a_picked_snap_mark_is_let_go_when_the_selection_changes() {
    let mut h = Harness::new();
    h.rectangle(10.0, 10.0, 20.0, 10.0);
    h.app.invoke(&h.ctx, "draw_polyline");
    h.frame(vec![]);
    let scale = f64::from(h.app.view_transform.unwrap().scale);
    let near = 2.0 / scale;
    h.click(h.at(60.0, 60.0), Modifiers::SHIFT);
    h.click(h.at(30.0 + near, 20.0 - near), Modifiers::NONE);
    h.app.finish_polyline(false);
    h.app.form = None;
    h.app.invoke(&h.ctx, "select");
    h.app.doc.selection.clear();
    h.frame(vec![]);
    let line = *h.app.doc.vectors.keys().last().unwrap();
    h.click(h.at(30.0, 20.0), Modifiers::NONE);
    assert_eq!(h.app.picked_snap(), Some(0));
    assert!(
        h.app.doc.selection.contains(&line),
        "the mark's vector is picked too"
    );
    // everything picked another way: Delete deletes, not lets the snap go
    h.app.invoke(&h.ctx, "select_all_vectors");
    h.frame(vec![]);
    assert_eq!(h.app.picked_snap(), None);
    h.app.invoke(&h.ctx, "delete");
    assert!(h.app.doc.vectors.is_empty(), "{}", h.app.status.message);
    // undone: still nothing picked
    h.app.invoke(&h.ctx, "undo");
    h.frame(vec![]);
    assert_eq!(h.app.picked_snap(), None);
}

#[test]
fn a_picked_snap_mark_is_let_go_by_undo_then_redo() {
    let mut h = Harness::new();
    h.rectangle(10.0, 10.0, 20.0, 10.0);
    h.app.invoke(&h.ctx, "draw_polyline");
    h.frame(vec![]);
    let near = 2.0 / f64::from(h.app.view_transform.unwrap().scale);
    h.click(h.at(60.0, 60.0), Modifiers::SHIFT);
    h.click(h.at(30.0 + near, 20.0 - near), Modifiers::NONE);
    h.app.finish_polyline(false);
    h.app.form = None;
    h.app.invoke(&h.ctx, "select");
    h.app.doc.selection.clear();
    h.frame(vec![]);
    h.click(h.at(30.0, 20.0), Modifiers::NONE);
    assert_eq!(h.app.picked_snap(), Some(0));
    let picked_with = h.app.doc.selection.clone();
    // undone and redone: the history is as deep as it was, and the same things are picked
    h.app.invoke(&h.ctx, "undo");
    h.frame(vec![]);
    h.app.invoke(&h.ctx, "redo");
    h.frame(vec![]);
    h.app.doc.selection = picked_with;
    assert_eq!(h.app.picked_snap(), None, "the mark is no longer picked");
    h.app.invoke(&h.ctx, "delete");
    assert_eq!(
        h.app.doc.vectors.len(),
        1,
        "Delete deletes the selection: {}",
        h.app.status.message
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
    // moved on its own, it goes where it is put and lets go of the corner
    h.app.doc.selection = [second].into();
    let params =
        serde_json::json!({ "relative": true, "x": 50.0, "y": 0.0, "anchor": "bottom_left" });
    h.app.apply("move", &params).unwrap();
    let put = h.app.doc.vectors[&second].bounds();
    assert!(
        (put.min.x - after.min.x - 50.0).abs() < 1e-9,
        "not sprung back"
    );
    assert_eq!(h.app.doc.constraints, []);
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
fn a_handle_beside_its_corner_drags_with_snapping_on_and_one_on_its_corner_moves_the_corner() {
    use cut_craft_core::geometry::Segment;
    use cut_craft_core::nodes::{handles, move_handle, nodes, span_to_bezier};
    let mut h = Harness::new();
    h.rectangle(10.0, 10.0, 20.0, 10.0);
    let id = *h.app.doc.selection.iter().next().unwrap();
    h.app.invoke(&h.ctx, "node_edit");
    // the bottom side a curve, the handle at its left pulled in close beside the corner,
    // well inside the snapping radius of the corner (snapping is on, as it is by default)
    let path = h.app.doc.vectors[&id].clone();
    let k = cut_craft_core::nodes::span_near(&path, Point::new(20.0, 10.0), 1.0).unwrap();
    let curved = span_to_bezier(&path, k);
    let left = handles(&curved)
        .into_iter()
        .find(|g| g.span == k && g.node.x < 15.0)
        .unwrap();
    let beside = left.node + Point::new(0.8, 0.0);
    h.app
        .doc
        .vectors
        .insert(id, move_handle(&curved, left.span, left.second, beside));
    let corners = nodes(&h.app.doc.vectors[&id]);
    // let go well away from anything the end could snap to
    h.drag(
        h.at(beside.x, beside.y),
        h.at(beside.x + 4.0, beside.y - 15.0),
    );
    let path = &h.app.doc.vectors[&id];
    assert_eq!(
        nodes(path),
        corners,
        "the press on the handle did not snap to the corner"
    );
    assert!(
        handles(path).iter().any(|g| g.at.y < 0.0),
        "the handle went down: {:?}",
        path.segments
    );
    // a handle lying on its corner: the corner wins
    let on = handles(path).into_iter().find(|g| g.at.y < 0.0).unwrap();
    let flat = move_handle(path, on.span, on.second, on.node);
    h.app.doc.vectors.insert(id, flat);
    h.app.options.snapping.smart_snapping = false;
    h.drag(
        h.at(on.node.x, on.node.y),
        h.at(on.node.x + 20.0, on.node.y - 25.0),
    );
    let moved = nodes(&h.app.doc.vectors[&id]);
    assert!(
        moved
            .iter()
            .any(|q| q.distance(on.node + Point::new(20.0, -25.0)) < 0.5),
        "the corner moved: {moved:?}"
    );
    assert!(matches!(
        h.app.doc.vectors[&id].segments[on.span],
        Segment::Cubic { .. }
    ));
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
fn measure_snaps_both_ends_to_vector_corners() {
    let mut h = Harness::new();
    h.rectangle(10.0, 10.0, 30.0, 40.0);
    h.app.options.snapping.smart_snapping = false;
    h.app.invoke(&h.ctx, "measure_inspect");
    h.frame(vec![]);
    let near = 2.0 / f64::from(h.app.view_transform.unwrap().scale);
    // two pixels off each corner, the reading is corner to corner
    h.drag(
        h.at(10.0 + near, 10.0 - near),
        h.at(40.0 - near, 50.0 + near),
    );
    assert!(
        h.app.status.message.starts_with("Distance 50.000 mm"),
        "{}",
        h.app.status.message
    );
    // Shift places the ends freely
    let (from, to) = (
        h.at(10.0 + near, 10.0 - near),
        h.at(40.0 - near, 50.0 + near),
    );
    let shift = Modifiers::SHIFT;
    h.frame_holding(shift, vec![Event::PointerMoved(from)]);
    h.press(from, true, shift);
    h.frame_holding(shift, vec![Event::PointerMoved(from.lerp(to, 0.5))]);
    h.frame_holding(shift, vec![Event::PointerMoved(to)]);
    h.press(to, false, shift);
    assert!(
        !h.app.status.message.starts_with("Distance 50.000 mm"),
        "{}",
        h.app.status.message
    );
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
fn a_drag_from_a_vector_lying_on_a_guide_moves_the_vector_not_the_guide() {
    let mut h = Harness::new();
    h.rectangle(100.0, 10.0, 40.0, 40.0);
    h.app.doc.selection.clear();
    h.app.doc.guides = vec![cut_craft_core::document::Guide::vertical(100.0)];
    h.frame(vec![]);
    let id = *h.app.doc.vectors.keys().next().unwrap();
    // from the rectangle's left side, which lies on the guide
    h.drag(h.at(100.0, 30.0), h.at(120.0, 30.0));
    assert!(
        (h.app.doc.guides[0].at - 100.0).abs() < 1e-9,
        "the guide stays"
    );
    let b = h.app.doc.vectors[&id].bounds();
    assert!((b.min.x - 120.0).abs() < 1.0, "the rectangle moved: {b:?}");
    // away from anything else, the guide is still dragged
    h.app.doc.selection.clear();
    h.frame(vec![]);
    h.drag(h.at(100.0, 150.0), h.at(80.0, 150.0));
    assert!(
        (h.app.doc.guides[0].at - 80.0).abs() < 1.0,
        "{:?}",
        h.app.doc.guides
    );
}

#[test]
fn right_clicking_a_guide_or_a_node_opens_its_own_menu() {
    let mut h = Harness::new();
    h.app.doc.guides = vec![cut_craft_core::document::Guide::vertical(150.0)];
    h.frame(vec![]);
    h.right_click(h.at(150.0, 120.0));
    assert!(
        h.text_at("Delete Guide").is_some(),
        "the guide's menu opens"
    );
    let mut h = Harness::new();
    h.rectangle(10.0, 10.0, 20.0, 10.0);
    h.app.invoke(&h.ctx, "node_edit");
    h.frame(vec![]);
    h.right_click(h.at(30.0, 20.0));
    assert!(h.text_at("Delete Point").is_some(), "the node's menu opens");
    // a guide inserted from the menu goes where it opened, not the middle of the job
    let mut h = Harness::new();
    h.right_click(h.at(70.0, 40.0));
    let insert = h.text_at("Guidelines").expect("the selection menu is open");
    h.click(insert, Modifiers::NONE);
    let vertical = h
        .text_at("Insert Vertical Guide")
        .expect("its submenu is open");
    h.click(vertical, Modifiers::NONE);
    let guide = h.app.doc.guides.last().expect("a guide is inserted");
    assert!((guide.at - 70.0).abs() < 1.0, "{guide:?}");
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
fn guide_positions_show_in_the_jobs_units() {
    use cut_craft_core::document::{Guide, Units};
    let mut h = Harness::new();
    h.app.doc.guides.push(Guide::vertical(50.8));
    h.app.doc.job.units = Units::Inch;
    h.app.guide_props = Some(GuideProps {
        index: 0,
        position: 50.8,
        angle: 90.0,
        locked: false,
        new_at: 25.4,
        relative: true,
        count: 1,
    });
    h.frame(vec![]);
    h.frame(vec![]);
    let texts = h.texts();
    assert!(shows_number(&texts, "2.000", " in"), "{texts:?}");
    assert!(shows_number(&texts, "1.000", " in"), "{texts:?}");
    h.app.doc.job.units = Units::Mm;
    h.frame(vec![]);
    let texts = h.texts();
    assert!(shows_number(&texts, "50.8", " mm"), "{texts:?}");
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

/// Untick (or tick) Smart snapping in the open Snapping Options, as its tick box does.
fn toggle_smart_snapping(h: &mut Harness) {
    let mut edited = h.app.snap.shown(&h.app.options.snapping).clone();
    edited.smart_snapping = !edited.smart_snapping;
    h.app.snap.edited(&h.app.options.snapping, edited);
    h.frame(vec![]);
}

#[test]
fn snapping_options_changes_wait_for_apply_and_close_asks_first() {
    let mut h = Harness::new();
    h.app.options.snapping.smart_snapping = true;
    h.app.snap.open = true;
    h.frame(vec![]);
    h.frame(vec![]);
    assert!(h.text_at(super::options_apply::ALL_APPLIED).is_some());
    toggle_smart_snapping(&mut h);
    h.frame(vec![]);
    assert!(h.app.options.snapping.smart_snapping, "not used yet");
    assert!(h.text_at(super::options_apply::NOT_APPLIED).is_some());
    // Apply uses it and leaves the window open
    click_button(&mut h, "Apply");
    assert!(!h.app.options.snapping.smart_snapping, "used now");
    assert!(h.app.snap.open);
    h.frame(vec![]);
    assert!(h.text_at(super::options_apply::ALL_APPLIED).is_some());
    // changed back, Close asks; Discard Changes keeps it as applied
    toggle_smart_snapping(&mut h);
    click_button(&mut h, "Close");
    h.frame(vec![]);
    assert!(h.text_at(super::options_apply::ASK).is_some());
    click_button(&mut h, "Discard Changes");
    assert!(!h.app.snap.open);
    assert!(!h.app.options.snapping.smart_snapping);
    // Cancel throws changes away too
    h.app.snap.open = true;
    h.frame(vec![]);
    h.frame(vec![]);
    toggle_smart_snapping(&mut h);
    click_button(&mut h, "Cancel");
    assert!(!h.app.snap.open);
    assert!(!h.app.options.snapping.smart_snapping);
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
