//! Snapping, guides, rulers, measuring and dimensions.

use super::*;

#[test]
fn drawing_snaps_to_nearby_corners() {
    let mut h = Harness::new();
    h.rectangle(10.0, 10.0, 20.0, 10.0);
    h.invoke("draw_circle");
    h.invoke("snap_options");
    assert!(h.snap_options_open());
    h.frame(vec![]);
    let scale = h.zoom_2d();
    let near = 2.0 / f64::from(scale); // two pixels off the corner
    let drag = Drag {
        start: Point::new(30.0 + near, 20.0 - near),
        end: Point::new(50.0, 50.0),
        from_center: false,
        constrain: false,
    };
    let snapped = h.snap_drag(drag);
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
    h.invoke("draw_circle");
    h.frame(vec![]);
    let scale = h.zoom_2d();
    let near = 2.0 / f64::from(scale); // two pixels off the side
    let drag = Drag {
        start: Point::new(50.0, 22.0),
        end: Point::new(30.0 + near, 22.0 + near),
        from_center: true,
        constrain: false,
    };
    let snapped = h.snap_drag(drag);
    let Some(Gesture::Dragging(d)) = snapped else {
        panic!("still a drag")
    };
    assert_eq!(d.end, Point::new(30.0, 22.0), "the circle touches the side");
}

#[test]
fn a_click_that_draws_nothing_does_not_tie_a_typed_in_circle_to_its_snap() {
    let mut h = Harness::new();
    h.rectangle(10.0, 10.0, 20.0, 10.0);
    h.invoke("draw_circle");
    h.frame(vec![]);
    // a plain click on the rectangle's corner: a circle needs a drag, so nothing is drawn
    h.click(h.at(30.0, 20.0), Modifiers::NONE);
    assert_eq!(h.doc().vectors.len(), 1);
    let circle = serde_json::json!({ "center": { "x": 200.0, "y": 150.0 }, "radius": 20.0 });
    h.run("draw_circle", &circle).unwrap();
    assert_eq!(h.doc().vectors.len(), 2);
    assert_eq!(h.doc().constraints, [], "not tied to the far corner");
}

