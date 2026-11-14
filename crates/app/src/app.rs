//! The main window: menus, Design and Toolpaths panels, and the 2D/3D views over the
//! document, its undo history and the command registry.

mod actions;
mod background;
mod cutting;
mod depth;
mod draw3d;
mod entry;
mod first_run;
mod fit_import;
mod guide;
mod help;
#[cfg(test)]
mod help_coverage;
mod layouts;
mod mac;
mod panels;
mod passes;
mod reach;
mod sample;
#[path = "screenshots.rs"]
mod screenshots;
mod send;
mod surfacing;
mod tabs;
#[cfg(test)]
mod tests;
mod tips;
mod title_bar;
mod trace_window;
mod units_ask;
mod updates;
mod views;

use crate::files::{self, Session};
use crate::form::{self, Field};
use crate::gui;
use crate::icons::Icons;
use crate::pages::{Page, Pages, Shown};
use crate::view2d::{self, Canvas, Gesture, Overlay, ViewTransform};
use crate::{commands, shortcut, view3d};
use cut_craft_bug_report::hint::HoverHint;
use cut_craft_core::document::{Document, EntityId, LayerId, Units};
use cut_craft_core::file::{JOB_EXTENSION, OLD_JOB_EXTENSION};
use cut_craft_core::geometry::{Path, Point, Rect};
use cut_craft_core::glam::DAffine2;
use cut_craft_core::handles::{self, Handle};
use cut_craft_core::simulation::Material;
use cut_craft_core::toolpath::{Tool, ToolDatabase};
use cut_craft_core::{Change, Drag, History, Registry};
use eframe::egui::{self, Align, Key, KeyboardShortcut, Layout, Modifiers};
use serde_json::Value;
use std::borrow::Cow;

/// The side tabs: Design holds the drawing and modeling tools.
const DESIGN_TABS: [&str; 5] = ["Design", "Sheets", "Layers", "Components", "Library"];
/// The browser's tabs in the Rail and Ribbon layouts: the design tabs' panels and the
/// toolpath list under one row of tabs.
const INSPECTOR_TABS: [&str; 5] = ["Sheets", "Layers", "Components", "Library", "Toolpaths"];
/// The ribbon's tabs: the drawing, modeling and toolpath groups.
const RIBBON_TABS: [&str; 3] = ["Design", "Model", "Toolpaths"];
/// Actions the app handles itself rather than through a command.
/// What the 3D view's fine mesh was made for: the model as at this revision, seen so.
#[derive(Clone, Debug, PartialEq)]
struct FineKey {
    revision: u64,
    /// The still detail's mesh cells.
    cells: usize,
    job: cut_craft_core::document::Job,
    orbit: view3d::Orbit,
    framing: view3d::Framing,
    fit: egui::Rect,
    look: view3d::Look,
    /// Whether the imported model is shown whole, in place of its component.
    importing: bool,
}

/// The mark a toolpath's cells take in the preview (`Material::cut_by`): its id.
fn preview_mark(id: EntityId) -> u32 {
    u32::try_from(id.0).unwrap_or(u32::MAX)
}

/// What the 3D view's surface on the GPU was built from: the model as at this revision,
/// wrapped and sided so (light is the shader's, so not these), at most this many cells (the
/// detail when still).
#[derive(Clone, Debug, PartialEq)]
struct GeometryKey {
    revision: u64,
    job: cut_craft_core::document::Job,
    wrap: Option<view3d::Wrap>,
    sides: bool,
    importing: bool,
    cells: usize,
}

/// One detail of the 3D view's surface on the GPU, and the model's walls and bottom made
/// with it (none over a preview).
#[derive(Clone)]
struct SurfaceLevel {
    cells: usize,
    surface: std::sync::Arc<view3d::gpu::Geometry>,
    solid: Option<std::sync::Arc<view3d::gpu::Geometry>>,
}

/// The 3D view's surface on the GPU at each detail it has been drawn at since the model
/// last changed, so the detail can follow the frame rate without remaking the surface at
/// every change: a coarser one is made from the fine one away from the window's thread.
struct SurfaceGpu {
    key: GeometryKey,
    levels: Vec<SurfaceLevel>,
    /// The surface at the detail when still, painted, that the coarser ones are made from.
    fine: Option<std::sync::Arc<Material>>,
    /// Coarser ones being made, by their cells.
    making: Vec<(usize, std::thread::JoinHandle<SurfaceLevel>)>,
}

impl SurfaceGpu {
    fn new(key: GeometryKey) -> Self {
        Self {
            key,
            levels: Vec::new(),
            fine: None,
            making: Vec::new(),
        }
    }

    /// Keeps the ones finished making.
    fn collect(&mut self) {
        let (done, making) = std::mem::take(&mut self.making)
            .into_iter()
            .partition::<Vec<_>, _>(|(_, job)| job.is_finished());
        self.making = making;
        for (_, job) in done {
            if let Ok(level) = job.join() {
                self.levels.push(level);
            }
        }
    }

    fn at(&self, cells: usize) -> Option<&SurfaceLevel> {
        self.levels.iter().find(|l| l.cells == cells)
    }

    /// The kept one nearest `cells`, the coarser on a tie: what is drawn while that one is
    /// made.
    fn nearest(&self, cells: usize) -> Option<&SurfaceLevel> {
        self.levels
            .iter()
            .min_by_key(|l| (l.cells.abs_diff(cells), l.cells))
    }

    /// Starts making the surface at `cells` from the fine one, unless it is being made;
    /// `ctx` is asked to draw again once it is ready.
    fn make(&mut self, cells: usize, solid: bool, ctx: &egui::Context) {
        let Some(fine) = self.fine.clone() else {
            return;
        };
        if self.making.iter().any(|(c, _)| *c == cells) {
            return;
        }
        let (wrap, sides, ctx) = (self.key.wrap, self.key.sides, ctx.clone());
        let job = std::thread::spawn(move || {
            let level = level_of(&view3d::coarse(&fine, cells), cells, wrap, sides, solid);
            ctx.request_repaint();
            level
        });
        self.making.push((cells, job));
    }
}

/// The GPU's surface (and, with `solid`, walls and bottom) of `surface`.
fn level_of(
    surface: &Material,
    cells: usize,
    wrap: Option<view3d::Wrap>,
    sides: bool,
    solid: bool,
) -> SurfaceLevel {
    SurfaceLevel {
        cells,
        surface: std::sync::Arc::new(view3d::gpu::surface(surface, wrap, sides)),
        solid: solid
            .then(|| std::sync::Arc::new(view3d::gpu::model(&view3d::solid(surface, wrap)))),
    }
}

/// What the 3D view's quick surface of the model was made from.
#[derive(Clone, Copy, Debug, PartialEq)]
struct RoughKey {
    revision: u64,
    job: cut_craft_core::document::Job,
    cells: usize,
    /// Whether the imported model is shown whole, in place of its component.
    importing: bool,
}

/// The imported model's faces on the GPU, and the model they were made from (kept, so no
/// other model takes its place in memory).
struct ModelGpu {
    model: std::sync::Arc<Vec<cmd_orientate_3d_model::Seen>>,
    geometry: std::sync::Arc<view3d::gpu::Geometry>,
}

/// What the imported model's drawn triangles depend on: the model (by its allocation),
/// the job, the view, and how it is coloured.
#[derive(Clone, PartialEq)]
struct ModelKey {
    model: usize,
    job: cut_craft_core::document::Job,
    orbit: view3d::Orbit,
    framing: view3d::Framing,
    fit: egui::Rect,
    look: view3d::Look,
    mark: egui::Color32,
}

/// What a select-mode drag holds: the whole selection, or one vector or model picked
/// under the pointer.
#[derive(Clone, Copy)]
enum Grab {
    Selection,
    Vector(EntityId),
    Component(EntityId),
}

/// Model ▸ Model Resolution's items, the resolution each sets and its name.
/// Edit ▸ Units' items: action, units and name.
const UNITS: [(&str, Units, &str); 2] = [
    ("units_mm", Units::Mm, "millimetres"),
    ("units_inch", Units::Inch, "inches"),
];
const RESOLUTIONS: [(&str, cut_craft_core::document::Resolution, &str); 3] = [
    (
        "model_resolution_standard",
        cut_craft_core::document::Resolution::Standard,
        "Standard",
    ),
    (
        "model_resolution_high",
        cut_craft_core::document::Resolution::High,
        "High",
    ),
    (
        "model_resolution_highest",
        cut_craft_core::document::Resolution::Highest,
        "Highest",
    ),
];

/// How much of the theme's darkest background shades the window while files are dragged
/// over it.
const DROP_SHADE: f32 = 0.8;
/// How far past the material the modeling plane is drawn while a model is imported, as a
/// share of the material's longer side, and how see-through it is.
const MODELING_PLANE_MARGIN: f64 = 0.1;
const MODELING_PLANE_ALPHA: f32 = 0.35;
/// How strongly Sketch Carving's green shows the areas it would carve over the picture.
const SKETCH_AREA_ALPHA: f32 = 0.45;
/// How strongly V-Carve's area to carve is shaded over the vectors.
const CARVE_AREA_ALPHA: f32 = 0.35;
/// Orientate 3D Model's rotation rings round the model, as a share of its longest side.
const RING_SHARE: f64 = 0.6;

/// Forms whose model is made as soon as they open and then follows their settings.
const LIVE_FORMS: &[&str] = &["orientate_3d_model", "relief_from_photo"];

/// Forms that make many pieces at once and go on remaking them as their settings change
/// (an ornament, a fill or a relief, and their variations), rather than adding another each time.
const REMAKE_FORMS: &[&str] = &["embellish", "vector_texture", "decorative_relief"];

const BUILT_INS: [&str; 105] = [
    "import_into_shape",
    "try_sample_project",
    "machine_files",
    "first_run_setup",
    "draw_rectangle_from_center",
    "draw_circle_from_edge",
    "tool_changer",
    "find_tool",
    "activate_layer_of_selection",
    "machine_configuration",
    "add_machine",
    "install_offline_package",
    "search_for_machine_online",
    "post_processor_management",
    "paste_in_place",
    "edit_text",
    "join_straight_line",
    "join_smooth_curve",
    "join_move_end_points",
    "import_vectors",
    "export_vectors",
    "open_folder",
    "open_clipart_folder",
    "component_levels",
    "clipart_library",
    "edit_toolpath",
    "create_shape_round",
    "create_shape_angled",
    "create_shape_smooth",
    "create_shape_flat",
    "left_to_right",
    "right_to_left",
    "top_to_bottom",
    "bottom_to_top",
    "delete_all_guides",
    "lock_all_existing_guides",
    "unlock_all_existing_guides",
    "options",
    "segment_3d_model_file",
    "import_toolpaths_file",
    "print_job_sheet",
    "import_cut_list",
    "snap_options",
    "node_edit",
    "transform",
    "measure_inspect",
    "print",
    "print_preview",
    "save_all_visible_toolpaths_as_template",
    "load_template",
    "save_selected_toolpath_as_template",
    "save_as_template",
    "new_from_template",
    "create_component_from_toolpath_preview",
    "toolpath_summary",
    "refresh_2d_view",
    "cut",
    "copy",
    "paste",
    "increment_and_save",
    "open_application_data_folder",
    "keyboard_shortcuts",
    "help_contents",
    "did_you_know",
    "third_party_licenses",
    "about_cut_craft",
    "check_for_updates",
    "addons",
    "undo",
    "redo",
    "exit",
    "new",
    "open",
    "save",
    "save_as",
    "import_vectors_bitmap",
    "import_component_3d_model",
    "recalculate_toolpath",
    "recalculate_visible_toolpaths",
    "recalculate_all_toolpaths",
    "delete_toolpath",
    "save_toolpaths",
    "install_post_processor",
    "close",
    "select",
    "select_all_vectors_on_current_layer",
    "job_size_and_position",
    "show_toolpaths_tab",
    "design_layout",
    "machining_layout",
    "interface_layout",
    "pan",
    "zoom_box",
    "zoom_to_job",
    "zoom_to_drawing",
    "zoom_selected",
    "tool_database",
    "preview_toolpath",
    "preview_all_toolpaths",
    "preview_visible_toolpaths",
    "undo_last_preview",
    "save_shaded_image",
    "edit_object",
    "job_sheet_editor",
    "reset_preview",
];
/// Where every Report a Bug Here… goes: Cut Craft's own repository.
const BUG_REPO: &str = "NoobyNull/Cut-Craft";
/// Cut Craft's crash-report service, where the crash logs of people who said yes go: a
/// Cloudflare Worker that groups them by cause for the maintainer and clears them out once
/// dealt with (`services/crash-reports`).
const CRASH_SERVICE: &str = "https://cutcraft-crash-reports.mattyax.workers.dev/v1/crash";
/// Points crash reports elsewhere, such as the service run locally for a test
/// (`http://localhost:8787/v1/crash`), never the live one.
const CRASH_SERVICE_VARIABLE: &str = "CUTCRAFT_CRASH_URL";
/// The Help windows.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Info {
    About,
    Shortcuts,
    Licenses,
    /// Toolpath Summary: estimated machining times.
    Summary,
    /// Help Contents (also a page's ?, and F1) on a page: `contents`, a tool page
    /// (`design`, `toolpaths`) or a tool's id.
    Help(&'static str),
}

/// The previewed material before a preview: `None` was uncut material.
#[derive(Clone, Debug)]
pub(super) struct BeforePreview(Option<Material>);

