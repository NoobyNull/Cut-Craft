//! 3D models in the 3D view: a model imported shows at once, picking components there,
//! drawing on the model, and the 3D view's handles.

use super::*;

#[test]
fn an_imported_model_shows_at_once_and_follows_its_form() {
    let mut h = Harness::new();
    let file = pyramid_stl("live-import");
    h.import(&file);
    assert_eq!(
        h.doc().components.len(),
        1,
        "issue #3: the model is in the scene at once"
    );
    let size = &h.form_value("size");
    assert!(
        (size["x"].as_f64().unwrap() - 20.0).abs() < 1.0,
        "Model Size shows its size: {size}"
    );
    // moving it in the form moves the same model
    h.edit_form(|p| {
        p["placing"]["center"] = Value::Bool(false);
        p["placing"]["position"] = serde_json::json!({ "x": 50.0, "y": 60.0 });
    });
    h.touch_form();
    h.frame(vec![]);
    assert_eq!(h.doc().components.len(), 1, "moved, not added");
    let k = h.doc().components.values().next().unwrap();
    let o = h.doc().job.area().min;
    assert!(
        (k.origin.x - 50.0).abs() < 1.0 && (k.origin.y - 60.0).abs() < 1.0,
        "at {:?} of {o:?}",
        k.origin
    );
    // and sizing it sizes it
    h.edit_form(|p| {
        p["size"]["x"] = Value::from(40.0);
    });
    h.touch_form();
    h.frame(vec![]);
    let k = h.doc().components.values().next().unwrap();
    let width = k.cell * f64::from(u32::try_from(k.columns).unwrap());
    assert!((width - 40.0).abs() < 2.0 * k.cell, "{} columns", k.columns);
    let _ = std::fs::remove_file(file);
}

#[test]
fn importing_a_model_shows_orientation_and_position_on_one_page() {
    let mut h = Harness::new();
    let file = pyramid_stl("pages");
    h.import(&file);
    h.frame(vec![]);
    h.frame(vec![]);
    let over = h.text_at("Initial Orientation").unwrap();
    for text in ["Depth Below Top", "Update Undercuts", "Import"] {
        assert!(h.scroll_to(over, text).is_some(), "{text}");
    }
    assert!(h.text_at("< Back").is_none(), "one page");
    assert_eq!(
        h.form_settings()["placing"]["depth_below_top"],
        5.0,
        "the whole model to start with, as its height"
    );
    let tall = |h: &Harness| {
        let k = h.doc().components.values().next().unwrap();
        k.height_range().unwrap().1
    };
    // the apex falls between cells
    let cell = h.doc().job.model_cell();
    assert!((tall(&h) - 5.0).abs() < 2.0 * cell, "{}", tall(&h));
    // the plane halfway down the model keeps its top half
    h.edit_form(|p| {
        p["placing"]["depth_below_top"] = Value::from(2.5);
    });
    h.touch_form();
    h.frame(vec![]);
    assert!((tall(&h) - 2.5).abs() < 2.0 * cell, "{}", tall(&h));
    let import = h.scroll_to(over, "Import").unwrap();
    h.click(import, Modifiers::NONE);
    assert!(h.form_id().is_none(), "Import closes the form");
    assert_eq!(h.doc().components.len(), 1, "keeping the model");
    let _ = std::fs::remove_file(file);
}

#[test]
fn a_double_click_in_the_3d_view_picks_the_component_under_it() {
    let mut h = Harness::new();
    h.open_document(cut_craft_core::testing::domed());
    h.given_orbit(view3d::Orbit::TOP);
    let fit = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(400.0, 400.0));
    let surface = h.surface_3d().unwrap();
    let screen = view3d::screen_of(&surface, h.orbit(), h.framing_3d(), fit, None);
    let local = |x: f64, y: f64| {
        let z = surface
            .at(cut_craft_core::geometry::Point::new(x, y))
            .unwrap();
        cut_craft_core::glam::DVec3::new(x - surface.origin.x, y - surface.origin.y, z)
    };
    assert_eq!(
        h.component_at_3d(fit, screen(local(20.0, 20.0))),
        Some(EntityId(1))
    );
    assert_eq!(
        h.component_at_3d(fit, screen(local(2.0, 2.0))),
        None,
        "off the dome"
    );
}

