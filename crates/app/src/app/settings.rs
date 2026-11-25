//! The app's fixed values (sizes, timings, built-in names, links and services) and the small
//! records its state keeps.

use super::{EntityId, Material, Units, egui, files, shortcut};

/// Model ▸ Model Resolution's items, the resolution each sets and its name.
/// Edit ▸ Units' items: action, units and name.
pub(super) const UNITS: [(&str, Units, &str); 2] = [
    ("units_mm", Units::Mm, "millimetres"),
    ("units_inch", Units::Inch, "inches"),
];
pub(super) const RESOLUTIONS: [(&str, cut_craft_core::document::Resolution, &str); 3] = [
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
pub(super) const DROP_SHADE: f32 = 0.8;
/// How far past the material the modeling plane is drawn while a model is imported, as a
/// share of the material's longer side, and how see-through it is.
pub(super) const MODELING_PLANE_MARGIN: f64 = 0.1;
pub(super) const MODELING_PLANE_ALPHA: f32 = 0.35;
/// How strongly Sketch Carving's green shows the areas it would carve over the picture.
pub(super) const SKETCH_AREA_ALPHA: f32 = 0.45;
/// How strongly V-Carve's area to carve is shaded over the vectors.
pub(super) const CARVE_AREA_ALPHA: f32 = 0.35;
/// Orientate 3D Model's rotation rings round the model, as a share of its longest side.
pub(super) const RING_SHARE: f64 = 0.6;

/// Where every Report a Bug Here… goes: Cut Craft's own repository.
pub(super) const BUG_REPO: &str = "NoobyNull/Cut-Craft";
/// Cut Craft's crash-report service, where the crash logs of people who said yes go: a
/// Cloudflare Worker that groups them by cause for the maintainer and clears them out once
/// dealt with (`services/crash-reports`).
pub(super) const CRASH_SERVICE: &str =
    "https://cutcraft-crash-reports.mattyax.workers.dev/v1/crash";
/// Points crash reports elsewhere, such as the service run locally for a test
/// (`http://localhost:8787/v1/crash`), never the live one.
pub(super) const CRASH_SERVICE_VARIABLE: &str = "CUTCRAFT_CRASH_URL";
/// A shortcut being set: for which action, and what the last press gave when it was not taken
/// straight away.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct ShortcutCapture {
    /// The action the shortcut is for.
    pub(super) id: String,
    /// A button that can't be a shortcut, and why.
    pub(super) refused: Option<&'static str>,
    /// A shortcut another action already has, and that action: kept until the person says
    /// to move it here or to leave it.
    pub(super) clash: Option<(shortcut::Trigger, String)>,
}

/// The Help windows.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Info {
    About,
    Shortcuts,
    /// Credits: the libraries, pictures and icons Cut Craft is built with.
    Credits,
    /// Toolpath Summary: estimated machining times.
    Summary,
    /// Help Contents (also a page's ?, and F1) on a page: `contents`, a tool page
    /// (`design`, `toolpaths`) or a tool's id.
    Help(&'static str),
}

/// The previewed material before a preview (`None` was uncut material), and the side
/// it was made on (`true` the bottom), so Undo Last Preview turns it over to the job's side.
#[derive(Clone, Debug)]
pub(in super::super) struct BeforePreview(pub(super) Option<Material>, pub(super) bool);

/// A save held while the user chooses what happens to an earlier save's G-code.
pub(in super::super) struct EarlierAsk {
    pub(super) form: SaveForm,
    pub(super) post: cut_craft_post::Post,
    /// Where the files go, as named.
    pub(super) path: std::path::PathBuf,
    /// The earlier save's files there.
    pub(super) earlier: Vec<std::path::PathBuf>,
    /// Keep the answer as the default.
    pub(super) remember: bool,
    /// Asked for Send to Machine: `earlier` are names on the machine, not paths here.
    pub(super) sending: bool,
}

