//! 3D models: importing and placing them, components, sculpting, reliefs and textures.

use super::*;

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

/// Relief from Photo's Create makes the relief and shows it in 3D; from then on it follows
/// the form. Before, the form had no button and nothing made the first relief, so nothing
/// ever happened (Matthew, 2026-11-22).
#[test]
fn relief_from_photos_create_button_makes_the_relief_and_shows_it_in_3d() {
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
    assert!(
        h.app.doc.components.is_empty(),
        "nothing made before Create"
    );
    let create = h.text_at("Create").expect("a Create button");
    h.click(create, egui::Modifiers::NONE);
    h.frame(vec![]);
    assert_eq!(h.app.doc.components.len(), 1, "Create made the relief");
    assert_eq!(h.app.view, super::View::ThreeD, "shown in 3D");
    assert!(h.text_at("Create").is_none(), "it now follows the form");
}

/// A colour photo with no depth yet: Create says what to do first, on the form.
#[test]
fn relief_from_photo_without_depth_says_to_estimate_it_first() {
    use cut_craft_core::bitmap::Bitmap;
    use cut_craft_core::geometry::{Point, Rect};
    let mut h = Harness::new();
    let mut c = Change::builder(&h.app.doc);
    c.add_bitmap(Bitmap {
        name: "photo".into(),
        columns: 2,
        rows: 1,
        grey: vec![30, 220],
        rect: Rect::new(Point::new(0.0, 0.0), Point::new(20.0, 10.0)),
        depth: Vec::new(),
        rgb: vec![10, 20, 30, 200, 210, 220],
    });
    h.app.record(Ok(c.finish()), "added");
    h.app.invoke(&h.ctx, "relief_from_photo");
    h.frame(vec![]);
    let create = h.text_at("Create").expect("a Create button");
    h.click(create, egui::Modifiers::NONE);
    h.frame(vec![]);
    assert!(h.app.doc.components.is_empty());
    let error = h
        .app
        .form
        .as_ref()
        .and_then(|f| f.error.clone())
        .unwrap_or_default();
    assert!(error.contains("Estimate Depth"), "{error}");
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
    h.app.trust.ask = |_| crate::trust::Answer::RunThisTime;
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

/// Estimate Depth on a colour photo, nothing made yet: the relief is made as soon as the
/// depth comes back, and shown in 3D, so the button does what its name promises.
#[cfg(unix)]
#[test]
fn estimate_depth_makes_the_relief_when_none_is_made_yet() {
    use cut_craft_core::bitmap::Bitmap;
    use cut_craft_core::geometry::{Point, Rect};
    use std::os::unix::fs::PermissionsExt;
    let addons = std::env::temp_dir().join(format!("cutcraft-addons-make-{}", std::process::id()));
    let dir = addons.join(crate::depth::ADDON_ID);
    std::fs::create_dir_all(&dir).unwrap();
    let answer = dir.join("answer.png");
    image::ImageBuffer::<image::Luma<u16>, _>::from_raw(2, 2, vec![0_u16, 65_535, 30_000, 50_000])
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
    h.app.trust.ask = |_| crate::trust::Answer::RunThisTime;
    let mut c = Change::builder(&h.app.doc);
    c.add_bitmap(Bitmap {
        name: "photo".into(),
        columns: 2,
        rows: 2,
        grey: vec![30, 220, 90, 160],
        rect: Rect::new(Point::new(0.0, 0.0), Point::new(20.0, 20.0)),
        depth: Vec::new(),
        rgb: vec![30, 30, 30, 220, 220, 220, 90, 90, 90, 160, 160, 160],
    });
    h.app.record(Ok(c.finish()), "added");
    h.app.invoke(&h.ctx, "relief_from_photo");
    h.frame(vec![]);
    let form = h.app.form.as_mut().unwrap();
    form.params["background_cut_off"] = 0.0.into();
    form.params["depth_cut_off"] = 0.0.into();
    let id = *h.app.doc.bitmaps.keys().next().unwrap();
    let installed = crate::depth::find(&addons).expect("installed");
    let photo = h.app.doc.bitmaps[&id].clone();
    h.app.depth_estimate =
        Some(crate::depth::Estimating::start(&installed, id, &photo, &h.ctx).unwrap());
    wait_for_depth(&mut h);
    assert_eq!(
        h.app.doc.bitmaps[&id].depth.len(),
        4,
        "the photo has its depth"
    );
    assert_eq!(h.app.doc.components.len(), 1, "and the relief is made");
    assert_eq!(h.app.view, super::View::ThreeD, "shown in 3D");

    // estimated again, to another depth: the relief is made again from it, still one
    let shape = |h: &Harness| h.app.doc.components.values().next().unwrap().clone();
    let first = shape(&h);
    image::ImageBuffer::<image::Luma<u16>, _>::from_raw(2, 2, vec![65_535_u16, 0, 50_000, 30_000])
        .unwrap()
        .save(&answer)
        .unwrap();
    let photo = h.app.doc.bitmaps[&id].clone();
    h.app.depth_estimate =
        Some(crate::depth::Estimating::start(&installed, id, &photo, &h.ctx).unwrap());
    wait_for_depth(&mut h);
    assert_eq!(h.app.doc.components.len(), 1, "made again in place");
    assert_ne!(shape(&h), first, "the relief follows the new depth");

    // another picture picked while the estimate ran: no relief is made from it
    h.app.form = None;
    h.app.doc.components.clear();
    let mut c = Change::builder(&h.app.doc);
    let other = c.add_bitmap(Bitmap {
        name: "other".into(),
        columns: 2,
        rows: 2,
        grey: vec![1, 2, 3, 4],
        rect: Rect::new(Point::new(30.0, 0.0), Point::new(50.0, 20.0)),
        depth: vec![0.5; 4],
        rgb: Vec::new(),
    });
    h.app.record(Ok(c.finish()), "added");
    h.app.doc.selected_bitmaps = [id].into();
    h.app.invoke(&h.ctx, "relief_from_photo");
    h.frame(vec![]);
    h.app.form.as_mut().unwrap().params["depth_from"] = "estimated".into();
    let photo = h.app.doc.bitmaps[&id].clone();
    h.app.depth_estimate =
        Some(crate::depth::Estimating::start(&installed, id, &photo, &h.ctx).unwrap());
    h.app.doc.selected_bitmaps = [other].into();
    wait_for_depth(&mut h);
    assert!(
        h.app.doc.components.is_empty(),
        "not made from the other picture"
    );
    std::fs::remove_dir_all(addons).unwrap();
}

/// Frames until the Depth add-on has answered, and one more for what follows from it.
#[cfg(unix)]
fn wait_for_depth(h: &mut Harness) {
    for _ in 0..200 {
        if h.app.depth_estimate.is_none() {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(20));
        h.frame(vec![]);
    }
    h.frame(vec![]);
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
        "the combine icons"
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
    // the model sits in the material (Material Setup's gap): its foot is the lowest point
    let foot = surface
        .heights
        .iter()
        .copied()
        .fold(f64::INFINITY, f64::min);
    assert!(top - foot > 1.0, "the dome stands up: {top} over {foot}");
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
        at(foot)
    );
    // and a drag on that plane still moves by what the pointer moved over
    let before = h.app.doc.components[&EntityId(1)].extent();
    let corner = handle_3d_at(&h, Handle3d::Transform(Handle::Scale { x: 1, y: 1 }));
    h.drag(cross, corner);
    let moved = h.app.doc.components[&EntityId(1)].extent();
    assert!((moved.center() - before.max).length() < 0.5, "{moved:?}");
}

