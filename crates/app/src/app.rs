//! The main window: menus, Design and Toolpaths panels, and the 2D/3D views over the
//! document, its undo history and the command registry.

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
/// Actions the app handles itself rather than through a command.
const BUILT_INS: [&str; 78] = [
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
        "Offset icon",
        "Creaticca Creative Agency from Freepik (magnific.com), free with this credit",
    ),
];

/// 3D view mesh cells along the longer side of the job.
// ponytail: fixed until the 3D view has a detail setting
const MESH_CELLS: usize = 250;
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
    ("pan", "Pan", "view/pan.svg"),
    ("zoom_box", "Zoom Box", "view/zoom_box.svg"),
    ("zoom_to_job", "Zoom to Job", "view/zoom_to_job.svg"),
    (
        "zoom_selected",
        "Zoom to Selection",
        "view/zoom_selected.svg",
    ),
];
/// Scrolling a view's height divided by this doubles the zoom.
// ponytail: fixed rate until a preferences dialog exists
const VIEW_ZOOM_QUARTERS: f32 = 4.0;
/// Aspire's startup tasks: action, label and icon.
const STARTUP_TASKS: [(&str, &str, &str); 3] = [
    ("new", "Create a new file", "file/new.svg"),
    (
        "new_from_template",
        "New file from template",
        "toolpaths/load_toolpath_template.svg",
    ),
    ("open", "Open an existing file", "file/open.svg"),
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
const RECENT_FILE_ICON: &str = "file/new.svg";
/// This version's changes, one `- ` line each, shown on the start page.
const WHATS_NEW: &str = include_str!("../whats-new.md");
/// Where Cut Craft is found online, shown on the start page: label, address and icon (the
/// social ones are placeholders: each site's home page).
const LINKS: [(&str, &str, &str); 7] = [
    (
        "GitHub",
        "https://github.com/NoobyNull/cut-craft",
        "file/github_link.png",
    ),
    (
        "Facebook",
        "https://www.facebook.com",
        "file/facebook_link.svg",
    ),
    ("X", "https://x.com", "file/twitter_link.svg"),
    (
        "YouTube",
        "https://www.youtube.com",
        "file/youtube_link.svg",
    ),
    (
        "Instagram",
        "https://www.instagram.com",
        "file/instagram_link.svg",
    ),
    (
        "Pinterest",
        "https://www.pinterest.com",
        "file/pinterest_link.svg",
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
const LOGO: &str = "assets/cut-craft.svg";
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

#[derive(PartialEq)]
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
    session: Session,
    /// Toolpath picked in the Toolpaths panel.
    selected_toolpath: Option<EntityId>,
    /// Installed post-processor; without one, toolpaths save through Cut Craft's default.
    post: Option<cut_craft_post::Post>,
    tools: ToolDatabase,
    /// Where `tools` is saved; `None` keeps it in memory (tests, or a file that failed to load).
    tools_path: Option<std::path::PathBuf>,
    tool_window: bool,
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
    /// The tool shown in the Tool Database window.
    tool_selected: usize,
    /// The material as cut by the previewed toolpaths, and its picture once drawn.
    simulation: Option<Material>,
    /// `simulation` at the 3D view's mesh resolution.
    simulation_coarse: Option<Material>,
    orbit: view3d::Orbit,
    framing: view3d::Framing,
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
    /// Window title last sent to the viewport.
    title: String,
}

impl CutCraftApp {
    fn new() -> Self {
        let menus = gui::MenuSystem::new();
        Self {
            screen: Screen::Start,
            start_menus: menus.without_job(),
            doc: Document::default(),
            history: History::default(),
            registry: commands::registry(),
            icons: Icons::load(&Icons::default_dir()),
            menus,
            toolbar: gui::Toolbar::new(),
            status: gui::StatusBar::new(),
            design_tab: 0,
            view: View::TwoD,
            cursor: None,
            form: None,
            view_transform: None,
            view_area: None,
            camera: None,
            view_tool: None,
            pages: Pages::default(),
            session: Session::default(),
            selected_toolpath: None,
            post: None,
            tools: ToolDatabase::starter(),
            tools_path: None,
            tool_window: false,
            clipboard: vec![],
            info: None,
            summary: SUMMARY_DEFAULTS,
            stroke: Vec::new(),
            snap: cut_craft_core::snap::SnapOptions::default(),
            snap_window: false,
            display: crate::display::Display::default(),
            tool_selected: 0,
            simulation: None,
            simulation_coarse: None,
            orbit: view3d::Orbit::default(),
            framing: view3d::Framing::default(),
            recent: vec![],
            playback: None,
            playing: false,
            playback_speed: PLAYBACK_SPEED,
            collapsed_levels: std::collections::HashSet::new(),
            guide_drag: None,
            rotating: false,
            options: crate::options::Options::default(),
            options_window: false,
            split: None,
            clipart: crate::clipart::Clipart::default(),
            recent_path: None,
            title: String::new(),
        }
    }

    fn available(&self, id: &str) -> bool {
        if let Some((_, command, _)) = gui::command_action(id) {
            return self.registry.get(command).is_some();
        }
        match id {
            "undo" => self.history.can_undo(),
            "close" => self.screen != Screen::Start,
            "zoom_selected" => self.doc.selection_bounds().is_some(),
            id if id.starts_with("open_recent:") => true,
            id if id.starts_with(SELECT_LEVEL) => true,
            "add_clipart_folder" => true,
            id if TOP_BAR_TOGGLES.contains(&id) => true,
            id if id.starts_with(EXPORT_LEVEL) => {
                self.registry.exporter("export_as_3d_clipart").is_some()
            }
            "redo" => self.history.can_redo(),
            "recalculate_toolpath"
            | "delete_toolpath"
            | "preview_toolpath"
            | "save_selected_toolpath_as_template"
            | "edit_toolpath" => self.selected_toolpath.is_some(),
            "edit_text" => self.doc.selection.iter().any(|id| {
                self.doc
                    .vector_groups
                    .get(id)
                    .is_some_and(|g| self.doc.texts.contains_key(g))
            }),
            "save_all_visible_toolpaths_as_template" => {
                self.doc.toolpaths.values().any(|t| t.visible)
            }
            "recalculate_all_toolpaths" | "preview_all_toolpaths" | "save_toolpaths" => {
                !self.doc.toolpaths.is_empty()
            }
            "recalculate_visible_toolpaths" => self.doc.toolpaths.values().any(|t| t.visible),
            "reset_preview" => self.simulation.is_some() || self.playback.is_some(),
            id if self.display.checked(id).is_some() => true,
            "cut" | "copy" => !self.doc.selection.is_empty(),
            "create_component_from_toolpath_preview" => self.simulation.is_some(),
            "paste" | "paste_in_place" => !self.clipboard.is_empty(),
            "increment_and_save" => self.screen == Screen::Job,
            _ => {
                BUILT_INS.contains(&id)
                    || self.registry.get(id).is_some()
                    || self.registry.exporter(id).is_some()
            }
        }
    }

    /// Menu items, tool buttons and shortcuts all end up here: built-in actions first, then
    /// the command registry.
    #[expect(clippy::too_many_lines, reason = "the one dispatch of every action")]
    fn invoke(&mut self, ctx: &egui::Context, id: &str) {
        // right-click menu items: a command with params chosen by the menu
        if let Some((form, command, params)) = gui::command_action(id) {
            if form {
                self.start_command(command);
                if let Some(open) = self.form.as_mut().filter(|f| f.id == command) {
                    open.params = params;
                }
            } else if let Err(e) = self.apply(command, &params) {
                self.status.set_message(&e);
            }
            return;
        }
        match id {
            "undo" => {
                if self.history.undo(&mut self.doc) {
                    self.status.set_message("Undone");
                }
            }
            "redo" => {
                if self.history.redo(&mut self.doc) {
                    self.status.set_message("Redone");
                }
            }
            // closing goes through the close request, which checks for unsaved work
            "exit" => ctx.send_viewport_cmd(egui::ViewportCommand::Close),
            "new"
            | "open"
            | "close"
            | "save"
            | "save_as"
            | "import_vectors_bitmap"
            | "import_component_3d_model"
            | "segment_3d_model_file"
            | "install_post_processor"
            | "add_clipart_folder" => self.file_action(id),
            "save_as_template"
            | "new_from_template"
            | "save_selected_toolpath_as_template"
            | "save_all_visible_toolpaths_as_template"
            | "load_template" => self.template_action(id),
            id if TOP_BAR_TOGGLES.contains(&id) => self.top_bar_toggle(id),
            // Aspire's four Create Shape icons: the form with that profile chosen
            "create_shape_round"
            | "create_shape_angled"
            | "create_shape_smooth"
            | "create_shape_flat" => {
                let profile = match id {
                    "create_shape_angled" => "angular",
                    _ => &id["create_shape_".len()..],
                };
                self.start_command("create_shape");
                if let Some(form) = self.form.as_mut().filter(|f| f.id == "create_shape") {
                    form.params["profile"] = Value::from(profile);
                }
            }
            // the Align Objects group's spacing buttons, by pick order
            "left_to_right" | "right_to_left" | "top_to_bottom" | "bottom_to_top" => {
                let command = if id.contains("left") {
                    "space_horizontally"
                } else {
                    "space_vertically"
                };
                if let Err(e) = self.apply(command, &serde_json::json!({ "order": id })) {
                    self.status.set_message(&e);
                }
            }
            "edit_toolpath" => self.edit_toolpath(),
            // Edit Text: Draw Text opens on the selected text's settings
            "edit_text" => self.start_command("draw_text"),
            // Aspire's three Join icons: Join / Close Vectors their way
            "join_straight_line" | "join_smooth_curve" | "join_move_end_points" => {
                let method = id.trim_start_matches("join_");
                let params = serde_json::json!({ "method": method });
                if let Err(e) = self.apply("join_close_vectors", &params) {
                    self.status.set_message(&e);
                }
            }
            "import_vectors" | "export_vectors" | "open_folder" | "open_clipart_folder" => {
                self.file_icon_action(id);
            }
            // the Components tab, as Aspire's Component Levels icon shows
            "component_levels" => {
                self.design_tab = DESIGN_TABS
                    .iter()
                    .position(|&t| t == "Components")
                    .unwrap_or_default();
                self.pages.open(Page::Design);
            }
            "delete_all_guides" => self.set_guides(vec![]),
            "lock_all_existing_guides" | "unlock_all_existing_guides" => {
                let locked = id.starts_with("lock");
                let guides = self
                    .doc
                    .guides
                    .iter()
                    .map(|g| cut_craft_core::document::Guide { locked, ..*g })
                    .collect();
                self.set_guides(guides);
            }
            "tool_database" => self.tool_window = true,
            id if self.display.toggle(id) => ctx.request_repaint(),
            "refresh_2d_view" => ctx.request_repaint(),
            "create_component_from_toolpath_preview" => self.component_from_preview(),
            "copy"
            | "cut"
            | "paste"
            | "paste_in_place"
            | "increment_and_save"
            | "open_application_data_folder"
            | "print"
            | "print_preview" => {
                self.clipboard_or_file_action(id);
            }
            "keyboard_shortcuts"
            | "third_party_licenses"
            | "toolpath_summary"
            | "about_cut_craft"
            | "snap_options" => self.open_window(id),
            "options" => self.options_window = true,
            // the Select tool: put away whatever tool is in hand
            "select" => {
                (self.form, self.view_tool) = (None, None);
                "Select".clone_into(&mut self.status.current_tool);
            }
            // menu items that are an existing tool reached another way
            "select_all_vectors_on_current_layer" => {
                let layer = serde_json::json!({ "layer": self.doc.current_layer });
                if let Err(e) = self.apply("select_layer_vectors", &layer) {
                    self.status.set_message(&e);
                }
            }
            "job_size_and_position" => self.start_command("job_setup"),
            "show_toolpaths_tab" => self.pages.open(Page::Toolpaths),
            "design_layout" => self.pages = Pages::design_layout(),
            "machining_layout" => self.pages = Pages::machining_layout(),
            "interface_layout" => self.pages.switch_layout(),
            "pan" | "zoom_box" | "zoom_to_job" | "zoom_to_drawing" | "zoom_selected"
            | "node_edit" | "measure_inspect" | "transform" => {
                self.view_action(ctx, id);
            }
            "recalculate_toolpath"
            | "recalculate_visible_toolpaths"
            | "recalculate_all_toolpaths"
            | "delete_toolpath"
            | "save_toolpaths" => {
                self.toolpath_action(id);
            }
            "preview_toolpath" => {
                if let Some(id) = self.selected_toolpath {
                    self.preview_toolpath(id);
                }
            }
            "preview_all_toolpaths" => self.preview_all(),
            "reset_preview" => {
                (self.simulation, self.simulation_coarse) = (None, None);
                (self.playback, self.playing) = (None, false);
            }
            id if id.starts_with("open_recent:") => {
                if let Ok(index) = id["open_recent:".len()..].parse() {
                    self.open_recent(index);
                }
            }
            id if id.starts_with(SELECT_LEVEL) || id.starts_with(EXPORT_LEVEL) => {
                self.level_action(id);
            }
            id if self.registry.exporter(id).is_some() => self.export(id),
            _ => self.start_command(id),
        }
    }

    fn file_action(&mut self, id: &str) {
        match id {
            // a new job starts in Job Setup, as Aspire's does
            "new" => {
                if self.confirm_discard() {
                    self.reset_job();
                    // the material being set up shows in the 2D view
                    self.view = View::TwoD;
                    self.start_command("job_setup");
                    if let Some(form) = &mut self.form {
                        form.new_job = true;
                    }
                    self.screen = Screen::NewJob;
                }
            }
            "open" => {
                if self.confirm_discard()
                    && let Some(path) = job_dialog().pick_file()
                {
                    self.open_path(&path);
                }
            }
            "close" => {
                if self.confirm_discard() {
                    self.reset_job();
                    self.screen = Screen::Start;
                }
            }
            "save" => match self.session.path.clone() {
                Some(path) => self.save_to(&path),
                None => self.file_action("save_as"),
            },
            "save_as" => {
                if let Some(path) = job_dialog()
                    .set_file_name(format!("untitled.{JOB_EXTENSION}"))
                    .save_file()
                {
                    self.save_to(&path.with_extension(JOB_EXTENSION));
                }
            }
            "import_vectors_bitmap" | "import_component_3d_model" => {
                let mut dialog = file_dialog();
                let models = id == "import_component_3d_model";
                for importer in self.registry.importers() {
                    if MODEL_IMPORTERS.contains(&importer.id) == models {
                        dialog =
                            dialog.add_filter(importer.format.label, importer.format.extensions);
                    }
                }
                if let Some(path) = dialog.pick_file() {
                    self.import_file(&path);
                }
            }
            "add_clipart_folder" => {
                if let Some(folder) = file_dialog().pick_folder()
                    && let Err(e) = self.clipart.add(folder)
                {
                    self.status.set_message(&e);
                }
            }
            "segment_3d_model_file" => {
                // the segmenting form, on an STL picked first
                if let Some(path) = file_dialog().add_filter("STL", &["stl"]).pick_file() {
                    self.start_command("segment_3d_model");
                    if let Some(form) = self.form.as_mut().filter(|f| f.id == "segment_3d_model") {
                        form.params["file"] = Value::from(path.display().to_string());
                    }
                }
            }
            "install_post_processor" => {
                if let Some(path) = file_dialog()
                    .add_filter("Post-processor", &["pp"])
                    .pick_file()
                {
                    match files::load_post(&path) {
                        Ok(post) => {
                            self.status
                                .set_message(&format!("Post-processor: {}", post.name));
                            self.post = Some(post);
                        }
                        Err(e) => self.status.set_message(&e),
                    }
                }
            }
            _ => unreachable!("not a file action: {id}"),
        }
    }

    /// Job and toolpath templates, saved and loaded through the templates folder.
    fn template_action(&mut self, id: &str) {
        match id {
            "save_as_template" => {
                if let Some(path) = template_dialog()
                    .set_file_name(format!("template.{TEMPLATE}"))
                    .save_file()
                {
                    let path = path.with_extension(TEMPLATE);
                    match std::fs::write(&path, cut_craft_core::file::save_job(&self.doc)) {
                        Ok(()) => self
                            .status
                            .set_message(&format!("Saved template {}", path.display())),
                        Err(e) => self
                            .status
                            .set_message(&format!("could not save {}: {e}", path.display())),
                    }
                }
            }
            "save_selected_toolpath_as_template" | "save_all_visible_toolpaths_as_template" => {
                let ids: Vec<_> = if id == "save_selected_toolpath_as_template" {
                    self.selected_toolpath.into_iter().collect()
                } else {
                    self.doc
                        .toolpaths
                        .iter()
                        .filter(|(_, t)| t.visible)
                        .map(|(&i, _)| i)
                        .collect()
                };
                if let Some(path) = toolpath_template_dialog()
                    .set_file_name(format!("toolpaths.{}", files::TOOLPATH_TEMPLATE_EXTENSION))
                    .save_file()
                {
                    let path = path.with_extension(files::TOOLPATH_TEMPLATE_EXTENSION);
                    let message =
                        match std::fs::write(&path, files::toolpath_template(&self.doc, &ids)) {
                            Ok(()) => format!("Saved toolpath template {}", path.display()),
                            Err(e) => format!("could not save {}: {e}", path.display()),
                        };
                    self.status.set_message(&message);
                }
            }
            "load_template" => {
                if let Some(path) = toolpath_template_dialog().pick_file() {
                    let change = std::fs::read(&path)
                        .map_err(|e| e.to_string())
                        .and_then(|bytes| files::load_toolpath_template(&self.doc, &bytes))
                        .map_err(cut_craft_core::CommandError::Failed);
                    self.record(change, &format!("Loaded {}", path.display()));
                }
            }
            "new_from_template" => {
                // like New and Open: unsaved work is asked about first
                if self.confirm_discard()
                    && let Some(path) = template_dialog().pick_file()
                {
                    self.open_path(&path);
                }
            }
            _ => unreachable!("not a template action: {id}"),
        }
    }

    fn toolpath_action(&mut self, id: &str) {
        match id {
            "recalculate_toolpath" => {
                if let Some(id) = self.selected_toolpath {
                    let result = self.registry.recalculate(&self.doc, id);
                    self.record(result, "Toolpath recalculated");
                }
            }
            // the cascade: Recalculate on each shown toolpath
            "recalculate_visible_toolpaths" => {
                let shown = self.doc.toolpaths.iter().filter(|(_, t)| t.visible);
                let ids: Vec<EntityId> = shown.map(|(&id, _)| id).collect();
                let result =
                    Change::chain_over(&self.doc, &ids, |d, &id| self.registry.recalculate(d, id));
                self.record(result, "Visible toolpaths recalculated");
            }
            "recalculate_all_toolpaths" => {
                let result = self.registry.recalculate_all(&self.doc);
                self.record(result, "All toolpaths recalculated");
            }
            // the toolpath list's button: the Delete Toolpath command on the selected one
            "delete_toolpath" => {
                if let Some(id) = self.selected_toolpath.take()
                    && let Err(e) =
                        self.apply("delete_toolpath", &serde_json::json!({ "toolpath": id }))
                {
                    self.status.set_message(&e);
                }
            }
            "save_toolpaths" => {
                let post = self.output_post();
                let extension = post.extension.clone();
                let dialog = file_dialog().add_filter(&post.name, &[extension.as_str()]);
                if let Some(path) = dialog.save_file() {
                    let path = path.with_extension(&extension);
                    let job = self.session.path.as_deref();
                    let result = match self.doc.tiling {
                        // one file per tile, each tile moved to the job's corner
                        Some(tiling) => {
                            let tiles = tiling.tiles(self.doc.job.area());
                            tiles.iter().enumerate().try_for_each(|(n, &tile)| {
                                let stem = path.file_stem().unwrap_or_default().to_string_lossy();
                                let named = path
                                    .with_file_name(format!("{stem}_tile{}.{extension}", n + 1));
                                files::save_through(
                                    &post,
                                    &tiling.tiled(&self.doc, tile),
                                    &named,
                                    job,
                                )
                            })
                        }
                        None => files::save_through(&post, &self.doc, &path, job),
                    };
                    self.status.set_message(
                        &result.map_or_else(
                            |e| e,
                            |()| format!("Saved toolpaths to {}", path.display()),
                        ),
                    );
                }
            }
            _ => unreachable!("not a toolpath action: {id}"),
        }
    }

    /// The post toolpaths are saved through: the installed one, else Cut Craft's `FluidNC` post
    /// for this kind of job.
    fn output_post(&self) -> Cow<'_, cut_craft_post::Post> {
        self.post.as_ref().map_or_else(
            || Cow::Owned(cut_craft_post::Post::default_for(&self.doc.job)),
            Cow::Borrowed,
        )
    }

    /// Runs a registered command: straight away if it has no settings, else opens its form.
    fn start_command(&mut self, id: &str) {
        let Some(cmd) = self.registry.get(id) else {
            self.status
                .set_message(&format!("{}: not implemented yet", gui::title_case(id)));
            return;
        };
        let page = self.page_of(cmd.id());
        let (meta, fields, params) = (
            cmd.meta(),
            form::fields(&cmd.schema()),
            cmd.initial_params(&self.doc),
        );
        meta.label.clone_into(&mut self.status.current_tool);
        self.view_tool = None;
        if fields.is_empty() {
            if let Err(e) = self.apply(cmd.id(), &params) {
                self.status.set_message(&e);
            }
        } else {
            self.form = Some(OpenForm {
                id: cmd.id(),
                label: meta.label,
                icon: meta.icon,
                fields,
                params,
                error: None,
                new_job: false,
                page,
                editing: None,
            });
            // a form shows in its page, so bring an auto-hidden page out
            if self.pages.shown(page) == Shown::Hidden {
                self.pages.open(page);
            }
        }
    }

    fn save_to(&mut self, path: &std::path::Path) {
        match self.session.save(&self.doc, &self.history, path) {
            Ok(()) => {
                self.status
                    .set_message(&format!("Saved {}", path.display()));
                self.remember(path);
            }
            Err(e) => self.status.set_message(&e),
        }
    }

    /// Opens a job file in place of whatever is open (already confirmed); whether it opened.
    fn open_path(&mut self, path: &std::path::Path) -> bool {
        let mut session = Session::default();
        match session.open(path) {
            Ok((doc, history)) => {
                self.reset_job();
                let template = path
                    .extension()
                    .is_some_and(|e| e == cut_craft_core::file::TEMPLATE_EXTENSION);
                if template {
                    // a template opens as a new file: saving asks where, the template stays
                    session.path = None;
                }
                (self.doc, self.history, self.session) = (doc, history, session);
                self.screen = Screen::Job;
                if !template {
                    self.remember(path);
                }
                if self.doc.notes.starts_with('.') {
                    self.start_command("notes");
                }
                true
            }
            Err(e) => {
                self.status.set_message(&e);
                false
            }
        }
    }

    /// Opens one of the recent files; one that no longer opens leaves the list.
    fn open_recent(&mut self, index: usize) {
        let Some(path) = self.recent.get(index).cloned() else {
            return;
        };
        if self.confirm_discard() && !self.open_path(&path) {
            self.recent.retain(|p| p != &path);
            self.save_recent();
        }
    }

    fn remember(&mut self, path: &std::path::Path) {
        files::remember(&mut self.recent, path, self.options.recent_count());
        self.save_recent();
    }

    fn save_recent(&mut self) {
        if let Some(path) = &self.recent_path
            && let Err(e) = files::save_recent(&self.recent, path)
        {
            self.status.set_message(&e);
        }
    }

    fn load_recent(&mut self) {
        self.recent_path = files::recent_files_path();
        if let Some(path) = &self.recent_path {
            self.recent = files::load_recent(path);
        }
    }

    /// Back to nothing: no job, no form, no preview.
    fn reset_job(&mut self) {
        (self.doc, self.history, self.session, self.form) = Default::default();
        (self.camera, self.view_tool) = (None, None);
        (
            self.selected_toolpath,
            self.simulation,
            self.simulation_coarse,
        ) = (None, None, None);
    }

    /// OK in Job Setup for a new job: the job is made, with nothing to undo yet.
    fn job_setup_ok(&mut self) {
        let Some(mut form) = self.form.take() else {
            return;
        };
        let cmd = self
            .registry
            .get(form.id)
            .expect("only registered commands have forms");
        let made = cmd
            .run_json(&self.doc, &form.params)
            .map_err(|e| e.to_string())
            .and_then(|change| self.doc.apply(&change).map_err(|e| e.to_string()));
        match made {
            Ok(()) => {
                (self.history, self.session) = Default::default();
                self.screen = Screen::Job;
                self.status.set_message("New job created");
                "Select".clone_into(&mut self.status.current_tool);
            }
            Err(e) => {
                form.error = Some(e);
                self.form = Some(form);
            }
        }
    }

    fn job_setup_cancel(&mut self) {
        self.reset_job();
        "Select".clone_into(&mut self.status.current_tool);
        self.screen = Screen::Start;
    }

    fn export(&mut self, id: &str) {
        let Some(format) = self.registry.exporter(id).map(|e| e.format) else {
            return;
        };
        let dialog = file_dialog().add_filter(format.label, format.extensions);
        if let Some(path) = dialog.save_file() {
            let path = path.with_extension(format.extensions[0]);
            let result = files::export(&self.registry, id, &self.doc, &path);
            self.status.set_message(
                &result.map_or_else(|e| e, |()| format!("Exported {}", path.display())),
            );
        }
    }

    /// True when it is fine to throw the current job away: no unsaved work, or the user chose
    /// to save it or not to. False if they cancelled.
    fn confirm_discard(&mut self) -> bool {
        if !self.session.is_dirty(&self.history) {
            return true;
        }
        on_the_desktop();
        let answer = rfd::MessageDialog::new()
            .set_title(APP_NAME)
            .set_description(format!(
                "Save changes to {}?",
                self.session.title(APP_NAME, &self.history)
            ))
            .set_buttons(rfd::MessageButtons::YesNoCancel)
            .show();
        match answer {
            rfd::MessageDialogResult::Yes => {
                match self
                    .session
                    .path
                    .clone()
                    .or_else(|| job_dialog().save_file())
                {
                    Some(path) => self.save_to(&path.with_extension(JOB_EXTENSION)),
                    None => return false,
                }
                !self.session.is_dirty(&self.history)
            }
            rfd::MessageDialogResult::No => {
                // asked once: a repeated close request must not ask again
                self.session.discard_changes(&self.history);
                true
            }
            _ => false,
        }
    }

    /// Applies a computed change for undo, reporting how it went in the status bar.
    fn record(&mut self, change: Result<Change, cut_craft_core::CommandError>, done: &str) {
        let mut made = vec![];
        let result = change.map_err(|e| e.to_string()).and_then(|c| {
            made = c.toolpaths_made();
            self.history
                .apply(&mut self.doc, c)
                .map_err(|e| e.to_string())
        });
        self.status
            .set_message(&result.map_or_else(|e| e, |()| done.to_owned()));
        self.warn_of_keep_out(&made);
    }

    /// A warning in the status bar for each toolpath in `made` that passes over a keep-out
    /// zone.
    fn warn_of_keep_out(&mut self, made: &[EntityId]) {
        let over: Vec<&str> = made
            .iter()
            .filter_map(|id| self.doc.toolpaths.get(id))
            .filter(|t| cut_craft_core::keep_out::crossed(&self.doc.keep_out, &t.moves))
            .map(|t| t.name.as_str())
            .collect();
        if !over.is_empty() {
            let message = format!("Warning: {} passes over a keep-out zone", over.join(", "));
            self.status.set_message(&message);
        }
    }

    /// Aspire's icons over the toolpath list: edit and hide the selected toolpath, and save
    /// it, or every visible one, as a template.
    fn toolpath_list_buttons(&mut self, ui: &mut egui::Ui) {
        let edge = ui.spacing().interact_size.y * TOOL_ICON_ROWS;
        let mut clicked = None;
        ui.horizontal(|ui| {
            for (id, icon, tip) in [
                (
                    "edit_toolpath",
                    "toolpaths/edit_toolpath.svg",
                    "Edit Toolpath",
                ),
                (
                    "hide_toolpath",
                    "toolpaths/hide_toolpaths.svg",
                    "Show or Hide Toolpath",
                ),
                (
                    "save_selected_toolpath_as_template",
                    "toolpaths/save_toolpath_template.svg",
                    "Save Toolpath as Template",
                ),
                (
                    "save_all_visible_toolpaths_as_template",
                    "toolpaths/save_all_as_template.svg",
                    "Save All Visible Toolpaths as Template",
                ),
            ] {
                let available =
                    id == "hide_toolpath" && self.selected_toolpath.is_some() || self.available(id);
                let button = self.icon_button(icon, "", tip, edge);
                if ui
                    .add_enabled(available, button)
                    .on_hover_text(tip)
                    .clicked()
                {
                    clicked = Some(id);
                }
            }
        });
        match clicked {
            Some("hide_toolpath") => {
                if let Some((id, t)) = self
                    .selected_toolpath
                    .and_then(|id| Some((id, self.doc.toolpaths.get(&id)?)))
                {
                    let params = serde_json::json!({ "toolpath": id, "name": t.name, "visible": !t.visible });
                    if let Err(e) = self.apply("toolpath_properties", &params) {
                        self.status.set_message(&e);
                    }
                }
            }
            Some(id) => self.invoke(ui.ctx(), id),
            None => {}
        }
    }

    /// The file icons Aspire has beside its menus: Import Vectors and Export Vectors (vector
    /// formats only), Open Folder (the job's folder) and Open Clipart Folder.
    fn file_icon_action(&mut self, id: &str) {
        let result = match id {
            "import_vectors" => {
                let mut dialog = file_dialog();
                for importer in self.registry.importers() {
                    if VECTOR_IMPORTERS.contains(&importer.id) {
                        dialog =
                            dialog.add_filter(importer.format.label, importer.format.extensions);
                    }
                }
                if let Some(path) = dialog.pick_file() {
                    self.import_file(&path);
                }
                Ok(())
            }
            "export_vectors" => {
                let mut dialog = file_dialog();
                let exporters: Vec<_> = self
                    .registry
                    .exporters()
                    .filter(|e| VECTOR_EXPORTERS.contains(&e.id))
                    .map(|e| (e.id, e.format))
                    .collect();
                for (_, format) in &exporters {
                    dialog = dialog.add_filter(format.label, format.extensions);
                }
                match dialog.save_file() {
                    // the extension picks the format; DXF when there is none
                    Some(path) => {
                        let ext = path.extension().map(|e| e.to_string_lossy().to_lowercase());
                        let (id, path) = exporters
                            .iter()
                            .find(|(_, f)| {
                                ext.as_deref().is_some_and(|x| f.extensions.contains(&x))
                            })
                            .map_or(("export_dxf", path.with_extension("dxf")), |(id, _)| {
                                (*id, path.clone())
                            });
                        files::export(&self.registry, id, &self.doc, &path).map(|()| {
                            self.status
                                .set_message(&format!("Exported {}", path.display()));
                        })
                    }
                    None => Ok(()),
                }
            }
            "open_folder" => {
                let folder = self
                    .session
                    .path
                    .as_deref()
                    .and_then(std::path::Path::parent)
                    .map(std::path::Path::to_path_buf)
                    .or_else(files::config_dir);
                folder.map_or(Err("no folder to open".to_owned()), |f| {
                    on_the_desktop();
                    files::show_folder(&f).map_err(|e| e.to_string())
                })
            }
            _ => match self
                .clipart
                .chosen
                .or((!self.clipart.folders.is_empty()).then_some(0))
                .and_then(|i| self.clipart.folders.get(i))
            {
                Some(folder) => {
                    on_the_desktop();
                    files::show_folder(folder).map_err(|e| e.to_string())
                }
                None => Err("add a folder to the Clipart library first".to_owned()),
            },
        };
        if let Err(e) = result {
            self.status.set_message(&e);
        }
    }

    /// Edit Toolpath: the selected toolpath's form, filled in with its settings.
    fn edit_toolpath(&mut self) {
        let Some((id, toolpath)) = self
            .selected_toolpath
            .and_then(|id| Some((id, self.doc.toolpaths.get(&id)?.clone())))
        else {
            return;
        };
        self.start_command(&toolpath.strategy);
        if let Some(form) = self.form.as_mut().filter(|f| f.id == toolpath.strategy) {
            form.params = toolpath.params;
            form.editing = Some(id);
        }
    }

    /// A form's Apply: its command, or for an edited toolpath that toolpath made again with
    /// the form's settings, in its place.
    fn apply_form(&mut self, id: &str, params: &Value) -> Result<(), String> {
        match self.form.as_ref().and_then(|f| f.editing) {
            Some(toolpath) => {
                let change = self.registry.edit(&self.doc, toolpath, Some(params));
                let failed = change.as_ref().err().map(ToString::to_string);
                self.record(change, "Toolpath edited");
                failed.map_or(Ok(()), Err)
            }
            None => self.apply(id, params),
        }
    }

    /// Runs a command and records it for undo.
    fn apply(&mut self, id: &str, params: &Value) -> Result<(), String> {
        let cmd = self
            .registry
            .get(id)
            .expect("only registered commands are applied");
        let change = cmd.run_json(&self.doc, params).map_err(|e| e.to_string())?;
        // rest machining, for any toolpath whose settings ask for it
        let change = cut_craft_core::rest::with_rest(&self.doc.job, params, change)
            .map_err(|e| e.to_string())?;
        let made = change.toolpaths_made();
        self.history
            .apply(&mut self.doc, change)
            .map_err(|e| e.to_string())?;
        self.status
            .set_message(&format!("{} applied", cmd.meta().label));
        // the spiral gadget says how many times round each strand goes, as Aspire's does
        if id == "wrapped_spiral_layout"
            && let Ok(p) = serde_json::from_value(params.clone())
            && let Some(turns) = cmd_wrapped_spiral_layout::revolutions(&self.doc, &p)
        {
            self.status
                .set_message(&format!("Each strand makes {turns:.2} revolutions"));
        }
        self.warn_of_keep_out(&made);
        Ok(())
    }

    /// The arrow keys nudge the selection by the snapping grid's squares: a whole one, half
    /// with Shift, a tenth with Ctrl + Shift (Matthew's steps; Aspire nudges with Ctrl + Shift).
    fn nudge(&mut self, ctx: &egui::Context) {
        if self.screen != Screen::Job || ctx.wants_keyboard_input() {
            return;
        }
        let (shift, ctrl) = ctx.input(|i| (i.modifiers.shift, i.modifiers.command));
        let share = match (shift, ctrl) {
            (true, true) => 0.1,
            (true, false) => 0.5,
            _ => 1.0,
        };
        let step = self.snap.grid_spacing * share;
        let by = ctx.input(|i| {
            [
                (Key::ArrowLeft, -1.0, 0.0),
                (Key::ArrowRight, 1.0, 0.0),
                (Key::ArrowDown, 0.0, -1.0),
                (Key::ArrowUp, 0.0, 1.0),
            ]
            .iter()
            .filter(|(key, ..)| i.key_pressed(*key))
            .fold(Point::ZERO, |sum, &(_, x, y)| sum + Point::new(x, y) * step)
        });
        if by == Point::ZERO || self.doc.selection_bounds().is_none() {
            return;
        }
        let params =
            serde_json::json!({ "relative": true, "x": by.x, "y": by.y, "anchor": "bottom_left" });
        if let Err(e) = self.apply("move", &params) {
            self.status.set_message(&e);
        }
    }

    fn shortcuts(&mut self, ctx: &egui::Context) {
        // single-letter shortcuts must not fire while typing in a form field
        if !ctx.wants_keyboard_input() {
            let job = self.screen == Screen::Job;
            let fired: Vec<String> = self
                .bindings()
                .into_iter()
                .filter(|(id, _)| job || ["new", "open"].contains(&id.as_str()))
                .filter(|(_, keys)| ctx.input_mut(|i| i.consume_shortcut(keys)))
                .map(|(id, _)| id)
                .collect();
            for id in fired {
                self.invoke(ctx, &id);
            }
        }
        // Escape in a field being typed in only leaves the field
        if !ctx.wants_keyboard_input()
            && ctx.input_mut(|i| {
                i.consume_shortcut(&KeyboardShortcut::new(Modifiers::NONE, Key::Escape))
            })
        {
            if self.screen == Screen::NewJob {
                self.job_setup_cancel();
            }
            self.form = None;
            self.view_tool = None;
            self.pages.done_with(Page::Design);
            self.pages.done_with(Page::Toolpaths);
        }
        self.nudge(ctx);
        ctx.input(|i| {
            if i.key_pressed(Key::F2) {
                self.view = View::TwoD;
            }
            if i.key_pressed(Key::F3) {
                self.view = View::ThreeD;
            }
        });
    }

    /// Every key binding: each command's own shortcut, plus the built-ins as the menus spell them.
    fn bindings(&self) -> Vec<(String, KeyboardShortcut)> {
        let commands = self
            .registry
            .iter()
            .filter_map(|c| Some((c.id().to_owned(), shortcut::parse(c.meta().shortcut)?)));
        let built_ins = self
            .menus
            .items()
            .into_iter()
            .filter(|i| BUILT_INS.contains(&i.action.as_str()))
            .filter_map(|i| Some((i.action.clone(), shortcut::parse(&i.shortcut)?)));
        let layouts = LAYOUT_KEYS
            .iter()
            .filter_map(|&(id, keys)| Some((id.to_owned(), shortcut::parse(keys)?)));
        commands.chain(built_ins).chain(layouts).collect()
    }

    fn menu_bar(&mut self, ui: &mut egui::Ui) {
        let mut clicked = None;
        let recent: Vec<gui::MenuItem> = self
            .recent
            .iter()
            .enumerate()
            .map(|(i, path)| {
                gui::MenuItem::with_action(&file_name(path), &format!("open_recent:{i}"))
            })
            .collect();
        egui::menu::bar(ui, |ui| {
            for menu in &self.current_menus().menus {
                ui.menu_button(&menu.name, |ui| {
                    menu_items(
                        ui,
                        &menu.items,
                        &Menus {
                            available: &|id| self.available(id),
                            checked: &|id| self.display.checked(id),
                            shortcut_of: &|id| self.registry.get(id).map(|c| c.meta().shortcut),
                            recent: &recent,
                        },
                        &mut clicked,
                    );
                });
            }
        });
        if let Some(id) = clicked {
            self.invoke(ui.ctx(), &id);
        }
    }

    /// The menu bar with or without a job open.
    fn current_menus(&self) -> &gui::MenuSystem {
        if self.screen == Screen::Start {
            &self.start_menus
        } else {
            &self.menus
        }
    }

    #[cfg(test)]
    fn menu_names(&self) -> Vec<&str> {
        self.current_menus()
            .menus
            .iter()
            .map(|m| m.name.as_str())
            .collect()
    }

    /// No job open: Aspire's startup tasks and the jobs opened lately.
    fn start_page(&mut self, ui: &mut egui::Ui) {
        let edge = ui.spacing().interact_size.y;
        let mut clicked = None;
        ui.strong("Startup Tasks");
        for (id, label, icon) in STARTUP_TASKS {
            let button = match self.icons.find(icon, id) {
                Some(path) => egui::Button::image_and_text(
                    Icons::image(path).fit_to_exact_size(egui::vec2(edge, edge)),
                    label,
                ),
                None => egui::Button::new(label),
            }
            .frame(false);
            let response = ui
                .add_enabled(self.available(id), button)
                .on_disabled_hover_text(format!("{label}: not implemented yet"));
            if response.clicked() {
                clicked = Some(id.to_owned());
            }
        }
        ui.separator();
        ui.strong("Recently opened files");
        if self.recent.is_empty() {
            ui.weak("None yet");
        }
        let glyph = self.icons.find(RECENT_FILE_ICON, "");
        for (i, path) in self.recent.iter().enumerate() {
            let name = file_name(path);
            let button = match glyph {
                Some(glyph) => egui::Button::image_and_text(
                    Icons::image(glyph).fit_to_exact_size(egui::vec2(edge, edge)),
                    name,
                ),
                None => egui::Button::new(name),
            }
            .frame(false);
            if ui
                .add(button)
                .on_hover_text(path.display().to_string())
                .clicked()
            {
                clicked = Some(format!("open_recent:{i}"));
            }
        }
        ui.separator();
        ui.strong(format!("What's new in v{}", env!("CARGO_PKG_VERSION")));
        for change in WHATS_NEW.lines().filter(|l| !l.trim().is_empty()) {
            ui.label(change.trim_start_matches("- "));
        }
        ui.separator();
        ui.strong("Links");
        for (label, url, icon) in LINKS {
            ui.horizontal(|ui| {
                if let Some(path) = self.icons.find(icon, "") {
                    ui.add(Icons::image(path).fit_to_exact_size(egui::vec2(edge, edge)));
                }
                ui.hyperlink_to(label, url);
            });
        }
        if let Some(id) = clicked {
            self.invoke(ui.ctx(), &id);
        }
    }

    /// The middle of the window with no job open: the Cut Craft mark.
    fn splash(ui: &mut egui::Ui) {
        let logo = std::path::Path::new(LOGO);
        let space = ui.available_size();
        let edge = space.min_elem() * SPLASH_SHARE;
        ui.vertical_centered(|ui| {
            ui.add_space((space.y - edge) / 2.0);
            if logo.exists() {
                ui.add(Icons::image(logo).fit_to_exact_size(egui::vec2(edge, edge)));
            }
            ui.heading(APP_NAME);
        });
    }

    /// The Design page: the chosen design tab's tools, under any form opened from them.
    fn design_page(&mut self, ui: &mut egui::Ui) {
        self.page_header(ui, Page::Design, DESIGN_TABS[self.design_tab]);
        egui::ScrollArea::vertical().show(ui, |ui| {
            // a tool's form takes the page's place, as Aspire's does
            if self.form.as_ref().is_some_and(|f| f.page == Page::Design) {
                self.command_form(ui);
                return;
            }
            match DESIGN_TABS[self.design_tab] {
                "Design" => {
                    let groups = self.design_groups("Design");
                    self.tool_groups(ui, &groups, Page::Design);
                }
                "Sheets" => self.sheets_panel(ui),
                "Clipart" => self.clipart_panel(ui),
                "Layers" => self.layers_panel(ui),
                "Components" => self.component_list(ui),
                tab => {
                    ui.weak(format!("{tab} tools not implemented yet"));
                }
            }
        });
    }

    /// The tool groups a design tab shows: Design has the drawing then the modeling tools.
    fn design_groups(&self, tab: &str) -> Vec<gui::ToolGroup> {
        match tab {
            "Design" => {
                let toolbar = &self.toolbar;
                toolbar
                    .groups
                    .iter()
                    .chain(&toolbar.modeling_groups)
                    .copied()
                    .collect()
            }
            _ => vec![],
        }
    }

    /// A page's title with its push-pin and Aspire's Switch Layout button.
    fn page_header(&mut self, ui: &mut egui::Ui, page: Page, title: &str) {
        let row = ui.spacing().interact_size.y;
        let (mut pin, mut switch) = (false, false);
        ui.horizontal(|ui| {
            ui.strong(title);
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                let pinned = self.pages.pinned(page);
                pin = ui
                    .selectable_label(pinned, "📌")
                    .on_hover_text(if pinned {
                        "Unpin: hide this page to its side tab"
                    } else {
                        "Pin this page open"
                    })
                    .clicked();
                switch = ui
                    .add(self.icon_button("view/interface_layout.svg", "", "Switch Layout", row))
                    .on_hover_text("Switch Layout (F11 designing, F12 machining)")
                    .clicked();
            });
        });
        ui.separator();
        if pin {
            let pinned = self.pages.pinned(page);
            self.pages.set_pinned(page, !pinned);
        }
        if switch {
            self.pages.switch_layout();
        }
    }

    /// The side tabs and the docked pages around the view; flown-out pages over it.
    fn job_pages(&mut self, ctx: &egui::Context) {
        let style = ctx.style();
        let (row, margin) = (style.spacing.interact_size.y, style.spacing.item_spacing.y);
        let width = DESIGN_PANEL_WIDTHS * style.spacing.interact_size.x;
        let strip = || egui::Frame::side_top_panel(&style).inner_margin(margin);
        let left_tabs = egui::SidePanel::left("design_tabs")
            .resizable(false)
            .exact_width(row + 2.0 * margin)
            .frame(strip())
            .show(ctx, |ui| self.design_tabs(ui))
            .response
            .rect;
        let right_tabs = egui::SidePanel::right("toolpath_tabs")
            .resizable(false)
            .exact_width(row + 2.0 * margin)
            .frame(strip())
            .show(ctx, |ui| self.toolpath_tabs(ui))
            .response
            .rect;
        if self.pages.shown(Page::Design) == Shown::Docked {
            egui::SidePanel::left("design_panel")
                .default_width(width)
                .show(ctx, |ui| self.design_page(ui));
        }
        if self.pages.shown(Page::Toolpaths) == Shown::Docked {
            egui::SidePanel::right("toolpaths_panel")
                .default_width(width)
                .show(ctx, |ui| self.toolpaths_page(ui));
        }
        let room = ctx.available_rect();
        // a click on the view puts a flown-out page away, unless its form is open
        let pressed = ctx.input(|i| {
            i.pointer
                .primary_pressed()
                .then(|| i.pointer.interact_pos())
                .flatten()
        });
        for (page, tabs) in [(Page::Design, left_tabs), (Page::Toolpaths, right_tabs)] {
            if self.pages.shown(page) != Shown::Flyout {
                continue;
            }
            let out = self.flyout(ctx, page, room);
            let form_here = self.form.as_ref().is_some_and(|f| f.page == page);
            let on_view = |at: egui::Pos2| {
                room.contains(at)
                    && ctx.layer_id_at(at).map(|l| l.order) != Some(egui::Order::Foreground)
            };
            if let Some(at) = pressed
                && on_view(at)
                && !out.contains(at)
                && !tabs.contains(at)
                && !form_here
            {
                self.pages.done_with(page);
            }
        }
    }

    /// The tool page a command's form shows in.
    fn page_of(&self, id: &str) -> Page {
        let toolpaths = self
            .toolbar
            .toolpath_groups
            .iter()
            .any(|g| g.tools.contains(&id));
        if toolpaths {
            Page::Toolpaths
        } else {
            Page::Design
        }
    }

    /// A tool picked from a page: a page that flew out for it goes away again, unless the
    /// tool opened a form there.
    fn pick(&mut self, ctx: &egui::Context, page: Page, id: &str) {
        self.invoke(ctx, id);
        if self.form.is_none() {
            self.pages.done_with(page);
        }
    }

    /// Aspire's design tabs, down the left edge.
    fn design_tabs(&mut self, ui: &mut egui::Ui) {
        let shown = self.pages.shown(Page::Design) != Shown::Hidden;
        let (hovered, clicked) = side_tabs(ui, &DESIGN_TABS, shown.then_some(self.design_tab));
        // an auto-hidden page comes out on hover; a docked one changes tab on click
        let unpinned = !self.pages.pinned(Page::Design);
        if let Some(tab) = clicked.or(hovered.filter(|_| unpinned)) {
            self.design_tab = tab;
            self.pages.open(Page::Design);
        }
    }

    /// The Toolpaths tab, down the right edge.
    fn toolpath_tabs(&mut self, ui: &mut egui::Ui) {
        let shown = self.pages.shown(Page::Toolpaths) != Shown::Hidden;
        let (hovered, clicked) = side_tabs(ui, &["Toolpaths"], shown.then_some(0));
        let unpinned = !self.pages.pinned(Page::Toolpaths);
        if clicked.or(hovered.filter(|_| unpinned)).is_some() {
            self.pages.open(Page::Toolpaths);
        }
    }

    /// An auto-hidden page flown out over the view from its side; returns where it is.
    fn flyout(&mut self, ctx: &egui::Context, page: Page, room: egui::Rect) -> egui::Rect {
        let width = DESIGN_PANEL_WIDTHS * ctx.style().spacing.interact_size.x;
        let frame = egui::Frame::side_top_panel(&ctx.style());
        let height = room.height() - frame.inner_margin.sum().y - frame.outer_margin.sum().y;
        let left = match page {
            Page::Design => room.left(),
            Page::Toolpaths => room.right() - width - frame.inner_margin.sum().x,
        };
        egui::Area::new(egui::Id::new(("flyout", page)))
            .fixed_pos(egui::pos2(left, room.top()))
            .order(egui::Order::Foreground)
            .show(ctx, |ui| {
                frame.show(ui, |ui| {
                    ui.set_width(width);
                    ui.set_height(height);
                    match page {
                        Page::Design => self.design_page(ui),
                        Page::Toolpaths => self.toolpaths_page(ui),
                    }
                });
            })
            .response
            .rect
    }

    /// The open form's buttons: OK and Cancel for a new job's setup, else Apply and Close.
    fn form_buttons(&mut self, ui: &mut egui::Ui) {
        let Some(open) = &self.form else { return };
        let new_job = open.new_job;
        let (apply_label, close_label) = if new_job {
            ("OK", "Cancel")
        } else {
            ("Apply", "Close")
        };
        let (mut apply, mut close) = (false, false);
        ui.horizontal(|ui| {
            apply = ui.button(apply_label).clicked();
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                close = ui.button(close_label).clicked();
            });
        });
        match (new_job, apply, close) {
            (true, true, _) => self.job_setup_ok(),
            (true, _, true) => self.job_setup_cancel(),
            (false, true, _) => {
                let (id, params) = (open.id, open.params.clone());
                let result = self.apply_form(id, &params);
                if let Some(open) = &mut self.form {
                    open.error = result.err();
                }
            }
            (false, _, true) => {
                let page = open.page;
                self.form = None;
                self.pages.done_with(page);
            }
            _ => {}
        }
    }

    fn command_form(&mut self, ui: &mut egui::Ui) {
        let Some(open) = &mut self.form else { return };
        ui.horizontal(|ui| {
            if let Some(path) = self.icons.find(open.icon, open.id) {
                let edge = ui.spacing().interact_size.y;
                ui.add(Icons::image(path).fit_to_exact_size(egui::vec2(edge, edge)));
            }
            ui.strong(open.label);
        });
        let (mut chosen, mut save_tool) = (None, false);
        if open.params.get("tool").is_some_and(Value::is_object) {
            let current = open.params["tool"]["name"].as_str().unwrap_or_default();
            ui.horizontal(|ui| {
                egui::ComboBox::from_label("Tool database")
                    .selected_text(current)
                    .show_ui(ui, |ui| {
                        for tool in &self.tools.tools {
                            if ui
                                .selectable_label(tool.name == current, &tool.name)
                                .clicked()
                            {
                                chosen = Some(tool.name.clone());
                            }
                        }
                    });
                save_tool = ui.button("Save to database").clicked();
            });
        }
        // rest machining: the tools that ran before this one, picked from the database
        let mut earlier = None;
        if open.params.get("after").is_some_and(Value::is_array) {
            egui::ComboBox::from_label("Add an earlier tool (rest machining)")
                .selected_text("")
                .show_ui(ui, |ui| {
                    for tool in &self.tools.tools {
                        if ui.selectable_label(false, &tool.name).clicked() {
                            earlier = Some(tool.name.clone());
                        }
                    }
                });
        }
        // a form that sets the units (Job Setup) shows its lengths in them as they change
        let edge = SECTION_PICTURE_ROWS * ui.spacing().interact_size.y;
        let picture = |path: &str| {
            self.icons
                .find(path, "")
                .map(|p| Icons::image(p).fit_to_exact_size(egui::vec2(edge, edge)))
        };
        let units = form::units_chosen(&open.params).unwrap_or(self.doc.job.units);
        let look = form::Look {
            units,
            picture: &picture,
            material: form::material(&self.doc, units),
        };
        form::show(ui, &open.fields, &mut open.params, &look);
        // every toolpath can rest machine (3D Finishing has its own, as `after`)
        if open.page == Page::Toolpaths && open.params.get("after").is_none() {
            rest_section(ui, &mut open.params, &self.tools.tools);
        }
        if let Some(error) = &open.error {
            ui.colored_label(ui.visuals().error_fg_color, error);
        }
        // Job Setup for a new job keeps its buttons pinned under the panel instead
        if !open.new_job {
            self.form_buttons(ui);
            ui.separator();
        }
        if let Some(name) = chosen {
            self.choose_tool(&name);
        }
        if save_tool {
            self.save_form_tool();
        }
        if let Some(name) = earlier {
            self.add_earlier_tool(&name);
        }
    }

    /// Adds a database tool to the open form's rest-machining list.
    fn add_earlier_tool(&mut self, name: &str) {
        if let (Some(form), Some(tool)) = (&mut self.form, self.tools.get(name))
            && let Some(after) = form.params.get_mut("after").and_then(Value::as_array_mut)
        {
            after.push(serde_json::to_value(tool).expect("tools serialize"));
        }
    }

    /// Reads the user's tool database; a broken one is reported and left untouched on disk.
    fn load_tools(&mut self) {
        self.tools_path = files::tool_database_path();
        if let Some(path) = &self.tools_path {
            match files::load_tools(path) {
                Ok(tools) => self.tools = tools,
                Err(e) => {
                    self.status
                        .set_message(&format!("{e}; tool changes will not be saved"));
                    self.tools_path = None;
                }
            }
        }
    }

    /// Copies a database tool into the open form.
    fn choose_tool(&mut self, name: &str) {
        if let (Some(form), Some(tool)) = (&mut self.form, self.tools.get(name)) {
            form.params["tool"] = serde_json::to_value(tool).expect("tools serialize");
        }
    }

    /// Stores the open form's tool in the database, replacing one of the same name.
    fn save_form_tool(&mut self) {
        let tool = self
            .form
            .as_ref()
            .and_then(|f| serde_json::from_value::<Tool>(f.params["tool"].clone()).ok());
        match tool {
            Some(tool) => {
                let done = format!("Saved {} to the tool database", tool.name);
                self.tools.put(tool);
                self.tools_changed(&done);
            }
            None => self
                .status
                .set_message("the form has no valid tool to save"),
        }
    }

    fn delete_tool(&mut self, index: usize) {
        if index < self.tools.tools.len() {
            let tool = self.tools.tools.remove(index);
            self.tools_changed(&format!("Deleted {} from the tool database", tool.name));
        }
    }

    /// Writes the tool database, if it has a file, and reports `done` or the failure.
    fn tools_changed(&mut self, done: &str) {
        let result = self
            .tools_path
            .as_deref()
            .map_or(Ok(()), |path| files::save_tools(&self.tools, path));
        self.status
            .set_message(&result.map_or_else(|e| e, |()| done.to_owned()));
    }

    /// Model > Create Component from Toolpath Preview: the block as previewed, standing on
    /// the material's bottom.
    fn component_from_preview(&mut self) {
        if let Some(preview) = &self.simulation {
            // the block as cut, standing on the material's bottom
            let mut block = preview.clone();
            for h in &mut block.heights {
                *h -= preview.bottom;
            }
            let component = cut_craft_core::component::Component::from_material(
                "Toolpath Preview",
                cut_craft_core::component::Combine::Add,
                &block,
            );
            let mut c = Change::builder(&self.doc);
            c.add_component(component);
            self.record(Ok(c.finish()), "Made a component from the toolpath preview");
        }
    }

    /// Cut, Copy, Paste, Increment and Save, Open Application Data Folder.
    fn clipboard_or_file_action(&mut self, id: &str) {
        match id {
            "copy" | "cut" => {
                self.clipboard = self
                    .doc
                    .selection
                    .iter()
                    .map(|id| self.doc.vectors[id].clone())
                    .collect();
                if id == "cut"
                    && let Err(e) = self.apply("delete", &serde_json::json!({}))
                {
                    self.status.set_message(&e);
                }
            }
            // pasted where they were copied from, in this job or another (Paste in Place)
            "paste" | "paste_in_place" => {
                let mut c = Change::builder(&self.doc);
                let ids: Vec<_> = self.clipboard.iter().map(|p| c.add(p.clone())).collect();
                c.select(ids);
                self.record(Ok(c.finish()), "Pasted");
            }
            "increment_and_save" => match self.session.path.clone() {
                Some(path) => self.save_to(&files::incremented(&path, std::path::Path::exists)),
                None => self.file_action("save_as"),
            },
            // ponytail: the drawing as a PDF opened in the system viewer, which prints it;
            // Aspire's own print window (scale, tiling) is not done
            "print" | "print_preview" => {
                let path = std::env::temp_dir().join("cut-craft-print.pdf");
                let opened = files::export(&self.registry, "export_pdf", &self.doc, &path)
                    .and_then(|()| {
                        on_the_desktop();
                        std::process::Command::new("xdg-open")
                            .arg(&path)
                            .spawn()
                            .map_err(|e| e.to_string())
                    });
                if let Err(e) = opened {
                    self.status.set_message(&format!("could not print: {e}"));
                }
            }
            "open_application_data_folder" => {
                let opened = files::config_dir()
                    .ok_or_else(|| "no home folder".to_owned())
                    .and_then(|dir| {
                        std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
                        on_the_desktop();
                        files::show_folder(&dir).map_err(|e| e.to_string())
                    });
                if let Err(e) = opened {
                    self.status
                        .set_message(&format!("could not open the folder: {e}"));
                }
            }
            _ => {}
        }
    }

    /// The Help windows: About, Keyboard Shortcuts, Third Party Licenses.
    fn info_window(&mut self, ctx: &egui::Context) {
        let Some(info) = self.info else { return };
        let mut open = true;
        let title = match info {
            Info::About => "About Cut Craft",
            Info::Shortcuts => "Keyboard Shortcuts",
            Info::Licenses => "Third Party Licenses",
            Info::Summary => "Toolpath Summary",
        };
        let shortcuts: Vec<(String, String)> = if info == Info::Shortcuts {
            let mut rows: Vec<(String, String)> = self
                .bindings()
                .into_iter()
                .map(|(id, keys)| {
                    let label = self
                        .registry
                        .get(&id)
                        .map_or_else(|| gui::title_case(&id), |c| c.meta().label.to_owned());
                    (ctx.format_shortcut(&keys), label)
                })
                .collect();
            rows.sort_by(|a, b| a.1.cmp(&b.1));
            rows
        } else {
            vec![]
        };
        let times: Vec<(String, f64, bool)> = self
            .doc
            .toolpaths
            .values()
            .map(|t| {
                (
                    t.name.clone(),
                    t.machining_minutes(self.summary.0) * self.summary.1,
                    t.visible,
                )
            })
            .collect();
        let summary = &mut self.summary;
        let units = self.doc.job.units;
        egui::Window::new(title)
            .open(&mut open)
            .collapsible(false)
            .show(ctx, |ui| match info {
                Info::Summary => summary_table(ui, &times, summary, units),
                Info::About => {
                    ui.heading("Cut Craft");
                    ui.label(format!("Version {}", env!("CARGO_PKG_VERSION")));
                    ui.label("2D and 3D design and CNC toolpaths.");
                }
                Info::Shortcuts => {
                    egui::ScrollArea::vertical()
                        .max_height(400.0)
                        .show(ui, |ui| {
                            egui::Grid::new("shortcuts").striped(true).show(ui, |ui| {
                                for (keys, label) in &shortcuts {
                                    ui.monospace(keys);
                                    ui.label(label);
                                    ui.end_row();
                                }
                            });
                        });
                }
                Info::Licenses => {
                    egui::Grid::new("licenses").striped(true).show(ui, |ui| {
                        for (name, licence) in THIRD_PARTY {
                            ui.label(name);
                            ui.label(licence);
                            ui.end_row();
                        }
                    });
                }
            });
        if !open {
            self.info = None;
        }
    }
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