/// Save Toolpaths' choices (Save Toolpaths form), while its window is open.
#[derive(Clone, Debug)]
pub(in super::super) struct SaveForm {
    /// Which toolpaths go in which file.
    pub(super) mode: files::SaveMode,
    /// Visible to multiple files: toolpaths in a row using one tool share a file.
    pub(super) group: bool,
    /// The post, by name in the library; `None` is the machine's default (Cut Craft's own
    /// without a machine).
    pub(super) post: Option<String>,
    /// Double-sided jobs: Top or Bottom on the end of each file name.
    pub(super) add_side: bool,
    /// How tools are changed: a file for each, the tool changer, or a pause.
    pub(super) changes: files::ToolChanges,
    /// Save the job sheet (the project's sheet, listing these files) beside the files.
    pub(super) job_sheet: bool,
}

/// The checks before cutting for a plan of toolpaths, on the job and machine as they were;
/// `found` is `None` while they run off the window's thread, or once they were cancelled.
#[derive(Clone, Debug)]
pub(in super::super) struct SaveChecks {
    pub(super) plan: Vec<Vec<EntityId>>,
    pub(super) changes: files::ToolChanges,
    pub(super) revision: u64,
    pub(super) machine: usize,
    pub(super) found: Option<Vec<cut_craft_core::preflight::Finding>>,
}

impl SaveChecks {
    /// Whether these are the checks for `other`'s plan, job and machine.
    pub(super) fn same(&self, other: &Self) -> bool {
        (&self.plan, self.changes, self.revision, self.machine)
            == (&other.plan, other.changes, other.revision, other.machine)
    }
}

/// How many plans' checks before cutting are kept.
pub(super) const KEPT_CHECKS: usize = 2;

/// What the progress window says while the checks before cutting run.
pub(super) const CHECKING: &str = "Checking the toolpaths before cutting";

/// Toolpath Summary's settings: the rapid rate (mm/min) and the units it is shown in, the
/// scale factor, and the toolpath picked to write notes for, with the notes being written.
#[derive(Clone, Debug)]
pub(super) struct Summary {
    pub(super) rapid: f64,
    pub(super) units: cut_craft_core::toolpath::RateUnits,
    pub(super) scale: f64,
    /// The Machine Acceleration: the times allow for the active machine's accelerations.
    pub(super) acceleration: bool,
    pub(super) picked: Option<EntityId>,
    pub(super) notes: String,
}

impl Default for Summary {
    fn default() -> Self {
        Self {
            rapid: 5000.0,
            units: cut_craft_core::toolpath::RateUnits::MmPerMin,
            scale: 1.0,
            acceleration: true,
            picked: None,
            notes: String::new(),
        }
    }
}

/// Guide Properties: which guide, and what is typed: its position and angle, whether it is
/// locked, and a new guide's position or offset and how many.
#[derive(Clone, Debug)]
pub(super) struct GuideProps {
    pub(super) index: usize,
    pub(super) position: f64,
    pub(super) angle: f64,
    pub(super) locked: bool,
    pub(super) new_at: f64,
    pub(super) relative: bool,
    pub(super) count: u32,
}

/// A toolpath's row in the Toolpath Summary: its id, name, time, whether it is shown, and
/// its notes.
/// Each toolpath's cutting time, minutes: the shown ones cut one after another from the
/// job's home (getting to each and its bit change included), a hidden one on its own; and
/// the whole job's, the trip back home included.
#[derive(Debug, Default)]
pub(crate) struct CutTimes {
    pub(super) each: std::collections::BTreeMap<EntityId, f64>,
    pub(super) total: f64,
}

pub(super) struct SummaryRow {
    pub(super) id: EntityId,
    pub(super) name: String,
    pub(super) minutes: f64,
    pub(super) visible: bool,
    pub(super) notes: String,
}

/// The fewest 3D view mesh cells along the longer side of the job: Options' Low detail.
/// The detail while moving and when still are Options' (3D View); at Very High, 1200
/// cells, the CPU mesh takes about 45 ms (measured in release) and the GPU's 40 MB.
pub(super) const MESH_CELLS: usize = 250;
/// Seconds between remakes of the model while a height handle drags it.
pub(super) const HEIGHT_DRAG_EVERY: f64 = 0.1;
/// How wide the Options, Snapping Options and Help windows open, as a share of the main
/// window's width.
pub(super) const DIALOG_WIDTH_SHARE: f32 = 0.4;