/// Save Toolpaths' choices (Save Toolpaths form), while its window is open.
#[derive(Clone, Debug)]
pub(super) struct SaveForm {
    /// Which toolpaths go in which file.
    mode: files::SaveMode,
    /// Visible to multiple files: toolpaths in a row using one tool share a file.
    group: bool,
    /// The post, by name in the library; `None` is the machine's default (Cut Craft's own
    /// without a machine).
    post: Option<String>,
    /// Double-sided jobs: Top or Bottom on the end of each file name.
    add_side: bool,
    /// How tools are changed: a file for each, the tool changer, or a pause.
    changes: files::ToolChanges,
    /// Save the job sheet (the project's sheet, listing these files) beside the files.
    job_sheet: bool,
}

/// What the checks before cutting found in Save Toolpaths' plan, kept while the plan, the
/// job and the machine stay as they were.
#[derive(Clone, Debug)]
pub(super) struct SaveChecks {
    plan: Vec<Vec<EntityId>>,
    changes: files::ToolChanges,
    revision: u64,
    machine: usize,
    found: Vec<cut_craft_core::preflight::Finding>,
}

/// Toolpath Summary's settings: the rapid rate (mm/min) and the units it is shown in, the
/// scale factor, and the toolpath picked to write notes for, with the notes being written.
#[derive(Clone, Debug)]
struct Summary {
    rapid: f64,
    units: cut_craft_core::toolpath::RateUnits,
    scale: f64,
    /// The Machine Acceleration: the times allow for the active machine's accelerations.
    acceleration: bool,
    picked: Option<EntityId>,
    notes: String,
}

impl Default for Summary {
    fn default() -> Self {
        Self {
            rapid: 5000.0,
            units: cut_craft_core::toolpath::RateUnits::MmPerMin,
            scale: 1.0,
            acceleration: false,
            picked: None,
            notes: String::new(),
        }
    }
}

/// Guide Properties: which guide, and what is typed: its position and angle, whether it is
/// locked, and a new guide's position or offset and how many.
#[derive(Clone, Debug)]
struct GuideProps {
    index: usize,
    position: f64,
    angle: f64,
    locked: bool,
    new_at: f64,
    relative: bool,
    count: u32,
}

/// A toolpath's row in the Toolpath Summary: its id, name, time, whether it is shown, and
/// its notes.
struct SummaryRow {
    id: EntityId,
    name: String,
    minutes: f64,
    visible: bool,
    notes: String,
}

/// The libraries Cut Craft is built on and the licences their makers declare.
// ponytail: the direct dependencies, kept by hand; generate from `cargo metadata` if it drifts
const THIRD_PARTY: [(&str, &str); 21] = [
    ("eframe / egui / egui_extras", "MIT or Apache-2.0"),
    ("log", "MIT or Apache-2.0"),
    ("egui_commonmark", "MIT or Apache-2.0"),
    ("pulldown-cmark", "MIT"),
    ("glam", "MIT or Apache-2.0"),
    ("i_overlay", "MIT or Apache-2.0"),
    ("qrcode (part labels)", "MIT or Apache-2.0"),
    ("md-5", "MIT or Apache-2.0"),
    ("image", "MIT or Apache-2.0"),
    ("usvg", "MPL-2.0"),
    ("dxf", "MIT"),
    ("serde / serde_json", "MIT or Apache-2.0"),
    ("schemars", "MIT"),
    ("rfd", "MIT"),
    ("chrono", "MIT or Apache-2.0"),
    ("env_logger", "MIT or Apache-2.0"),
    ("log", "MIT or Apache-2.0"),
    ("wgpu (through eframe)", "MIT or Apache-2.0"),
    ("Rust standard library", "MIT or Apache-2.0"),
    (
        "GitHub and Thingiverse link icons",
        "Icons8 (icons8.com), free with this credit",
    ),
    (
        "Pointing hand and statue pictures (Orientate 3D Model)",
        "Rendered from free models shared on printables.com",
    ),
];

/// The fewest 3D view mesh cells along the longer side of the job: Options' Low detail.
/// The detail while moving and when still are Options' (3D View); at Very High, 1200
/// cells, the CPU mesh takes about 45 ms (measured in release) and the GPU's 40 MB.
const MESH_CELLS: usize = 250;
/// Seconds between remakes of the model while a height handle drags it.
const HEIGHT_DRAG_EVERY: f64 = 0.1;
/// How wide the Options, Snapping Options and Help windows open, as a share of the main
/// window's width.
const DIALOG_WIDTH_SHARE: f32 = 0.4;

/// How big those windows open.
fn dialog_size(ctx: &egui::Context) -> egui::Vec2 {
    ctx.content_rect().size()
        * egui::vec2(
            DIALOG_WIDTH_SHARE,
            cut_craft_bug_report::DIALOG_HEIGHT_SHARE,
        )
}

/// The logo on the opening splash: the CC monogram with the name under it, on its own dark
/// ground, so it reads the same on the light and the dark theme.
const SPLASH_LOGO: &[u8] = include_bytes!("../../../assets/splash-logo.jpg");
/// How long the 3D view stays still before it is drawn finely, seconds, until Options says
/// otherwise.
pub(crate) const SETTLE_SECONDS: f64 = 0.25;
/// How often unsaved work is copied for recovery after a crash, seconds, until Options says
/// otherwise.
pub(crate) const AUTOSAVE_SECONDS: f64 = 30.0;
/// Drawing tools that take the whole path the pointer was dragged along, as `points`.
const STROKE_TOOLS: [&str; 3] = ["draw_freehand", "sculpt", "interactive_vector_trim"];
/// How far one notch of a mouse wheel scrolls, in points, as egui reports a line.
const SCROLL_NOTCH: f32 = 50.0;
/// How much a wheel notch grows the sculpting brush, or its strength; with Ctrl, finer.
const BRUSH_STEP: f64 = 0.1;
const BRUSH_FINE_STEP: f64 = 0.02;
/// Importers that make components; Import Component / 3D Model offers these, Import
/// Vectors / Bitmap the rest.
const MODEL_IMPORTERS: [&str; 2] = ["import_stl", "import_3d_clipart"];
/// Import Vectors and Export Vectors: the vector formats only.
const VECTOR_IMPORTERS: [&str; 3] = ["import_dxf", "import_svg", "import_crv3d"];
const VECTOR_EXPORTERS: [&str; 5] = [
    "export_dxf",
    "export_svg",
    "export_pdf",
    "export_eps",
    "export_ai",
];
/// The layout keys: F11 designing (Design pinned), F12 machining (Toolpaths pinned).
const LAYOUT_KEYS: [(&str, &str); 3] = [
    ("design_layout", "F11"),
    ("machining_layout", "F12"),
    ("node_edit", "N"),
];
/// The view buttons over the top right of the 2D view: action, label and icon.
const VIEW_BUTTONS: [(&str, &str, &str); 4] = [
    ("pan", "Pan", "view/pan.png"),
    ("zoom_box", "Zoom Box", "view/zoom_box.png"),
    ("zoom_to_job", "Zoom to Job", "view/zoom_to_job.png"),
    (
        "zoom_selected",
        "Zoom to Selection",
        "view/zoom_selected.png",
    ),
];
/// Scrolling a view's height divided by this doubles the zoom.
// ponytail: fixed rate until a preferences dialog exists
const VIEW_ZOOM_QUARTERS: f32 = 4.0;
/// The startup tasks: action, label and icon.
const STARTUP_TASKS: [(&str, &str, &str); 4] = [
    ("new", "Create a new file", "file/new.png"),
    (
        "try_sample_project",
        "Try a sample project",
        "drawing/create_vectors/draw_text.png",
    ),
    (
        "new_from_template",
        "New file from template",
        "toolpaths/load_toolpath_template.png",
    ),
    ("open", "Open an existing file", "file/open.png"),
];
/// The top bar's split and snapping toggles.
const TOP_BAR_TOGGLES: [&str; 5] = [
    "split_stacked",
    "split_side_by_side",
    "toggle_geometry_snapping",
    "toggle_smart_snapping",
    "toggle_grid_snapping",
];
/// The level menu's Select Components and Export as 3D Clipart: these, then the level's id.
pub const SELECT_LEVEL: &str = "select_level_components:";
pub const EXPORT_LEVEL: &str = "export_level_clipart:";
/// Lines (widget outlines, the 2D view's vectors) are egui's widths times this, at the
/// standard text size; they grow and shrink with it.
const LINE_SCALE: f32 = 1.5;
/// Easy-to-read UI fonts, the first installed used; egui's own stay behind for symbols.
const UI_FONTS: [&str; 5] = ["Noto Sans", "Open Sans", "DejaVu Sans", "Segoe UI", "Arial"];
/// A preview starts at ten times the machine's own speed (×1: each move at its tool's feed
/// or plunge rate, rapids at the Toolpath Summary's rapid rate).
pub(crate) const PLAYBACK_SPEED: f64 = 10.0;
/// What a playing preview's GPU surface was made from: the playback's revision, the detail,
/// the job's revision, the wrap and whether the sides are drawn.
type PlaybackGpuKey = (u64, usize, u64, Option<view3d::Wrap>, bool);
/// A frame's time for cutting a playing preview: the window still answers at once.
const PLAYBACK_BUDGET: std::time::Duration = std::time::Duration::from_millis(30);
/// How often a playing preview's 3D view is made again from the cut so far: at full detail
/// that takes a while, so not every frame (the tool still moves every frame).
const PLAYBACK_REDRAW: std::time::Duration = std::time::Duration::from_millis(100);

/// A frame's time for a jump (the Progress slider, Run to Retract): more than for playing,
/// so it gets there within a second or so; the window answers less often meanwhile.
fn jump_budget(budget: std::time::Duration) -> std::time::Duration {
    budget * 4
}
/// How many times faster than the machine a preview can play: a long finishing toolpath
/// takes hours at the machine's speed.
const PLAYBACK_SPEEDS: std::ops::RangeInclusive<f64> = 0.25..=1000.0;
/// The shank of the tool drawn in the 3D view while a preview plays: steel grey.
const TOOL_SHANK: egui::Color32 = egui::Color32::from_rgb(168, 172, 178);
/// Beside each recently opened file.
const RECENT_FILE_ICON: &str = "file/new.png";
/// This version's changes, one `- ` line each, shown on the start page.
const WHATS_NEW: &str = include_str!("../whats-new.md");
/// The repository's contributors on GitHub (login, commits, profile), as
/// `scripts/contributors` last wrote them; Help ▸ About lists them.
const CONTRIBUTORS: &str = include_str!("../contributors.tsv");
/// Machine ▸ Search for Machine Online: the machine packages in Cut Craft's repository.
const MACHINES_ONLINE: &str = "https://github.com/NoobyNull/Cut-Craft/tree/main/machines";
/// Where Cut Craft is found online, in Help ▸ Community (GitHub on the start page too): label,
/// address and icon (the social ones are placeholders: each site's home page).
pub(crate) const LINKS: [(&str, &str, &str); 7] = [
    (
        "GitHub",
        "https://github.com/NoobyNull/cut-craft",
        "file/github_link.png",
    ),
    (
        "Facebook",
        "https://www.facebook.com",
        "file/facebook_link.png",
    ),
    ("X", "https://x.com", "file/twitter_link.png"),
    (
        "YouTube",
        "https://www.youtube.com",
        "file/youtube_link.png",
    ),
    (
        "Instagram",
        "https://www.instagram.com",
        "file/instagram_link.png",
    ),
    (
        "Pinterest",
        "https://www.pinterest.com",
        "file/pinterest_link.png",
    ),
    (
        "Thingiverse",
        "https://www.thingiverse.com",
        "file/thingiverse_link.png",
    ),
];
/// The design panel's starting width, in the theme's interactive widths: room for Job Setup's
/// pictures beside its choices.
// ponytail: fixed until panel widths are saved with the window layout
const DESIGN_PANEL_WIDTHS: f32 = 7.0;
/// The most of the window's width one resizable side panel may take.
// ponytail: fixed share until panel widths are saved with the window layout
const SIDE_PANEL_SHARE: f32 = 1.0 / 3.0;

/// Tool buttons a pinned page always has room for side by side, in a captioned box.
// ponytail: fixed count until a preferences dialog exists
const PAGE_BUTTONS: f32 = 5.0;
/// Inner margin of a captioned box of tools on a page.
const TOOL_BOX_MARGIN: f32 = 4.0;

/// The narrowest a tool page is, outer edge to outer edge: [`PAGE_BUTTONS`] tool buttons in a
/// captioned box, inside the page's frame.
fn least_page_width(style: &egui::Style) -> f32 {
    let s = &style.spacing;
    let button = TOOL_ICON_ROWS * s.interact_size.y + 2.0 * s.button_padding.x;
    let row = PAGE_BUTTONS * button + (PAGE_BUTTONS - 1.0) * s.item_spacing.x;
    // the box's margins and its 1-point border, as `spread_tools` draws it
    let boxed = row + 2.0 * TOOL_BOX_MARGIN + 2.0;
    // room for the frame's border and rounding, so the fifth button never just misses
    let slack = s.item_spacing.x;
    boxed + egui::Frame::side_top_panel(style).total_margin().sum().x + slack
}

/// How wide a side tab strip is, outer edge to outer edge.
fn side_tabs_width(style: &egui::Style) -> f32 {
    style.spacing.interact_size.y + 2.0 * style.spacing.item_spacing.y
}

