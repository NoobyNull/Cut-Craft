//! The main window: menus, Design and Toolpaths panels, and the 2D/3D views over the
//! document, its undo history and the command registry.

mod action_runs;
mod actions;
mod app_icon;
mod background;
pub(crate) mod built_ins;
mod calculate_again;
mod canvas_2d;
mod canvas_layout;
mod check_list;
mod command_form;
mod commit;
mod cutting;
mod depth;
mod dialog_windows;
mod draw3d;
mod drawing_input;
mod edit_apply;
mod entry;
mod first_run;
mod fit_import;
mod follow;
mod form_apply;
mod form_panel;
mod gcode_view;
mod generate_border;
mod gestures_2d;
mod guide;
mod run;
pub use run::*;
mod tool_sections;
#[cfg(test)]
use tool_sections::{KNIFE_LINE, held_name, tool_line};
use tool_sections::{
    ListAsk, ListSlot, PickFor, ToolAsk, ToolList, bordered, clearing_section, put_listed,
    rest_section, selector_section, shows_rest, tool_picker, tools_section, trimmed,
};
mod dialogs;
pub(crate) use dialogs::message_dialog;
use dialogs::{
    DIALOG_FOLDER, DIALOG_PARENT, brush_dialog, file_dialog, filtered_dialog, form_settings_dialog,
    job_dialog, on_the_desktop, saving_path, template_dialog, texture_dialog,
    toolpath_template_dialog,
};
mod readouts;
use readouts::{
    hms, inspected, inspected_rows, length, measured, measured_rows, sheet_length, summary_table,
};
mod settings;
pub use settings::*;
mod bit_changes;
#[cfg(test)]
pub(crate) mod hard_coded_text;
mod help;
#[cfg(test)]
mod help_coverage;
#[path = "help_pictures.rs"]
mod help_pictures;
#[path = "help_tours.rs"]
mod help_tours;
mod installed;
mod job_files;
mod keyboard;
mod language_page;
mod layout_choice;
mod layouts;
mod mac;
mod machine_fit;
mod machine_setup;
mod node_editing;
mod notes_panel;
mod options_apply;
mod options_window;
mod output;
mod pace;
mod page_layout;
mod panels;
mod passes;
mod placement;
mod player;
mod reach;
mod sample;
#[path = "screenshots.rs"]
mod screenshots;
mod selection_2d;
mod send;
mod setup_material;
mod shortcuts_page;
mod snapping;
mod split_views;
mod surfacing;
mod tabs;
#[cfg(test)]
mod tests;
#[cfg(test)]
pub(crate) use tests::document_menus;
mod tips;
mod title_bar;
mod toolpath_output;
mod tools_materials;
mod trace_window;
mod turning;
mod units_ask;
mod updates;
mod view_modes;
mod views;
mod web_api;
mod web_api_work;

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
use cut_craft_i18n::tr;
use eframe::egui::{self, Align, Key, KeyboardShortcut, Layout, Modifiers};
use serde_json::Value;
use std::borrow::Cow;

/// The side tabs: Design holds the drawing and modeling tools.
const DESIGN_TABS: [&str; 5] = ["Design", "Sheets", "Layers", "Components", "Library"];
/// The browser's tabs in the Rail and Ribbon layouts: the design tabs' panels and the
/// toolpath list under one row of tabs.
const INSPECTOR_TABS: [&str; 6] = [
    "Sheets",
    "Layers",
    "Components",
    "Library",
    "Toolpaths",
    "Checks",
];
/// The ribbon's tabs: the drawing, modeling and toolpath groups.
/// The Ribbon's tabs; Gadgets shows only once a gadget or add-on is installed.
const RIBBON_TABS: [&str; 5] = ["Design", "Model", "Toolpaths", "Favourites", "Gadgets"];
/// Every page tab's English name, each with its words in `locales/en/subtabs.ftl`.
#[cfg(test)]
pub(crate) fn page_tabs() -> impl Iterator<Item = &'static str> {
    DESIGN_TABS
        .into_iter()
        .chain(INSPECTOR_TABS)
        .chain(RIBBON_TABS)
        .chain(["Toolpaths"])
}
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
    /// Whether Import 3D Model shows a model (on its modeling plane, not seated), and
    /// whether whole, in place of its component.
    importing: (bool, bool),
}