#[test]
fn right_clicking_a_component_in_the_3d_view_picks_it_and_opens_its_menu() {
    let mut h = Harness::new();
    h.open_document(cut_craft_core::testing::domed());
    h.given_orbit(view3d::Orbit::TOP);
    h.show_view(View::ThreeD);
    h.frame(vec![]);
    h.frame(vec![]);
    let fit = h.fit_3d();
    let surface = h.surface_3d().unwrap();
    let screen = view3d::screen_of(&surface, h.orbit(), h.framing_3d(), fit, None);
    let z = surface.at(Point::new(20.0, 20.0)).unwrap();
    let top = screen(cut_craft_core::glam::DVec3::new(
        20.0 - surface.origin.x,
        20.0 - surface.origin.y,
        z,
    ));
    h.right_click(top);
    assert!(h.doc().selected_components.contains(&EntityId(1)));
    assert_eq!(h.under_pointer_3d(), [EntityId(1)]);
    assert!(h.text_at("dome").is_some(), "the menu, headed by its name");
    assert!(
        leaf(&h.view_3d_menu_here(), &["dome"]).is_checked,
        "listed as selected"
    );
    assert!(
        !leaf(&h.view_3d_menu_here(), &["Node Edit Mode"]).is_enabled,
        "a vector item, greyed for a component"
    );
    assert!(
        leaf(
            &h.view_3d_menu_here(),
            &["Component", "Combine Mode", "Add"]
        )
        .is_checked
    );
    h.choose_on_view_3d_menu(&["Component", "Create Vector Boundary"]);
    assert_eq!(h.doc().vectors.len(), 1, "shape to vector outline");
    h.choose_on_view_3d_menu(&["Copy"]);
    h.choose_on_view_3d_menu(&["Paste"]);
    assert_eq!(h.doc().components.len(), 2, "components paste too");
    h.select_components([EntityId(1)]);
    h.choose_on_view_3d_menu(&["Delete"]);
    assert_eq!(h.doc().components.len(), 1);
}

#[test]
fn a_drag_with_a_drawing_tool_in_the_3d_view_draws_where_it_meets_the_model() {
    let (mut h, on_model, cell) = drawing_on_the_dome("draw_rectangle");
    let turned = h.orbit();
    // on the dome's top, well up off the material's top plane
    h.drag(on_model(18.0, 18.0), on_model(23.0, 21.0));
    assert_eq!(
        h.orbit(),
        turned,
        "the left button draws, it does not turn the view"
    );
    assert_eq!(h.doc().vectors.len(), 1, "{:?}", h.form_error());
    let b = h.doc().vectors.values().next().unwrap().bounds();
    let close = |a: Point, b: Point| a.distance(b) < 1.5 * cell;
    assert!(
        close(b.min, Point::new(18.0, 18.0)) && close(b.max, Point::new(23.0, 21.0)),
        "{b:?}, cells of {cell}"
    );
    assert!(h.can_undo());
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
    assert_ne!(h.orbit(), turned);
    assert_eq!(h.doc().vectors.len(), 1, "turning draws nothing");
}

#[test]
fn drawing_in_the_3d_view_meets_the_model_as_finely_as_the_settled_view_draws_it() {
    let (mut h, _, _) = drawing_on_the_dome("draw_rectangle");
    let fine = h.fine_surface();
    let hit = h.drawing_surface().expect("a model to draw on");
    assert_eq!((hit.columns, hit.rows), (fine.columns, fine.rows));
    // cells off the model are NaN, so compare bit for bit
    let bits = |m: &Material| m.heights.iter().map(|h| h.to_bits()).collect::<Vec<_>>();
    assert_eq!(bits(&hit), bits(&fine));
    let again = h.drawing_surface().unwrap();
    assert!(
        std::sync::Arc::ptr_eq(&hit, &again),
        "kept while nothing changes"
    );
    h.select_components([EntityId(1)]);
    h.invoke("delete");
    assert!(h.drawing_surface().is_none(), "nothing left to draw on");
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
    assert_eq!(h.doc().vectors.len(), 1);
    let path = h.doc().vectors.values().next().unwrap();
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
    let fit = h.fit_3d();
    let surface = h.surface_3d().unwrap();
    let top = surface.at(Point::new(20.0, 20.0)).unwrap();
    // the model sits in the material (Material Setup's gap): its foot is the lowest point
    let foot = surface
        .heights
        .iter()
        .copied()
        .fold(f64::INFINITY, f64::min);
    assert!(top - foot > 1.0, "the dome stands up: {top} over {foot}");
    let camera = view3d::Camera::new(&surface, h.orbit(), h.framing_3d(), fit, None);
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
        at(foot)
    );
    // and a drag on that plane still moves by what the pointer moved over
    let before = h.doc().components[&EntityId(1)].extent();
    let corner = handle_3d_at(&h, Handle3d::Transform(Handle::Scale { x: 1, y: 1 }));
    h.drag(cross, corner);
    let moved = h.doc().components[&EntityId(1)].extent();
    assert!((moved.center() - before.max).length() < 0.5, "{moved:?}");
}