/// How far the views start below the top of the room the pages share: the central panel's
/// margin, the top bar and the line under it, as `views` lays them out.
fn views_top(style: &egui::Style) -> f32 {
    let s = &style.spacing;
    let bar = s.interact_size.y + 2.0 * s.button_padding.y;
    // `ui.separator()`'s own room, and the gap after the bar and after the line
    let line = 6.0;
    egui::Frame::central_panel(style).total_margin().top + bar + line + 2.0 * s.item_spacing.y
}

/// How wide the Design and Toolpaths pages are in `ui`'s room, docked or flown out (see
/// [`crate::pages::page_width`]). Called with the room between the menu and status bars, before
/// the side tabs take theirs; the result never depends on what the pages show.
fn page_width(ui: &egui::Ui) -> f32 {
    let style = ui.style();
    let room = ui.available_rect_before_wrap();
    let between_tabs = room.width() - 2.0 * side_tabs_width(style);
    crate::pages::page_width(
        (between_tabs, room.height()),
        views_top(style),
        least_page_width(style),
    )
}

/// A tool page docked beside the views, drawn by `add`: exactly `width` wide whatever it
/// shows, so forms and fields sizing themselves to the room can never widen, narrow or
/// shift it, and it never keeps a width from an earlier run. Returns where it is.
///
/// What the page shows is laid out in a child that does not report its size to the panel:
/// egui moves a right-hand panel's inner edge over by however much its contents overflow,
/// so a field a few points too wide would otherwise nudge the view every frame it shows.
fn show_page<R>(
    panel: egui::Panel,
    width: f32,
    ui: &mut egui::Ui,
    add: impl FnOnce(&mut egui::Ui) -> R,
) -> egui::Rect {
    panel
        .resizable(false)
        .exact_size(width)
        .show(ui, |ui| {
            let room = ui.max_rect();
            let mut page = ui.new_child(egui::UiBuilder::new().max_rect(room).layout(*ui.layout()));
            page.shrink_clip_rect(room);
            add(&mut page);
            // the page's whole height, as the panel would have taken
            ui.allocate_rect(room, egui::Sense::hover());
        })
        .response
        .rect
}

/// A resizable page beside the views: it starts [`DESIGN_PANEL_WIDTHS`] wide and never takes
/// more than [`SIDE_PANEL_SHARE`] of the window. Forms and frames inside size themselves to the
/// room they are given, so an unbounded panel could widen a little every frame while the window
/// redraws continuously, until the views were gone.
fn side_panel(panel: egui::Panel, ctx: &egui::Context) -> egui::Panel {
    let width = DESIGN_PANEL_WIDTHS * ctx.global_style().spacing.interact_size.x;
    let most = width.max(SIDE_PANEL_SHARE * ctx.content_rect().width());
    panel.default_size(width).max_size(most)
}
/// Size of a form section's picture, in rows of the theme's interactive height.
// ponytail: fixed ratio until a preferences dialog exists
const SECTION_PICTURE_ROWS: f32 = 3.0;
/// The Cut Craft mark, shown with no job open.
/// How much of the middle of the window the mark takes, by its shorter side.
// ponytail: fixed share until there is a preferences dialog
const SPLASH_SHARE: f32 = 1.0 / 3.0;
const APP_NAME: &str = "Cut Craft";
/// How the desktop knows the window: the same name as the launcher, `cut-craft.desktop`
/// (see `scripts/install-launcher`), so the taskbar shows its icon.
const APP_ID: &str = "cut-craft";
/// Tool icon edge, in rows of the theme's interactive height.
// ponytail: fixed ratio until a preferences dialog exists
const TOOL_ICON_ROWS: f32 = 1.75;

/// What the window shows: the start page, Job Setup for a new job, or the job.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Screen {
    /// No job open: the startup tasks and recently opened files.
    Start,
    /// Job Setup for a new job; OK makes it, Cancel goes back to the start page.
    NewJob,
    Job,
}

/// A 2D view tool that takes the left button from drawing and selecting.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ViewTool {
    Pan,
    /// One box, then back to selecting.
    ZoomBox,
    /// Drag the selected vectors' nodes; right-click a node or span for its menu (N).
    NodeEdit,
    /// Drag between two points for their distance and angle; click a vector for its span.
    Measure,
    /// Handles on the selection to move, scale and rotate it by dragging.
    Transform,
}

#[derive(Debug, PartialEq)]
enum View {
    TwoD,
    ThreeD,
}

/// Both views at once.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Split {
    /// 2D above 3D
    Stacked,
    /// 2D left of 3D
    SideBySide,
}

/// The component editing icons in the 3D view (help: Editing in the 3D View):
/// Total Height, Shape Height, Base Height and Additional Settings, a row of round buttons
/// along the near edge of the selected component's box in Transform mode.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Handle3d {
    /// Dragged up or down: its total height, shape and base together.
    Total,
    /// Dragged up or down: its shape height.
    Height,
    /// Dragged up or down: its base height, the solid bottom it stands on.
    Base,
    /// Opens its combine mode, fade, tilt and appearance.
    Settings,
    /// One of the 3D Transform Handles over the selection's box: the 2D view's handle
    /// at the same place; the rotation arrow above the far edge is `Rotate { x: 0, y: 1 }`.
    Transform(Handle),
}

impl Handle3d {
    const ALL: [Self; 4] = [Self::Total, Self::Height, Self::Base, Self::Settings];

    /// What it does, for the status bar; a rotation handle names the angles of `steps`.
    fn hint(self, steps: handles::Steps) -> String {
        match self {
            Self::Total => "Total height: drag up or down".to_owned(),
            Self::Height => "Shape height: drag up or down".to_owned(),
            Self::Base => "Base height: drag up or down".to_owned(),
            Self::Settings => "Combine mode, fade, tilt and appearance".to_owned(),
            Self::Transform(handle) => handle.hint(steps),
        }
    }

    /// Its picture, drawn inside the round button; the transform handles are painted where
    /// they are placed.
    fn icon(self) -> Option<&'static str> {
        match self {
            Self::Total => Some("modeling/total_height.png"),
            Self::Height => Some("modeling/shape_height.png"),
            Self::Base => Some("modeling/base_height.png"),
            Self::Settings => Some("modeling/additional_settings.png"),
            Self::Transform(_) => None,
        }
    }
}

/// The popup the Settings handle opens: Component Properties' settings as being edited,
/// applied through the command as they change (a dragged value as it goes, one undo).
struct Settings3d {
    component: EntityId,
    params: cmd_component_properties::Params,
    dirty: bool,
    /// While a value is dragged, the undo depth after its last step: the next step takes it
    /// back first.
    dragged: Option<usize>,
    /// Where it opened on screen.
    at: egui::Pos2,
    /// Its combine-mode radios, where they were drawn last frame.
    radios: Vec<(cut_craft_core::component::Combine, egui::Pos2)>,
    /// Set pressed: the fade's or tilt's anchors being clicked in the 3D view, and the
    /// first point once clicked.
    anchors: Option<(Anchor, Option<Point>)>,
}

/// Whose anchors the 3D view's clicks place (help: in the 3D view, the fade's or
/// tilt's direction is set there).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Anchor {
    Fade,
    Tilt,
}

impl Anchor {
    /// What to click next, once `first` is placed or not.
    fn prompt(self, first: bool) -> &'static str {
        match (self, first) {
            (Self::Fade, false) => "Click the point that keeps its height",
            (Self::Fade, true) => "Click the point the component fades down toward",
            (Self::Tilt, false) => "Click the point that does not move",
            (Self::Tilt, true) => "Click the point tilted up",
        }
    }
}