/// How big those windows open.
pub(super) fn dialog_size(ctx: &egui::Context) -> egui::Vec2 {
    ctx.content_rect().size()
        * egui::vec2(
            DIALOG_WIDTH_SHARE,
            cut_craft_bug_report::DIALOG_HEIGHT_SHARE,
        )
}

/// The logo on the opening splash: the CC monogram with the name under it, on its own dark
/// ground, so it reads the same on the light and the dark theme.
pub(super) const SPLASH_LOGO: &[u8] = include_bytes!("../../../../assets/splash-logo.jpg");
/// How long the 3D view stays still before it is drawn finely, seconds, until Options says
/// otherwise.
pub(crate) const SETTLE_SECONDS: f64 = 0.25;
/// How often unsaved work is copied for recovery after a crash, seconds, until Options says
/// otherwise.
pub(crate) const AUTOSAVE_SECONDS: f64 = 30.0;
/// Drawing tools that take the whole path the pointer was dragged along, as `points`.
pub(super) const STROKE_TOOLS: [&str; 3] = ["draw_freehand", "sculpt", "interactive_vector_trim"];
/// How far one notch of a mouse wheel scrolls, in points, as egui reports a line.
pub(super) const SCROLL_NOTCH: f32 = 50.0;
/// How much a wheel notch grows the sculpting brush, or its strength; with Ctrl, finer.
pub(super) const BRUSH_STEP: f64 = 0.1;
pub(super) const BRUSH_FINE_STEP: f64 = 0.02;
/// The view buttons over the top right of the 2D view, by action: each one's name and icon
/// are its [`crate::app::built_ins`] entry's.
pub(super) const VIEW_BUTTONS: [&str; 4] = ["pan", "zoom_box", "zoom_to_job", "zoom_selected"];
/// Scrolling a view's height divided by this doubles the zoom.
// ponytail: fixed rate until a preferences dialog exists
pub(crate) const VIEW_ZOOM_QUARTERS: f32 = 4.0;
/// The start page's tasks: action id and icon. Each one's words are `start-task-` and its id
/// (`locales/en/start.ftl`).
pub(super) const STARTUP_TASKS: [(&str, &str); 4] = [
    ("new", "file/new.png"),
    ("try_sample_project", "drawing/create_vectors/draw_text.png"),
    ("new_from_template", "toolpaths/load_toolpath_template.png"),
    ("open", "file/open.png"),
];

/// A menu's words' key (`locales/en/menus.ftl`): `menu-` and its English name, lower case and
/// hyphenated, so a menu is still found by its English name.
pub(crate) fn menu_key(name: &str) -> String {
    cut_craft_i18n::key_of("menu", name)
}
/// The level menu's Select Components and Export as 3D Clipart: these, then the level's id.
pub const SELECT_LEVEL: &str = "select_level_components:";
pub const EXPORT_LEVEL: &str = "export_level_clipart:";
/// Lines (widget outlines, the 2D view's vectors) are egui's widths times this, at the
/// standard text size; they grow and shrink with it.
pub(super) const LINE_SCALE: f32 = 1.5;
/// Easy-to-read UI fonts, the first installed used; egui's own stay behind for symbols.
pub(super) const UI_FONTS: [&str; 5] =
    ["Noto Sans", "Open Sans", "DejaVu Sans", "Segoe UI", "Arial"];
