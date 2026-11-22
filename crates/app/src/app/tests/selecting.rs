//! Selecting and grouping: clicks, shift-clicks, stepping into groups.

use super::*;

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
    assert!(h.text_at("Create").is_some());
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
fn right_clicking_a_vector_picks_it_unless_it_is_already_selected() {
    let mut h = Harness::new();
    h.rectangle(10.0, 10.0, 40.0, 40.0);
    h.rectangle(100.0, 10.0, 40.0, 40.0);
    let ids: Vec<_> = h.app.doc.vectors.keys().copied().collect();
    h.app.doc.selection = [ids[0]].into();
    // on the edge of the other one: it is picked, so the menu's Delete acts on it
    h.right_click(h.at(100.0, 30.0));
    assert_eq!(h.app.doc.selection, [ids[1]].into());
    let delete = h.text_at("Delete").expect("the menu is open");
    h.click(delete, Modifiers::NONE);
    assert!(h.app.doc.vectors.contains_key(&ids[0]), "the other stays");
    assert!(
        !h.app.doc.vectors.contains_key(&ids[1]),
        "the one clicked goes"
    );
    // on one of several selected: the selection is kept
    h.rectangle(200.0, 10.0, 40.0, 40.0);
    h.app.invoke(&h.ctx, "select_all_vectors");
    let all = h.app.doc.selection.clone();
    h.right_click(h.at(200.0, 30.0));
    assert_eq!(h.app.doc.selection, all);
}

#[test]
fn dragging_an_unselected_grouped_vector_moves_its_whole_group() {
    let mut h = Harness::new();
    h.rectangle(10.0, 10.0, 40.0, 40.0);
    h.rectangle(100.0, 10.0, 40.0, 40.0);
    h.app.invoke(&h.ctx, "select_all_vectors");
    h.app.invoke(&h.ctx, "group");
    h.app.doc.selection.clear();
    h.frame(vec![]);
    let lefts = |h: &Harness| -> Vec<f64> {
        h.app
            .doc
            .vectors
            .values()
            .map(|v| v.bounds().min.x)
            .collect()
    };
    let before = lefts(&h);
    let member = *h.app.doc.vectors.keys().next().unwrap();
    let outlines = h.app.grabbed_outlines(super::Grab::Vector(member));
    assert_eq!(outlines.len(), 2, "the drag shows the whole group moving");
    h.drag(h.at(10.0, 30.0), h.at(30.0, 30.0));
    let after = lefts(&h);
    assert_eq!(h.app.doc.selection.len(), 2, "the whole group is picked");
    for (b, a) in before.iter().zip(&after) {
        assert!((a - b - 20.0).abs() < 1.0, "{before:?} -> {after:?}");
    }
}

#[test]
fn double_clicking_a_group_steps_inside_and_a_click_then_picks_one_item() {
    let mut h = Harness::new();
    two_grouped(&mut h);
    h.double_click(h.at(10.0, 30.0));
    assert!(h.app.doc.inside_group.is_some(), "stepped inside");
    assert_eq!(h.app.doc.selection.len(), 1, "the one rectangle clicked");
    // a drag moves that one alone, and it stays in the group
    h.click(h.at(100.0, 30.0), Modifiers::NONE);
    assert_eq!(h.app.doc.selection.len(), 1);
    let moved = h.app.doc.selection.iter().next().copied().unwrap();
    h.drag(h.at(100.0, 30.0), h.at(120.0, 30.0));
    let lefts: Vec<f64> = h
        .app
        .doc
        .vectors
        .values()
        .map(|v| v.bounds().min.x)
        .collect();
    assert!((lefts[0] - 10.0).abs() < 1.0, "the other stays: {lefts:?}");
    assert!((h.app.doc.vectors[&moved].bounds().min.x - 120.0).abs() < 1.0);
    assert_eq!(h.app.doc.group_members(moved).len(), 2, "still grouped");
    // Esc steps out: a click picks the whole group again
    h.frame(vec![key(Key::Escape)]);
    assert_eq!(h.app.doc.inside_group, None);
    h.click(h.at(10.0, 30.0), Modifiers::NONE);
    assert_eq!(h.app.doc.selection.len(), 2);
}

