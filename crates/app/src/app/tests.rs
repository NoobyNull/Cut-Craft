//! The app driven headlessly, one frame at a time, with synthetic input.

use super::*;
use eframe::egui::{Event, PointerButton, Pos2, RawInput, Rect, vec2};

#[path = "accessibility_tests.rs"]
mod accessibility;
#[path = "web_api_tests.rs"]
mod web_api;

mod calculating;
mod canvas;
mod collisions;
mod drawing;
mod help_pages;
mod how_to;
mod interface;
mod jobs_and_files;
mod keys_and_mouse;
mod layers_sheets;
mod machines;
mod materials;
mod models_3d;
mod moving;
mod nodes_and_text;
mod previews;
mod rotary;
mod saving_toolpaths;
mod selecting;
mod snapping_and_guides;
mod split_views;
mod tool_database;
mod toolpath_list;
mod toolpaths;
mod unsigned_code;
mod updating;
mod verbs;
mod view_3d;

/// A headless frame's output: no renderer takes its texture changes, so they are let go.
fn headless(mut out: egui::FullOutput) -> egui::FullOutput {
    out.textures_delta.clear();
    out
}

/// A file dropped on the window, as the windowing layer reports one.
#[derive(Debug)]
struct Dropped(std::path::PathBuf);

impl egui::DroppedFile for Dropped {
    fn path(&self) -> &std::path::Path {
        &self.0
    }

    fn bytes(&self) -> Result<Vec<u8>, String> {
        std::fs::read(&self.0).map_err(|e| e.to_string())
    }
}

struct Harness {
    ctx: egui::Context,
    app: CutCraftApp,
    time: f64,
    /// What the last frame drew.
    shapes: Vec<egui::epaint::ClippedShape>,
    /// What the last frame put on the system clipboard.
    copied: String,
    /// The last frame's request for the window's picture, if it made one.
    screenshot: Option<egui::ScreenshotCallback>,
    /// The window's size.
    window: egui::Vec2,
}

impl Harness {
    /// The app as it starts: the start page, no job open.
    fn at_start() -> Self {
        // Windows and Linux unless a test asks for a Mac (`on_a_mac`): egui takes the system
        // the tests run on, and the Mac's menus, keys and title bar differ
        let ctx = egui::Context::default();
        ctx.set_os(egui::os::OperatingSystem::Nix);
        let mut h = Self {
            ctx,
            app: CutCraftApp::new(),
            time: 0.0,
            shapes: vec![],
            copied: String::new(),
            screenshot: None,
            window: vec2(1280.0, 800.0),
        };
        // the tests were written against the Classic window; a test of another layout
        // picks it
        h.app.options.general.layout = crate::options::WindowLayout::Classic;
        // previews appear cut at once unless a test plays one
        h.app.display.animate_preview = false;
        // slow jobs finish in place, as quick ones do, unless a test waits on one
        h.app.wait_before_showing = std::time::Duration::from_mins(10);
        // no browser opens from a test
        h.app.reporter.browser = false;
        // nor Material Setup on the first click on the Toolpaths page, unless a test asks
        h.app.ask_material = false;
        // nor a question about crossing vectors: the controls are clicked with whatever is
        // selected
        let checks = &mut h.app.options.confirmations;
        checks.toolpath_intersections = crate::options::IntersectionCheck::DontCheck;
        checks.radiused_offset_intersections = crate::options::IntersectionCheck::DontCheck;
        checks.sharp_offset_intersections = crate::options::IntersectionCheck::DontCheck;
        h.frame(vec![]);
        h.frame(vec![]);
        h
    }

    /// The start page, with Classic Startup Flow ticked: New goes through Job Setup.
    fn at_classic_start() -> Self {
        let mut h = Self::at_start();
        h.app.options.job_defaults.classic_startup = true;
        h
    }

    /// A new job made the way a user makes one: New, then OK in Job Setup.
    fn new() -> Self {
        let mut h = Self::at_start();
        h.app.invoke(&h.ctx, "new");
        h.app.job_setup_ok();
        h.frame(vec![]);
        h.frame(vec![]);
        h
    }

