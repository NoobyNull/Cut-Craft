//! Each tool's tour for its help page's step pictures ([`super::help_pictures`]): the steps a
//! beginner takes, each set up in the app as they would leave it, with what to ring and the
//! words that go over its picture. A step builds on the one before.

use super::help_pictures::{Mark, Step, Tour};
use super::{CutCraftApp, View};
use crate::pages::Page;
use cut_craft_core::geometry::Point;
use eframe::egui;
use std::fmt::Write as _;

#[path = "help_tours/arranging.rs"]
mod arranging;
#[path = "help_tours/basics.rs"]
mod basics;
#[path = "help_tours/drawing.rs"]
mod drawing;
#[path = "help_tours/files.rs"]
mod files;
#[path = "help_tours/lists.rs"]
mod lists;
#[path = "help_tours/lists_more.rs"]
mod lists_more;
#[path = "help_tours/modelling.rs"]
mod modelling;
#[path = "help_tours/toolpaths.rs"]
mod toolpaths;

/// Every tool's tour, a group at a time, in the order they are taken.
const GROUPS: &[&[Tour]] = &[
    basics::TOURS,
    toolpaths::TOURS,
    files::TOURS,
    drawing::TOURS,
    arranging::TOURS,
    modelling::TOURS,
    lists::TOURS,
    lists_more::TOURS,
];

/// Every tool's tour.
pub fn every_tour() -> impl Iterator<Item = &'static Tour> {
    GROUPS.iter().flat_map(|group| group.iter())
}

const CIRCLE_X: f64 = 200.0;
const CIRCLE_Y: f64 = 150.0;
const CIRCLE_R: f64 = 90.0;

fn circle() -> serde_json::Value {
    serde_json::json!({ "center": { "x": CIRCLE_X, "y": CIRCLE_Y }, "radius": CIRCLE_R })
}

const RECT_X: f64 = 60.0;
const RECT_Y: f64 = 60.0;
const RECT_W: f64 = 260.0;
/// The rectangle beside the circle in [`two_shapes`].
const RECT2_X: f64 = CIRCLE_X + CIRCLE_R + 40.0;
const RECT_H: f64 = 160.0;
const STAR_X: f64 = 200.0;
const STAR_Y: f64 = 150.0;
const STAR_R: f64 = 100.0;

fn rectangle() -> serde_json::Value {
    serde_json::json!({ "x": RECT_X, "y": RECT_Y, "width": RECT_W, "height": RECT_H })
}

fn star() -> serde_json::Value {
    serde_json::json!({
        "center": { "x": STAR_X, "y": STAR_Y },
        "outer_radius": STAR_R,
        "inner_radius": STAR_R * 0.45,
        "points": 5,
        "rotation": 90.0
    })
}

/// A circle and, off to its right, a rectangle.
fn two_shapes(app: &mut CutCraftApp) {
    draw(app, "draw_circle", circle());
    let mut rect = rectangle();
    rect["x"] = RECT2_X.into();
    let rect = with(app, "draw_rectangle", &rect);
    draw(app, "draw_rectangle", rect);
    app.form = None;
    app.doc.selection.clear();
}

/// Tool `id`'s own settings with `overrides` laid over them.
fn with(app: &CutCraftApp, id: &str, overrides: &serde_json::Value) -> serde_json::Value {
    let mut params = app.registry.get(id).map_or(
        serde_json::Value::Null,
        cut_craft_core::DynCommand::default_params,
    );
    if let (Some(base), Some(over)) = (params.as_object_mut(), overrides.as_object()) {
        for (k, v) in over {
            base.insert(k.clone(), v.clone());
        }
    }
    params
}

/// Presses the open form's Calculate.
fn calculate(app: &mut CutCraftApp, _: &egui::Context) {
    if let Some(open) = &app.form {
        let (id, params) = (open.id, open.params.clone());
        app.calculate_checked(id, &params);
    }
}

/// Every toolpath previewed, cut into the material in the 3D view.
fn preview_3d(app: &mut CutCraftApp, ctx: &egui::Context) {
    app.form = None;
    app.doc.selection.clear();
    app.display.animate_preview = false;
    app.invoke(ctx, "preview_all_toolpaths");
    app.view = View::ThreeD;
    app.framing = crate::view3d::Framing::default().zoomed_about(egui::vec2(-70.0, -10.0), 2.2);
}