#[test]
fn one_click_on_a_model_in_3d_picks_it_and_shows_its_height_buttons() {
    let (mut h, dome) = dome_from_above();
    assert!(h.app.doc.selected_components.is_empty());
    h.click(dome, Modifiers::NONE);
    h.frame(vec![]);
    assert_eq!(h.app.doc.selected_components, [EntityId(1)].into());
    assert_eq!(h.app.view_tool, Some(ViewTool::Transform));
    for which in [Handle3d::Total, Handle3d::Height, Handle3d::Base] {
        assert!(
            h.app.handles_3d.iter().any(|(k, _)| *k == which),
            "{which:?} shows"
        );
    }
    // clicking it again, or double-clicking it, keeps it picked with its buttons
    h.click(dome, Modifiers::NONE);
    h.frame(vec![]);
    assert_eq!(h.app.view_tool, Some(ViewTool::Transform));
    h.double_click(dome);
    h.frame(vec![]);
    assert_eq!(h.app.view_tool, Some(ViewTool::Transform));
    assert_eq!(h.app.doc.selected_components, [EntityId(1)].into());
    // a click off every model lets it go
    let off = h.app.fit_3d.unwrap().left_top() + vec2(4.0, 4.0);
    h.click(off, Modifiers::NONE);
    h.frame(vec![]);
    assert!(h.app.doc.selected_components.is_empty());
    assert_eq!(h.app.handles_3d, vec![], "no buttons with nothing picked");
}