    fn frame(&mut self, events: Vec<Event>) {
        self.frame_holding(Modifiers::NONE, events);
    }

    /// One frame with `files` dropped on the window (or only dragged over it).
    fn drop_files(&mut self, files: &[std::path::PathBuf], dropped: bool) {
        self.time += 0.1;
        let input = RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, self.window)),
            time: Some(self.time),
            dropped_files: if dropped {
                files
                    .iter()
                    .map(|p| std::sync::Arc::new(Dropped(p.clone())) as egui::DroppedFileHandle)
                    .collect()
            } else {
                vec![]
            },
            hovered_files: if dropped {
                vec![]
            } else {
                files
                    .iter()
                    .map(|p| egui::HoveredFile {
                        path: Some(p.clone()),
                        ..Default::default()
                    })
                    .collect()
            },
            ..Default::default()
        };
        self.shapes = headless(self.ctx.run_ui(input, |ui| self.app.pass(ui))).shapes;
    }

    /// One frame with `modifiers` held, as the windowing layer reports them every frame.
    fn frame_holding(&mut self, modifiers: Modifiers, events: Vec<Event>) {
        self.time += 0.1;
        let mut events = events;
        events.insert(0, Event::ModifiersChanged(modifiers));
        let input = RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, self.window)),
            time: Some(self.time),
            events,
            ..Default::default()
        };
        let out = headless(self.ctx.run_ui(input, |ui| self.app.pass(ui)));
        self.shapes = out.shapes;
        self.copied = out
            .platform_output
            .commands
            .iter()
            .find_map(|c| match c {
                egui::OutputCommand::CopyText(text) => Some(text.clone()),
                _ => None,
            })
            .unwrap_or_default();
        self.screenshot = out
            .viewport_output
            .values()
            .flat_map(|v| &v.commands)
            .find_map(|c| match c {
                egui::ViewportCommand::Screenshot(callback) => Some(callback.clone()),
                _ => None,
            });
    }

    /// Every place the last frame drew `text`.
    fn texts_at(&self, text: &str) -> Vec<Pos2> {
        let drawn = self
            .shapes
            .iter()
            .filter_map(|clipped| match &clipped.shape {
                egui::Shape::Text(t) if t.galley.text() == text => {
                    Some(t.pos + t.galley.size() / 2.0)
                }
                _ => None,
            });
        drawn.collect()
    }

    /// The middle of `text` once the panel under `over` is wheeled down until it is in the
    /// window; `None` if it never is.
    fn scroll_to(&mut self, over: Pos2, text: &str) -> Option<Pos2> {
        for _ in 0..40 {
            if let Some(at) = self.text_at(text).filter(|at| at.y < 780.0) {
                return Some(at);
            }
            self.frame(vec![
                Event::PointerMoved(over),
                Event::MouseWheel {
                    unit: egui::MouseWheelUnit::Point,
                    delta: vec2(0.0, -100.0),
                    phase: egui::TouchPhase::Move,
                    source: egui::MouseWheelSource::Wheel,
                    modifiers: Modifiers::NONE,
                },
            ]);
            self.frame(vec![]);
        }
        None
    }

    /// The open form's heading `text`: the rightmost place it is drawn, as the Design
    /// page's tip box may name the tool too.
    fn form_heading(&self, text: &str) -> Pos2 {
        self.texts_at(text)
            .into_iter()
            .max_by(|a, b| a.x.total_cmp(&b.x))
            .unwrap_or_else(|| panic!("{text}"))
    }

    /// Presses Apply on the changes waiting in the Options window.
    fn apply_options(&mut self) {
        self.app
            .options_button(&self.ctx, super::options_apply::Button::Apply);
    }

    /// Every text the last frame drew.
    fn texts(&self) -> Vec<String> {
        (self.shapes.iter())
            .filter_map(|clipped| match &clipped.shape {
                egui::Shape::Text(t) => Some(t.galley.text().to_owned()),
                _ => None,
            })
            .collect()
    }

    /// The middle of `text` where the last frame drew it.
    /// The middle of each drawing of `text`'s words, without the space a picture before
    /// them on their line leaves at their start.
    fn words_at(&self, text: &str) -> Vec<Pos2> {
        self.shapes
            .iter()
            .filter_map(|clipped| match &clipped.shape {
                egui::Shape::Text(t) if t.galley.text() == text => {
                    let row = t.galley.rows.first()?;
                    Some(t.pos + row.rect_without_leading_space().center().to_vec2())
                }
                _ => None,
            })
            .collect()
    }

    fn text_at(&self, text: &str) -> Option<Pos2> {
        self.shapes.iter().find_map(|clipped| match &clipped.shape {
            egui::Shape::Text(t) if t.galley.text() == text => Some(t.pos + t.galley.size() / 2.0),
            _ => None,
        })
    }

    fn right_click(&mut self, at: Pos2) {
        self.frame(vec![Event::PointerMoved(at)]);
        for pressed in [true, false] {
            let button = Event::PointerButton {
                pos: at,
                button: PointerButton::Secondary,
                pressed,
                modifiers: Modifiers::NONE,
            };
            self.frame(vec![button]);
        }
        self.frame(vec![]);
    }

    /// Screen position of a job coordinate in the 2D view.
    fn at(&self, x: f64, y: f64) -> Pos2 {
        self.app
            .view_transform
            .expect("the 2D view has been drawn")
            .to_screen(Point::new(x, y))
    }

    fn press(&mut self, pos: Pos2, pressed: bool, modifiers: Modifiers) {
        let button = Event::PointerButton {
            pos,
            button: PointerButton::Primary,
            pressed,
            modifiers,
        };
        self.frame_holding(modifiers, vec![button]);
    }

    fn drag(&mut self, from: Pos2, to: Pos2) {
        self.frame(vec![Event::PointerMoved(from)]);
        self.press(from, true, Modifiers::NONE);
        self.frame(vec![Event::PointerMoved(from.lerp(to, 0.5))]);
        self.frame(vec![Event::PointerMoved(to)]);
        self.press(to, false, Modifiers::NONE);
        self.frame(vec![]);
    }

    /// A click with `modifiers` held for the whole gesture, as a held key would be.
    fn click(&mut self, at: Pos2, modifiers: Modifiers) {
        self.frame_holding(modifiers, vec![Event::PointerMoved(at)]);
        self.press(at, true, modifiers);
        self.press(at, false, modifiers);
        self.frame_holding(modifiers, vec![]);
    }

    fn double_click(&mut self, at: Pos2) {
        self.frame(vec![Event::PointerMoved(at)]);
        for pressed in [true, false, true, false] {
            self.press(at, pressed, Modifiers::NONE);
        }
        self.frame(vec![]);
    }

    /// Clicks the item at `path` (names, top down) of the right-click menu `menu` builds.
    fn choose(&mut self, menu: impl Fn(&CutCraftApp) -> Vec<gui::MenuItem>, path: &[&str]) {
        let items = menu(&self.app);
        let item = leaf(&items, path);
        assert!(
            item.is_enabled && self.app.available(&item.action),
            "{path:?} is greyed"
        );
        self.app.invoke(&self.ctx, &item.action);
    }

    /// Draws a rectangle through the command, then closes its form.
    fn rectangle(&mut self, x: f64, y: f64, width: f64, height: f64) {
        let params = serde_json::json!({ "x": x, "y": y, "width": width, "height": height, "corners": "square", "radius": 0.0 });
        self.app.apply("draw_rectangle", &params).unwrap();
        self.app.form = None;
        self.frame(vec![]);
    }
}

