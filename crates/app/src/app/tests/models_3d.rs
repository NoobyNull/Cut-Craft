//! 3D models: importing and placing them, components, sculpting, reliefs and textures.

use super::*;

#[test]
fn importing_models_offers_what_the_options_say() {
    use crate::options::ModelTypes;
    let mut h = Harness::new();
    h.invoke("orientate_3d_model");
    assert_eq!(
        h.form_settings()["offered"],
        "full",
        "full alone, by default"
    );
    assert_eq!(h.form_settings()["model_type"], "full");
    h.close_form();
    h.given_options(|o| o.import.model_types = ModelTypes::Flat);
    h.invoke("orientate_3d_model");
    assert_eq!(h.form_value("model_type"), "flat");
    h.close_form();
    h.given_options(|o| o.import.model_types = ModelTypes::Both);
    h.invoke("orientate_3d_model");
    assert_eq!(h.form_settings()["offered"], "both");
    assert_eq!(
        h.form_settings()["model_type"],
        "full",
        "the choice starts on Full"
    );
    h.frame(vec![]);
    h.frame(vec![]);
    assert!(h.text_at("Full 3D model").is_some(), "the choice is shown");
}

#[test]
fn clicking_a_component_in_the_2d_view_selects_it() {
    let mut h = Harness::new();
    h.open_document(cut_craft_core::testing::domed());
    h.frame(vec![]);
    h.click(h.at(20.0, 20.0), Modifiers::NONE);
    assert_eq!(
        h.doc().selected_components.len(),
        1,
        "the dome under the click"
    );
    h.click(h.at(2.0, 2.0), Modifiers::NONE);
    assert!(h.doc().selected_components.is_empty(), "off the dome");
}

#[test]
fn a_sculpting_drag_deposits_along_the_stroke() {
    let mut h = Harness::new();
    h.open_document(cut_craft_core::testing::domed());
    h.frame(vec![]);
    let top = |h: &Harness| {
        h.doc()
            .components
            .values()
            .next()
            .unwrap()
            .height_at(Point::new(20.0, 20.0))
    };
    let before = top(&h).unwrap();
    h.invoke("sculpt");
    h.edit_form(|p| p["tool"] = Value::from("deposit"));
    h.frame(vec![Event::PointerMoved(h.at(16.0, 20.0))]);
    h.press(h.at(16.0, 20.0), true, Modifiers::NONE);
    for x in [18.0, 20.0, 22.0, 24.0] {
        h.frame(vec![Event::PointerMoved(h.at(x, 20.0))]);
    }
    h.press(h.at(24.0, 20.0), false, Modifiers::NONE);
    h.frame(vec![]);
    assert!(top(&h).unwrap() > before, "the stroke raised the dome");
    assert!(h.doc().vectors.is_empty(), "sculpting draws no vectors");
}

/// Relief from Photo follows its settings once applied, as depth2stl's preview did: one
/// relief, made again as a setting changes, never a pile of them.
#[test]
fn relief_from_photo_follows_its_settings_as_one_relief() {
    use cut_craft_core::bitmap::Bitmap;
    use cut_craft_core::geometry::{Point, Rect};
    let mut h = Harness::new();
    let mut c = Change::builder(h.doc());
    c.add_bitmap(Bitmap {
        name: "map".into(),
        columns: 4,
        rows: 4,
        grey: vec![0, 0, 0, 0, 0, 120, 255, 0, 0, 180, 220, 0, 0, 0, 0, 0],
        rect: Rect::new(Point::new(0.0, 0.0), Point::new(40.0, 40.0)),
        depth: Vec::new(),
        rgb: Vec::new(),
    });
    h.make(c.finish(), "added");
    h.invoke("relief_from_photo");
    h.frame(vec![]);
    h.edit_form(|p| {
        p["depth_from"] = "picture".into();
        p["max_height"] = 10.0.into();
    });
    let params = h.form_settings().clone();
    h.apply_form("relief_from_photo", &params).unwrap();
    assert_eq!(h.doc().components.len(), 1);
    let top = |h: &Harness| {
        let k = h.doc().components.values().next().unwrap();
        k.height_range().unwrap().1
    };
    let before = top(&h);
    h.edit_form(|p| {
        p["max_height"] = 20.0.into();
    });
    h.touch_form();
    h.frame(vec![]);
    assert_eq!(h.doc().components.len(), 1, "made again in place");
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
    let mut c = Change::builder(h.doc());
    c.add_bitmap(Bitmap {
        name: "map".into(),
        columns: 4,
        rows: 4,
        grey: vec![0, 0, 0, 0, 0, 120, 255, 0, 0, 180, 220, 0, 0, 0, 0, 0],
        rect: Rect::new(Point::new(0.0, 0.0), Point::new(40.0, 40.0)),
        depth: Vec::new(),
        rgb: Vec::new(),
    });
    h.make(c.finish(), "added");
    h.invoke("relief_from_photo");
    h.frame(vec![]);
    assert!(h.doc().components.is_empty(), "nothing made before Create");
    let create = h.text_at("Create").expect("a Create button");
    h.click(create, egui::Modifiers::NONE);
    h.frame(vec![]);
    assert_eq!(h.doc().components.len(), 1, "Create made the relief");
    assert_eq!(*h.view(), super::View::ThreeD, "shown in 3D");
    assert!(h.text_at("Create").is_none(), "it now follows the form");
}

