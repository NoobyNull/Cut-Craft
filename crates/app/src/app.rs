//! The main window: menus, Design and Toolpaths panels, and the 2D/3D views over the
//! document, its undo history and the command registry.

mod actions;
mod layouts;
mod panels;
#[path = "screenshots.rs"]
mod screenshots;
#[cfg(test)]
mod tests;
mod views;

use crate::files::{self, Session};
use crate::form::{self, Field};
use crate::gui;
use crate::icons::Icons;
use crate::pages::{Page, Pages, Shown};
use crate::view2d::{self, Canvas, Gesture, Overlay, ViewTransform};
use crate::{commands, shortcut, view3d};
use cut_craft_core::document::{Document, EntityId, LayerId};
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

/// Aspire 12's side tabs: Design holds the drawing and modeling tools.
const DESIGN_TABS: [&str; 5] = ["Design", "Sheets", "Layers", "Components", "Clipart"];
/// The browser's tabs in the Rail and Ribbon layouts: the design tabs' panels and the
/// toolpath list under one row of tabs.
const INSPECTOR_TABS: [&str; 5] = ["Sheets", "Layers", "Components", "Clipart", "Toolpaths"];
/// The ribbon's tabs: the drawing, modeling and toolpath groups.
const RIBBON_TABS: [&str; 3] = ["Design", "Model", "Toolpaths"];
/// Actions the app handles itself rather than through a command.
/// What the 3D view's fine mesh was made for: the model as at this revision, seen so.
#[derive(Clone, Debug, PartialEq)]
struct FineKey {
    revision: u64,
    job: cut_craft_core::document::Job,
    orbit: view3d::Orbit,
    framing: view3d::Framing,
    fit: egui::Rect,
    look: view3d::Look,
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

/// Forms whose model is made as soon as they open and then follows their settings.
const LIVE_FORMS: &[&str] = &["orientate_3d_model"];

const BUILT_INS: [&str; 84] = [
    "find_tool",
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
    "third_party_licenses",
    "about_cut_craft",
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
    "reset_preview",
];
/// The Help windows.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Info {
    About,
    Shortcuts,
    Licenses,
    /// Toolpath Summary: estimated machining times.
    Summary,
}

/// Toolpath Summary's rapid rate (mm/min) and scale factor until the user changes them.
// ponytail: session-only like the other display settings [inf]; Aspire keeps them per machine
const SUMMARY_DEFAULTS: (f64, f64) = (5000.0, 1.0);