/// A new job, nothing drawn, the 2D view.
/// The active machine (one made if none is set up) fitted with what a toolpath `needs`: a
/// laser, or a 4th axis on firmware that drives one, so its tool shows on the Toolpaths page.
fn fitted(app: &mut CutCraftApp, needs: cut_craft_core::Needs) {
    if app.machines.active().is_none() {
        app.machines.add();
    }
    let Some(machine) = app.machines.active_mut() else {
        return;
    };
    match needs {
        cut_craft_core::Needs::Laser => machine.laser = true,
        cut_craft_core::Needs::Rotary => {
            machine.rotary = true;
            machine.firmware = Some(cut_craft_machines::machine::Firmware::GrblHal);
        }
        cut_craft_core::Needs::Nothing => {}
    }
}

fn fresh_job(app: &mut CutCraftApp, ctx: &egui::Context) {
    app.form = None;
    // the last tour's windows closed, so none is left in the next tour's pictures
    (app.tool_manager, app.info, app.part_labels_window) = (None, None, false);
    // the Design tab down the left (not Sheets, Layers or the Library)
    app.design_tab = 0;
    (app.save_form, app.gcode_view) = (None, None);
    // the last tour's job goes without asking whether to save it
    app.reset_job();
    app.invoke(ctx, "new");
    app.job_setup_ok();
    // each page's first tab, as a fresh start shows them
    (app.design_subtab, app.toolpath_subtab) = (0, 0);
    app.view = View::TwoD;
    app.camera = None;
}

/// Runs drawing tool `id` with `params`, as its form would.
fn draw(app: &mut CutCraftApp, id: &str, params: serde_json::Value) {
    let _ = app.apply(id, &params);
    if let Some(open) = app.form.as_mut().filter(|f| f.id == id) {
        open.params = params;
    }
}

/// Selects the shape through `(x, y)` of the job.
fn select(app: &mut CutCraftApp, x: f64, y: f64) {
    let at = app.doc.job.area().min + Point::new(x, y);
    app.doc.selection = app.doc.pick(at, 2.0).into_iter().collect();
}

/// Sets setting `key` of the open form to `value`.
fn set(app: &mut CutCraftApp, key: &str, value: serde_json::Value) {
    if let Some(open) = app.form.as_mut() {
        open.params[key] = value;
    }
}

/// Four small circles in a row, all selected: holes to drill or thread.
fn holes(app: &mut CutCraftApp) {
    for x in [100.0, 180.0, 260.0, 340.0] {
        let hole = serde_json::json!({ "center": { "x": x, "y": 150.0 }, "radius": 8.0 });
        draw(app, "draw_circle", hole);
    }
    app.form = None;
    app.doc.selection = app.doc.vectors.keys().copied().collect();
}

/// Three upright open lines, all selected: flutes.
fn lines(app: &mut CutCraftApp) {
    for x in [140.0, 200.0, 260.0] {
        let line = serde_json::json!({
            "points": [{ "x": x, "y": 50.0 }, { "x": x, "y": 250.0 }],
            "closed": false,
            "curves": []
        });
        draw(app, "draw_polyline", with(app, "draw_polyline", &line));
    }
    app.form = None;
    app.doc.selection = app.doc.vectors.keys().copied().collect();
}

/// A rectangle (the rail) and, to its right, an open quarter-round (the profile).
fn moulding_shapes(app: &mut CutCraftApp) {
    rect(app);
    // 40 mm across, 12 mm high: a quarter round
    let (x, y, across, high) = (RECT_X + RECT_W + 30.0, RECT_Y, 40.0, 12.0);
    let points: Vec<_> = (0..=8)
        .map(|i| {
            let a = f64::from(i) / 8.0 * std::f64::consts::FRAC_PI_2;
            serde_json::json!({ "x": x + across * (1.0 - a.cos()), "y": y + high * a.sin() })
        })
        .collect();
    let profile = serde_json::json!({ "points": points, "closed": false, "curves": [] });
    draw(app, "draw_polyline", with(app, "draw_polyline", &profile));
    app.form = None;
}

/// The [`rectangle`], drawn.
fn rect(app: &mut CutCraftApp) {
    let rect = with(app, "draw_rectangle", &rectangle());
    draw(app, "draw_rectangle", rect);
}

/// A circle made into a dome-shaped 3D model.
fn dome(app: &mut CutCraftApp, ctx: &egui::Context) {
    draw(app, "draw_circle", circle());
    app.form = None;
    select(app, CIRCLE_X - CIRCLE_R, CIRCLE_Y);
    app.invoke(ctx, "create_shape_round");
    // lower than the material is thick
    set(app, "total_height", 12.0.into());
    if let Some(open) = &app.form {
        let (id, params) = (open.id, open.params.clone());
        let _ = app.apply(id, &params);
    }
    app.form = None;
    app.doc.selection.clear();
    app.view = View::ThreeD;
}