/// What the vectors on the clipboard belonged to where they were copied, so a pasted word is
/// still a word: by clipboard position, each one's group and layer, and the groups' text
/// settings and part names.
#[derive(Default)]
struct Copied {
    /// Each copied vector's groups, innermost first: those wholly copied.
    groups: Vec<Vec<u64>>,
    layers: Vec<LayerId>,
    /// The job they were copied from (`job_epoch`): layer numbers mean nothing in another.
    job: u64,
    texts: std::collections::BTreeMap<u64, Value>,
    names: std::collections::BTreeMap<u64, String>,
}

/// The mark a toolpath's cells take in the preview (`Material::cut_by`): its id, unless a
/// reorder has moved it since it was first previewed (see [`CutCraftApp::preview_mark`]).
/// What the parts above a toolpath's flutes hit in a preview.
#[derive(Clone, Debug)]
pub(crate) struct Collided {
    /// What hits, where, and how to put it right.
    pub sentence: String,
    pub hits: Vec<cut_craft_core::collision::Hit>,
    /// A stamp of the toolpath when checked ([`background::fingerprint`]): once it is
    /// recalculated or its bit edited, what was found no longer holds.
    pub moves: u64,
}

fn preview_mark(id: EntityId) -> u32 {
    u32::try_from(id.0).unwrap_or(u32::MAX)
}

/// What a preview grid was worked out for: the job opened, the history's revision and the
/// toolpaths' moves, the detail slider and the block.
type GridKey = ((u64, u64, usize), u8, cut_craft_core::document::Job);

/// What the 3D view's surface on the GPU was built from: the model as at this revision,
/// wrapped and sided so (light is the shader's, so not these), at most this many cells (the
/// detail when still).
#[derive(Clone, Debug, PartialEq)]
struct GeometryKey {
    revision: u64,
    job: cut_craft_core::document::Job,
    wrap: Option<view3d::Wrap>,
    sides: bool,
    importing: (bool, bool),
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
    /// A preview drawn at its own detail: the sheet round it (with a hole where it goes)
    /// and the preview itself in tiles, both painted.
    preview: Option<(std::sync::Arc<view3d::gpu::Geometry>, view3d::tiles::Tiled)>,
    /// Coarser ones being made, by their cells.
    making: Vec<(usize, std::thread::JoinHandle<SurfaceLevel>)>,
}

impl SurfaceGpu {
    fn new(key: GeometryKey) -> Self {
        Self {
            key,
            levels: Vec::new(),
            fine: None,
            preview: None,
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
        let job = cut_craft_core::work::background("3d view detail", move || {
            let level = level_of(&view3d::coarse(&fine, cells), cells, wrap, sides, solid);
            ctx.request_repaint();
            level
        });
        if let Ok(job) = job {
            self.making.push((cells, job));
        }
    }
}

/// A rotary job's full 3D models as the 3D view draws them, kept by the change and the
/// import (its faces' address) they show.
type TurnedModels = (
    (u64, Option<usize>),
    std::sync::Arc<Vec<cmd_orientate_3d_model::Seen>>,
);

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
    /// Whether Import 3D Model shows a model (on its modeling plane, not seated), and
    /// whether whole, in place of its component.
    importing: (bool, bool),
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
    /// An imported picture, moved on its own
    Picture(EntityId),
}

use crate::file_filters::{Dialog, VECTOR_EXPORTERS};

/// What the window shows: the start page, Job Setup for a new job, or the job.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Screen {
    /// No job open: the startup tasks and recently opened files.
    Start,
    /// Job Setup for a new job; OK makes it, Cancel goes back to the start page.
    NewJob,
    Job,
}

