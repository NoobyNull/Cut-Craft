//! The 3D view: orbiting, the view cube, the surface, its detail and the frame rate.

use super::*;

#[test]
fn a_double_click_on_empty_space_steps_out_of_the_group() {
    let mut h = Harness::new();
    two_grouped(&mut h);
    h.double_click(h.at(10.0, 30.0));
    assert!(h.doc().inside_group.is_some());
    h.double_click(h.at(300.0, 300.0));
    assert_eq!(h.doc().inside_group, None);
    assert!(
        h.status_message().contains("Stepped out"),
        "{}",
        h.status_message()
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
    let menu = h.view_3d_menu();
    assert_eq!(count(&menu), 1);
    assert_eq!(
        leaf(&menu, &["Text", "Convert to Vectors"]).action,
        "convert_text_to_curves"
    );
}

#[test]
fn typing_f_in_a_text_field_does_not_fit_the_3d_view_but_f6_still_does() {
    let mut h = Harness::new();
    h.open_document(cut_craft_core::testing::domed());
    h.show_view(View::ThreeD);
    h.frame(vec![]);
    h.frame(vec![]);
    let over = h.fit_3d().center();
    let zoomed = h.framing_3d().zoomed_about(egui::Vec2::ZERO, 2.0);
    let typing = |h: &mut Harness, k: Key| {
        h.ctx
            .memory_mut(|m| m.request_focus(egui::Id::unique("a text field")));
        h.frame(vec![Event::PointerMoved(over), key(k)]);
    };
    h.given_framing_3d(zoomed);
    typing(&mut h, Key::F);
    assert_eq!(
        h.framing_3d(),
        zoomed,
        "an f typed in a field leaves the view"
    );
    typing(&mut h, Key::F6);
    assert_eq!(
        h.framing_3d(),
        zoomed.fitted(),
        "F6 types nothing, so it fits"
    );
    h.given_framing_3d(zoomed);
    h.frame(vec![Event::PointerMoved(over), key(Key::F)]);
    assert_eq!(
        h.framing_3d(),
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
    let mut h = transform_3d(view3d::Orbit::TOP, false);
    let at = |h: &Harness, which: Handle| handle_3d_at(h, Handle3d::Transform(which));
    let extent = |h: &Harness| h.doc().components[&EntityId(1)].extent();
    let before = extent(&h);
    let corner = Handle::Scale { x: 1, y: 1 };
    // the middle moves: dragged onto the top-right corner handle, the component's centre
    // lands where that corner was
    h.drag(at(&h, Handle::Move), at(&h, corner));
    let moved = extent(&h);
    assert!((moved.center() - before.max).length() < 0.5, "{moved:?}");
    h.undo();
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
    h.undo();
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
    // nearly side on in perspective: the top of the view is sky, not material
    let side_on = view3d::Orbit {
        pitch: 80f64.to_radians(),
        ..view3d::Orbit::FRONT
    };
    let mut h = transform_3d(side_on, true);
    let middle = handle_3d_at(&h, Handle3d::Transform(Handle::Move));
    let sky = h.fit_3d().center_top() + vec2(0.0, 5.0);
    let before = h.doc().components[&EntityId(1)].extent();
    h.drag(middle, sky);
    assert!(!h.dragging_in_3d(), "letting go ends the drag");
    assert_eq!(
        h.doc().components[&EntityId(1)].extent(),
        before,
        "let go over the sky, nothing moves"
    );
    // moving and clicking afterwards does not pick the abandoned drag back up
    h.click(middle + vec2(0.0, 30.0), Modifiers::NONE);
    assert_eq!(h.doc().components[&EntityId(1)].extent(), before);
}

#[test]
fn escape_cancels_a_3d_handle_drag_and_stays_in_transform_mode() {
    use cut_craft_core::handles::Handle;
    let mut h = transform_3d(view3d::Orbit::TOP, false);
    let extent = |h: &Harness| h.doc().components[&EntityId(1)].extent();
    let before = extent(&h);
    let depth = h.undo_steps();
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
    assert_eq!(h.undo_steps(), depth, "nothing to undo");
    assert_eq!(
        h.view_tool(),
        Some(ViewTool::Transform),
        "still transforming"
    );
    assert!(!h.dragging_in_3d());
    // a height: set as it is dragged, so Escape takes it back
    let height = |h: &Harness| h.doc().components[&EntityId(1)].shaping.shape_height;
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
    assert_eq!(h.undo_steps(), depth);
    assert_eq!(h.view_tool(), Some(ViewTool::Transform));
}

#[test]
fn the_3d_views_transform_handles_move_a_selection_of_vectors_only() {
    use cut_craft_core::handles::Handle;
    let mut h = transform_3d(view3d::Orbit::TOP, false);
    h.select_components([]);
    h.rectangle(2.0, 2.0, 6.0, 6.0);
    let id = *h
        .doc()
        .selection
        .iter()
        .next()
        .expect("the rectangle is selected");
    h.show_view(View::ThreeD);
    h.pick_view_tool(ViewTool::Transform);
    h.frame(vec![]);
    h.frame(vec![]);
    assert!(
        !h.handles_3d().iter().any(|(k, _)| *k == Handle3d::Height),
        "a vector has no component icons"
    );
    let before = h.doc().vectors[&id].bounds();
    let from = handle_3d_at(&h, Handle3d::Transform(Handle::Move));
    let corner = handle_3d_at(&h, Handle3d::Transform(Handle::Scale { x: 1, y: 1 }));
    h.drag(from, corner);
    let after = h.doc().vectors[&id].bounds();
    assert!((after.center() - before.max).length() < 0.5, "{after:?}");
}

#[test]
fn with_nothing_selected_the_3d_transform_mode_says_how_to_start() {
    let mut h = transform_3d(view3d::Orbit::TOP, false);
    h.select_components([]);
    let middle = h.fit_3d().center();
    h.frame(vec![Event::PointerMoved(middle)]);
    h.frame(vec![]);
    assert_eq!(
        h.handles_3d(),
        &[] as &[(Handle3d, Pos2)],
        "no selection, no handles"
    );
    let hint = cut_craft_bug_report::hint::current(&h.ctx).unwrap_or_default();
    assert!(hint.contains("select it"), "{hint}");
}

#[test]
fn a_click_in_3d_leaves_another_view_tool_alone() {
    let (mut h, dome) = dome_from_above();
    h.pick_view_tool(ViewTool::Measure);
    h.click(dome, Modifiers::NONE);
    h.frame(vec![]);
    assert_eq!(h.doc().selected_components, [EntityId(1)].into());
    assert_eq!(h.view_tool(), Some(ViewTool::Measure));
}

#[test]
fn set_in_the_3d_settings_places_the_fade_anchors_by_two_clicks() {
    let mut h = transform_3d(view3d::Orbit::TOP, false);
    let settings = handle_3d_at(&h, Handle3d::Settings);
    h.click(settings, Modifiers::NONE);
    h.frame(vec![]);
    let fade = h.text_at("Fade").expect("the settings popup");
    h.click(fade, Modifiers::NONE);
    h.frame(vec![]);
    let set = h.texts_at("Set")[0];
    h.click(set, Modifiers::NONE);
    h.frame(vec![]);
    let anchors = |h: &Harness| h.anchors_3d();
    assert_eq!(anchors(&h), Some((Anchor::Fade, None)));
    assert_eq!(h.status_message(), Anchor::Fade.prompt(false));
    // two clicks on the dome, left then right of its middle
    let fit = h.fit_3d();
    let surface = h.surface_3d().unwrap();
    let screen = view3d::screen_of(&surface, h.orbit(), h.framing_3d(), fit, None);
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
    assert_eq!(h.status_message(), Anchor::Fade.prompt(true));
    h.click(on_dome(24.0), Modifiers::NONE);
    h.frame(vec![]);
    h.frame(vec![]);
    let fade = h.doc().components[&EntityId(1)]
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
        h.show_view(View::ThreeD);
        h.given_options(|o| o.controls.invert_drag_y = invert);
        h.frame(vec![]);
        h.frame(vec![]);
        let from = h.fit_3d().center() + vec2(60.0, 60.0);
        let before = h.orbit();
        h.drag(from, from + vec2(0.0, 40.0));
        (before, h.orbit())
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
    let shape = h.default_settings("create_shape");
    h.run("create_shape", &shape).unwrap();
    h.show_view(View::ThreeD);
    h.frame(vec![]);
    h.frame(vec![]);
    h
}

#[test]
fn the_view_cube_looks_from_the_face_edge_or_corner_clicked() {
    use cut_craft_core::glam::DVec3;
    let mut h = shape_in_3d();
    let looks_from = |h: &Harness, from: DVec3| {
        let towards = h.orbit().towards_viewer();
        assert!(
            towards.abs_diff_eq(from.normalize(), 1e-9),
            "{towards} not {from}"
        );
    };
    let cube = h.view_cube();
    let framing = h.framing_3d();
    // the right face's middle, with its name on it
    let right = cube.screen(h.orbit(), DVec3::X);
    assert!(
        h.texts_at("Right")
            .iter()
            .any(|at| at.distance(right) < 1.0),
        "named"
    );
    h.click(right, Modifiers::NONE);
    looks_from(&h, DVec3::X);
    assert_eq!(h.framing_3d(), framing, "turned, not refitted");
    // face on now: its top corner at the back, then its top edge
    let middle = view3d::ViewCube::part_middle;
    let corner = cube.screen(h.orbit(), middle(DVec3::new(1.0, 1.0, 1.0)));
    h.click(corner, Modifiers::NONE);
    looks_from(&h, DVec3::new(1.0, 1.0, 1.0));
    let edge = cube.screen(h.orbit(), middle(DVec3::new(1.0, 0.0, 1.0)));
    h.click(edge, Modifiers::NONE);
    looks_from(&h, DVec3::new(1.0, 0.0, 1.0));
    // a double-click looks from the other side, and the view behind takes no clicks
    let selected = h.doc().selected_components.clone();
    h.double_click(cube.screen(h.orbit(), middle(DVec3::new(1.0, 0.0, 1.0))));
    looks_from(&h, DVec3::new(-1.0, 0.0, -1.0));
    assert_eq!(h.doc().selected_components, selected);
    assert_eq!(h.framing_3d(), framing);
}

#[test]
fn the_view_cubes_buttons_turn_nudge_and_go_home() {
    use cut_craft_core::glam::DVec3;
    use view3d::{CubeButton, Nudge, Orbit};
    let mut h = shape_in_3d();
    let looks_from = |h: &Harness, from: DVec3| {
        let towards = h.orbit().towards_viewer();
        assert!(towards.abs_diff_eq(from, 1e-9), "{towards} not {from}");
    };
    let press = |h: &mut Harness, button| {
        let at = h.view_cube().button_rect(button).center();
        h.click(at, Modifiers::NONE);
        h.frame(vec![]);
    };
    // looking from a corner the turns do nothing: they need a face straight on
    let start = h.orbit();
    press(&mut h, CubeButton::TurnRight);
    assert_eq!(h.orbit(), start);
    // straight at the front: a quarter turn clockwise lays Z along the screen's x
    h.given_orbit(Orbit::FRONT);
    h.frame(vec![]);
    press(&mut h, CubeButton::TurnRight);
    looks_from(&h, DVec3::NEG_Y);
    assert!(h.orbit().project(DVec3::Z).0.x > 0.99, "Z points right");
    // the nudge above brings round the face now at the top of the screen: Left
    press(&mut h, CubeButton::Nudge(Nudge::Up));
    looks_from(&h, DVec3::NEG_X);
    // Home goes back to the starting view and fits the job
    h.given_framing_3d(h.framing_3d().zoomed_about(egui::Vec2::ZERO, 3.0));
    press(&mut h, CubeButton::Home);
    assert_eq!(h.orbit(), Orbit::default());
    assert_eq!(h.framing_3d().zoom, 1.0);
}

#[test]
fn the_home_buttons_menu_makes_this_view_home_and_resets_it() {
    use view3d::{CubeButton, Orbit};
    let mut h = shape_in_3d();
    h.given_orbit(Orbit::RIGHT);
    h.frame(vec![]);
    let home = h.view_cube().button_rect(CubeButton::Home).center();
    h.right_click(home);
    h.click(h.text_at("Make This View Home").unwrap(), Modifiers::NONE);
    assert!(h.options().view_3d.home_view.is_some());
    h.given_orbit(Orbit::TOP);
    h.frame(vec![]);
    h.click(home, Modifiers::NONE);
    h.frame(vec![]);
    assert!((h.orbit().towards_viewer() - Orbit::RIGHT.towards_viewer()).length() < 1e-9);
    h.right_click(home);
    h.click(h.text_at("Reset Home").unwrap(), Modifiers::NONE);
    assert_eq!(h.options().view_3d.home_view, None);
}

#[test]
fn the_view_turns_smoothly_to_a_clicked_face_and_a_turntable_stands_it_upright() {
    use cut_craft_core::glam::DVec3;
    use view3d::{CubeButton, Rotation};
    let mut h = shape_in_3d();
    h.given_options(|o| o.view_3d.smooth_turns = true);
    let right = h.view_cube().screen(h.orbit(), DVec3::X);
    h.click(right, Modifiers::NONE);
    // a frame (a tenth of a second) on, part of the way round
    let partway = h.orbit().towards_viewer();
    assert!(partway.x > 0.05 && partway.x < 0.99, "{partway}");
    for _ in 0..4 {
        h.frame(vec![]);
    }
    assert!(h.orbit().towards_viewer().abs_diff_eq(DVec3::X, 1e-9));
    // turned on its side, then dragged as a turntable: upright again, smoothly
    h.given_options(|o| o.view_3d.rotation = crate::options::Rotation::Turntable);
    let turn = h.view_cube().button_rect(CubeButton::TurnRight).center();
    h.click(turn, Modifiers::NONE);
    for _ in 0..4 {
        h.frame(vec![]);
    }
    assert!(h.orbit().roll.abs() > 1.0, "on its side");
    let cube = h.view_cube().rect();
    let from = cube.left_bottom() + egui::vec2(-cube.width(), cube.height());
    h.drag(from, from + egui::vec2(-40.0, 0.0));
    for _ in 0..4 {
        h.frame(vec![]);
    }
    assert!(h.orbit().roll.abs() < 1e-9, "upright: {}", h.orbit().roll);
    assert_eq!(h.options().view_3d.rotation.view(), Rotation::Turntable);
}

#[test]
fn the_3d_view_turns_to_perspective_and_tilts_underneath() {
    let mut h = shape_in_3d();
    let start = h.orbit();
    let button = h.text_at("Perspective").expect("the perspective toggle");
    h.click(button, Modifiers::NONE);
    assert!(h.framing_3d().perspective);
    h.click(h.text_at("Fit").unwrap(), Modifiers::NONE);
    assert!(h.framing_3d().perspective, "Fit keeps the projection");
    // a long drag down, below the cube, tilts the view past side on to under the job
    let cube = h.view_cube().rect();
    let from = cube.left_bottom() + vec2(-cube.width(), cube.height());
    h.drag(from, from + vec2(0.0, 300.0));
    let pitch = h.orbit().pitch;
    assert!(
        pitch > std::f64::consts::FRAC_PI_2 && pitch > start.pitch,
        "{pitch}"
    );
    assert!(h.orbit().towards_viewer().z < 0.0, "from underneath");
    h.click(button, Modifiers::NONE);
    assert!(!h.framing_3d().perspective, "back to orthographic");
}

#[test]
fn the_3d_views_vectors_button_hides_and_shows_them_and_is_remembered() {
    let mut h = shape_in_3d();
    assert!(h.vectors_in_3d(), "shown to start with");
    let button = h
        .text_at("Vectors")
        .expect("a Vectors button on the 3D view");
    h.click(button, Modifiers::NONE);
    assert!(!h.vectors_in_3d(), "hidden");
    assert!(!h.options().view_3d.draw_vectors, "kept with the options");
    let mut d = crate::display::Display::default();
    h.options().apply_to(&mut d);
    assert!(!d.vectors_3d, "hidden again next time");
    // the View menu's item is the same switch, and is kept too
    h.invoke("draw_vectors_in_3d_view");
    assert!(h.vectors_in_3d() && h.options().view_3d.draw_vectors);
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
    let start = h.orbit();
    h.click(button, Modifiers::NONE);
    assert!(h.spinning_3d(), "spinning");
    assert!(h.text_at("Speed").is_some(), "with its speed beside it");
    assert_eq!(h.text_at("Vectors"), Some(vectors), "nothing moved");
    for _ in 0..5 {
        h.frame(vec![]);
    }
    let speed = h.options().view_3d.spin_speed();
    let turned = h.orbit().yaw - start.yaw;
    // five frames of a tenth of a second, give or take the click's own frames
    assert!(
        turned > 0.4 * speed.to_radians() && turned < 0.8 * speed.to_radians(),
        "{turned}"
    );
    assert!(
        (h.orbit().pitch - start.pitch).abs() < 1e-12,
        "the tilt kept"
    );
    // a drag in the view stops it, and turns the view as a drag always does
    let area = h.view_3d_area();
    h.drag(area.center(), area.center() + vec2(40.0, 0.0));
    assert!(!h.spinning_3d(), "a drag stops it");
    let still = h.orbit();
    h.frame(vec![]);
    h.frame(vec![]);
    assert_eq!(h.orbit(), still, "and it stays still");
    assert!(h.text_at("Speed").is_none());
    // clicking the button again starts it; clicking once more stops it
    let button = h.text_at("Auto-rotate").unwrap();
    h.click(button, Modifiers::NONE);
    assert!(h.spinning_3d());
    h.click(button, Modifiers::NONE);
    assert!(!h.spinning_3d(), "the button stops it too");
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
        h.given_options(|o| {
            o.view_3d.simplify_while_moving = simplify;
            if let Some(settle) = settle {
                o.view_3d.settle_seconds = settle;
            }
        });
        let o = h.doc().job.area().min;
        let rect = cut_craft_core::geometry::Rect::new(
            o + Point::new(150.0, 100.0),
            o + Point::new(450.0, 250.0),
        );
        let cell = h.doc().job.model_cell();
        let mut c = Change::builder(h.doc());
        c.add_component(Component::from_fn(
            "relief",
            Combine::Add,
            rect,
            cell,
            |p| Some((p.x * 0.5).sin() + 2.0),
        ));
        h.make(c.finish(), "added");
        h.show_view(View::ThreeD);
        h.given_3d_moved_at(h.time - ago);
        h.frame(vec![]);
        h.fine_triangles().is_some()
    }
}

#[test]
fn a_space_mouse_event_after_a_pause_is_not_the_pause_long() {
    use crate::spacemouse::{Input, LONGEST_STEP, moved};
    let mut h = Harness::new();
    h.rectangle(10.0, 10.0, 20.0, 10.0);
    h.show_view(View::ThreeD);
    h.frame(vec![]);
    let (tx, rx) = std::sync::mpsc::channel();
    h.given_space_mouse(rx);
    let before = (h.orbit(), h.framing_3d());
    // a light twist and push, the first event after half a minute still
    let motion = ([0, 0, 40], [0, 35, 0]);
    tx.send(Input::Motion {
        translate: motion.0,
        rotate: motion.1,
        period: 30_000,
    })
    .unwrap();
    h.frame(vec![]);
    let settings = &h.options().space_mouse;
    let rotation = h.options().view_3d.rotation.view();
    let (orbit, framing) = moved(before, motion, LONGEST_STEP, settings, rotation);
    assert!(
        (h.orbit().yaw - orbit.yaw).abs() < 1e-9 && h.framing_3d() == framing,
        "{:?} against {orbit:?}",
        h.orbit()
    );
    assert!(h.orbit() != before.0, "it still turned");
}

#[test]
fn a_space_typed_between_a_polylines_angle_and_length_does_not_finish_it() {
    let mut h = Harness::new();
    h.given_options(|o| {
        o.snapping.smart_snapping = false;
        o.snapping.geometry_snapping = false;
    });
    h.invoke("draw_polyline");
    h.frame(vec![]);
    let at = h.at(10.0, 10.0);
    h.click(at, Modifiers::NONE);
    // "90A 2L": the Space comes as a key and as typed text
    h.frame(vec![Event::Text("90A".into())]);
    h.frame(vec![key(Key::Space), Event::Text(" ".into())]);
    h.frame(vec![Event::Text("2L".into())]);
    assert_eq!(h.polyline_points(), 1, "still drawing");
    h.frame(vec![key(Key::Enter)]);
    assert_eq!(h.polyline_points(), 2, "{}", h.status_message());
    // with nothing typed, Space still finishes it
    let at = h.at(40.0, 10.0);
    h.click(at, Modifiers::NONE);
    h.frame(vec![key(Key::Space), Event::Text(" ".into())]);
    assert_eq!(h.polyline_points(), 0, "finished");
    assert_eq!(h.doc().vectors.len(), 1);
}

#[test]
fn save_shaded_image_saves_the_3d_view_s_part_of_the_window() {
    let mut h = Harness::new();
    h.open_document(cut_craft_core::testing::domed());
    assert!(h.available("save_shaded_image"));
    let path = std::env::temp_dir().join(format!("cut-craft-shaded-{}.png", std::process::id()));
    h.save_shaded_image(path.clone());
    h.frame(vec![]);
    assert_eq!(*h.view(), View::ThreeD, "the 3D view is shown");
    let asked = h
        .screenshot
        .take()
        .expect("the window's picture is asked for");
    // the window's picture, 1280 x 800, red inside the 3D view and blue outside it
    let area = h.view_3d_area();
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
        h.status_message().starts_with("Saved the shaded image"),
        "{}",
        h.status_message()
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
    h.open_document(cut_craft_core::testing::domed());
    let first = h.drawing_surface().expect("a model to draw on");
    let again = h.drawing_surface().unwrap();
    assert!(std::sync::Arc::ptr_eq(&first, &again), "kept");
    h.given_options(|o| o.performance.still_detail = crate::options::Detail::Low);
    let changed = h.drawing_surface().unwrap();
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
    assert_eq!(h.moving_cells(), h.options().performance.moving_cells());
    assert_eq!(h.turning_cells(), h.options().performance.still_cells());
}

#[test]
fn the_status_bar_shows_the_3d_detail_and_frame_rate_when_asked() {
    let mut h = turned_slowly(true, 3);
    assert!(status_text(&h, "3D detail").is_none(), "off unless asked");
    h.given_options(|o| o.performance.show_frame_rate = true);
    for _ in 0..10 {
        h.given_3d_moved_at(h.time + 0.1);
        h.frame(vec![]);
    }
    let note = h.frame_rate_note();
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
    let o = h.doc().job.area().min;
    let rect = cut_craft_core::geometry::Rect::new(
        o + Point::new(150.0, 100.0),
        o + Point::new(450.0, 250.0),
    );
    let cell = h.doc().job.model_cell();
    let mut c = Change::builder(h.doc());
    c.add_component(Component::from_fn(
        "relief",
        Combine::Add,
        rect,
        cell,
        |p| Some((p.x * 0.5).sin() + 2.0),
    ));
    h.make(c.finish(), "added");
    h.show_view(View::ThreeD);
    h.frame(vec![]);
    // a moment after anything moved, the view settles and is drawn finely
    h.time += 1.0;
    h.frame(vec![]);
    let fine = h.fine_triangles();
    let key = h.fine_made_for().unwrap();
    let quick = view3d::mesh(
        &cut_craft_core::component::model_shape(h.doc(), MESH_CELLS),
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
    let kept = h.fine_made_for();
    h.time += 1.0;
    h.frame(vec![]);
    assert_eq!(h.fine_made_for(), kept, "kept, not rebuilt");
    // an edit to the model makes it again
    let mut c = Change::builder(h.doc());
    c.add_component(Component::from_fn("dome", Combine::Add, rect, cell, |_| {
        Some(4.0)
    }));
    h.make(c.finish(), "added");
    h.time += 1.0;
    h.frame(vec![]);
    assert_ne!(h.fine_made_for(), kept);
}

#[test]
fn the_3d_views_quick_surface_is_kept_until_the_model_changes() {
    let mut h = Harness::new();
    h.open_document(cut_craft_core::testing::domed());
    h.show_view(View::ThreeD);
    h.frame(vec![]);
    h.frame(vec![]);
    let kept = h.quick_made_for();
    assert!(kept.is_some(), "made once");
    h.frame(vec![]);
    assert_eq!(h.quick_made_for(), kept, "kept");
    h.select_components([EntityId(1)]);
    let change = Change::transform_selected(h.doc(), |_| {
        cut_craft_core::glam::DAffine2::from_translation(cut_craft_core::glam::DVec2::X)
    });
    h.make(change.expect("the move applies"), "moved");
    h.frame(vec![]);
    assert_ne!(h.quick_made_for(), kept, "made again");
}