#[test]
fn a_picked_model_is_outlined_in_3d_even_out_of_transform_mode() {
    let (mut h, _) = dome_from_above();
    assert!(h.app.picked_outline.is_none());
    h.app.doc.selected_components = [EntityId(1)].into();
    h.frame(vec![]);
    let (_, rings) = h.app.picked_outline.as_ref().expect("an outline");
    assert!(!rings.is_empty() && rings.iter().all(|r| r.len() > 3));
    h.app.doc.selected_components.clear();
    h.frame(vec![]);
    assert!(h.app.picked_outline.is_none());
    // a new job forgets the old one's outline (its revisions start again)
    h.app.doc.selected_components = [EntityId(1)].into();
    h.frame(vec![]);
    h.app.reset_job();
    assert!(h.app.picked_outline.is_none());
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
    // a double-click on the selected component enters it
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
    let said = |h: &Harness| h.app.status.message.clone();
    assert!(said(&h).starts_with("Base height") && said(&h).ends_with(" mm"));
    // the total height handle scales shape and base together, said in an inch job's inches
    h.app.doc.job.units = cut_craft_core::document::Units::Inch;
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
fn a_vector_behind_the_model_is_not_drawn_over_it() {
    // a tall block on the right half; an outline on the floor left of it
    use cut_craft_core::component::{Combine, Component};
    let mut h = Harness::new();
    // material thick enough for the wall, so it stands whole in it
    h.app.doc.job.thickness = h.app.doc.job.width / 2.0 + 10.0;
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
    let mut h = Harness::at_classic_start();
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
fn orientate_3d_models_rings_turn_plus_the_way_their_plus_arrow_points_and_minus_back() {
    let mut h = Harness::new();
    let file = pyramid_stl("ring-ways");
    h.app.import_file(&file);
    h.app.view = View::ThreeD;
    h.frame(vec![]);
    h.frame(vec![]);
    let turn = |h: &Harness, k: &str| h.app.form.as_ref().unwrap().params["turn"][k].as_f64();
    // the + and − arrows as drawn now: the rings follow the model as it turns
    let marks_now = |h: &Harness, i: usize| {
        let (rings, sphere) = h.app.rings_3d.clone();
        let all: Vec<&[Pos2]> = rings.iter().map(|(_, p)| p.as_slice()).collect();
        let gap = 9.0 * egui::Style::default().spacing.interact_size.y / 2.0;
        let marks = crate::axes::rings_marks(&all, sphere.unwrap().0, gap);
        let (at, along) = marks[i].unwrap();
        (rings[i].1[at], along)
    };
    for (i, key) in ["x", "y", "z"].into_iter().enumerate() {
        // along the ring the + way from its arrows, then back
        let (from, along) = marks_now(&h, i);
        let before = turn(&h, key).unwrap();
        h.drag(from, from + along * 30.0);
        let after = turn(&h, key).unwrap();
        assert!(after > before + 1.0, "{key}: {before} -> {after}");
        let (from, along) = marks_now(&h, i);
        h.drag(from, from - along * 30.0);
        let back = turn(&h, key).unwrap();
        assert!(back < after - 1.0, "{key}: {after} -> {back}");
    }
    let _ = std::fs::remove_file(file);
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

#[test]
fn esc_on_import_3d_model_cancels_the_import() {
    let mut h = Harness::new();
    let file = pyramid_stl("esc-cancels-import");
    h.app.import_file(&file);
    h.frame(vec![]);
    assert_eq!(
        h.app.form.as_ref().map(|f| f.id),
        Some("orientate_3d_model")
    );
    assert!(
        !h.app.doc.components.is_empty(),
        "placed while the form is open"
    );
    h.frame(vec![escape()]);
    assert!(h.app.form.is_none());
    assert!(
        h.app.doc.components.is_empty(),
        "Esc is the form's Cancel: the model is not imported"
    );
    assert!(
        h.app.status.message.contains("cancelled"),
        "{}",
        h.app.status.message
    );
    let _ = std::fs::remove_file(file);
}

/// A model that could not be placed is not imported: Import waits for a placement that
/// worked, and the form stays open with what went wrong, for Cancel or new settings.
#[test]
fn import_waits_for_a_model_that_was_placed() {
    let dir = fresh_dir("unplaceable");
    let file = dir.join("nothing.stl");
    std::fs::write(&file, "solid nothing\nendsolid nothing\n").unwrap();
    let mut h = Harness::new();
    h.app.import_file(&file);
    h.frame(vec![]);
    h.frame(vec![]);
    let _ = std::fs::remove_dir_all(&dir);
    let form = h.app.form.as_ref().expect("the import form is open");
    assert_eq!(form.id, "orientate_3d_model");
    assert!(form.error.is_some(), "it says what went wrong");
    assert!(h.app.doc.components.is_empty());
    let at = h.text_at("Import").expect("an Import button, greyed");
    h.click(at, Modifiers::NONE);
    h.frame(vec![]);
    assert!(
        !h.app.status.message.contains("Model imported"),
        "{}",
        h.app.status.message
    );
    assert!(
        h.app.form.is_some(),
        "the form stays for Cancel or new settings"
    );
}

#[test]
fn ctrl_v_of_another_windows_vectors_leaves_out_this_windows_old_components() {
    let mut h = Harness::new();
    let domed = cut_craft_core::testing::domed();
    let component = domed.components.values().next().unwrap().clone();
    // copied here earlier: a rectangle and a component
    h.rectangle(10.0, 10.0, 20.0, 20.0);
    h.app.doc.selection = h.app.doc.vectors.keys().copied().collect();
    h.app.invoke(&h.ctx, "copy");
    h.app.clipboard_components = vec![component];
    let before = h.app.doc.components.len();
    // another window's circle comes in through the system clipboard
    let circle = vec![Path::circle(Point::new(50.0, 50.0), 5.0)];
    h.frame(vec![Event::Paste(serde_json::to_string(&circle).unwrap())]);
    assert_eq!(h.app.doc.vectors.len(), 2, "the circle is pasted");
    assert_eq!(
        h.app.doc.components.len(),
        before,
        "no component from this window's old copy comes with it"
    );
}

#[test]
fn baking_component_properties_resets_the_form_so_apply_again_does_not_bake_twice() {
    let mut h = Harness::new();
    h.app.doc = cut_craft_core::testing::domed();
    let top = |h: &Harness| h.app.doc.components[&EntityId(1)].height_range().unwrap().1;
    let high = top(&h);
    h.app.invoke(&h.ctx, "component_properties");
    h.frame(vec![]);
    let form = h.app.form.as_mut().expect("Component Properties is open");
    form.params["shape_height"] = serde_json::json!(150.0);
    form.params["bake"] = serde_json::json!(true);
    let params = form.params.clone();
    h.app.apply_form("component_properties", &params).unwrap();
    assert!((top(&h) - high * 1.5).abs() < 1e-6, "baked once");
    let form = h.app.form.as_ref().expect("still open");
    assert_eq!(
        form.params["bake"],
        serde_json::json!(false),
        "Bake unticked"
    );
    assert_eq!(form.params["shape_height"], serde_json::json!(100.0));
    let params = form.params.clone();
    h.app.apply_form("component_properties", &params).unwrap();
    assert!(
        (top(&h) - high * 1.5).abs() < 1e-6,
        "not baked a second time"
    );
}

#[test]
fn the_status_bar_names_the_level_a_new_component_went_on() {
    let mut h = Harness::new();
    h.rectangle(10.0, 10.0, 50.0, 50.0);
    let shape = h.app.registry.get("create_shape").unwrap().default_params();
    h.app.apply("create_shape", &shape).unwrap();
    assert!(
        h.app.status.message.contains("on level Level 1"),
        "{}",
        h.app.status.message
    );
}

#[test]
fn the_status_bar_names_no_level_when_a_tool_replaces_components() {
    let mut h = Harness::new();
    h.rectangle(10.0, 10.0, 50.0, 50.0);
    let shape = h.app.registry.get("create_shape").unwrap().default_params();
    h.app.apply("create_shape", &shape).unwrap();
    // Split replaces the component it cuts by two pieces: not a new component
    let piece = *h.app.doc.components.keys().next_back().unwrap();
    h.rectangle(0.0, 0.0, 30.0, 30.0);
    h.app.doc.selected_components = [piece].into_iter().collect();
    let none = h.app.registry.get("split").unwrap().default_params();
    h.app.apply("split", &none).unwrap();
    let said = h.app.status.message.clone();
    assert!(said.starts_with("Split Component applied"), "{said}");
    assert!(!said.contains("level"), "{said}");
}

#[test]
fn only_new_components_all_on_one_level_have_their_level_named() {
    use cut_craft_core::component::{Component, Level};
    let mut h = Harness::new();
    h.rectangle(10.0, 10.0, 50.0, 50.0);
    let shape = h.app.registry.get("create_shape").unwrap().default_params();
    h.app.apply("create_shape", &shape).unwrap();
    let doc = &h.app.doc;
    let made = doc.components.values().next().unwrap().clone();
    let on = |level| Component {
        level,
        ..made.clone()
    };
    // one new component: its level
    let mut c = Change::builder(doc);
    c.add_component(on(0));
    assert_eq!(CutCraftApp::new_components_level(&c.finish()), Some(0));
    // new components shared over levels (Slice Model): none named
    let mut c = Change::builder(doc);
    c.add_component(on(0));
    c.add_component(on(1));
    assert_eq!(CutCraftApp::new_components_level(&c.finish()), None);
    // a tool that sets out levels itself (Slice Model, Bake): none named
    let mut c = Change::builder(doc);
    c.set_level(
        1,
        Some(Level {
            name: "Slice 1".into(),
            ..Level::default()
        }),
    );
    c.add_component(on(1));
    assert_eq!(CutCraftApp::new_components_level(&c.finish()), None);
    // a component replaced by remove and add (Split, Emboss, Sculpt): none named
    let mut c = Change::builder(doc);
    c.remove_component(*doc.components.keys().next().unwrap());
    c.add_component(on(0));
    assert_eq!(CutCraftApp::new_components_level(&c.finish()), None);
}