/// The libraries Cut Craft is built on and the licences their makers declare.
// ponytail: the direct dependencies, kept by hand; generate from `cargo metadata` if it drifts
const THIRD_PARTY: [(&str, &str); 16] = [
    ("eframe / egui / egui_extras", "MIT or Apache-2.0"),
    ("glam", "MIT or Apache-2.0"),
    ("i_overlay", "MIT or Apache-2.0"),
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

/// 3D view mesh cells along the longer side of the job while the view moves.
// ponytail: fixed until the 3D view has a detail setting
const MESH_CELLS: usize = 250;
/// The most mesh cells along the job's longer side once the 3D view is still: the model's
/// own cells up to this. Built once per view (about 45 ms at 1200, measured in release).
// ponytail: CPU-drawn mesh; a GPU one with a depth buffer could draw every cell, moving too
const FINE_MESH_CELLS: usize = 1200;
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

/// Cut Craft's own mark, built in.
const LOGO: &[u8] = include_bytes!("../../../assets/cut-craft.svg");
/// How long the 3D view stays still before it is drawn finely, seconds.
const SETTLE_SECONDS: f64 = 0.25;
/// How often unsaved work is copied for recovery after a crash, seconds.
const AUTOSAVE_SECONDS: f64 = 30.0;
/// Drawing tools that take the whole path the pointer was dragged along, as `points`.
const STROKE_TOOLS: [&str; 2] = ["draw_freehand", "sculpt"];
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
/// Aspire's layout keys: F11 designing (Design pinned), F12 machining (Toolpaths pinned).
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
/// Aspire's startup tasks: action, label and icon.
const STARTUP_TASKS: [(&str, &str, &str); 3] = [
    ("new", "Create a new file", "file/new.png"),
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
/// Text sizes and control heights are egui's times this, to read easily.
// ponytail: fixed until Options has a text size
const TEXT_SCALE: f32 = 1.15;
/// Lines (widget outlines, the 2D view's vectors) are egui's widths times this.
const LINE_SCALE: f32 = 1.5;
/// Easy-to-read UI fonts, the first installed used; egui's own stay behind for symbols.
const UI_FONTS: [&str; 5] = ["Noto Sans", "Open Sans", "DejaVu Sans", "Segoe UI", "Arial"];
/// A playing preview's speed to start with, in mm of tool travel a second, and its range.
// ponytail: fixed starting speed; the bar's slider changes it
const PLAYBACK_SPEED: f64 = 200.0;
const PLAYBACK_SPEEDS: std::ops::RangeInclusive<f64> = 5.0..=20_000.0;
/// The tool drawn in the 3D view while a preview plays: this many diameters tall.
const TOOL_HEIGHT_DIAMETERS: f64 = 4.0;
/// Beside each recently opened file.
const RECENT_FILE_ICON: &str = "file/new.png";
/// This version's changes, one `- ` line each, shown on the start page.
const WHATS_NEW: &str = include_str!("../whats-new.md");
/// Where Cut Craft is found online, shown on the start page: label, address and icon (the
/// social ones are placeholders: each site's home page).
/// Machine ▸ Search for Machine Online: the machine packages in Cut Craft's repository.
const MACHINES_ONLINE: &str = "https://github.com/NoobyNull/Cut-Craft/tree/main/machines";
const LINKS: [(&str, &str, &str); 7] = [
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

/// What the window shows: Aspire's start page, Job Setup for a new job, or the job.
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

/// Both views at once, as Aspire's top bar splits them.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Split {
    /// 2D above 3D
    Stacked,
    /// 2D left of 3D
    SideBySide,
}

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
    /// while it is the one shape selected (as Aspire's drawing forms do).
    drawn: Option<EntityId>,
    /// Settings changed since the shape it made last followed them.
    dirty: bool,
    /// Orientate 3D Model's Model Size as the form filled it in from the model, while the
    /// user has not typed one: it follows the orientation then.
    auto_size: Option<(f64, f64)>,
    /// The settings the shape it made was last made with.
    applied: Option<Value>,
}

#[expect(
    clippy::struct_excessive_bools,
    reason = "the windows and modes it shows"
)]
struct CutCraftApp {
    screen: Screen,
    doc: Document,
    history: History,
    registry: Registry,
    icons: Icons,
    menus: gui::MenuSystem,
    /// The menus with no job open.
    start_menus: gui::MenuSystem,
    toolbar: gui::Toolbar,
    status: gui::StatusBar,
    design_tab: usize,
    view: View,
    cursor: Option<Point>,
    form: Option<OpenForm>,
    /// How the 2D view mapped the job last frame.
    view_transform: Option<ViewTransform>,
    /// Where the 2D view was on screen last frame.
    view_area: Option<egui::Rect>,
    /// The user's zoom and pan; `None` fits the job to the view.
    camera: Option<ViewTransform>,
    view_tool: Option<ViewTool>,
    /// The Design and Toolpaths pages: pinned open, or hidden to a side tab.
    pages: Pages,
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
    session: Session,
    /// Toolpath picked in the Toolpaths panel.
    selected_toolpath: Option<EntityId>,
    /// The machines (Machine Configuration), the active one's default post saving
    /// toolpaths; without one, Cut Craft's own `FluidNC` post for the job does.
    machines: cut_craft_machines::Machines,
    /// Where `machines` is kept; `None` keeps them in memory (tests).
    machines_path: Option<std::path::PathBuf>,
    /// Cut Craft's posts and the user's custom ones.
    posts: cut_craft_machines::Library,
    /// The open Machine Configuration Management and Post-Processor Management windows.
    machine_window: Option<cut_craft_machines::MachineManager>,
    post_window: Option<cut_craft_machines::PostManager>,
    tools: ToolDatabase,
    /// Where `tools` is saved; `None` keeps it in memory (tests, or a file that failed to load).
    tools_path: Option<std::path::PathBuf>,
    /// The open Tool Database window.
    tool_manager: Option<cut_craft_tool_database::Manager>,
    /// Right-click ▸ Report a Bug Here…
    reporter: cut_craft_bug_report::Reporter,
    /// `--screenshots`: the README's pictures being taken.
    shots: Option<screenshots::Shots>,
    /// What Cut and Copy last took, pasted back in place.
    clipboard: Vec<cut_craft_core::geometry::Path>,
    /// An open Help window.
    info: Option<Info>,
    /// Toolpath Summary's rapid rate (mm/min) and scale factor.
    summary: (f64, f64),
    /// Where the pointer has been during this drag, for stroke tools such as Freehand.
    stroke: Vec<Point>,
    /// Snapping Options (F4) and whether their window is open.
    snap: cut_craft_core::snap::SnapOptions,
    snap_window: bool,
    /// What the views draw.
    display: crate::display::Display,
    /// The material as cut by the previewed toolpaths, and its picture once drawn.
    simulation: Option<Material>,
    /// `simulation` at the 3D view's mesh resolution.
    simulation_coarse: Option<Material>,
    /// The 3D view's fine mesh once it is still, with what it was made for.
    fine_3d: Option<(FineKey, egui::Mesh)>,
    /// When the 3D view last moved (input time, seconds).
    moved_3d: f64,
    orbit: view3d::Orbit,
    framing: view3d::Framing,
    /// Where the 3D view last drew its view cube.
    view_cube: Option<view3d::ViewCube>,
    /// Jobs opened or saved lately, newest first.
    recent: Vec<std::path::PathBuf>,
    /// A preview playing (or paused): the tool cutting the material a distance at a time.
    playback: Option<cut_craft_core::playback::Playback>,
    playing: bool,
    /// How fast the preview plays, in mm of tool travel a second.
    playback_speed: f64,
    /// Component tree levels collapsed (Aspire's Expand / Collapse).
    collapsed_levels: std::collections::HashSet<u64>,
    /// A guide being dragged: whether it is vertical, and which it is (`None` a new one).
    guide_drag: Option<(bool, Option<usize>)>,
    /// Transform mode's corners rotate rather than scale.
    rotating: bool,
    /// Edit ▸ Options, kept between runs, and whether its window is open.
    options: crate::options::Options,
    options_window: bool,
    /// Both views shown at once.
    split: Option<Split>,
    /// The Clipart tab's library.
    clipart: crate::clipart::Clipart,
    /// Where `recent` is saved; `None` keeps it in memory (tests).
    recent_path: Option<std::path::PathBuf>,
    /// Where unsaved work is copied every [`AUTOSAVE_SECONDS`] for the next run to recover;
    /// `None` keeps no copy (tests).
    recovery_path: Option<std::path::PathBuf>,
    /// The history revision and input time (seconds) of the last recovery copy.
    autosaved: (u64, f64),
    /// A recovery copy being written.
    autosave: Option<std::thread::JoinHandle<Result<(), String>>>,
    /// Window title last sent to the viewport.
    title: String,
}