/// A colour photo with no depth yet: Create says what to do first, on the form.
#[test]
fn relief_from_photo_without_depth_says_to_estimate_it_first() {
    use cut_craft_core::bitmap::Bitmap;
    use cut_craft_core::geometry::{Point, Rect};
    let mut h = Harness::new();
    let mut c = Change::builder(h.doc());
    c.add_bitmap(Bitmap {
        name: "photo".into(),
        columns: 2,
        rows: 1,
        grey: vec![30, 220],
        rect: Rect::new(Point::new(0.0, 0.0), Point::new(20.0, 10.0)),
        depth: Vec::new(),
        rgb: vec![10, 20, 30, 200, 210, 220],
    });
    h.make(c.finish(), "added");
    h.invoke("relief_from_photo");
    h.frame(vec![]);
    let create = h.text_at("Create").expect("a Create button");
    h.click(create, egui::Modifiers::NONE);
    h.frame(vec![]);
    assert!(h.doc().components.is_empty());
    let error = h.form_error().unwrap_or_default().to_owned();
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
    h.given_add_ons_folder(addons.clone());
    h.given_unsigned_runs_this_time();
    let mut c = Change::builder(h.doc());
    c.add_bitmap(Bitmap {
        name: "photo".into(),
        columns: 2,
        rows: 1,
        grey: vec![30, 220],
        rect: Rect::new(Point::new(0.0, 0.0), Point::new(20.0, 10.0)),
        depth: Vec::new(),
        rgb: Vec::new(),
    });
    h.make(c.finish(), "added");
    h.invoke("relief_from_photo");
    h.frame(vec![]);
    let id = *h.doc().bitmaps.keys().next().unwrap();
    let installed = crate::depth::find(&addons).expect("installed");
    let photo = h.doc().bitmaps[&id].clone();
    h.start_depth_estimate(&installed, id, &photo);
    for _ in 0..200 {
        if !h.estimating_depth() {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(20));
        h.frame(vec![]);
    }
    assert!(!h.estimating_depth(), "the add-on answered");
    assert_eq!(h.doc().bitmaps[&id].depth, [0.0, 1.0]);
    assert!(h.undo_step());
    assert!(h.doc().bitmaps[&id].depth.is_empty(), "undone");
    // an answer for a picture that has changed since (another job's, sharing its id) is
    // turned away
    let mut other = photo.clone();
    other.grey = vec![90, 90];
    let mut c = Change::builder(h.doc());
    c.replace_bitmap(id, other);
    h.make(c.finish(), "edited");
    h.start_depth_estimate(&installed, id, &photo);
    for _ in 0..200 {
        if !h.estimating_depth() {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(20));
        h.frame(vec![]);
    }
    assert!(!h.estimating_depth(), "the add-on answered");
    assert!(
        h.doc().bitmaps[&id].depth.is_empty(),
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
    h.given_add_ons_folder(addons.clone());
    h.given_unsigned_runs_this_time();
    let mut c = Change::builder(h.doc());
    c.add_bitmap(Bitmap {
        name: "photo".into(),
        columns: 2,
        rows: 2,
        grey: vec![30, 220, 90, 160],
        rect: Rect::new(Point::new(0.0, 0.0), Point::new(20.0, 20.0)),
        depth: Vec::new(),
        rgb: vec![30, 30, 30, 220, 220, 220, 90, 90, 90, 160, 160, 160],
    });
    h.make(c.finish(), "added");
    h.invoke("relief_from_photo");
    h.frame(vec![]);
    h.edit_form(|p| {
        p["background_cut_off"] = 0.0.into();
        p["depth_cut_off"] = 0.0.into();
    });
    let id = *h.doc().bitmaps.keys().next().unwrap();
    let installed = crate::depth::find(&addons).expect("installed");
    let photo = h.doc().bitmaps[&id].clone();
    h.start_depth_estimate(&installed, id, &photo);
    wait_for_depth(&mut h);
    assert_eq!(
        h.doc().bitmaps[&id].depth.len(),
        4,
        "the photo has its depth"
    );
    assert_eq!(h.doc().components.len(), 1, "and the relief is made");
    assert_eq!(*h.view(), super::View::ThreeD, "shown in 3D");

    // estimated again, to another depth: the relief is made again from it, still one
    let shape = |h: &Harness| h.doc().components.values().next().unwrap().clone();
    let first = shape(&h);
    image::ImageBuffer::<image::Luma<u16>, _>::from_raw(2, 2, vec![65_535_u16, 0, 50_000, 30_000])
        .unwrap()
        .save(&answer)
        .unwrap();
    let photo = h.doc().bitmaps[&id].clone();
    h.start_depth_estimate(&installed, id, &photo);
    wait_for_depth(&mut h);
    assert_eq!(h.doc().components.len(), 1, "made again in place");
    assert_ne!(shape(&h), first, "the relief follows the new depth");

    // another picture picked while the estimate ran: no relief is made from it
    h.close_form();
    h.given_document(|d| d.components.clear());
    let mut c = Change::builder(h.doc());
    let other = c.add_bitmap(Bitmap {
        name: "other".into(),
        columns: 2,
        rows: 2,
        grey: vec![1, 2, 3, 4],
        rect: Rect::new(Point::new(30.0, 0.0), Point::new(50.0, 20.0)),
        depth: vec![0.5; 4],
        rgb: Vec::new(),
    });
    h.make(c.finish(), "added");
    h.given_document(|d| d.selected_bitmaps = [id].into());
    h.invoke("relief_from_photo");
    h.frame(vec![]);
    h.edit_form(|p| p["depth_from"] = "estimated".into());
    let photo = h.doc().bitmaps[&id].clone();
    h.start_depth_estimate(&installed, id, &photo);
    h.given_document(|d| d.selected_bitmaps = [other].into());
    wait_for_depth(&mut h);
    assert!(
        h.doc().components.is_empty(),
        "not made from the other picture"
    );
    std::fs::remove_dir_all(addons).unwrap();
}

/// Frames until the Depth add-on has answered, and one more for what follows from it.
#[cfg(unix)]
fn wait_for_depth(h: &mut Harness) {
    for _ in 0..200 {
        if !h.estimating_depth() {
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
    h.open_document(cut_craft_core::testing::domed());
    let params = h.opening_settings("insert_level");
    h.run("insert_level", &params).unwrap();
    h.choose_on_level(&["Mirror Mode", "Left To Right"]);
    assert_eq!(h.doc().levels[&0].mirror, Mirror::LeftToRight);
    assert!(!leaf(&h.level_menu(), &["Wrapping"]).is_enabled, "flat job");
    h.select_components([EntityId(1)]);
    h.choose_on_component(&["Move to Level", "Level 2"]);
    assert_eq!(h.doc().components[&EntityId(1)].level, 1);
    assert!(leaf(&h.component_menu(), &["Move to Level", "Level 2"]).is_checked);
}

#[test]
fn the_level_menu_selects_its_components_and_exports_them() {
    let mut h = Harness::new();
    h.open_document(cut_craft_core::testing::two_components());
    h.choose_on_level(&["Select Components"]);
    let all: Vec<EntityId> = h.doc().components.keys().copied().collect();
    assert_eq!(
        h.doc()
            .selected_components
            .iter()
            .copied()
            .collect::<Vec<_>>(),
        all
    );
    let menu = h.level_menu();
    let export = leaf(&menu, &["Export as 3D Clipart"]);
    assert!(export.is_enabled && h.available(&export.action));
}

#[test]
fn the_component_menu_acts_on_the_component_clicked() {
    use cut_craft_core::component::Combine;
    let mut h = Harness::new();
    h.rectangle(10.0, 10.0, 50.0, 50.0);
    let params = h.opening_settings("create_shape");
    h.run("create_shape", &params).unwrap();
    let component = |h: &Harness| h.doc().components.values().next().unwrap().clone();
    assert!(leaf(&h.component_menu(), &["Combine Mode", "Add"]).is_checked);
    assert_eq!(
        leaf(&h.component_menu(), &["Combine Mode", "Add"])
            .icon
            .as_deref(),
        Some("modeling/combine_add.png"),
        "the combine icons"
    );
    h.choose_on_component(&["Combine Mode", "Subtract"]);
    assert_eq!(component(&h).combine, Combine::Subtract);
    assert!(leaf(&h.component_menu(), &["Combine Mode", "Subtract"]).is_checked);
    h.choose_on_component(&["Hide", "This"]);
    assert!(!component(&h).visible);
    h.choose_on_component(&["Properties"]);
    let (id, settings) = h.take_form().unwrap();
    assert_eq!(
        (id, settings["combine"].as_str()),
        ("component_properties", Some("subtract"))
    );
    h.choose_on_component(&["Delete"]);
    assert!(h.doc().components.is_empty());
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
        assert!(h.available(id), "{id}");
    }
    let groups = &h.toolbar().groups;
    let file = groups.iter().find(|g| g.name == "File Operations").unwrap();
    assert!(file.tools.contains(&"export_vectors") && file.tools.contains(&"open_folder"));
    h.invoke("component_levels");
    assert_eq!(h.design_tab(), "Components");
    // with no library folder there is nothing to open, and it says so
    h.invoke("open_clipart_folder");
    assert!(
        h.status_message().contains("Library first"),
        "{}",
        h.status_message()
    );
}

#[test]
fn a_collapsed_level_hides_its_components() {
    let mut h = Harness::new();
    h.open_document(cut_craft_core::testing::domed());
    h.pin(Page::Design);
    h.show_design_tab("Components");
    h.frame(vec![]);
    let k = h.doc().components.values().next().unwrap();
    let name = format!("{} ({:?})", k.name, k.combine);
    assert!(h.text_at(&name).is_some(), "{name} listed");
    let level = *h.doc().levels.keys().next().unwrap();
    h.collapse_level(level);
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
    assert_eq!(h.doc().job.resolution, Resolution::Standard);
    assert_eq!(
        h.resolution_checked("model_resolution_standard"),
        Some(true)
    );
    assert!(h.available("model_resolution_highest"));
    let standard = h.doc().job.model_cell();
    h.invoke("model_resolution_highest");
    assert_eq!(h.doc().job.resolution, Resolution::Highest);
    assert!(
        h.doc().job.model_cell() < standard / 2.0,
        "finer cells for new models"
    );
    assert_eq!(h.resolution_checked("model_resolution_highest"), Some(true));
    assert_eq!(
        h.resolution_checked("model_resolution_standard"),
        Some(false)
    );
    assert!(
        h.status_message().contains("Highest"),
        "{}",
        h.status_message()
    );
    h.invoke("undo");
    assert_eq!(h.doc().job.resolution, Resolution::Standard, "undone");
    assert_eq!(h.resolution_checked("preview_quality_1"), None);
}

#[test]
fn dragging_a_model_in_the_2d_view_moves_it() {
    use cut_craft_core::component::{Combine, Component};
    let mut h = Harness::new();
    let o = h.doc().job.area().min;
    let rect = cut_craft_core::geometry::Rect::new(
        o + Point::new(100.0, 100.0),
        o + Point::new(160.0, 140.0),
    );
    let mut c = Change::builder(h.doc());
    c.add_component(Component::from_fn("model", Combine::Add, rect, 1.0, |_| {
        Some(2.0)
    }));
    h.make(c.finish(), "added");
    h.frame(vec![]);
    // issue #4: pressed on the model (not yet picked) and dragged, it is picked and moves
    let (from, to) = (h.at(130.0, 120.0), h.at(180.0, 140.0));
    h.drag(from, to);
    let k = h.doc().components.values().next().unwrap();
    let moved = k.extent().min - rect.min;
    assert!(
        (moved.x - 50.0).abs() < 1.0 && (moved.y - 20.0).abs() < 1.0,
        "moved by {moved:?}"
    );
    assert_eq!(h.doc().selected_components.len(), 1, "and picked");
    h.invoke("undo");
    let k = h.doc().components.values().next().unwrap();
    assert!(
        (k.extent().min - rect.min).length() < 1e-6,
        "one undo puts it back"
    );
    // a vector selected is dragged by pressing inside its box
    h.rectangle(10.0, 10.0, 40.0, 30.0);
    let before = h.doc().selection_bounds().unwrap();
    let (from, to) = (h.at(30.0, 25.0), h.at(40.0, 25.0));
    h.drag(from, to);
    let after = h.doc().selection_bounds().unwrap();
    assert!(
        (after.min.x - before.min.x - 10.0).abs() < 0.5,
        "{before:?} → {after:?}"
    );
    // a drag from empty space still selects by window
    h.select([]);
    h.select_components([]);
    let (from, to) = (h.at(5.0, 5.0), h.at(90.0, 80.0));
    h.drag(from, to);
    assert_eq!(
        h.doc().selection.len(),
        1,
        "the window picked the rectangle"
    );
}

#[test]
fn the_model_menu_runs_the_component_tools_it_names() {
    let h = Harness::new();
    let items = h.job_menus().items();
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
        assert!(h.available(id), "{label} runs its tool");
    }
}

#[test]
fn a_vector_behind_the_model_is_not_drawn_over_it() {
    // a tall block on the right half; an outline on the floor left of it
    use cut_craft_core::component::{Combine, Component};
    let mut h = Harness::new();
    // material thick enough for the wall, so it stands whole in it
    let width = h.doc().job.width;
    h.given_document(|d| d.job.thickness = width / 2.0 + 10.0);
    let job = h.doc().job;
    let o = job.area().min;
    let wall = cut_craft_core::geometry::Rect::new(
        o + Point::new(job.width / 2.0, 0.0),
        o + Point::new(job.width, job.height),
    );
    let mut c = Change::builder(h.doc());
    c.add_component(Component::from_fn(
        "wall",
        Combine::Add,
        wall,
        job.model_cell(),
        |_| Some(job.width / 2.0),
    ));
    h.make(c.finish(), "added");
    h.rectangle(
        job.width / 8.0,
        job.height / 4.0,
        job.width / 8.0,
        job.height / 2.0,
    );
    h.show_view(View::ThreeD);
    // lines drawn in the vectors' colour
    let lines = |h: &mut Harness, orbit, vectors| {
        h.given_orbit(orbit);
        h.given_vectors_in_3d(vectors);
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
    h.invoke("new");
    assert_eq!(h.screen(), Screen::NewJob);
    let file = pyramid_stl("new-job-import");
    h.import(&file);
    assert_eq!(h.screen(), Screen::Job, "the job is set up as it stood");
    assert_eq!(h.form_id(), Some("orientate_3d_model"));
    h.close_form();
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
    h.import(&file);
    h.edit_form(|p| {
        p["orientation"]["rotation"] = Value::from(90.0);
        p["size"]["x"] = Value::from(50.0);
    });
    h.touch_form();
    h.frame(vec![]);
    h.close_form();
    let (&id, k) = h.doc().components.iter().next().unwrap();
    let width = |k: &cut_craft_core::component::Component| k.cell * k.columns as f64;
    assert!((width(k) - 50.0).abs() <= k.cell, "{}", width(k));
    let source = k.source.clone().expect("the import remembers its file");
    assert_eq!(source.path, file);
    let again = format!("{}{}", gui::REIMPORT_MODEL, id.0);
    let menu = gui::component_menu(h.doc(), id, k);
    assert_eq!(menu[0].action, again, "first on the model's menu");
    assert!(h.available(&again));
    // the form again with the settings it had, its Apply replacing the model
    h.invoke(&again);
    assert_eq!(h.form_settings()["orientation"]["rotation"], 90.0);
    assert_eq!(
        h.form_settings()["size"]["x"],
        50.0,
        "at the size it was made"
    );
    assert_eq!(h.form_drawn(), Some(id));
    // most often to scale it again
    h.edit_form(|p| {
        p["orientation"]["rotation"] = Value::from(0.0);
        p["size"]["x"] = Value::from(25.0);
    });
    h.touch_form();
    h.frame(vec![]);
    assert_eq!(h.doc().components.len(), 1, "replaced, not added");
    let k = h.doc().components.values().next().unwrap();
    assert!((width(k) - 25.0).abs() <= k.cell, "{}", width(k));
    h.close_form();
    // changed, it is imported again as it is now, with a warning
    let mut changed = std::fs::read(&file).unwrap();
    changed.extend(b"\n");
    std::fs::write(&file, changed).unwrap();
    h.invoke(&again);
    assert!(h.form_id().is_some(), "from the changed file");
    assert!(
        h.status_message().contains("has changed"),
        "{}",
        h.status_message()
    );
    h.close_form();
    // gone, it is not offered
    std::fs::remove_file(&file).unwrap();
    assert!(!h.available(&again));
}

#[test]
fn importing_a_model_shows_the_3d_view() {
    let mut h = Harness::new();
    h.show_view(View::TwoD);
    let file = pyramid_stl("to-3d");
    h.import(&file);
    assert!(*h.view() == View::ThreeD, "placed in 3D, shown there");
    // and again from its file
    h.touch_form();
    h.frame(vec![]);
    h.close_form();
    h.show_view(View::TwoD);
    let id = *h.doc().components.keys().next().unwrap();
    h.invoke(&format!("{}{}", gui::REIMPORT_MODEL, id.0));
    assert!(*h.view() == View::ThreeD, "a re-import too");
}

#[test]
fn sculpting_works_in_the_3d_view_with_its_keys_and_wheel() {
    let mut h = Harness::new();
    h.open_document(cut_craft_core::testing::domed());
    h.show_view(View::ThreeD);
    h.frame(vec![]);
    let total = |h: &Harness| -> f64 {
        let k = h.doc().components.values().next().unwrap();
        k.heights.iter().filter(|z| !z.is_nan()).sum()
    };
    let before = total(&h);
    h.invoke("sculpt");
    h.frame(vec![]);
    // 3 picks Deposit; the wheel grows the brush, Shift and the wheel its strength
    let mid = h.view_3d_area().center();
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
    let params = h.form_settings();
    assert_eq!(params["tool"], "deposit");
    assert!(
        (params["diameter"].as_f64().unwrap() - 11.0).abs() < 1e-9,
        "{params}"
    );
    h.frame_holding(Modifiers::SHIFT, vec![wheel(Modifiers::SHIFT)]);
    let strength = h.form_value("strength").as_f64().unwrap();
    assert!((strength - 22.0).abs() < 1e-9, "{strength}");
    let orbit = h.orbit();
    // a left drag over the model sculpts it, and does not turn the view
    h.press(mid, true, Modifiers::NONE);
    for dx in [5.0, 10.0, 15.0, 20.0] {
        h.frame(vec![Event::PointerMoved(mid + egui::vec2(dx, 0.0))]);
    }
    h.press(mid + egui::vec2(20.0, 0.0), false, Modifiers::NONE);
    h.frame(vec![]);
    assert!(total(&h) > before, "the stroke deposited");
    assert_eq!(h.orbit(), orbit, "the view stayed put");
}

#[test]
fn e_on_a_shape_component_opens_create_shape_on_it() {
    let mut h = Harness::new();
    h.rectangle(10.0, 10.0, 40.0, 40.0);
    let mut params = h.default_settings("create_shape");
    params["profile"] = "angular".into();
    h.run("create_shape", &params).unwrap();
    let id = *h.doc().components.keys().next().unwrap();
    h.select([]);
    h.select_components([id]);
    assert!(h.available("edit_object"));
    h.frame(vec![key(Key::E)]);
    assert_eq!(h.form_id(), Some("create_shape"), "Create Shape opened");
    assert_eq!(h.form_value("profile"), "angular");
    let mut edited = h.form_settings().clone();
    edited["profile"] = "flat".into();
    edited["height"] = 3.0.into();
    h.apply_form("create_shape", &edited).unwrap();
    assert_eq!(h.doc().components.len(), 1, "remade, not added");
    let top = h.doc().components[&id]
        .height_at(Point::new(30.0, 30.0))
        .unwrap();
    assert!((top - 3.0).abs() < 1e-9, "{top}");
}

#[test]
fn export_as_stl_opens_its_form_and_keeps_its_settings() {
    let mut h = Harness::new();
    h.invoke("export_as_stl");
    h.frame(vec![]);
    h.frame(vec![]);
    assert!(h.stl_window_open());
    for text in [
        "Export Model as STL",
        "Triangulate",
        "Back face of triangulation",
    ] {
        assert!(h.text_at(text).is_some(), "{text}");
    }
    assert_eq!(h.options().stl, fmt_export_as_stl::Settings::default());
}

#[test]
fn orientate_3d_models_rings_turn_plus_the_way_their_plus_arrow_points_and_minus_back() {
    let mut h = Harness::new();
    let file = pyramid_stl("ring-ways");
    h.import(&file);
    h.show_view(View::ThreeD);
    h.frame(vec![]);
    h.frame(vec![]);
    let turn = |h: &Harness, k: &str| h.form_value("turn")[k].as_f64();
    // the + and − arrows as drawn now: the rings follow the model as it turns
    let marks_now = |h: &Harness, i: usize| {
        let (rings, sphere) = h.turn_rings();
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
    h.open_document(cut_craft_core::testing::domed());
    let id = EntityId(1);
    let action = format!("{}{}", gui::SAVE_BRUSH, id.0);
    let menu = gui::component_menu(h.doc(), id, &h.doc().components[&id]);
    assert!(
        menu.iter().any(|m| m.action == action),
        "on the component's menu"
    );
    assert!(h.available(&action));
    h.save_brush_to(id, &path).unwrap();
    let top = |h: &Harness| {
        h.doc().components[&id]
            .height_at(Point::new(20.0, 20.0))
            .unwrap()
    };
    let before = top(&h);
    h.invoke("sculpt");
    let mut params = h.form_settings().clone();
    params["tool"] = "deposit".into();
    params["strength"] = 100.0.into();
    params["brush"] = path.display().to_string().into();
    params["points"] = serde_json::json!([{ "x": 20.0, "y": 20.0 }, { "x": 20.0, "y": 20.0 }]);
    h.run("sculpt", &params).unwrap();
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
    let o = h.doc().job.area().min;
    let at = |h: &Harness, p: Point| h.at(p.x - o.x, p.y - o.y);
    let (corner, far) = (at(&h, rect.max), at(&h, rect.max + Point::new(30.0, 20.0)));
    let before = h.doc().components.values().next().unwrap().extent();
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
        h.doc().components.values().next().unwrap().extent(),
        before,
        "nothing changes until let go"
    );
    h.press(far, false, Modifiers::NONE);
    h.frame(vec![]);
    let drawn = textured_meshes(&h);
    assert_eq!(drawn.len(), 1, "drawn once, where it now is: {drawn:?}");
    let after = h.doc().components.values().next().unwrap().extent();
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
        let change = Change::transform_selected(h.doc(), |_| {
            cut_craft_core::glam::DAffine2::from_scale(cut_craft_core::glam::DVec2::splat(1.1))
        });
        h.make(change.unwrap(), "scaled");
        h.frame(vec![]);
    }
    assert_eq!(textures(&h), before, "the old pictures are let go");
}

#[test]
fn a_decorative_relief_follows_its_settings_as_one_component() {
    let mut h = Harness::new();
    h.rectangle(10.0, 10.0, 150.0, 150.0);
    let square: Vec<EntityId> = h.doc().vectors.keys().copied().collect();
    h.select(square.iter().copied());
    h.invoke("decorative_relief");
    h.frame(vec![]);
    let mut params = h.form_settings().clone();
    h.apply_form("decorative_relief", &params).unwrap();
    assert_eq!(h.doc().components.len(), 1, "one relief made");
    let first = h.doc().components.values().next().unwrap().heights.clone();
    let undo_depth = h.undo_steps();
    // a new variation carves another in its place, live
    params["seed"] = 7.into();
    h.edit_form(|p| *p = params.clone());
    h.touch_form();
    h.frame(vec![]);
    assert_eq!(h.doc().components.len(), 1, "in place of the first");
    let second = &h.doc().components.values().next().unwrap().heights;
    assert!(!cut_craft_core::component::same_heights(&first, second));
    assert_eq!(h.undo_steps(), undo_depth);
    h.invoke("undo");
    assert!(h.doc().components.is_empty());
}

#[test]
fn esc_on_import_3d_model_cancels_the_import() {
    let mut h = Harness::new();
    let file = pyramid_stl("esc-cancels-import");
    h.import(&file);
    h.frame(vec![]);
    assert_eq!(h.form_id(), Some("orientate_3d_model"));
    assert!(
        !h.doc().components.is_empty(),
        "placed while the form is open"
    );
    h.frame(vec![escape()]);
    assert!(h.form_id().is_none());
    assert!(
        h.doc().components.is_empty(),
        "Esc is the form's Cancel: the model is not imported"
    );
    assert!(
        h.status_message().contains("cancelled"),
        "{}",
        h.status_message()
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
    h.import(&file);
    h.frame(vec![]);
    h.frame(vec![]);
    let _ = std::fs::remove_dir_all(&dir);
    assert_eq!(h.form_id().unwrap(), "orientate_3d_model");
    assert!(h.form_error().is_some(), "it says what went wrong");
    assert!(h.doc().components.is_empty());
    let at = h.text_at("Import").expect("an Import button, greyed");
    h.click(at, Modifiers::NONE);
    h.frame(vec![]);
    assert!(
        !h.status_message().contains("Model imported"),
        "{}",
        h.status_message()
    );
    assert!(
        h.form_id().is_some(),
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
    let all: Vec<EntityId> = h.doc().vectors.keys().copied().collect();
    h.select(all);
    h.invoke("copy");
    h.given_copied_components(vec![component]);
    let before = h.doc().components.len();
    // another window's circle comes in through the system clipboard
    let circle = vec![Path::circle(Point::new(50.0, 50.0), 5.0)];
    h.frame(vec![Event::Paste(serde_json::to_string(&circle).unwrap())]);
    assert_eq!(h.doc().vectors.len(), 2, "the circle is pasted");
    assert_eq!(
        h.doc().components.len(),
        before,
        "no component from this window's old copy comes with it"
    );
}

#[test]
fn baking_component_properties_resets_the_form_so_apply_again_does_not_bake_twice() {
    let mut h = Harness::new();
    h.open_document(cut_craft_core::testing::domed());
    let top = |h: &Harness| h.doc().components[&EntityId(1)].height_range().unwrap().1;
    let high = top(&h);
    h.invoke("component_properties");
    h.frame(vec![]);
    h.edit_form(|p| {
        p["shape_height"] = serde_json::json!(150.0);
        p["bake"] = serde_json::json!(true);
    });
    let params = h.form_settings().clone();
    h.apply_form("component_properties", &params).unwrap();
    assert!((top(&h) - high * 1.5).abs() < 1e-6, "baked once");
    assert_eq!(
        h.form_settings()["bake"],
        serde_json::json!(false),
        "Bake unticked"
    );
    assert_eq!(h.form_settings()["shape_height"], serde_json::json!(100.0));
    let params = h.form_settings().clone();
    h.apply_form("component_properties", &params).unwrap();
    assert!(
        (top(&h) - high * 1.5).abs() < 1e-6,
        "not baked a second time"
    );
}

#[test]
fn the_status_bar_names_the_level_a_new_component_went_on() {
    let mut h = Harness::new();
    h.rectangle(10.0, 10.0, 50.0, 50.0);
    let shape = h.default_settings("create_shape");
    h.run("create_shape", &shape).unwrap();
    assert!(
        h.status_message().contains("on level Level 1"),
        "{}",
        h.status_message()
    );
}

#[test]
fn the_status_bar_names_no_level_when_a_tool_replaces_components() {
    let mut h = Harness::new();
    h.rectangle(10.0, 10.0, 50.0, 50.0);
    let shape = h.default_settings("create_shape");
    h.run("create_shape", &shape).unwrap();
    // Split replaces the component it cuts by two pieces: not a new component
    let piece = *h.doc().components.keys().next_back().unwrap();
    h.rectangle(0.0, 0.0, 30.0, 30.0);
    h.select_components([piece]);
    let none = h.default_settings("split");
    h.run("split", &none).unwrap();
    let said = h.status_message().to_owned();
    assert!(said.starts_with("Split Component applied"), "{said}");
    assert!(!said.contains("level"), "{said}");
}

#[test]
fn only_new_components_all_on_one_level_have_their_level_named() {
    use cut_craft_core::component::{Component, Level};
    let mut h = Harness::new();
    h.rectangle(10.0, 10.0, 50.0, 50.0);
    let shape = h.default_settings("create_shape");
    h.run("create_shape", &shape).unwrap();
    let doc = h.doc();
    let made = doc.components.values().next().unwrap().clone();
    let on = |level| Component {
        level,
        ..made.clone()
    };
    // one new component: its level
    let mut c = Change::builder(doc);
    c.add_component(on(0));
    assert_eq!(Harness::new_components_level(&c.finish()), Some(0));
    // new components shared over levels (Slice Model): none named
    let mut c = Change::builder(doc);
    c.add_component(on(0));
    c.add_component(on(1));
    assert_eq!(Harness::new_components_level(&c.finish()), None);
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
    assert_eq!(Harness::new_components_level(&c.finish()), None);
    // a component replaced by remove and add (Split, Emboss, Sculpt): none named
    let mut c = Change::builder(doc);
    c.remove_component(*doc.components.keys().next().unwrap());
    c.add_component(on(0));
    assert_eq!(Harness::new_components_level(&c.finish()), None);
}