impl CutCraftApp {
    /// Pick a tool to edit or delete; edits are saved as they are made.
    fn tool_database_window(&mut self, ctx: &egui::Context) {
        let (mut open, mut delete, mut edited) = (self.tool_window, None, false);
        let mut imported = None;
        egui::Window::new("Tool Database")
            .open(&mut open)
            .show(ctx, |ui| {
                if ui
                    .button("Import Vectric tool database…")
                    .on_hover_text("Adds the tools of a .vtdb file (Aspire / VCarve 12.5)")
                    .clicked()
                    && let Some(path) = file_dialog()
                        .add_filter("Vectric tool database", &["vtdb"])
                        .pick_file()
                {
                    imported = Some(path);
                }
                let Some(last) = self.tools.tools.len().checked_sub(1) else {
                    ui.weak("No tools: save one from a toolpath form");
                    return;
                };
                self.tool_selected = self.tool_selected.min(last);
                ui.horizontal(|ui| {
                    egui::ComboBox::from_id_source("tool_database_tools")
                        .selected_text(&self.tools.tools[self.tool_selected].name)
                        .show_ui(ui, |ui| {
                            for (i, tool) in self.tools.tools.iter().enumerate() {
                                ui.selectable_value(&mut self.tool_selected, i, &tool.name);
                            }
                        });
                    if ui.button("Delete").clicked() {
                        delete = Some(self.tool_selected);
                    }
                });
                let fields = form::fields(
                    &serde_json::to_value(schemars::schema_for!(Tool)).expect("schemas serialize"),
                );
                let tool = &mut self.tools.tools[self.tool_selected];
                let mut value = serde_json::to_value(&*tool).expect("tools serialize");
                let look = form::Look {
                    units: self.doc.job.units,
                    picture: &|_| None,
                    material: form::material(&self.doc, self.doc.job.units),
                };
                if form::show(ui, &fields, &mut value, &look)
                    && let Ok(changed) = serde_json::from_value(value)
                {
                    *tool = changed;
                    edited = true;
                }
                // Aspire shows the feed per tooth beside the feeds, to check against the maker's
                if let Some(load) = tool.chip_load() {
                    let (per, unit) = form::shown_in("mm", look.units).unwrap_or((1.0, "mm"));
                    let places = if unit == "mm" { 3 } else { 4 };
                    ui.label(format!("Chip load: {:.places$} {unit}", load / per));
                }
            });
        self.tool_window = open;
        if let Some(path) = imported {
            self.import_tools(&path);
        } else if let Some(index) = delete {
            self.delete_tool(index);
        } else if edited {
            self.tools_changed("Tool database saved");
        }
    }