/// The item at `path` (names, top down) of a menu.
fn leaf<'a>(menu: &'a [gui::MenuItem], path: &[&str]) -> &'a gui::MenuItem {
    let (name, rest) = path.split_first().expect("a path to an item");
    let item = menu
        .iter()
        .find(|i| i.name == *name)
        .unwrap_or_else(|| panic!("no {name} in the menu"));
    if rest.is_empty() {
        item
    } else {
        leaf(&item.children, rest)
    }
}

fn near(a: Point, b: Point) -> bool {
    // one screen pixel is under a millimetre at this window size
    a.distance(b) < 2.0
}

/// Two rectangles grouped, nothing selected, the 2D view drawn.
fn two_grouped(h: &mut Harness) {
    h.rectangle(10.0, 10.0, 40.0, 40.0);
    h.rectangle(100.0, 10.0, 40.0, 40.0);
    h.app.invoke(&h.ctx, "select_all_vectors");
    h.app.invoke(&h.ctx, "group");
    h.app.doc.selection.clear();
    h.frame(vec![]);
}

/// A 20 × 10 mm, 5 mm high pyramid as an ASCII STL, in a file of its own.
fn pyramid_stl(name: &str) -> std::path::PathBuf {
    use std::fmt::Write as _;
    let base = [
        [0.0, 0.0, 0.0],
        [20.0, 0.0, 0.0],
        [20.0, 10.0, 0.0],
        [0.0, 10.0, 0.0],
    ];
    let top = [10.0, 5.0, 5.0];
    let mut faces: Vec<[[f64; 3]; 3]> = (0..4).map(|i| [base[i], base[(i + 1) % 4], top]).collect();
    faces.extend([[base[0], base[2], base[1]], [base[0], base[3], base[2]]]);
    let mut text = String::from("solid pyramid\n");
    for face in faces {
        text += "facet normal 0 0 0\n outer loop\n";
        for v in face {
            let _ = writeln!(text, "  vertex {} {} {}", v[0], v[1], v[2]);
        }
        text += " endloop\nendfacet\n";
    }
    text += "endsolid pyramid\n";
    let path = std::env::temp_dir().join(format!("cut-craft-{name}-{}.stl", std::process::id()));
    std::fs::write(&path, text).unwrap();
    path
}