/// Orientate 3D Model's rotation rings on screen (each ring's axis and its points), and the
/// middle sphere's centre and radius.
type Rings3d = (
    Vec<(&'static str, Vec<egui::Pos2>)>,
    Option<(egui::Pos2, f32)>,
);

/// A command whose settings form is open.
struct OpenForm {
    id: &'static str,
    label: &'static str,
    icon: &'static str,
    fields: Vec<Field>,
    params: Value,
    error: Option<String>,
    /// Job Setup for a new job: OK and Cancel instead of Apply and Close.
    new_job: bool,
    /// The tool page it shows in: Toolpaths for toolpath strategies, else Design.
    page: Page,
    /// Edit Toolpath: the toolpath Apply replaces rather than adding a new one.
    editing: Option<EntityId>,
    /// The shape a drawing tool's form last made: its Apply makes it again, in its place,
    /// while it is the one shape selected (as the drawing forms do).
    drawn: Option<EntityId>,
    /// Settings changed since the shape it made last followed them.
    dirty: bool,
    /// What Suggest last found for the form's clearing bits, shown under them.
    suggested: Option<String>,
    /// Orientate 3D Model's Model Size (X, Y, Z) as last shown, and the sizes the user typed
    /// (with the ratio locked, only the latest): the rest follow the model as it turns.
    shown_size: Option<[f64; 3]>,
    typed_size: [Option<f64>; 3],
    /// Orientate 3D Model's model as placed, each triangle with whether a cutter from above
    /// cannot reach it: the 3D view shows it whole while the form is open.
    model: Option<std::sync::Arc<Vec<cmd_orientate_3d_model::Seen>>>,
    /// Whether `model` is the whole model (drawn in place of its component) or, past
    /// Options ▸ Model preview faces, only the faces out of reach (drawn over it).
    model_whole: bool,
    /// The parts of `model` past the material's edges, cut off it along them: the 3D view
    /// shows them as a faint red ghost (Options ▸ Out-of-bounds opacity) so a model placed
    /// partly off the material is seen whole, not clipped.
    /// Its faces are all unmarked: a cutter's reach is not judged past the material.
    beyond: Option<std::sync::Arc<Vec<cmd_orientate_3d_model::Seen>>>,
    /// The settings the shape it made was last made with.
    applied: Option<Value>,
    /// A form that remakes what it made ([`REMAKE_FORMS`]): the history's revision and the selection
    /// just after its pieces were made, and the settings it was made with. While nothing else
    /// has been done or picked since, a change of settings takes them back and makes them
    /// again.
    remade: Option<(u64, cut_craft_core::selection::Selection, Value)>,
    /// While a value is dragged, the undo depth after its last step and the shape and
    /// settings from before that step: the next step takes it back first.
    dragged: Option<(usize, Option<EntityId>, Option<Value>)>,
    /// How many changes could be undone when it opened: Cancel undoes back to there.
    undo_from: usize,
    /// How its drag is read: another way for a toolbox variant (Draw Circle from Edge).
    way: crate::flyout::DragWay,
}

/// See [`CutCraftApp::sketch_areas`].
struct SketchAreas {
    params: serde_json::Value,
    revision: u64,
    selection: cut_craft_core::selection::Selection,
    areas: Vec<cut_craft_core::geometry::Rect>,
}

/// See [`CutCraftApp::carve_area`].
struct CarveArea {
    params: serde_json::Value,
    revision: u64,
    selection: cut_craft_core::selection::Selection,
    /// The area V-Carve would carve, cut into triangles to fill on the 2D view.
    triangles: Vec<[cut_craft_core::geometry::Point; 3]>,
}

#[expect(
    clippy::struct_excessive_bools,
    reason = "the windows and modes it shows"
)]
struct CutCraftApp {
    screen: Screen,
    doc: Document,
    history: History,
    registry: std::sync::Arc<Registry>,
    icons: Icons,
    menus: gui::MenuSystem,
    /// The menus with no job open.
    start_menus: gui::MenuSystem,
    toolbar: gui::Toolbar,
    status: gui::StatusBar,
    design_tab: usize,
    /// The Design tab's open sub-tab (see [`gui::DESIGN_SUBTABS`]).
    design_subtab: usize,
    /// The Design sub-tab shown as a section to unfold next frame: a tool on it was picked
    /// from a menu, a shortcut or Find Tool.
    design_unfold: Option<&'static str>,
    /// The Toolpaths page's open sub-tab (see [`gui::TOOLPATH_SUBTABS`]).
    toolpath_subtab: usize,
    /// The Toolpaths sub-tab shown as a section to unfold next frame, as `design_unfold`.
    toolpath_unfold: Option<&'static str>,
    /// How many toolpaths the job had when the Toolpaths page last looked: one more opens
    /// the List sub-tab, so a toolpath just made is seen there.
    toolpaths_seen: usize,
    /// Each tool's first step, once asked for (see [`guide::first_step`]).
    first_steps: std::cell::RefCell<std::collections::HashMap<String, Option<String>>>,
    view: View,
    cursor: Option<Point>,
    form: Option<OpenForm>,
    /// Trace Bitmap's trace window, while its form is open.
    trace_view: Option<trace_window::TraceView>,
    /// How the 2D view mapped the job last frame.
    view_transform: Option<ViewTransform>,
    /// Where the 2D view was on screen last frame.
    view_area: Option<egui::Rect>,
    /// Where the 3D view was last drawn, for the sculpting brush's wheel.
    view_3d_area: Option<egui::Rect>,
    /// View ▸ Save Shaded Image: the file the 3D view's picture goes to, until the window's
    /// picture it is cut from is asked for; then what saving it said, for the status bar.
    shaded_image: Option<std::path::PathBuf>,
    shaded_saved: std::sync::Arc<std::sync::Mutex<Option<String>>>,
    /// The user's zoom and pan; `None` fits the job to the view.
    camera: Option<ViewTransform>,
    view_tool: Option<ViewTool>,
    /// The Design and Toolpaths pages: pinned open, or hidden to a side tab.
    pages: Pages,
    /// Where the docked Design and Toolpaths pages were drawn last frame, if docked.
    page_rects: [Option<egui::Rect>; 2],
    /// Where a flown-out page was drawn last frame, if one was.
    flyout_rect: Option<egui::Rect>,
    /// How many tool buttons fit side by side in a captioned box on the last page drawn.
    buttons_across: std::cell::Cell<f32>,
    /// Find Tool (Ctrl+K): the text typed, while its window is open.
    search: Option<String>,
    /// The browser's tab (`INSPECTOR_TABS`) in the Rail and Ribbon layouts.
    inspector_tab: usize,
    /// The Rail layout's open group (an index into `all_groups`), and whether its list
    /// stays open after a tool is picked.
    rail_group: Option<usize>,
    rail_pinned: bool,
    /// The Ribbon layout's tab (`RIBBON_TABS`).
    ribbon_tab: usize,
    /// Where the options are saved; `None` keeps them in memory (tests).
    options_path: Option<std::path::PathBuf>,
    /// The user's own spelling words, one a line; none in tests.
    words_path: Option<std::path::PathBuf>,
    /// Draw Text's dictionary, looked for once when spelling is first wanted.
    speller: Option<cut_craft_core::spell::Speller>,
    speller_looked: bool,
    session: Session,
    /// Toolpath picked in the Toolpaths panel.
    selected_toolpath: Option<EntityId>,
    /// The toolpath group picked in the list instead of a toolpath: previewed and saved as
    /// one, and joined by the toolpaths made while it is picked.
    selected_group: Option<String>,
    /// Quick Engrave's Immediate Output: the post and side name it saves with.
    immediate_output: SaveForm,
    /// Relief from Photo's Estimate Depth, while the Depth add-on works on a picture.
    depth_estimate: Option<crate::depth::Estimating>,
    /// A slow job running off the window's thread, its progress shown over the window.
    busy: Option<crate::busy::Busy<CutCraftApp>>,
    /// How long a job may take before its progress is shown; quicker ones finish as if
    /// run in place.
    wait_before_showing: std::time::Duration,
    /// Gadgets ▸ Add-ons, and the installs it (or Relief from Photo) starts; where add-ons
    /// are installed (none in tests).
    addons: crate::addons::Addons,
    /// The machines (Machine Configuration), the active one's default post saving
    /// toolpaths; without one, Cut Craft's own `FluidNC` post for the job does.
    machines: cut_craft_machines::Machines,
    /// Where `machines` is kept; `None` keeps them in memory (tests).
    machines_path: Option<std::path::PathBuf>,
    /// Cut Craft's posts and the user's custom ones.
    posts: cut_craft_machines::Library,
    /// The open Machine Configuration Management and Post-Processor Management windows.
    machine_window: Option<cut_craft_machines::MachineManager>,
    /// First-time setup's questions, while they are shown.
    first_run: Option<first_run::Setup>,
    post_window: Option<cut_craft_machines::PostManager>,
    tools: ToolDatabase,
    /// Where `tools` is saved; `None` keeps it in memory (tests, or a file that failed to load).
    tools_path: Option<std::path::PathBuf>,
    /// The open Tool Database window.
    tool_manager: Option<cut_craft_tool_database::Manager>,
    /// What the open Select Tool picks for: a tool in one of the form's lists, or a tool
    /// changer holder; `None` picks the form's own tool.
    picking: Option<PickFor>,
    /// A Calculate waiting on the deeper-than-the-material warning.
    deep_cut: Option<depth::DeepCut>,
    /// An import waiting on which units its drawing is in.
    units_ask: Option<units_ask::UnitsAsk>,
    /// Right-click ▸ Report a Bug Here…
    reporter: cut_craft_bug_report::Reporter,
    /// Help ▸ Check for Updates, and the check at startup.
    updates: crate::update::Updates,
    /// Restart Now: the new version, started once the window has closed.
    restart_after_close: Option<std::path::PathBuf>,
    /// Gadgets from the My Gadgets folder, not from Cut Craft's own signed builds: id → label.
    unsigned_gadgets: std::collections::HashMap<String, String>,
    /// The unsigned gadgets the user has said yes to running this session.
    unsigned_gadgets_allowed: std::collections::HashSet<String>,
    /// Asks whether to run the unsigned gadget with this label (a dialog; tests answer).
    unsigned_gadget_ok: fn(&str) -> bool,
    /// A SpaceMouse's moves, once the app is running (not in tests or screenshots).
    space_mouse: Option<std::sync::mpsc::Receiver<crate::spacemouse::Input>>,
    /// `--screenshots`: the README's pictures being taken.
    shots: Option<screenshots::Shots>,
    /// What Cut and Copy last took, pasted back in place.
    clipboard: Vec<cut_craft_core::geometry::Path>,
    clipboard_components: Vec<cut_craft_core::component::Component>,
    /// Where the 3D view's picture was fitted last frame, for picking.
    fit_3d: Option<egui::Rect>,
    /// The 3D view's edit handles, where they were drawn last frame.
    handles_3d: Vec<(Handle3d, egui::Pos2)>,
    /// The components under the 3D view's last right-click, topmost first.
    under_3d: Vec<EntityId>,
    /// A handle being dragged, from where the pointer pressed.
    drag_3d: Option<(Handle3d, egui::Pos2)>,
    /// A height handle's drag as it follows the pointer: the component's settings and its
    /// top when the drag began, and the settings last applied (undone and applied again
    /// each frame the pointer moves, so the whole drag is one undo).
    drag_base: Option<(cmd_component_properties::Params, f64, Option<Value>)>,
    settings_3d: Option<Settings3d>,
    /// Orientate 3D Model's rotation handles as drawn last frame: each ring's axis
    /// (`x`, `y`, `z`) and its points on screen, then the middle sphere's centre and radius.
    rings_3d: Rings3d,
    /// The handle a drag started on, which the drag then turns the model about (`xyz`: the
    /// sphere, all three axes).
    turn_ring: Option<&'static str>,
    /// An open Help window.
    info: Option<Info>,
    /// Help Contents' rendered pages and the `help:` links it watches.
    help_cache: egui_commonmark::CommonMarkCache,
    /// What is typed in the Help window's search box.
    help_search: String,
    /// Every help page as (topic, title, Markdown), made the first time help is searched.
    help_index: Vec<(&'static str, String, String)>,
    /// The Did You Know? window, open on this tip.
    tip: Option<usize>,
    /// The Did You Know? window's rendered tips.
    tips_cache: egui_commonmark::CommonMarkCache,
    /// Sketch Carving's green areas, with the form settings, revision and selection they
    /// were found for.
    sketch_areas: Option<SketchAreas>,
    /// V-Carve's shaded area while its form is open, with the form settings, revision and
    /// selection it was found for.
    carve_area: Option<CarveArea>,
    /// The open form's reach line's measured shapes, kept while nothing they depend on changes.
    reach_found: Option<reach::ReachFound>,
    /// Toolpath Summary's settings.
    summary: Summary,
    /// Where the pointer has been during this drag, for stroke tools such as Freehand.
    stroke: Vec<Point>,
    /// The previewed material before the last preview, for Undo Last Preview.
    before_preview: Option<BeforePreview>,
    /// Node editing: the nodes picked with a box, which a drag of one moves together.
    picked_nodes: Vec<cut_craft_core::nodes::NodeAt>,
    /// Node editing's mirror lines, H and V: with the first, nodes either side of the
    /// selection's middle move opposite ways across; with the second, up and down.
    node_mirror: [bool; 2],
    /// Where the 2D view's right-click menu last opened, in job coordinates: guides inserted
    /// from it go there.
    menu_at: Option<Point>,
    /// Guide Properties, open on a guide.
    guide_props: Option<GuideProps>,
    /// Machine ▸ Tool Changer…: the active machine's holders loaded from the tools.
    tool_changer_window: bool,
    /// Import Into Selected Shape's drawing and its window, while it is open.
    fit_import: Option<fit_import::FitImport>,
    /// Measure / Inspect's last result, shown in its form while the tool is on.
    measure_rows: Vec<(String, String)>,
    /// Save Toolpaths' form, while it is open.
    save_form: Option<SaveForm>,
    /// Machine ▸ Machine Files, while it is open.
    machine_files: Option<send::MachineFiles>,
    /// The checks shown in Save Toolpaths' form.
    save_checks: Option<SaveChecks>,
    /// Whether Snapping Options (F4) is open; the options are kept in `options.snapping`.
    snap_window: bool,
    /// Gadgets ▸ Job Setup Sheet Editor is open.
    job_sheet_window: bool,
    /// Toolpaths ▸ Print Part Labels is open.
    part_labels_window: bool,
    /// Export as STL's form is open.
    stl_window: bool,
    /// The triangle count and largest error of its last Triangulate.
    stl_result: Option<(usize, f64)>,
    /// The Layers List leaves out the layers with nothing on the active sheet.
    layers_on_sheet: bool,
    /// What the views draw.
    display: crate::display::Display,
    /// The material as cut by the previewed toolpaths, and its picture once drawn.
    simulation: Option<Material>,
    /// `simulation` at the 3D view's moving detail, and that detail (mesh cells).
    simulation_coarse: Option<(usize, Material)>,
    /// The model alone at the 3D view's moving detail (picking, handles, the quick mesh):
    /// made once a change, not every frame.
    rough_3d: Option<(RoughKey, Material)>,
    /// How many toolpaths were out of date last frame.
    stale_known: usize,
    /// Toolpaths went out of date while a form was open: once it closes, ask whether to
    /// recalculate them.
    stale_while_form: bool,
    /// The "recalculate all?" question showing: how many toolpaths are out of date, and
    /// whether Don't ask again is ticked.
    recalculate_ask: Option<(usize, bool)>,
    /// Counts the jobs opened or made: with the history's revision, which job and which
    /// change of it the 2D view draws (a new job's history starts its revisions again).
    job_epoch: u64,
    /// When a height handle's drag last remade the model (the app's clock, seconds).
    height_applied_at: Option<f64>,
    /// The level clipart imported through Orientate 3D Model goes on (Import to Level),
    /// until another command starts.
    import_level: Option<u64>,
    /// Draw Line / Polyline's polyline being drawn in the 2D view.
    polyline: crate::polyline::Draft,
    /// Points the drawing being made was placed on snaps at, and what they snapped to: kept
    /// there once it is made.
    placed: Vec<(
        cut_craft_core::geometry::Point,
        cut_craft_core::constraint::Anchor,
    )>,
    /// The snap mark picked in the 2D view (an index into the job's constraints): Delete
    /// lets that point go.
    picked_constraint: Option<usize>,
    /// The 3D view's fine mesh once it is still, with what it was made for.
    fine_3d: Option<(FineKey, egui::Mesh)>,
    /// The model at the fine mesh's detail, for drawing in the 3D view to hit what is drawn,
    /// with the history revision, job and detail when still (mesh cells) it was made for.
    drawn_on: Option<(
        (u64, cut_craft_core::document::Job, usize),
        std::sync::Arc<Material>,
    )>,
    /// The imported model's triangles as last drawn, kept while nothing about the view or
    /// the model changes: sorting and lighting a big model every frame is what made the
    /// window drag while a model was being imported.
    model_3d: Option<(ModelKey, std::sync::Arc<egui::Mesh>)>,
    /// The window's GPU, when it draws with wgpu, for the 3D view to draw on (the headless
    /// tests have none and draw the meshes above).
    gpu_3d: Option<view3d::gpu::Gpu>,
    /// The 3D view's surface as the GPU draws it, with what it was built from.
    surface_gpu: Option<SurfaceGpu>,
    /// The model's walls and bottom, made with the surface they hang from (no preview).
    solid_gpu: Option<std::sync::Arc<view3d::gpu::Geometry>>,
    /// The imported model as the GPU draws it.
    model_gpu: Option<ModelGpu>,
    /// The same for the imported model's parts past the material's edges.
    ghost_gpu: Option<ModelGpu>,
    /// When the 3D view last moved (input time, seconds).
    moved_3d: f64,
    /// Picks the 3D view's detail while it moves from how quickly frames come (Options ▸
    /// Adjust detail to keep the view smooth).
    pacer: crate::frame_rate::Pacer,
    /// The detail the 3D view draws at this frame, and whether it is moving; the frame
    /// (`egui::Context::cumulative_pass_nr`) that was judged.
    detail_3d: (crate::options::Detail, bool),
    paced_pass: u64,
    orbit: view3d::Orbit,
    framing: view3d::Framing,
    /// Where the 3D view last drew its view cube.
    view_cube: Option<view3d::ViewCube>,
    /// Jobs opened or saved lately, newest first.
    recent: Vec<std::path::PathBuf>,
    /// A preview playing (or paused): the tool cutting the material a distance at a time.
    playback: Option<cut_craft_core::playback::Playback>,
    playing: bool,
    /// Where the tool is being moved to (Single Step, Run to Retract, the Progress slider),
    /// cut a little each frame until it gets there, or Stop.
    playback_goal: Option<f64>,
    /// A preview stopped or played to its end, kept so its controls stay to scrub it back
    /// and forth, till another preview or Reset Preview.
    kept_playback: Option<cut_craft_core::playback::Playback>,
    /// A playing preview's material as the 3D view draws it, and the GPU's surface of it,
    /// kept by what they were made from (the playback's revision, the detail and the job's
    /// revision; for the GPU, its wrap and sides too) so a paused preview is not made again
    /// each frame, nor a moving one more than once a frame.
    playback_view: Option<((u64, usize, u64), Material)>,
    playback_gpu: Option<(PlaybackGpuKey, std::sync::Arc<view3d::gpu::Geometry>)>,
    /// How many times the playback's view has been made (for tests).
    playback_builds: usize,
    /// The playback's revision the 3D view shows, and when it was taken: while it plays
    /// it is taken again every [`PLAYBACK_REDRAW`], and at once when it stops somewhere.
    playback_shown: Option<(u64, std::time::Instant)>,
    /// How often that is, [`PLAYBACK_REDRAW`] (longer in tests, whose frames are slow).
    playback_redraw: std::time::Duration,
    /// Where the Close up slider is centred, held while it is dragged so it stays put.
    close_up: Option<f64>,
    /// How long a frame may spend cutting a playing preview; the rest waits for the next
    /// frame, so a long toolpath plays slower instead of freezing the window.
    playback_budget: std::time::Duration,
    /// How many times faster than the machine the preview plays (1: the machine's speed).
    playback_speed: f64,
    /// Component tree levels collapsed (Expand / Collapse).
    collapsed_levels: std::collections::HashSet<u64>,
    /// A guide being dragged: whether it is vertical, and which it is (`None` a new one).
    guide_drag: Option<(bool, Option<usize>)>,
    /// Transform mode's corners rotate rather than scale.
    rotating: bool,
    /// Edit ▸ Options, kept between runs, and whether its window is open.
    options: crate::options::Options,
    options_window: bool,
    /// The Options category shown, by its place in the list.
    options_page: usize,
    /// Show advanced settings is ticked: the Advanced category lists. Off at every start, so
    /// reaching them is always a deliberate step.
    show_advanced: bool,
    /// Both views shown at once.
    split: Option<Split>,
    /// The Library tab.
    clipart: crate::clipart::Clipart,
    /// Files from the Library waiting to be added, one after another, each with the level
    /// Add to Level puts it on.
    import_queue: std::collections::VecDeque<(std::path::PathBuf, Option<crate::clipart::ToLevel>)>,
    /// Where `recent` is saved; `None` keeps it in memory (tests).
    recent_path: Option<std::path::PathBuf>,
    /// Where unsaved work is copied (as often as Options says) for the next run to recover;
    /// `None` keeps no copy (tests).
    recovery_path: Option<std::path::PathBuf>,
    /// The history revision and input time (seconds) of the last recovery copy.
    autosaved: (u64, f64),
    /// A recovery copy being written.
    autosave: Option<std::thread::JoinHandle<Result<(), String>>>,
    /// Window title last sent to the viewport.
    title: String,
    /// Whether the window opened with the desktop's title bar. A Mac keeps the window it opened
    /// with until Cut Craft restarts, so its menu row follows this rather than Options.
    title_bar_at_start: bool,
}

/// A sheet's size in the Sheets List: one decimal in mm, three in inches.
fn sheet_length(mm: f64, units: cut_craft_core::document::Units) -> String {
    match units {
        cut_craft_core::document::Units::Mm => format!("{mm:.1}"),
        cut_craft_core::document::Units::Inch => format!("{:.3}", mm / 25.4),
    }
}

/// A length in the job's units.
fn length(mm: f64, units: cut_craft_core::document::Units) -> String {
    match units {
        cut_craft_core::document::Units::Mm => format!("{mm:.3} mm"),
        cut_craft_core::document::Units::Inch => format!("{:.4} in", mm / 25.4),
    }
}

/// Measure between two points, as rows of the Measure / Inspect form: the distance, the angle
/// from horizontal, the X and Y distances and the two points.
fn measured_rows(
    from: Point,
    to: Point,
    units: cut_craft_core::document::Units,
) -> Vec<(String, String)> {
    let d = to - from;
    let point = |p: Point| format!("{}, {}", length(p.x, units), length(p.y, units));
    vec![
        ("Distance".into(), length(d.length(), units)),
        (
            "Angle".into(),
            format!("{:.2}°", d.y.atan2(d.x).to_degrees()),
        ),
        ("X distance".into(), length(d.x.abs(), units)),
        ("Y distance".into(), length(d.y.abs(), units)),
        ("First point".into(), point(from)),
        ("Second point".into(), point(to)),
    ]
}

/// The first four rows of [`measured_rows`] on one line, for the status bar.
fn measured(from: Point, to: Point, units: cut_craft_core::document::Units) -> String {
    let rows = measured_rows(from, to, units);
    format!(
        "Distance {}, angle {}, X {}, Y {}",
        rows[0].1, rows[1].1, rows[2].1, rows[3].1
    )
}

/// The clicked span's type, length, ends (and an arc's centre and radius), and its vector's
/// perimeter, area and span count, as rows of the Measure / Inspect form.
fn inspected_rows(
    path: &Path,
    at: Point,
    reach: f64,
    units: cut_craft_core::document::Units,
) -> Vec<(String, String)> {
    use cut_craft_core::geometry::Segment;
    let point = |p: Point| format!("{}, {}", length(p.x, units), length(p.y, units));
    let span = |k: usize| {
        let from = if k == 0 {
            path.start
        } else {
            path.segments[k - 1].end()
        };
        Path {
            start: from,
            segments: vec![path.segments[k]],
            closed: false,
        }
    };
    let measure = |p: &Path| {
        p.flatten(cut_craft_core::polygon::TOLERANCE)
            .windows(2)
            .map(|w| w[0].distance(w[1]))
            .sum::<f64>()
    };
    let mut rows = vec![];
    if let Some(k) = cut_craft_core::nodes::span_near(path, at, reach) {
        let one = span(k);
        let kind = match path.segments[k] {
            Segment::Line { .. } => "Line",
            Segment::Arc { .. } => "Arc",
            Segment::Cubic { .. } => "Bezier Curve",
        };
        rows.push(("Span".into(), kind.into()));
        rows.push(("Span length".into(), length(measure(&one), units)));
        rows.push(("Start".into(), point(one.start)));
        rows.push(("End".into(), point(path.segments[k].end())));
        if let Segment::Arc { center, .. } = path.segments[k] {
            rows.push(("Centre".into(), point(center)));
            rows.push(("Radius".into(), length(center.distance(one.start), units)));
        }
    }
    rows.push(("Vector perimeter".into(), length(measure(path), units)));
    if path.closed {
        let area = cut_craft_core::polygon::area(path);
        rows.push((
            "Vector area".into(),
            match units {
                cut_craft_core::document::Units::Mm => format!("{area:.3} mm²"),
                cut_craft_core::document::Units::Inch => format!("{:.4} in²", area / 645.16),
            },
        ));
    }
    rows.push(("Number of spans".into(), path.segments.len().to_string()));
    rows
}

/// [`inspected_rows`] on one line, for the status bar.
fn inspected(path: &Path, at: Point, reach: f64, units: cut_craft_core::document::Units) -> String {
    let rows = inspected_rows(path, at, reach, units);
    let get = |name: &str| rows.iter().find(|r| r.0 == name).map(|r| r.1.as_str());
    let span = match (get("Span"), get("Span length")) {
        (Some(kind), Some(len)) => format!("{kind}, length {len}; "),
        _ => String::new(),
    };
    let area = get("Vector area").map_or_else(String::new, |a| format!(", area {a}"));
    let perimeter = get("Vector perimeter").unwrap_or("");
    format!("{span}vector perimeter {perimeter}{area}")
}

/// Hours, minutes and seconds.
fn hms(minutes: f64) -> String {
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "a positive time"
    )]
    let s = (minutes * 60.0).round() as u64;
    format!("{}:{:02}:{:02}", s / 3600, s / 60 % 60, s % 60)
}