    /// Adds the tools of a Vectric `.vtdb` file to the database and saves it.
    fn import_tools(&mut self, path: &std::path::Path) {
        match cut_craft_vtdb::read(path) {
            Ok(tools) => {
                let n = tools.len();
                self.tools.tools.extend(tools);
                self.tools_changed(&format!("Imported {n} tools from {}", file_name(path)));
            }
            Err(e) => self.status.set_message(&e),
        }
    }

    /// Icon buttons for groups of command ids: implemented ones run, the rest are greyed out.
    fn tool_groups(&mut self, ui: &mut egui::Ui, groups: &[gui::ToolGroup], page: Page) {
        let edge = TOOL_ICON_ROWS * ui.spacing().interact_size.y;
        let mut clicked = None;
        // plain sections, as Aspire's tool pages have: a heading over its tools
        for group in groups {
            ui.strong(group.name);
            ui.horizontal_wrapped(|ui| {
                for &id in group.tools {
                    let cmd = self.registry.get(id);
                    let label =
                        cmd.map_or_else(|| gui::title_case(id), |c| c.meta().label.to_owned());
                    let explicit = cmd.map_or("", |c| c.meta().icon);
                    let button = self
                        .icon_button(explicit, id, &label, edge)
                        .selected(self.status.current_tool == label);
                    let response = ui
                        .add_enabled(self.available(id), button)
                        .on_hover_text(&label)
                        .on_disabled_hover_text(format!("{label}: not implemented yet"));
                    if response.clicked() {
                        clicked = Some(id);
                    }
                }
            });
            ui.add_space(ui.spacing().item_spacing.y);
        }
        if let Some(id) = clicked {
            self.pick(ui.ctx(), page, id);
        }
    }