/// Picks `name` in the open Select Tool window and clicks its Select.
fn select_in_tool_database(h: &mut Harness, name: &str) {
    h.frame(vec![]);
    h.frame(vec![]);
    let manager = h.app.tool_manager.as_mut().expect("Select Tool");
    assert!(manager.selecting, "the Tool Database opens to pick in");
    let at = manager
        .draft
        .tools
        .iter()
        .position(|t| t.name == name)
        .unwrap();
    manager.pick = Some(cut_craft_tool_database::Pick::Tool(at));
    h.frame(vec![]);
    h.frame(vec![]);
    let select = *h.texts_at("Select").last().expect("Select, in place of OK");
    h.click(select, Modifiers::NONE);
    h.frame(vec![]);
    assert!(h.app.tool_manager.is_none(), "closed");
}

/// The domed document in the 3D view, seen at a slant, with drawing tool `tool` open, and
/// where a point on the model's surface shows on screen.
fn drawing_on_the_dome(tool: &str) -> (Harness, impl Fn(f64, f64) -> Pos2, f64) {
    let mut h = Harness::new();
    h.app.doc = cut_craft_core::testing::domed();
    h.app.view = View::ThreeD;
    h.app.invoke(&h.ctx, tool);
    h.frame(vec![]);
    h.frame(vec![]);
    let fit = h.app.fit_3d.expect("the 3D view has been drawn");
    let surface = h.app.surface_3d().unwrap().into_owned();
    let camera = view3d::Camera::new(&surface, h.app.orbit, h.app.framing, fit, None);
    let cell = surface.cell;
    let on_model = move |x: f64, y: f64| {
        let z = surface.at(Point::new(x, y)).unwrap();
        camera.screen(cut_craft_core::glam::DVec3::new(
            x - surface.origin.x,
            y - surface.origin.y,
            z,
        ))
    };
    (h, on_model, cell)
}

/// The 3D view in Transform mode over the domed fixture, its component selected.
fn transform_3d(orbit: view3d::Orbit, perspective: bool) -> Harness {
    let mut h = Harness::new();
    h.app.doc = cut_craft_core::testing::domed();
    h.app.doc.selected_components = [EntityId(1)].into();
    h.app.orbit = orbit;
    h.app.framing.perspective = perspective;
    h.app.view = View::ThreeD;
    h.app.invoke(&h.ctx, "transform");
    h.frame(vec![]);
    h.frame(vec![]);
    h
}