/// A sheet's size in the Sheets List: one decimal in mm, as Aspire's, three in inches.
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

/// Measure between two points: distance, angle from horizontal, and the X and Y distances.
fn measured(from: Point, to: Point, units: cut_craft_core::document::Units) -> String {
    let d = to - from;
    format!(
        "Distance {}, angle {:.2}°, X {}, Y {}",
        length(d.length(), units),
        d.y.atan2(d.x).to_degrees(),
        length(d.x.abs(), units),
        length(d.y.abs(), units)
    )
}

/// The clicked span's type and length, and its vector's perimeter and area.
fn inspected(path: &Path, at: Point, reach: f64, units: cut_craft_core::document::Units) -> String {
    use cut_craft_core::geometry::Segment;
    use std::fmt::Write as _;
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
    let mut text =
        cut_craft_core::nodes::span_near(path, at, reach).map_or_else(String::new, |k| {
            let kind = match path.segments[k] {
                Segment::Line { .. } => "Line",
                Segment::Arc { .. } => "Arc",
                Segment::Cubic { .. } => "Bezier Curve",
            };
            format!("{kind}, length {}; ", length(measure(&span(k)), units))
        });
    let _ = write!(text, "vector perimeter {}", length(measure(path), units));
    if path.closed {
        let area = cut_craft_core::polygon::area(path);
        let _ = match units {
            cut_craft_core::document::Units::Mm => write!(text, ", area {area:.3} mm²"),
            cut_craft_core::document::Units::Inch => {
                write!(text, ", area {:.4} in²", area / 645.16)
            }
        };
    }
    text
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

/// Each toolpath's estimated time, the visible ones' total, and the rate and scale they use.
fn summary_table(
    ui: &mut egui::Ui,
    times: &[(String, f64, bool)],
    (rapid, scale): &mut (f64, f64),
    units: cut_craft_core::document::Units,
) {
    let total: f64 = times.iter().filter(|t| t.2).map(|t| t.1).sum();
    ui.strong(format!("Total machining time: {}", hms(total)));
    ui.separator();
    egui::Grid::new("summary").striped(true).show(ui, |ui| {
        for (name, minutes, visible) in times {
            ui.add_enabled(*visible, egui::Label::new(name));
            ui.label(hms(*minutes));
            ui.end_row();
        }
    });
    ui.separator();
    ui.weak("Time estimates based on the tools' feed rates");
    let fields = [
        crate::form::Field {
            key: "rapid_rate".into(),
            label: "Rapid rate".into(),
            help: Some("The machine's G0 speed".into()),
            kind: crate::form::Kind::Number {
                unit: Some("mm/min".into()),
                min: Some(1.0),
                max: None,
                integer: false,
                steps: vec![],
            },
            section: false,
            icon: None,
            when: None,
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
            },
            section: false,
            icon: None,
            when: None,
        },
    ];
    let mut values = serde_json::json!({ "rapid_rate": *rapid, "scale_factor": *scale });
    let picture = |_: &str| None;
    let look = crate::form::Look {
        units,
        picture: &picture,
        material: cut_craft_core::calc::Values::default(),
    };
    if crate::form::show(ui, &fields, &mut values, &look) {
        *rapid = values["rapid_rate"].as_f64().unwrap_or(*rapid);
        *scale = values["scale_factor"].as_f64().unwrap_or(*scale);
    }
}