    /// An icon button, or a text one when the icon is missing.
    fn icon_button(&self, icon: &str, id: &str, label: &str, edge: f32) -> egui::Button<'static> {
        match self.icons.find(icon, id) {
            Some(path) => {
                egui::Button::image(Icons::image(path).fit_to_exact_size(egui::vec2(edge, edge)))
            }
            None => egui::Button::new(label.to_owned()),
        }
    }

    /// Layers, top first: visibility and lock toggles, the current layer highlighted, and
    /// Aspire's layer menu on a right-click.
    fn layers_panel(&mut self, ui: &mut egui::Ui) {
        let edge = ui.spacing().interact_size.y;
        let label = |id: &str| self.registry.get(id).map_or("", |c| c.meta().label);
        let mut clicked: Option<(LayerId, &str)> = None;
        let mut chosen = None;
        for layer in self.doc.layers.iter().rev() {
            ui.horizontal(|ui| {
                let (eye, eye_tip) = if layer.visible {
                    ("layers/visible.svg", "Hide")
                } else {
                    ("layers/hidden.svg", "Show")
                };
                let (lock, lock_tip) = if layer.locked {
                    ("layers/locked.svg", "Unlock")
                } else {
                    ("layers/unlocked.svg", "Lock")
                };
                if ui
                    .add(self.icon_button(eye, "", eye_tip, edge))
                    .on_hover_text(eye_tip)
                    .clicked()
                {
                    clicked = Some((layer.id, "toggle_visible"));
                }
                if ui
                    .add(self.icon_button(lock, "", lock_tip, edge))
                    .on_hover_text(lock_tip)
                    .clicked()
                {
                    clicked = Some((layer.id, "toggle_locked"));
                }
                let count = self.doc.vectors_on(layer.id).count();
                let row = ui
                    .selectable_label(self.doc.current_layer == layer.id, &layer.name)
                    .on_hover_text(format!("{count} vectors"));
                if row.clicked() {
                    clicked = Some((layer.id, "set_current_layer"));
                }
                if row.double_clicked() {
                    clicked = Some((layer.id, "layer_properties"));
                }
                if let Some(id) = self.popup(&row, || gui::layer_menu(&self.doc, layer)) {
                    chosen = Some(id);
                }
            });
        }
        let add = ui.button(label("add_layer")).clicked();
        if let Some((layer, action)) = clicked {
            self.layer_action(ui.ctx(), layer, action);
        }
        if add {
            self.invoke(ui.ctx(), "add_layer");
        }
        if let Some(id) = chosen {
            self.invoke(ui.ctx(), &id);
        }
    }

    /// The level menu's Select Components (`SELECT_LEVEL<id>`) and Export as 3D Clipart
    /// (`EXPORT_LEVEL<id>`, which selects them and exports).
    fn level_action(&mut self, id: &str) {
        let (how, level) = id.split_once(':').unwrap_or_default();
        if let Ok(level) = level.parse() {
            self.doc.selected_components = self.doc.level_components(level).into_iter().collect();
            if format!("{how}:") == EXPORT_LEVEL {
                self.export("export_as_3d_clipart");
            }
        }
    }

    /// The Clipart tab: its library, and the chosen folder's clipart to double-click in.
    fn clipart_panel(&mut self, ui: &mut egui::Ui) {
        let registry = &self.registry;
        let importable = |ext: &str| registry.importer_for(ext).is_some();
        match crate::clipart::panel(ui, &mut self.clipart, &importable) {
            Some(crate::clipart::Picked::AddFolder) => self.invoke(ui.ctx(), "add_clipart_folder"),
            Some(crate::clipart::Picked::OpenFolder) => {
                self.invoke(ui.ctx(), "open_clipart_folder");
            }
            Some(crate::clipart::Picked::Import(path)) => self.import_file(&path),
            None => {}
        }
    }

    /// Brings `path` into the job: meshes through Orientate 3D Model, as Aspire's import
    /// does; anything else through its importer.
    fn import_file(&mut self, path: &std::path::Path) {
        let stl = path
            .extension()
            .is_some_and(|e| e.eq_ignore_ascii_case("stl"));
        if stl {
            self.start_command("orientate_3d_model");
            if let Some(form) = self.form.as_mut().filter(|f| f.id == "orientate_3d_model") {
                form.params["file"] = Value::from(path.display().to_string());
            }
            return;
        }
        let change = files::import(&self.registry, &self.doc, path)
            .map_err(cut_craft_core::CommandError::Failed);
        self.record(change, &format!("Imported {}", path.display()));
    }

    /// The Sheets tab: Aspire's Sheets List (each sheet's name and size, the active one
    /// marked) and its Add New Sheet button. A click activates a sheet; a right-click has its
    /// menu.
    fn sheets_panel(&mut self, ui: &mut egui::Ui) {
        ui.strong("Sheets List");
        let units = self.doc.job.units;
        let (mut activate, mut chosen) = (None, None);
        egui::Grid::new("sheets_list").striped(true).show(ui, |ui| {
            for heading in ["Name", "X", "Y", "Z"] {
                ui.strong(heading);
            }
            ui.end_row();
            for (i, sheet) in self.doc.sheets.iter().enumerate() {
                let job = self.doc.sheet_job(i).unwrap_or(sheet.job);
                let active = i == self.doc.active_sheet;
                let row = ui.radio(active, &sheet.name);
                if row.clicked() && !active {
                    activate = Some(i);
                }
                if let Some(id) = self.popup(&row, || gui::sheet_menu(&self.doc, i)) {
                    chosen = Some(id);
                }
                for mm in [job.width, job.height, job.thickness] {
                    ui.label(sheet_length(mm, units));
                }
                ui.end_row();
            }
        });
        let add = ui.button("Add New Sheet...").clicked();
        if let Some(sheet) = activate
            && let Err(e) = self.apply("activate_sheet", &serde_json::json!({ "sheet": sheet }))
        {
            self.status.set_message(&e);
        }
        if add {
            self.invoke(ui.ctx(), "add_sheet");
        }
        if let Some(id) = chosen {
            self.invoke(ui.ctx(), &id);
        }
    }

    /// A right-click menu on `response`, built when it opens: the action chosen, if any.
    fn popup(
        &self,
        response: &egui::Response,
        menu: impl FnOnce() -> Vec<gui::MenuItem>,
    ) -> Option<String> {
        let mut clicked = None;
        response.context_menu(|ui| {
            let menus = Menus {
                available: &|id| self.available(id),
                checked: &|_| None,
                shortcut_of: &|id| self.registry.get(id).map(|c| c.meta().shortcut),
                recent: &[],
            };
            menu_items(ui, &menu(), &menus, &mut clicked);
        });
        clicked
    }

    /// A layer-panel action: the toggles are undoable; choosing a layer is view state like the
    /// selection, so it is not.
    fn layer_action(&mut self, ctx: &egui::Context, layer: LayerId, action: &str) {
        let Some(l) = self.doc.layer(layer).cloned() else {
            return;
        };
        let properties = |visible: bool, locked: bool| serde_json::json!({ "layer": l.id, "name": l.name, "visible": visible, "locked": locked });
        let result = match action {
            "toggle_visible" => self.apply("layer_properties", &properties(!l.visible, l.locked)),
            "toggle_locked" => self.apply("layer_properties", &properties(l.visible, !l.locked)),
            "set_current_layer" => self
                .registry
                .get(action)
                .expect("set_current_layer is registered")
                .run_json(&self.doc, &serde_json::json!({ "layer": layer }))
                .map_err(|e| e.to_string())
                .map(|change| {
                    self.doc
                        .apply(&change)
                        .expect("the panel lists existing layers");
                }),
            // the properties form starts from the current layer
            "layer_properties" => {
                self.layer_action(ctx, layer, "set_current_layer");
                self.invoke(ctx, action);
                Ok(())
            }
            _ => unreachable!("not a layer panel action: {action}"),
        };
        if let Err(e) = result {
            self.status.set_message(&e);
        }
    }

    /// Cuts the toolpath from the previewed material (uncut material if there is none yet).
    /// The cascade: start again from uncut material, then preview each visible toolpath.
    fn preview_all(&mut self) {
        self.finish_playback();
        self.simulation = None;
        let visible: Vec<EntityId> = self
            .doc
            .toolpaths
            .iter()
            .filter(|(_, t)| t.visible)
            .map(|(&id, _)| id)
            .collect();
        if self.display.animate_preview {
            self.play(&visible);
            return;
        }
        for id in visible {
            self.preview_toolpath(id);
        }
    }

    /// Starts playing `toolpaths` into the previewed material (uncut if there is none yet).
    fn play(&mut self, toolpaths: &[EntityId]) {
        self.finish_playback();
        let start = self
            .simulation
            .take()
            .unwrap_or_else(|| Material::new(&self.doc.job, self.display.preview_cells()));
        let list: Vec<&cut_craft_core::toolpath::Toolpath> = toolpaths
            .iter()
            .filter_map(|id| self.doc.toolpaths.get(id))
            .collect();
        self.playback = Some(cut_craft_core::playback::Playback::new(start, &list));
        (self.playing, self.simulation_coarse) = (true, None);
        if self.display.auto_open_3d {
            self.view = View::ThreeD;
        }
    }

    /// Ends a playing preview, keeping what it has cut (Stop, or the end of the path).
    fn finish_playback(&mut self) {
        if let Some(playback) = self.playback.take() {
            self.simulation = Some(playback.material().clone());
            (self.playing, self.simulation_coarse) = (false, None);
        }
    }

    fn preview_toolpath(&mut self, id: EntityId) {
        if self.display.animate_preview {
            self.play(&[id]);
            return;
        }
        self.finish_playback();
        let Some(toolpath) = self.doc.toolpaths.get(&id) else {
            return;
        };
        self.simulation
            .get_or_insert_with(|| Material::new(&self.doc.job, self.display.preview_cells()))
            .cut(toolpath);
        self.simulation_coarse = None;
        if self.display.auto_open_3d {
            self.view = View::ThreeD;
        }
        self.status
            .set_message(&format!("Previewed {}", toolpath.name));
    }

    /// Aspire's Preview Control while a preview plays: Run / Pause, Single Step, Run to
    /// Retract and Stop, the tool's place along the path (drag to scrub) and the speed. A
    /// playing preview moves on here each frame.
    fn playback_bar(&mut self, ui: &mut egui::Ui) {
        let Some(playback) = self.playback.as_mut() else {
            return;
        };
        if self.playing {
            let dt = f64::from(ui.input(|i| i.stable_dt));
            playback.seek(playback.at() + self.playback_speed * dt);
            ui.ctx().request_repaint();
        }
        let mut stop = playback.finished();
        ui.horizontal(|ui| {
            let run = if self.playing { "Pause" } else { "Run" };
            if ui.button(run).clicked() {
                self.playing = !self.playing;
            }
            if ui.button("Single Step").clicked() {
                self.playing = false;
                playback.seek(playback.next_step());
            }
            if ui.button("Run to Retract").clicked() {
                self.playing = false;
                playback.seek(playback.next_retract());
            }
            stop |= ui
                .button("Stop")
                .on_hover_text("Keep what is cut so far")
                .clicked();
            let mut at = playback.at();
            let length = playback.length();
            if ui
                .add(
                    egui::Slider::new(&mut at, 0.0..=length)
                        .suffix(" mm")
                        .show_value(false),
                )
                .on_hover_text("Where the tool is along the toolpaths")
                .changed()
            {
                self.playing = false;
                playback.seek(at);
            }
            ui.add(
                egui::Slider::new(&mut self.playback_speed, PLAYBACK_SPEEDS)
                    .logarithmic(true)
                    .suffix(" mm/s")
                    .text("Speed"),
            );
        });
        if stop {
            self.finish_playback();
        }
    }

    /// Aspire's arrows over the tree: the picked component up or down its level, as
    /// (component, direction, the id it moves to) when one is clicked.
    fn tree_arrows(
        &self,
        ui: &mut egui::Ui,
    ) -> Option<(EntityId, cmd_reorder_component::Direction, EntityId)> {
        use cmd_reorder_component::{Direction, neighbour};
        let mut moved = None;
        let last = self.doc.selected_components.last();
        ui.horizontal(|ui| {
            for (label, direction) in [("Move Up", Direction::Up), ("Move Down", Direction::Down)] {
                let to = last.and_then(|id| neighbour(&self.doc, id, direction));
                if ui
                    .add_enabled(to.is_some(), egui::Button::new(label))
                    .clicked()
                {
                    moved = last.zip(to).map(|(id, to)| (id, direction, to));
                }
            }
        });
        moved
    }

    /// One component's row in the tree: whether a click picked it (Shift: adds or takes
    /// away; a right-click on an unpicked one picks it) and any action from its menu.
    fn component_row(
        &self,
        ui: &mut egui::Ui,
        id: EntityId,
        component: &cut_craft_core::component::Component,
    ) -> (Option<(EntityId, bool)>, Option<String>) {
        let text = format!("{} ({:?})", component.name, component.combine);
        let on = self.doc.selected_components.contains(&id);
        let row = ui.selectable_label(on, text);
        let mut picked = None;
        if row.clicked() {
            picked = Some((id, ui.input(|i| i.modifiers.shift)));
        }
        if row.secondary_clicked() && !on {
            picked = Some((id, false));
        }
        let action = self.popup(&row, || gui::component_menu(&self.doc, id, component));
        (picked, action)
    }

    /// The component tree, top first: each level (the last on top) with its components
    /// under it, the model being built from the bottom up. Aspire's level and component
    /// menus on a right-click; right-clicking an unpicked component picks it first.
    /// A level's row in the component tree: Expand or Collapse, then its name and combine
    /// mode; the name's response (for its menu) and whether the icon was clicked.
    fn level_header(
        &self,
        ui: &mut egui::Ui,
        level: &cut_craft_core::component::Level,
        collapsed: bool,
    ) -> (egui::Response, bool) {
        let shown = if level.visible { "" } else { " (hidden)" };
        ui.horizontal(|ui| {
            let (icon, tip) = if collapsed {
                ("view/expand.svg", "Expand")
            } else {
                ("view/collapse.svg", "Collapse")
            };
            let edge = ui.spacing().interact_size.y;
            let toggled = ui
                .add(self.icon_button(icon, "", tip, edge))
                .on_hover_text(tip)
                .clicked();
            let name = format!("{}{shown} ({:?})", level.name, level.combine);
            (ui.label(egui::RichText::new(name).strong()), toggled)
        })
        .inner
    }

    fn component_list(&mut self, ui: &mut egui::Ui) {
        // the tree is the active sheet's, as Aspire's heading says
        if let Some(sheet) = self.doc.sheets.get(self.doc.active_sheet) {
            ui.strong(format!("Components ({})", sheet.name));
        }
        if self.doc.components.is_empty() {
            ui.weak("No components yet");
        }
        let components = &self.doc.components;
        self.doc
            .selected_components
            .retain(|id| components.contains_key(id));
        let first = self.doc.levels.keys().next().copied().unwrap_or(0);
        let on_level = |k: &cut_craft_core::component::Component, level: u64| {
            let at = if self.doc.levels.contains_key(&k.level) {
                k.level
            } else {
                first
            };
            at == level
        };
        let (mut chosen, mut picked) = (None, None);
        let moved = self.tree_arrows(ui);
        let mut toggle = None;
        for (&level_id, level) in self.doc.levels.iter().rev() {
            let collapsed = self.collapsed_levels.contains(&level_id);
            let (header, toggled) = self.level_header(ui, level, collapsed);
            if toggled {
                toggle = Some(level_id);
            }
            if let Some(action) =
                self.popup(&header, || gui::level_menu(&self.doc, level_id, level))
            {
                chosen = Some(action);
            }
            if collapsed {
                continue;
            }
            ui.indent(("level", level_id), |ui| {
                let on_this: Vec<(EntityId, &cut_craft_core::component::Component)> = self
                    .doc
                    .components
                    .iter()
                    .rev()
                    .filter(|(_, k)| on_level(k, level_id))
                    .map(|(&id, k)| (id, k))
                    .collect();
                let mut shown_groups = vec![];
                for &(id, component) in &on_this {
                    let Some(group) = &component.group else {
                        let (pick, action) = self.component_row(ui, id, component);
                        picked = pick.or(picked);
                        chosen = action.or(chosen.take());
                        continue;
                    };
                    // a group shows once, where its top member is, its members under it
                    if shown_groups.contains(&group.id) {
                        continue;
                    }
                    shown_groups.push(group.id);
                    let header = ui.label(format!("{} ({:?})", group.name, group.combine));
                    if let Some(action) = self.popup(&header, || gui::group_menu(group)) {
                        chosen = Some(action);
                    }
                    ui.indent(("group", group.id), |ui| {
                        for &(member, k) in &on_this {
                            if k.group.as_ref().is_some_and(|g| g.id == group.id) {
                                let (pick, action) = self.component_row(ui, member, k);
                                picked = pick.or(picked);
                                chosen = action.or(chosen.take());
                            }
                        }
                    });
                }
            });
        }
        // click picks one; Shift-click adds or takes away
        match picked {
            Some((id, true)) => {
                if !self.doc.selected_components.remove(&id) {
                    self.doc.selected_components.insert(id);
                }
            }
            Some((id, false)) => self.doc.selected_components = [id].into(),
            None => {}
        }
        if let Some(level) = toggle
            && !self.collapsed_levels.remove(&level)
        {
            self.collapsed_levels.insert(level);
        }
        if let Some((component, direction, to)) = moved {
            let params = cmd_reorder_component::Params {
                component,
                direction,
            };
            let params = serde_json::to_value(params).expect("params serialize");
            if self.apply("reorder_component", &params).is_ok() {
                // the moved component took its neighbour's id; keep it picked
                self.doc.selected_components = [to].into();
            }
        }
        if let Some(action) = chosen {
            self.invoke(ui.ctx(), &action);
        }
    }

    /// What the 3D view shows: the toolpath preview if there is one, else the model.
    // ponytail: rebuilt on every call, measured at about 0.1 ms of a 7.5 ms frame; cache it on
    // the history revision if that grows
    fn surface_3d(&mut self) -> Option<Cow<'_, Material>> {
        if let Some(playback) = &self.playback {
            return Some(Cow::Owned(view3d::coarse(playback.material(), MESH_CELLS)));
        }
        if let Some(material) = &self.simulation {
            return Some(Cow::Borrowed(
                self.simulation_coarse
                    .get_or_insert_with(|| view3d::coarse(material, MESH_CELLS)),
            ));
        }
        // before a preview, the model alone: nothing where no component is
        (!self.doc.components.is_empty()).then(|| {
            Cow::Owned(cut_craft_core::component::model_shape(
                &self.doc, MESH_CELLS,
            ))
        })
    }

    /// The preview or model as a block, with Aspire's 3D mouse controls: left- or right-drag
    /// rotates, middle-drag (or Ctrl+right, or left and right together) pans, the wheel zooms
    /// about the cursor, and a double-click or F fits it again.
    fn view_3d(&mut self, ui: &mut egui::Ui) {
        self.playback_bar(ui);
        if self.simulation.is_none() && self.playback.is_none() && self.doc.components.is_empty() {
            ui.centered_and_justified(|ui| {
                ui.weak("Nothing in 3D yet: create a shape or preview the toolpaths")
            });
            return;
        }
        ui.horizontal(|ui| {
            for (label, orbit) in [
                ("Top", view3d::Orbit::TOP),
                ("Front", view3d::Orbit::FRONT),
                ("Right", view3d::Orbit::RIGHT),
                ("Iso", view3d::Orbit::default()),
            ] {
                if ui.small_button(label).clicked() {
                    self.orbit = orbit;
                    self.framing = view3d::Framing::default();
                }
            }
            if ui.small_button("Fit").clicked() {
                self.framing = view3d::Framing::default();
            }
        });
        let (area, response) =
            ui.allocate_exact_size(ui.available_size(), egui::Sense::click_and_drag());
        let (left, right, middle, ctrl, scroll, fit_key) = ui.input(|i| {
            (
                i.pointer.primary_down(),
                i.pointer.secondary_down(),
                i.pointer.middle_down(),
                i.modifiers.ctrl,
                i.raw_scroll_delta.y,
                i.key_pressed(egui::Key::F) || i.key_pressed(egui::Key::F6),
            )
        });
        let delta = response.drag_delta();
        if response.dragged() && delta != egui::Vec2::ZERO {
            match view3d::drag_mode([left, right, middle], ctrl) {
                Some(view3d::DragMode::Rotate) => {
                    self.orbit = self.orbit.dragged(delta, area.size());
                }
                Some(view3d::DragMode::Pan) => self.framing = self.framing.panned(delta),
                None => {}
            }
        }
        if let Some(at) = response.hover_pos().filter(|_| scroll != 0.0) {
            // as in the 2D view: a quarter of the view's height doubles the zoom
            let factor = 2f32.powf(scroll / (area.height() / VIEW_ZOOM_QUARTERS));
            self.framing = self.framing.zoomed_about(at - area.center(), factor);
        }
        // a double-click picks the component under it, as in Aspire; off the model it fits
        let fit = area.shrink(ui.spacing().item_spacing.x * 2.0);
        let double = response
            .double_clicked()
            .then(|| response.interact_pointer_pos())
            .flatten();
        let picked = double.and_then(|at| self.component_at_3d(fit, at));
        if let Some(id) = picked {
            if ui.input(|i| i.modifiers.shift) {
                if !self.doc.selected_components.remove(&id) {
                    self.doc.selected_components.insert(id);
                }
            } else {
                self.doc.selected_components = [id].into();
            }
        } else if double.is_some() || (response.hovered() && fit_key) {
            self.framing = view3d::Framing::default();
        }
        self.paint_3d(ui, area);
    }

    /// The topmost shown component at the point of the 3D view's picture `fit` under `at`.
    fn component_at_3d(&mut self, fit: egui::Rect, at: egui::Pos2) -> Option<EntityId> {
        let (job, previewing) = (self.shown_job(), self.simulation.is_some());
        let (orbit, framing, on) = (self.orbit, self.framing, self.display.auto_wrapping);
        let surface = self.surface_3d()?.into_owned();
        let wrap = wrap_for(&job, on, previewing, &surface);
        let (column, row) = view3d::pick(&surface, orbit, framing, fit, wrap, at)?;
        let p = surface.centre(column, row);
        self.doc
            .components
            .iter()
            .rev()
            .filter(|(_, k)| k.visible)
            .find(|(_, k)| k.height_at(p).is_some())
            .map(|(&id, _)| id)
    }

    /// The 3D view's picture: background, block and the overlays the View menu turns on.
    fn paint_3d(&mut self, ui: &egui::Ui, area: egui::Rect) {
        let fit = area.shrink(ui.spacing().item_spacing.x * 2.0);
        let shown = self.display;
        let [r, g, b] = self.shown_job().colour;
        let look = view3d::Look {
            colour: if shown.colour_shaded {
                egui::Color32::from_rgb(r, g, b)
            } else {
                ui.visuals().widgets.inactive.fg_stroke.color
            },
            // the block's sides belong to a preview; the model alone has no excess material
            sides: shown.material_block && (self.simulation.is_some() || self.playback.is_some()),
            light: view3d::Look::light_from(self.orbit, shown.light_follows),
            wrap: None,
        };
        let (orbit, framing) = (self.orbit, self.framing);
        let painter = ui.painter_at(area);
        if shown.shaded_background {
            painter.add(view3d::background(area, ui.visuals()));
        }
        let vectors: Vec<Vec<cut_craft_core::geometry::Point>> = if shown.vectors_3d {
            self.doc
                .vectors
                .iter()
                .filter(|(id, _)| self.doc.shown(**id))
                .map(|(_, path)| path.flatten(cut_craft_core::polygon::TOLERANCE))
                .collect()
        } else {
            vec![]
        };
        let stroke = ui.visuals().widgets.noninteractive.fg_stroke;
        let job = self.shown_job();
        let previewing = self.simulation.is_some() || self.playback.is_some();
        // a playing preview's tool: where its tip is, and its diameter
        let tool_at = self
            .playback
            .as_ref()
            .and_then(cut_craft_core::playback::Playback::tool)
            .map(|(tip, tool)| (tip, tool.diameter));
        let Some(surface) = self.surface_3d() else {
            return;
        };
        let wrap = wrap_for(&job, shown.auto_wrapping, previewing, &surface);
        let look = view3d::Look { wrap, ..look };
        painter.add(view3d::mesh(&surface, orbit, framing, &look, fit));
        if wrap.is_some() {
            // ponytail: the overlays are drawn flat, so they are left off the wrapped view
            return;
        }
        let screen = view3d::screen_of(&surface, orbit, framing, fit, None);
        let local = |p: cut_craft_core::geometry::Point, z: f64| {
            cut_craft_core::glam::DVec3::new(p.x - surface.origin.x, p.y - surface.origin.y, z)
        };
        // a playing preview's tool, an upright cylinder at the tip
        if let Some((tip, diameter)) = tool_at {
            let at = local(tip.truncate(), tip.z);
            let outline = view3d::cylinder(
                &screen,
                at,
                diameter / 2.0,
                diameter * TOOL_HEIGHT_DIAMETERS,
            );
            let selection = ui.visuals().selection;
            painter.add(egui::Shape::convex_polygon(
                outline,
                selection.bg_fill,
                selection.stroke,
            ));
        }
        // each vector flat, in one plane, on top of what it outlines
        for points in &vectors {
            let z = view3d::vector_plane(&surface, points);
            let line = points.iter().map(|&p| screen(local(p, z))).collect();
            painter.add(egui::Shape::line(line, stroke));
        }
        let size = view3d::plan_size(&surface);
        if shown.modeling_plane {
            let corners = [
                (0.0, 0.0),
                (size.x, 0.0),
                (size.x, size.y),
                (0.0, size.y),
                (0.0, 0.0),
            ];
            let line = corners
                .iter()
                .map(|&(x, y)| screen(cut_craft_core::glam::DVec3::new(x, y, 0.0)))
                .collect::<Vec<_>>();
            let dash = ui.spacing().item_spacing.x;
            painter.add(egui::Shape::dashed_line(&line, stroke, dash, dash / 2.0));
        }
        if shown.origin {
            // X, Y and Z from the job's origin, a tenth of the job long, in the usual axis colours
            let o = local(cut_craft_core::geometry::Point::ZERO, 0.0);
            let reach = size.x.max(size.y) / 10.0;
            let width = stroke.width * 2.0;
            for (axis, colour) in [
                (cut_craft_core::glam::DVec3::X, egui::Color32::RED),
                (cut_craft_core::glam::DVec3::Y, egui::Color32::GREEN),
                (cut_craft_core::glam::DVec3::Z, egui::Color32::BLUE),
            ] {
                painter.line_segment([screen(o), screen(o + axis * reach)], (width, colour));
            }
        }
    }

    /// The Toolpaths page: strategies, any form opened from them, and the toolpath list.
    fn toolpaths_page(&mut self, ui: &mut egui::Ui) {
        self.page_header(ui, Page::Toolpaths, "Toolpaths");
        let groups = self.toolbar.toolpath_groups.clone();
        if self
            .form
            .as_ref()
            .is_some_and(|f| f.page == Page::Toolpaths)
        {
            egui::ScrollArea::vertical().show(ui, |ui| self.command_form(ui));
            return;
        }
        // the toolpath list in its own bottom half, as Aspire's, so it never scrolls away
        egui::TopBottomPanel::bottom("toolpath_list")
            .resizable(true)
            .default_height(ui.available_height() / 2.0)
            .show_inside(ui, |ui| self.toolpath_list(ui));
        egui::ScrollArea::vertical().show(ui, |ui| {
            self.tool_groups(ui, &groups, Page::Toolpaths);
        });
    }

    /// The Toolpath List: its buttons, then each toolpath, the selected one highlighted and
    /// a right-click on one opening its menu.
    fn toolpath_list(&mut self, ui: &mut egui::Ui) {
        ui.strong("Toolpath List");
        self.toolpath_list_buttons(ui);
        let actions = [gui::ToolGroup {
            name: "Toolpath List Actions",
            tools: &[
                "recalculate_toolpath",
                "recalculate_all_toolpaths",
                "delete_toolpath",
                "preview_toolpath",
                "preview_all_toolpaths",
                "reset_preview",
            ],
        }];
        self.tool_groups(ui, &actions, Page::Toolpaths);
        if self.doc.toolpaths.is_empty() {
            ui.weak("No toolpaths yet");
        }
        let mut chosen = None;
        egui::ScrollArea::vertical().show(ui, |ui| {
            for (&id, toolpath) in &self.doc.toolpaths {
                let selected = self.selected_toolpath == Some(id);
                let label = ui
                    .selectable_label(selected, &toolpath.name)
                    .on_hover_text(&toolpath.tool.name);
                if label.clicked() {
                    self.selected_toolpath = if selected { None } else { Some(id) };
                }
                // Aspire's toolpath menu acts on the row right-clicked, so it selects it
                if label.secondary_clicked() {
                    self.selected_toolpath = Some(id);
                }
                if let Some(action) = self.popup(&label, || gui::toolpath_menu(id, toolpath)) {
                    chosen = Some(action);
                }
            }
        });
        if let Some(action) = chosen {
            self.invoke(ui.ctx(), &action);
        }
    }

    fn status_bar(&self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.label(&self.status.message);
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if let Some(p) = self.cursor {
                    ui.monospace(format!("X {:8.2}  Y {:8.2} mm", p.x, p.y));
                    ui.separator();
                }
                ui.label(format!("Tool: {}", self.status.current_tool));
            });
        });
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
    rfd::FileDialog::new()
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
}