/// Where the 3D view drew `which`.
fn handle_3d_at(h: &Harness, which: Handle3d) -> Pos2 {
    h.app
        .handles_3d
        .iter()
        .find(|(k, _)| *k == which)
        .map(|(_, at)| *at)
        .expect("the handle is drawn")
}

fn escape() -> Event {
    Event::Key {
        key: egui::Key::Escape,
        physical_key: None,
        pressed: true,
        repeat: false,
        modifiers: Modifiers::NONE,
    }
}

/// A harness on the domed test job, looking down on it in the 3D view, and where on the
/// screen the dome's middle is.
fn dome_from_above() -> (Harness, Pos2) {
    let mut h = Harness::new();
    h.app.doc = cut_craft_core::testing::domed();
    h.app.orbit = view3d::Orbit::TOP;
    h.app.view = View::ThreeD;
    h.frame(vec![]);
    h.frame(vec![]);
    let fit = h.app.fit_3d.unwrap();
    let surface = h.app.surface_3d().unwrap().into_owned();
    let screen = view3d::screen_of(&surface, h.app.orbit, h.app.framing, fit, None);
    let z = surface.at(Point::new(20.0, 20.0)).unwrap();
    let middle = screen(cut_craft_core::glam::DVec3::new(
        20.0 - surface.origin.x,
        20.0 - surface.origin.y,
        z,
    ));
    (h, middle)
}

/// The icons built into the program.
fn icon_set() -> std::collections::HashSet<String> {
    crate::icons::built_in::BUILT_IN
        .iter()
        .map(|(key, _)| (*key).to_owned())
        .collect()
}

/// The middle of the 2D view on screen.
fn view_middle(h: &Harness) -> Pos2 {
    h.app
        .view_area
        .expect("the 2D view has been drawn")
        .center()
}

fn key(key: Key) -> Event {
    Event::Key {
        key,
        physical_key: None,
        pressed: true,
        repeat: false,
        modifiers: Modifiers::NONE,
    }
}

/// A profile toolpath around a 40 mm square, previewed playing.
fn playing_preview() -> Harness {
    let mut h = Harness::new();
    h.rectangle(10.0, 10.0, 50.0, 50.0);
    h.app.invoke(&h.ctx, "profile_toolpath");
    let params = h.app.form.as_ref().unwrap().params.clone();
    h.app.apply("profile_toolpath", &params).unwrap();
    h.app.form = None;
    h.app.display.animate_preview = true;
    h.app.invoke(&h.ctx, "preview_all_toolpaths");
    assert!(h.app.playing && h.app.playback.is_some(), "it plays");
    h.frame(vec![]);
    h
}

/// `texts` show a number box holding `value` in `unit` (drawn as two pieces of text).
fn shows_number(texts: &[String], value: &str, unit: &str) -> bool {
    texts.windows(2).any(|w| w[0] == value && w[1] == unit)
}

/// Every page in `topic`'s side bar, whichever list holds it.
fn side_bar(h: &Harness, topic: &str) -> Vec<String> {
    let bar = h.app.help_sidebar(topic, &h.app.help_page(topic));
    [bar.like_this, bar.related, bar.links_here].concat()
}

/// Opens Edit ▸ Options on General Settings, with the recent file list at 9, and presses
/// Reset to defaults there: a change waiting for Apply.
fn options_with_a_change_not_applied() -> Harness {
    let mut h = Harness::new();
    h.app.options.general.recent_file_list_size = 9;
    h.app.invoke(&h.ctx, "options");
    h.frame(vec![]);
    h.frame(vec![]);
    let at = *h.texts_at("General Settings").last().expect("listed");
    h.click(at, Modifiers::NONE);
    h.frame(vec![]);
    assert!(h.text_at(super::options_apply::ALL_APPLIED).is_some());
    let reset = h.text_at("Reset to defaults").expect("the page's reset");
    h.click(reset, Modifiers::NONE);
    h.frame(vec![]);
    h
}

