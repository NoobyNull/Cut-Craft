//! `cut-craft --screenshots DIR`: the README's pictures, taken by the app of its own window.
//! Each scene is set up as a person would leave it, given a moment to settle (the 3D view
//! draws finely once still), captured, and saved as `DIR/NN-name.png`; then the app quits.
//! The pictures show the app's own icons (`assets/icons`); nothing of the user's is read or
//! saved.

use super::{CutCraftApp, Split, View};
use crate::options::WindowLayout;
use cut_craft_core::component::{Combine, Component};
use cut_craft_core::document::{GapFrom, ModelPosition};
use cut_craft_core::geometry::Point;
use eframe::egui;
use std::path::PathBuf;

/// How far the 3D scenes are zoomed in from fitting the whole job.
const ZOOM_3D: f32 = 1.4;
/// How long a scene is left before it is captured, seconds: past the 3D view's settling.
const HOLD_SECONDS: f64 = 1.0;
/// The window's size for the pictures, points.
pub const WINDOW: [f32; 2] = [1600.0, 1000.0];

/// A scene: its file name and how it is set up (each builds on the one before).
type Scene = (&'static str, fn(&mut CutCraftApp, &egui::Context));

const SCENES: [Scene; 27] = [
    ("start-page", |_, _| {}),
    ("design-2d", design),
    ("model-3d", model),
    ("split-view", |app, _| app.split = Some(Split::SideBySide)),
    ("toolpath-preview", toolpaths),
    ("tool-database", tool_database),
    ("machine-configuration", machine),
    ("import-3d-model", import),
    ("rail-layout", rail),
    ("ribbon-layout", ribbon),
    ("rotary-import", rotary_import),
    // Help: Toolpaths ▸ Looking at the G-code
    ("view-gcode", view_gcode),
    // Help: Updates (one-click install and relaunch)
    ("update-offer", |app, _| {
        update_window(app, update_offer(Ok(())));
    }),
    ("update-downloading", |app, _| {
        update_window(
            app,
            crate::update::Phase::Working(cut_craft_updater::Step::Downloading {
                done: 9_200_000,
                total: 21_400_000,
            }),
        );
    }),
    ("update-checking", |app, _| {
        update_window(
            app,
            crate::update::Phase::Working(cut_craft_updater::Step::Checking),
        );
    }),
    ("update-installing", |app, _| {
        update_window(
            app,
            crate::update::Phase::Working(cut_craft_updater::Step::Installing),
        );
    }),
    ("update-by-hand", |app, _| {
        let version = cut_craft_updater::Version::new(0, 6, 14);
        let steps = cut_craft_updater::Install::flatpak_steps(&version);
        update_window(app, update_offer(Err(steps)));
    }),
    // Help: 3D View ▸ Changing a model's height (each builds on the one before)
    ("height-picked", height_picked),
    ("height-buttons", |app, ctx| {
        app.view_action(ctx, "transform");
    }),
    ("height-taller", height_taller),
    // Help: Material Setup ▸ Model Position in Material
    ("model-top-at-top", |app, ctx| {
        model_position(app, ctx, GapFrom::Above, 0.0, true);
    }),
    ("model-bottom-on-bottom", |app, ctx| {
        model_position(app, ctx, GapFrom::Below, 0.0, true);
    }),
    ("model-sticks-out", |app, ctx| {
        model_position(app, ctx, GapFrom::Below, 15.0, false);
    }),
    ("help-wiki", |app, _| {
        help_window(app, "pocket_toolpath", "");
    }),
    ("help-question", |app, _| {
        help_window(app, "contents", "how do I cut out a sign with bridges?");
    }),
    // Help: Changing settings (Apply, Cancel and Close)
    ("options-not-applied", options_not_applied),
    ("options-apply-question", |app, _| {
        app.options_closing = true;
    }),
];

/// The Help window over the job on page `topic`, with `search` typed in its search box.
fn help_window(app: &mut CutCraftApp, topic: &'static str, search: &str) {
    app.updates = crate::update::Updates::default();
    app.form = None;
    search.clone_into(&mut app.help_search);
    app.info = Some(super::Info::Help(topic));
}

/// Edit ▸ Options on Appearance with the dark theme picked and not applied yet.
fn options_not_applied(app: &mut CutCraftApp, ctx: &egui::Context) {
    app.updates = crate::update::Updates::default();
    app.gcode_view = None;
    app.info = None;
    app.options_window = true;
    app.options_goto = Some("appearance");
    let mut edited = app.options.clone();
    edited.appearance.theme = crate::options::Theme::Dark;
    app.options_edited(edited);
    // its window drawn inside Cut Craft's, where the picture is taken
    ctx.set_embed_viewports(true);
}

/// The medallion in the 3D view with Material Setup open beside it, its Model Position in
/// Material set to `from` and `gap`; `apply`, pressed Apply (else only typed in).
fn model_position(
    app: &mut CutCraftApp,
    ctx: &egui::Context,
    from: GapFrom,
    gap: f64,
    apply: bool,
) {
    app.updates = crate::update::Updates::default();
    if app.form.as_ref().is_none_or(|f| f.id != "material_setup") {
        app.doc = cut_craft_core::document::Document::default();
        (app.simulation, app.playback) = (None, None);
        model(app, ctx);
        app.invoke(ctx, "material_setup");
    }
    let position = serde_json::to_value(ModelPosition { from, gap }).expect("serializes");
    if let Some(form) = app.form.as_mut() {
        form.params["position"] = position;
        if apply {
            let params = form.params.clone();
            let _ = app.apply("material_setup", &params);
        }
    }
    app.view = View::ThreeD;
}

/// The Updates window over the job, at `phase`.
fn update_window(app: &mut CutCraftApp, phase: crate::update::Phase) {
    app.gcode_view = None;
    app.updates.show(phase);
}

/// Version 0.6.14 offered, as GitHub would describe it; `ready` says whether it installs.
fn update_offer(ready: Result<(), String>) -> crate::update::Phase {
    let version = cut_craft_updater::Version::new(0, 6, 14);
    crate::update::Phase::Found(cut_craft_updater::Found::Release {
        release: cut_craft_updater::Release {
            tag: format!("v{version}"),
            version,
            notes: "- Install Now downloads, installs and reopens Cut Craft on your job\n\
                    - Pocket and Profile remember the last bit you picked"
                .into(),
            page: cut_craft_updater::RELEASES_PAGE.into(),
            assets: vec![],
        },
        ready,
    })
}

/// Where the run is.
pub struct Shots {
    dir: PathBuf,
    scene: usize,
    /// When the scene was set up (input time); `None` before.
    since: Option<f64>,
    /// Asked for the window's picture, waiting for it.
    asked: bool,
    /// Where it lands.
    shot: cut_craft_bug_report::Shot,
}

impl Shots {
    pub fn new(dir: PathBuf) -> Self {
        Self {
            dir,
            scene: 0,
            since: None,
            asked: false,
            shot: cut_craft_bug_report::Shot::default(),
        }
    }

    /// Once a frame, after drawing: sets up, waits, captures and saves each scene in turn.
    pub fn step(&mut self, app: &mut CutCraftApp, ctx: &egui::Context) {
        ctx.request_repaint();
        let now = ctx.input(|i| i.time);
        let Some(&(name, setup)) = SCENES.get(self.scene) else {
            std::process::exit(0);
        };
        let Some(since) = self.since else {
            setup(app, ctx);
            self.since = Some(now);
            return;
        };
        if !self.asked {
            if now - since >= HOLD_SECONDS {
                cut_craft_bug_report::take_screenshot(ctx, &self.shot);
                self.asked = true;
            }
            return;
        }
        if let Some(image) = cut_craft_bug_report::taken(&self.shot) {
            let path = self.dir.join(format!("{:02}-{name}.png", self.scene + 1));
            let saved = cut_craft_bug_report::png(&image)
                .map_err(std::io::Error::other)
                .and_then(|png| cut_craft_core::file::write_atomic(&path, &png));
            match saved {
                Ok(()) => println!("{}", path.display()),
                Err(e) => eprintln!("{}: {e}", path.display()),
            }
            (self.scene, self.since, self.asked) = (self.scene + 1, None, false);
        }
    }
}

/// A new job with shapes drawn on it, the Design tab open.
fn design(app: &mut CutCraftApp, ctx: &egui::Context) {
    app.invoke(ctx, "new");
    app.job_setup_ok();
    draw_shapes(app);
}

/// A rectangle, circle, star and hexagon drawn on the job, the Design tab open.
fn draw_shapes(app: &mut CutCraftApp) {
    let shapes = [
        (
            "draw_rectangle",
            serde_json::json!({ "x": 40.0, "y": 40.0, "width": 220.0, "height": 140.0, "corners": "rounded", "radius": 15.0 }),
        ),
        (
            "draw_circle",
            serde_json::json!({ "center": { "x": 400.0, "y": 110.0 }, "radius": 70.0 }),
        ),
        (
            "draw_star",
            serde_json::json!({ "center": { "x": 400.0, "y": 290.0 }, "outer_radius": 70.0, "inner_radius": 32.0, "points": 5, "rotation": 90.0 }),
        ),
        (
            "draw_polygon",
            serde_json::json!({ "center": { "x": 150.0, "y": 290.0 }, "radius": 70.0, "sides": 6, "rotation": 0.0 }),
        ),
    ];
    for (id, params) in shapes {
        let _ = app.apply(id, &params);
    }
    app.form = None;
    app.view = View::TwoD;
}

/// A relief beside a dome, in the 3D view.
fn model(app: &mut CutCraftApp, _: &egui::Context) {
    let job = app.doc.job;
    let o = job.area().min;
    let cell = job.model_cell();
    let (centre, size) = (o + Point::new(150.0, 110.0), 95.0);
    let area = cut_craft_core::geometry::Rect::new(
        centre - Point::splat(size),
        centre + Point::splat(size),
    );
    // a medallion: a raised rim round rings that fade out from the middle
    let medallion = move |p: Point| {
        let r = (p - centre).length() / size;
        (r <= 1.0).then(|| {
            let rim = if r > 0.88 { 6.0 } else { 0.0 };
            let rings = 2.5 * (r * 22.0).cos() * (1.0 - r) + 3.0 * (1.0 - r * r);
            2.0 + rim + rings.max(0.0)
        })
    };
    let mut c = cut_craft_core::Change::builder(&app.doc);
    c.add_component(Component::from_fn(
        "Medallion",
        Combine::Add,
        area,
        cell,
        medallion,
    ));
    app.record(Ok(c.finish()), "Made a medallion");
    app.doc.selection.clear();
    let circle = app
        .doc
        .vectors
        .iter()
        .find(|(_, p)| (p.bounds().center() - (o + Point::new(400.0, 110.0))).length() < 1.0)
        .map(|(&id, _)| id);
    if let Some(id) = circle {
        app.doc.selection = [id].into();
        let shape = app
            .registry
            .get("create_shape")
            .map(cut_craft_core::DynCommand::default_params);
        if let Some(params) = shape {
            let _ = app.apply("create_shape", &params);
        }
    }
    app.doc.selection.clear();
    // material thick enough for the model, so it sits in it whole
    let tall = cmd_scale_model_z_height::model_height(&app.doc);
    app.doc.job.thickness = app.doc.job.thickness.max((tall + 4.0).ceil());
    app.form = None;
    app.view = View::ThreeD;
    app.framing = crate::view3d::Framing::default().zoomed_about(egui::Vec2::ZERO, ZOOM_3D);
}

/// A profile and a pocket, previewed cut into the material.
fn toolpaths(app: &mut CutCraftApp, ctx: &egui::Context) {
    app.split = None;
    let o = app.doc.job.area().min;
    let pick = |app: &mut CutCraftApp, at: Point| {
        let id = app.doc.pick(o + at, 2.0);
        app.doc.selection = id.into_iter().collect();
    };
    pick(app, Point::new(40.0, 110.0));
    if let Some(params) = app
        .registry
        .get("profile_toolpath")
        .map(cut_craft_core::DynCommand::default_params)
    {
        let _ = app.apply("profile_toolpath", &params);
    }
    // the circle, by its left edge
    pick(app, Point::new(330.0, 110.0));
    if let Some(params) = app
        .registry
        .get("pocket_toolpath")
        .map(cut_craft_core::DynCommand::default_params)
    {
        let _ = app.apply("pocket_toolpath", &params);
    }
    app.doc.selection.clear();
    app.display.animate_preview = false;
    app.invoke(ctx, "preview_all_toolpaths");
    app.view = View::ThreeD;
    app.framing = crate::view3d::Framing::default().zoomed_about(egui::Vec2::ZERO, ZOOM_3D);
}

/// The Tool Database, a tool picked.
fn tool_database(app: &mut CutCraftApp, ctx: &egui::Context) {
    app.invoke(ctx, "reset_preview");
    app.view = View::TwoD;
    app.invoke(ctx, "tool_database");
    if let Some(manager) = &mut app.tool_manager {
        manager.pick = Some(cut_craft_tool_database::Pick::Tool(1));
    }
}

/// A machine with a five-slot tool changer, in Machine Configuration.
fn machine(app: &mut CutCraftApp, ctx: &egui::Context) {
    app.tool_manager = None;
    app.machines.add();
    if let Some(m) = app.machines.active_mut() {
        m.name = "Shop Router".into();
        m.manufacturer = "Home built".into();
        m.controller = "FluidNC".into();
        (m.width, m.height) = (1220.0, 610.0);
        m.set_slot_count(5);
        m.slots[0] = Some("6 mm End Mill".into());
        m.slots[1] = Some("3 mm End Mill".into());
        m.slots[2] = Some("90° V-Bit".into());
    }
    app.invoke(ctx, "machine_configuration");
}

/// The Rail layout: the Create Vectors list open beside the rail, Draw Circle's form in the
/// inspector.
fn rail(app: &mut CutCraftApp, ctx: &egui::Context) {
    app.form = None;
    app.view = View::TwoD;
    app.options.general.layout = WindowLayout::Rail;
    app.rail_group = app
        .all_groups()
        .iter()
        .position(|g| g.name == "Create Vectors");
    app.rail_pinned = true;
    app.invoke(ctx, "draw_circle");
}

/// The Ribbon layout on its Toolpaths tab: the toolpath list in the browser on the left,
/// Pocket Toolpath's form in the inspector on the right.
fn ribbon(app: &mut CutCraftApp, ctx: &egui::Context) {
    app.form = None;
    app.options.general.layout = WindowLayout::Ribbon;
    app.ribbon_tab = 2;
    app.inspector_tab = 4;
    app.invoke(ctx, "pocket_toolpath");
}

/// A full 3D model imported into a rotary job: shown turned round the axis in the stock.
fn rotary_import(app: &mut CutCraftApp, ctx: &egui::Context) {
    app.options.general.layout = crate::options::WindowLayout::Classic;
    app.form = None;
    app.doc.components.clear();
    app.doc.job =
        cut_craft_core::document::Job::rotary(200.0, 90.0, cut_craft_core::document::Along::X);
    import(app, ctx);
    app.framing = crate::view3d::Framing::default();
}

/// View G-code on the Output tab: the profile and pocket's G-code in its window, on a flat
/// job again (the job is replaced as is: a New would ask about the unsaved rotary one).
fn view_gcode(app: &mut CutCraftApp, ctx: &egui::Context) {
    app.form = None;
    app.doc = cut_craft_core::document::Document::default();
    draw_shapes(app);
    toolpaths(app, ctx);
    app.invoke(ctx, "reset_preview");
    app.view = View::TwoD;
    app.pages.set_pinned(super::Page::Toolpaths, true);
    app.toolpath_subtab = crate::gui::TOOLPATH_SUBTABS
        .iter()
        .position(|t| t.name == "Output")
        .unwrap_or_default();
    app.view_gcode(app.default_save_form());
    // its window drawn inside Cut Craft's, where the picture is taken
    ctx.set_embed_viewports(true);
}

/// Orientate 3D Model with a model in the scene: a stepped pyramid, made here.
fn import(app: &mut CutCraftApp, _: &egui::Context) {
    use std::fmt::Write as _;
    app.machine_window = None;
    let path = std::env::temp_dir().join(format!("cut-craft-shots-{}.stl", std::process::id()));
    let (w, d, h) = (120.0, 80.0, 30.0);
    let corners = [[0.0, 0.0, 0.0], [w, 0.0, 0.0], [w, d, 0.0], [0.0, d, 0.0]];
    let top = [w / 2.0, d / 2.0, h];
    let mut text = String::from("solid pyramid\n");
    let mut faces: Vec<[[f64; 3]; 3]> = (0..4)
        .map(|i| [corners[i], corners[(i + 1) % 4], top])
        .collect();
    faces.extend([
        [corners[0], corners[2], corners[1]],
        [corners[0], corners[3], corners[2]],
    ]);
    for face in faces {
        text += "facet normal 0 0 0\n outer loop\n";
        for v in face {
            let _ = writeln!(text, "  vertex {} {} {}", v[0], v[1], v[2]);
        }
        text += " endloop\nendfacet\n";
    }
    text += "endsolid pyramid\n";
    if std::fs::write(&path, text).is_ok() {
        app.import_file(&path);
    }
    app.view = View::ThreeD;
    app.framing = crate::view3d::Framing::default().zoomed_about(egui::Vec2::ZERO, ZOOM_3D);
}

/// A flat job again with the medallion picked in the 3D view, Transform mode off: the
/// first step of Help ▸ 3D View ▸ Changing a model's height.
fn height_picked(app: &mut CutCraftApp, ctx: &egui::Context) {
    app.updates.open = false;
    ctx.set_embed_viewports(false);
    app.pages.set_pinned(super::Page::Toolpaths, false);
    app.pages.done_with(super::Page::Toolpaths);
    app.form = None;
    app.view_tool = None;
    app.doc = cut_craft_core::document::Document::default();
    draw_shapes(app);
    model(app, ctx);
    let medallion = app
        .doc
        .components
        .iter()
        .find(|(_, k)| k.name == "Medallion")
        .map(|(&id, _)| id);
    app.doc.selected_components = medallion.into_iter().collect();
    // the medallion in the middle of the view, its height buttons in sight below it
    let medallion_at = egui::vec2(-275.0, 40.0);
    app.framing = crate::view3d::Framing::default()
        .zoomed_about(egui::Vec2::ZERO, ZOOM_3D)
        .zoomed_about(medallion_at, 1.5)
        .panned(egui::vec2(275.0, -90.0));
}

/// The picked medallion's Shape Height raised to 180 %, as a drag up on its Shape Height
/// button leaves it, Transform mode still on.
fn height_taller(app: &mut CutCraftApp, _: &egui::Context) {
    let Some(&id) = app.doc.selected_components.iter().next() else {
        return;
    };
    let mut params = cmd_component_properties::Params::of(id, &app.doc.components[&id]);
    params.shape_height = 180.0;
    if let Ok(params) = serde_json::to_value(&params) {
        let _ = app.apply("component_properties", &params);
    }
    app.status.set_message("Shape height 180%");
}