/// Each toolpath's estimated time, the visible ones' total, and the rate (in the units
/// picked) and scale they use; a toolpath clicked has its notes written below, and Apply
/// gives back the notes to keep with it.
/// The Machine Acceleration: the summary's times allow for the active `machine`'s
/// accelerations (its name and X, Y, Z from Machine Configuration), while there is one.
fn acceleration_tick(
    ui: &mut egui::Ui,
    summary: &mut Summary,
    machine: Option<(String, [f64; 3])>,
) {
    let (name, acceleration) = machine.unzip();
    let tip = match acceleration {
        Some([x, y, z]) => format!(
            "Stops at corners and speeds up and slows down along each move as {} allows: \
             {x:.0}, {y:.0}, {z:.0} mm/s² (Machine Configuration)",
            name.unwrap_or_default()
        ),
        None => "Needs an active machine with its accelerations in Machine Configuration".into(),
    };
    ui.add_enabled(
        acceleration.is_some(),
        egui::Checkbox::new(&mut summary.acceleration, "Machine acceleration"),
    )
    .on_hover_hint(&tip)
    .on_disabled_hover_text(&tip);
}

/// The Toolpath Summary's own settings: the rapid rate and the time scale factor.
fn summary_fields() -> [crate::form::Field; 2] {
    [
        crate::form::Field {
            key: "rapid_rate".into(),
            label: "Rapid rate".into(),
            help: Some("The machine's G0 speed".into()),
            kind: crate::form::Kind::Number {
                unit: None,
                min: Some(0.001),
                max: None,
                integer: false,
                steps: vec![],
                slider: false,
                percent_when: None,
            },
            section: false,
            icon: None,
            when: None,
            enabled: None,
            group: None,
            tab: None,
            dim: None,
            diagram: None,
        },
        crate::form::Field {
            key: "scale_factor".into(),
            label: "Scale factor".into(),
            help: Some(
                "Multiplies every time, for a machine slower than its programmed feeds".into(),
            ),
            kind: crate::form::Kind::Number {
                unit: None,
                min: Some(0.1),
                max: None,
                integer: false,
                steps: vec![],
                slider: false,
                percent_when: None,
            },
            section: false,
            icon: None,
            when: None,
            enabled: None,
            group: None,
            tab: None,
            dim: None,
            diagram: None,
        },
    ]
}

fn summary_table(
    ui: &mut egui::Ui,
    rows: &[SummaryRow],
    summary: &mut Summary,
    units: cut_craft_core::document::Units,
    machine: Option<(String, [f64; 3])>,
) -> Option<(EntityId, String)> {
    let total: f64 = rows.iter().filter(|r| r.visible).map(|r| r.minutes).sum();
    ui.strong(format!("Total machining time: {}", hms(total)));
    ui.separator();
    egui::Grid::new("summary").striped(true).show(ui, |ui| {
        for row in rows {
            let picked = summary.picked == Some(row.id);
            let name = ui
                .add_enabled(row.visible, egui::Button::selectable(picked, &row.name))
                .on_hover_hint("Click to write its notes");
            if name.clicked() {
                summary.picked = Some(row.id);
                summary.notes.clone_from(&row.notes);
            }
            ui.label(hms(row.minutes));
            ui.weak(row.notes.lines().next().unwrap_or_default());
            ui.end_row();
        }
    });
    ui.separator();
    ui.weak("Time estimates based on the tools' feed rates");
    let per = summary.units.mm_per_min();
    let fields = summary_fields();
    let mut values = serde_json::json!({
        "rapid_rate": summary.rapid / per,
        "scale_factor": summary.scale,
    });
    let picture = |_: &str| None;
    let look = crate::form::Look {
        units,
        picture: &picture,
        material: cut_craft_core::calc::Values::default(),
        tool: None,
        group_end: None,
    };
    if crate::form::show(ui, &fields, &mut values, &look) {
        summary.rapid = values["rapid_rate"]
            .as_f64()
            .map_or(summary.rapid, |v| v * per);
        summary.scale = values["scale_factor"].as_f64().unwrap_or(summary.scale);
    }
    egui::ComboBox::from_label("Rapid rate units")
        .selected_text(summary.units.label())
        .show_ui(ui, |ui| {
            for u in cut_craft_core::toolpath::RateUnits::ALL {
                ui.selectable_value(&mut summary.units, u, u.label());
            }
        });
    acceleration_tick(ui, summary, machine);
    let picked = rows.iter().find(|r| Some(r.id) == summary.picked)?;
    ui.separator();
    ui.label(format!("Notes for {}", picked.name));
    ui.add(egui::TextEdit::multiline(&mut summary.notes).desired_rows(3));
    ui.button("Apply")
        .clicked()
        .then(|| (picked.id, summary.notes.clone()))
}

use cut_craft_core::file::TEMPLATE_EXTENSION as TEMPLATE;

/// Job templates, starting in Cut Craft's `templates` folder.
/// A file dialog opening in one of the user's settings folders (made if need be).
fn settings_dialog(filter: &str, extension: &str, folder: &str) -> rfd::FileDialog {
    let dialog = file_dialog().add_filter(filter, &[extension]);
    match files::config_dir().map(|d| d.join(folder)) {
        Some(dir) if std::fs::create_dir_all(&dir).is_ok() => dialog.set_directory(dir),
        _ => dialog,
    }
}

fn template_dialog() -> rfd::FileDialog {
    settings_dialog("Cut Craft job template", TEMPLATE, "templates")
}

/// Sculpting brushes, kept in the brushes folder.
fn brush_dialog() -> rfd::FileDialog {
    settings_dialog("Sculpting brush", "ccclip", "brushes")
}