/// Clicks the button named `label`, then draws a frame.
fn click_button(h: &mut Harness, label: &str) {
    let at = h
        .text_at(label)
        .unwrap_or_else(|| panic!("{label}: {:?}", h.texts()));
    h.click(at, Modifiers::NONE);
    h.frame(vec![]);
}

const CRASH: &str = "Cut Craft 0.3.0 on linux crashed: panicked at crates/app/src/view3d.rs:9:9:\n\
                     index out of bounds\n\nBacktrace:\n...";

/// How wide each docked page was drawn over `frames` frames, Design then Toolpaths.
fn page_widths(h: &mut Harness, frames: usize) -> Vec<[Option<f32>; 2]> {
    (0..frames)
        .map(|_| {
            h.frame(vec![]);
            h.app.page_rects.map(|r| r.map(|r| r.width()))
        })
        .collect()
}

fn both_pinned() -> Harness {
    let mut h = Harness::new();
    h.app.pages.set_pinned(Page::Toolpaths, true);
    h.frame(vec![]);
    h
}

/// The middles of the corner arrows the last frame drew on flyout buttons.
fn flyout_arrows(h: &Harness) -> Vec<Pos2> {
    h.shapes
        .iter()
        .filter_map(|clipped| match &clipped.shape {
            egui::Shape::Path(p) if p.points.len() == 3 && p.closed => Some(
                p.points
                    .iter()
                    .fold(Pos2::ZERO, |sum, q| sum + q.to_vec2() / 3.0),
            ),
            _ => None,
        })
        .collect()
}

/// The corner arrow of the flyout button whose tip names `tool` (its face or one it holds).
fn flyout_arrow_naming(h: &mut Harness, tool: &str) -> Pos2 {
    for arrow in flyout_arrows(h) {
        let over = arrow - vec2(12.0, 12.0);
        h.frame(vec![Event::PointerMoved(over)]);
        h.frame(vec![]);
        if guide::hovered(&h.ctx).is_some_and(|t| t.about.contains(tool)) {
            return arrow;
        }
    }
    panic!("no flyout button names {tool}");
}

/// The harness on a Mac, as egui sees the system.
fn on_a_mac() -> Harness {
    let mut h = Harness::new();
    h.ctx.set_os(egui::os::OperatingSystem::Mac);
    h.frame(vec![]);
    h.frame(vec![]);
    h
}

/// Frames until the slow job running now is done and its answer used.
fn until_not_busy(h: &mut Harness) {
    for _ in 0..2000 {
        if h.app.busy.is_none() {
            return;
        }
        std::thread::sleep(std::time::Duration::from_millis(5));
        h.frame(vec![]);
    }
    // the last frame may have been the one that finished it
    if h.app.busy.is_none() {
        return;
    }
    panic!("the job never finished");
}

/// Two profile toolpaths, First with a 6 mm end mill and Second with a 3 mm one.
fn two_tools(h: &mut Harness) {
    for (x, name, diameter) in [(10.0, "First", 6.0), (80.0, "Second", 3.0)] {
        h.rectangle(x, 10.0, 50.0, 50.0);
        h.app.invoke(&h.ctx, "profile_toolpath");
        h.frame(vec![]);
        let mut params = h.app.form.as_ref().unwrap().params.clone();
        params["name"] = name.into();
        params["tool"]["diameter"] = diameter.into();
        params["tool"]["name"] = format!("{diameter} mm End Mill").into();
        h.app.apply("profile_toolpath", &params).unwrap();
        h.app.form = None;
    }
}

/// The screen boxes of the pictures the last frame drew in full (models; one being dragged
/// is also drawn faint where it was).
fn textured_meshes(h: &Harness) -> Vec<Rect> {
    h.shapes
        .iter()
        .filter_map(|c| match &c.shape {
            egui::Shape::Mesh(m)
                if m.texture_id != egui::TextureId::default()
                    && m.vertices.iter().all(|v| v.color == egui::Color32::WHITE) =>
            {
                let mut r = Rect::NOTHING;
                for v in &m.vertices {
                    r.extend_with(v.pos);
                }
                Some(r)
            }
            _ => None,
        })
        .collect()
}