use cut_craft_core::file::TEMPLATE_EXTENSION as TEMPLATE;

/// Job templates, starting in Cut Craft's `templates` folder.
fn template_dialog() -> rfd::FileDialog {
    let dialog = file_dialog().add_filter("Cut Craft job template", &[TEMPLATE]);
    match files::config_dir().map(|d| d.join("templates")) {
        Some(dir) if std::fs::create_dir_all(&dir).is_ok() => dialog.set_directory(dir),
        _ => dialog,
    }
}

/// Toolpath templates, in the same `templates` folder.
fn toolpath_template_dialog() -> rfd::FileDialog {
    let dialog =
        file_dialog().add_filter("Toolpath template", &[files::TOOLPATH_TEMPLATE_EXTENSION]);
    match files::config_dir().map(|d| d.join("templates")) {
        Some(dir) if std::fs::create_dir_all(&dir).is_ok() => dialog.set_directory(dir),
        _ => dialog,
    }
}

fn job_dialog() -> rfd::FileDialog {
    file_dialog().add_filter("Cut Craft job", &[JOB_EXTENSION, OLD_JOB_EXTENSION])
}

/// The system's file dialog. Tests must never open one: it lands on the desktop of whoever
/// runs them and waits for an answer.
fn file_dialog() -> rfd::FileDialog {
    on_the_desktop();
    let dialog = rfd::FileDialog::new();
    match DIALOG_FOLDER.lock().ok().and_then(|f| f.clone()) {
        Some(folder) => dialog.set_directory(folder),
        None => dialog,
    }
}