/// A rotary job 60 mm across and 160 mm long along X, holding a turned bead: round, fatter
/// in the middle, all the way round the axis.
fn turned_bead(app: &mut CutCraftApp, ctx: &egui::Context) {
    use cut_craft_core::component::{Combine, Component};
    use cut_craft_core::document::{Along, Job};
    fresh_job(app, ctx);
    let job = Job {
        clearance: 5.0,
        ..Job::rotary(160.0, 60.0, Along::X)
    };
    let mut change = cut_craft_core::Change::builder(&app.doc);
    change.set_job(job);
    app.record(Ok(change.finish()), "Rotary job");
    let bead = Component::from_fn("Bead", Combine::Add, app.doc.job.area(), 0.5, |p| {
        let t = p.x / 160.0;
        Some(18.0 + 12.0 * (std::f64::consts::PI * t).sin().powi(2))
    });
    let mut change = cut_craft_core::Change::builder(&app.doc);
    change.add_new_component(bead);
    app.record(Ok(change.finish()), "Bead added");
    app.view = View::ThreeD;
}

/// A small circle near the job's corner with a Profile toolpath round it.
fn small_profile(app: &mut CutCraftApp, ctx: &egui::Context) {
    let part = serde_json::json!({ "center": { "x": 70.0, "y": 70.0 }, "radius": 40.0 });
    draw(app, "draw_circle", part);
    app.form = None;
    select(app, 30.0, 70.0);
    app.invoke(ctx, "profile_toolpath");
    calculate(app, ctx);
    app.form = None;
}

/// A made-up picture, selected: a sun over hills under a sky that darkens upwards.
fn picture(app: &mut CutCraftApp) {
    use cut_craft_core::bitmap::Bitmap;
    let (columns, rows) = (240_u32, 150_u32);
    let grey = (0..rows)
        .flat_map(|r| {
            (0..columns).map(move |c| {
                landscape(
                    f64::from(c) / f64::from(columns),
                    f64::from(r) / f64::from(rows),
                )
            })
        })
        .collect();
    let mut change = cut_craft_core::Change::builder(&app.doc);
    let rect =
        cut_craft_core::geometry::Rect::new(Point::new(40.0, 50.0), Point::new(360.0, 250.0));
    let id = change.add_bitmap(Bitmap {
        name: "Hills".into(),
        columns: columns as usize,
        rows: rows as usize,
        grey,
        rect,
        depth: Vec::new(),
        rgb: Vec::new(),
    });
    app.record(Ok(change.finish()), "Picture added");
    app.doc.selected_bitmaps = [id].into();
}

/// The made-up picture's grey at `(x, y)` across and down it (0 to 1): 0 black, 255 white.
#[expect(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
fn landscape(x: f64, y: f64) -> u8 {
    let far = 0.55 + 0.08 * (x * 9.0).sin() + 0.05 * (x * 23.0).cos();
    let near = 0.72 + 0.1 * (x * 5.0 + 1.0).sin();
    let sun = ((x - 0.7).powi(2) + (y - 0.3).powi(2)).sqrt() < 0.12;
    let tone = if y > near {
        0.15 + 0.1 * (x * 60.0).sin().abs()
    } else if y > far {
        0.4
    } else if sun {
        0.97
    } else {
        0.55 + 0.4 * y
    };
    (tone.clamp(0.0, 1.0) * 255.0) as u8
}

/// The Design page pinned open on sub-tab `tab` (0 Create, 1 Edit, 2 Arrange, 4 Import).
/// The Depth add-on, as Add-ons leaves it once installed, in an add-ons folder of the
/// pictures' own (so not the user's).
fn depth_installed(app: &mut CutCraftApp) {
    let dir = std::env::temp_dir().join("cut-craft-help-pictures-addons");
    let depth = dir.join("depth");
    let _ = std::fs::create_dir_all(&depth);
    let _ = std::fs::write(
        depth.join("addon.toml"),
        "id = \"depth\"\nname = \"Depth (Relief from Photo)\"\nkind = \"tool\"\nversion = \"0.1.0\"",
    );
    let _ = std::fs::write(
        depth.join("installed.json"),
        r#"{"repo":{"owner":"NoobyNull","name":"CutCraft-Depth"},"tag":"v0.1.0","version":"0.1.0","signed":false}"#,
    );
    app.addons = crate::addons::Addons::new(Some(dir));
    app.refresh_installed();
}