/// A 60 × 40 mm model at (100, 100) on a new job, picked and drawn.
fn picked_model(h: &mut Harness) -> cut_craft_core::geometry::Rect {
    use cut_craft_core::component::{Combine, Component};
    let o = h.app.doc.job.area().min;
    let rect = cut_craft_core::geometry::Rect::new(
        o + Point::new(100.0, 100.0),
        o + Point::new(160.0, 140.0),
    );
    let mut c = Change::builder(&h.app.doc);
    let id = c.add_component(Component::from_fn("model", Combine::Add, rect, 1.0, |p| {
        Some(1.0 + (p.x - rect.min.x) / 10.0)
    }));
    h.app.record(Ok(c.finish()), "added");
    h.app.doc.selected_components = [id].into();
    h.frame(vec![]);
    rect
}

/// The status bar's text that contains `part`: its middle and how many rows it was laid in.
fn status_text(h: &Harness, part: &str) -> Option<(Pos2, usize)> {
    h.shapes.iter().find_map(|clipped| match &clipped.shape {
        egui::Shape::Text(t) if t.galley.text().contains(part) && t.pos.y > 700.0 => {
            Some((t.pos + t.galley.size() / 2.0, t.galley.rows.len()))
        }
        _ => None,
    })
}

/// A harness with a domed model in the 3D view, turned every frame for `frames` frames
/// (the harness's frames are a tenth of a second apart: ten a second, slower than wanted).
fn turned_slowly(adjust: bool, frames: usize) -> Harness {
    let mut h = Harness::new();
    h.app.options.performance.adjust_to_frame_rate = adjust;
    h.app.doc = cut_craft_core::testing::domed();
    h.app.view = View::ThreeD;
    for _ in 0..frames {
        h.app.moved_3d = h.time + 0.1;
        h.frame(vec![]);
    }
    h
}

/// A rectangle selected and the Profile Toolpath form open on it.
fn profile_form() -> Harness {
    let mut h = Harness::new();
    h.rectangle(10.0, 10.0, 50.0, 50.0);
    h.app.invoke(&h.ctx, "profile_toolpath");
    for _ in 0..3 {
        h.frame(vec![]);
    }
    h
}

/// A job with a pocket toolpath made from its one rectangle, the form closed.
fn pocketed() -> Harness {
    let mut h = Harness::new();
    h.rectangle(10.0, 10.0, 50.0, 50.0);
    let params = h
        .app
        .registry
        .get("pocket_toolpath")
        .unwrap()
        .initial_params(&h.app.doc);
    h.app.apply("pocket_toolpath", &params).unwrap();
    h.app.form = None;
    h.frame(vec![]);
    h
}

/// Moves the job's first vector, as dragging it would.
fn move_first_vector(h: &mut Harness) {
    let id = *h.app.doc.vectors.keys().next().unwrap();
    let moved = h.app.doc.vectors[&id].transformed(
        &cut_craft_core::glam::DAffine2::from_translation(Point::new(5.0, 0.0)),
    );
    let mut c = cut_craft_core::Change::builder(&h.app.doc);
    c.replace(id, moved);
    let change = c.finish();
    h.app.history.apply(&mut h.app.doc, change).unwrap();
}

/// A 6 x 6 square in an R12 DXF, which never says its units.
fn unitless_dxf(name: &str) -> std::path::PathBuf {
    use std::fmt::Write as _;
    let mut text = String::from(
        "  0\nSECTION\n  2\nHEADER\n  9\n$ACADVER\n  1\nAC1009\n  0\nENDSEC\n  0\nSECTION\n  2\n\
         ENTITIES\n  0\nPOLYLINE\n  8\n0\n 66\n     1\n 70\n     1\n",
    );
    for (x, y) in [(0, 0), (6, 0), (6, 6), (0, 6)] {
        let _ = write!(
            text,
            "  0\nVERTEX\n  8\n0\n 10\n{x}.0\n 20\n{y}.0\n 30\n0.0\n"
        );
    }
    text += "  0\nSEQEND\n  0\nENDSEC\n  0\nEOF\n";
    let path = std::env::temp_dir().join(format!("cut-craft-{name}-{}.dxf", std::process::id()));
    std::fs::write(&path, text).unwrap();
    path
}