/// Menu entries, recursing into submenus. Sets `clicked` to the chosen item's action.
fn menu_items(
    ui: &mut egui::Ui,
    items: &[gui::MenuItem],
    menus: &Menus,
    clicked: &mut Option<String>,
) {
    // one line per item: the menu widens to its longest name instead
    ui.style_mut().wrap = Some(false);
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
        fonts.font_data.insert(name.clone(), data);
        fonts
            .families
            .entry(egui::FontFamily::Proportional)
            .or_default()
            .insert(0, name);
    }
    ctx.set_fonts(fonts);
    ctx.style_mut(|style| {
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
    let layer = egui::LayerId::new(egui::Order::Foreground, egui::Id::new("focus_ring"));
    let stroke = ctx.style().visuals.selection.stroke;
    let width = stroke.width * 2.0;
    ctx.layer_painter(layer).rect_stroke(
        rect.expand(width),
        ctx.style().visuals.widgets.inactive.rounding,
        egui::Stroke::new(width, stroke.color),
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
                            .clamp_range(0.0..=f64::INFINITY),
                    );
                    ui.end_row();
                    ui.label("Overlap")
                        .on_hover_text("Grow each unmachined area by this");
                    ui.add(
                        egui::DragValue::new(&mut rest.overlap)
                            .suffix(" mm")
                            .clamp_range(0.0..=f64::INFINITY),
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
            ui.close_menu();
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
        ui.painter()
            .rect(rect, look.rounding, look.weak_bg_fill, look.bg_stroke);
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
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.ui(ctx);
    }
}

impl CutCraftApp {
    /// One frame of the whole window; separate from `update` so tests can drive it headlessly.
    fn ui(&mut self, ctx: &egui::Context) {
        // picks are view state: undo or another sheet can take what they name away
        let doc = &mut self.doc;
        doc.selected_components
            .retain(|id| doc.components.contains_key(id));
        doc.selected_bitmaps
            .retain(|id| doc.bitmaps.contains_key(id));
        if ctx.input(|i| i.viewport().close_requested()) && !self.confirm_discard() {
            ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
        }
        paint_focus_ring(ctx);
        let title = match self.screen {
            Screen::Start => APP_NAME.to_owned(),
            _ => self.session.title(APP_NAME, &self.history),
        };
        if title != self.title {
            ctx.send_viewport_cmd(egui::ViewportCommand::Title(title.clone()));
            self.title = title;
        }
        self.shortcuts(ctx);
        egui::TopBottomPanel::top("menu_bar").show(ctx, |ui| self.menu_bar(ui));
        egui::TopBottomPanel::bottom("status_bar").show(ctx, |ui| self.status_bar(ui));
        self.tool_database_window(ctx);
        self.info_window(ctx);
        self.snap_options_window(ctx);
        self.options_window(ctx);
        match self.screen {
            Screen::Start => {
                egui::SidePanel::left("start_page").show(ctx, |ui| self.start_page(ui));
                egui::CentralPanel::default().show(ctx, Self::splash);
                (self.view_transform, self.cursor) = (None, None);
                return;
            }
            Screen::NewJob => {
                egui::SidePanel::left("design_panel")
                    .default_width(DESIGN_PANEL_WIDTHS * ctx.style().spacing.interact_size.x)
                    .show(ctx, |ui| {
                        egui::TopBottomPanel::bottom("form_buttons")
                            .show_inside(ui, |ui| self.form_buttons(ui));
                        egui::ScrollArea::vertical().show(ui, |ui| self.command_form(ui));
                    });
            }
            Screen::Job => self.job_pages(ctx),
        }
        egui::CentralPanel::default().show(ctx, |ui| {
            self.top_bar(ui);
            ui.separator();
            if let Some(split) = self.split {
                let whole = ui.available_rect_before_wrap();
                let (first, second) = match split {
                    Split::Stacked => whole.split_top_bottom_at_fraction(0.5),
                    Split::SideBySide => whole.split_left_right_at_fraction(0.5),
                };
                ui.allocate_ui_at_rect(first, |ui| {
                    ui.set_clip_rect(first);
                    self.view_2d(ui);
                });
                ui.allocate_ui_at_rect(second, |ui| {
                    ui.set_clip_rect(second);
                    self.view_3d(ui);
                });
                return;
            }
            match self.view {
                View::TwoD => self.view_2d(ui),
                View::ThreeD => {
                    self.view_3d(ui);
                    self.cursor = None;
                }
            }
        });
    }

    /// The top bar's sheet, layer and level drop-downs: a command to apply, if one was
    /// picked; actions to invoke go in `clicked`.
    fn drop_downs(
        &self,
        ui: &mut egui::Ui,
        clicked: &mut Option<&'static str>,
    ) -> Option<(&'static str, Value)> {
        let doc = &self.doc;
        let mut apply = None;
        let edge = ui.spacing().interact_size.y;
        // each drop-down as Aspire's: its icon, then the name with a drop-down arrow
        let drop_down =
            |ui: &mut egui::Ui, icon: &str, name: &str, contents: &mut dyn FnMut(&mut egui::Ui)| {
                if let Some(path) = self.icons.find(icon, "") {
                    ui.add(Icons::image(path).fit_to_exact_size(egui::vec2(edge, edge)));
                }
                egui::ComboBox::from_id_source(icon)
                    .selected_text(name)
                    .show_ui(ui, |ui| contents(ui));
            };
        let sheet = doc
            .sheets
            .get(doc.active_sheet)
            .map_or("", |s| s.name.as_str());
        drop_down(ui, "view/interface_layout.svg", sheet, &mut |ui| {
            bordered(ui, |ui| {
                for (i, s) in doc.sheets.iter().enumerate() {
                    if ui.radio(i == doc.active_sheet, &s.name).clicked() {
                        apply = Some(("activate_sheet", serde_json::json!({ "sheet": i })));
                        ui.close_menu();
                    }
                }
            });
            bordered(ui, |ui| {
                if ui.button("Add New Sheet...").clicked() {
                    *clicked = Some("add_sheet");
                    ui.close_menu();
                }
            });
        });
        let layer = doc.layer(doc.current_layer).map_or("", |l| l.name.as_str());
        drop_down(ui, "view/layers.svg", layer, &mut |ui| {
            bordered(ui, |ui| {
                for l in doc.layers.iter().rev() {
                    if ui.radio(l.id == doc.current_layer, &l.name).clicked() {
                        let params = serde_json::json!({ "layer": l.id });
                        apply = Some(("set_current_layer", params));
                        ui.close_menu();
                    }
                }
            });
            bordered(ui, |ui| {
                if ui.button("Add New Layer...").clicked() {
                    *clicked = Some("add_layer");
                    ui.close_menu();
                }
            });
        });
        // new components go on the first level
        let level = doc.levels.values().next().map_or("", |l| l.name.as_str());
        drop_down(ui, "view/component_levels.svg", level, &mut |ui| {
            bordered(ui, |ui| {
                for (&id, l) in doc.levels.iter().rev() {
                    let mut shown = l.visible;
                    if ui.checkbox(&mut shown, &l.name).changed() {
                        let params = cmd_level_properties::Params {
                            visible: shown,
                            ..cmd_level_properties::Params::of(id, l)
                        };
                        let params = serde_json::to_value(params).expect("params serialize");
                        apply = Some(("level_properties", params));
                    }
                }
            });
        });
        apply
    }

    /// The top bar's geometry, smart and grid snapping toggles, right to left.
    fn snap_toggles(&self, ui: &mut egui::Ui, edge: f32, clicked: &mut Option<&'static str>) {
        let s = &self.snap;
        for (id, on, icon, tip) in [
            (
                "toggle_grid_snapping",
                s.snap_to_grid,
                "grid",
                "Grid Snapping",
            ),
            (
                "toggle_smart_snapping",
                s.smart_snapping,
                "smart",
                "Smart Snapping",
            ),
            (
                "toggle_geometry_snapping",
                s.geometry_snapping,
                "geometry",
                "Geometry Snapping",
            ),
        ] {
            let state = if on { "on" } else { "off" };
            let path = format!("snapping/{icon}_snapping_{state}.svg");
            let button = self.icon_button(&path, "", tip, edge).selected(on);
            if ui.add(button).on_hover_text(tip).clicked() {
                *clicked = Some(id);
            }
        }
    }

    /// The top bar's split and snapping toggles.
    fn top_bar_toggle(&mut self, id: &str) {
        match id {
            "split_stacked" | "split_side_by_side" => {
                let split = if id == "split_stacked" {
                    Split::Stacked
                } else {
                    Split::SideBySide
                };
                self.split = (self.split != Some(split)).then_some(split);
            }
            "toggle_geometry_snapping" => self.snap.geometry_snapping ^= true,
            "toggle_smart_snapping" => self.snap.smart_snapping ^= true,
            _ => self.snap.snap_to_grid ^= true,
        }
    }

    /// Aspire's top bar over the views: 2D / 3D and the split, the sheet, layer and level
    /// drop-downs, and the geometry, smart and grid snapping toggles.
    fn top_bar(&mut self, ui: &mut egui::Ui) {
        let edge = ui.spacing().interact_size.y;
        let (mut clicked, mut apply) = (None::<&str>, None::<(&str, Value)>);
        let height = edge + 2.0 * ui.spacing().button_padding.y;
        let (bar, _) = ui.allocate_exact_size(
            egui::vec2(ui.available_width(), height),
            egui::Sense::hover(),
        );
        // left: the views and their split
        ui.allocate_ui_at_rect(bar, |ui| {
            ui.horizontal_centered(|ui| {
                ui.selectable_value(&mut self.view, View::TwoD, "2D View (F2)");
                ui.selectable_value(&mut self.view, View::ThreeD, "3D View (F3)");
                for (id, icon, tip, split) in [
                    (
                        "split_stacked",
                        "view/split_over_and_under.svg",
                        "2D over 3D",
                        Split::Stacked,
                    ),
                    (
                        "split_side_by_side",
                        "view/split_side_by_side.svg",
                        "2D beside 3D",
                        Split::SideBySide,
                    ),
                ] {
                    let button = self
                        .icon_button(icon, "", tip, edge)
                        .selected(self.split == Some(split));
                    if ui.add(button).on_hover_text(tip).clicked() {
                        clicked = Some(id);
                    }
                }
            });
        });
        // right: snapping
        ui.allocate_ui_at_rect(bar, |ui| {
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                self.snap_toggles(ui, edge, &mut clicked);
            });
        });
        // middle: the side, sheet, layer and level, centred on the bar by last frame's width
        let width_id = ui.id().with("top_bar_middle_width");
        let width: f32 = ui.data(|d| d.get_temp(width_id)).unwrap_or_default();
        let middle = egui::Rect::from_center_size(bar.center(), egui::vec2(width, height));
        let drawn = ui
            .allocate_ui_at_rect(middle, |ui| {
                ui.horizontal_centered(|ui| {
                    if self.doc.job.double_sided.is_some() {
                        let side = if self.doc.bottom_side {
                            "Bottom Side"
                        } else {
                            "Top Side"
                        };
                        if ui.button(side).on_hover_text("Swap Sides").clicked() {
                            clicked = Some("swap_sides");
                        }
                    }
                    apply = self.drop_downs(ui, &mut clicked);
                })
                .response
                .rect
                .width()
            })
            .inner;
        if (drawn - width).abs() > 0.5 {
            ui.data_mut(|d| d.insert_temp(width_id, drawn));
            ui.ctx().request_repaint();
        }
        if let Some((id, params)) = apply
            && let Err(e) = self.apply(id, &params)
        {
            self.status.set_message(&e);
        }
        if let Some(id) = clicked {
            self.invoke(ui.ctx(), id);
        }
    }

    /// The job the views show: while Job Setup is open, the one being set up.
    fn shown_job(&self) -> cut_craft_core::document::Job {
        self.form
            .as_ref()
            .filter(|f| f.id == "job_setup")
            .and_then(|f| self.registry.get(f.id)?.run_json(&self.doc, &f.params).ok())
            .and_then(|change| change.job().copied())
            .unwrap_or(self.doc.job)
    }

    /// Zoom and pan: Aspire's 2D view controls.
    fn view_action(&mut self, ctx: &egui::Context, id: &str) {
        let label = match id {
            "pan" => {
                self.view_tool = Some(ViewTool::Pan);
                "Pan"
            }
            "measure_inspect" => {
                self.view_tool = Some(ViewTool::Measure);
                "Measure"
            }
            "transform" => {
                self.rotating = false;
                if self.view_tool == Some(ViewTool::Transform) {
                    self.view_tool = None;
                    "Select"
                } else {
                    self.view_tool = Some(ViewTool::Transform);
                    "Transform"
                }
            }
            "node_edit" => {
                if self.view_tool == Some(ViewTool::NodeEdit) {
                    self.view_tool = None;
                    "Select"
                } else {
                    self.view_tool = Some(ViewTool::NodeEdit);
                    "Node Edit"
                }
            }
            "zoom_box" => {
                self.view_tool = Some(ViewTool::ZoomBox);
                "Zoom Box"
            }
            "zoom_selected" => {
                let margin = ctx.style().spacing.interact_size.y;
                match (self.doc.selection_bounds(), self.view_area) {
                    (Some(bounds), Some(area)) if bounds.size().max_element() > 0.0 => {
                        self.camera = Some(ViewTransform::fit(area, margin, bounds));
                    }
                    _ => self.status.set_message("select something to zoom to"),
                }
                return;
            }
            // zoom to job, zoom to drawing (F6)
            _ => {
                self.camera = None;
                return;
            }
        };
        label.clone_into(&mut self.status.current_tool);
    }

    fn view_2d(&mut self, ui: &mut egui::Ui) {
        let job = self.shown_job();
        let mut canvas = Canvas::new(ui, &job, self.camera);
        let area = canvas.area;
        self.zoom_and_pan(ui, &mut canvas);
        let reach = canvas.pick_tolerance(ui);
        let gesture = canvas.gesture(ui);
        // a guide being dragged takes the drag over
        let gesture = if self.guide_gesture(ui, &canvas, reach) {
            None
        } else {
            self.snapped(ui, gesture, canvas.transform.scale)
        };
        self.track_stroke(gesture);
        let overlay = match (gesture, self.view_tool, self.drawing_tool()) {
            (_, Some(ViewTool::NodeEdit), _) => self.node_overlay(gesture, reach),
            (Some(Gesture::Dragging(drag)), Some(ViewTool::Measure), _) => {
                self.status
                    .set_message(&measured(drag.start, drag.end, self.doc.job.units));
                Overlay::Preview(vec![Path::polyline(&[drag.start, drag.end], false)])
            }
            (Some(Gesture::Dragging(drag)), Some(ViewTool::ZoomBox), _) => Overlay::Window {
                start: drag.start,
                end: drag.end,
                crossing: false,
            },
            (_, Some(ViewTool::Pan), _) => Overlay::None,
            (Some(Gesture::Dragging(drag)), Some(ViewTool::Transform), _) => {
                self.transform_overlay(ui, drag, reach)
            }
            (Some(Gesture::Dragging(drag)), _, Some(id)) => {
                Overlay::Preview(self.preview(id, drag))
            }
            (Some(Gesture::Dragging(drag)), _, None) => Overlay::Window {
                start: drag.start,
                end: drag.end,
                crossing: drag.end.x < drag.start.x,
            },
            _ => Overlay::None,
        };
        canvas.paint(
            ui,
            &self.doc,
            &overlay,
            self.selected_toolpath,
            &self.display,
        );
        // while Job Setup is open a red square marks the datum, as in Aspire
        if self.form.as_ref().is_some_and(|f| f.id == "job_setup") {
            let edge = ui.spacing().interact_size.y / 2.0;
            let at = canvas.transform.to_screen(job.offset);
            ui.painter_at(area).rect_filled(
                egui::Rect::from_center_size(at, egui::vec2(edge, edge)),
                0.0,
                ui.visuals().error_fg_color,
            );
        }
        if self.view_tool == Some(ViewTool::Transform) {
            self.paint_handles(ui, &canvas);
        }
        canvas.rulers(ui, job.units);
        self.view_menu(ui, &canvas, reach);
        self.view_buttons(ui, area);
        self.view_transform = Some(canvas.transform);
        self.view_area = Some(area);
        self.cursor = canvas.cursor();
        match (gesture, self.view_tool, self.drawing_tool()) {
            (Some(Gesture::Dropped { drag, .. }), Some(ViewTool::ZoomBox), _) => {
                let size = drag.end - drag.start;
                if size.x.abs() > 0.0 && size.y.abs() > 0.0 {
                    let margin = ui.spacing().interact_size.y;
                    self.camera = Some(ViewTransform::fit(
                        area,
                        margin,
                        Rect::new(drag.start, drag.end),
                    ));
                }
                self.view_tool = None;
                "Select".clone_into(&mut self.status.current_tool);
            }
            (Some(Gesture::Dropped { drag, shift }), Some(ViewTool::NodeEdit), _) => {
                self.node_drop(drag, shift, reach);
            }
            (_, Some(ViewTool::Measure), _) => self.measure(gesture, reach),
            (Some(Gesture::Click { at, shift }), Some(ViewTool::NodeEdit), _) => {
                self.click_select(at, shift, reach);
            }
            (gesture, Some(ViewTool::Transform), _) => self.transform_gesture(ui, gesture, reach),
            (Some(Gesture::Dropped { drag, .. }), None, Some(id)) => self.draw(id, drag),
            (Some(Gesture::Dropped { drag, shift }), None, None) => {
                let ids = self
                    .doc
                    .window(Rect::new(drag.start, drag.end), drag.end.x < drag.start.x);
                self.select(ids, shift);
            }
            (Some(Gesture::Click { at, shift }), None, None) => {
                self.click_select(at, shift, canvas.pick_tolerance(ui));
            }
            _ => {}
        }
    }

    /// Guides: dragged out of a ruler (top: across, left: up and down), dragged along when
    /// unlocked, and dragged back onto a ruler to delete them. True while one is dragged.
    fn guide_gesture(&mut self, ui: &egui::Ui, canvas: &Canvas, reach: f64) -> bool {
        if canvas.drag_started()
            && let Some(origin) = ui.input(|i| i.pointer.press_origin())
        {
            let at = canvas.transform.to_job(origin);
            let selecting = self.view_tool.is_none() && self.drawing_tool().is_none();
            self.guide_drag = match canvas.ruler_at(ui, origin) {
                Some(vertical) => Some((vertical, None)),
                None if selecting => self
                    .doc
                    .guides
                    .iter()
                    .position(|g| {
                        let along = if g.vertical { at.x } else { at.y };
                        !g.locked && (along - g.at).abs() <= reach
                    })
                    .map(|i| (self.doc.guides[i].vertical, Some(i))),
                None => None,
            };
        }
        let Some((vertical, index)) = self.guide_drag else {
            return false;
        };
        let Some(pointer) = ui.input(|i| i.pointer.latest_pos()) else {
            return true;
        };
        let at = canvas.transform.to_job(pointer);
        let along = if vertical { at.x } else { at.y };
        if !canvas.drag_stopped() {
            let screen = canvas.transform.to_screen(Point::new(along, along));
            let area = canvas.area;
            let line = if vertical {
                [
                    egui::pos2(screen.x, area.top()),
                    egui::pos2(screen.x, area.bottom()),
                ]
            } else {
                [
                    egui::pos2(area.left(), screen.y),
                    egui::pos2(area.right(), screen.y),
                ]
            };
            ui.painter_at(area)
                .line_segment(line, ui.visuals().selection.stroke);
            return true;
        }
        self.guide_drag = None;
        let onto_ruler = canvas.ruler_at(ui, pointer).is_some();
        let mut guides = self.doc.guides.clone();
        match (index, onto_ruler) {
            (Some(i), true) => {
                guides.remove(i);
            }
            (Some(i), false) => guides[i].at = along,
            (None, false) => guides.push(if vertical {
                cut_craft_core::document::Guide::vertical(along)
            } else {
                cut_craft_core::document::Guide::horizontal(along)
            }),
            (None, true) => {}
        }
        self.set_guides(guides);
        true
    }

    /// Replaces the guides, undoably.
    fn set_guides(&mut self, guides: Vec<cut_craft_core::document::Guide>) {
        let mut c = Change::builder(&self.doc);
        c.set_guides(guides);
        self.record(Ok(c.finish()), "Guides changed");
    }

    /// Transform mode while dragging: the selection as the handle would leave it, else the
    /// selection window.
    fn transform_overlay(&self, ui: &egui::Ui, drag: Drag, reach: f64) -> Overlay {
        if let Some(t) = self.handle_drag(ui, drag, reach) {
            let doc = &self.doc;
            let moved = doc
                .selection
                .iter()
                .map(|id| doc.vectors[id].transformed(&t));
            return Overlay::Preview(moved.collect());
        }
        Overlay::Window {
            start: drag.start,
            end: drag.end,
            crossing: drag.end.x < drag.start.x,
        }
    }

    /// Transform mode: a drag from a handle transforms the selection, any other drag
    /// selects by window; a click on the middle turns rotation on or off, elsewhere selects.
    fn transform_gesture(&mut self, ui: &egui::Ui, gesture: Option<Gesture>, reach: f64) {
        match gesture {
            Some(Gesture::Dropped { drag, shift }) => {
                if let Some(t) = self.handle_drag(ui, drag, reach) {
                    let change = Change::transform_selected(&self.doc, |_| t);
                    self.record(change, "Transformed");
                } else {
                    let window = Rect::new(drag.start, drag.end);
                    let ids = self.doc.window(window, drag.end.x < drag.start.x);
                    self.select(ids, shift);
                }
            }
            Some(Gesture::Click { at, shift }) => {
                let on_middle = self
                    .doc
                    .selection_bounds()
                    .and_then(|b| handles::pick(b, self.rotating, at, reach))
                    == Some(Handle::Move);
                if on_middle {
                    self.rotating ^= true;
                } else {
                    self.click_select(at, shift, reach);
                }
            }
            _ => {}
        }
    }

    /// The transform a drag in Transform mode makes, if it started on one of the selection's
    /// handles.
    fn handle_drag(&self, ui: &egui::Ui, drag: Drag, reach: f64) -> Option<DAffine2> {
        let bounds = self.doc.selection_bounds()?;
        let handle = handles::pick(bounds, self.rotating, drag.start, reach)?;
        let keys = ui.input(|i| handles::Keys {
            alt: i.modifiers.alt,
            shift: i.modifiers.shift,
        });
        Some(handles::transform(
            bounds, handle, drag.start, drag.end, keys,
        ))
    }

    /// Transform mode's handles on the selection: white squares scale, black ones rotate,
    /// and a cross in the middle moves.
    fn paint_handles(&self, ui: &egui::Ui, canvas: &Canvas) {
        let Some(bounds) = self.doc.selection_bounds() else {
            return;
        };
        let painter = ui.painter_at(canvas.area);
        let visuals = ui.visuals();
        let stroke = visuals.selection.stroke;
        let half = ui.spacing().interact_size.y * view2d::NODE_HANDLE_SHARE / 2.0;
        for handle in handles::handles(self.rotating) {
            let at = canvas
                .transform
                .to_screen(handles::position(bounds, handle));
            let square = egui::Rect::from_center_size(at, egui::vec2(half, half) * 2.0);
            match handle {
                Handle::Move => {
                    for d in [egui::vec2(half, 0.0), egui::vec2(0.0, half)] {
                        painter.line_segment([at - d, at + d], stroke);
                    }
                }
                Handle::Scale { .. } => {
                    painter.rect_filled(square, 0.0, visuals.extreme_bg_color);
                    painter.rect_stroke(square, 0.0, stroke);
                }
                Handle::Rotate { .. } => {
                    painter.rect_filled(square, 0.0, visuals.text_color());
                }
            }
        }
    }

    /// Aspire's view buttons over the top right of the 2D view, below the ruler.
    fn view_buttons(&mut self, ui: &mut egui::Ui, area: egui::Rect) {
        let row = ui.spacing().interact_size.y;
        let edge = TOOL_ICON_ROWS * row;
        let gap = ui.spacing().item_spacing;
        let room = egui::Rect::from_min_max(
            area.min + egui::vec2(row, row) + gap,
            egui::pos2(area.max.x - gap.x, area.max.y),
        );
        let in_hand = |id: &str| match self.view_tool {
            Some(ViewTool::Pan) => id == "pan",
            Some(ViewTool::ZoomBox) => id == "zoom_box",
            Some(ViewTool::NodeEdit) => id == "node_edit",
            Some(ViewTool::Measure) => id == "measure_inspect",
            Some(ViewTool::Transform) => id == "transform",
            None => false,
        };
        let mut clicked = None;
        // Aspire's toolpath drawing toggles over the top left of the 2D view
        ui.allocate_ui_at_rect(room, |ui| {
            ui.with_layout(Layout::left_to_right(Align::Min), |ui| {
                let shown = self.display.previews_2d;
                let drawing = if shown {
                    "view/2d_toolpath_drawing_on.svg"
                } else {
                    "view/2d_toolpath_drawing_off.svg"
                };
                for (id, icon, tip, on) in [
                    (
                        "draw_2d_previews",
                        drawing,
                        "Draw 2D Toolpath Previews",
                        shown,
                    ),
                    (
                        "make_2d_previews_solid",
                        "view/solid_2d_toolpath_drawing.svg",
                        "Make 2D Previews Solid",
                        self.display.solid_previews,
                    ),
                ] {
                    let button = self.icon_button(icon, "", tip, edge).selected(on);
                    if ui.add(button).on_hover_text(tip).clicked() {
                        clicked = Some(id);
                    }
                }
            });
        });
        ui.allocate_ui_at_rect(room, |ui| {
            // laid out from the right, so the first button is the leftmost
            ui.with_layout(Layout::right_to_left(Align::Min), |ui| {
                for (id, label, icon) in VIEW_BUTTONS.into_iter().rev() {
                    let button = self
                        .icon_button(icon, id, label, edge)
                        .selected(in_hand(id));
                    let response = ui
                        .add_enabled(self.available(id), button)
                        .on_hover_text(label)
                        .on_disabled_hover_text(format!("{label}: select something first"));
                    if response.clicked() {
                        clicked = Some(id);
                    }
                }
            });
        });
        if let Some(id) = clicked {
            self.invoke(ui.ctx(), id);
        }
    }

    /// The open form's command, if it can be drawn with the mouse.
    fn drawing_tool(&self) -> Option<&'static str> {
        let form = self.form.as_ref()?;
        self.registry
            .get(form.id)
            .filter(|c| c.interactive())
            .map(cut_craft_core::DynCommand::id)
    }

    /// Measure / Inspect: a finished drag measures between its ends, a click inspects a vector.
    fn measure(&mut self, gesture: Option<Gesture>, reach: f64) {
        match gesture {
            Some(Gesture::Dropped { drag, .. }) => {
                self.status
                    .set_message(&measured(drag.start, drag.end, self.doc.job.units));
            }
            Some(Gesture::Click { at, .. }) => {
                let message = self.doc.pick(at, reach).map_or_else(
                    || "click a vector, or drag between two points".to_owned(),
                    |id| inspected(&self.doc.vectors[&id], at, reach, self.doc.job.units),
                );
                self.status.set_message(&message);
            }
            _ => {}
        }
    }

    /// The 2D view's right-click menu: a node's while node editing, else the selection's.
    fn view_menu(&mut self, ui: &egui::Ui, canvas: &Canvas, reach: f64) {
        let pressed = ui
            .input(|i| i.pointer.press_origin())
            .map(|q| canvas.transform.to_job(q));
        let node_target = pressed
            .filter(|_| self.view_tool == Some(ViewTool::NodeEdit))
            .and_then(|at| self.node_target(at, reach));
        let selection_menu = || match node_target {
            Some(target) => gui::node_menu(&target),
            None => self.menus.selection_menu(&self.doc),
        };
        if let Some(action) = self.popup(canvas.response(), selection_menu) {
            self.invoke(ui.ctx(), &action);
        }
    }

    /// Node handles on the selected vectors, and the dragged node's vector as it would be.
    fn node_overlay(&self, gesture: Option<Gesture>, reach: f64) -> Overlay {
        // the dragged node's vector as it would be, drawn over the old one
        let moving = match gesture {
            Some(Gesture::Dragging(drag)) => self
                .node_target(drag.start, reach)
                .and_then(|t| {
                    let path = &self.doc.vectors[&t.vector];
                    let k = cut_craft_core::nodes::node_near(path, drag.start, reach)?;
                    Some(cut_craft_core::nodes::move_node(path, k, drag.end))
                })
                .into_iter()
                .collect(),
            _ => vec![],
        };
        let at = self
            .doc
            .selection
            .iter()
            .flat_map(|id| cut_craft_core::nodes::nodes(&self.doc.vectors[id]))
            .collect();
        Overlay::Nodes { at, moving }
    }

    /// A drag from a node moves it; any other drag selects, as with the Select tool.
    fn node_drop(&mut self, drag: Drag, shift: bool, reach: f64) {
        if let Some(params) = self.node_move(drag, reach) {
            if let Err(e) = self.apply("move_node", &params) {
                self.status.set_message(&e);
            }
        } else {
            let ids = self
                .doc
                .window(Rect::new(drag.start, drag.end), drag.end.x < drag.start.x);
            self.select(ids, shift);
        }
    }

    /// A selected, editable vector with a node or span within `reach` of `at`.
    fn node_target(&self, at: Point, reach: f64) -> Option<cut_craft_core::nodes::NodeAt> {
        self.doc
            .selection
            .iter()
            .find(|id| {
                let path = &self.doc.vectors[id];
                self.doc.editable(**id)
                    && (cut_craft_core::nodes::node_near(path, at, reach).is_some()
                        || cut_craft_core::nodes::span_near(path, at, reach).is_some())
            })
            .map(|&vector| cut_craft_core::nodes::NodeAt {
                vector,
                at: cut_craft_core::params::Xy::new(at.x, at.y),
                reach,
            })
    }

    /// Move Node's params when `drag` starts on a selected vector's node.
    fn node_move(&self, drag: Drag, reach: f64) -> Option<Value> {
        let target = self.node_target(drag.start, reach)?;
        cut_craft_core::nodes::node_near(&self.doc.vectors[&target.vector], drag.start, reach)?;
        let to = cut_craft_core::params::Xy::new(drag.end.x, drag.end.y);
        Some(serde_json::json!({ "from": target, "to": to }))
    }

    /// Paths `id` would create for `drag`, by running the command itself.
    fn preview(&self, id: &str, drag: Drag) -> Vec<Path> {
        let (Some(cmd), Some(form)) = (self.registry.get(id), &self.form) else {
            return vec![];
        };
        cmd.drag_params(drag, &self.with_stroke(id, &form.params))
            .and_then(|params| cmd.run_json(&self.doc, &params))
            .map(|change| change.added().cloned().collect())
            .unwrap_or_default()
    }

    /// The Help windows, Toolpath Summary and Snapping Options.
    fn open_window(&mut self, id: &str) {
        self.info = match id {
            "keyboard_shortcuts" => Some(Info::Shortcuts),
            "third_party_licenses" => Some(Info::Licenses),
            "toolpath_summary" => Some(Info::Summary),
            "about_cut_craft" => Some(Info::About),
            _ => {
                self.snap_window = true;
                return;
            }
        };
    }

    /// Drawing and node drags snap to the snap points (Shift holds snapping off).
    fn snapped(&self, ui: &egui::Ui, gesture: Option<Gesture>, scale: f32) -> Option<Gesture> {
        let snapping = self.drawing_tool().is_some() || self.view_tool == Some(ViewTool::NodeEdit);
        if !snapping || ui.input(|i| i.modifiers.shift) {
            return gesture;
        }
        let reach = self.snap.snapping_radius / f64::from(scale);
        let snap = |mut drag: Drag| {
            drag.start = self.snap.snap(&self.doc, drag.start, reach);
            drag.end = self.snap.snap(&self.doc, drag.end, reach);
            drag
        };
        gesture.map(|g| match g {
            Gesture::Dragging(drag) => Gesture::Dragging(snap(drag)),
            Gesture::Dropped { drag, shift } => Gesture::Dropped {
                drag: snap(drag),
                shift,
            },
            click @ Gesture::Click { .. } => click,
        })
    }

    /// Edit ▸ Options: Aspire's sections; a change is saved and applied at once.
    fn options_window(&mut self, ctx: &egui::Context) {
        let mut open = self.options_window;
        let schema = serde_json::to_value(schemars::schema_for!(crate::options::Options))
            .expect("a schema serialises");
        let fields = crate::form::fields(&schema);
        let mut values = serde_json::to_value(&self.options).expect("options serialise");
        let mut changed = None;
        egui::Window::new("Options")
            .open(&mut open)
            .collapsible(false)
            .show(ctx, |ui| {
                let look = crate::form::Look {
                    units: self.doc.job.units,
                    picture: &|_| None,
                    material: crate::form::material(&self.doc, self.doc.job.units),
                };
                if crate::form::show(ui, &fields, &mut values, &look) {
                    changed = serde_json::from_value::<crate::options::Options>(values).ok();
                }
            });
        self.options_window = open;
        if let Some(options) = changed {
            options.apply_to(&mut self.display);
            self.recent.truncate(options.recent_count());
            if let Err(e) = options.save() {
                self.status.set_message(&e);
            }
            self.options = options;
        }
    }

    /// Snapping Options: what drawing and node drags snap to.
    fn snap_options_window(&mut self, ctx: &egui::Context) {
        let mut open = self.snap_window;
        let schema = serde_json::to_value(schemars::schema_for!(cut_craft_core::snap::SnapOptions))
            .expect("a schema serialises");
        let fields = crate::form::fields(&schema);
        let mut values = serde_json::to_value(&self.snap).expect("options serialise");
        let units = self.doc.job.units;
        egui::Window::new("Snapping Options")
            .open(&mut open)
            .collapsible(false)
            .show(ctx, |ui| {
                let picture = |_: &str| None;
                let look = crate::form::Look {
                    units,
                    picture: &picture,
                    material: crate::form::material(&self.doc, units),
                };
                if crate::form::show(ui, &fields, &mut values, &look)
                    && let Ok(options) = serde_json::from_value(values)
                {
                    self.snap = options;
                }
            });
        self.snap_window = open;
    }

    /// The wheel zooms about the cursor; the middle button (or the Pan tool) pans.
    fn zoom_and_pan(&mut self, ui: &egui::Ui, canvas: &mut Canvas) {
        let area = canvas.area;
        // the wheel zooms about the cursor: a quarter of the view's height doubles the zoom
        if let Some(at) = canvas.hover() {
            let scroll = ui.input(|i| i.raw_scroll_delta.y);
            if scroll != 0.0 {
                let factor = 2f32.powf(scroll / (area.height() / VIEW_ZOOM_QUARTERS));
                canvas.transform = canvas.transform.zoomed_about(at, factor);
                self.camera = Some(canvas.transform);
            }
        }
        let pan = match self.view_tool {
            Some(ViewTool::Pan) => canvas.dragged_by(egui::PointerButton::Primary),
            _ => None,
        }
        .or_else(|| canvas.dragged_by(egui::PointerButton::Middle));
        if let Some(by) = pan {
            canvas.transform = canvas.transform.panned(by);
            self.camera = Some(canvas.transform);
        }
    }

    /// Remembers where the pointer goes while dragging, for stroke tools.
    fn track_stroke(&mut self, gesture: Option<Gesture>) {
        if let Some(Gesture::Dragging(drag)) = gesture {
            // a new drag starts a new stroke
            if self.stroke.first() != Some(&drag.start) {
                self.stroke = vec![drag.start];
            }
            if self.stroke.last() != Some(&drag.end) {
                self.stroke.push(drag.end);
            }
        }
    }

    /// A stroke tool's params with the points dragged through so far.
    fn with_stroke(&self, id: &str, params: &Value) -> Value {
        let mut params = params.clone();
        if STROKE_TOOLS.contains(&id) {
            let points: Vec<_> = self
                .stroke
                .iter()
                .map(|q| serde_json::json!({ "x": q.x, "y": q.y }))
                .collect();
            params["points"] = Value::from(points);
        }
        params
    }

    /// Finishes a drag with a drawing tool: the form shows the dragged values, then they apply.
    fn draw(&mut self, id: &'static str, drag: Drag) {
        let Some(cmd) = self.registry.get(id) else {
            return;
        };
        let Some(current) = self.form.as_ref().map(|f| self.with_stroke(id, &f.params)) else {
            return;
        };
        self.stroke.clear();
        let result = cmd
            .drag_params(drag, &current)
            .map_err(|e| e.to_string())
            .and_then(|params| {
                if let Some(form) = &mut self.form {
                    form.params.clone_from(&params);
                }
                self.apply(id, &params)
            });
        if let Some(form) = &mut self.form {
            form.error = result.err();
        }
    }

    /// A click in the 2D view: picks the vector under it, else the picture; Shift adds or
    /// takes away.
    fn click_select(&mut self, at: cut_craft_core::geometry::Point, shift: bool, tolerance: f64) {
        let hit = self.doc.pick(at, tolerance);
        // a picture is picked where no vector is: click picks it, Shift-click adds or
        // takes it away
        let picture = hit.is_none().then(|| self.doc.pick_bitmap(at)).flatten();
        match (picture, shift) {
            (Some(b), true) => {
                if !self.doc.selected_bitmaps.remove(&b) {
                    self.doc.selected_bitmaps.insert(b);
                }
            }
            (picture, false) => self.doc.selected_bitmaps = picture.into_iter().collect(),
            (None, true) => {}
        }
        // a component's depth map is picked where no vector or picture is
        let component = (hit.is_none() && picture.is_none())
            .then(|| self.doc.pick_component(at))
            .flatten();
        match (component, shift) {
            (Some(k), true) => {
                if !self.doc.selected_components.remove(&k) {
                    self.doc.selected_components.insert(k);
                }
            }
            (component, false) => self.doc.selected_components = component.into_iter().collect(),
            (None, true) => {}
        }
        if shift {
            // a grouped vector brings or takes its whole group
            let mut ids = self.doc.selection.clone();
            if let Some(id) = hit {
                let members = self.doc.group_members(id);
                if ids.contains(&id) {
                    for m in &members {
                        ids.remove(m);
                    }
                } else {
                    ids.extend(members);
                }
            }
            self.select(ids, false);
        } else {
            self.select(hit, false);
        }
    }

    /// Selection is view state: applied directly, never recorded for undo.
    /// Picking a grouped vector picks its whole group.
    fn select(&mut self, ids: impl IntoIterator<Item = EntityId>, add: bool) {
        let doc = &self.doc;
        let whole = ids.into_iter().flat_map(|id| doc.group_members(id));
        let mut c = Change::builder(doc);
        if add {
            c.select(doc.selection.iter().copied().chain(whole));
        } else {
            c.select(whole);
        }
        self.doc
            .apply(&c.finish())
            .expect("picked ids come from the document");
    }
}