#[test]
fn one_click_on_a_model_in_3d_picks_it_and_shows_its_height_buttons() {
    let (mut h, dome) = dome_from_above();
    assert!(h.doc().selected_components.is_empty());
    h.click(dome, Modifiers::NONE);
    h.frame(vec![]);
    assert_eq!(h.doc().selected_components, [EntityId(1)].into());
    assert_eq!(h.view_tool(), Some(ViewTool::Transform));
    for which in [Handle3d::Total, Handle3d::Height, Handle3d::Base] {
        assert!(
            h.handles_3d().iter().any(|(k, _)| *k == which),
            "{which:?} shows"
        );
    }
    // clicking it again, or double-clicking it, keeps it picked with its buttons
    h.click(dome, Modifiers::NONE);
    h.frame(vec![]);
    assert_eq!(h.view_tool(), Some(ViewTool::Transform));
    h.double_click(dome);
    h.frame(vec![]);
    assert_eq!(h.view_tool(), Some(ViewTool::Transform));
    assert_eq!(h.doc().selected_components, [EntityId(1)].into());
    // a click off every model lets it go
    let off = h.fit_3d().left_top() + vec2(4.0, 4.0);
    h.click(off, Modifiers::NONE);
    h.frame(vec![]);
    assert!(h.doc().selected_components.is_empty());
    assert_eq!(h.handles_3d(), vec![], "no buttons with nothing picked");
}

#[test]
fn a_picked_model_is_outlined_in_3d_even_out_of_transform_mode() {
    let (mut h, _) = dome_from_above();
    assert!(h.picked_outline().is_none());
    h.select_components([EntityId(1)]);
    h.frame(vec![]);
    let rings = h.picked_outline().expect("an outline");
    assert!(!rings.is_empty() && rings.iter().all(|r| r.len() > 3));
    h.select_components([]);
    h.frame(vec![]);
    assert!(h.picked_outline().is_none());
    // a new job forgets the old one's outline (its revisions start again)
    h.select_components([EntityId(1)]);
    h.frame(vec![]);
    h.reset_job();
    assert!(h.picked_outline().is_none());
}

#[test]
fn the_3d_views_handles_scale_the_selected_component_and_open_its_settings() {
    use cut_craft_core::component::Combine;
    let mut h = Harness::new();
    h.open_document(cut_craft_core::testing::domed());
    h.select_components([EntityId(1)]);
    h.given_orbit(view3d::Orbit::TOP);
    h.show_view(View::ThreeD);
    h.frame(vec![]);
    h.frame(vec![]);
    assert!(h.handles_3d().is_empty(), "not until Transform mode");
    // a double-click on the selected component enters it
    let fit = h.fit_3d();
    let surface = h.surface_3d().unwrap();
    let screen = view3d::screen_of(&surface, h.orbit(), h.framing_3d(), fit, None);
    let z = surface.at(Point::new(20.0, 20.0)).unwrap();
    h.double_click(screen(cut_craft_core::glam::DVec3::new(
        20.0 - surface.origin.x,
        20.0 - surface.origin.y,
        z,
    )));
    assert_eq!(h.view_tool(), Some(ViewTool::Transform));
    h.frame(vec![]);
    let at = |h: &Harness, which: Handle3d| {
        h.handles_3d()
            .iter()
            .find(|(k, _)| *k == which)
            .map(|(_, at)| *at)
            .expect("the handles are drawn over the selected component")
    };
    let dome = |h: &Harness| h.doc().components[&EntityId(1)].clone();
    // dragging the height handle up a quarter of the view doubles the shape height; the
    // component follows the drag, and the whole drag is one undo
    let quarter = h.fit_3d().height() / VIEW_ZOOM_QUARTERS;
    let depth = h.undo_steps();
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
    assert_eq!(h.undo_steps(), depth + 1, "one undo for the drag");
    assert!(!h.dragging_3d_handle(), "let go");
    h.invoke("undo");
    assert_eq!(dome(&h).shaping.shape_height, 100.0, "undoable");
    // the view is fitted to the model again, so the handles move
    h.frame(vec![]);
    // the base handle raises it on a solid bottom
    let from = at(&h, Handle3d::Base);
    h.drag(from, from - vec2(0.0, quarter));
    let base = dome(&h).shaping.base_height;
    assert!((base - h.doc().job.thickness).abs() < 0.5, "{base}");
    let said = |h: &Harness| h.status_message().to_owned();
    assert!(said(&h).starts_with("Base height") && said(&h).ends_with(" mm"));
    // the total height handle scales shape and base together, said in an inch job's inches
    h.given_document(|d| d.job.units = cut_craft_core::document::Units::Inch);
    h.frame(vec![]);
    let before = dome(&h);
    let from = at(&h, Handle3d::Total);
    h.drag(from, from - vec2(0.0, quarter));
    assert!(said(&h).starts_with("Total height") && said(&h).ends_with(" in"));
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
    let subtract = h.combine_radio_at(Combine::Subtract);
    h.click(subtract, Modifiers::NONE);
    h.frame(vec![]);
    assert_eq!(dome(&h).combine, Combine::Subtract);
    let fade = h.text_at("Fade").unwrap();
    h.click(fade, Modifiers::NONE);
    h.frame(vec![]);
    assert!(dome(&h).shaping.fade.is_some());
    h.invoke("undo");
    assert!(dome(&h).shaping.fade.is_none(), "one undo each");
    let close = h.text_at("Close").unwrap();
    h.click(close, Modifiers::NONE);
    h.frame(vec![]);
    assert!(h.text_at("Fade").is_none(), "closed");
}