/// Texture Toolpath's textures, kept in the textures folder.
fn texture_dialog() -> rfd::FileDialog {
    settings_dialog(
        "Texture settings",
        cmd_texture_toolpath::TEXTURE_EXTENSION,
        "textures",
    )
}

/// Toolpath templates, in the same `templates` folder.
fn toolpath_template_dialog() -> rfd::FileDialog {
    settings_dialog(
        "Toolpath template",
        files::TOOLPATH_TEMPLATE_EXTENSION,
        "templates",
    )
}

/// A toolpath form's saved settings, in the same `templates` folder.
fn form_settings_dialog() -> rfd::FileDialog {
    settings_dialog(
        "Toolpath settings",
        files::FORM_SETTINGS_EXTENSION,
        "templates",
    )
}

fn job_dialog() -> rfd::FileDialog {
    file_dialog().add_filter("Cut Craft job", &[JOB_EXTENSION, OLD_JOB_EXTENSION])
}

/// The system's file dialog. Tests must never open one: it lands on the desktop of whoever
/// runs them and waits for an answer.
fn file_dialog() -> rfd::FileDialog {
    on_the_desktop();
    let dialog = rfd::FileDialog::new();
    let dialog = match dialog_parent() {
        Some(parent) => dialog.set_parent(&*parent),
        None => dialog,
    };
    match DIALOG_FOLDER.lock().ok().and_then(|f| f.clone()) {
        Some(folder) => dialog.set_directory(folder),
        None => dialog,
    }
}

/// Where file dialogs open (Options ▸ File dialog default), kept up to date each frame; the
/// dialogs are opened from places that do not have the app to hand.
static DIALOG_FOLDER: std::sync::Mutex<Option<std::path::PathBuf>> = std::sync::Mutex::new(None);

/// A message box from the system, as the window's own like the file dialogs.
fn message_dialog() -> rfd::MessageDialog {
    on_the_desktop();
    let dialog = rfd::MessageDialog::new();
    match dialog_parent() {
        Some(parent) => dialog.set_parent(&*parent),
        None => dialog,
    }
}

/// Cut Craft's window, kept up to date each frame: the system's dialogs open as its own,
/// kept above it and brought forward with it, rather than as windows of their own.
static DIALOG_PARENT: std::sync::Mutex<Option<std::sync::Arc<dyn Window>>> =
    std::sync::Mutex::new(None);

/// A window a dialog can belong to (eframe's is winit's).
trait Window:
    raw_window_handle::HasWindowHandle + raw_window_handle::HasDisplayHandle + Send + Sync
{
}

impl<W: raw_window_handle::HasWindowHandle + raw_window_handle::HasDisplayHandle + Send + Sync>
    Window for W
{
}

fn dialog_parent() -> Option<std::sync::Arc<dyn Window>> {
    DIALOG_PARENT.lock().ok()?.clone()
}

/// Every window or program the app puts on the desktop passes here; in tests it fails.
fn on_the_desktop() {
    #[cfg(test)]
    panic!("a test opened a real dialog or program: list the control in needs_a_person");
}

/// What drawing the menus needs from the app.
struct Menus<'a> {
    available: &'a dyn Fn(&str) -> bool,
    /// Ticked state of the items that are settings; `None` keeps the item's own.
    checked: &'a dyn Fn(&str) -> Option<bool>,
    shortcut_of: &'a dyn Fn(&str) -> Option<&'static str>,
    /// Filled in where the menus have a Recent Files entry.
    recent: &'a [gui::MenuItem],
    /// An item's picture by its icon path; `None` when the icon is missing.
    image: &'a dyn Fn(&str) -> Option<egui::Image<'static>>,
}

/// Menu entries, recursing into submenus. Sets `clicked` to the chosen item's action.
fn menu_items(
    ui: &mut egui::Ui,
    items: &[gui::MenuItem],
    menus: &Menus,
    clicked: &mut Option<String>,
) {
    // one line per item: the menu widens to its longest name instead
    ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Extend);
    // each category in a border, all as wide as the widest item
    let font = egui::TextStyle::Button.resolve(ui.style());
    let text = |t: &str| {
        ui.painter()
            .layout_no_wrap(t.to_owned(), font.clone(), egui::Color32::PLACEHOLDER)
            .size()
            .x
    };
    let spacing = ui.spacing();
    let room = 2.0 * (spacing.button_padding.x + spacing.item_spacing.x) + spacing.icon_width;
    let widest = items
        .iter()
        .map(|i| {
            let shortcut = (menus.shortcut_of)(&i.action).unwrap_or(&i.shortcut);
            text(&i.name) + text(&shortcut_label(ui.ctx(), shortcut)) + room
        })
        .fold(0.0, f32::max);
    for category in gui::categories(items) {
        egui::Frame::group(ui.style()).show(ui, |ui| {
            ui.set_min_width(widest);
            ui.with_layout(Layout::top_down_justified(Align::LEFT), |ui| {
                for item in category {
                    menu_item(ui, item, menus, clicked);
                }
            });
        });
    }
}

/// A menu item's shortcut as the system writes them: `Ctrl+Z`, or ⌘Z on a Mac.
fn shortcut_label<'a>(ctx: &egui::Context, text: &'a str) -> Cow<'a, str> {
    if mac::is_mac(ctx) {
        Cow::Owned(mac::shortcut_text(text))
    } else {
        Cow::Borrowed(text)
    }
}

/// The app's fonts: the system's face (`system`, from [`UI_FONTS`]) first, then egui's own,
/// which every system has. Those end with Hack, for the arrows and ▸ of menu paths, and the
/// monochrome emoji fonts, for ⚠ and ★: a character none of them has draws as an empty box,
/// which `every_label_character_has_a_glyph` guards against.
fn ui_fonts(system: Option<(String, Vec<u8>, u32)>) -> egui::FontDefinitions {
    let mut fonts = egui::FontDefinitions::default();
    let proportional = fonts
        .families
        .entry(egui::FontFamily::Proportional)
        .or_default();
    proportional.push("Hack".to_owned());
    if let Some((name, data, index)) = system {
        proportional.insert(0, name.clone());
        let mut data = egui::FontData::from_owned(data);
        data.index = index;
        fonts.font_data.insert(name, std::sync::Arc::new(data));
    }
    fonts
}

/// An easier-to-read look: a heavier system font, text and controls `text_scale` times egui's
/// (Options ▸ Appearance ▸ Text size), and lines thicker to match; on a Mac, the Mac's own
/// font and look ([`mac`]).
fn readable(ctx: &egui::Context, text_scale: f32) {
    let line_scale = LINE_SCALE * text_scale / 1.15;
    let families: &[&str] = if mac::is_mac(ctx) {
        &mac::UI_FONTS
    } else {
        &UI_FONTS
    };
    let fonts = ui_fonts(cut_craft_core::font::system_face(families));
    ctx.set_fonts(fonts);
    ctx.all_styles_mut(|style| {
        for font in style.text_styles.values_mut() {
            font.size *= text_scale;
        }
        style.spacing.interact_size *= text_scale;
        let v = &mut style.visuals;
        for w in [
            &mut v.widgets.noninteractive,
            &mut v.widgets.inactive,
            &mut v.widgets.hovered,
            &mut v.widgets.active,
            &mut v.widgets.open,
        ] {
            w.fg_stroke.width *= line_scale;
            w.bg_stroke.width *= line_scale;
        }
        v.window_stroke.width *= line_scale;
        v.selection.stroke.width *= line_scale;
    });
    if mac::is_mac(ctx) {
        mac::style(ctx);
    }
}

/// Where the keyboard focus ring goes: round the control Tab has focused, if any.
fn focus_ring(ctx: &egui::Context) -> Option<egui::Rect> {
    let id = ctx.memory(egui::Memory::focused)?;
    ctx.read_response(id).map(|r| r.rect)
}

/// The focus ring for mouseless use: a clear outline round the focused control, over
/// everything else.
fn paint_focus_ring(ctx: &egui::Context) {
    let Some(rect) = focus_ring(ctx) else {
        return;
    };
    let layer = egui::LayerId::new(egui::Order::Foreground, egui::Id::unique("focus_ring"));
    let stroke = ctx.global_style().visuals.selection.stroke;
    let width = stroke.width * 2.0;
    ctx.layer_painter(layer).rect_stroke(
        rect.expand(width),
        ctx.global_style().visuals.widgets.inactive.corner_radius,
        egui::Stroke::new(width, stroke.color),
        egui::StrokeKind::Outside,
    );
}

/// What a toolpath form's Tool section asks for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ToolAsk {
    /// Select…: the Tool Database to pick the tool in, opened on this one.
    Select,
    /// Edit…: the Tool Database to change this tool in.
    Edit,
}

/// What a drag knife form says of its tool, in place of a bit's size and pass: a knife
/// has no flutes; it presses into the top by a share of its spring travel.
pub(crate) const KNIFE_LINE: &str = "Drag knife: zero Z with the blade tip touching the \
                                     material. It presses in by the Pressure, a share of the \
                                     Spring Travel, so no flute length is needed";

/// A toolpath form's tool, kept short: its name (with its tool changer holder), its size and
/// pass in one line, Select… to pick it in the Tool Database (where the tool changer's
/// holders are too, and every setting) and Edit… to change it there. What was asked for.
fn tool_picker(
    ui: &mut egui::Ui,
    id: &str,
    top: &Value,
    tools: &ToolDatabase,
    slots: &[Option<String>],
    units: cut_craft_core::document::Units,
) -> Option<ToolAsk> {
    let tool = &top["tool"];
    let current = tool["name"].as_str().unwrap_or_default();
    let picked = serde_json::from_value::<Tool>(tool.clone())
        .ok()
        .filter(|_| !current.is_empty());
    match &picked {
        Some(tool) => {
            // The Passes: how many the cut depth takes, as set in Edit Passes
            #[expect(clippy::cast_precision_loss, reason = "a small count")]
            let passes = top["depth"]
                .as_f64()
                .filter(|_| tool.pass_depth > 0.0)
                .map(|depth| tool.pass_depths(depth).len() as f64);
            ui.strong(held_name(current, slots));
            if id == cut_craft_core::drag_knife::STRATEGY {
                ui.label(KNIFE_LINE);
            } else {
                ui.label(tool_line(tool, passes, units));
            }
            if surfacing::wants_surfacing_bit(top, tool) {
                ui.weak(surfacing::NOT_A_SURFACING_BIT);
            }
            if tools.get(current).is_none() {
                ui.colored_label(
                    ui.visuals().warn_fg_color,
                    "Not in the Tool Database: Select… one, or add it there",
                );
            }
        }
        None => {
            ui.colored_label(
                ui.visuals().warn_fg_color,
                "No tool picked yet: Select… one",
            );
        }
    }
    ui.horizontal(|ui| {
        let select = ui
            .button("Select…")
            .on_hover_hint(
                "Pick the bit this toolpath cuts with in the Tool Database: it brings its own \
                 feeds, speeds and pass depth",
            )
            .clicked();
        let edit = ui
            .button("Edit…")
            .on_hover_hint("Change this tool in the Tool Database")
            .clicked();
        if select {
            Some(ToolAsk::Select)
        } else {
            edit.then_some(ToolAsk::Edit)
        }
    })
    .inner
}

/// Which tool of a list a pick is for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ListSlot {
    /// One more.
    New,
    /// In place of this one.
    At(usize),
}

/// Whether a toolpath form shows Rest Machining: every toolpath that cuts with a tool can
/// rest machine after the tools that cut before it, except 3D Finishing (its own, as `after`)
/// and a form with clearing bits, which does that with them (an older job's list still
/// shows, so it can be changed).
fn shows_rest(params: &Value) -> bool {
    params.get("after").is_none()
        && (params.get("clearance_tools").is_none()
            || cut_craft_core::rest::Rest::of(params).is_some())
}

/// What a form's list of tools asks for.
#[derive(Clone, Debug, PartialEq, Eq)]
enum ListAsk {
    /// Select Tool, for a tool of the list.
    Select(ListSlot),
    /// The Tool Database to change this tool in.
    Edit(String),
    /// Suggest: the quickest clearing bits from the user's own tools.
    Suggest,
}

/// A form's lists of tools, each picked in Select Tool as the form's own tool is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ToolList {
    /// The clearing bits (`clearance_tools`), largest first.
    Clearing,
    /// The form's tools, in the order they cut (3D Finishing's `tools`).
    Tools,
    /// Rest Machining's earlier tools (`rest.after`).
    Rest,
}

/// What Select Tool picks for, other than the open form's own tool.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum PickFor {
    /// A tool of one of the open form's lists.
    Listed(ToolList, ListSlot),
    /// The active machine's tool changer holder (counted from 1).
    Holder(usize),
}