/// Opens the main window and runs until it is closed.
pub fn run(title: &str) {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title(title)
            .with_app_id(APP_ID),
        renderer: eframe::Renderer::Wgpu,
        // light, as Aspire is, whatever the desktop's theme
        follow_system_theme: false,
        default_theme: eframe::Theme::Light,
        ..Default::default()
    };
    eframe::run_native(
        title,
        options,
        Box::new(|cc| {
            egui_extras::install_image_loaders(&cc.egui_ctx);
            readable(&cc.egui_ctx);
            if let Some(rs) = &cc.wgpu_render_state {
                let info = rs.adapter.get_info();
                log::info!(
                    "GPU: {} ({:?}, driver: {})",
                    info.name,
                    info.backend,
                    info.driver
                );
            }
            let mut app = CutCraftApp::new();
            app.load_tools();
            app.options = crate::options::Options::load();
            app.options.apply_to(&mut app.display);
            app.load_recent();
            app.clipart = crate::clipart::Clipart::load();
            Box::new(app)
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

#[cfg(test)]
mod tests {
    use super::*;
    use eframe::egui::{Event, PointerButton, Pos2, RawInput, Rect, vec2};

    /// Drives the real UI headlessly, one frame per call, with synthetic pointer input.
    #[test]
    fn cut_copy_and_paste_vectors() {
        let mut h = Harness::new();
        h.rectangle(10.0, 10.0, 20.0, 10.0);
        let before = h.app.doc.vectors.len();
        assert!(before > 0);
        assert!(!h.app.available("paste"));
        h.app.invoke(&h.ctx, "copy");
        h.app.invoke(&h.ctx, "paste");
        assert_eq!(h.app.doc.vectors.len(), before + h.app.clipboard.len());
        assert_eq!(
            h.app.doc.selection.len(),
            h.app.clipboard.len(),
            "the pasted copies are selected"
        );
        h.app.invoke(&h.ctx, "cut");
        assert_eq!(
            h.app.doc.vectors.len(),
            before,
            "cut takes the selection away"
        );
        h.app.invoke(&h.ctx, "undo");
        assert_eq!(h.app.doc.vectors.len(), before + h.app.clipboard.len());
    }

    #[test]
    fn notes_starting_with_a_dot_open_with_the_job() {
        let dir = std::env::temp_dir().join(format!("cut-craft-notes-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let mut h = Harness::new();
        for (notes, opens) in [(".Read me", true), ("quiet", false)] {
            h.app.doc.notes = notes.into();
            let path = dir.join("notes.ccjob");
            h.app.save_to(&path);
            h.app.form = None;
            assert!(h.app.open_path(&path));
            assert_eq!(
                h.app.form.as_ref().map(|f| f.id) == Some("notes"),
                opens,
                "{notes}"
            );
        }
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn drawing_snaps_to_nearby_corners() {
        let mut h = Harness::new();
        h.rectangle(10.0, 10.0, 20.0, 10.0);
        h.app.invoke(&h.ctx, "draw_circle");
        h.app.invoke(&h.ctx, "snap_options");
        assert!(h.app.snap_window);
        h.frame(vec![]);
        let scale = h.app.view_transform.unwrap().scale;
        let near = 2.0 / f64::from(scale); // two pixels off the corner
        let drag = Drag {
            start: Point::new(30.0 + near, 20.0 - near),
            end: Point::new(50.0, 50.0),
            from_center: false,
            constrain: false,
        };
        let mut snapped = None;
        let ctx = h.ctx.clone();
        let _ = ctx.run(RawInput::default(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                snapped = h.app.snapped(ui, Some(Gesture::Dragging(drag)), scale);
            });
        });
        let Some(Gesture::Dragging(d)) = snapped else {
            panic!("still a drag")
        };
        assert_eq!(d.start, Point::new(30.0, 20.0), "onto the corner");
        assert_eq!(d.end, Point::new(50.0, 50.0), "nothing near");
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
    fn freehand_follows_the_whole_drag() {
        let mut h = Harness::new();
        h.app.invoke(&h.ctx, "draw_freehand");
        let points = [(10.0, 10.0), (20.0, 30.0), (30.0, 10.0), (40.0, 30.0)];
        h.frame(vec![Event::PointerMoved(h.at(10.0, 10.0))]);
        h.press(h.at(10.0, 10.0), true, Modifiers::NONE);
        for (x, y) in points {
            h.frame(vec![Event::PointerMoved(h.at(x, y))]);
        }
        h.press(h.at(40.0, 30.0), false, Modifiers::NONE);
        h.frame(vec![]);
        let drawn = h.app.doc.vectors.values().next().expect("a stroke");
        let b = drawn.bounds();
        assert!(
            b.size().y > 15.0,
            "it went up and down, not straight: {b:?}"
        );
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

    #[test]
    fn a_vendor_tool_database_imports_when_one_is_in_downloads() {
        let mut h = Harness::new();
        let before = h.app.tools.tools.len();
        h.app
            .import_tools(std::path::Path::new("/nonexistent.vtdb"));
        assert_eq!(h.app.tools.tools.len(), before, "nothing added on failure");
        let Some(home) = std::env::var_os("HOME") else {
            return;
        };
        let Some(file) = std::fs::read_dir(std::path::Path::new(&home).join("Downloads"))
            .ok()
            .and_then(|d| {
                d.flatten()
                    .map(|e| e.path())
                    .find(|p| p.extension().is_some_and(|x| x == "vtdb"))
            })
        else {
            return;
        };
        h.app.import_tools(&file);
        assert!(h.app.tools.tools.len() > before + 10, "{}", file.display());
    }

    #[test]
    fn node_editing_drags_a_corner_and_offers_its_menu() {
        let mut h = Harness::new();
        h.rectangle(10.0, 10.0, 20.0, 10.0);
        let id = *h.app.doc.selection.iter().next().unwrap();
        h.app.invoke(&h.ctx, "node_edit");
        assert_eq!(h.app.view_tool, Some(ViewTool::NodeEdit));
        // smart snapping would line the corner up with the other top corner
        h.app.snap.smart_snapping = false;
        let (from, to) = (h.at(30.0, 20.0), h.at(40.0, 25.0));
        h.drag(from, to);
        let corners = cut_craft_core::nodes::nodes(&h.app.doc.vectors[&id]);
        assert!(
            corners
                .iter()
                .any(|q| q.distance(Point::new(40.0, 25.0)) < 0.5),
            "the corner followed the drag: {corners:?}"
        );
        assert_eq!(
            h.app.doc.vectors.len(),
            1,
            "a node drag selects nothing new"
        );
        let target = h
            .app
            .node_target(Point::new(10.0, 10.0), 1.0)
            .expect("a node there");
        let menu = gui::node_menu(&target);
        let delete = leaf(&menu, &["Delete Point"]);
        h.app.invoke(&h.ctx, &delete.action);
        assert_eq!(
            cut_craft_core::nodes::nodes(&h.app.doc.vectors[&id]).len(),
            3
        );
        h.app.invoke(&h.ctx, "node_edit");
        assert_eq!(h.app.view_tool, None, "N toggles it off");
    }

    #[test]
    fn a_template_opens_as_a_new_unsaved_job() {
        let dir = std::env::temp_dir().join(format!("cut-craft-template-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("plate.ccjobt");
        let mut h = Harness::new();
        h.rectangle(10.0, 10.0, 20.0, 10.0);
        std::fs::write(&path, cut_craft_core::file::save_job(&h.app.doc)).unwrap();
        let mut fresh = Harness::new();
        assert!(fresh.app.open_path(&path));
        assert_eq!(fresh.app.doc.vectors.len(), h.app.doc.vectors.len());
        assert_eq!(fresh.app.session.path, None, "saving asks for a new name");
        assert!(!fresh.app.recent.contains(&path));
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn measuring_reports_distances_and_spans() {
        use cut_craft_core::document::Units;
        let m = measured(Point::ZERO, Point::new(3.0, 4.0), Units::Mm);
        assert_eq!(m, "Distance 5.000 mm, angle 53.13°, X 3.000 mm, Y 4.000 mm");
        let square = cut_craft_core::testing::square(0.0, 0.0, 10.0);
        let i = inspected(&square, Point::new(5.0, 0.0), 0.5, Units::Mm);
        assert_eq!(
            i,
            "Line, length 10.000 mm; vector perimeter 40.000 mm, area 100.000 mm²"
        );
    }

    #[test]
    fn times_read_as_hours_minutes_and_seconds() {
        assert_eq!(hms(0.0), "0:00:00");
        assert_eq!(hms(61.5), "1:01:30");
    }

    #[test]
    fn help_items_open_their_windows() {
        let mut h = Harness::new();
        for (id, info) in [
            ("about_cut_craft", Info::About),
            ("keyboard_shortcuts", Info::Shortcuts),
            ("third_party_licenses", Info::Licenses),
            ("toolpath_summary", Info::Summary),
        ] {
            h.app.invoke(&h.ctx, id);
            assert_eq!(h.app.info, Some(info));
        }
    }

    #[test]
    fn a_toolpath_preview_becomes_a_component() {
        let mut h = Harness::new();
        h.rectangle(10.0, 10.0, 50.0, 50.0);
        let params = h
            .app
            .registry
            .get("pocket_toolpath")
            .unwrap()
            .initial_params(&h.app.doc);
        h.app.apply("pocket_toolpath", &params).unwrap();
        assert!(!h.app.available("create_component_from_toolpath_preview"));
        h.app.invoke(&h.ctx, "preview_all_toolpaths");
        h.app
            .invoke(&h.ctx, "create_component_from_toolpath_preview");
        let made = h.app.doc.components.values().next().expect("a component");
        let thickness = h.app.doc.job.thickness;
        let (lo, hi) = made.height_range().unwrap();
        assert!((hi - thickness).abs() < 1e-9 && lo < hi, "{lo} {hi}");
    }

    #[test]
    fn clicking_a_picture_selects_it_and_delete_removes_it() {
        use cut_craft_core::bitmap::Bitmap;
        use cut_craft_core::geometry::{Point, Rect};
        let mut h = Harness::new();
        let mut c = Change::builder(&h.app.doc);
        let rect = Rect::new(Point::new(20.0, 20.0), Point::new(120.0, 120.0));
        let id = c.add_bitmap(Bitmap {
            name: "b".into(),
            columns: 1,
            rows: 1,
            grey: vec![0],
            rect,
        });
        h.app.record(Ok(c.finish()), "added");
        h.frame(vec![]);
        let at = h.at(70.0, 70.0);
        h.click(at, Modifiers::NONE);
        assert_eq!(h.app.doc.selected_bitmaps, [id].into());
        h.click(h.at(5.0, 5.0), Modifiers::NONE);
        assert!(
            h.app.doc.selected_bitmaps.is_empty(),
            "clicking elsewhere lets it go"
        );
        h.click(at, Modifiers::NONE);
        h.app.invoke(&h.ctx, "delete");
        assert!(h.app.doc.bitmaps.is_empty());
    }

    #[test]
    fn clicking_one_of_a_group_picks_the_group() {
        let mut h = Harness::new();
        h.rectangle(10.0, 10.0, 20.0, 20.0);
        h.rectangle(60.0, 10.0, 20.0, 20.0);
        h.app.invoke(&h.ctx, "select_all_vectors");
        h.app.invoke(&h.ctx, "group");
        h.click(h.at(5.0, 5.0), Modifiers::NONE);
        assert!(h.app.doc.selection.is_empty());
        h.click(h.at(10.0, 20.0), Modifiers::NONE);
        assert_eq!(h.app.doc.selection.len(), 2, "the edge of one picks both");
        h.click(h.at(10.0, 20.0), Modifiers::SHIFT);
        assert!(
            h.app.doc.selection.is_empty(),
            "Shift-click takes the group away"
        );
        h.click(h.at(10.0, 20.0), Modifiers::NONE);
        h.app.invoke(&h.ctx, "ungroup");
        h.click(h.at(10.0, 20.0), Modifiers::NONE);
        assert_eq!(h.app.doc.selection.len(), 1, "ungrouped, one at a time");
    }

    struct Harness {
        ctx: egui::Context,
        app: CutCraftApp,
        time: f64,
        /// What the last frame drew.
        shapes: Vec<egui::epaint::ClippedShape>,
    }

    impl Harness {
        /// The app as it starts: the start page, no job open.
        fn at_start() -> Self {
            let mut h = Self {
                ctx: egui::Context::default(),
                app: CutCraftApp::new(),
                time: 0.0,
                shapes: vec![],
            };
            // previews appear cut at once unless a test plays one
            h.app.display.animate_preview = false;
            h.frame(vec![]);
            h.frame(vec![]);
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

        /// One frame with `modifiers` held, as the windowing layer reports them every frame.
        fn frame_holding(&mut self, modifiers: Modifiers, events: Vec<Event>) {
            self.time += 0.1;
            let input = RawInput {
                screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(1280.0, 800.0))),
                time: Some(self.time),
                modifiers,
                events,
                ..Default::default()
            };
            self.shapes = self.ctx.run(input, |ctx| self.app.ui(ctx)).shapes;
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

        /// The middle of `text` where the last frame drew it.
        fn text_at(&self, text: &str) -> Option<Pos2> {
            self.shapes.iter().find_map(|clipped| match &clipped.shape {
                egui::Shape::Text(t) if t.galley.text() == text => {
                    Some(t.pos + t.galley.size() / 2.0)
                }
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

    #[test]
    fn dragging_with_the_rectangle_tool_draws_that_rectangle() {
        let mut h = Harness::new();
        h.app.invoke(&h.ctx, "draw_rectangle");
        h.frame(vec![]);
        let (from, to) = (h.at(100.0, 100.0), h.at(300.0, 200.0));
        h.drag(from, to);
        assert_eq!(h.app.doc.vectors.len(), 1);
        let b = h.app.doc.vectors.values().next().unwrap().bounds();
        assert!(
            near(b.min, Point::new(100.0, 100.0)) && near(b.max, Point::new(300.0, 200.0)),
            "{b:?}"
        );
        assert!(h.app.history.can_undo());
        let width = h.app.form.as_ref().unwrap().params["width"]
            .as_f64()
            .unwrap();
        assert!(
            (width - 200.0).abs() < 2.0,
            "the form shows the dragged size, got {width}"
        );
    }

    #[test]
    fn clicking_selects_without_adding_undo_steps() {
        let mut h = Harness::new();
        h.rectangle(10.0, 10.0, 50.0, 50.0);
        h.click(h.at(500.0, 300.0), Modifiers::NONE);
        assert!(
            h.app.doc.selection.is_empty(),
            "clicking empty space clears the selection"
        );
        h.click(h.at(60.0, 35.0), Modifiers::NONE);
        assert_eq!(
            h.app.doc.selection.len(),
            1,
            "clicking the right edge selects it"
        );
        assert!(h.app.history.undo(&mut h.app.doc));
        assert!(
            h.app.doc.vectors.is_empty(),
            "the last undo step is the draw, not a click"
        );
    }

    #[test]
    fn shift_click_adds_and_removes() {
        let mut h = Harness::new();
        h.rectangle(10.0, 10.0, 50.0, 50.0);
        h.rectangle(100.0, 10.0, 50.0, 50.0);
        h.click(h.at(10.0, 35.0), Modifiers::NONE);
        h.click(h.at(150.0, 35.0), Modifiers::SHIFT);
        assert_eq!(h.app.doc.selection.len(), 2);
        h.click(h.at(10.0, 35.0), Modifiers::SHIFT);
        assert_eq!(h.app.doc.selection.len(), 1);
    }

    #[test]
    fn left_to_right_windows_and_right_to_left_crosses() {
        let mut h = Harness::new();
        h.rectangle(10.0, 10.0, 50.0, 50.0);
        h.rectangle(100.0, 10.0, 50.0, 50.0);
        h.drag(h.at(0.0, 0.0), h.at(120.0, 80.0));
        assert_eq!(
            h.app.doc.selection.len(),
            1,
            "only the first rectangle is fully inside"
        );
        h.drag(h.at(120.0, 80.0), h.at(0.0, 0.0));
        assert_eq!(
            h.app.doc.selection.len(),
            2,
            "crossing also takes the one it touches"
        );
    }

    #[test]
    fn forms_open_with_values_from_the_document() {
        let mut h = Harness::new();
        h.rectangle(10.0, 10.0, 50.0, 20.0);
        h.app.invoke(&h.ctx, "set_size");
        let params = &h.app.form.as_ref().unwrap().params;
        assert_eq!(
            (params["width"].as_f64(), params["height"].as_f64()),
            (Some(50.0), Some(20.0))
        );
    }

    fn cut_min_x(app: &CutCraftApp) -> f64 {
        cut_craft_core::testing::cut_bounds(app.doc.toolpaths.values().next().unwrap())
            .min
            .x
    }

    #[test]
    fn guides_come_out_of_the_rulers_move_and_go_back_in() {
        let mut h = Harness::new();
        h.frame(vec![]);
        let area = h.app.view_area.unwrap();
        let ruler = h.ctx.style().spacing.interact_size.y / 2.0;
        let top_ruler = egui::pos2(area.center().x, area.min.y + ruler);
        h.drag(top_ruler, h.at(100.0, 80.0));
        assert_eq!(h.app.doc.guides.len(), 1, "a guide across the job");
        let g = h.app.doc.guides[0];
        assert!(!g.vertical && (g.at - 80.0).abs() < 0.5, "{g:?}");
        h.drag(h.at(150.0, 80.0), h.at(150.0, 60.0));
        assert!((h.app.doc.guides[0].at - 60.0).abs() < 0.5, "dragged along");
        h.app.invoke(&h.ctx, "lock_all_existing_guides");
        h.drag(h.at(150.0, 60.0), h.at(150.0, 40.0));
        assert!(
            (h.app.doc.guides[0].at - 60.0).abs() < 0.5,
            "a locked guide stays"
        );
        h.app.invoke(&h.ctx, "unlock_all_existing_guides");
        h.drag(h.at(150.0, 60.0), top_ruler);
        assert!(
            h.app.doc.guides.is_empty(),
            "back onto the ruler deletes it"
        );
        h.app.doc.guides = vec![cut_craft_core::document::Guide::vertical(5.0)];
        h.app.invoke(&h.ctx, "delete_all_guides");
        assert_eq!(h.app.doc.guides, [] as [cut_craft_core::document::Guide; 0]);
    }

    #[test]
    fn transform_mode_scales_by_a_corner_and_rotates_after_a_second_click() {
        let mut h = Harness::new();
        h.rectangle(10.0, 10.0, 20.0, 10.0);
        h.app.invoke(&h.ctx, "transform");
        assert_eq!(h.app.view_tool, Some(ViewTool::Transform));
        let (from, to) = (h.at(30.0, 20.0), h.at(50.0, 30.0));
        h.drag(from, to);
        let b = h.app.doc.selection_bounds().unwrap();
        assert!(
            (b.max.x - 50.0).abs() < 0.5 && (b.max.y - 30.0).abs() < 0.5,
            "{b:?}"
        );
        assert!((b.min.x - 10.0).abs() < 1e-6, "the opposite corner stays");
        assert!(!h.app.rotating);
        let middle = h.at(b.center().x, b.center().y);
        h.click(middle, Modifiers::NONE);
        assert!(
            h.app.rotating,
            "a second click on the middle turns rotation on"
        );
    }

    #[test]
    fn a_toolpath_over_a_keep_out_zone_is_warned_about() {
        let mut h = Harness::new();
        h.rectangle(10.0, 10.0, 50.0, 50.0);
        let zone = cut_craft_core::testing::square(55.0, 20.0, 10.0);
        h.app.doc.keep_out = vec![zone];
        h.app.invoke(&h.ctx, "profile_toolpath");
        let params = h.app.form.as_ref().unwrap().params.clone();
        h.app.apply("profile_toolpath", &params).unwrap();
        assert!(
            h.app.status.message.contains("keep-out"),
            "{}",
            h.app.status.message
        );
        h.app.doc.keep_out.clear();
        h.app.apply("profile_toolpath", &params).unwrap();
        assert!(!h.app.status.message.contains("keep-out"));
    }

    #[test]
    fn toolpaths_follow_their_vectors_when_recalculated() {
        let mut h = Harness::new();
        h.rectangle(10.0, 10.0, 50.0, 50.0);
        h.app.invoke(&h.ctx, "profile_toolpath");
        let params = h.app.form.as_ref().unwrap().params.clone();
        h.app.apply("profile_toolpath", &params).unwrap();
        let before = cut_min_x(&h.app);
        let shift =
            serde_json::json!({ "relative": true, "x": 100.0, "y": 0.0, "anchor": "bottom_left" });
        h.app.apply("move", &shift).unwrap();
        h.app.invoke(&h.ctx, "recalculate_all_toolpaths");
        assert!((cut_min_x(&h.app) - before - 100.0).abs() < 1e-6);
        h.app.invoke(&h.ctx, "undo");
        assert!(
            (cut_min_x(&h.app) - before).abs() < 1e-6,
            "recalculating is one undo step"
        );
    }

    #[test]
    fn hiding_a_layer_from_the_panel_is_undoable_and_stops_picking() {
        let mut h = Harness::new();
        h.app.design_tab = DESIGN_TABS.iter().position(|&t| t == "Layers").unwrap();
        h.rectangle(10.0, 10.0, 50.0, 50.0);
        h.app.layer_action(&h.ctx, LayerId(1), "toggle_visible");
        h.frame(vec![]);
        h.click(h.at(60.0, 35.0), Modifiers::NONE);
        assert!(h.app.doc.selection.is_empty());
        h.app.invoke(&h.ctx, "undo");
        assert!(h.app.doc.layers[0].visible);
    }

    #[test]
    fn choosing_a_layer_makes_it_current_without_an_undo_step() {
        let mut h = Harness::new();
        h.app
            .apply("add_layer", &serde_json::json!({ "name": "Top" }))
            .unwrap();
        h.app.layer_action(&h.ctx, LayerId(1), "set_current_layer");
        assert_eq!(h.app.doc.current_layer, LayerId(1));
        h.app.invoke(&h.ctx, "undo");
        assert_eq!(
            h.app.doc.layers.len(),
            1,
            "the last undo step added the layer"
        );
    }

    #[test]
    fn the_layer_menu_acts_on_the_layer_clicked() {
        let mut h = Harness::new();
        h.rectangle(10.0, 10.0, 50.0, 50.0);
        h.app
            .apply("add_layer", &serde_json::json!({ "name": "Top" }))
            .unwrap();
        let layer =
            |i: usize| move |app: &CutCraftApp| gui::layer_menu(&app.doc, &app.doc.layers[i]);
        h.choose(layer(1), &["Move Selection Here"]);
        assert!(
            h.app
                .doc
                .vectors
                .keys()
                .all(|&id| h.app.doc.layer_of(id) == LayerId(2))
        );
        h.choose(layer(0), &["Activate"]);
        assert_eq!(h.app.doc.current_layer, LayerId(1));
        h.choose(layer(0), &["Show", "Only This"]);
        let visible: Vec<bool> = h.app.doc.layers.iter().map(|l| l.visible).collect();
        assert_eq!(visible, [true, false]);
        h.choose(layer(0), &["Lock"]);
        let locked = layer(0)(&h.app);
        assert!(!leaf(&locked, &["Lock"]).is_enabled && leaf(&locked, &["Unlock"]).is_enabled);
        h.choose(layer(1), &["Rename"]);
        let form = h.app.form.take().unwrap();
        assert_eq!(
            (form.id, form.params["name"].as_str()),
            ("layer_properties", Some("Top"))
        );
        h.choose(layer(0), &["Delete", "Empty"]);
        let names: Vec<&str> = h.app.doc.layers.iter().map(|l| l.name.as_str()).collect();
        assert_eq!(names, ["Top"], "the first layer had nothing left on it");
    }

    #[test]
    fn the_toolpath_menu_acts_on_the_toolpath_clicked() {
        let mut h = Harness::new();
        h.rectangle(10.0, 10.0, 50.0, 50.0);
        for _ in 0..2 {
            let params = h.app.registry.get("profile_toolpath").unwrap();
            let params = params.initial_params(&h.app.doc);
            h.app.apply("profile_toolpath", &params).unwrap();
        }
        let toolpath = |i: usize| {
            move |app: &CutCraftApp| {
                let (&id, t) = app.doc.toolpaths.iter().nth(i).unwrap();
                gui::toolpath_menu(id, t)
            }
        };
        let min_x = |app: &CutCraftApp| -> Vec<f64> {
            let bounds = app.doc.toolpaths.values();
            bounds
                .map(|t| cut_craft_core::testing::cut_bounds(t).min.x)
                .collect()
        };
        h.choose(toolpath(0), &["Hide", "This"]);
        let visible: Vec<bool> = h.app.doc.toolpaths.values().map(|t| t.visible).collect();
        assert_eq!(visible, [false, true]);
        let before = min_x(&h.app);
        let shift =
            serde_json::json!({ "relative": true, "x": 100.0, "y": 0.0, "anchor": "bottom_left" });
        h.app.apply("move", &shift).unwrap();
        h.choose(toolpath(1), &["Recalculate", "Visible"]);
        let after = min_x(&h.app);
        assert!(
            (after[0] - before[0]).abs() < 1e-6 && (after[1] - before[1] - 100.0).abs() < 1e-6,
            "only the shown toolpath follows its vectors: {before:?} → {after:?}"
        );
        let second = *h.app.doc.toolpaths.keys().nth(1).unwrap();
        h.choose(toolpath(1), &["Rename"]);
        let form = h.app.form.take().unwrap();
        assert_eq!(form.id, "toolpath_properties");
        assert_eq!(form.params["toolpath"], serde_json::json!(second));
        h.choose(toolpath(1), &["Delete", "Invisible"]);
        assert_eq!(
            h.app.doc.toolpaths.keys().copied().collect::<Vec<_>>(),
            [second]
        );
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
    fn the_selection_menu_runs_edit_tools_on_the_selection() {
        let mut h = Harness::new();
        h.rectangle(10.0, 10.0, 40.0, 40.0);
        h.rectangle(30.0, 30.0, 40.0, 40.0);
        h.app
            .apply("add_layer", &serde_json::json!({ "name": "Top" }))
            .unwrap();
        h.app.invoke(&h.ctx, "select_all_vectors");
        let menu = |app: &CutCraftApp| app.menus.selection_menu(&app.doc);
        h.choose(menu, &["Move to Layer", "Top"]);
        let doc = &h.app.doc;
        assert!(doc.vectors.keys().all(|&id| doc.layer_of(id) == LayerId(2)));
        h.choose(menu, &["Weld"]);
        assert_eq!(h.app.doc.vectors.len(), 1);
        h.choose(menu, &["Undo"]);
        assert_eq!(h.app.doc.vectors.len(), 2);
        h.choose(menu, &["Align", "Left"]);
        let left: Vec<f64> = h
            .app
            .doc
            .vectors
            .values()
            .map(|v| v.bounds().min.x)
            .collect();
        assert!((left[0] - left[1]).abs() < 1e-9, "{left:?}");
        h.choose(menu, &["Cut"]);
        assert!(h.app.doc.vectors.is_empty(), "Cut takes the selection away");
    }

    #[test]
    fn right_clicking_the_view_opens_the_selection_menu() {
        let mut h = Harness::new();
        h.rectangle(10.0, 10.0, 40.0, 40.0);
        h.rectangle(30.0, 30.0, 40.0, 40.0);
        h.app.invoke(&h.ctx, "select_all_vectors");
        h.right_click(h.at(90.0, 90.0));
        let weld = h.text_at("Weld").expect("the selection menu is open");
        h.click(weld, Modifiers::NONE);
        assert_eq!(h.app.doc.vectors.len(), 1, "{}", h.app.status.message);
    }

    #[test]
    fn right_clicking_a_toolpath_selects_it_and_opens_its_menu() {
        let mut h = Harness::new();
        h.rectangle(10.0, 10.0, 50.0, 50.0);
        let params = h.app.registry.get("profile_toolpath").unwrap();
        let params = params.initial_params(&h.app.doc);
        h.app.apply("profile_toolpath", &params).unwrap();
        h.app.invoke(&h.ctx, "machining_layout");
        // the toolpath list's section settles its height over a frame or two
        h.frame(vec![]);
        h.frame(vec![]);
        let (&id, toolpath) = h.app.doc.toolpaths.iter().next().unwrap();
        let row = h
            .text_at(&toolpath.name.clone())
            .expect("the toolpath is listed");
        h.right_click(row);
        assert_eq!(h.app.selected_toolpath, Some(id));
        assert!(h.text_at("Recalculate").is_some(), "its menu is open");
    }

    #[test]
    fn toolpath_forms_take_tools_from_the_database_and_save_them_back() {
        let mut h = Harness::new();
        let path =
            std::env::temp_dir().join(format!("cut-craft-tools-{}/tools.json", std::process::id()));
        h.app.tools_path = Some(path.clone());
        h.app.invoke(&h.ctx, "pocket_toolpath");
        h.app.choose_tool("3 mm End Mill");
        h.frame(vec![]);
        let params = &mut h.app.form.as_mut().unwrap().params;
        assert_eq!(params["tool"]["diameter"], 3.0);
        params["tool"]["name"] = "Mine".into();
        h.app.save_form_tool();
        assert_eq!(h.app.tools.get("Mine").map(|t| t.diameter), Some(3.0));
        assert_eq!(files::load_tools(&path).unwrap(), h.app.tools);
    }

    #[test]
    fn the_tool_database_window_lists_and_deletes_tools() {
        let mut h = Harness::new();
        h.app.invoke(&h.ctx, "tool_database");
        h.frame(vec![]);
        assert!(h.app.tool_window);
        let count = h.app.tools.tools.len();
        h.app.delete_tool(0);
        h.frame(vec![]);
        assert_eq!(h.app.tools.tools.len(), count - 1);
    }

    #[test]
    fn rest_machining_takes_earlier_tools_from_the_database() {
        let mut h = Harness::new();
        h.app.invoke(&h.ctx, "3d_finishing_toolpath");
        h.app.add_earlier_tool("6 mm Ball Nose");
        let form = h.app.form.as_ref().unwrap();
        assert_eq!(form.params["after"][0]["name"], "6 mm Ball Nose");
    }

    #[test]
    fn the_tool_database_window_shows_the_chip_load() {
        let mut h = Harness::new();
        h.app.tools = ToolDatabase {
            tools: vec![Tool::default()],
        };
        h.app.invoke(&h.ctx, "tool_database");
        h.frame(vec![]);
        h.frame(vec![]);
        // 1000 mm/min over 18000 rpm x 2 flutes
        assert!(
            h.text_at("Chip load: 0.028 mm").is_some(),
            "the window shows the feed per tooth"
        );
    }

    #[test]
    fn previewing_all_toolpaths_is_previewing_each_in_turn() {
        let mut h = Harness::new();
        h.rectangle(10.0, 10.0, 50.0, 50.0);
        for id in ["drilling_toolpath", "profile_toolpath"] {
            h.app.invoke(&h.ctx, id);
            let params = h.app.form.as_ref().unwrap().params.clone();
            h.app.apply(id, &params).unwrap();
        }
        h.app.invoke(&h.ctx, "preview_all_toolpaths");
        assert!(h.app.view == View::ThreeD, "previewing shows the 3D view");
        h.frame(vec![]);
        let all = h.app.simulation.clone().expect("a preview");
        h.app.invoke(&h.ctx, "reset_preview");
        assert!(h.app.simulation.is_none());
        for id in h.app.doc.toolpaths.keys().copied().collect::<Vec<_>>() {
            h.app.selected_toolpath = Some(id);
            h.app.invoke(&h.ctx, "preview_toolpath");
        }
        assert_eq!(h.app.simulation, Some(all));
    }

    #[test]
    fn the_3d_view_shows_the_model_until_there_is_a_preview() {
        let mut h = Harness::new();
        h.app.design_tab = DESIGN_TABS.iter().position(|&t| t == "Design").unwrap();
        assert!(
            h.app
                .toolbar
                .modeling_groups
                .iter()
                .any(|g| g.tools.contains(&"create_shape_round"))
        );
        h.rectangle(10.0, 10.0, 50.0, 50.0);
        let shape = h.app.registry.get("create_shape").unwrap().default_params();
        h.app.apply("create_shape", &shape).unwrap();
        h.app.view = View::ThreeD;
        h.frame(vec![]);
        let raised =
            |app: &mut CutCraftApp| app.surface_3d().unwrap().heights.iter().any(|&z| z > 0.0);
        assert!(raised(&mut h.app), "the model");
        let profile = h
            .app
            .registry
            .get("profile_toolpath")
            .unwrap()
            .default_params();
        h.app.apply("profile_toolpath", &profile).unwrap();
        h.app.invoke(&h.ctx, "preview_all_toolpaths");
        h.frame(vec![]);
        assert!(!raised(&mut h.app), "the preview, cut down from the top");
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
    fn the_window_is_titled_cut_craft() {
        let h = Harness::new();
        assert_eq!(h.app.title, "Untitled - Cut Craft");
    }

    #[test]
    fn the_align_objects_group_spaces_by_pick_order() {
        let mut h = Harness::new();
        h.rectangle(100.0, 10.0, 10.0, 10.0);
        let first = *h.app.doc.selection.iter().next().unwrap();
        h.rectangle(10.0, 10.0, 10.0, 10.0);
        h.rectangle(50.0, 10.0, 10.0, 10.0);
        let all: Vec<EntityId> = h.app.doc.vectors.keys().copied().collect();
        h.app.doc.selection = all.into_iter().collect();
        for id in [
            "left_to_right",
            "right_to_left",
            "top_to_bottom",
            "bottom_to_top",
        ] {
            assert!(h.app.available(id), "{id}");
        }
        h.app.invoke(&h.ctx, "left_to_right");
        let left = h.app.doc.vectors[&first].bounds().min.x;
        assert!(
            (left - 10.0).abs() < 1e-6,
            "the first picked goes leftmost: {left}"
        );
        let group = h
            .app
            .toolbar
            .groups
            .iter()
            .find(|g| g.name == "Align Objects")
            .unwrap();
        assert!(group.tools.contains(&"left_to_right") && group.tools.contains(&"align_left"));
    }

    #[test]
    fn the_create_shape_icons_open_it_with_their_profile() {
        let mut h = Harness::new();
        for (id, profile) in [
            ("create_shape_round", "round"),
            ("create_shape_angled", "angular"),
            ("create_shape_smooth", "smooth"),
            ("create_shape_flat", "flat"),
        ] {
            h.app.invoke(&h.ctx, id);
            let form = h.app.form.as_ref().expect("Create Shape's form");
            assert_eq!(
                (form.id, form.params["profile"].as_str()),
                ("create_shape", Some(profile))
            );
        }
    }

    #[test]
    fn edit_toolpath_reopens_its_form_and_replaces_it() {
        let mut h = Harness::new();
        h.rectangle(10.0, 10.0, 50.0, 50.0);
        h.app.invoke(&h.ctx, "profile_toolpath");
        let params = h.app.form.as_ref().unwrap().params.clone();
        h.app.apply("profile_toolpath", &params).unwrap();
        h.app.form = None;
        let id = *h.app.doc.toolpaths.keys().next().unwrap();
        h.app.selected_toolpath = Some(id);
        h.app.doc.selection = cut_craft_core::selection::Selection::new();
        h.app.invoke(&h.ctx, "edit_toolpath");
        let form = h.app.form.as_mut().expect("the toolpath's form");
        assert_eq!((form.id, form.editing), ("profile_toolpath", Some(id)));
        form.params["name"] = Value::from("Edited");
        let params = form.params.clone();
        h.app.apply_form("profile_toolpath", &params).unwrap();
        assert_eq!(h.app.doc.toolpaths.len(), 1, "replaced, not added");
        assert_eq!(h.app.doc.toolpaths[&id].name, "Edited");
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
            h.app.status.message.contains("Clipart library"),
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
    fn the_join_icons_join_two_vectors_their_way() {
        let mut h = Harness::new();
        let line =
            |x0: f64, x1: f64| Path::polyline(&[Point::new(x0, 10.0), Point::new(x1, 10.0)], false);
        let change = Change::swap(&h.app.doc, [], vec![line(0.0, 10.0), line(20.0, 30.0)]);
        h.app.doc.apply(&change).unwrap();
        for id in [
            "join_straight_line",
            "join_smooth_curve",
            "join_move_end_points",
        ] {
            assert!(h.app.available(id), "{id}");
        }
        h.app.invoke(&h.ctx, "join_smooth_curve");
        assert_eq!(h.app.doc.vectors.len(), 1, "{}", h.app.status.message);
        let edit = h
            .app
            .toolbar
            .groups
            .iter()
            .find(|g| g.name == "Edit Objects")
            .unwrap();
        assert!(edit.tools.contains(&"join_move_end_points"));
    }

    #[test]
    fn edit_text_opens_draw_text_on_the_selected_text() {
        let mut h = Harness::new();
        assert!(!h.app.available("edit_text"), "no text selected");
        let params = serde_json::json!({ "text": "HI", "font": "Test Boxes" });
        let mut full = h.app.registry.get("draw_text").unwrap().default_params();
        for (k, v) in params.as_object().unwrap() {
            full[k] = v.clone();
        }
        h.app.apply("draw_text", &full).unwrap();
        assert!(h.app.available("edit_text"));
        h.app.invoke(&h.ctx, "edit_text");
        let form = h.app.form.as_ref().expect("Draw Text's form");
        assert_eq!(
            (form.id, form.params["text"].as_str()),
            ("draw_text", Some("HI"))
        );
    }

    #[test]
    fn a_new_job_is_set_up_over_the_2d_view() {
        let mut h = Harness::at_start();
        h.app.view = View::ThreeD;
        h.app.invoke(&h.ctx, "new");
        assert!(
            h.app.view == View::TwoD,
            "the material shows while it is set up"
        );
    }

    #[test]
    fn it_starts_on_the_start_page_with_the_no_job_menus() {
        let h = Harness::at_start();
        assert_eq!(h.app.screen, Screen::Start);
        assert_eq!(h.app.title, "Cut Craft", "no job, so no file name");
        assert_eq!(
            h.app.menu_names(),
            ["File", "Edit", "Machine", "Toolpaths", "Gadgets", "Help"]
        );
        assert!(
            h.app.view_transform.is_none(),
            "no 2D view before there is a job"
        );
    }

    #[test]
    fn create_a_new_file_goes_through_job_setup_to_the_job() {
        let mut h = Harness::at_start();
        h.app.invoke(&h.ctx, "new");
        h.frame(vec![]);
        assert_eq!(h.app.screen, Screen::NewJob);
        let form = h.app.form.as_mut().expect("the Job Setup form");
        assert_eq!(form.id, "job_setup");
        form.params["size"]["width"] = 300.0.into();
        assert!(
            (h.app.shown_job().width - 300.0).abs() < 1e-9,
            "the 2D view shows the job being set up"
        );
        h.app.job_setup_ok();
        h.frame(vec![]);
        assert_eq!(h.app.screen, Screen::Job);
        assert!(h.app.form.is_none());
        assert!((h.app.doc.job.width - 300.0).abs() < 1e-9);
        assert!(
            !h.app.history.can_undo(),
            "a new job starts with nothing to undo"
        );
        assert_eq!(
            h.app.status.current_tool, "Select",
            "Job Setup is done with"
        );
        assert!(h.app.menu_names().contains(&"Model"));
        assert!(h.app.view_transform.is_some());
    }

    #[test]
    fn the_start_page_says_whats_new_and_links_out() {
        let h = Harness::at_start();
        let heading = format!("What's new in v{}", env!("CARGO_PKG_VERSION"));
        assert!(h.text_at(&heading).is_some(), "{heading}");
        let first = WHATS_NEW.lines().find(|l| !l.trim().is_empty()).unwrap();
        assert!(
            h.text_at(first.trim_start_matches("- ")).is_some(),
            "{first}"
        );
        for (label, url, _) in LINKS {
            assert!(h.text_at(label).is_some(), "{label}");
            assert!(url.starts_with("https://"), "{url}");
        }
    }

    #[test]
    fn cancelling_job_setup_goes_back_to_the_start_page() {
        let mut h = Harness::at_start();
        h.app.invoke(&h.ctx, "new");
        h.app.job_setup_cancel();
        h.frame(vec![]);
        assert_eq!(h.app.screen, Screen::Start);
        assert!(h.app.form.is_none());
    }

    /// The icons are made from Aspire's and not in git; their manifest is.
    fn icon_set() -> std::collections::HashSet<String> {
        let manifest = std::fs::read(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../re/icon_set.json"
        ))
        .expect("re/icon_set.json");
        serde_json::from_slice::<Vec<Value>>(&manifest)
            .unwrap()
            .iter()
            .map(|e| e["path"].as_str().unwrap().to_owned())
            .collect()
    }

    #[test]
    fn start_page_and_job_setup_icons_are_in_the_icon_set() {
        let known = icon_set();
        for (_, _, icon) in STARTUP_TASKS {
            assert!(known.contains(icon), "{icon}");
        }
        assert!(known.contains(RECENT_FILE_ICON));
        // every picture a Job Setup section can show, whatever is chosen
        let fields = form::fields(&commands::registry().get("job_setup").unwrap().schema());
        let mut pictures = 0;
        for f in &fields {
            let Some(template) = &f.icon else { continue };
            let choices = |kind: &form::Kind| match kind {
                form::Kind::Choice(options, _) => options.iter().map(|o| o.value.clone()).collect(),
                _ => vec![],
            };
            let values: Vec<Value> = match &f.kind {
                form::Kind::Choice(..) => choices(&f.kind).into_iter().map(Value::from).collect(),
                form::Kind::Group(inner) => inner
                    .iter()
                    .flat_map(|i| {
                        choices(&i.kind)
                            .into_iter()
                            .map(|c| serde_json::json!({ i.key.as_str(): c }))
                    })
                    .collect(),
                _ => vec![Value::Null],
            };
            for value in values {
                let path = form::icon_path(template, &value);
                if !path.contains('{') {
                    assert!(known.contains(&path), "{} shows {path}", f.key);
                    pictures += 1;
                }
            }
        }
        assert!(
            pictures >= 9,
            "job type, size, Z zero and datum pictures: {pictures}"
        );
    }

    #[test]
    fn closing_the_job_goes_back_to_the_start_page() {
        let mut h = Harness::new();
        h.app.invoke(&h.ctx, "close");
        h.frame(vec![]);
        assert_eq!(h.app.screen, Screen::Start);
    }

    #[test]
    fn saved_and_opened_jobs_are_listed_as_recent_files() {
        let mut h = Harness::new();
        let dir = std::env::temp_dir().join(format!("cut-craft-recent-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        h.app.recent_path = Some(dir.join("recent.json"));
        let job = dir.join("first.ccjob");
        h.app.save_to(&job);
        assert_eq!(h.app.recent, std::slice::from_ref(&job));
        assert_eq!(
            files::load_recent(&dir.join("recent.json")),
            std::slice::from_ref(&job)
        );
        h.app.invoke(&h.ctx, "close");
        h.app.invoke(&h.ctx, "open_recent:0");
        assert_eq!(h.app.screen, Screen::Job);
        assert_eq!(h.app.session.path.as_deref(), Some(job.as_path()));
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn a_recent_file_that_has_gone_is_dropped() {
        let mut h = Harness::at_start();
        h.app.recent = vec![std::env::temp_dir().join("cut-craft-gone.ccjob")];
        h.app.invoke(&h.ctx, "open_recent:0");
        assert_eq!(h.app.screen, Screen::Start);
        assert_eq!(h.app.recent, Vec::<std::path::PathBuf>::new());
        assert!(h.app.status.message.contains("cut-craft-gone.ccjob"));
    }

    /// The middle of the 2D view on screen.
    fn view_middle(h: &Harness) -> Pos2 {
        h.app
            .view_area
            .expect("the 2D view has been drawn")
            .center()
    }

    #[test]
    fn the_wheel_zooms_about_the_cursor() {
        let mut h = Harness::new();
        let at = view_middle(&h) + vec2(40.0, -30.0);
        let (before, under) = {
            let t = h.app.view_transform.unwrap();
            (t.scale, t.to_job(at))
        };
        h.frame(vec![Event::PointerMoved(at)]);
        h.frame(vec![Event::Scroll(vec2(0.0, 120.0))]);
        h.frame(vec![]);
        let t = h.app.view_transform.unwrap();
        assert!(t.scale > before, "scrolling up zooms in");
        assert!(t.to_job(at).distance(under) < 0.01, "about the cursor");
    }

    #[test]
    fn the_middle_button_and_the_pan_tool_pan() {
        let mut h = Harness::new();
        let p = Point::new(100.0, 100.0);
        let before = h.app.view_transform.unwrap().to_screen(p);
        let from = view_middle(&h);
        h.frame(vec![Event::PointerMoved(from)]);
        let middle = |pressed: bool| Event::PointerButton {
            pos: from + vec2(40.0, 10.0) * f32::from(u8::from(!pressed)),
            button: PointerButton::Middle,
            pressed,
            modifiers: Modifiers::NONE,
        };
        h.frame(vec![middle(true)]);
        h.frame(vec![Event::PointerMoved(from + vec2(20.0, 5.0))]);
        h.frame(vec![Event::PointerMoved(from + vec2(40.0, 10.0))]);
        h.frame(vec![middle(false)]);
        h.frame(vec![]);
        let after = h.app.view_transform.unwrap().to_screen(p);
        assert!(
            (after - before - vec2(40.0, 10.0)).length() < 1.0,
            "{before:?} → {after:?}"
        );
        // the Pan tool does the same with the left button, and draws nothing
        h.app.invoke(&h.ctx, "pan");
        let before = h.app.view_transform.unwrap().to_screen(p);
        h.drag(from, from + vec2(-30.0, 0.0));
        let after = h.app.view_transform.unwrap().to_screen(p);
        assert!(
            (after - before - vec2(-30.0, 0.0)).length() < 1.0,
            "{before:?} → {after:?}"
        );
        assert!(h.app.doc.vectors.is_empty() && h.app.doc.selection.is_empty());
    }

    #[test]
    fn zoom_box_zoom_to_selection_and_zoom_to_job() {
        let mut h = Harness::new();
        let fitted = h.app.view_transform.unwrap().scale;
        h.app.invoke(&h.ctx, "zoom_box");
        h.drag(h.at(100.0, 100.0), h.at(200.0, 150.0));
        let boxed = h.app.view_transform.unwrap();
        assert!(boxed.scale > 3.0 * fitted, "the box fills the view");
        assert!(h.app.doc.selection.is_empty(), "zooming selects nothing");
        let area = h.app.view_area.unwrap();
        assert!(area.contains(boxed.to_screen(Point::new(150.0, 125.0))));
        h.rectangle(400.0, 300.0, 20.0, 10.0);
        h.app.invoke(&h.ctx, "zoom_selected");
        h.frame(vec![]);
        let t = h.app.view_transform.unwrap();
        let (a, b) = (
            t.to_screen(Point::new(400.0, 300.0)),
            t.to_screen(Point::new(420.0, 310.0)),
        );
        assert!(area.contains(a) && area.contains(b));
        assert!(
            (b.x - a.x) > area.width() / 2.0,
            "the selection fills the view"
        );
        h.app.invoke(&h.ctx, "zoom_to_job");
        h.frame(vec![]);
        assert!((h.app.view_transform.unwrap().scale - fitted).abs() < 1e-3);
    }

    #[test]
    fn the_side_tabs_are_aspire_12s_with_drawing_and_modeling_on_design() {
        assert_eq!(
            DESIGN_TABS,
            ["Design", "Sheets", "Layers", "Components", "Clipart"]
        );
        let h = Harness::new();
        let names: Vec<&str> = h
            .app
            .design_groups("Design")
            .iter()
            .map(|g| g.name)
            .collect();
        assert!(
            names.contains(&"Create Vectors") && names.contains(&"Shape Creation"),
            "{names:?}"
        );
        assert!(h.app.design_groups("Components").is_empty());
    }

    #[test]
    fn the_components_tab_is_the_component_list() {
        let mut h = Harness::new();
        h.app.pages = Pages::design_layout();
        h.app.design_tab = DESIGN_TABS.iter().position(|&t| t == "Components").unwrap();
        h.frame(vec![]);
        assert!(h.text_at("No components yet").is_some());
    }

    #[test]
    fn the_view_buttons_sit_at_the_top_right_of_the_2d_view_starting_with_pan() {
        let mut h = Harness::new();
        // text buttons, as where the icons are missing
        h.app.icons = Icons::load(std::path::Path::new("/no/icons"));
        h.frame(vec![]);
        h.frame(vec![]);
        let area = h.app.view_area.unwrap();
        let over_view = |at: &Pos2| area.contains(*at);
        let pan = *h
            .texts_at("Pan")
            .iter()
            .find(|p| over_view(p))
            .expect("Pan over the view");
        let selection = h.text_at("Zoom to Selection").unwrap();
        assert!(
            pan.x > area.center().x && selection.x > pan.x,
            "on the right, Pan first: {pan:?} {selection:?} in {area:?}"
        );
        assert!(pan.y < area.center().y, "along the top: {pan:?}");
        h.click(pan, Modifiers::NONE);
        assert_eq!(h.app.view_tool, Some(ViewTool::Pan));
    }

    #[test]
    fn the_view_buttons_are_the_view_actions() {
        for (id, _, icon) in VIEW_BUTTONS {
            assert!(icon_set().contains(icon), "{icon}");
            assert!(BUILT_INS.contains(&id), "{id}");
        }
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

    #[test]
    fn tab_moves_a_visible_focus_through_the_controls() {
        let mut h = Harness::new();
        h.frame(vec![]);
        assert_eq!(focus_ring(&h.ctx), None, "nothing focused yet");
        h.frame(vec![key(Key::Tab)]);
        h.frame(vec![]);
        let first = focus_ring(&h.ctx).expect("Tab focuses a control, ringed");
        h.frame(vec![key(Key::Tab)]);
        h.frame(vec![]);
        let second = focus_ring(&h.ctx).expect("the next control");
        assert_ne!(first, second, "Tab moves on");
    }

    #[test]
    fn a_preview_plays_pauses_scrubs_and_keeps_what_is_cut() {
        let mut h = Harness::new();
        h.rectangle(10.0, 10.0, 50.0, 50.0);
        h.app.invoke(&h.ctx, "profile_toolpath");
        let params = h.app.form.as_ref().unwrap().params.clone();
        h.app.apply("profile_toolpath", &params).unwrap();
        h.app.form = None;
        h.app.display.animate_preview = true;
        h.app.invoke(&h.ctx, "preview_all_toolpaths");
        assert!(h.app.playing && h.app.playback.is_some(), "it plays");
        assert!(h.app.view == View::ThreeD);
        h.frame(vec![]);
        h.frame(vec![]);
        let along = h.app.playback.as_ref().unwrap().at();
        assert!(along > 0.0, "the tool moves on each frame");
        assert!(h.text_at("Pause").is_some() && h.text_at("Run to Retract").is_some());
        h.click(h.text_at("Pause").unwrap(), Modifiers::NONE);
        assert!(!h.app.playing);
        let paused = h.app.playback.as_ref().unwrap().at();
        h.frame(vec![]);
        assert_eq!(
            h.app.playback.as_ref().unwrap().at(),
            paused,
            "paused stays put"
        );
        h.click(h.text_at("Stop").unwrap(), Modifiers::NONE);
        assert!(
            h.app.playback.is_none() && h.app.simulation.is_some(),
            "kept as far as it went"
        );
    }

    #[test]
    fn any_toolpath_rest_machines_after_the_tools_named_in_its_form() {
        let mut h = Harness::new();
        h.rectangle(10.0, 10.0, 40.0, 40.0);
        h.app.invoke(&h.ctx, "pocket_toolpath");
        // the section, drawn on its own (it is below the fold of the pocket form)
        let ctx = egui::Context::default();
        let mut section = serde_json::json!({});
        let out = ctx.run(RawInput::default(), |ctx| {
            egui::CentralPanel::default()
                .show(ctx, |ui| rest_section(ui, &mut section, &h.app.tools.tools));
        });
        let drawn = |text: &str| {
            out.shapes
                .iter()
                .any(|c| matches!(&c.shape, egui::Shape::Text(t) if t.galley.text() == text))
        };
        assert!(drawn("Rest Machining") && drawn("Add an earlier tool"));
        let mut params = h.app.form.as_ref().unwrap().params.clone();
        h.app.apply("pocket_toolpath", &params).unwrap();
        let whole = h.app.doc.toolpaths.values().last().unwrap().moves.len();
        let mut big = h.app.form.as_ref().unwrap().params["tool"].clone();
        big["diameter"] = Value::from(params["tool"]["diameter"].as_f64().unwrap() * 4.0);
        params["rest"] =
            serde_json::json!({ "after": [big], "minimum_detail": 0.0, "overlap": 0.0 });
        h.app.apply("pocket_toolpath", &params).unwrap();
        let rest = h.app.doc.toolpaths.values().last().unwrap();
        assert!(
            rest.moves.len() < whole,
            "{} of {whole} moves",
            rest.moves.len()
        );
        assert!(
            cut_craft_core::rest::Rest::of(&rest.params).is_some(),
            "kept to recalculate"
        );
    }

    #[test]
    fn the_arrow_keys_nudge_the_selection() {
        let mut h = Harness::new();
        h.rectangle(10.0, 10.0, 20.0, 20.0);
        let square = h.app.snap.grid_spacing;
        let at = |h: &Harness| h.app.doc.selection_bounds().unwrap().min;
        let before = at(&h);
        h.frame(vec![key(Key::ArrowRight)]);
        h.frame(vec![key(Key::ArrowUp)]);
        let moved = at(&h) - before;
        assert!(
            (moved.x - square).abs() < 1e-9 && (moved.y - square).abs() < 1e-9,
            "{moved:?}"
        );
        h.frame(vec![key(Key::ArrowLeft), key(Key::ArrowDown)]);
        assert!((at(&h) - before).length() < 1e-9, "back where it was");
        for (modifiers, share) in [
            (Modifiers::SHIFT, 0.5),
            (Modifiers::SHIFT | Modifiers::COMMAND, 0.1),
        ] {
            let from = at(&h);
            let press = Event::Key {
                key: Key::ArrowRight,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers,
            };
            h.frame_holding(modifiers, vec![press]);
            let moved = (at(&h) - from).x;
            assert!(
                (moved - square * share).abs() < 1e-9,
                "{modifiers:?}: {moved}"
            );
        }
    }

    #[test]
    fn f11_designs_f12_machines_and_switch_layout_swaps() {
        let mut h = Harness::new();
        assert_eq!(h.app.pages, Pages::design_layout());
        h.frame(vec![key(Key::F12)]);
        assert_eq!(h.app.pages, Pages::machining_layout());
        h.frame(vec![key(Key::F11)]);
        assert_eq!(h.app.pages, Pages::design_layout());
        h.app.invoke(&h.ctx, "interface_layout");
        assert_eq!(
            h.app.pages,
            Pages::machining_layout(),
            "the Switch Layout button"
        );
    }

    #[test]
    fn a_tool_from_a_flown_out_page_hides_it_unless_it_opens_a_form() {
        let mut h = Harness::new();
        h.app.pages = Pages::machining_layout();
        h.app.pages.open(Page::Design);
        h.app.pick(&h.ctx, Page::Design, "draw_rectangle");
        assert_eq!(
            h.app.pages.shown(Page::Design),
            Shown::Flyout,
            "its form shows there"
        );
        h.app.form = None;
        h.app.pick(&h.ctx, Page::Design, "select_all_vectors");
        assert_eq!(h.app.pages.shown(Page::Design), Shown::Hidden);
    }

    #[test]
    fn a_tools_form_takes_the_place_of_the_page_as_in_aspire() {
        let mut h = Harness::new();
        h.app.pages.set_pinned(Page::Design, true);
        h.app.pages.set_pinned(Page::Toolpaths, true);
        h.frame(vec![]);
        assert!(h.text_at("Create Vectors").is_some());
        assert!(h.text_at("Toolpath List").is_some());
        h.app.invoke(&h.ctx, "draw_circle");
        h.frame(vec![]);
        assert!(h.text_at("Create Vectors").is_none(), "design tools hidden");
        assert!(h.text_at("Toolpath List").is_some(), "the other page stays");
        h.app.invoke(&h.ctx, "profile_toolpath");
        h.frame(vec![]);
        assert!(
            h.text_at("Create Vectors").is_some(),
            "back once its form went"
        );
        assert!(h.text_at("Toolpath List").is_none(), "toolpath page hidden");
    }

    #[test]
    fn the_sheets_tab_lists_each_sheets_size_and_activates_one() {
        use cut_craft_core::document::Units;
        let mut h = Harness::new();
        h.app.pages.set_pinned(Page::Design, true);
        h.app.design_tab = 1;
        h.app.doc = cut_craft_core::testing::two_sheets();
        // the list's grid measures its columns on the first frame
        h.frame(vec![]);
        h.frame(vec![]);
        assert!(h.text_at("Sheets List").is_some());
        assert!(h.text_at("Sheet 2").is_some());
        assert!(
            h.text_at(&sheet_length(300.0, Units::Mm)).is_some(),
            "Sheet 2's X"
        );
        assert!(
            h.text_at(&sheet_length(12.0, Units::Mm)).is_some(),
            "Sheet 2's Z"
        );
        assert!(h.text_at("Add New Sheet...").is_some());
        let at = h.text_at("Sheet 2").unwrap();
        h.click(at, Modifiers::NONE);
        assert_eq!(h.app.doc.active_sheet, 1, "clicking a sheet activates it");
        assert_eq!(h.app.doc.job.width, 300.0);
    }

    #[test]
    fn the_clipart_tab_shows_a_library_folder_and_imports_what_is_double_clicked() {
        let dir = std::env::temp_dir().join(format!("cut-craft-clipart-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("flower.bmp"), crate::icons::tiny_bmp()).unwrap();
        std::fs::write(dir.join("notes.txt"), "not clipart").unwrap();
        let mut h = Harness::new();
        h.app.pages.set_pinned(Page::Design, true);
        h.app.design_tab = 4;
        h.app.clipart.folders = vec![dir.clone()];
        h.app.clipart.chosen = Some(0);
        h.frame(vec![]);
        h.frame(vec![]);
        let folder = dir.file_name().unwrap().to_string_lossy().into_owned();
        assert!(h.text_at(&folder).is_some(), "the library lists the folder");
        assert!(
            h.text_at("notes.txt").is_none(),
            "only what can be imported"
        );
        let at = h
            .text_at("flower.bmp")
            .expect("the picture is in the browser");
        h.double_click(at);
        assert_eq!(h.app.doc.bitmaps.len(), 1, "double-clicking imports it");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn the_top_bar_splits_the_views_and_toggles_snapping() {
        let mut h = Harness::new();
        h.app.doc = cut_craft_core::testing::two_sheets();
        h.frame(vec![]);
        for name in ["Sheet 1", "Layer 1", "Level 1"] {
            assert!(h.text_at(name).is_some(), "{name} in the top bar");
        }
        h.app.invoke(&h.ctx, "split_side_by_side");
        assert_eq!(h.app.split, Some(Split::SideBySide));
        h.frame(vec![]);
        h.app.invoke(&h.ctx, "split_side_by_side");
        assert_eq!(h.app.split, None, "a second click joins them again");
        let before = h.app.snap.clone();
        for id in [
            "toggle_geometry_snapping",
            "toggle_smart_snapping",
            "toggle_grid_snapping",
        ] {
            assert!(h.app.available(id), "{id}");
            h.app.invoke(&h.ctx, id);
        }
        let s = &h.app.snap;
        assert_eq!(
            (s.geometry_snapping, s.smart_snapping, s.snap_to_grid),
            (
                !before.geometry_snapping,
                !before.smart_snapping,
                !before.snap_to_grid
            )
        );
    }

    #[test]
    fn toolpath_forms_open_in_the_toolpaths_page() {
        let mut h = Harness::new();
        h.app.invoke(&h.ctx, "profile_toolpath");
        assert_eq!(h.app.form.as_ref().unwrap().page, Page::Toolpaths);
        assert_eq!(
            h.app.pages.shown(Page::Toolpaths),
            Shown::Flyout,
            "brought out to show its form"
        );
        h.app.invoke(&h.ctx, "draw_circle");
        assert_eq!(h.app.form.as_ref().unwrap().page, Page::Design);
    }

    #[test]
    fn clicking_the_view_puts_a_flown_out_page_away() {
        let mut h = Harness::new();
        h.app.pages.open(Page::Toolpaths);
        h.frame(vec![]);
        h.click(h.at(20.0, 200.0), Modifiers::NONE);
        assert_eq!(h.app.pages.shown(Page::Toolpaths), Shown::Hidden);
        assert_eq!(h.app.pages.shown(Page::Design), Shown::Docked);
    }

    #[test]
    fn menu_items_that_reuse_existing_tools() {
        let mut h = Harness::new();
        h.rectangle(10.0, 10.0, 20.0, 20.0);
        h.app
            .apply("add_layer", &serde_json::json!({ "name": "Top" }))
            .unwrap();
        h.rectangle(50.0, 10.0, 20.0, 20.0);
        let on_top = *h.app.doc.selection.iter().next().unwrap();
        h.app.invoke(&h.ctx, "select_all_vectors");
        h.app.invoke(&h.ctx, "select_all_vectors_on_current_layer");
        assert_eq!(
            h.app.doc.selection,
            [on_top].into(),
            "Select Vectors on Layer, current layer"
        );
        h.app.invoke(&h.ctx, "job_size_and_position");
        assert_eq!(h.app.form.as_ref().map(|f| f.id), Some("job_setup"));
        h.app.invoke(&h.ctx, "show_toolpaths_tab");
        assert_ne!(h.app.pages.shown(Page::Toolpaths), Shown::Hidden);
        h.app.invoke(&h.ctx, "zoom_box");
        h.app.invoke(&h.ctx, "select");
        assert!(
            h.app.form.is_none() && h.app.view_tool.is_none(),
            "the Select tool"
        );
    }

    #[test]
    fn run_and_form_items_apply_or_open_a_command_with_their_params() {
        let mut h = Harness::new();
        let add = gui::MenuItem::run::<cmd_add_layer::AddLayer>(
            "Add",
            &cmd_add_layer::Params { name: "Top".into() },
        );
        assert!(h.app.available(&add.action));
        h.app.invoke(&h.ctx, &add.action);
        assert_eq!(
            h.app.doc.layers.last().map(|l| l.name.as_str()),
            Some("Top")
        );
        let mut rename: cmd_layer_properties::Params = h.app.doc.layers[0].clone().into();
        rename.name = "Renamed".into();
        let rename =
            gui::MenuItem::form::<cmd_layer_properties::LayerProperties>("Rename", &rename);
        h.app.invoke(&h.ctx, &rename.action);
        let form = h.app.form.as_ref().unwrap();
        assert_eq!(
            (form.id, form.params["name"].as_str()),
            ("layer_properties", Some("Renamed"))
        );
        assert!(!h.app.available("run:no_such_command:{}"));
    }

    #[test]
    fn toolpaths_save_through_the_default_fluidnc_post_until_one_is_installed() {
        use cut_craft_core::document::{Along, Job};
        let mut h = Harness::new();
        assert_eq!(
            h.app.output_post().name,
            "Cut Craft FluidNC ATC (mm) (*.nc)"
        );
        h.app.doc.job = Job::rotary(300.0, 50.0, Along::X);
        assert_eq!(
            h.app.output_post().name,
            "Cut Craft FluidNC ATC Wrap A G93 (mm) (*.nc)"
        );
    }

    /// Controls that wait on a person or open other programs: the system's file dialogs, the
    /// save-changes question (the smoke test's job has unsaved work), the file manager and
    /// quitting. Running one in a test puts a real window on the desktop.
    fn needs_a_person(app: &CutCraftApp, id: &str) -> bool {
        const ASKS: [&str; 26] = [
            "add_clipart_folder",
            "segment_3d_model_file",
            "new",
            "open",
            "close",
            "save",
            "save_as",
            "increment_and_save",
            "open_application_data_folder",
            "import_vectors_bitmap",
            "import_component_3d_model",
            "save_as_template",
            "new_from_template",
            "save_selected_toolpath_as_template",
            "save_all_visible_toolpaths_as_template",
            "load_template",
            "print",
            "print_preview",
            "install_post_processor",
            "save_toolpaths",
            "exit",
            "recent_files",
            "import_vectors",
            "export_vectors",
            "open_folder",
            "open_clipart_folder",
        ];
        ASKS.contains(&id) || app.registry.exporter(id).is_some() || id.starts_with(EXPORT_LEVEL)
    }

    /// Every enabled menu item and tool button, run through the app as a click would be on a
    /// small job with vectors, a toolpath and a 3D shape: none may panic or be a dead end.
    #[test]
    fn every_enabled_control_works() {
        let mut h = Harness::new();
        h.app.doc.job.width = 100.0;
        h.app.doc.job.height = 100.0;
        h.rectangle(10.0, 10.0, 40.0, 40.0);
        h.rectangle(30.0, 30.0, 40.0, 40.0);
        h.app.invoke(&h.ctx, "select_all_vectors");
        for id in ["profile_toolpath", "create_shape"] {
            let params = h.app.registry.get(id).unwrap().initial_params(&h.app.doc);
            h.app.apply(id, &params).unwrap();
        }
        h.app.selected_toolpath = h.app.doc.toolpaths.keys().next().copied();
        h.frame(vec![]);
        let toolbar = &h.app.toolbar;
        let mut ids: Vec<String> = h
            .app
            .menus
            .items()
            .iter()
            .map(|i| i.action.clone())
            .collect();
        for group in toolbar
            .groups
            .iter()
            .chain(&toolbar.modeling_groups)
            .chain(&toolbar.toolpath_groups)
        {
            ids.extend(group.tools.iter().map(|&t| t.to_owned()));
        }
        ids.extend(VIEW_BUTTONS.iter().map(|(id, ..)| (*id).to_owned()));
        // the right-click menus, on each thing they open on
        let doc = &h.app.doc;
        let mut popups = h.app.menus.selection_menu(doc);
        for layer in &doc.layers {
            popups.extend(gui::layer_menu(doc, layer));
        }
        for (&id, toolpath) in &doc.toolpaths {
            popups.extend(gui::toolpath_menu(id, toolpath));
        }
        for (&id, component) in &doc.components {
            popups.extend(gui::component_menu(doc, id, component));
        }
        for (&id, level) in &doc.levels {
            popups.extend(gui::level_menu(doc, id, level));
        }
        for group in doc.components.values().filter_map(|k| k.group.as_ref()) {
            popups.extend(gui::group_menu(group));
        }
        let enabled = gui::leaves(&popups).into_iter().filter(|i| i.is_enabled);
        ids.extend(enabled.map(|i| i.action.clone()));
        ids.sort();
        ids.dedup();
        let start = (h.app.doc.clone(), h.app.selected_toolpath);
        let mut ran = 0;
        for id in ids {
            if !h.app.available(&id) || needs_a_person(&h.app, &id) {
                continue;
            }
            (h.app.doc, h.app.selected_toolpath) = start.clone();
            h.app.history = History::default();
            (h.app.form, h.app.view_tool) = (None, None);
            h.app.status.set_message("");
            h.app.invoke(&h.ctx, &id);
            // a tool with a form runs with what the form starts with
            if let Some(form) = h.app.form.take() {
                let outcome = h.app.apply(form.id, &form.params);
                if let Err(why) = outcome {
                    assert!(!why.contains("not implemented"), "{id}: {why}");
                }
            }
            h.frame(vec![]);
            let said = &h.app.status.message;
            assert!(!said.contains("not implemented"), "{id}: {said}");
            ran += 1;
        }
        assert!(ran >= 92, "only {ran} controls ran");
    }

    #[test]
    fn preview_all_leaves_hidden_toolpaths_out() {
        let mut h = Harness::new();
        h.rectangle(10.0, 10.0, 50.0, 50.0);
        for id in ["drilling_toolpath", "profile_toolpath"] {
            let params = h.app.registry.get(id).unwrap().initial_params(&h.app.doc);
            h.app.apply(id, &params).unwrap();
        }
        let profile = *h.app.doc.toolpaths.keys().last().unwrap();
        h.app.selected_toolpath = Some(profile);
        h.app.invoke(&h.ctx, "preview_toolpath");
        let only_profile = h.app.simulation.clone();
        let drilling = *h.app.doc.toolpaths.keys().next().unwrap();
        h.app.doc.toolpaths.get_mut(&drilling).unwrap().visible = false;
        h.app.invoke(&h.ctx, "preview_all_toolpaths");
        assert_eq!(h.app.simulation, only_profile);
    }

    #[test]
    fn deleting_the_selected_toolpath_is_undoable() {
        let mut h = Harness::new();
        h.rectangle(10.0, 10.0, 50.0, 50.0);
        h.app.invoke(&h.ctx, "drilling_toolpath");
        let params = h.app.form.as_ref().unwrap().params.clone();
        h.app.apply("drilling_toolpath", &params).unwrap();
        h.app.selected_toolpath = h.app.doc.toolpaths.keys().next().copied();
        h.app.invoke(&h.ctx, "delete_toolpath");
        assert!(h.app.doc.toolpaths.is_empty());
        h.app.invoke(&h.ctx, "undo");
        assert_eq!(h.app.doc.toolpaths.len(), 1);
    }
}
