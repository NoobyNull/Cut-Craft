//! `cut-craft --screenshots DIR`: the README's pictures, taken by the app of its own window.
//! Each scene is set up as a person would leave it, given a moment to settle (the 3D view
//! draws finely once still), captured, and saved as `DIR/NN-name.png`; then the app quits.
//! The pictures show the app's own icons (`assets/icons`); nothing of the user's is read or
//! saved.

use super::{CutCraftApp, Split, View};
use cut_craft_core::component::{Combine, Component};
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

const SCENES: [Scene; 8] = [
    ("start-page", |_, _| {}),
    ("design-2d", design),
    ("model-3d", model),
    ("split-view", |app, _| app.split = Some(Split::SideBySide)),
    ("toolpath-preview", toolpaths),
    ("tool-database", tool_database),
    ("machine-configuration", machine),
    ("import-3d-model", import),
];

/// Where the run is.
pub struct Shots {
    dir: PathBuf,
    scene: usize,
    /// When the scene was set up (input time); `None` before.
    since: Option<f64>,
    /// Asked for the window's picture, waiting for it.
    asked: bool,
}

impl Shots {
    pub fn new(dir: PathBuf) -> Self {
        Self {
            dir,
            scene: 0,
            since: None,
            asked: false,
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
                ctx.send_viewport_cmd(egui::ViewportCommand::Screenshot);
                self.asked = true;
            }
            return;
        }
        let shot = ctx.input(|i| {
            i.raw.events.iter().find_map(|e| match e {
                egui::Event::Screenshot { image, .. } => Some(image.clone()),
                _ => None,
            })
        });
        if let Some(image) = shot {
            let path = self.dir.join(format!("{:02}-{name}.png", self.scene + 1));
            let saved = std::fs::create_dir_all(&self.dir)
                .and_then(|()| std::fs::write(&path, cut_craft_bug_report::png(&image)));
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