/// A form's clearing bits (`clearance_tools`): off, the form's tool cuts it all; on, bigger
/// bits cut first, largest first, each picked with Select… in the Tool Database, and the
/// form's tool cuts only what they left. `finishing_smallest` (Pocket) wants each bigger than
/// the form's tool; V-Carve's V-bit follows end mills of any size. Whether the list changed,
/// and what was asked for.
fn clearing_section(
    ui: &mut egui::Ui,
    params: &mut Value,
    slots: &[Option<String>],
    finishing_smallest: bool,
    suggested: Option<&str>,
    units: cut_craft_core::document::Units,
) -> (bool, Option<ListAsk>) {
    if !params["clearance_tools"].is_array() {
        params["clearance_tools"] = Value::Array(vec![]);
    }
    let finishing: Option<Tool> = serde_json::from_value(params["tool"].clone()).ok();
    let mut list: Vec<Tool> =
        serde_json::from_value(params["clearance_tools"].clone()).unwrap_or_default();
    let (mut changed, mut ask) = (false, None);
    ui.group(|ui| {
        ui.set_width(ui.available_width());
        let mut on = !list.is_empty();
        let (label, hint, about) = if finishing_smallest {
            (
                "Clear with bigger bits first",
                "Bigger bits take out most of the material first, so the Tool above only \
                 cuts what they could not reach: quicker, and easier on a small bit",
                "Biggest first. The Tool above finishes: it cuts only what these left.",
            )
        } else {
            (
                "Clear the flat bottom with end mills first",
                "End mills clear the flat bottom first, so the V-bit only carves the slopes \
                 and what they could not reach",
                "Biggest first. The V-bit finishes: it carves only what these left.",
            )
        };
        if ui.checkbox(&mut on, label).on_hover_hint(hint).changed() {
            if on {
                ask = Some(ListAsk::Select(ListSlot::New));
            } else {
                list.clear();
                changed = true;
            }
        }
        // Pocket: the quickest bits from the user's own, tool changer bits first
        if finishing_smallest
            && finishing.is_some()
            && ui
                .button("Suggest clearing bits")
                .on_hover_hint(
                    "Try your own bigger end mills, one or two at a time, and pick the quickest \
                     way to finish this pocket (bits in the tool changer first; Options sets \
                     how long a bit change takes)",
                )
                .clicked()
        {
            ask = Some(ListAsk::Suggest);
        }
        if let Some(note) = suggested {
            ui.weak(note);
        }
        if list.is_empty() {
            return;
        }
        ui.weak(about);
        let depth = params["depth"].as_f64().filter(|d| *d > 0.0);
        let mut remove = None;
        for (i, tool) in list.iter().enumerate() {
            match ui
                .push_id(("clearing", i), |ui| {
                    clearing_bit(ui, i, tool, slots, depth, units)
                })
                .inner
            {
                Some(BitAsk::List(asked)) => ask = Some(asked),
                Some(BitAsk::Remove) => remove = Some(i),
                None => {}
            }
        }
        if let Some(i) = remove {
            list.remove(i);
            changed = true;
        }
        if ui
            .button("Add a smaller clearing bit…")
            .on_hover_hint(
                "Another bit, between the ones above and the finishing bit, for corners too \
                 tight for them",
            )
            .clicked()
        {
            ask = Some(ListAsk::Select(ListSlot::New));
        }
        if let Some(finishing) = &finishing
            && let Err(e) =
                cut_craft_core::rest::check_clearing(finishing, &list, finishing_smallest)
        {
            ui.colored_label(ui.visuals().error_fg_color, e.to_string());
        }
    });
    if changed {
        params["clearance_tools"] = serde_json::to_value(&list).expect("tools serialize");
    }
    (changed, ask)
}

/// What a clearing bit's own buttons ask for.
enum BitAsk {
    List(ListAsk),
    Remove,
}

/// The `i`th clearing bit: its name (with its holder), one line, whether it reaches `depth`,
/// and Select…, Edit… and Remove.
fn clearing_bit(
    ui: &mut egui::Ui,
    i: usize,
    tool: &Tool,
    slots: &[Option<String>],
    depth: Option<f64>,
    units: cut_craft_core::document::Units,
) -> Option<BitAsk> {
    ui.strong(held_name(&tool.name, slots));
    ui.label(tool_line(tool, None, units));
    // can this bit reach the depth? the same line the form's own Tool has
    if let Some(line) = cut_craft_core::reach::form_line(tool, depth, None, units) {
        reach::show(ui, &line);
    }
    ui.horizontal(|ui| {
        if ui
            .button("Select…")
            .on_hover_hint("Pick another bit for this in the Tool Database")
            .clicked()
        {
            return Some(BitAsk::List(ListAsk::Select(ListSlot::At(i))));
        }
        if ui
            .button("Edit…")
            .on_hover_hint("Change this bit in the Tool Database")
            .clicked()
        {
            return Some(BitAsk::List(ListAsk::Edit(tool.name.clone())));
        }
        ui.small_button("×")
            .on_hover_hint("Remove this bit")
            .clicked()
            .then_some(BitAsk::Remove)
    })
    .inner
}

/// `params`' list `which` with `tool` put `at` its slot: clearing bits kept largest first,
/// the others in the order they were picked.
fn put_listed(params: &mut Value, which: ToolList, at: ListSlot, tool: &Tool) {
    let list = match which {
        ToolList::Clearing => &mut params["clearance_tools"],
        ToolList::Tools => &mut params["tools"],
        ToolList::Rest => {
            if !params["rest"].is_object() {
                params["rest"] = serde_json::to_value(cut_craft_core::rest::Rest::default())
                    .expect("rest settings serialise");
            }
            &mut params["rest"]["after"]
        }
    };
    let mut tools: Vec<Tool> = serde_json::from_value(list.clone()).unwrap_or_default();
    match at {
        ListSlot::At(i) if i < tools.len() => tools[i] = tool.clone(),
        _ => tools.push(tool.clone()),
    }
    if which == ToolList::Clearing {
        tools.sort_by(|a, b| b.diameter.total_cmp(&a.diameter));
    }
    *list = serde_json::to_value(&tools).expect("tools serialize");
}

/// The form tool's name as the tool changer has it: `T3  name` when loaded in holder 3.
fn held_name(name: &str, slots: &[Option<String>]) -> String {
    match slots.iter().position(|s| s.as_deref() == Some(name)) {
        Some(i) => format!("T{}  {name}", i + 1),
        None => name.to_owned(),
    }
}

/// A tool in one line for its form: how wide, its angle or tip if it has one, how deep a
/// pass, and how many passes the cut takes (`passes`, when known). Lengths in the job's
/// `units`: a tool keeps its own units in the Tool Database, but a toolpath shows it in the
/// job's.
fn tool_line(tool: &Tool, passes: Option<f64>, units: cut_craft_core::document::Units) -> String {
    use cut_craft_core::toolpath::ToolShape;
    let length = |mm: f64| format!("{} {}", trimmed(mm / units.per()), units.suffix());
    let across = length(tool.diameter);
    let line = match tool.shape {
        ToolShape::EndMill => format!("{across} end mill"),
        ToolShape::BallNose => format!("{across} ball nose"),
        ToolShape::VBit => format!("{}° V-bit, {across} across", trimmed(tool.angle)),
        ToolShape::RadiusedEndMill => {
            format!("{across} end mill, {} tip radius", length(tool.tip_radius))
        }
        ToolShape::ThreadMill => format!("{across} thread mill"),
        ToolShape::FormTool => format!("{across} form tool"),
        ToolShape::SurfacingBit => format!("{across} surfacing bit"),
    };
    let pass = length(tool.pass_depth);
    let line = format!("{line}, {pass} a pass");
    match passes {
        Some(passes) => {
            let s = if (passes - 1.0).abs() < f64::EPSILON {
                ""
            } else {
                "es"
            };
            format!("{line}, {} pass{s}", trimmed(passes))
        }
        None => line,
    }
}

/// A list of tools (a set of finishing tools, rest machining's earlier tools), each as the
/// form's own tool is shown: its name (with its tool changer holder) and size in one line,
/// Select… to pick another in the Tool Database, Edit… to change it there, and × to remove
/// it; Add a tool… picks one more. `empty` is said while there are none. Whether it changed
/// (a removal), and what was asked for.
fn tool_list(
    ui: &mut egui::Ui,
    key: &str,
    list: &mut Value,
    tools: &ToolDatabase,
    slots: &[Option<String>],
    empty: &str,
    units: cut_craft_core::document::Units,
) -> (bool, Option<ListAsk>) {
    let mut items: Vec<Tool> = serde_json::from_value(list.clone()).unwrap_or_default();
    let (mut ask, mut remove) = (None, None);
    if items.is_empty() {
        ui.weak(empty);
    }
    for (i, tool) in items.iter().enumerate() {
        ui.push_id((key, i), |ui| {
            ui.strong(held_name(&tool.name, slots));
            ui.label(tool_line(tool, None, units));
            if tools.get(&tool.name).is_none() {
                ui.colored_label(
                    ui.visuals().warn_fg_color,
                    "Not in the Tool Database: Select… one, or add it there",
                );
            }
            ui.horizontal(|ui| {
                if ui
                    .button("Select…")
                    .on_hover_hint(
                        "Pick another bit for this in the Tool Database: it brings its own \
                         feeds, speeds and pass depth",
                    )
                    .clicked()
                {
                    ask = Some(ListAsk::Select(ListSlot::At(i)));
                }
                if ui
                    .button("Edit…")
                    .on_hover_hint("Change this tool in the Tool Database")
                    .clicked()
                {
                    ask = Some(ListAsk::Edit(tool.name.clone()));
                }
                if ui.small_button("×").on_hover_hint("Remove").clicked() {
                    remove = Some(i);
                }
            });
        });
    }
    if ui
        .push_id((key, "add"), |ui| {
            ui.button("Add a tool…")
                .on_hover_hint("Pick one more in the Tool Database")
        })
        .inner
        .clicked()
    {
        ask = Some(ListAsk::Select(ListSlot::New));
    }
    let changed = remove.is_some();
    if let Some(i) = remove {
        items.remove(i);
        *list = serde_json::to_value(&items).expect("tools serialize");
    }
    (changed, ask)
}

/// A form's own list of tools (3D Finishing's `tools`, in the order they cut), under its
/// heading: see [`tool_list`].
fn tools_section(
    ui: &mut egui::Ui,
    key: &str,
    params: &mut Value,
    tools: &ToolDatabase,
    slots: &[Option<String>],
    units: cut_craft_core::document::Units,
) -> (bool, Option<ListAsk>) {
    ui.group(|ui| {
        ui.set_width(ui.available_width());
        ui.strong("Tools");
        tool_list(
            ui,
            key,
            &mut params[key],
            tools,
            slots,
            "No tool picked yet: Add a tool… to pick one",
            units,
        )
    })
    .inner
}

/// `value` to three places at most, without trailing zeros.
fn trimmed(value: f64) -> String {
    let text = format!("{value:.3}");
    text.trim_end_matches('0').trim_end_matches('.').to_owned()
}

/// The Rest Machining section of a toolpath form: the tools that cut before this one
/// (picked from the database, each removable), and, once there are any, the minimum
/// detail and overlap. Kept as `rest` in the form's settings.
/// A toolpath form's Vector Selector: associated, the toolpath takes the vectors that pass
/// it (open, closed, on which layers) each time it is calculated, as templates need.
fn selector_section(ui: &mut egui::Ui, params: &mut Value, layer_names: &[String], units: Units) {
    use cut_craft_core::selector::{Circles, ClosedKind, Layers, Selector};
    let before = Selector::of(params);
    let mut on = before.is_some();
    let mut selector = before.clone().unwrap_or_default();
    ui.group(|ui| {
        ui.set_width(ui.available_width());
        ui.checkbox(&mut on, "Associate a Vector Selector with this toolpath")
            .on_hover_hint("Take the vectors that pass it each time the toolpath is calculated");
        if !on {
            return;
        }
        ui.checkbox(&mut selector.open_vectors, "Open vectors");
        ui.checkbox(&mut selector.closed_vectors, "Closed vectors");
        // which closed vectors: all, only circles, or only circles of a size
        ui.add_enabled_ui(selector.closed_vectors, |ui| {
            ui.indent("closed_kind", |ui| {
                ui.radio_value(
                    &mut selector.closed_kind,
                    ClosedKind::AllClosedVectors,
                    "All closed vectors",
                );
                ui.radio_value(
                    &mut selector.closed_kind,
                    ClosedKind::OnlyCircles,
                    "Only circles",
                )
                .on_hover_hint("For drilling: only the closed vectors that are circles");
                if selector.closed_kind != ClosedKind::OnlyCircles {
                    return;
                }
                ui.indent("circles", |ui| {
                    ui.radio_value(
                        &mut selector.circles,
                        Circles::SelectAllCircles,
                        "All circles",
                    );
                    ui.radio_value(
                        &mut selector.circles,
                        Circles::SelectCirclesMatching,
                        "Circles of this diameter",
                    );
                    if selector.circles == Circles::SelectCirclesMatching {
                        egui::Grid::new("circle_size")
                            .num_columns(2)
                            .show(ui, |ui| {
                                ui.label("Diameter");
                                tabs::length(ui, &mut selector.diameter, units);
                                ui.end_row();
                                ui.label("Tolerance").on_hover_hint(
                                    "How far a circle's diameter may be from it and still pass",
                                );
                                tabs::length(ui, &mut selector.tolerance, units);
                                ui.end_row();
                            });
                    }
                });
            });
        });
        for (layers, label) in [
            (Layers::AllVisibleLayers, "All visible layers"),
            (Layers::CurrentLayerOnly, "Current layer only"),
            (Layers::SelectedLayers, "Selected layers"),
        ] {
            ui.radio_value(&mut selector.layers, layers, label);
        }
        if selector.layers == Layers::SelectedLayers {
            for name in layer_names {
                let mut ticked = selector.layer_names.contains(name);
                if ui.checkbox(&mut ticked, name).changed() {
                    if ticked {
                        selector.layer_names.push(name.clone());
                    } else {
                        selector.layer_names.retain(|n| n != name);
                    }
                }
            }
        }
    });
    let after = on.then_some(selector);
    if after != before {
        match after {
            Some(selector) => {
                params["selector"] =
                    serde_json::to_value(&selector).expect("selector settings serialise");
            }
            None => {
                if let Some(map) = params.as_object_mut() {
                    map.remove("selector");
                }
            }
        }
    }
}

