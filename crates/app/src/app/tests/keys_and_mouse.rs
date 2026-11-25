//! Keys and the mouse: shortcuts, the wheel, panning, zooming and the Mac's keys.

use super::*;

/// #588: a pinch on a trackpad zooms the 2D view about the pointer, as the wheel does; in
/// Trackpad mode two fingers pan instead of zooming.
#[test]
fn a_pinch_zooms_and_in_trackpad_mode_two_fingers_pan() {
    let mut h = Harness::new();
    let at = view_middle(&h) + vec2(40.0, -30.0);
    let (before, under) = {
        let t = h.app.view_transform.unwrap();
        (t.scale, t.to_job(at))
    };
    h.frame(vec![Event::PointerMoved(at)]);
    h.frame(vec![Event::Zoom(1.5)]);
    h.frame(vec![]);
    let t = h.app.view_transform.unwrap();
    assert!(t.scale > before * 1.4, "pinching out zooms in");
    assert!(t.to_job(at).distance(under) < 0.01, "about the cursor");
    // Trackpad mode: two fingers move the drawing with them, and leave the zoom
    h.app.options.controls.trackpad = true;
    let p = Point::new(100.0, 100.0);
    let (scale, was) = (t.scale, t.to_screen(p));
    h.frame(vec![Event::MouseWheel {
        unit: egui::MouseWheelUnit::Point,
        delta: vec2(30.0, -20.0),
        phase: egui::TouchPhase::Move,
        source: egui::MouseWheelSource::Trackpad,
        modifiers: Modifiers::NONE,
    }]);
    for _ in 0..30 {
        h.frame(vec![]);
    }
    let t = h.app.view_transform.unwrap();
    assert!((t.scale - scale).abs() < 1e-9, "no zoom");
    let moved = t.to_screen(p) - was;
    assert!(
        (moved - vec2(30.0, -20.0)).length() < 1.0,
        "the drawing follows the fingers: {moved:?}"
    );
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
    // its name is the tool's everywhere, so it is on the tool pages too: the one over the view
    let selection = *h
        .texts_at("Zoom to Selection")
        .iter()
        .find(|p| over_view(p))
        .expect("Zoom to Selection over the view");
    assert!(
        pan.x > area.center().x && selection.x > pan.x,
        "on the right, Pan first: {pan:?} {selection:?} in {area:?}"
    );
    assert!(pan.y < area.center().y, "along the top: {pan:?}");
    h.click(pan, Modifiers::NONE);
    assert_eq!(h.app.view_tool, Some(ViewTool::Pan));
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
fn a_shortcut_set_in_options_waits_for_apply() {
    use super::shortcuts_page::Edit;
    let mut h = Harness::new();
    h.app.options_window = true;
    h.app.options_goto = Some(super::shortcuts_page::CATEGORY);
    h.frame(vec![]);
    h.frame(vec![]);
    h.app
        .apply_shortcut_edit(Edit::Start("zoom_to_drawing".to_owned()));
    h.frame(vec![key(Key::F8)]);
    let f8 = shortcut::parse_trigger("F8").unwrap();
    assert!(
        h.app.shortcuts_of("zoom_to_drawing").contains(&f8),
        "listed"
    );
    assert!(
        !h.app
            .bindings(&h.ctx)
            .contains(&("zoom_to_drawing".to_owned(), f8)),
        "not used yet"
    );
    h.frame(vec![]);
    click_button(&mut h, "Apply");
    assert!(
        h.app
            .bindings(&h.ctx)
            .contains(&("zoom_to_drawing".to_owned(), f8)),
        "used once applied"
    );
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

/// A drag with `button` from `from` by `by`, a frame at a time.
fn drag_with(h: &mut Harness, button: PointerButton, from: Pos2, by: egui::Vec2) {
    h.frame(vec![Event::PointerMoved(from)]);
    h.frame(vec![mouse(button, from, true)]);
    h.frame(vec![Event::PointerMoved(from + by * 0.5)]);
    h.frame(vec![Event::PointerMoved(from + by)]);
    h.frame(vec![mouse(button, from + by, false)]);
    h.frame(vec![]);
}

/// Where job point `p` shows in the 2D view.
fn on_screen(h: &Harness, p: Point) -> Pos2 {
    h.app.view_transform.unwrap().to_screen(p)
}

#[test]
fn a_thumb_button_set_to_pan_pans_both_views() {
    let mut h = Harness::new();
    let p = Point::new(100.0, 100.0);
    let from = view_middle(&h);
    // to start with only the wheel pans: a thumb button drag does nothing
    let before = on_screen(&h, p);
    drag_with(&mut h, PointerButton::Extra1, from, vec2(40.0, 10.0));
    assert!(
        (on_screen(&h, p) - before).length() < 0.5,
        "not a pan button yet"
    );
    // Mouse Back added to Pan (hold and drag): it pans as the wheel does
    h.app.add_shortcut(
        shortcut::PAN_DRAG,
        shortcut::parse_trigger("Mouse Back").unwrap(),
    );
    h.apply_options();
    assert_eq!(
        h.app.options.controls.shortcuts[shortcut::PAN_DRAG],
        ["Wheel Click", "Mouse Back"]
    );
    let before = on_screen(&h, p);
    drag_with(&mut h, PointerButton::Extra1, from, vec2(40.0, 10.0));
    let moved = on_screen(&h, p) - before;
    assert!((moved - vec2(40.0, 10.0)).length() < 1.0, "{moved:?}");
    // the wheel still pans
    let before = on_screen(&h, p);
    drag_with(&mut h, PointerButton::Middle, from, vec2(-20.0, 0.0));
    assert!((on_screen(&h, p) - before - vec2(-20.0, 0.0)).length() < 1.0);
    // and in the 3D view
    h.app.view = View::ThreeD;
    h.frame(vec![]);
    let (orbit, framing) = (h.app.orbit, h.app.framing);
    let mid = h.app.view_3d_area.expect("the 3D view is drawn").center();
    drag_with(&mut h, PointerButton::Extra1, mid, vec2(30.0, 0.0));
    assert!(h.app.framing.pan.x > framing.pan.x + 10.0, "panned");
    assert_eq!(h.app.orbit, orbit, "not turned");
}

#[test]
fn tilting_the_wheel_pans_sideways_unless_set_not_to() {
    let mut h = Harness::new();
    let p = Point::new(100.0, 100.0);
    let at = view_middle(&h);
    h.frame(vec![Event::PointerMoved(at)]);
    let (before, scale) = (on_screen(&h, p), h.app.view_transform.unwrap().scale);
    for _ in 0..20 {
        h.frame(vec![tilt(-10.0)]);
    }
    let moved = on_screen(&h, p) - before;
    assert!(moved.x < -20.0 && moved.y.abs() < 0.5, "{moved:?}");
    assert!(
        (h.app.view_transform.unwrap().scale - scale).abs() < 1e-6,
        "no zoom"
    );
    // the other way round
    h.app.options.controls.tilt = crate::options::Tilt::PanReversed;
    let before = on_screen(&h, p);
    for _ in 0..20 {
        h.frame(vec![tilt(-10.0)]);
    }
    assert!(on_screen(&h, p).x - before.x > 20.0);
    // or not at all
    h.app.options.controls.tilt = crate::options::Tilt::Nothing;
    let before = on_screen(&h, p);
    for _ in 0..20 {
        h.frame(vec![tilt(-10.0)]);
    }
    assert!((on_screen(&h, p) - before).length() < 0.5);
    // in the 3D view it pans too
    h.app.options.controls.tilt = crate::options::Tilt::Pan;
    h.app.view = View::ThreeD;
    h.frame(vec![]);
    let mid = h.app.view_3d_area.expect("the 3D view is drawn").center();
    h.frame(vec![Event::PointerMoved(mid)]);
    let framing = h.app.framing;
    for _ in 0..20 {
        h.frame(vec![tilt(-10.0)]);
    }
    assert!(h.app.framing.pan.x < framing.pan.x - 20.0);
    assert!((h.app.framing.zoom - framing.zoom).abs() < 1e-6);
}

#[test]
fn a_mouse_button_set_as_a_shortcut_runs_its_action() {
    let mut h = Harness::new();
    let forward = shortcut::parse_trigger("Mouse Forward").unwrap();
    h.app.add_shortcut("pan", forward);
    h.apply_options();
    assert_eq!(h.app.view_tool, None);
    let at = view_middle(&h);
    h.frame(vec![mouse(PointerButton::Extra2, at, true)]);
    h.frame(vec![mouse(PointerButton::Extra2, at, false)]);
    assert_eq!(h.app.view_tool, Some(ViewTool::Pan), "Pan picked up");
    // the menus show it
    assert_eq!(h.app.shortcut_text("pan").as_deref(), Some("Mouse Forward"));
    // with Ctrl held it is another shortcut, which runs nothing
    h.app.view_tool = None;
    h.frame_holding(
        Modifiers::COMMAND,
        vec![Event::PointerButton {
            pos: at,
            button: PointerButton::Extra2,
            pressed: true,
            modifiers: Modifiers::COMMAND,
        }],
    );
    assert_eq!(h.app.view_tool, None);
}

#[test]
fn setting_a_shortcut_takes_the_next_button_and_asks_before_taking_anothers() {
    use super::shortcuts_page::Edit;
    let mut h = Harness::new();
    let at = view_middle(&h);
    h.app.options_window = true;
    h.app.options_goto = Some(super::shortcuts_page::CATEGORY);
    h.frame(vec![]);
    h.frame(vec![]);
    assert!(
        h.texts().iter().any(|t| t == "Pan (hold and drag)"),
        "{:?}",
        h.texts()
    );
    // Add… for Zoom to Drawing, then the thumb button: it is set, and runs nothing yet
    h.app
        .apply_shortcut_edit(Edit::Start("zoom_to_drawing".to_owned()));
    h.frame(vec![mouse(PointerButton::Extra1, at, true)]);
    h.frame(vec![mouse(PointerButton::Extra1, at, false)]);
    assert_eq!(h.app.capture, None, "done waiting");
    let back = shortcut::parse_trigger("Mouse Back").unwrap();
    assert!(h.app.shortcuts_of("zoom_to_drawing").contains(&back));
    // the same button for Pan (hold and drag): it asks first
    h.app
        .apply_shortcut_edit(Edit::Start(shortcut::PAN_DRAG.to_owned()));
    h.frame(vec![mouse(PointerButton::Extra1, at, true)]);
    let asking = h.app.capture.clone().expect("still asking");
    assert_eq!(asking.clash, Some((back, "zoom_to_drawing".to_owned())));
    h.frame(vec![]);
    assert!(
        h.texts()
            .iter()
            .any(|t| t.contains("Mouse Back already runs")),
        "{:?}",
        h.texts()
    );
    h.app.apply_shortcut_edit(Edit::Move);
    assert!(h.app.shortcuts_of(shortcut::PAN_DRAG).contains(&back));
    assert!(!h.app.shortcuts_of("zoom_to_drawing").contains(&back));
    // a key can't pan; Esc stops waiting, changing nothing
    h.app
        .apply_shortcut_edit(Edit::Start(shortcut::PAN_DRAG.to_owned()));
    h.frame(vec![key(Key::K)]);
    assert!(h.app.capture.as_ref().unwrap().refused.is_some());
    h.frame(vec![key(Key::Escape)]);
    assert_eq!(h.app.capture, None);
    assert!(h.app.options_window, "Esc only stopped the waiting");
    // a key for an action, then Reset puts its own back
    h.app
        .apply_shortcut_edit(Edit::Start("zoom_to_drawing".to_owned()));
    // (another key: K is still held, so egui reads it again as a repeat)
    h.frame(vec![key(Key::F8)]);
    assert_eq!(h.app.capture, None, "F8 runs nothing else");
    assert!(
        h.app
            .shortcuts_of("zoom_to_drawing")
            .contains(&shortcut::parse_trigger("F8").unwrap())
    );
    h.app
        .apply_shortcut_edit(Edit::Reset("zoom_to_drawing".to_owned()));
    assert!(
        !h.app
            .options
            .controls
            .shortcuts
            .contains_key("zoom_to_drawing")
    );
}

#[test]
fn ctrl_c_taken_away_in_options_no_longer_copies() {
    let mut h = Harness::new();
    h.rectangle(10.0, 10.0, 20.0, 20.0);
    h.app.doc.selection = h.app.doc.vectors.keys().copied().collect();
    let ctrl_c = shortcut::parse_trigger("Ctrl+C").unwrap();
    h.app.remove_shortcut("copy", ctrl_c);
    h.apply_options();
    h.frame(vec![Event::Copy]);
    assert!(h.app.clipboard.is_empty(), "Ctrl+C runs nothing now");
    h.app.add_shortcut("copy", ctrl_c);
    h.apply_options();
    h.frame(vec![Event::Copy]);
    assert!(!h.app.clipboard.is_empty(), "given back, it copies");
}

#[test]
fn reset_asks_before_taking_back_a_key_another_action_now_has() {
    use super::shortcuts_page::Edit;
    let mut h = Harness::new();
    let ctrl_z = shortcut::parse_trigger("Ctrl+Z").unwrap();
    h.app.remove_shortcut("undo", ctrl_z);
    h.app.add_shortcut("draw_circle", ctrl_z);
    h.app.apply_shortcut_edit(Edit::Reset("undo".into()));
    let clash = h.app.capture.as_ref().and_then(|c| c.clash.clone());
    assert_eq!(
        clash,
        Some((ctrl_z, "draw_circle".to_owned())),
        "asked, as adding the key would"
    );
    assert!(
        !h.app.shortcuts_of("undo").contains(&ctrl_z),
        "not two actions on one key meanwhile"
    );
    h.app.apply_shortcut_edit(Edit::Move);
    assert!(h.app.shortcuts_of("undo").contains(&ctrl_z));
    assert!(!h.app.shortcuts_of("draw_circle").contains(&ctrl_z));
    assert!(
        !h.app.options.controls.shortcuts.contains_key("undo"),
        "undo is back to its own"
    );
}

#[test]
fn f1_f2_and_f3_are_shortcuts_like_any_other() {
    let mut h = Harness::new();
    let f2 = shortcut::parse_trigger("F2").unwrap();
    let bindings = h.app.bindings(&h.ctx);
    assert_eq!(
        shortcut::clash(&bindings, f2, "draw_circle"),
        Some("view_2d"),
        "setting F2 for another action is asked about"
    );
    let rows = h.app.shortcut_rows("F1");
    assert!(
        rows.iter()
            .any(|r| r.id == "tool_help" && r.label == "Help on the Open Tool"),
        "listed by name"
    );
    // still work
    h.app.view = View::ThreeD;
    h.frame(vec![key(Key::F2)]);
    assert_eq!(h.app.view, View::TwoD);
    h.frame(vec![key(Key::F3)]);
    assert_eq!(h.app.view, View::ThreeD);
    // moved to F8, F2 no longer switches
    h.app.remove_shortcut("view_2d", f2);
    h.apply_options();
    h.app
        .add_shortcut("view_2d", shortcut::parse_trigger("F8").unwrap());
    h.apply_options();
    h.frame(vec![key(Key::F2)]);
    assert_eq!(h.app.view, View::ThreeD);
    h.frame(vec![key(Key::F8)]);
    assert_eq!(h.app.view, View::TwoD);
}