/// The Borders add-on, as if installed (a stand-in program, never run in a tour).
fn borders_installed(app: &mut CutCraftApp) {
    let dir = std::env::temp_dir().join("cut-craft-help-pictures-addons");
    let borders = dir.join(crate::borders::ADDON_ID);
    let _ = std::fs::create_dir_all(&borders);
    let _ = std::fs::write(
        borders.join("addon.toml"),
        "id = \"borders\"\nname = \"Borders (Generate Border)\"\nkind = \"tool\"\nversion = \"0.1.0\"",
    );
    let _ = std::fs::write(borders.join(crate::borders::program_name()), b"");
    let _ = std::fs::write(
        borders.join("installed.json"),
        r#"{"repo":{"owner":"Power-Whittle","name":"CutCraft-Borders"},"tag":"v0.1.0","version":"0.1.0","signed":false}"#,
    );
    // a key box of its own, empty: never the computer's password store
    let key = dir.join("google-key");
    let _ = std::fs::remove_file(&key);
    app.border.keys = Some(crate::borders::KeyStore::File(key));
    app.addons = crate::addons::Addons::new(Some(dir));
    app.refresh_installed();
}

/// A border as Generate saves it, in a Library of its own (not the user's): the hand-drawn
/// sample in `crates/app/samples/`.
fn sample_border(app: &mut CutCraftApp) {
    let root = std::env::temp_dir().join("cut-craft-help-pictures-library");
    let folder = crate::borders::folder(&root);
    let files = cut_craft_border_tile::files(&folder, "Classical Baroque");
    let _ = std::fs::create_dir_all(&folder);
    let _ = std::fs::write(&files.depth, include_bytes!("../samples/border-depth.png"));
    let _ = std::fs::write(&files.picture, include_bytes!("../samples/border.jpg"));
    let recipe = cut_craft_border_tile::Recipe::new("classical_baroque", "16:9", 160.0, 90.0, 6.0);
    if let Ok(json) = serde_json::to_string_pretty(&recipe) {
        let _ = std::fs::write(&files.recipe, json);
    }
    app.clipart = crate::clipart::Clipart::default();
    app.clipart.folders = vec![root];
    app.clipart.choose(Some(folder));
}

fn on_design_tab(app: &mut CutCraftApp, tab: usize) {
    app.pages.set_pinned(Page::Design, true);
    app.design_subtab = tab;
}

/// Nothing drawn: the tool needs nothing selected.
fn shapes_none(_: &mut CutCraftApp) {}

/// Everything drawn so far, selected in the order drawn.
fn select_all(app: &mut CutCraftApp) {
    app.form = None;
    app.doc.selection = app.doc.vectors.keys().copied().collect();
}

/// A circle, selected.
fn shapes_circle(app: &mut CutCraftApp) {
    draw(app, "draw_circle", circle());
    select_all(app);
}

/// A star, selected.
fn shapes_star(app: &mut CutCraftApp) {
    draw(app, "draw_star", star());
    select_all(app);
}

/// The first vector drawn, as a node-editing command names it.
fn first_vector(app: &CutCraftApp) -> serde_json::Value {
    serde_json::json!(app.doc.vectors.keys().next().map_or(0, |id| id.0))
}

/// A rectangle, selected.
fn shapes_rect(app: &mut CutCraftApp) {
    rect(app);
    select_all(app);
}

/// A circle and a smaller rectangle overlapping it, both selected (the rectangle last).
fn shapes_pair(app: &mut CutCraftApp) {
    draw(app, "draw_circle", circle());
    let over = serde_json::json!({ "x": 230.0, "y": 90.0, "width": 160.0, "height": 110.0 });
    let over = with(app, "draw_rectangle", &over);
    draw(app, "draw_rectangle", over);
    select_all(app);
}

/// Three circles of different sizes, unevenly spaced and out of line, all selected.
fn shapes_three(app: &mut CutCraftApp) {
    for (x, y, r) in [
        (80.0, 120.0, 30.0),
        (150.0, 190.0, 45.0),
        (330.0, 140.0, 35.0),
    ] {
        let c = serde_json::json!({ "center": { "x": x, "y": y }, "radius": r });
        draw(app, "draw_circle", c);
    }
    select_all(app);
}

/// Three open lines whose ends nearly meet, all selected.
fn shapes_lines(app: &mut CutCraftApp) {
    let spans = [
        [(80.0, 80.0), (200.0, 80.0)],
        [(206.0, 84.0), (206.0, 220.0)],
        [(200.0, 226.0), (80.0, 226.0)],
    ];
    for [(x0, y0), (x1, y1)] in spans {
        let line = serde_json::json!({
            "points": [{ "x": x0, "y": y0 }, { "x": x1, "y": y1 }],
            "closed": false,
            "curves": []
        });
        draw(app, "draw_polyline", with(app, "draw_polyline", &line));
    }
    select_all(app);
}