/// The width of everything in the job.
fn drawn_width(h: &Harness) -> f64 {
    h.app
        .doc
        .vectors
        .values()
        .map(cut_craft_core::geometry::Path::bounds)
        .reduce(cut_craft_core::geometry::Rect::union)
        .map_or(0.0, |b| b.size().x)
}

fn job_corner(h: &Harness) -> cut_craft_core::geometry::Point {
    h.app.doc.job.area().min
}

/// Names toolpaths 1, 2 and 3 of `three_toolpaths` Pocket, Drill and Cut Out, on a pinned
/// Toolpaths page.
fn named_toolpaths() -> Harness {
    let mut h = Harness::new();
    h.app.doc = cut_craft_core::testing::three_toolpaths();
    let mut c = Change::builder(&h.app.doc);
    for (id, name) in [(1, "Pocket"), (2, "Drill"), (3, "Cut Out")] {
        let mut t = h.app.doc.toolpaths[&EntityId(id)].clone();
        t.name = name.into();
        c.replace_toolpath(EntityId(id), t);
    }
    h.app.doc.apply(&c.finish()).unwrap();
    h.app.screen = Screen::Job;
    h.app.pages.set_pinned(Page::Toolpaths, true);
    h.frame(vec![]);
    h.frame(vec![]);
    h
}

fn toolpath_names(h: &Harness) -> Vec<String> {
    h.app
        .doc
        .toolpaths
        .values()
        .map(|t| t.name.clone())
        .collect()
}

/// The Analyse tab with a profile toolpath to check, the Toolpaths page pinned open.
fn analysing() -> Harness {
    let mut h = Harness::new();
    h.app.pages.set_pinned(Page::Toolpaths, true);
    h.rectangle(10.0, 10.0, 50.0, 50.0);
    h.app.invoke(&h.ctx, "profile_toolpath");
    let params = h.app.form.as_ref().unwrap().params.clone();
    h.app.apply("profile_toolpath", &params).unwrap();
    h.app.form = None;
    h.app.toolpath_subtab = gui::TOOLPATH_SUBTABS
        .iter()
        .position(|t| t.name == "Analyse")
        .unwrap();
    h
}

/// A press or a let-go of `button` at `pos`.
fn mouse(button: PointerButton, pos: Pos2, pressed: bool) -> Event {
    Event::PointerButton {
        pos,
        button,
        pressed,
        modifiers: Modifiers::NONE,
    }
}

/// A sideways scroll of `x` points: the wheel tilted, or Shift and the wheel.
fn tilt(x: f32) -> Event {
    Event::MouseWheel {
        unit: egui::MouseWheelUnit::Point,
        delta: vec2(x, 0.0),
        phase: egui::TouchPhase::Move,
        source: egui::MouseWheelSource::Wheel,
        modifiers: Modifiers::NONE,
    }
}

fn key_with(key: Key, modifiers: Modifiers) -> Event {
    Event::Key {
        key,
        physical_key: None,
        pressed: true,
        repeat: false,
        modifiers,
    }
}

/// The Output tab with one Profile toolpath ready to save.
fn ready_for_output() -> Harness {
    let mut h = analysing();
    h.app.toolpath_subtab = gui::TOOLPATH_SUBTABS
        .iter()
        .position(|t| t.name == "Output")
        .unwrap();
    h.frame(vec![]);
    h.frame(vec![]);
    h
}

fn one_file_form() -> SaveForm {
    SaveForm {
        mode: crate::files::SaveMode::VisibleToOneFile,
        group: false,
        post: None,
        add_side: false,
        changes: crate::files::ToolChanges::SeparateFiles,
        job_sheet: false,
    }
}

/// An empty folder of the test's own under the temp folder, named for `name`, this run and a
/// count, so no two tests (or runs of one) share it.
fn fresh_dir(name: &str) -> std::path::PathBuf {
    static NEXT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    let n = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!("cut-craft-{name}-{}-{n}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("a temp folder");
    dir
}