#[test]
fn a_star_drawn_from_a_snapped_circle_centre_follows_the_circle() {
    let mut h = Harness::new();
    let circle = serde_json::json!({ "center": { "x": 50.0, "y": 50.0 }, "radius": 10.0 });
    h.run("draw_circle", &circle).unwrap();
    h.close_form();
    let circle = *h.doc().vectors.keys().next().unwrap();
    h.invoke("draw_star");
    h.frame(vec![]);
    let near = 2.0 / f64::from(h.zoom_2d());
    // a five-point star: its middle is not the middle of its box
    h.drag(h.at(50.0 + near, 50.0 - near), h.at(50.0, 80.0));
    h.close_form();
    h.frame(vec![]);
    assert_eq!(h.doc().vectors.len(), 2, "a star is drawn");
    let star = *h.doc().vectors.keys().last().unwrap();
    assert_eq!(h.doc().constraints.len(), 1, "tied to the circle's centre");
    let middle = |h: &Harness| {
        let points = cut_craft_core::nodes::nodes(&h.doc().vectors[&star]);
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
    h.invoke("draw_polyline");
    h.frame(vec![]);
    let scale = f64::from(h.zoom_2d());
    let near = 2.0 / scale; // two pixels off the corner
    let corner = h.at(30.0 + near, 20.0 - near);
    // hovering: the status bar names the snap, and the pointer is a crosshair
    assert_eq!(h.hover(corner), egui::CursorIcon::Crosshair);
    let hint = cut_craft_bug_report::hint::current(&h.ctx).unwrap_or_default();
    assert!(
        hint.starts_with("Draw Polyline: click the first point"),
        "{hint}"
    );
    assert!(hint.contains("snapped to End"), "{hint}");
    h.click(corner, Modifiers::NONE);
    assert_eq!(h.polyline_drawn(), vec![Point::new(30.0, 20.0)]);
    // Shift places it freely
    let other = h.at(10.0 + near, 10.0 + near);
    h.click(other, Modifiers::SHIFT);
    let last = *h.polyline_drawn().last().unwrap();
    assert!(last.distance(Point::new(10.0, 10.0)) > 1e-6, "{last:?}");
}

/// Moves vector `id` by `by`, as the Move tool would.
fn move_vector(h: &mut Harness, id: EntityId, by: Point) {
    h.select([id]);
    let change = Change::transform_selected(h.doc(), |_| {
        cut_craft_core::glam::DAffine2::from_translation(by)
    });
    h.make(change.expect("it moves"), "Moved");
}

#[test]
fn a_line_drawn_to_a_snapped_corner_stays_on_it_until_its_mark_is_deleted() {
    let mut h = Harness::new();
    h.rectangle(10.0, 10.0, 20.0, 10.0);
    let square = *h.doc().vectors.keys().next().unwrap();
    h.invoke("draw_polyline");
    h.frame(vec![]);
    let scale = f64::from(h.zoom_2d());
    let near = 2.0 / scale;
    h.click(h.at(60.0, 60.0), Modifiers::SHIFT);
    h.click(h.at(30.0 + near, 20.0 - near), Modifiers::NONE);
    h.finish_polyline();
    h.close_form();
    h.frame(vec![]);
    let line = *h.doc().vectors.keys().last().unwrap();
    assert_eq!(h.doc().constraints.len(), 1, "the snapped end is kept");
    move_vector(&mut h, square, Point::new(5.0, 5.0));
    let end = h.doc().vectors[&line].segments[0].end();
    assert_eq!(end, Point::new(35.0, 25.0), "the end followed the corner");
    // its mark picked and deleted: the line stays, the snap goes
    h.frame(vec![]);
    h.click(h.at(35.0, 25.0), Modifiers::NONE);
    assert_eq!(h.picked_snap(), Some(0));
    h.invoke("delete");
    assert_eq!(h.doc().constraints, []);
    assert_eq!(h.doc().vectors.len(), 2, "nothing deleted");
    move_vector(&mut h, square, Point::new(5.0, 5.0));
    let end = h.doc().vectors[&line].segments[0].end();
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
    h.invoke("draw_polyline");
    h.frame(vec![]);
    let scale = f64::from(h.zoom_2d());
    let near = 2.0 / scale;
    h.click(h.at(60.0, 60.0), Modifiers::SHIFT);
    h.click(h.at(30.0 + near, 20.0 - near), Modifiers::NONE);
    h.finish_polyline();
    h.close_form();
    h.invoke("select");
    h.select([]);
    h.frame(vec![]);
    let line = *h.doc().vectors.keys().last().unwrap();
    h.click(h.at(30.0, 20.0), Modifiers::NONE);
    assert_eq!(h.picked_snap(), Some(0));
    assert!(
        h.doc().selection.contains(&line),
        "the mark's vector is picked too"
    );
    // everything picked another way: Delete deletes, not lets the snap go
    h.invoke("select_all_vectors");
    h.frame(vec![]);
    assert_eq!(h.picked_snap(), None);
    h.invoke("delete");
    assert!(h.doc().vectors.is_empty(), "{}", h.status_message());
    // undone: still nothing picked
    h.invoke("undo");
    h.frame(vec![]);
    assert_eq!(h.picked_snap(), None);
}

#[test]
fn a_picked_snap_mark_is_let_go_by_undo_then_redo() {
    let mut h = Harness::new();
    h.rectangle(10.0, 10.0, 20.0, 10.0);
    h.invoke("draw_polyline");
    h.frame(vec![]);
    let near = 2.0 / f64::from(h.zoom_2d());
    h.click(h.at(60.0, 60.0), Modifiers::SHIFT);
    h.click(h.at(30.0 + near, 20.0 - near), Modifiers::NONE);
    h.finish_polyline();
    h.close_form();
    h.invoke("select");
    h.select([]);
    h.frame(vec![]);
    h.click(h.at(30.0, 20.0), Modifiers::NONE);
    assert_eq!(h.picked_snap(), Some(0));
    let picked_with: Vec<EntityId> = h.doc().selection.iter().copied().collect();
    // undone and redone: the history is as deep as it was, and the same things are picked
    h.invoke("undo");
    h.frame(vec![]);
    h.invoke("redo");
    h.frame(vec![]);
    h.select(picked_with);
    assert_eq!(h.picked_snap(), None, "the mark is no longer picked");
    h.invoke("delete");
    assert_eq!(
        h.doc().vectors.len(),
        1,
        "Delete deletes the selection: {}",
        h.status_message()
    );
}

#[test]
fn a_rectangle_drawn_from_a_snapped_corner_moves_whole_with_it() {
    let mut h = Harness::new();
    h.rectangle(10.0, 10.0, 20.0, 10.0);
    let first = *h.doc().vectors.keys().next().unwrap();
    h.invoke("draw_rectangle");
    h.frame(vec![]);
    let scale = f64::from(h.zoom_2d());
    let near = 2.0 / scale;
    h.drag(h.at(30.0 + near, 20.0 - near), h.at(45.123, 37.456));
    h.close_form();
    h.frame(vec![]);
    let second = *h.doc().vectors.keys().last().unwrap();
    assert_ne!(first, second);
    let before = h.doc().vectors[&second].bounds();
    assert_eq!(h.doc().constraints.len(), 1, "{:?}", h.doc().constraints);
    move_vector(&mut h, first, Point::new(-4.0, 3.0));
    let after = h.doc().vectors[&second].bounds();
    assert!((after.size() - before.size()).length() < 1e-9, "same size");
    let moved = after.min - before.min;
    assert!(moved.distance(Point::new(-4.0, 3.0)) < 1e-9, "{moved:?}");
    // moved on its own, it goes where it is put and lets go of the corner
    h.select([second]);
    let params =
        serde_json::json!({ "relative": true, "x": 50.0, "y": 0.0, "anchor": "bottom_left" });
    h.run("move", &params).unwrap();
    let put = h.doc().vectors[&second].bounds();
    assert!(
        (put.min.x - after.min.x - 50.0).abs() < 1e-9,
        "not sprung back"
    );
    assert_eq!(h.doc().constraints, []);
}

#[test]
fn a_polylines_next_point_snaps_tangent_to_a_circle() {
    let mut h = Harness::new();
    let circle = serde_json::json!({ "center": { "x": 50.0, "y": 70.0 }, "radius": 10.0 });
    h.run("draw_circle", &circle).unwrap();
    h.close_form();
    h.given_options(|o| o.snapping.job_center_and_corners = false);
    h.invoke("draw_polyline");
    h.frame(vec![]);
    h.polyline_click(Point::new(70.0, 70.0));
    let hit = h.snap_at(Point::new(55.3, 61.6)).expect("a snap");
    assert_eq!(hit.kind, cut_craft_core::snap::SnapKind::Tangent);
    let tangent = Point::new(55.0, 70.0 - 75.0_f64.sqrt());
    assert!(hit.at.distance(tangent) < 1e-6, "{hit:?}");
}

#[test]
fn a_freehand_stroke_does_not_snap_square_onto_a_side() {
    let mut h = Harness::new();
    h.rectangle(10.0, 10.0, 20.0, 40.0);
    h.invoke("draw_freehand");
    h.frame(vec![]);
    let scale = h.zoom_2d();
    let near = 2.0 / f64::from(scale);
    let drag = Drag {
        start: Point::new(50.0, 22.0),
        end: Point::new(30.0 + near, 22.0 + near),
        from_center: false,
        constrain: false,
    };
    let snapped = h.snap_drag(drag);
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
    let id = *h.doc().selection.iter().next().unwrap();
    h.invoke("node_edit");
    // the bottom side a curve, the handle at its left pulled in close beside the corner,
    // well inside the snapping radius of the corner (snapping is on, as it is by default)
    let path = h.doc().vectors[&id].clone();
    let k = cut_craft_core::nodes::span_near(&path, Point::new(20.0, 10.0), 1.0).unwrap();
    let curved = span_to_bezier(&path, k);
    let left = handles(&curved)
        .into_iter()
        .find(|g| g.span == k && g.node.x < 15.0)
        .unwrap();
    let beside = left.node + Point::new(0.8, 0.0);
    h.given_document(|d| {
        d.vectors
            .insert(id, move_handle(&curved, left.span, left.second, beside));
    });
    let corners = nodes(&h.doc().vectors[&id]);
    // let go well away from anything the end could snap to
    h.drag(
        h.at(beside.x, beside.y),
        h.at(beside.x + 4.0, beside.y - 15.0),
    );
    let path = &h.doc().vectors[&id];
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
    h.given_document(|d| {
        d.vectors.insert(id, flat);
    });
    h.given_options(|o| o.snapping.smart_snapping = false);
    h.drag(
        h.at(on.node.x, on.node.y),
        h.at(on.node.x + 20.0, on.node.y - 25.0),
    );
    let moved = nodes(&h.doc().vectors[&id]);
    assert!(
        moved
            .iter()
            .any(|q| q.distance(on.node + Point::new(20.0, -25.0)) < 0.5),
        "the corner moved: {moved:?}"
    );
    assert!(matches!(
        h.doc().vectors[&id].segments[on.span],
        Segment::Cubic { .. }
    ));
}

#[test]
fn the_measure_tool_shows_its_results_in_a_form() {
    let mut h = Harness::new();
    h.invoke("measure_inspect");
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
    h.given_options(|o| o.snapping.smart_snapping = false);
    h.invoke("measure_inspect");
    h.frame(vec![]);
    let near = 2.0 / f64::from(h.zoom_2d());
    // two pixels off each corner, the reading is corner to corner
    h.drag(
        h.at(10.0 + near, 10.0 - near),
        h.at(40.0 - near, 50.0 + near),
    );
    assert!(
        h.status_message().starts_with("Distance 50.000 mm"),
        "{}",
        h.status_message()
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
        !h.status_message().starts_with("Distance 50.000 mm"),
        "{}",
        h.status_message()
    );
}

#[test]
fn guides_come_out_of_the_rulers_move_and_go_back_in() {
    let mut h = Harness::new();
    h.frame(vec![]);
    let area = h.view_2d_area();
    let ruler = h.ctx.global_style().spacing.interact_size.y / 2.0;
    let top_ruler = egui::pos2(area.center().x, area.min.y + ruler);
    h.drag(top_ruler, h.at(100.0, 80.0));
    assert_eq!(h.doc().guides.len(), 1, "a guide across the job");
    let g = h.doc().guides[0];
    assert!(!g.vertical && (g.at - 80.0).abs() < 0.5, "{g:?}");
    h.drag(h.at(150.0, 80.0), h.at(150.0, 60.0));
    assert!((h.doc().guides[0].at - 60.0).abs() < 0.5, "dragged along");
    h.invoke("lock_all_existing_guides");
    h.drag(h.at(150.0, 60.0), h.at(150.0, 40.0));
    assert!(
        (h.doc().guides[0].at - 60.0).abs() < 0.5,
        "a locked guide stays"
    );
    h.invoke("unlock_all_existing_guides");
    h.drag(h.at(150.0, 60.0), top_ruler);
    assert!(h.doc().guides.is_empty(), "back onto the ruler deletes it");
    h.given_document(|d| d.guides = vec![cut_craft_core::document::Guide::vertical(5.0)]);
    h.invoke("delete_all_guides");
    assert_eq!(h.doc().guides, [] as [cut_craft_core::document::Guide; 0]);
}

#[test]
fn a_drag_from_a_vector_lying_on_a_guide_moves_the_vector_not_the_guide() {
    let mut h = Harness::new();
    h.rectangle(100.0, 10.0, 40.0, 40.0);
    h.select([]);
    h.given_document(|d| d.guides = vec![cut_craft_core::document::Guide::vertical(100.0)]);
    h.frame(vec![]);
    let id = *h.doc().vectors.keys().next().unwrap();
    // from the rectangle's left side, which lies on the guide
    h.drag(h.at(100.0, 30.0), h.at(120.0, 30.0));
    assert!(
        (h.doc().guides[0].at - 100.0).abs() < 1e-9,
        "the guide stays"
    );
    let b = h.doc().vectors[&id].bounds();
    assert!((b.min.x - 120.0).abs() < 1.0, "the rectangle moved: {b:?}");
    // away from anything else, the guide is still dragged
    h.select([]);
    h.frame(vec![]);
    h.drag(h.at(100.0, 150.0), h.at(80.0, 150.0));
    assert!(
        (h.doc().guides[0].at - 80.0).abs() < 1.0,
        "{:?}",
        h.doc().guides
    );
}

#[test]
fn right_clicking_a_guide_or_a_node_opens_its_own_menu() {
    let mut h = Harness::new();
    h.given_document(|d| d.guides = vec![cut_craft_core::document::Guide::vertical(150.0)]);
    h.frame(vec![]);
    h.right_click(h.at(150.0, 120.0));
    assert!(
        h.text_at("Delete Guide").is_some(),
        "the guide's menu opens"
    );
    let mut h = Harness::new();
    h.rectangle(10.0, 10.0, 20.0, 10.0);
    h.invoke("node_edit");
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
    let guide = h.doc().guides.last().expect("a guide is inserted");
    assert!((guide.at - 70.0).abs() < 1.0, "{guide:?}");
}

#[test]
fn the_rotate_snap_angle_comes_from_snapping_options() {
    use cut_craft_core::handles::Handle;
    let mut h = Harness::new();
    h.open_document(cut_craft_core::testing::domed());
    h.select_components([EntityId(1)]);
    h.given_orbit(view3d::Orbit::TOP);
    h.show_view(View::ThreeD);
    h.given_options(|o| o.snapping.rotate_step = cut_craft_core::snap::RotateSnap::Ten);
    h.invoke("transform");
    h.frame(vec![]);
    h.frame(vec![]);
    let at = |h: &Harness, which: Handle| {
        h.handles_3d()
            .iter()
            .find(|(k, _)| *k == Handle3d::Transform(which))
            .map(|(_, at)| *at)
            .expect("the transform handles are drawn over the selection")
    };
    let before = h.doc().components[&EntityId(1)].extent();
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
    let turned = h.doc().components[&EntityId(1)].extent();
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
    h.given_document(|d| d.guides.push(Guide::vertical(50.8)));
    h.given_document(|d| d.job.units = Units::Inch);
    h.given_guide_props(GuideProps {
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
    h.given_document(|d| d.job.units = Units::Mm);
    h.frame(vec![]);
    let texts = h.texts();
    assert!(shows_number(&texts, "50.8", " mm"), "{texts:?}");
}

#[test]
fn the_top_bar_splits_the_views_and_toggles_snapping() {
    let mut h = Harness::new();
    h.open_document(cut_craft_core::testing::two_sheets());
    h.frame(vec![]);
    for name in ["Sheet 1", "Layer 1", "Level 1"] {
        assert!(h.text_at(name).is_some(), "{name} in the top bar");
    }
    h.invoke("split_side_by_side");
    assert_eq!(h.split(), Some(Split::SideBySide));
    h.frame(vec![]);
    h.invoke("split_side_by_side");
    assert_eq!(h.split(), None, "a second click joins them again");
    let before = h.options().snapping.clone();
    for id in [
        "toggle_geometry_snapping",
        "toggle_smart_snapping",
        "toggle_grid_snapping",
    ] {
        assert!(h.available(id), "{id}");
        h.invoke(id);
    }
    let s = &h.options().snapping;
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
    h.edit_snap_options(|s| s.smart_snapping = !s.smart_snapping);
    h.frame(vec![]);
}

#[test]
fn snapping_options_changes_wait_for_apply_and_close_asks_first() {
    let mut h = Harness::new();
    h.given_options(|o| o.snapping.smart_snapping = true);
    h.open_snap_options();
    h.frame(vec![]);
    h.frame(vec![]);
    assert!(h.text_at(super::options_apply::ALL_APPLIED).is_some());
    toggle_smart_snapping(&mut h);
    h.frame(vec![]);
    assert!(h.options().snapping.smart_snapping, "not used yet");
    assert!(h.text_at(super::options_apply::NOT_APPLIED).is_some());
    // Apply uses it and leaves the window open
    click_button(&mut h, "Apply");
    assert!(!h.options().snapping.smart_snapping, "used now");
    assert!(h.snap_options_open());
    h.frame(vec![]);
    assert!(h.text_at(super::options_apply::ALL_APPLIED).is_some());
    // changed back, Close asks; Discard Changes keeps it as applied
    toggle_smart_snapping(&mut h);
    click_button(&mut h, "Close");
    h.frame(vec![]);
    assert!(h.text_at(super::options_apply::ASK).is_some());
    click_button(&mut h, "Discard Changes");
    assert!(!h.snap_options_open());
    assert!(!h.options().snapping.smart_snapping);
    // Cancel throws changes away too
    h.open_snap_options();
    h.frame(vec![]);
    h.frame(vec![]);
    toggle_smart_snapping(&mut h);
    click_button(&mut h, "Cancel");
    assert!(!h.snap_options_open());
    assert!(!h.options().snapping.smart_snapping);
}

#[test]
fn a_shift_click_on_a_dimension_with_its_form_open_edits_it() {
    let mut h = Harness::new();
    let mut params = serde_json::to_value(cmd_dimensions::Params::default()).unwrap();
    params["font"] = "Test Boxes".into();
    params["custom_text"] = "W".into();
    h.run("dimensions", &params).unwrap();
    let group = *h.doc().texts.keys().next().unwrap();
    h.invoke("dimensions");
    // the dimension line runs from (10, 20) to (60, 20): a Shift-click on it
    h.shift_click_shape("dimensions", Point::new(35.0, 20.0));
    assert_eq!(h.form_value("replaces"), &serde_json::json!(group));
    assert_eq!(h.form_value("custom_text"), "W");
    assert!(h.form_has_shape(), "the shape it edits");
}

#[test]
fn guides_are_inserted_from_the_menu_and_their_properties_turn_lock_and_add_parallels() {
    let mut h = Harness::new();
    h.given_menu_at(Point::new(40.0, 30.0));
    for insert in ["guide:insert:v", "guide:insert:h", "guide:insert:a"] {
        assert!(h.available(insert));
        h.invoke(insert);
    }
    let guides = h.doc().guides.clone();
    assert_eq!(guides.len(), 3);
    assert!(guides[0].vertical && (guides[0].at - 40.0).abs() < 1e-9);
    assert!(!guides[1].vertical && (guides[1].at - 30.0).abs() < 1e-9);
    let angled = guides[2];
    assert!((angled.angle() - 45.0).abs() < 1e-9, "{angled:?}");
    assert!(
        angled.distance(Point::new(40.0, 30.0)) < 1e-9,
        "through where the menu opened"
    );
    assert_eq!(h.guide_props_open_on(), Some(2), "its properties open");
    // Guide Properties on the first: a new position and angle, locked
    h.given_guide_props(GuideProps {
        index: 0,
        position: 50.0,
        angle: 60.0,
        locked: true,
        new_at: 10.0,
        relative: true,
        count: 2,
    });
    h.guide_props_apply();
    let g = h.doc().guides[0];
    assert!(
        g.locked && (g.at - 50.0).abs() < 1e-9 && (g.angle() - 60.0).abs() < 1e-9,
        "{g:?}"
    );
    // two parallels, 10 and 20 to its left
    h.guide_props_create();
    assert_eq!(h.doc().guides.len(), 5);
    for (k, i) in [(1.0, 3), (2.0, 4)] {
        let made = h.doc().guides[i];
        let want = g.parallel(10.0 * k);
        assert!((made.at - want.at).abs() < 1e-9 && (made.angle() - want.angle()).abs() < 1e-9);
        assert!(!made.locked);
    }
    h.guide_props_delete();
    assert_eq!(h.doc().guides.len(), 4);
    assert!(h.guide_props_open_on().is_none(), "the window closed");
    // the menu's lock toggles
    assert!(!h.doc().guides[0].locked);
    h.invoke("guide:lock:0");
    assert!(h.doc().guides[0].locked);
    let menu = gui::guide_menu(0, true);
    assert_eq!(menu[1].name, "Unlock Guide");
}