/// Words made with Draw Text, selected.
fn shapes_text(app: &mut CutCraftApp) {
    let words = serde_json::json!({
        "text": "CUT CRAFT", "height": 40.0, "anchor": { "x": 60.0, "y": 140.0 }
    });
    let words = with(app, "draw_text", &words);
    draw(app, "draw_text", words);
    select_all(app);
}

/// Words, then an arch to lay them on, both selected (the arch last).
fn shapes_text_curve(app: &mut CutCraftApp) {
    let words = serde_json::json!({
        "text": "CUT CRAFT", "height": 24.0, "anchor": { "x": 120.0, "y": 60.0 }
    });
    let words = with(app, "draw_text", &words);
    draw(app, "draw_text", words);
    let arch = serde_json::json!({
        "center": { "x": 200.0, "y": 90.0 }, "radius": 120.0,
        "start_angle": 20.0, "end_angle": 160.0
    });
    let arch = with(app, "draw_arc", &arch);
    draw(app, "draw_arc", arch);
    select_all(app);
}

/// Words and a plaque round them, both selected.
fn shapes_text_rect(app: &mut CutCraftApp) {
    shapes_rect(app);
    shapes_text(app);
}

/// The made-up picture, selected.
fn shapes_picture(app: &mut CutCraftApp) {
    picture(app);
}

/// The made-up picture and a circle over it, both selected.
fn shapes_picture_circle(app: &mut CutCraftApp) {
    picture(app);
    draw(app, "draw_circle", circle());
    select_all(app);
}

/// A small star, then a curve to copy it along, both selected (the star first).
fn shapes_copy_along(app: &mut CutCraftApp) {
    let small = serde_json::json!({
        "center": { "x": 60.0, "y": 150.0 }, "outer_radius": 20.0, "inner_radius": 9.0,
        "points": 5, "rotation": 90.0
    });
    let small = with(app, "draw_star", &small);
    draw(app, "draw_star", small);
    let path = serde_json::json!({
        "points": [{ "x": 100.0, "y": 80.0 }, { "x": 220.0, "y": 220.0 }, { "x": 360.0, "y": 90.0 }],
        "closed": false,
        "curves": []
    });
    draw(app, "draw_polyline", with(app, "draw_polyline", &path));
    select_all(app);
}

/// Two open lines whose ends nearly meet, both selected.
fn shapes_lines2(app: &mut CutCraftApp) {
    shapes_lines(app);
    let last = app.doc.vectors.keys().last().copied();
    app.doc.selection.retain(|id| Some(*id) != last);
}

/// Nothing drawn: the Model tool needs nothing selected.
fn model_none(_: &mut CutCraftApp, _: &egui::Context) {}

/// A circle, selected.
fn model_circle(app: &mut CutCraftApp, _: &egui::Context) {
    shapes_circle(app);
}

/// A rectangle (the rail) and an open quarter-round (the cross-section), selected in that order.
fn model_rails(app: &mut CutCraftApp, _: &egui::Context) {
    moulding_shapes(app);
    select_all(app);
}

/// An open profile line, selected: the half outline of a spindle.
fn model_profile(app: &mut CutCraftApp, _: &egui::Context) {
    let points: Vec<_> = (0..=12)
        .map(|i| {
            let t = f64::from(i) / 12.0;
            let x = 120.0 + 160.0 * t;
            let y = 150.0 + 25.0 + 30.0 * (t * std::f64::consts::PI * 2.0).sin().abs();
            serde_json::json!({ "x": x, "y": y })
        })
        .collect();
    let profile = serde_json::json!({ "points": points, "closed": false, "curves": [] });
    draw(app, "draw_polyline", with(app, "draw_polyline", &profile));
    select_all(app);
}

/// A dome-shaped 3D part, picked in the Components list.
fn model_dome(app: &mut CutCraftApp, ctx: &egui::Context) {
    dome(app, ctx);
    model_close_up(app);
    app.doc.selected_components = app.doc.components.keys().copied().collect();
}