#[test]
fn a_shape_changed_in_its_own_tool_inside_a_group_stays_in_the_group() {
    let mut h = Harness::new();
    two_grouped(&mut h);
    h.double_click(h.at(10.0, 30.0));
    let old = *h.app.doc.selection.iter().next().unwrap();
    let (command, mut params) = h
        .app
        .registry
        .drawn_by(&h.app.doc, &h.app.doc.vectors[&old])
        .expect("a rectangle the tool drew");
    params["width"] = Value::from(60.0);
    h.app.redraw(command, &params, old).unwrap();
    let made = *h.app.doc.vectors.keys().max().unwrap();
    assert_ne!(made, old);
    assert_eq!(h.app.doc.group_members(made).len(), 2, "still in the group");
}

#[test]
fn dragging_the_move_cross_with_shift_or_ctrl_keeps_to_one_axis_in_select_mode() {
    // Ctrl as the keyboard gives it: Ctrl, and the command key on this platform
    let ctrl = Modifiers {
        ctrl: true,
        command: true,
        ..Modifiers::NONE
    };
    for held in [Modifiers::SHIFT, ctrl] {
        let mut h = Harness::new();
        h.rectangle(10.0, 10.0, 40.0, 40.0);
        h.frame(vec![]);
        let id = *h.app.doc.vectors.keys().next().unwrap();
        // from the cross in the middle, mostly right and a little up, the key held throughout
        let (from, to) = (h.at(30.0, 30.0), h.at(60.0, 35.0));
        h.frame_holding(held, vec![Event::PointerMoved(from)]);
        h.press(from, true, held);
        h.frame_holding(held, vec![Event::PointerMoved(from.lerp(to, 0.5))]);
        h.frame_holding(held, vec![Event::PointerMoved(to)]);
        h.press(to, false, held);
        h.frame(vec![]);
        let b = h.app.doc.vectors[&id].bounds();
        assert!(
            (b.min.y - 10.0).abs() < 1e-9,
            "kept level with {held:?}: {b:?}"
        );
        assert!(
            (b.min.x - 40.0).abs() < 1.0,
            "moved across with {held:?}: {b:?}"
        );
    }
}

#[test]
fn a_slow_click_with_a_little_jitter_on_a_selected_vector_does_not_move_it() {
    let mut h = Harness::new();
    h.rectangle(10.0, 10.0, 40.0, 40.0);
    let id = *h.app.doc.vectors.keys().next().unwrap();
    h.app.doc.selection = [id].into();
    h.frame(vec![]);
    let (before, steps) = (h.app.doc.vectors.clone(), h.app.history.undo_depth());
    // held past a click's time, and the hand shifts a pixel: on a side's handle, and
    // inside the selection
    for (x, y) in [(10.0, 30.0), (20.0, 40.0)] {
        let at = h.at(x, y);
        h.frame(vec![Event::PointerMoved(at)]);
        h.press(at, true, Modifiers::NONE);
        for _ in 0..8 {
            h.frame(vec![]);
        }
        h.frame(vec![Event::PointerMoved(at + vec2(1.0, 1.0))]);
        h.press(at + vec2(1.0, 1.0), false, Modifiers::NONE);
        h.frame(vec![]);
        assert_eq!(h.app.doc.vectors, before, "not moved from {x}, {y}");
    }
    assert_eq!(h.app.history.undo_depth(), steps, "no Moved step");
}

