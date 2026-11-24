//! The 3D view: orbiting, the view cube, the surface, its detail and the frame rate.

use super::*;

#[test]
fn a_double_click_on_empty_space_steps_out_of_the_group() {
    let mut h = Harness::new();
    two_grouped(&mut h);
    h.double_click(h.at(10.0, 30.0));
    assert!(h.app.doc.inside_group.is_some());
    h.double_click(h.at(300.0, 300.0));
    assert_eq!(h.app.doc.inside_group, None);
    assert!(
        h.app.status.message.contains("Stepped out"),
        "{}",
        h.app.status.message
    );
}

#[test]
fn the_3d_views_menu_offers_convert_to_vectors_once_under_text() {
    fn count(items: &[gui::MenuItem]) -> usize {
        items
            .iter()
            .map(|i| usize::from(i.action == "convert_text_to_curves") + count(&i.children))
            .sum()
    }
    let h = Harness::new();
    let menu = h.app.menus.view_3d_menu(&h.app.doc, &[]);
    assert_eq!(count(&menu), 1);
    assert_eq!(
        leaf(&menu, &["Text", "Convert to Vectors"]).action,
        "convert_text_to_curves"
    );
}

#[test]
fn typing_f_in_a_text_field_does_not_fit_the_3d_view_but_f6_still_does() {
    let mut h = Harness::new();
    h.app.doc = cut_craft_core::testing::domed();
    h.app.view = View::ThreeD;
    h.frame(vec![]);
    h.frame(vec![]);
    let over = h.app.fit_3d.expect("the 3D view has been drawn").center();
    let zoomed = h.app.framing.zoomed_about(egui::Vec2::ZERO, 2.0);
    let typing = |h: &mut Harness, k: Key| {
        h.ctx
            .memory_mut(|m| m.request_focus(egui::Id::unique("a text field")));
        h.frame(vec![Event::PointerMoved(over), key(k)]);
    };
    h.app.framing = zoomed;
    typing(&mut h, Key::F);
    assert_eq!(
        h.app.framing, zoomed,
        "an f typed in a field leaves the view"
    );
    typing(&mut h, Key::F6);
    assert_eq!(
        h.app.framing,
        zoomed.fitted(),
        "F6 types nothing, so it fits"
    );
    h.app.framing = zoomed;
    h.frame(vec![Event::PointerMoved(over), key(Key::F)]);
    assert_eq!(
        h.app.framing,
        zoomed.fitted(),
        "F fits when nothing is typed in"
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
fn a_click_in_3d_leaves_another_view_tool_alone() {
    let (mut h, dome) = dome_from_above();
    h.app.view_tool = Some(ViewTool::Measure);
    h.click(dome, Modifiers::NONE);
    h.frame(vec![]);
    assert_eq!(h.app.doc.selected_components, [EntityId(1)].into());
    assert_eq!(h.app.view_tool, Some(ViewTool::Measure));
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

/// #588: Invert drag up / down tilts the 3D view the other way.
#[test]
fn inverting_up_down_tilts_the_3d_view_the_other_way() {
    let tilt_after_drag = |invert: bool| {
        let mut h = Harness::new();
        h.app.view = View::ThreeD;
        h.app.options.controls.invert_drag_y = invert;
        h.frame(vec![]);
        h.frame(vec![]);
        let fit = h.app.fit_3d.expect("the 3D view is drawn");
        let from = fit.center() + vec2(60.0, 60.0);
        let before = h.app.orbit;
        h.drag(from, from + vec2(0.0, 40.0));
        (before, h.app.orbit)
    };
    let (before, plain) = tilt_after_drag(false);
    let (_, inverted) = tilt_after_drag(true);
    let (a, b) = (plain.towards_viewer(), inverted.towards_viewer());
    let start = before.towards_viewer();
    assert!(a != start && b != start, "both drags turn the view");
    assert!(
        (a.z - start.z) * (b.z - start.z) < 0.0,
        "the tilt goes the other way: {start:?} {a:?} {b:?}"
    );
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
fn the_3d_views_vectors_button_hides_and_shows_them_and_is_remembered() {
    let mut h = shape_in_3d();
    assert!(h.app.display.vectors_3d, "shown to start with");
    let button = h
        .text_at("Vectors")
        .expect("a Vectors button on the 3D view");
    h.click(button, Modifiers::NONE);
    assert!(!h.app.display.vectors_3d, "hidden");
    assert!(!h.app.options.view_3d.draw_vectors, "kept with the options");
    let mut d = crate::display::Display::default();
    h.app.options.apply_to(&mut d);
    assert!(!d.vectors_3d, "hidden again next time");
    // the View menu's item is the same switch, and is kept too
    let ctx = h.ctx.clone();
    h.app.invoke(&ctx, "draw_vectors_in_3d_view");
    assert!(h.app.display.vectors_3d && h.app.options.view_3d.draw_vectors);
}

#[test]
fn auto_rotate_spins_the_3d_view_until_a_drag_stops_it() {
    let mut h = shape_in_3d();
    // the existing buttons stay where they were
    let vectors = h.text_at("Vectors").unwrap();
    let button = h
        .text_at("Auto-rotate")
        .expect("an Auto-rotate button on the 3D view");
    assert!(button.x > vectors.x, "after the existing buttons");
    assert!(
        h.text_at("Speed").is_none(),
        "the speed shows while it spins"
    );
    let start = h.app.orbit;
    h.click(button, Modifiers::NONE);
    assert!(h.app.spinning_3d.is_some(), "spinning");
    assert!(h.text_at("Speed").is_some(), "with its speed beside it");
    assert_eq!(h.text_at("Vectors"), Some(vectors), "nothing moved");
    for _ in 0..5 {
        h.frame(vec![]);
    }
    let speed = h.app.options.view_3d.spin_speed();
    let turned = h.app.orbit.yaw - start.yaw;
    // five frames of a tenth of a second, give or take the click's own frames
    assert!(
        turned > 0.4 * speed.to_radians() && turned < 0.8 * speed.to_radians(),
        "{turned}"
    );
    assert!(
        (h.app.orbit.pitch - start.pitch).abs() < 1e-12,
        "the tilt kept"
    );
    // a drag in the view stops it, and turns the view as a drag always does
    let area = h.app.view_3d_area.unwrap();
    h.drag(area.center(), area.center() + vec2(40.0, 0.0));
    assert!(h.app.spinning_3d.is_none(), "a drag stops it");
    let still = h.app.orbit;
    h.frame(vec![]);
    h.frame(vec![]);
    assert_eq!(h.app.orbit, still, "and it stays still");
    assert!(h.text_at("Speed").is_none());
    // clicking the button again starts it; clicking once more stops it
    let button = h.text_at("Auto-rotate").unwrap();
    h.click(button, Modifiers::NONE);
    assert!(h.app.spinning_3d.is_some());
    h.click(button, Modifiers::NONE);
    assert!(h.app.spinning_3d.is_none(), "the button stops it too");
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
fn a_space_typed_between_a_polylines_angle_and_length_does_not_finish_it() {
    let mut h = Harness::new();
    h.app.options.snapping.smart_snapping = false;
    h.app.options.snapping.geometry_snapping = false;
    h.app.invoke(&h.ctx, "draw_polyline");
    h.frame(vec![]);
    let at = h.at(10.0, 10.0);
    h.click(at, Modifiers::NONE);
    // "90A 2L": the Space comes as a key and as typed text
    h.frame(vec![Event::Text("90A".into())]);
    h.frame(vec![key(Key::Space), Event::Text(" ".into())]);
    h.frame(vec![Event::Text("2L".into())]);
    assert_eq!(h.app.polyline.points.len(), 1, "still drawing");
    h.frame(vec![key(Key::Enter)]);
    assert_eq!(h.app.polyline.points.len(), 2, "{}", h.app.status.message);
    // with nothing typed, Space still finishes it
    let at = h.at(40.0, 10.0);
    h.click(at, Modifiers::NONE);
    h.frame(vec![key(Key::Space), Event::Text(" ".into())]);
    assert!(h.app.polyline.is_empty(), "finished");
    assert_eq!(h.app.doc.vectors.len(), 1);
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
fn the_detail_while_moving_is_two_steps_below_the_detail_when_still_unless_picked() {
    use crate::options::{Detail, MovingDetail};
    let mut v = crate::options::Options::default().performance;
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
fn the_surface_drawn_on_is_made_again_when_the_detail_when_still_changes() {
    let mut h = Harness::new();
    h.app.doc = cut_craft_core::testing::domed();
    let first = h.app.drawing_surface().expect("a model to draw on");
    let again = h.app.drawing_surface().unwrap();
    assert!(std::sync::Arc::ptr_eq(&first, &again), "kept");
    h.app.options.performance.still_detail = crate::options::Detail::Low;
    let changed = h.app.drawing_surface().unwrap();
    assert!(!std::sync::Arc::ptr_eq(&first, &changed), "made again");
}

#[test]
fn the_view_follows_the_frame_rate_by_default_aiming_for_thirty_a_second() {
    let mut v = crate::options::Options::default().performance;
    assert!(v.adjust_to_frame_rate);
    assert!((v.fps_wanted() - crate::options::TARGET_FPS).abs() < 1e-9);
    assert!((crate::options::TARGET_FPS - 30.0).abs() < 1e-9);
    assert!(!v.show_frame_rate, "the status bar note is opt-in");
    v.target_fps = 500.0;
    assert!((v.fps_wanted() - 120.0).abs() < 1e-9);
    v.target_fps = 0.0;
    assert!((v.fps_wanted() - 10.0).abs() < 1e-9);
}

#[test]
fn with_the_frame_rate_setting_off_moving_keeps_options_own_detail() {
    let h = turned_slowly(false, 20);
    assert_eq!(
        h.app.moving_cells(),
        h.app.options.performance.moving_cells()
    );
    assert_eq!(
        h.app.turning_cells(),
        h.app.options.performance.still_cells()
    );
}

#[test]
fn the_status_bar_shows_the_3d_detail_and_frame_rate_when_asked() {
    let mut h = turned_slowly(true, 3);
    assert!(status_text(&h, "3D detail").is_none(), "off unless asked");
    h.app.options.performance.show_frame_rate = true;
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
        importing: (false, false),
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

/// The 3D view's origin arrows start at the machine's Z zero: on the material's top, or on
/// its bottom (the bed) once Job Setup says Machine Bed.
#[test]
fn the_3d_origin_moves_to_the_bed_with_the_z_zero() {
    use cut_craft_core::document::{Job, ZZero};
    let mut job = Job {
        thickness: 24.5,
        ..Job::default()
    };
    assert!(panels::origin_3d(&job, false).abs() < 1e-12, "on top");
    job.z_zero = ZZero::MachineBed;
    assert!(
        (panels::origin_3d(&job, false) + 24.5).abs() < 1e-12,
        "on the bed, 24.5 mm under the top"
    );
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