/// Where file dialogs open (Options ▸ File dialog default), kept up to date each frame; the
/// dialogs are opened from places that do not have the app to hand.
static DIALOG_FOLDER: std::sync::Mutex<Option<std::path::PathBuf>> = std::sync::Mutex::new(None);

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
            text(&i.name) + text(shortcut) + room
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

/// An easier-to-read look: a heavier system font, larger text and thicker lines.
fn readable(ctx: &egui::Context) {
    let mut fonts = egui::FontDefinitions::default();
    if let Some((name, data, index)) = cut_craft_core::font::system_face(&UI_FONTS) {
        let mut data = egui::FontData::from_owned(data);
        data.index = index;
        fonts
            .font_data
            .insert(name.clone(), std::sync::Arc::new(data));
        fonts
            .families
            .entry(egui::FontFamily::Proportional)
            .or_default()
            .insert(0, name);
    }
    ctx.set_fonts(fonts);
    ctx.all_styles_mut(|style| {
        for font in style.text_styles.values_mut() {
            font.size *= TEXT_SCALE;
        }
        style.spacing.interact_size *= TEXT_SCALE;
        let v = &mut style.visuals;
        for w in [
            &mut v.widgets.noninteractive,
            &mut v.widgets.inactive,
            &mut v.widgets.hovered,
            &mut v.widgets.active,
            &mut v.widgets.open,
        ] {
            w.fg_stroke.width *= LINE_SCALE;
            w.bg_stroke.width *= LINE_SCALE;
        }
        v.window_stroke.width *= LINE_SCALE;
        v.selection.stroke.width *= LINE_SCALE;
    });
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

/// The Rest Machining section of a toolpath form: the tools that cut before this one
/// (picked from the database, each removable), and, once there are any, the minimum
/// detail and overlap. Kept as `rest` in the form's settings.
fn rest_section(ui: &mut egui::Ui, params: &mut Value, database: &[Tool]) {
    use cut_craft_core::rest::Rest;
    let mut rest: Rest = params
        .get("rest")
        .and_then(|r| serde_json::from_value(r.clone()).ok())
        .unwrap_or_default();
    let before = rest.clone();
    ui.group(|ui| {
        ui.set_width(ui.available_width());
        ui.strong("Rest Machining")
            .on_hover_text("Cut only what the tools that ran before this one could not reach");
        let mut remove = None;
        for (i, tool) in rest.after.iter().enumerate() {
            ui.horizontal(|ui| {
                ui.label(&tool.name);
                if ui.small_button("✕").on_hover_text("Remove").clicked() {
                    remove = Some(i);
                }
            });
        }
        if let Some(i) = remove {
            rest.after.remove(i);
        }
        egui::ComboBox::from_label("Add an earlier tool")
            .selected_text("")
            .show_ui(ui, |ui| {
                for tool in database {
                    if ui.selectable_label(false, &tool.name).clicked() {
                        rest.after.push(tool.clone());
                    }
                }
            });
        if !rest.after.is_empty() {
            egui::Grid::new("rest_settings")
                .num_columns(2)
                .show(ui, |ui| {
                    ui.label("Minimum detail")
                        .on_hover_text("Leave alone unmachined areas narrower than this");
                    ui.add(
                        egui::DragValue::new(&mut rest.minimum_detail)
                            .suffix(" mm")
                            .range(0.0..=f64::INFINITY),
                    );
                    ui.end_row();
                    ui.label("Overlap")
                        .on_hover_text("Grow each unmachined area by this");
                    ui.add(
                        egui::DragValue::new(&mut rest.overlap)
                            .suffix(" mm")
                            .range(0.0..=f64::INFINITY),
                    );
                    ui.end_row();
                });
        }
    });
    if rest != before {
        params["rest"] = serde_json::to_value(&rest).expect("rest settings serialise");
    }
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
        ui.menu_button(&item.name, |ui| {
            menu_items(ui, &item.children, menus, clicked);
        });
    } else {
        let button = egui::Button::new(&item.name)
            .shortcut_text((menus.shortcut_of)(&item.action).unwrap_or(&item.shortcut))
            .selected((menus.checked)(&item.action).unwrap_or(item.is_checked));
        let response = ui
            .add_enabled(item.is_enabled && (menus.available)(&item.action), button)
            .on_disabled_hover_text("Not implemented yet");
        if response.clicked() {
            *clicked = Some(item.action.clone());
            ui.close();
        }
    }
}