#[test]
fn escape_during_a_2d_drag_cancels_it_and_the_release_does_nothing() {
    let mut h = Harness::new();
    h.rectangle(10.0, 10.0, 40.0, 40.0);
    let id = *h.app.doc.vectors.keys().next().unwrap();
    let before = h.app.doc.vectors.clone();
    let escaped = |h: &mut Harness, from: (f64, f64), to: (f64, f64)| {
        let (from, to) = (h.at(from.0, from.1), h.at(to.0, to.1));
        h.frame(vec![Event::PointerMoved(from)]);
        h.press(from, true, Modifiers::NONE);
        h.frame(vec![Event::PointerMoved(from.lerp(to, 0.5))]);
        h.frame(vec![Event::PointerMoved(to)]);
        h.frame(vec![key(Key::Escape)]);
        h.press(to, false, Modifiers::NONE);
        h.frame(vec![]);
    };
    // Select mode: a move from inside the selection, and a scale from its corner
    for (from, to) in [((20.0, 40.0), (80.0, 60.0)), ((50.0, 50.0), (90.0, 90.0))] {
        h.app.doc.selection = [id].into();
        h.frame(vec![]);
        escaped(&mut h, from, to);
        assert_eq!(h.app.doc.vectors, before, "nothing changed");
    }
    // Transform mode: a scale from the corner
    h.app.doc.selection = [id].into();
    h.app.invoke(&h.ctx, "transform");
    h.frame(vec![]);
    escaped(&mut h, (50.0, 50.0), (90.0, 90.0));
    assert_eq!(h.app.doc.vectors, before, "not scaled, nor moved");
    // a drawing tool: no rectangle drawn
    h.app.invoke(&h.ctx, "draw_rectangle");
    h.frame(vec![]);
    escaped(&mut h, (100.0, 100.0), (150.0, 150.0));
    assert_eq!(h.app.doc.vectors, before, "nothing drawn");
    // a guide pulled out of the ruler: none is made, now or at the next drag's end
    h.app.form = None;
    h.app.invoke(&h.ctx, "select");
    h.frame(vec![]);
    let area = h.app.view_area.unwrap();
    let ruler = h.ctx.global_style().spacing.interact_size.y / 2.0;
    let top_ruler = egui::pos2(area.center().x, area.min.y + ruler);
    let to = h.at(100.0, 80.0);
    h.frame(vec![Event::PointerMoved(top_ruler)]);
    h.press(top_ruler, true, Modifiers::NONE);
    h.frame(vec![Event::PointerMoved(top_ruler.lerp(to, 0.5))]);
    h.frame(vec![Event::PointerMoved(to)]);
    h.frame(vec![key(Key::Escape)]);
    h.press(to, false, Modifiers::NONE);
    h.frame(vec![]);
    assert!(h.app.doc.guides.is_empty(), "no guide");
    h.app.doc.selection.clear();
    h.click(h.at(10.0, 30.0), Modifiers::NONE);
    assert_eq!(h.app.doc.selection, [id].into(), "a click picks again");
    h.drag(h.at(200.0, 200.0), h.at(250.0, 250.0));
    assert!(
        h.app.doc.guides.is_empty(),
        "nor later: {:?}",
        h.app.doc.guides
    );
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
fn escape_cancels_a_slow_job() {
    let mut h = Harness::new();
    h.app.wait_before_showing = std::time::Duration::ZERO;
    let (_go, wait) = std::sync::mpsc::channel::<()>();
    h.app.in_background(
        "Previewing Slow Toolpath",
        "test:slow",
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
fn clicking_the_view_puts_a_flown_out_page_away() {
    let mut h = Harness::new();
    h.app.pages.open(Page::Toolpaths);
    h.frame(vec![]);
    h.click(h.at(20.0, 200.0), Modifiers::NONE);
    assert_eq!(h.app.pages.shown(Page::Toolpaths), Shown::Hidden);
    assert_eq!(h.app.pages.shown(Page::Design), Shown::Docked);
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
fn a_drawing_imports_into_the_selected_circle_and_its_window_redoes_it_in_one_undo_step() {
    use cut_craft_core::geometry::{Path, Point, Rect};
    let mut h = Harness::new();
    // a circle of radius 50 at (100, 100), selected
    let mut c = Change::builder(&h.app.doc);
    let circle = c.add(Path::circle(Point::new(100.0, 100.0), 50.0));
    c.select([circle]);
    h.app.record(Ok(c.finish()), "circle");
    let before = h.app.history.undo_depth();
    let svg = std::env::temp_dir().join(format!("cut-craft-into-{}.svg", std::process::id()));
    std::fs::write(
        &svg,
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="20mm" height="10mm" viewBox="0 0 20 10"><rect x="0" y="0" width="20" height="10"/></svg>"#,
    )
    .unwrap();
    h.app.import_into(&svg, circle);
    h.frame(vec![]);
    h.frame(vec![]);
    let drawn = |h: &Harness| {
        h.app
            .doc
            .vectors
            .iter()
            .filter(|(id, _)| **id != circle)
            .map(|(_, p)| p.bounds())
            .reduce(Rect::union)
            .expect("the drawing came in")
    };
    let b = drawn(&h);
    // inside the circle less the 5 mm margin: its corners within 45 of the middle
    let corner = b.max - Point::new(100.0, 100.0);
    assert!((corner.length() - 45.0).abs() < 0.1, "{b:?}");
    assert!(
        (b.size().x / b.size().y - 2.0).abs() < 1e-6,
        "proportions kept"
    );
    assert!(
        h.text_at("Import Into Shape").is_some(),
        "its window is open"
    );
    assert!(h.app.doc.vectors.contains_key(&circle), "the shape is kept");
    // Stretch redoes it, still one step
    h.app.fit_import.as_mut().unwrap().way = super::fit_import::Way::Stretch;
    h.app.place_fit_import();
    assert!((drawn(&h).size().x / drawn(&h).size().y - 1.0).abs() < 1e-6);
    assert_eq!(h.app.history.undo_depth(), before + 1);
    h.app.history.undo(&mut h.app.doc);
    assert_eq!(h.app.doc.vectors.len(), 1, "one undo takes it out");
    h.frame(vec![]);
    assert!(
        h.app.fit_import.is_none(),
        "the window closes once the job changed"
    );
    let _ = std::fs::remove_file(svg);
}

#[test]
fn import_into_selected_shape_with_nothing_selected_says_what_to_pick() {
    let mut h = Harness::new();
    h.app.invoke(&h.ctx, "import_into_shape");
    assert!(
        h.app
            .status
            .message
            .starts_with("Select the shape to import into"),
        "{}",
        h.app.status.message
    );
}

/// A circle selected as Draw Circle opens is loaded into the form, and changing the form
/// changes that circle: no second circle, no button to press.
#[test]
fn a_selected_shape_opens_in_its_drawing_tool_and_follows_the_form() {
    let mut h = Harness::new();
    let circle = serde_json::json!({ "center": { "x": 50.0, "y": 50.0 }, "radius": 10.0 });
    h.app.apply("draw_circle", &circle).unwrap();
    let id = *h.app.doc.vectors.keys().next().unwrap();
    assert!(h.app.doc.selection.contains(&id));
    h.app.invoke(&h.ctx, "draw_circle");
    h.frame(vec![]);
    let form = h.app.form.as_ref().unwrap();
    assert_eq!(form.drawn, Some(id), "the form edits the selected circle");
    assert_eq!(form.params["radius"].as_f64(), Some(10.0));
    assert!(h.text_at("Create").is_none() && h.text_at("Apply").is_none());
    let form = h.app.form.as_mut().unwrap();
    form.params["radius"] = Value::from(20.0);
    form.dirty = true;
    h.frame(vec![]);
    assert_eq!(h.app.doc.vectors.len(), 1, "the same circle, not another");
    let width = h.app.doc.vectors.values().next().unwrap().bounds().size().x;
    assert!((width - 40.0).abs() < 1e-6, "{width}");
    // a rectangle is not Draw Circle's: it opens on new settings
    h.app.invoke(&h.ctx, "undo");
    h.app.form = None;
    h.app.doc.selection.clear();
    h.rectangle(10.0, 10.0, 20.0, 10.0);
    h.app.invoke(&h.ctx, "draw_circle");
    h.frame(vec![]);
    assert_eq!(h.app.form.as_ref().unwrap().drawn, None);
}

#[test]
fn shift_cmd_z_redoes_on_a_mac() {
    let mut h = Harness::new();
    h.ctx.set_os(egui::os::OperatingSystem::Mac);
    h.rectangle(10.0, 10.0, 50.0, 50.0);
    h.app.invoke(&h.ctx, "undo");
    h.frame(vec![]);
    assert!(h.app.doc.vectors.is_empty());
    let held = Modifiers::COMMAND | Modifiers::SHIFT;
    h.frame_holding(held, vec![key_with(Key::Z, held)]);
    assert_eq!(h.app.doc.vectors.len(), 1, "⇧⌘Z is Redo, as the menu shows");
}

#[test]
fn a_shift_shortcut_set_in_options_beats_the_plain_keys_own() {
    let mut h = Harness::new();
    h.rectangle(10.0, 10.0, 50.0, 50.0);
    h.app.doc.selection = h.app.doc.vectors.keys().copied().collect();
    let shift_m = shortcut::parse_trigger("Shift+M").unwrap();
    h.app.add_shortcut("draw_circle", shift_m);
    h.apply_options();
    h.frame_holding(Modifiers::SHIFT, vec![key_with(Key::M, Modifiers::SHIFT)]);
    assert_eq!(
        h.app.form.as_ref().map(|f| f.id),
        Some("draw_circle"),
        "Shift+M runs what it was set for, not Move (M)"
    );
    h.app.form = None;
    h.frame(vec![key_with(Key::M, Modifiers::NONE)]);
    assert_eq!(
        h.app.form.as_ref().map(|f| f.id),
        Some("move"),
        "M is still Move"
    );
}

#[test]
fn esc_closes_only_the_window_in_front_of_the_form() {
    let mut h = Harness::new();
    h.app.start_command("draw_circle");
    h.app.invoke(&h.ctx, "find_tool");
    h.frame(vec![]);
    h.frame(vec![escape()]);
    assert!(h.app.search.is_none(), "Find Tool closes");
    assert!(h.app.form.is_some(), "the form behind it stays");
    // the library's Properties window, in front of the form
    h.app
        .clipart
        .open_properties(std::path::Path::new("bird.stl"));
    h.frame(vec![escape()]);
    assert!(!h.app.clipart.close_properties(), "Properties closed");
    assert!(h.app.form.is_some(), "the form behind it stays");
    // nothing in front: Esc closes the form
    h.frame(vec![escape()]);
    assert!(h.app.form.is_none());
}

#[test]
fn esc_from_a_move_opened_over_a_form_goes_back_to_it_with_its_edits() {
    let mut h = Harness::new();
    h.rectangle(10.0, 10.0, 20.0, 20.0);
    h.app.doc.selection = h.app.doc.vectors.keys().copied().collect();
    h.app.invoke(&h.ctx, "profile_toolpath");
    h.app.form.as_mut().unwrap().params["name"] = Value::from("Edited");
    h.frame(vec![]);
    // a stray M: Move takes the form's place
    h.frame(vec![key(Key::M)]);
    assert_eq!(h.app.form.as_ref().map(|f| f.id), Some("move"));
    h.frame(vec![escape()]);
    let form = h.app.form.as_ref().expect("the toolpath's form is back");
    assert_eq!(form.id, "profile_toolpath");
    assert_eq!(form.params["name"], "Edited", "with its edits");
    // Esc again closes it, as ever
    h.frame(vec![escape()]);
    assert!(h.app.form.is_none());
}

/// A form kept behind a placing tool belongs to its job: after a new job, Esc from a placing
/// tool never brings the last job's form back.
#[test]
fn a_new_job_lets_go_of_the_form_kept_behind_a_placing_tool() {
    let mut h = Harness::new();
    h.rectangle(10.0, 10.0, 20.0, 20.0);
    h.app.doc.selection = h.app.doc.vectors.keys().copied().collect();
    h.app.invoke(&h.ctx, "profile_toolpath");
    h.frame(vec![]);
    h.frame(vec![key(Key::M)]);
    assert_eq!(h.app.form.as_ref().map(|f| f.id), Some("move"));
    h.app.reset_job();
    h.rectangle(10.0, 10.0, 20.0, 20.0);
    h.app.doc.selection = h.app.doc.vectors.keys().copied().collect();
    h.frame(vec![]);
    h.frame(vec![key(Key::M)]);
    assert_eq!(h.app.form.as_ref().map(|f| f.id), Some("move"));
    h.frame(vec![escape()]);
    assert_eq!(
        h.app.form.as_ref().map(|f| f.id),
        None,
        "the last job's form is not brought back"
    );
}

#[test]
fn the_ribbon_folds_the_least_used_groups_first_and_never_a_favourites() {
    use super::layouts::{GroupFit, fold_plan};
    let fit = |uses, kept| GroupFit {
        open: 100.0,
        folded: 40.0,
        uses,
        kept,
    };
    let groups = [fit(5, false), fit(0, false), fit(0, false), fit(9, false)];
    assert_eq!(fold_plan(&groups, 400.0), [false; 4], "all fit");
    // one fold saves 60: the rightmost of the two used least goes first
    assert_eq!(fold_plan(&groups, 340.0), [false, false, true, false]);
    assert_eq!(fold_plan(&groups, 280.0), [false, true, true, false]);
    assert_eq!(fold_plan(&groups, 220.0), [true, true, true, false]);
    // a group holding a favourite stays open, even if never used
    let kept = [fit(5, false), fit(0, true), fit(0, false)];
    assert_eq!(fold_plan(&kept, 200.0), [true, false, true]);
    // room for nothing: everything that may fold does, and the row scrolls
    assert_eq!(fold_plan(&kept, 10.0), [true, false, true]);
}

#[test]
fn a_narrow_ribbon_folds_the_groups_used_least_and_picks_are_counted() {
    let folded = |h: &Harness| -> Vec<String> {
        h.shapes
            .iter()
            .filter_map(|c| match &c.shape {
                egui::Shape::Text(t) if t.galley.text().ends_with('▾') => {
                    Some(t.galley.text().to_owned())
                }
                _ => None,
            })
            .collect()
    };
    let mut h = Harness::new();
    h.app.options.general.layout = crate::options::WindowLayout::Ribbon;
    h.window = vec2(5000.0, 800.0);
    h.frame(vec![]);
    h.frame(vec![]);
    assert!(folded(&h).is_empty(), "room for all: {:?}", folded(&h));
    h.window = vec2(2600.0, 800.0);
    h.frame(vec![]);
    h.frame(vec![]);
    let before = folded(&h);
    assert!(before.contains(&"Align Objects ▾".to_owned()), "{before:?}");
    // Align Left used a lot: its group opens and another folds instead
    h.app.options.tool_uses.insert("align_left".into(), 50);
    h.frame(vec![]);
    h.frame(vec![]);
    assert!(h.text_at("Align Objects ▾").is_none(), "{:?}", folded(&h));
    assert!(h.text_at("Align Objects").is_some());
    // a favourite's group never folds
    h.app.options.favourites.design = vec!["weld_vectors".into()];
    h.frame(vec![]);
    h.frame(vec![]);
    assert!(
        h.text_at("Join, Combine and Trim ▾").is_none(),
        "{:?}",
        folded(&h)
    );
    // a folded group's button lists its tools
    let button = folded(&h).first().cloned().expect("something still folded");
    let group = h
        .app
        .all_groups()
        .into_iter()
        .find(|g| format!("{} ▾", g.name) == button)
        .unwrap();
    let first = h.app.tool_label(group.tools[0]);
    assert!(h.text_at(&first).is_none(), "{first} hidden while folded");
    let at = h.text_at(&button).unwrap();
    h.press(at, true, Modifiers::NONE);
    h.press(at, false, Modifiers::NONE);
    h.frame(vec![]);
    h.frame(vec![]);
    assert!(
        h.text_at(&first).is_some(),
        "{button} lists {first} by name"
    );
    // a pick from a button counts
    let ctx = h.ctx.clone();
    h.app.pick(&ctx, Page::Design, "draw_circle");
    assert_eq!(h.app.options.tool_uses.get("draw_circle"), Some(&1));
}