/// The dome, picked, and a smaller circle over it, selected.
fn model_dome_circle(app: &mut CutCraftApp, ctx: &egui::Context) {
    model_dome(app, ctx);
    let inner =
        serde_json::json!({ "center": { "x": CIRCLE_X, "y": CIRCLE_Y }, "radius": CIRCLE_R * 0.5 });
    draw(app, "draw_circle", inner);
    app.form = None;
    let last = app.doc.vectors.keys().last().copied();
    app.doc.selection = last.into_iter().collect();
}

/// A rectangle, selected.
fn model_rect(app: &mut CutCraftApp, _: &egui::Context) {
    shapes_rect(app);
}

/// The made-up picture, selected.
fn model_picture(app: &mut CutCraftApp, _: &egui::Context) {
    picture(app);
}

/// Two dome-shaped 3D parts side by side, both picked in the Components list.
fn model_dome2(app: &mut CutCraftApp, ctx: &egui::Context) {
    dome(app, ctx);
    let small = serde_json::json!({ "center": { "x": 330.0, "y": 220.0 }, "radius": 40.0 });
    draw(app, "draw_circle", small);
    app.form = None;
    let last = app.doc.vectors.keys().last().copied();
    app.doc.selection = last.into_iter().collect();
    app.invoke(ctx, "create_shape_round");
    set(app, "total_height", 8.0.into());
    if let Some(open) = &app.form {
        let (id, params) = (open.id, open.params.clone());
        let _ = app.apply(id, &params);
    }
    app.form = None;
    app.doc.selection.clear();
    app.doc.selected_components = app.doc.components.keys().copied().collect();
    model_close_up(app);
}

/// The open tool's settings, changed by `settings`, applied; the result shown close up in the
/// 3D view.
fn model_result(app: &mut CutCraftApp, settings: &[(&str, f64)]) {
    for (key, value) in settings {
        set(app, key, (*value).into());
    }
    if let Some(open) = &app.form {
        let (id, params) = (open.id, open.params.clone());
        let _ = app.apply(id, &params);
    }
    app.form = None;
    app.doc.selection.clear();
    model_close_up(app);
}

/// The 3D view, closer in on the model.
fn model_close_up(app: &mut CutCraftApp) {
    app.view = View::ThreeD;
    app.framing = crate::view3d::Framing::default().zoomed_about(egui::vec2(-70.0, -10.0), 2.2);
}

/// Two gently curving rails, then an open half-round cross-section, selected in that order.
fn model_two_rails(app: &mut CutCraftApp, _: &egui::Context) {
    for y in [110.0, 190.0] {
        let points: Vec<_> = (0..=10)
            .map(|i| {
                let t = f64::from(i) / 10.0;
                serde_json::json!({ "x": 60.0 + 240.0 * t, "y": y + 25.0 * (t * std::f64::consts::PI).sin() })
            })
            .collect();
        let rail = serde_json::json!({ "points": points, "closed": false, "curves": [] });
        draw(app, "draw_polyline", with(app, "draw_polyline", &rail));
    }
    let points: Vec<_> = (0..=8)
        .map(|i| {
            let a = f64::from(i) / 8.0 * std::f64::consts::PI;
            serde_json::json!({ "x": 350.0 - 25.0 * a.cos(), "y": 150.0 + 20.0 * a.sin() })
        })
        .collect();
    let section = serde_json::json!({ "points": points, "closed": false, "curves": [] });
    draw(app, "draw_polyline", with(app, "draw_polyline", &section));
    select_all(app);
}

/// A small dome (the piece to tile), picked in the Components list, and a rectangle (the
/// area to fill), selected.
fn model_texture(app: &mut CutCraftApp, ctx: &egui::Context) {
    let piece = serde_json::json!({ "center": { "x": 40.0, "y": 40.0 }, "radius": 12.0 });
    draw(app, "draw_circle", piece);
    app.form = None;
    let last = app.doc.vectors.keys().last().copied();
    app.doc.selection = last.into_iter().collect();
    app.invoke(ctx, "create_shape_round");
    set(app, "total_height", 5.0.into());
    if let Some(open) = &app.form {
        let (id, params) = (open.id, open.params.clone());
        let _ = app.apply(id, &params);
    }
    app.form = None;
    app.doc.selected_components = app.doc.components.keys().copied().collect();
    rect(app);
    let last = app.doc.vectors.keys().last().copied();
    app.doc.selection = last.into_iter().collect();
}

/// The Design page's lists, by their place along its top.
const SHEETS: usize = 1;
const LAYERS: usize = 2;
const COMPONENTS: usize = 3;

/// The Design page showing list `tab` ([`SHEETS`], [`LAYERS`] or [`COMPONENTS`]), no form
/// over it.
fn on_list(app: &mut CutCraftApp, tab: usize) {
    app.form = None;
    app.pages.set_pinned(Page::Design, true);
    app.design_tab = tab;
}