/// A preview starts at ten times the machine's own speed (×1: each move at its bit's feed
/// or plunge rate, rapids at the Toolpath Summary's rapid rate).
pub(crate) const PLAYBACK_SPEED: f64 = 10.0;
/// How many times faster than the machine a preview can play: a long finishing toolpath
/// takes hours at the machine's speed.
pub(super) const PLAYBACK_SPEEDS: std::ops::RangeInclusive<f64> = 0.25..=1000.0;
/// The shank of the tool drawn in the 3D view while a preview plays: steel grey.
pub(super) const TOOL_SHANK: egui::Color32 = egui::Color32::from_rgb(168, 172, 178);
/// Beside each recently opened file.
pub(super) const RECENT_FILE_ICON: &str = "file/new.png";
/// This version's changes, one `- ` line each, shown on the start page.
pub(super) const WHATS_NEW: &str = include_str!("../../whats-new.md");
/// The repository's contributors on GitHub (login, commits, profile), one a line in
/// `contributors.tsv`; Help ▸ About lists them.
pub(super) const CONTRIBUTORS: &str = include_str!("../../contributors.tsv");
/// Machine ▸ Search for Machine Online: the machine packages in Cut Craft's repository.
pub(super) const MACHINES_ONLINE: &str =
    "https://github.com/NoobyNull/Cut-Craft/tree/main/machines";
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
pub(super) const DESIGN_PANEL_WIDTHS: f32 = 7.0;
/// The most of the window's width one resizable side panel may take.
// ponytail: fixed share until panel widths are saved with the window layout
pub(super) const SIDE_PANEL_SHARE: f32 = 1.0 / 3.0;

/// Tool buttons a pinned page always has room for side by side, in a captioned box.
// ponytail: fixed count until a preferences dialog exists
pub(super) const PAGE_BUTTONS: f32 = 5.0;
/// Inner margin of a captioned box of tools on a page.
pub(super) const TOOL_BOX_MARGIN: f32 = 4.0;

/// The narrowest a tool page is, outer edge to outer edge: [`PAGE_BUTTONS`] tool buttons in a
/// captioned box, inside the page's frame.
pub(super) fn least_page_width(style: &egui::Style) -> f32 {
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
pub(super) fn side_tabs_width(style: &egui::Style) -> f32 {
    style.spacing.interact_size.y + 2.0 * style.spacing.item_spacing.y
}

/// How far the views start below the top of the room the pages share: the central panel's
/// margin, the top bar and the line under it, as `views` lays them out.
pub(super) fn views_top(style: &egui::Style) -> f32 {
    let s = &style.spacing;
    let bar = s.interact_size.y + 2.0 * s.button_padding.y;
    // `ui.separator()`'s own room, and the gap after the bar and after the line
    let line = 6.0;
    egui::Frame::central_panel(style).total_margin().top + bar + line + 2.0 * s.item_spacing.y
}

/// How wide the Design and Toolpaths pages are in `ui`'s room, docked or flown out (see
/// [`crate::pages::page_width`]). Called with the room between the menu and status bars, before
/// the side tabs take theirs; the result never depends on what the pages show.
pub(super) fn page_width(ui: &egui::Ui) -> f32 {
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
pub(super) fn show_page<R>(
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
pub(super) fn side_panel(panel: egui::Panel, ctx: &egui::Context) -> egui::Panel {
    let width = DESIGN_PANEL_WIDTHS * ctx.global_style().spacing.interact_size.x;
    let most = width.max(SIDE_PANEL_SHARE * ctx.content_rect().width());
    panel.default_size(width).max_size(most)
}
/// Size of a form section's picture, in rows of the theme's interactive height.
// ponytail: fixed ratio until a preferences dialog exists
pub(super) const SECTION_PICTURE_ROWS: f32 = 3.0;
/// The Cut Craft mark, shown with no job open.
/// How much of the middle of the window the mark takes, by its shorter side.
// ponytail: fixed share until there is a preferences dialog
pub(super) const SPLASH_SHARE: f32 = 1.0 / 3.0;
pub(super) const APP_NAME: &str = "Cut Craft";
/// How the desktop knows the window: the same name as the launcher, `cut-craft.desktop`,
/// so the taskbar shows its icon.
pub(super) const APP_ID: &str = "cut-craft";
/// Tool icon edge, in rows of the theme's interactive height.
// ponytail: fixed ratio until a preferences dialog exists
pub(super) const TOOL_ICON_ROWS: f32 = 1.75;
