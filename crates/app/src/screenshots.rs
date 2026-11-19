//! `cut-craft --screenshots DIR`: the README's pictures, taken by the app of its own window.
//! Each scene is set up as a person would leave it, given a moment to settle (the 3D view
//! draws finely once still), captured, and saved as `DIR/NN-name.png`; then the app quits.
//! The pictures show the app's own icons (`assets/icons`); nothing of the user's is read or
//! saved. Every scene is taken in the light theme, except a scene named `…-dark`, which is the
//! scene before it again in the dark theme; it is also saved beside that one as
//! `NN-…-light-and-dark.png`, the two side by side.

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
/// Each half's width in a light-and-dark picture, pixels.
const HALF_WIDTH: u32 = 1200;
/// The clear gap between the halves of a light-and-dark picture, pixels.
const HALF_GAP: u32 = 24;
/// The end of a scene's name that takes it again in the dark theme.
const DARK: &str = "-dark";

/// A scene: its file name and how it is set up (each builds on the one before).
type Scene = (&'static str, fn(&mut CutCraftApp, &egui::Context));

const SCENES: [Scene; 49] = [
    ("start-page", |_, _| {}),
    ("design-2d", design),
    ("design-2d-dark", |_, _| {}),
    ("model-3d", model),
    ("split-view", |app, _| app.split = Some(Split::SideBySide)),
    ("toolpath-preview", toolpaths),
    ("tool-database", tool_database),
    ("machine-configuration", machine),
    ("import-3d-model", import),
    ("rail-layout", rail),
    ("rail-layout-dark", |_, _| {}),
    ("ribbon-layout", ribbon),
    ("ribbon-layout-dark", |_, _| {}),
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
    ("height-start", height_start),
    ("height-buttons", |app, ctx| {
        // one click on the medallion: picked, outlined, its height buttons shown
        let medallion = app
            .doc
            .components
            .iter()
            .find(|(_, k)| k.name == "Medallion")
            .map(|(&id, _)| id);
        app.doc.selected_components = medallion.into_iter().collect();
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
    // Help: Notes (the four steps make notes-steps.gif)
    ("notes-step-1", |app, ctx| {
        notes(app, ctx, "", false, "");
    }),
    ("notes-step-2", |app, ctx| {
        notes(app, ctx, NOTES_TYPED, false, "");
    }),
    ("notes-step-3", |app, ctx| {
        notes(app, ctx, NOTES_FULL, false, "thickness");
        app.notes_ui.picked = Some("material.thickness".into());
    }),
    ("notes-step-4", |app, ctx| {
        notes(app, ctx, NOTES_FULL, true, "");
    }),
    ("notes-explorer", |app, ctx| {
        notes(app, ctx, NOTES_FULL, false, "pocket");
    }),
    ("notes-gcode", |app, ctx| {
        notes(app, ctx, NOTES_FULL, false, "");
        app.form = None;
        app.doc.notes = NOTES_FULL.into();
        app.doc.notes_in_gcode = true;
        app.view_gcode(app.default_save_form());
        ctx.set_embed_viewports(true);
    }),
    ("help-wiki", |app, _| {
        help_window(app, "pocket_toolpath", "");
    }),
    ("help-question", |app, _| {
        help_window(app, "contents", "how do I cut out a sign with bridges?");
    }),
    // Help: Checks before cutting (each builds on the one before)
    ("checks-running", checks_running),
    ("checks-found", checks_found),
    ("checks-ticked", |app, _| {
        let found = app.save_checks.iter().find_map(|c| c.found.clone());
        for f in found.unwrap_or_default() {
            app.acknowledged.insert(f.message);
        }
    }),
    ("tool-number-taken", tool_number_taken),
    // Help: Post-Processor Management (the filters, then Edit Capabilities)
    ("post-filters", |app, ctx| post_window(app, ctx, false)),
    ("post-filters-rotary", |app, ctx| {
        post_window(app, ctx, true);
        if let Some(w) = &mut app.post_window {
            let f = w.filter_mut();
            (f.arcs, f.straight) = (false, false);
            f.family = Some(cut_craft_machines::Family::Grbl);
        }
    }),
    ("post-capabilities", |app, _| {
        if let Some(w) = &mut app.post_window {
            *w.filter_mut() = cut_craft_machines::Filter::all();
            w.edit_capabilities(&app.posts, "Cut Craft GRBL Arcs (mm) (*.nc)");
        }
    }),
    // Help: The 4th axis and your firmware (standard GRBL: the 4th axis greyed out)
    ("machine-firmware", machine_firmware),
    ("machine-rotary", |app, _| {
        if let Some(m) = app
            .machine_window
            .as_mut()
            .and_then(|w| w.draft.active_mut())
        {
            m.firmware = Some(cut_craft_machines::Firmware::FluidNc);
        }
    }),
    // Help: Changing settings (Apply, Cancel and Close)
    ("options-not-applied", options_not_applied),
    ("options-apply-question", |app, _| {
        app.options_closing = true;
    }),
    ("snapping-not-applied", snapping_not_applied),
    // Help: Toolpaths ▸ Pressing Calculate again
    ("calculate-again", calculate_again),
    // Help: Reporting a bug (the details of the computer)
    ("bug-report-form", |app, ctx| {
        ctx.set_embed_viewports(true);
        (app.info, app.form, app.tool_manager) = (None, None, None);
        (app.snap_window, app.snap_draft) = (false, None);
        cut_craft_bug_report::open_here(ctx);
    }),
    // Help: Long jobs and the time left
    ("progress-window", progress_window),
];

/// Post-Processor Management on its own, opened for a machine in mm with a 4th axis
/// (`rotary`) or without.
fn post_window(app: &mut CutCraftApp, ctx: &egui::Context, rotary: bool) {
    ctx.set_embed_viewports(true);
    (app.info, app.form, app.tool_manager, app.machine_window) = (None, None, None, None);
    app.gcode_view = None;
    app.machines.add();
    if let Some(m) = app.machines.active_mut() {
        m.name = "Shop Router".into();
        m.inches = false;
        m.rotary = rotary;
        m.firmware = Some(cut_craft_machines::Firmware::FluidNc);
    }
    app.post_window = None;
    app.invoke(ctx, "post_processor_management");
}

/// The Help window over the job on page `topic`, with `search` typed in its search box.
fn help_window(app: &mut CutCraftApp, topic: &'static str, search: &str) {
    app.updates = crate::update::Updates::default();
    app.form = None;
    search.clone_into(&mut app.help_search);
    app.info = Some(super::Info::Help(topic));
}

/// Notes typed with the toolbar: a heading, a check list and a numbered list.
const NOTES_TYPED: &str = "# Oak sign for the porch\n\
    **Before cutting**\n\
    - [ ] Clamp all four corners\n\
    - [ ] Zero Z on the *top* of the board\n\
    1. Profile the sign out last\n\
    See [the finishing guide](https://example.com/finish) for oil.";

/// The same notes with live variables in.
const NOTES_FULL: &str = "# Oak sign for the porch\n\
    **Before cutting**\n\
    - [x] Clamp all four corners\n\
    - [ ] Zero Z on the *top* of the {{material.thickness}} board ({{material.z_zero}})\n\
    ## Toolpaths ({{toolpaths.run_time}} in all)\n\
    {{toolpaths.list}}\n\
    Machine: {{machine.name}}, printed {{date}}";

/// The sample shapes with a profile and a pocket, and the Notes form open with `text`:
/// written, or in Preview; the Variable Explorer searched for `search`.
fn notes(app: &mut CutCraftApp, ctx: &egui::Context, text: &str, preview: bool, search: &str) {
    app.updates = crate::update::Updates::default();
    ctx.set_embed_viewports(false);
    app.gcode_view = None;
    if app.doc.toolpaths.is_empty() {
        app.form = None;
        app.doc = cut_craft_core::document::Document::default();
        (app.simulation, app.playback) = (None, None);
        draw_shapes(app);
        toolpaths(app, ctx);
        app.view = View::TwoD;
    }
    if app.form.as_ref().is_none_or(|f| f.id != "notes") {
        app.invoke(ctx, "notes");
    }
    if let Some(form) = app.form.as_mut() {
        form.params["notes"] = serde_json::Value::String(text.into());
    }
    app.notes_ui.preview = preview;
    app.notes_ui.search = search.into();
    app.notes_ui.picked = None;
}

/// A long calculation's progress window: its step, how far it has got and the time left,
/// started from how long the last one took.
fn progress_window(app: &mut CutCraftApp, _: &egui::Context) {
    app.info = None;
    (app.snap_window, app.snap_draft) = (false, None);
    let kind = "screenshots:progress-window";
    cut_craft_core::eta::record(kind, 150.0);
    let (keep, wait) = std::sync::mpsc::channel::<()>();
    // never told to finish: the app quits once the picture is taken
    std::mem::forget(keep);
    app.in_background(
        "Calculating 3D Finishing",
        kind,
        move || {
            cut_craft_core::progress::stage("Following the model's surface");
            cut_craft_core::progress::note("Steep areas take a little longer than flat ones.");
            cut_craft_core::progress::report(0.2);
            wait.recv().ok();
        },
        |_, ()| {},
    );
}

/// The sample sign's toolpaths on a machine that says what it can do, Save Toolpaths open.
fn checks_job(app: &mut CutCraftApp, ctx: &egui::Context) {
    app.form = None;
    // the help scenes' window closed, the Toolpaths page not pinned over the windows
    app.info = None;
    app.help_search.clear();
    app.pages.set_pinned(super::Page::Toolpaths, false);
    app.doc = cut_craft_core::document::Document::default();
    draw_shapes(app);
    toolpaths(app, ctx);
    app.invoke(ctx, "reset_preview");
    app.view = View::TwoD;
    app.machines.add();
    if let Some(m) = app.machines.active_mut() {
        m.name = "Shop Router".into();
        (m.width, m.height, m.depth) = (1220.0, 610.0, 100.0);
        m.max_rate = [5000.0, 5000.0, 1500.0];
        m.spindle = [10000.0, 24000.0];
    }
    app.acknowledged.clear();
    app.save_form = Some(app.default_save_form());
    // its window drawn inside Cut Craft's, where the picture is taken
    ctx.set_embed_viewports(true);
}

/// Snapping Options (F4) with Smart snapping unticked and not applied yet.
fn snapping_not_applied(app: &mut CutCraftApp, _: &egui::Context) {
    (app.options_window, app.options_draft, app.options_closing) = (false, None, false);
    // Material Setup's page put away, so nothing covers the window
    app.form = None;
    app.pages.set_pinned(super::Page::Toolpaths, false);
    app.view = View::TwoD;
    app.snap_window = true;
    let mut edited = app.options.snapping.clone();
    edited.smart_snapping = !edited.smart_snapping;
    app.snap_draft = Some(edited);
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

/// The checks part-way: the settings checked, the machine's being checked.
fn checks_running(app: &mut CutCraftApp, ctx: &egui::Context) {
    checks_job(app, ctx);
    let Some(form) = app.save_form.clone() else {
        return;
    };
    let Some(plan) = app
        .form_post(&form)
        .ok()
        .and_then(|p| app.plan(&form, &p).ok())
    else {
        return;
    };
    app.save_checks = vec![super::SaveChecks {
        plan,
        changes: form.changes,
        revision: app.history.revision(),
        machine: app.machines.active,
        found: None,
    }];
    app.in_background(
        super::CHECKING,
        "check",
        || {
            cut_craft_core::progress::stage(&cut_craft_core::preflight::Phase::Machine.stage(""));
            cut_craft_core::progress::report(0.55);
            std::thread::sleep(std::time::Duration::from_secs(3));
        },
        |_, ()| {},
    );
}

/// The checks done: what they found, nothing ticked yet.
fn checks_found(app: &mut CutCraftApp, ctx: &egui::Context) {
    // the pretend run before is let go, so the real checks run
    app.save_checks.clear();
    checks_job(app, ctx);
}

/// The Tool Database: a bit given a tool number another bit has.
fn tool_number_taken(app: &mut CutCraftApp, ctx: &egui::Context) {
    app.save_form = None;
    // nothing pinned over the Tool Database's window
    app.pages.set_pinned(super::Page::Toolpaths, false);
    if let Some(first) = app.tools.tools.first_mut() {
        first.number = 2;
    }
    if let Some(second) = app.tools.tools.get_mut(1) {
        second.number = 2;
    }
    app.invoke(ctx, "tool_database");
    if let Some(manager) = &mut app.tool_manager {
        manager.pick = Some(cut_craft_tool_database::Pick::Tool(1));
    }
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

/// A Profile toolpath calculated, then Calculate pressed again with nothing changed: the
/// form says it is already up to date and offers Calculate as New.
fn calculate_again(app: &mut CutCraftApp, ctx: &egui::Context) {
    (app.options_window, app.options_closing, app.options_draft) = (false, false, None);
    (app.snap_window, app.snap_closing, app.snap_draft) = (false, false, None);
    app.form = None;
    app.gcode_view = None;
    app.updates = crate::update::Updates::default();
    (app.simulation, app.playback) = (None, None);
    app.doc = cut_craft_core::document::Document::default();
    app.view = View::TwoD;
    draw_shapes(app);
    let o = app.doc.job.area().min;
    app.doc.selection = app
        .doc
        .pick(o + Point::new(40.0, 110.0), 2.0)
        .into_iter()
        .collect();
    app.pages.set_pinned(super::Page::Toolpaths, true);
    app.invoke(ctx, "profile_toolpath");
    for _ in 0..2 {
        if let Some(params) = app.form.as_ref().map(|f| f.params.clone()) {
            let _ = app.apply_form("profile_toolpath", &params);
        }
    }
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
    /// The scene before's picture: the light half of a `…-dark` scene's pair.
    last: Option<std::sync::Arc<egui::ColorImage>>,
}

impl Shots {
    pub fn new(dir: PathBuf) -> Self {
        Self {
            dir,
            scene: 0,
            since: None,
            asked: false,
            shot: cut_craft_bug_report::Shot::default(),
            last: None,
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
            ctx.set_theme(if name.ends_with(DARK) {
                egui::ThemePreference::Dark
            } else {
                egui::ThemePreference::Light
            });
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
            let number = self.scene + 1;
            self.save(&format!("{number:02}-{name}.png"), &image);
            if let (Some(base), Some(light)) = (name.strip_suffix(DARK), &self.last) {
                let pair = side_by_side(light, &image, HALF_WIDTH);
                self.save(&format!("{number:02}-{base}-light-and-dark.png"), &pair);
            }
            self.last = Some(image);
            (self.scene, self.since, self.asked) = (self.scene + 1, None, false);
        }
    }

    /// Saves `image` as `file` in the folder, and says where (or why not).
    fn save(&self, file: &str, image: &egui::ColorImage) {
        let path = self.dir.join(file);
        let saved = cut_craft_bug_report::png(image)
            .map_err(std::io::Error::other)
            .and_then(|png| cut_craft_core::file::write_atomic(&path, &png));
        match saved {
            Ok(()) => println!("{}", path.display()),
            Err(e) => eprintln!("{}: {e}", path.display()),
        }
    }
}

/// `light` and `dark` side by side, each made `half` pixels wide (proportions kept), with a
/// clear gap of [`HALF_GAP`] between them.
fn side_by_side(light: &egui::ColorImage, dark: &egui::ColorImage, half: u32) -> egui::ColorImage {
    let [light, dark] = [light, dark].map(|picture| {
        let [w, h] = picture.size.map(|n| u32::try_from(n).unwrap_or(u32::MAX));
        let rgba = picture
            .pixels
            .iter()
            .flat_map(egui::Color32::to_array)
            .collect();
        let whole = image::RgbaImage::from_raw(w, h, rgba).unwrap_or_default();
        let height = (u64::from(half) * u64::from(h) / u64::from(w.max(1))).max(1);
        let height = u32::try_from(height).unwrap_or(u32::MAX);
        image::imageops::resize(&whole, half, height, image::imageops::FilterType::Lanczos3)
    });
    let height = light.height().max(dark.height());
    let mut both = image::RgbaImage::new(2 * half + HALF_GAP, height);
    image::imageops::overlay(&mut both, &light, 0, 0);
    image::imageops::overlay(&mut both, &dark, i64::from(half + HALF_GAP), 0);
    let size = [both.width(), both.height()].map(|n| n as usize);
    egui::ColorImage::from_rgba_premultiplied(size, both.as_raw())
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

/// Machine Configuration for a machine on standard GRBL with a rotary added: the 4th axis
/// greyed out, and the warning that says why.
fn machine_firmware(app: &mut CutCraftApp, ctx: &egui::Context) {
    ctx.set_embed_viewports(true);
    (app.info, app.form, app.tool_manager, app.machine_window) = (None, None, None, None);
    app.post_window = None;
    app.machines.add();
    if let Some(m) = app.machines.active_mut() {
        m.name = "Garage Router".into();
        m.manufacturer = "Home built".into();
        m.controller = "Arduino Uno + CNC shield".into();
        (m.width, m.height, m.depth) = (600.0, 400.0, 80.0);
        m.rotary = true;
    }
    app.invoke(ctx, "machine_configuration");
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

/// A flat job again with the medallion in the 3D view, nothing picked: the first step of
/// Help ▸ 3D View ▸ Changing a model's height.
fn height_start(app: &mut CutCraftApp, ctx: &egui::Context) {
    app.updates.open = false;
    ctx.set_embed_viewports(false);
    app.pages.set_pinned(super::Page::Toolpaths, false);
    app.pages.done_with(super::Page::Toolpaths);
    app.form = None;
    app.view_tool = None;
    app.doc = cut_craft_core::document::Document::default();
    draw_shapes(app);
    model(app, ctx);
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

#[cfg(test)]
mod tests {
    use super::*;

    fn plain(size: [usize; 2], colour: egui::Color32) -> egui::ColorImage {
        egui::ColorImage::new(size, vec![colour; size[0] * size[1]])
    }

    #[test]
    fn light_and_dark_sit_side_by_side_at_half_width_with_a_clear_gap() {
        let light = plain([40, 20], egui::Color32::WHITE);
        let dark = plain([40, 20], egui::Color32::BLACK);
        let both = side_by_side(&light, &dark, 20);
        assert_eq!(both.size, [2 * 20 + HALF_GAP as usize, 10]);
        let at = |x: usize, y: usize| both.pixels[y * both.size[0] + x];
        assert_eq!(at(5, 5), egui::Color32::WHITE);
        assert_eq!(
            at(20 + HALF_GAP as usize / 2, 5),
            egui::Color32::TRANSPARENT
        );
        assert_eq!(at(both.size[0] - 5, 5), egui::Color32::BLACK);
    }

    #[test]
    fn every_dark_scene_follows_its_light_one() {
        for (i, (name, _)) in SCENES.iter().enumerate() {
            if let Some(base) = name.strip_suffix(DARK) {
                assert!(
                    i > 0 && SCENES[i - 1].0 == base,
                    "{name} must follow {base}"
                );
            }
        }
    }
}