/// Tabs down a side of the window, text reading downwards as Aspire's side tabs do; the tab
/// hovered and the tab clicked this frame.
fn side_tabs(
    ui: &mut egui::Ui,
    labels: &[&str],
    current: Option<usize>,
) -> (Option<usize>, Option<usize>) {
    let font = egui::TextStyle::Button.resolve(ui.style());
    let pad = ui.spacing().button_padding;
    let (mut hovered, mut clicked) = (None, None);
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
        if response.hovered() {
            hovered = Some(i);
        }
        if response.clicked() {
            clicked = Some(i);
        }
    }
    (hovered, clicked)
}

/// A path as the menus and start page list it: just the file name.
fn file_name(path: &std::path::Path) -> String {
    path.file_name().map_or_else(
        || path.display().to_string(),
        |name| name.to_string_lossy().into_owned(),
    )
}

impl eframe::App for CutCraftApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
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

/// Opens the main window and runs until it is closed; with `shots`, takes the README's
/// pictures into that folder instead and quits ([`screenshots`]).
pub fn run(title: &str, shots: Option<std::path::PathBuf>) {
    // winit 0.29 delivered dropped files only on X11; 0.30 should on Wayland too, but until that
    // is tried by hand a Wayland desktop runs Cut Craft through XWayland unless Options says to
    // use Wayland directly
    let settings = crate::options::Options::load();
    let wayland = settings.general.wayland_native;
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
        viewport: match shots {
            // the same size each time, whatever window size the user's runs keep
            Some(_) => egui::ViewportBuilder::default().with_inner_size(screenshots::WINDOW),
            None => egui::ViewportBuilder::default(),
        }
        .with_title(title)
        .with_app_id(APP_ID),
        persist_window: shots.is_none(),
        renderer: eframe::Renderer::Wgpu,
        multisampling: settings.view_3d.anti_aliasing.samples(),
        ..Default::default()
    };
    // the pictures' run keeps its egui state apart from the user's
    let name = if shots.is_some() {
        "Cut Craft screenshots"
    } else {
        title
    };
    eframe::run_native(
        name,
        options,
        Box::new(|cc| {
            // light, as Aspire is, whatever the desktop's theme
            cc.egui_ctx.set_theme(egui::ThemePreference::Light);
            egui_extras::install_image_loaders(&cc.egui_ctx);
            readable(&cc.egui_ctx);
            if let Some(rs) = &cc.wgpu_render_state {
                let info = rs.adapter.get_info();
                eprintln!(
                    "GPU: {} ({:?}, driver: {})",
                    info.name, info.backend, info.driver
                );
            }
            let mut app = CutCraftApp::new();
            if let Some(dir) = shots {
                // what a fresh clone shows: its icons, the starter tools, nothing of the user's
                app.shots = Some(screenshots::Shots::new(dir));
                return Ok(Box::new(app));
            }
            app.load_tools();
            app.load_machines();
            app.options = crate::options::Options::load();
            app.options_path = crate::options::Options::path();
            app.options.apply_to(&mut app.display);
            app.load_recent();
            app.clipart = crate::clipart::Clipart::load();
            app.recovery_path = files::config_dir().map(|d| d.join("recovery.ccjob"));
            app.open_recovery();
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
            -job.model_shift(lowest, heights.fold(f64::NEG_INFINITY, f64::max))
        },
    })
}