fn rest_section(
    ui: &mut egui::Ui,
    params: &mut Value,
    tools: &ToolDatabase,
    slots: &[Option<String>],
    units: cut_craft_core::document::Units,
) -> Option<ListAsk> {
    use cut_craft_core::rest::Rest;
    let mut rest: Rest = params
        .get("rest")
        .and_then(|r| serde_json::from_value(r.clone()).ok())
        .unwrap_or_default();
    let before = rest.clone();
    let mut ask = None;
    ui.group(|ui| {
        ui.set_width(ui.available_width());
        ui.strong("Rest Machining")
            .on_hover_hint("Cut only what the tools that ran before this one could not reach");
        let mut after = serde_json::to_value(&rest.after).expect("tools serialize");
        let (changed, asked) = tool_list(
            ui,
            "rest_after",
            &mut after,
            tools,
            slots,
            "Off: add the bigger tools that cut before this one to cut only what they left",
            units,
        );
        ask = asked;
        if changed && let Ok(list) = serde_json::from_value(after) {
            rest.after = list;
        }
        if !rest.after.is_empty() {
            egui::Grid::new("rest_settings")
                .num_columns(2)
                .show(ui, |ui| {
                    ui.label("Minimum detail")
                        .on_hover_hint("Leave alone unmachined areas narrower than this");
                    form::length_edit(ui, &mut rest.minimum_detail, units, 0.0);
                    ui.end_row();
                    ui.label("Overlap")
                        .on_hover_hint("Grow each unmachined area by this");
                    form::length_edit(ui, &mut rest.overlap, units, 0.0);
                    ui.end_row();
                });
        }
    });
    if rest != before {
        params["rest"] = serde_json::to_value(&rest).expect("rest settings serialise");
    }
    ask
}

/// A category of a drop-down in a border filling its width, as menus draw theirs.
fn bordered(ui: &mut egui::Ui, add: impl FnOnce(&mut egui::Ui)) {
    egui::Frame::group(ui.style()).show(ui, |ui| {
        ui.set_min_width(ui.available_width());
        ui.with_layout(Layout::top_down_justified(Align::LEFT), add);
    });
}

fn menu_item(ui: &mut egui::Ui, item: &gui::MenuItem, menus: &Menus, clicked: &mut Option<String>) {
    if item.action == "recent_files" {
        ui.add_enabled_ui(!menus.recent.is_empty(), |ui| {
            ui.menu_button(&item.name, |ui| {
                menu_items(ui, menus.recent, menus, clicked);
            });
        });
    } else if !item.children.is_empty() {
        ui.add_enabled_ui(item.is_enabled, |ui| {
            ui.menu_button(&item.name, |ui| {
                menu_items(ui, &item.children, menus, clicked);
            });
        });
    } else {
        let picture = item.icon.as_deref().and_then(menus.image);
        let button = match picture {
            Some(picture) => {
                let edge = ui.spacing().interact_size.y;
                let picture = picture.fit_to_exact_size(egui::Vec2::splat(edge));
                egui::Button::image_and_text(picture, &item.name)
            }
            None => egui::Button::new(&item.name),
        }
        .shortcut_text(shortcut_label(
            ui.ctx(),
            (menus.shortcut_of)(&item.action).unwrap_or(&item.shortcut),
        ))
        .selected((menus.checked)(&item.action).unwrap_or(item.is_checked));
        let available = (menus.available)(&item.action);
        let mut response = ui.add_enabled(item.is_enabled && available, button);
        if response.hovered()
            && let Some(about) = gui::about(&item.action)
        {
            cut_craft_bug_report::hint::hint(ui.ctx(), about);
        }
        if !available {
            response = response.on_disabled_hover_text("Not implemented yet");
        }
        if response.clicked() {
            *clicked = Some(item.action.clone());
            ui.close();
        }
    }
}

/// Tabs down a side of the window, text reading downwards as the side tabs do; the tab
/// hovered and the tab clicked this frame.
fn side_tabs(ui: &mut egui::Ui, labels: &[&str], current: Option<usize>) -> Vec<egui::Response> {
    let font = egui::TextStyle::Button.resolve(ui.style());
    let pad = ui.spacing().button_padding;
    let mut tabs = vec![];
    for (i, label) in labels.iter().enumerate() {
        let galley = ui.painter().layout_no_wrap(
            (*label).to_owned(),
            font.clone(),
            ui.visuals().text_color(),
        );
        let text = galley.size();
        let size = egui::vec2(ui.available_width(), text.x + 2.0 * pad.x);
        let (rect, response) = ui.allocate_exact_size(size, egui::Sense::click());
        let look = ui
            .style()
            .interact_selectable(&response, current == Some(i));
        ui.painter().rect(
            rect,
            look.corner_radius,
            look.weak_bg_fill,
            look.bg_stroke,
            egui::StrokeKind::Middle,
        );
        // turned a quarter clockwise about its top-left corner, so it reads downwards
        let at = egui::pos2(rect.center().x + text.y / 2.0, rect.top() + pad.x);
        let mut shape = egui::epaint::TextShape::new(at, galley, look.text_color())
            .with_angle(std::f32::consts::FRAC_PI_2);
        shape.override_text_color = Some(look.text_color());
        ui.painter().add(shape);
        tabs.push(response);
    }
    tabs
}

/// A path as the menus and start page list it: just the file name.
fn file_name(path: &std::path::Path) -> String {
    path.file_name().map_or_else(
        || path.display().to_string(),
        |name| name.to_string_lossy().into_owned(),
    )
}

impl eframe::App for CutCraftApp {
    fn ui(&mut self, ui: &mut egui::Ui, frame: &mut eframe::Frame) {
        if let (Some(window), Ok(mut parent)) = (frame.winit_window(), DIALOG_PARENT.lock()) {
            *parent = Some(window.clone());
        }
        let job_folder = (self.options.general.file_dialog_default
            == crate::options::FileDialogDefault::Job)
            .then(|| self.session.path.as_deref()?.parent().map(Into::into))
            .flatten();
        if let Ok(mut folder) = DIALOG_FOLDER.lock() {
            *folder = job_folder;
        }
        self.pass(ui);
    }
}

/// The main window's frame: the desktop's title bar or Cut Craft's own (`system_title_bar`);
/// a Mac's window always keeps its traffic lights ([`mac::viewport`]).
fn window_frame(builder: egui::ViewportBuilder, system_title_bar: bool) -> egui::ViewportBuilder {
    if egui::os::OperatingSystem::from_target_os() == egui::os::OperatingSystem::Mac {
        mac::viewport(builder, system_title_bar)
    } else {
        builder.with_decorations(system_title_bar)
    }
}

/// Opens the main window and runs until it is closed; with `shots`, takes the README's
/// pictures into that folder instead and quits ([`screenshots`]).
pub fn run(title: &str, shots: Option<std::path::PathBuf>) {
    // winit 0.29 delivered dropped files only on X11; 0.30 should on Wayland too, but until that
    // is tried by hand a Wayland desktop runs Cut Craft through XWayland unless Options says to
    // use Wayland directly
    let settings = crate::options::Options::load();
    let (wayland, theme) = (settings.advanced.wayland_native, settings.appearance.theme);
    let frame = settings.appearance.system_title_bar;
    let text_scale = settings.appearance.text_scale();
    let x11 = !wayland && std::env::var_os("DISPLAY").is_some();
    let options = eframe::NativeOptions {
        event_loop_builder: Some(Box::new(move |builder| {
            // only Linux and the BSDs have X11 to choose
            #[cfg(all(unix, not(target_os = "macos")))]
            if x11 {
                use winit::platform::x11::EventLoopBuilderExtX11;
                builder.with_x11();
            }
            #[cfg(not(all(unix, not(target_os = "macos"))))]
            let _ = (builder, x11);
        })),
        viewport: window_frame(
            match shots {
                // the same size each time, whatever window size the user's runs keep
                Some(_) => egui::ViewportBuilder::default().with_inner_size(screenshots::WINDOW),
                None => egui::ViewportBuilder::default(),
            }
            .with_title(title)
            .with_app_id(APP_ID),
            frame,
        ),
        persist_window: shots.is_none(),
        renderer: eframe::Renderer::Wgpu,
        multisampling: settings.view_3d.anti_aliasing.samples(),
        // the 3D view sorts its faces by depth
        depth_buffer: view3d::gpu::DEPTH_BITS,
        wgpu_options: eframe::egui_wgpu::WgpuConfiguration {
            wgpu_setup: eframe::egui_wgpu::WgpuSetup::CreateNew(
                eframe::egui_wgpu::WgpuSetupCreateNew {
                    device_descriptor: std::sync::Arc::new(view3d::gpu::device),
                    // a card that can run the 3D view before OpenGL or software (see `rank`)
                    native_adapter_selector: Some(std::sync::Arc::new(view3d::gpu::choose_adapter)),
                    ..eframe::egui_wgpu::WgpuSetupCreateNew::without_display_handle()
                },
            ),
            ..Default::default()
        },
        ..Default::default()
    };
    let samples = options.multisampling;
    // the pictures' run keeps its egui state apart from the user's
    let name = shots.as_ref().map_or(title, |_| "Cut Craft screenshots");
    eframe::run_native(
        name,
        options,
        Box::new(|cc| {
            cc.egui_ctx.set_theme(theme.preference());
            egui_extras::install_image_loaders(&cc.egui_ctx);
            readable(&cc.egui_ctx, text_scale);
            let mut app = CutCraftApp::new();
            app.title_bar_at_start = frame;
            if let Some(rs) = &cc.wgpu_render_state {
                let info = rs.adapter.get_info();
                log::info!(
                    "GPU: {} ({:?}, driver: {}), buffers up to {} MB",
                    info.name,
                    info.backend,
                    info.driver,
                    rs.device.limits().max_storage_buffer_binding_size / 1_000_000
                );
                let flags = rs.adapter.get_downlevel_capabilities().flags;
                if view3d::gpu::fits(flags, &rs.device.limits()) {
                    app.gpu_3d = Some(view3d::gpu::Gpu::new(rs, samples));
                    // the grid kernels (preview, 3D toolpaths) on the same card
                    cut_craft_core::compute::use_compute(Some(std::sync::Arc::new(
                        crate::compute::GpuCompute::new(rs.device.clone(), rs.queue.clone()),
                    )));
                } else {
                    // OpenGL on an older card: no compute shaders or storage buffers
                    log::warn!(
                        "GPU can't run the 3D view's shaders: drawing it without them, \
                         grid work on the processor"
                    );
                }
            }
            if let Some(dir) = shots {
                // what a fresh clone shows: its icons, the starter tools, nothing of the user's
                app.shots = Some(screenshots::Shots::new(dir));
                crate::startup::window_opened();
                return Ok(Box::new(app));
            }
            app.load_tools();
            app.load_machines();
            app.load_options();
            app.words_path = files::config_dir().map(|d| d.join("words.txt"));
            app.addons = crate::addons::Addons::new(crate::depth::addons_dir());
            app.load_recent();
            app.clipart = crate::clipart::Clipart::load();
            app.recovery_path = files::config_dir().map(|d| d.join("recovery.ccjob"));
            app.open_recovery();
            // a crash last time: offer its crash log as a report
            if let Some(crash) =
                files::config_dir().and_then(|d| cut_craft_bug_report::logs::take_crash(&d))
            {
                app.offer_crash(&cc.egui_ctx, crash);
            }
            app.space_mouse = Some(crate::spacemouse::listen(cc.egui_ctx.clone()));
            app.check_for_updates_at_startup(&cc.egui_ctx);
            // only now: a panic setting up the app (above) still says why in a message box,
            // rather than the window never appearing and nothing said
            crate::startup::window_opened();
            Ok(Box::new(app))
        }),
    )
    .expect("event loop error");
}

/// How the 3D view wraps `surface` round a rotary job's axis, when Auto-Wrapping is `on`: a
/// preview's surface is at 0, the model placed as Material Setup says.
fn wrap_for(
    job: &cut_craft_core::document::Job,
    on: bool,
    previewing: bool,
    surface: &Material,
) -> Option<view3d::Wrap> {
    job.rotary.filter(|_| on).map(|r| view3d::Wrap {
        along_x: r.along == cut_craft_core::document::Along::X,
        radius: r.diameter / 2.0,
        surface: if previewing {
            0.0
        } else {
            let heights = surface.heights.iter().copied();
            let lowest = heights.clone().fold(f64::INFINITY, f64::min);
            let shift = -job.model_shift(lowest, heights.fold(f64::NEG_INFINITY, f64::max));
            // nothing modelled yet (every cell a hole): the surface is the stock's
            if shift.is_finite() { shift } else { 0.0 }
        },
    })
}

/// Whether one closed vector is selected, for a drawing to be imported into (the 2D view's
/// right-click Import Into This Shape…).
pub fn shape_to_import_into(doc: &cut_craft_core::document::Document) -> bool {
    fit_import::chosen_shape(doc).is_ok()
}