/// Runs command `id` with `params`, as a right-click menu's item does, no form left open.
#[expect(
    clippy::needless_pass_by_value,
    reason = "the tours write the settings in place, as json!(...)"
)]
fn run(app: &mut CutCraftApp, id: &str, params: serde_json::Value) {
    let params = with(app, id, &params);
    let _ = app.apply(id, &params);
    app.form = None;
}

/// Presses the open form's Apply and closes it.
fn apply_form(app: &mut CutCraftApp) {
    if let Some(open) = &app.form {
        let (id, params) = (open.id, open.params.clone());
        let _ = app.apply(id, &params);
    }
    app.form = None;
}

/// Opens command `id`'s form filled in with `params` over its own settings, as a right-click
/// menu's item does.
fn menu_form(app: &mut CutCraftApp, ctx: &egui::Context, id: &str, params: &serde_json::Value) {
    let params = with(app, id, params);
    app.invoke(ctx, &format!("form:{id}:{params}"));
}

/// Where node editing's right-click at `(x, y)` of the job is, on the vector there.
fn node_params(app: &CutCraftApp, x: f64, y: f64) -> serde_json::Value {
    let at = app.doc.job.area().min + Point::new(x, y);
    let vector = app.doc.pick(at, 2.0).map_or(0, |id| id.0);
    serde_json::json!({ "vector": vector, "at": { "x": at.x, "y": at.y }, "reach": 5.0 })
}

/// Runs node editing's command `id` on the node or span at `(x, y)` of the job.
fn on_node(app: &mut CutCraftApp, id: &str, x: f64, y: f64) {
    let params = node_params(app, x, y);
    let _ = app.apply(id, &params);
}

/// The rectangle on Layer 1, then Layer 2 (made current) with a circle on it; nothing
/// selected.
fn two_layers(app: &mut CutCraftApp) {
    rect(app);
    app.form = None;
    run(app, "add_layer", serde_json::json!({ "name": "Layer 2" }));
    let round = serde_json::json!({ "center": { "x": 440.0, "y": 150.0 }, "radius": 60.0 });
    draw(app, "draw_circle", round);
    app.form = None;
    app.doc.selection.clear();
}

/// A new job with two toolpaths, Engrave round a circle and Cut out round a rectangle, and
/// the Toolpaths page showing.
fn two_toolpaths(app: &mut CutCraftApp, ctx: &egui::Context) {
    fresh_job(app, ctx);
    small_profile(app, ctx);
    let outline = serde_json::json!({ "x": 200.0, "y": 60.0, "width": 200.0, "height": 150.0 });
    let outline = with(app, "draw_rectangle", &outline);
    draw(app, "draw_rectangle", outline);
    app.form = None;
    select(app, 200.0, 100.0);
    app.invoke(ctx, "profile_toolpath");
    calculate(app, ctx);
    app.form = None;
    app.doc.selection.clear();
    name_toolpaths(app, &["Engrave", "Cut out"]);
    app.pages.set_pinned(Page::Toolpaths, true);
}

/// Every toolpath, in the list's order.
fn toolpath_ids(app: &CutCraftApp) -> Vec<cut_craft_core::document::EntityId> {
    app.doc.toolpaths.keys().copied().collect()
}

/// The toolpaths named `names`, in the list's order.
fn name_toolpaths(app: &mut CutCraftApp, names: &[&str]) {
    for (toolpath, name) in app.doc.toolpaths.values_mut().zip(names) {
        toolpath.name = (*name).into();
    }
}

/// Toolpath number `n` (from 0) clicked in the list, and only it.
fn pick_toolpath(app: &mut CutCraftApp, n: usize) {
    app.selected_toolpath = toolpath_ids(app).get(n).copied();
    app.more_toolpaths.clear();
}

/// The toolpaths' cut order turned round.
fn swap_toolpaths(app: &mut CutCraftApp) {
    let mut order = toolpath_ids(app);
    order.reverse();
    run(
        app,
        "arrange_toolpaths",
        serde_json::json!({ "order": order, "groups": {} }),
    );
}

/// The first component's group.
fn component_group(app: &CutCraftApp) -> u64 {
    app.doc
        .components
        .values()
        .find_map(|k| k.group.as_ref().map(|g| g.id))
        .unwrap_or(0)
}