/// A snap mark picked in the 2D view, and what was picked and done when it was: once the
/// selection, the tool or the job changes, it is no longer picked.
#[derive(Clone, Debug)]
struct PickedSnap {
    snap: cut_craft_core::constraint::Constraint,
    selection: cut_craft_core::selection::Selection,
    tool: Option<ViewTool>,
    /// The undo history's revision, which never repeats: an edit, undo or redo since lets
    /// it go, even one that leaves the history as deep as it was
    revision: u64,
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
    /// The view picked fills the room, the other in a corner of it
    Picture,
}

/// The picked components' outlines, with the components and revision they were traced for.
type PickedOutline = (
    (Vec<EntityId>, u64),
    Vec<Vec<cut_craft_core::geometry::Point>>,
);

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
    /// While a value is dragged, the history's revision after its last step: the next step
    /// takes it back first, if nothing has changed the history since.
    dragged: Option<u64>,
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
#[expect(
    clippy::struct_excessive_bools,
    reason = "the form's states, each on or off"
)]
struct OpenForm {
    id: &'static str,
    label: &'static str,
    icon: &'static str,
    /// How it behaves, as its command says: what its button says, and whether what it made
    /// follows it.
    kind: cut_craft_core::Form,
    fields: Vec<Field>,
    params: Value,
    error: Option<String>,
    /// What its last Apply made that is not quite what was asked (Embellish drawn finer
    /// than it carves): red text at the panel's foot, never a pop-up.
    warning: Option<String>,
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
    /// (with the ratio locked, only the latest): the rest follow the model as it turns. Set
    /// Size's width and height as last shown, to tell which one was typed in.
    shown_size: Option<[f64; 3]>,
    typed_size: [Option<f64>; 3],
    /// Orientate 3D Model's position X and Y and Depth Below Top as Align in Material last
    /// showed them: one changed since was typed, and drops that alignment.
    shown_place: [Option<f64>; 3],
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
    /// A form that remakes what it made ([`Form::Remake`](cut_craft_core::Form::Remake)): the history's revision and the selection
    /// just after its pieces were made, and the settings it was made with. While nothing else
    /// has been done or picked since, a change of settings takes them back and makes them
    /// again.
    remade: Option<(u64, cut_craft_core::selection::Selection, Value)>,
    /// While a value is dragged, the history's revision after its last step and the shape
    /// and settings from before that step: the next step takes it back first, if nothing
    /// (an Undo, another edit) has changed the history since.
    dragged: Option<(u64, Option<EntityId>, Option<Value>)>,
    /// How many changes could be undone when it opened: Cancel undoes back to there, then
    /// makes again the changes in `list_steps`.
    undo_from: usize,
    /// The changes made in the toolpath list beside it while it is open (reorder, group,
    /// delete, show or hide), each as its command and settings: they are not the form's, so
    /// Cancel makes them again after taking back everything since it opened.
    list_steps: Vec<(String, Value)>,
    /// While the toolpath list beside it is drawn: what it applies goes in `list_steps`.
    in_list: bool,
    /// How its drag is read: another way for a toolbox variant (Draw Circle from Edge).
    way: crate::flyout::DragWay,
    /// A toolpath form's toolpaths from its last Calculate: Calculate again updates them in
    /// place, or leaves them be when nothing has changed ([`calculate_again`]).
    made: Option<calculate_again::Made>,
    /// What its last press said that is not a warning ("Already up to date"), with the
    /// settings it was said for: shown while they are unchanged.
    note: Option<(Value, String)>,
    /// Setup Material: Job Setup showing only the job type and the material's size.
    brief: bool,
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
pub(crate) struct CutCraftApp {
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
    /// The toolpaths Ctrl-clicked in the list besides `selected_toolpath`, to group, move
    /// or drag together.
    more_toolpaths: std::collections::BTreeSet<EntityId>,
    /// A toolpath row was the last thing clicked: Del deletes the picked toolpath, not the
    /// drawing's selection. Any other click hands Del back to the drawing.
    toolpath_list_clicked: bool,
    /// The form a placing tool (Move, Rotate, Set Size: Transform Objects) was opened over,
    /// with its edits: Esc on the placing tool's form goes back to it.
    form_behind: Option<OpenForm>,
    /// Whether the first click on the Toolpaths page of a job that has not had Material
    /// Setup opens it (tests leave it off).
    ask_material: bool,
    /// The Toolpath List's height as its divider was last dragged; None for its share of
    /// the page.
    list_height: Option<f32>,
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
    /// Where the 2D view's top left corner was on screen last frame, to move `camera` with it.
    view_2d_at: Option<egui::Pos2>,
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
    /// Whether the Rail layout's list shows the favourites (its ★ button) instead of a group.
    rail_favourites: bool,
    /// The Ribbon layout's tab (`RIBBON_TABS`), which the Canvas layout's header shares.
    ribbon_tab: usize,
    /// What the Canvas layout has open.
    canvas: canvas_layout::CanvasState,
    /// Where the options are saved; `None` keeps them in memory (tests).
    options_path: Option<std::path::PathBuf>,
    /// The user's own spelling words, one a line; none in tests.
    words_path: Option<std::path::PathBuf>,
    /// Where how long each kind of slow job took is kept between runs
    /// ([`cut_craft_core::eta`]); `None` in tests.
    job_times_path: Option<std::path::PathBuf>,
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
    /// Generate Border's key, and the border it is generating.
    border: generate_border::BorderForm,
    /// Relief from Photo's High detail tick: unset means on once the model is fetched.
    depth_high_detail: Option<bool>,
    /// The High detail model downloading, or why its last download failed.
    depth_detail_fetch: Option<crate::depth::FetchingDetail>,
    depth_detail_failed: Option<String>,
    /// A slow job running off the window's thread, its progress shown over the window.
    busy: Option<crate::busy::Busy<CutCraftApp>>,
    /// How long a job may take before its progress is shown; quicker ones finish as if
    /// run in place.
    wait_before_showing: std::time::Duration,
    /// Gadgets ▸ Add-ons, and the installs it (or Relief from Photo) starts; where add-ons
    /// are installed (none in tests).
    addons: crate::addons::Addons,
    /// The language packs installed, each with how much of Cut Craft it covers (a
    /// percentage), for Options ▸ Language while it is open; read again when it next opens.
    language_packs: Option<Vec<(crate::languages::Pack, u32)>>,
    /// The machines (Machine Configuration), the active one's default post saving
    /// toolpaths; without one, Cut Craft's own `FluidNC` post for the job does.
    machines: cut_craft_machines::Machines,
    /// Where `machines` is kept; `None` keeps them in memory (tests).
    machines_path: Option<std::path::PathBuf>,
    /// The machines file could not be read at start: it is never saved over this run.
    machines_unsaved: bool,
    /// Cut Craft's posts and the user's custom ones.
    posts: cut_craft_machines::Library,
    /// The open Machine Configuration Management and Post-Processor Management windows.
    machine_window: Option<cut_craft_machines::MachineManager>,
    /// First-time setup's questions, while they are shown.
    first_run: Option<first_run::Setup>,
    /// The window layout question, while it is open.
    layout_question: Option<layout_choice::LayoutQuestion>,
    post_window: Option<cut_craft_machines::PostManager>,
    tools: ToolDatabase,
    /// Where `tools` is saved; `None` keeps it in memory (tests, or a file that failed to load).
    tools_path: Option<std::path::PathBuf>,
    /// The tool database file that could not be read at start: nothing is saved over it,
    /// and every save says so.
    tools_unreadable: Option<std::path::PathBuf>,
    /// The open Tool Database window.
    tool_manager: Option<cut_craft_tool_database::Manager>,
    /// The Materials Library: materials by class and hardness.
    materials: cut_craft_core::material_library::MaterialLibrary,
    /// Where `materials` is saved; `None` keeps it in memory.
    materials_path: Option<std::path::PathBuf>,
    /// The Materials Library file that could not be read at start, as `tools_unreadable`.
    materials_unreadable: Option<std::path::PathBuf>,
    /// The open Materials Library window.
    materials_window: Option<cut_craft_tool_database::materials::MaterialsWindow>,
    /// What the open Select Tool picks for: a tool in one of the form's lists, or a tool
    /// changer holder; `None` picks the form's own tool.
    picking: Option<PickFor>,
    /// A Calculate waiting on the deeper-than-the-material warning.
    deep_cut: Option<depth::DeepCut>,
    /// An import waiting on which units its drawing is in.
    units_ask: Option<units_ask::UnitsAsk>,
    /// Save Toolpaths held for the question about G-code an earlier save left there.
    earlier_ask: Option<EarlierAsk>,
    /// View G-code's window, while open.
    gcode_view: Option<gcode_view::GcodeView>,
    /// Save Toolpaths held for the question about what the checks before cutting found.
    save_anyway: Option<gcode_view::SaveAnyway>,
    /// Save or Send pressed in Save Toolpaths, waiting for its window to close.
    save_after: Option<gcode_view::SaveAfter>,
    /// Save Toolpaths' form, opened again once its save has asked all it asks.
    form_back: Option<SaveForm>,
    /// The problems Save Anyway was answered for: the next save goes on despite them, and
    /// asks again if the checks find any not among them. Empty when not answered.
    checks_accepted: Vec<String>,
    /// The warnings of the checks before cutting ticked as read, by what they say: Save
    /// and Send to Machine wait until each one found is ticked.
    acknowledged: std::collections::BTreeSet<String>,
    /// Right-click ▸ Report a Bug Here…
    reporter: cut_craft_bug_report::Reporter,
    /// Help ▸ Check for Updates, and the check at startup.
    updates: crate::update::Updates,
    /// Install Now or Relaunch Now: the new version, started once the window has closed.
    restart_after_close: Option<std::path::PathBuf>,
    /// The gadgets the user installed (not Cut Craft's own), by name: label and command id.
    my_gadgets: Vec<(String, &'static str)>,
    /// The add-ons installed that are not gadgets, by name, each with the tool it brings
    /// when Cut Craft knows it, and whether it is official (`crate::addons::installed_tools`).
    installed_addons: Vec<(String, Option<&'static str>, bool)>,
    /// Rail: the Gadgets list is open.
    rail_gadgets: bool,
    /// Gadgets from the My Gadgets folder or unsigned add-ons, not from Cut Craft's own
    /// signed builds, by command id.
    unsigned_gadgets: std::collections::HashMap<String, crate::gadgets::Unsigned>,
    /// The one check before unsigned code runs: gadgets and add-on programs alike.
    trust: crate::trust::Trust,
    /// A SpaceMouse's moves, once the app is running (not in tests or screenshots).
    space_mouse: Option<std::sync::mpsc::Receiver<crate::spacemouse::Input>>,
    /// `--screenshots`: the README's pictures being taken.
    shots: Option<screenshots::Shots>,
    /// `--help-pictures`: the help pages' step pictures being taken.
    help_shots: Option<help_pictures::HelpShots>,
    /// What Cut and Copy last took, pasted back in place.
    clipboard: Vec<cut_craft_core::geometry::Path>,
    /// What the copied vectors were part of: groups, layers, text and part names.
    clipboard_from: Copied,
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
    /// While a ring is held: the way + runs round it under the pointer, on screen, and how
    /// far it reaches from its middle, points.
    ring_grip: Option<(egui::Vec2, f32)>,
    /// An open Help window.
    info: Option<Info>,
    /// Help Contents' rendered pages and the `help:` links it watches.
    help_cache: egui_commonmark::CommonMarkCache,
    /// What is typed in the Help window's search box.
    help_search: String,
    /// Every help page as (topic, title, Markdown), made the first time help is searched.
    help_index: Vec<(&'static str, String, String)>,
    /// The help search's ranked index of [`Self::help_index`], made with it.
    help_ranked: Option<cut_craft_help::search::Index>,
    /// The help pages read before and after the one shown: Back and Forward.
    help_history: cut_craft_help::wiki::History<&'static str>,
    /// The Did You Know? window, open on this tip.
    tip: Option<usize>,
    /// The Did You Know? window's rendered tips.
    tips_cache: egui_commonmark::CommonMarkCache,
    /// Help ▸ Credits: its search and rendered licence texts.
    credits: crate::credits::Window,
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
    /// The side of a double-sided job the preview has up (`Document::bottom_side`): when
    /// the job turns over (Swap Sides, or its undo or redo), the preview turns with it.
    preview_side: bool,
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
    /// The checks before cutting shown in Save Toolpaths' form and the Analyse tab.
    save_checks: Vec<SaveChecks>,
    /// Whether Snapping Options (F4) is open; the options are kept in `options.snapping`.
    snap_window: bool,
    /// Snapping Options changed and not applied yet; `None` when it shows them as they are.
    snap_draft: Option<cut_craft_core::snap::SnapOptions>,
    /// Close was pressed on Snapping Options with changes not applied: it asks first.
    snap_closing: bool,
    /// Gadgets ▸ Job Setup Sheet Editor is open.
    job_sheet_window: bool,
    /// Toolpaths ▸ Print Part Labels is open.
    part_labels_window: bool,
    /// Toolpaths ▸ Clearing Settings is open (or Clearing Settings… in a form's Cutting
    /// window): the job's own, kept in `doc.job.clearing`.
    clearing_window: bool,
    /// The job's Clearing Settings before a slider in them began to be dragged.
    clearing_before: Option<cut_craft_core::entry::ClearingSettings>,
    /// Export as STL's form is open.
    stl_window: bool,
    /// The triangle count and largest error of its last Triangulate.
    stl_result: Option<(usize, f64)>,
    /// The Layers List leaves out the layers with nothing on the active sheet.
    layers_on_sheet: bool,
    /// What the views draw.
    display: crate::display::Display,
    /// The Preview tab's detail slider while it is dragged, before it is let go.
    held_detail: Option<u8>,
    /// The preview grid last worked out, and what it was for: the job opened, the
    /// revision and the toolpaths' moves, the detail and the block.
    preview_grid_kept: std::cell::Cell<Option<(GridKey, crate::preview_detail::Grid)>>,
    /// The material as cut by the previewed toolpaths, and its picture once drawn.
    simulation: Option<Material>,
    /// `simulation` at the 3D view's moving detail, and that detail (mesh cells).
    simulation_coarse: Option<(usize, Material)>,
    /// The model alone at the 3D view's moving detail (picking, handles, the quick mesh):
    /// made once a change, not every frame.
    rough_3d: Option<(RoughKey, Material)>,
    /// A rotary job's full 3D models as the view draws them round the axis, kept by the
    /// change and the import they show.
    turned_3d: Option<TurnedModels>,
    /// What the collet nut, spindle or shank hit in the last preview, by toolpath: the
    /// toolpath list marks them ⚠ in red and the 3D view shows the stock in the way red.
    collisions: std::collections::BTreeMap<EntityId, Collided>,
    /// The model's height, lowest point to highest, and the history's revision it was
    /// measured at: Model Position in Material's picture and warning, measured once a change.
    model_tall: std::cell::Cell<Option<(u64, f64)>>,
    /// The Notes panel's own state: Write or Preview, the Variable Explorer's search…
    notes_ui: notes_panel::NotesUi,
    /// The notes' variables and what they were worked out from (the job's revision, the
    /// machine, the minute…): worked out again only when one of those changes.
    note_vars:
        std::cell::RefCell<Option<(String, std::rc::Rc<Vec<cut_craft_core::notes::Variable>>)>>,
    /// The cutting times and what they were worked out from (the job's revision, the
    /// machine, the Toolpath Summary's settings): worked out again only when one changes.
    cut_times: std::cell::RefCell<Option<(String, std::rc::Rc<CutTimes>)>>,
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
    /// What told the toolpaths apart at the last change, so a reorder, or its undo or redo,
    /// moving them between ids is seen (see [`follow`]).
    toolpath_keys: follow::Keys,
    /// The mark each toolpath's cells take in the preview, where it is not its own id: the
    /// one it was previewed under before a reorder moved it to another id.
    preview_marks: std::collections::BTreeMap<EntityId, u32>,
    /// What a tool with no form (Group) warned of, in red at the foot of the tool panel, for
    /// the job (`job_epoch`) and change (history revision) it made: any later change, Undo
    /// too, takes it away.
    tool_note: Option<(u64, u64, String)>,
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
    /// The snap mark picked in the 2D view: Delete lets that point go.
    picked_constraint: Option<PickedSnap>,
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
    /// While Auto-rotate spins the 3D view: when it last turned it (input time, seconds).
    spinning_3d: Option<f64>,
    /// The picked components' outlines the 3D view draws (job coordinates, closed), kept
    /// with what they were traced for: the components and the document's revision.
    picked_outline: Option<PickedOutline>,
    /// Picks the 3D view's detail while it moves from how quickly frames come (Options ▸
    /// Adjust detail to keep the view smooth).
    pacer: crate::frame_rate::Pacer,
    /// How briskly a calculation is going, for the status bar's Crawl / Walk / Run / Sprint.
    gauge: cut_craft_core::work::Gauge,
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
    /// The preview player: a preview playing, paused or kept, and how it plays.
    player: player::Player,
    /// Where the Close up slider is centred, held while it is dragged so it stays put.
    close_up: Option<f64>,
    /// A rotary job's preview as the machine runs it: the file's own moves played into the
    /// material turning with A, kept in step with the playing (or kept) preview.
    turning: Option<turning::Turning>,
    /// Component tree levels collapsed (Expand / Collapse).
    collapsed_levels: std::collections::HashSet<u64>,
    /// A guide being dragged: whether it is vertical, and which it is (`None` a new one).
    guide_drag: Option<(bool, Option<usize>)>,
    /// Transform mode's corners rotate rather than scale.
    rotating: bool,
    /// Edit ▸ Options, kept between runs, and whether its window is open.
    options: crate::options::Options,
    options_window: bool,
    /// Changes made in the Options window and not applied yet; `None` when it shows the
    /// options as they are. They take effect only on Apply.
    options_draft: Option<crate::options::Options>,
    /// Options ▸ Job Defaults' job settings as being edited, and why the job they make was
    /// refused, if it was (kept as typed until it is put right).
    job_defaults_form: Option<(Value, Option<String>)>,
    /// Close was pressed with changes not applied: the window asks Apply or Discard.
    options_closing: bool,
    /// The Options category shown, by its place in the list.
    options_page: usize,
    /// Show advanced settings is ticked: the Advanced category lists. Off at every start, so
    /// reaching them is always a deliberate step.
    show_advanced: bool,
    /// Options ▸ Web API: the server, if it is listening, and the calls waiting to run.
    web_api: web_api::WebApi,
    /// The Options category to show when the window next draws, by its key (Help ▸
    /// Keyboard Shortcuts' Change Shortcuts opens Mouse and Shortcuts).
    options_goto: Option<&'static str>,
    /// A shortcut being set in Options ▸ Mouse and Shortcuts: the next key or mouse button
    /// pressed is taken for it, and runs nothing.
    capture: Option<ShortcutCapture>,
    /// What the shortcut list is narrowed to, typed in its search box.
    shortcut_search: String,
    /// Both views shown at once.
    split: Option<Split>,
    /// How the two views share the room while both show.
    split_layout: split_views::SplitLayout,
    /// The Library tab.
    clipart: crate::clipart::Clipart,
    /// Files from the Library waiting to be added, one after another, each with the level
    /// Add to Level puts it on.
    import_queue: std::collections::VecDeque<(std::path::PathBuf, Option<crate::clipart::ToLevel>)>,
    /// A drawing or picture Cut Craft was started with (Open With) while no job was open:
    /// imported once Job Setup's OK has made the job to hold it.
    import_after_setup: Option<std::path::PathBuf>,
    /// Where `recent` is saved; `None` keeps it in memory (tests).
    recent_path: Option<std::path::PathBuf>,
    /// Where unsaved work is copied (as often as Options says) for the next run to recover;
    /// `None` keeps no copy (tests).
    recovery_path: Option<std::path::PathBuf>,
    /// Held while this run lasts: marks `recovery_path` as this window's, so another window
    /// does not take it for a crash's.
    recovery_lock: Option<std::fs::File>,
    /// Install Now kept the unsaved work in the recovery copy for the new version to open.
    work_kept_for_update: bool,
    /// The history revision of the last recovery copy written, and the input time (seconds)
    /// of the last try at one.
    autosaved: (u64, f64),
    /// A recovery copy being written, and the history revision it holds.
    autosave: Option<(u64, std::thread::JoinHandle<Result<(), String>>)>,
    /// Window title last sent to the viewport.
    title: String,
    /// Whether the window opened with the desktop's title bar. A Mac keeps the window it opened
    /// with until Cut Craft restarts, so its menu row follows this rather than Options.
    title_bar_at_start: bool,
}

use cut_craft_core::file::TEMPLATE_EXTENSION as TEMPLATE;

/// What drawing the menus needs from the app.
struct Menus<'a> {
    available: &'a dyn Fn(&str) -> bool,
    /// Why an item is greyed out ([`CutCraftApp::unavailable_why`]).
    why: &'a dyn Fn(&str) -> String,
    /// Ticked state of the items that are settings; `None` keeps the item's own.
    checked: &'a dyn Fn(&str) -> Option<bool>,
    shortcut_of: &'a dyn Fn(&str) -> Option<String>,
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
            let shortcut = (menus.shortcut_of)(&i.action).unwrap_or_else(|| i.shortcut.clone());
            text(&crate::tool_names::menu_item_name(i))
                + text(&shortcut_label(ui.ctx(), &shortcut))
                + room
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

fn menu_item(ui: &mut egui::Ui, item: &gui::MenuItem, menus: &Menus, clicked: &mut Option<String>) {
    let name = crate::tool_names::menu_item_name(item);
    if item.action == "recent_files" {
        ui.add_enabled_ui(!menus.recent.is_empty(), |ui| {
            ui.menu_button(&name, |ui| {
                menu_items(ui, menus.recent, menus, clicked);
            });
        });
    } else if !item.children.is_empty() {
        ui.add_enabled_ui(item.is_enabled, |ui| {
            ui.menu_button(&name, |ui| {
                menu_items(ui, &item.children, menus, clicked);
            });
        });
    } else {
        let picture = item.icon.as_deref().and_then(menus.image);
        let button = match picture {
            Some(picture) => {
                let edge = ui.spacing().interact_size.y;
                let picture = picture.fit_to_exact_size(egui::Vec2::splat(edge));
                egui::Button::image_and_text(picture, &name)
            }
            None => egui::Button::new(&name),
        }
        .shortcut_text(shortcut_label(
            ui.ctx(),
            &(menus.shortcut_of)(&item.action).unwrap_or_else(|| item.shortcut.clone()),
        ))
        .selected((menus.checked)(&item.action).unwrap_or(item.is_checked));
        let available = (menus.available)(&item.action);
        let mut response = ui.add_enabled(item.is_enabled && available, button);
        if response.hovered()
            && let Some(about) = gui::about(&item.action)
        {
            cut_craft_bug_report::hint::hint(ui.ctx(), &about);
        }
        if !available {
            response = response.on_disabled_hover_text((menus.why)(&item.action));
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

/// How the 3D view wraps `surface` round a rotary job's axis, when Auto-Wrapping is `on`: a
/// preview's surface is at 0, its uncut material out to a square's or board's own faces;
/// the model placed as Material Setup says.
fn wrap_for(
    job: &cut_craft_core::document::Job,
    on: bool,
    previewing: bool,
    surface: &Material,
) -> Option<view3d::Wrap> {
    job.rotary.filter(|_| on).map(|r| view3d::Wrap {
        along_x: r.along == cut_craft_core::document::Along::X,
        radius: r.diameter / 2.0,
        outline: view3d::outline_of(&r).filter(|_| previewing),
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

/// Whether one vector is selected, for a drawing to be imported into (the 2D view's
/// right-click Import Into This Shape…) or, an open one, bent along.
pub fn shape_to_import_into(doc: &cut_craft_core::document::Document) -> bool {
    fit_import::chosen_shape(doc).is_ok()
}

/// Whether the one vector selected is an open line, for the right-click item to read
/// Import Along This Line….
pub fn line_to_import_along(doc: &cut_craft_core::document::Document) -> bool {
    fit_import::chosen_is_line(doc)
}