/// A new double-sided job.
fn double_sided(app: &mut CutCraftApp, ctx: &egui::Context) {
    fresh_job(app, ctx);
    let job = cut_craft_core::document::Job {
        double_sided: Some(cut_craft_core::document::DoubleSided::default()),
        ..app.doc.job
    };
    let mut change = cut_craft_core::Change::builder(&app.doc);
    change.set_job(job);
    app.record(Ok(change.finish()), "Double-sided job");
}

/// A new rotary job 60 mm across and 160 mm long along X, unrolled in the 2D view.
fn rotary_job(app: &mut CutCraftApp, ctx: &egui::Context) {
    use cut_craft_core::document::{Along, Job};
    fresh_job(app, ctx);
    let job = Job {
        clearance: 5.0,
        ..Job::rotary(160.0, 60.0, Along::X)
    };
    let mut change = cut_craft_core::Change::builder(&app.doc);
    change.set_job(job);
    app.record(Ok(change.finish()), "Rotary job");
}

/// The open arc's middle and radius.
const ARC_X: f64 = 200.0;
const ARC_Y: f64 = 90.0;
const ARC_R: f64 = 120.0;

/// An open arc, bulging up, selected.
fn open_arc(app: &mut CutCraftApp) {
    let arch = serde_json::json!({
        "center": { "x": ARC_X, "y": ARC_Y }, "radius": ARC_R,
        "start_angle": 20.0, "end_angle": 160.0
    });
    let arch = with(app, "draw_arc", &arch);
    draw(app, "draw_arc", arch);
    select_all(app);
}

/// A line straight up through the circle's middle, selected alone.
fn centre_line(app: &mut CutCraftApp) {
    let line = serde_json::json!({
        "points": [{ "x": CIRCLE_X, "y": 40.0 }, { "x": CIRCLE_X, "y": 260.0 }],
        "closed": false,
        "curves": []
    });
    draw(app, "draw_polyline", with(app, "draw_polyline", &line));
    app.form = None;
    let last = app.doc.vectors.keys().last().copied();
    app.doc.selection = last.into_iter().collect();
}

/// Three circles inside the rectangle, two of one size and a smaller one.
fn holes_inside(app: &mut CutCraftApp) {
    for (x, y, r) in [
        (152.0, 140.0, 20.0),
        (240.0, 140.0, 20.0),
        (196.0, 185.0, 10.0),
    ] {
        let hole = serde_json::json!({ "center": { "x": x, "y": y }, "radius": r });
        draw(app, "draw_circle", hole);
    }
    app.form = None;
}

/// Three circles one above another, unevenly spaced, all selected.
fn stacked_three(app: &mut CutCraftApp) {
    for (x, y, r) in [
        (200.0, 60.0, 25.0),
        (215.0, 115.0, 20.0),
        (190.0, 240.0, 30.0),
    ] {
        let c = serde_json::json!({ "center": { "x": x, "y": y }, "radius": r });
        draw(app, "draw_circle", c);
    }
    select_all(app);
}

/// A small G-code file cutting a square, written for Import Toolpaths to read.
fn gcode_file() -> String {
    let dir = std::env::temp_dir().join("cut-craft-help");
    let path = dir.join("sign.nc");
    let gcode = "G21 G90\nT1 M6\nS18000 M3\nG0 Z5\nG0 X20 Y20\nG1 Z-3 F300\n\
                 G1 X120 F1200\nG1 Y120\nG1 X20\nG1 Y20\nG0 Z5\nM5\nM30\n";
    let _ = std::fs::create_dir_all(&dir).and_then(|()| std::fs::write(&path, gcode));
    path.display().to_string()
}

/// A small STL of two pyramids base to base (undercut all round), for Segment 3D Model.
fn stl_file() -> String {
    let dir = std::env::temp_dir().join("cut-craft-help");
    let path = dir.join("gem.stl");
    let corners = [(40.0, 0.0), (0.0, 40.0), (-40.0, 0.0), (0.0, -40.0)];
    let mut stl = String::from("solid gem\n");
    for i in 0..4 {
        let (a, b) = (corners[i], corners[(i + 1) % 4]);
        for tip in [30.0, -30.0] {
            let (p, q) = if tip > 0.0 { (a, b) } else { (b, a) };
            let _ = write!(
                stl,
                "facet normal 0 0 0\nouter loop\nvertex {} {} 0\nvertex {} {} 0\nvertex 0 0 {tip}\n\
                 endloop\nendfacet\n",
                p.0, p.1, q.0, q.1
            );
        }
    }
    stl.push_str("endsolid gem\n");
    let _ = std::fs::create_dir_all(&dir).and_then(|()| std::fs::write(&path, stl));
    path.display().to_string()
}
