//! Edit ▸ Options: settings kept between runs, in Aspire's sections. Only the settings Cut
//! Craft acts on are here: what the views draw when it starts, and the recent files kept.

use crate::display::Display;
use eframe::egui;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// What the 3D view draws when Cut Craft starts.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[expect(
    clippy::struct_excessive_bools,
    reason = "the dialog's own check boxes"
)]
pub struct View3d {
    pub draw_origin: bool,
    pub draw_material_block: bool,
    #[schemars(title = "Use color shaded view")]
    pub color_shaded_view: bool,
    #[schemars(title = "Use shaded background")]
    pub shaded_background: bool,
    pub light_follows_viewer: bool,
    /// What fills the 3D view behind the material, when the shaded background is on
    #[serde(default)]
    #[schemars(title = "Shaded background style")]
    pub background_style: BackgroundStyle,
    /// Solid, and the top of a gradient
    #[serde(default = "background_top")]
    #[schemars(extend("x-kind" = "colour"))]
    pub background_color: [u8; 3],
    /// The bottom of a gradient
    #[serde(default = "background_bottom")]
    #[schemars(extend("x-kind" = "colour"))]
    pub gradient_background_color: [u8; 3],
    /// The picture behind an Image background
    #[serde(default)]
    pub image_file_path: String,
    /// Smooths the edges of everything drawn (from the next start)
    #[serde(default)]
    #[schemars(title = "Anti-aliasing mode")]
    pub anti_aliasing: AntiAliasing,
    /// Draw the model coarser while the view turns or zooms, and at full detail once it is
    /// still: smoother on a slow computer, blockier while moving
    #[serde(default)]
    #[schemars(title = "Simplify the model while the view moves")]
    pub simplify_while_moving: bool,
    /// How finely the model is drawn once the view is still: higher shows more detail and
    /// takes longer to draw after each change
    #[serde(default)]
    #[schemars(title = "Detail when still")]
    pub still_detail: Detail,
    /// How finely the model is drawn while it or the view moves (turning the view, dragging
    /// a handle, playing a preview): lower keeps it smooth on a slower computer. Never finer
    /// than the detail when still
    #[serde(default)]
    #[schemars(
        title = "Detail while moving",
        extend("x-hide-when" = {"adjust_to_frame_rate": "true"})
    )]
    pub moving_detail: MovingDetail,
    /// While the view moves, draw the model as finely as this computer can keep smooth:
    /// coarser when frames come slowly, finer again when they come quickly, never finer
    /// than the detail when still
    #[serde(default = "yes")]
    #[schemars(title = "Adjust detail to keep the view smooth")]
    pub adjust_to_frame_rate: bool,
    /// How many pictures a second the 3D view aims for while it moves: higher is smoother
    /// and coarser, lower sharper and choppier. 30 suits most; films run at 24
    #[serde(default = "target_fps")]
    #[schemars(
        title = "Smoothness to aim for",
        extend("x-unit" = "frames/s", "x-show-when" = {"adjust_to_frame_rate": "true"}),
        range(min = 10, max = 120)
    )]
    pub target_fps: f64,
    /// Show the 3D view's detail and frames a second at the foot of the window, to see what
    /// the setting above does
    #[serde(default)]
    #[schemars(title = "Show detail and frame rate in the status bar")]
    pub show_frame_rate: bool,
    /// How dragging turns the view: about the job's vertical with its up kept up, or any
    /// way without stops
    #[serde(default)]
    pub rotation: Rotation,
    /// How near the pointer must be to the model, in screen pixels, for a click to pick a
    /// point on it (drawing on the model, placing a fade's anchor). Larger is easier to hit,
    /// smaller is more exact
    #[serde(default = "click_reach")]
    #[schemars(extend("x-unit" = "px"), range(min = 1, max = 20))]
    pub click_reach: f32,
    /// How wide the Perspective view sees, as a camera lens does: wider makes near things
    /// look larger and far ones smaller, narrower looks flatter
    #[serde(default = "perspective_angle")]
    #[schemars(extend("x-unit" = "°"), range(min = 20, max = 90))]
    pub perspective_angle: f64,
    /// How long the view must be still before it is drawn at the detail when still again
    #[serde(default = "settle_seconds")]
    #[schemars(
        title = "Wait before drawing full detail",
        extend("x-unit" = "s"),
        range(min = 0.05, max = 2)
    )]
    pub settle_seconds: f64,
}

fn click_reach() -> f32 {
    crate::view3d::PICK_REACH
}

fn perspective_angle() -> f64 {
    crate::view3d::FIELD_OF_VIEW.to_degrees()
}

fn yes() -> bool {
    true
}

fn target_fps() -> f64 {
    TARGET_FPS
}

/// Frames a second the 3D view aims for while it moves, until Options says otherwise.
pub const TARGET_FPS: f64 = 30.0;

fn settle_seconds() -> f64 {
    crate::app::SETTLE_SECONDS
}

impl View3d {
    /// The click reach in pixels, kept to the setting's range.
    pub fn pick_reach(&self) -> f32 {
        self.click_reach.clamp(1.0, 20.0)
    }

    /// The perspective angle in radians, kept to the setting's range.
    pub fn field_of_view(&self) -> f64 {
        self.perspective_angle.clamp(20.0, 90.0).to_radians()
    }

    /// Seconds still before full detail, kept to the setting's range.
    pub fn settle(&self) -> f64 {
        self.settle_seconds.clamp(0.05, 2.0)
    }

    /// Frames a second wanted while moving, kept to the setting's range.
    pub fn fps_wanted(&self) -> f64 {
        self.target_fps.clamp(10.0, 120.0)
    }

    /// Mesh cells along the job's longer side once the view is still, at most.
    pub fn still_cells(&self) -> usize {
        self.still_detail.cells()
    }

    /// Mesh cells along the job's longer side while the model or the view moves.
    pub fn moving_cells(&self) -> usize {
        let still = self.still_detail;
        let level = match self.moving_detail {
            MovingDetail::OneStepLower => still.lower(1),
            MovingDetail::TwoStepsLower => still.lower(2),
            MovingDetail::Low => Detail::Low,
            MovingDetail::Medium => Detail::Medium,
            MovingDetail::High => Detail::High,
            MovingDetail::VeryHigh => Detail::VeryHigh,
        };
        level.min(still).cells()
    }
}

/// How finely the 3D view draws the model.
#[derive(
    Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum Detail {
    /// Quickest: about 250 points across the job
    Low,
    /// About 500 points across the job
    Medium,
    /// About 800 points across the job
    High,
    /// Finest: about 1200 points across the job, or the model's own detail when that is less
    #[default]
    #[schemars(title = "Very High")]
    VeryHigh,
}

impl Detail {
    const ALL: [Self; 4] = [Self::Low, Self::Medium, Self::High, Self::VeryHigh];

    /// Mesh cells along the job's longer side.
    // ponytail: four fixed steps; the top one is the old fixed 1200, the bottom the old 250
    pub fn cells(self) -> usize {
        match self {
            Self::Low => 250,
            Self::Medium => 500,
            Self::High => 800,
            Self::VeryHigh => 1200,
        }
    }

    /// Its name, as Options lists it.
    pub fn name(self) -> &'static str {
        match self {
            Self::Low => "Low",
            Self::Medium => "Medium",
            Self::High => "High",
            Self::VeryHigh => "Very High",
        }
    }

    /// `steps` levels coarser, Low at the coarsest.
    pub fn lower(self, steps: usize) -> Self {
        Self::ALL[self.at().saturating_sub(steps)]
    }

    /// `steps` levels finer, Very High at the finest.
    pub fn higher(self, steps: usize) -> Self {
        Self::ALL[(self.at() + steps).min(Self::ALL.len() - 1)]
    }

    fn at(self) -> usize {
        Self::ALL.iter().position(|&d| d == self).unwrap_or(0)
    }
}

/// How finely the 3D view draws the model while it or the view moves.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum MovingDetail {
    /// One level below the detail when still
    #[schemars(title = "One step lower")]
    OneStepLower,
    /// Two levels below the detail when still (Very High moves at Medium)
    #[default]
    #[schemars(title = "Two steps lower")]
    TwoStepsLower,
    Low,
    Medium,
    High,
    #[schemars(title = "Very High")]
    VeryHigh,
}

/// How dragging (and a SpaceMouse) turns the 3D view.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Rotation {
    /// Spin about the job's vertical, tilt between straight down and straight up: the job's
    /// up stays up, as on a machine bed
    Turntable,
    /// Any way, without stops: the job can roll and turn upside down
    #[default]
    Free,
}

impl Rotation {
    pub fn view(self) -> crate::view3d::Rotation {
        match self {
            Self::Turntable => crate::view3d::Rotation::Turntable,
            Self::Free => crate::view3d::Rotation::Free,
        }
    }
}

/// A 3Dconnexion SpaceMouse turning the 3D view (Linux, through spacenavd).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[expect(
    clippy::struct_excessive_bools,
    reason = "the dialog's own check boxes"
)]
pub struct SpaceMouse {
    /// Object: the cap moves the model as if holding it. Camera: it moves the viewpoint, so
    /// every direction is the other way
    #[serde(default)]
    pub mode: SpaceMouseMode,
    /// How fast the view follows the cap
    #[serde(default = "one")]
    #[schemars(range(min = 0.1, max = 10.0))]
    pub speed: f64,
    /// Turn each movement the other way
    #[serde(default)]
    #[schemars(title = "Invert left / right")]
    pub invert_x: bool,
    #[serde(default)]
    #[schemars(title = "Invert up / down")]
    pub invert_y: bool,
    #[serde(default)]
    #[schemars(title = "Invert zoom (push / pull)")]
    pub invert_zoom: bool,
    #[serde(default)]
    #[schemars(title = "Invert tilt")]
    pub invert_tilt: bool,
    #[serde(default)]
    #[schemars(title = "Invert spin")]
    pub invert_spin: bool,
    #[serde(default)]
    #[schemars(title = "Invert roll (Free rotation only)")]
    pub invert_roll: bool,
}

impl Default for SpaceMouse {
    fn default() -> Self {
        Self {
            mode: SpaceMouseMode::default(),
            speed: one(),
            invert_x: false,
            invert_y: false,
            invert_zoom: false,
            invert_tilt: false,
            invert_spin: false,
            invert_roll: false,
        }
    }
}

fn one() -> f64 {
    1.0
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum SpaceMouseMode {
    /// The cap moves the model, as if holding it (the usual CAD way)
    #[default]
    Object,
    /// The cap moves the viewpoint round the model
    Camera,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum BackgroundStyle {
    Solid,
    #[default]
    Gradient,
    Image,
}

/// Multisampling: smooths the jagged edges of lines and the model in the 3D view.
// ponytail: 4x is the only count every graphics card takes; others can stop the window opening
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum AntiAliasing {
    #[default]
    None,
    #[schemars(title = "4X")]
    X4,
}

impl AntiAliasing {
    /// Samples a pixel for the window's renderer.
    pub fn samples(self) -> u16 {
        match self {
            Self::None => 0,
            Self::X4 => 4,
        }
    }
}

/// The light theme's own background, so the view looks as it did before these settings.
fn background_top() -> [u8; 3] {
    rgb(egui::Visuals::light().faint_bg_color)
}

fn background_bottom() -> [u8; 3] {
    rgb(egui::Visuals::light().extreme_bg_color)
}

fn rgb(c: egui::Color32) -> [u8; 3] {
    [c.r(), c.g(), c.b()]
}

/// Toolpath settings.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[expect(
    clippy::struct_excessive_bools,
    reason = "the dialog's own check boxes"
)]
pub struct ToolpathSettings {
    /// Swap to the 3D view after calculating a toolpath
    #[schemars(title = "Auto open 3D view")]
    pub auto_open_3d_view: bool,
    /// Draw toolpaths in the 2D view
    #[schemars(title = "Create 2D previews")]
    pub create_2d_previews: bool,
    /// Draw them as the area the tool covers
    #[schemars(title = "Solid 2D previews")]
    pub solid_2d_previews: bool,
    #[serde(default = "solid_colour")]
    #[schemars(title = "2D solid preview color", extend("x-kind" = "colour"))]
    pub solid_2d_preview_color: [u8; 3],
    /// Saved toolpaths leave out cutting points this close to the line past them, for
    /// shorter files; 0 keeps every point. Profile, pocket and other 2D toolpaths
    #[serde(default = "tolerance")]
    #[schemars(title = "2D toolpath tolerance", extend("x-unit" = "mm"), range(min = 0))]
    pub tolerance_2d: f64,
    /// The same for 3D roughing, finishing, texture, moulding and the like
    #[serde(default = "tolerance")]
    #[schemars(title = "3D toolpath tolerance", extend("x-unit" = "mm"), range(min = 0))]
    pub tolerance_3d: f64,
    /// The same for V-carving, prism, inlay and chamfer toolpaths
    #[serde(default = "tolerance")]
    #[schemars(title = "V-Carve toolpath tolerance", extend("x-unit" = "mm"), range(min = 0))]
    pub tolerance_vcarve: f64,
    /// How many times faster than the machine a toolpath preview plays when it starts: 1
    /// plays each move at its tool's feed or plunge rate, rapids at the rapid rate; the
    /// slider over the preview changes it while it plays
    #[serde(default = "preview_speed")]
    #[schemars(extend("x-unit" = "×"), range(min = 0.25, max = 1000))]
    pub preview_multiplier: f64,
    /// Write each keep-out zone's name on it in the 2D and 3D views ("Keep-out: Clamp 1").
    /// The View menu turns them on and off too
    #[serde(default = "on")]
    #[schemars(title = "Show keep-out zone labels")]
    pub keep_out_labels: bool,
    /// Off (safest): a toolpath that would cut into a keep-out zone, such as a clamp, is not
    /// made. On: it is made with the part under the zone left uncut, the tool lifting over
    /// the zone, as if the zone were cut out of whatever is under it
    #[serde(default)]
    #[schemars(title = "Clip toolpaths at keep-out zones")]
    pub clip_at_keep_out_zones: bool,
}

fn preview_speed() -> f64 {
    crate::app::PLAYBACK_SPEED
}

impl ToolpathSettings {
    /// The preview's starting speed, times the machine's, kept to the slider's range.
    pub fn preview_speed(&self) -> f64 {
        self.preview_multiplier.clamp(0.25, 1000.0)
    }
}

fn solid_colour() -> [u8; 3] {
    Display::default().solid_colour
}

/// A ten-thousandth of an inch.
// ponytail: Aspire's defaults are not known
fn tolerance() -> f64 {
    0.002_54
}

impl ToolpathSettings {
    /// The tolerance for a toolpath of this family.
    pub fn tolerance(&self, family: cut_craft_core::simplify::Family) -> f64 {
        use cut_craft_core::simplify::Family;
        match family {
            Family::TwoD => self.tolerance_2d,
            Family::ThreeD => self.tolerance_3d,
            Family::VCarve => self.tolerance_vcarve,
        }
    }

    /// `doc` with each toolpath's cuts thinned to its tolerance, ready to save.
    pub fn thinned(
        &self,
        doc: &cut_craft_core::document::Document,
    ) -> cut_craft_core::document::Document {
        use cut_craft_core::simplify::{family, simplify};
        let mut doc = doc.clone();
        for toolpath in doc.toolpaths.values_mut() {
            // a laser's power goes move by move, so its moves stay as they are
            if toolpath.power.is_empty() {
                toolpath.moves = simplify(&toolpath.moves, self.tolerance(family(toolpath)));
            }
        }
        doc
    }
}

/// What happens when something looks wrong.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Confirmations {
    /// Crossing vectors chosen for a V-carve or pocket toolpath
    #[serde(default)]
    #[schemars(title = "Toolpath vector intersections check")]
    pub toolpath_intersections: IntersectionCheck,
    /// Crossing vectors chosen for a radiused (round-cornered) offset
    #[serde(default)]
    #[schemars(title = "Radiused offset intersections check")]
    pub radiused_offset_intersections: IntersectionCheck,
    /// Crossing vectors chosen for a sharp-cornered offset
    #[serde(default)]
    #[schemars(title = "Sharp offset intersections check")]
    pub sharp_offset_intersections: IntersectionCheck,
    /// Problems the checks before cutting find when saving toolpaths
    #[serde(default)]
    #[schemars(title = "Checks before cutting")]
    pub cutting_checks: CuttingChecks,
    /// The checks before cutting warn when a tool's taper or shape will leave more material
    /// than this on the model. Raise it if the warnings are about bits too small to matter
    #[serde(default = "material_left")]
    #[schemars(
        title = "Warn when material left is over",
        extend("x-unit" = "mm"),
        range(min = 0, max = 10)
    )]
    pub material_left: f64,
}

impl Default for Confirmations {
    fn default() -> Self {
        Self {
            toolpath_intersections: IntersectionCheck::default(),
            radiused_offset_intersections: IntersectionCheck::default(),
            sharp_offset_intersections: IntersectionCheck::default(),
            cutting_checks: CuttingChecks::default(),
            material_left: material_left(),
        }
    }
}

fn material_left() -> f64 {
    cut_craft_core::checks::LEFT
}

impl Confirmations {
    /// The material-left threshold, mm, kept to the setting's range.
    pub fn left(&self) -> f64 {
        self.material_left.clamp(0.0, 10.0)
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum IntersectionCheck {
    /// Ask whether to go on, open the Vector Validator, or stop
    #[default]
    Ask,
    /// Select the crossing vectors in the Vector Validator instead
    OpenVectorValidator,
    /// Go on without checking
    #[schemars(title = "Don't check")]
    DontCheck,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum CuttingChecks {
    /// List the problems and ask before saving
    #[default]
    Ask,
    /// Save, and say what was found in the status bar
    #[schemars(title = "Save and tell")]
    SaveAndTell,
}

/// General settings.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct General {
    /// How many files File ▸ Recent Files and the start page list
    #[schemars(range(min = 1, max = 20))]
    pub recent_file_list_size: u32,
    /// Read every icon at startup, so none shows a loading spinner the first time it is
    /// drawn; off reads each when first drawn, for low-end systems (from the next start).
    /// Dialog pictures are always read at startup
    #[serde(default = "on")]
    pub preload_icons: bool,
    /// The folder file dialogs open in
    #[serde(default)]
    #[schemars(title = "File dialog default")]
    pub file_dialog_default: FileDialogDefault,
    /// How the window is arranged: the Design and Toolpaths pages with side tabs, a rail of
    /// tool groups with an inspector, or a ribbon of tabs over the view
    #[serde(default)]
    #[schemars(title = "Window layout")]
    pub layout: WindowLayout,
    /// How often unsaved work is copied, so it can be opened again after a crash or a power
    /// cut. Shorter loses less work; longer writes to the disk less often
    #[serde(default = "autosave_seconds")]
    #[schemars(
        title = "Keep a recovery copy every",
        extend("x-unit" = "s"),
        range(min = 10, max = 600)
    )]
    pub autosave_seconds: u32,
    /// With tool groups folded: how long to press and hold a toolbox button with a small
    /// arrow before its group of tools opens; a quick click runs the tool shown. Longer if
    /// groups open by accident
    #[serde(default = "flyout_hold_seconds")]
    #[schemars(
        title = "Hold to open a tool group",
        extend("x-unit" = "s"),
        range(min = 0.2, max = 1.5)
    )]
    pub flyout_hold_seconds: f64,
    /// Show each group of tools that do one job different ways (the circles, the text
    /// tools, the joins…) as one button with a small arrow, to open for the rest. Off shows
    /// every tool in a labelled box, so none is missed
    #[serde(default)]
    #[schemars(title = "Fold tool groups into one button")]
    pub fold_tool_groups: bool,
}

fn autosave_seconds() -> u32 {
    // a whole number of seconds
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "30"
    )]
    let seconds = crate::app::AUTOSAVE_SECONDS as u32;
    seconds
}

fn flyout_hold_seconds() -> f64 {
    crate::flyout::HOLD_SECONDS
}

impl General {
    /// Seconds between recovery copies, kept to the setting's range.
    pub fn autosave_interval(&self) -> f64 {
        f64::from(self.autosave_seconds.clamp(10, 600))
    }

    /// Seconds a toolbox button is held before its flyout opens, kept to the setting's range.
    pub fn flyout_hold(&self) -> f64 {
        self.flyout_hold_seconds.clamp(0.2, 1.5)
    }
}

/// Importing pictures and 3D models: the size pictures come in at, and what Orientate 3D
/// Model offers.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Import {
    /// How big an imported picture comes in: this many of its pixels make an inch. 96 suits
    /// screenshots and web pictures; a scan or a photo for printing is usually 300. A picture
    /// bigger than the material is still shrunk to fit
    #[serde(default = "picture_pixels_per_inch")]
    #[schemars(title = "Picture size (pixels per inch)", range(min = 10, max = 1200))]
    pub picture_pixels_per_inch: f64,
    /// What an imported model is kept as: whole (its underside on the other side of a
    /// double-sided job, wrapped in a rotary one), its top seen from above, or the choice
    /// of both on the form
    #[serde(default)]
    #[schemars(title = "Imported model type")]
    pub model_types: ModelTypes,
    /// A model with more faces than this is shown as the surface it makes while it is
    /// imported, with only the faces out of reach drawn over it; drawing millions of faces
    /// a frame makes the view drag
    #[serde(default = "preview_faces")]
    #[schemars(title = "Model preview faces", range(min = 1000))]
    pub preview_faces: u32,
    /// How much of a model placed past the material's edges shows while it is imported:
    /// that part is drawn in see-through red, this many percent solid, so it is not lost
    /// from sight but does not hide the material either
    #[serde(default = "out_of_bounds_percent")]
    #[schemars(title = "Out-of-bounds opacity (%)", range(min = 1, max = 100))]
    pub out_of_bounds_percent: u8,
}

fn preview_faces() -> u32 {
    250_000
}

fn picture_pixels_per_inch() -> f64 {
    cut_craft_core::import_settings::PIXELS_PER_INCH
}

fn out_of_bounds_percent() -> u8 {
    5
}

impl Import {
    /// Hands these settings to the importers.
    pub fn apply(&self) {
        cut_craft_core::import_settings::set_pixels_per_inch(self.picture_pixels_per_inch);
    }

    /// The red a model's parts past the material's edges are drawn in: see-through, as
    /// solid as Out-of-bounds opacity says (kept to 1 to 100 percent).
    pub fn out_of_bounds_colour(&self) -> egui::Color32 {
        let percent = u16::from(self.out_of_bounds_percent.clamp(1, 100));
        let alpha = u8::try_from((percent * 255 + 50) / 100).unwrap_or(u8::MAX);
        egui::Color32::from_rgba_unmultiplied(255, 0, 0, alpha)
    }
}

impl Default for Import {
    fn default() -> Self {
        Self {
            picture_pixels_per_inch: picture_pixels_per_inch(),
            model_types: ModelTypes::default(),
            preview_faces: preview_faces(),
            out_of_bounds_percent: out_of_bounds_percent(),
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ModelTypes {
    #[default]
    #[schemars(title = "Full 3D model")]
    Full,
    #[schemars(title = "Flat model")]
    Flat,
    #[schemars(title = "Both, chosen on the form")]
    Both,
}

/// Settings that change how hard Cut Craft works the computer, or how exactly it cuts: listed
/// only once Show advanced settings is ticked, and put back to their defaults when the file
/// was changed outside Cut Craft.
// ponytail: wayland_native moved here from General; one saved there before starts off again
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Advanced {
    /// On a Wayland desktop, open the window there directly rather than through `XWayland`;
    /// files cannot be dragged into it then (from the next start). Listed only on Wayland
    #[serde(default)]
    #[schemars(title = "Use Wayland directly (no file drag and drop)")]
    pub wayland_native: bool,
    /// How many processor cores previews, simulations and imports may use at once; 0 uses
    /// them all. Fewer keeps the computer responsive for other programs, but makes previews
    /// slower
    #[serde(default)]
    #[schemars(title = "Processor cores to use (0 = all)", range(min = 0, max = 256))]
    pub cores: u32,
    /// How many points each stretch of a curved DXF spline becomes when it is imported. Fewer
    /// gives flat-sided curves; more gives smoother curves but bigger, slower jobs
    // left out of the file (and so the check) at its default, so options sealed before it
    // existed still pass; every setting added here later wants the same
    #[serde(
        default = "dxf_spline_detail",
        skip_serializing_if = "is_dxf_spline_detail"
    )]
    #[schemars(
        title = "DXF curve detail (points per span)",
        range(min = 4, max = 256)
    )]
    pub dxf_spline_detail: u32,
    /// How often the Library reads its folders again to find files added, moved or deleted.
    /// Sooner shows changes quicker, but keeps a network drive or a slow disk busy
    #[serde(
        default = "library_rescan_seconds",
        skip_serializing_if = "is_library_rescan_seconds"
    )]
    #[schemars(
        title = "Library: read the folders again every",
        extend("x-unit" = "s"),
        range(min = 1, max = 3600)
    )]
    pub library_rescan_seconds: u32,
    /// The Library stops listing after this many files, so adding a whole drive can't stall
    /// it. More lists bigger collections, but uses more memory and time on every read
    #[serde(
        default = "library_most_files",
        skip_serializing_if = "is_library_most_files"
    )]
    #[schemars(
        title = "Library: most files listed",
        range(min = 1000, max = 10_000_000)
    )]
    pub library_most_files: u32,
    /// Saved Library previews not used for this long are deleted when Cut Craft starts. Longer
    /// keeps more of them on disk, so folders open faster (from the next start)
    #[serde(
        default = "preview_cache_days",
        skip_serializing_if = "is_preview_cache_days"
    )]
    #[schemars(
        title = "Library: keep unused previews for",
        extend("x-unit" = "days"),
        range(min = 1, max = 3650)
    )]
    pub preview_cache_days: u32,
    /// How many Library previews are kept on the graphics card at once. More scrolls back
    /// faster, but uses more video memory, which can run out on built-in graphics (from the
    /// next start)
    #[serde(
        default = "previews_on_card",
        skip_serializing_if = "is_previews_on_card"
    )]
    #[schemars(
        title = "Library: previews kept on the graphics card",
        range(min = 50, max = 5000)
    )]
    pub previews_on_card: u32,
    /// How closely curves are followed by the straight pieces offsets, pockets and profiles
    /// are cut from, and by the arcs written to G-code. Coarser shows as flats on curved cuts;
    /// finer makes toolpaths slower to work out and G-code files bigger
    #[serde(default = "curve_accuracy", skip_serializing_if = "is_curve_accuracy")]
    #[schemars(
        title = "Curve accuracy for toolpaths",
        extend("x-unit" = "µm"),
        range(min = 1, max = 100)
    )]
    pub curve_accuracy: u32,
    /// How far a 3D roughing move may dip below the machining allowance before it is split.
    /// Coarser eats into the allowance left for finishing; finer makes bigger files
    #[serde(
        default = "roughing_3d_accuracy",
        skip_serializing_if = "is_roughing_3d_accuracy"
    )]
    #[schemars(
        title = "3D roughing accuracy",
        extend("x-unit" = "µm"),
        range(min = 1, max = 100)
    )]
    pub roughing_3d_accuracy: u32,
    /// ⚠ How far a 3D finishing move may dip below the model before it is split. Raising this
    /// lets the tool cut into the finished part (gouging); lowering it makes much bigger files
    #[serde(
        default = "finishing_3d_accuracy",
        skip_serializing_if = "is_finishing_3d_accuracy"
    )]
    #[schemars(
        title = "3D finishing accuracy",
        extend("x-unit" = "µm"),
        range(min = 1, max = 20)
    )]
    pub finishing_3d_accuracy: u32,
    /// How much material must be left above the model for rest machining to go back for it.
    /// Lower finds smaller leftovers but may chase marks the last tool left; higher skips them
    #[serde(default = "rest_threshold", skip_serializing_if = "is_rest_threshold")]
    #[schemars(
        title = "Rest machining threshold",
        extend("x-unit" = "µm"),
        range(min = 5, max = 1000)
    )]
    pub rest_threshold: u32,
    /// The most sheets Nest Parts' Add sheets as needed makes in one go. More nests bigger
    /// batches, but a part that can never fit makes it run much longer before it gives up
    #[serde(
        default = "nest_most_sheets",
        skip_serializing_if = "is_nest_most_sheets"
    )]
    #[schemars(title = "Nesting: most sheets added", range(min = 1, max = 1000))]
    pub nest_most_sheets: u32,
    /// The most filler parts one nesting places. More fills big sheets with small parts, but
    /// slows nesting down
    #[serde(
        default = "nest_most_fillers",
        skip_serializing_if = "is_nest_most_fillers"
    )]
    #[schemars(title = "Nesting: most filler parts", range(min = 1, max = 100_000))]
    pub nest_most_fillers: u32,
}

/// The default of an Advanced setting, and whether a value is it: settings at their default
/// are left out of the file, so the check written before they existed still matches.
macro_rules! advanced_default {
    ($default:ident, $is:ident, $value:expr) => {
        fn $default() -> u32 {
            $value
        }

        fn $is(value: &u32) -> bool {
            *value == $default()
        }
    };
}

advanced_default!(
    dxf_spline_detail,
    is_dxf_spline_detail,
    cut_craft_core::import_settings::SPLINE_SAMPLES
);
advanced_default!(
    library_rescan_seconds,
    is_library_rescan_seconds,
    u32::try_from(crate::clipart::RELIST.as_secs()).expect("a few seconds")
);
advanced_default!(
    library_most_files,
    is_library_most_files,
    u32::try_from(crate::clipart::MOST).expect("a count that fits")
);
advanced_default!(
    preview_cache_days,
    is_preview_cache_days,
    u32::try_from(crate::clipart::preview::UNUSED_DAYS).expect("a count of days")
);
advanced_default!(
    previews_on_card,
    is_previews_on_card,
    u32::try_from(crate::clipart::preview::KEEP).expect("a count that fits")
);
advanced_default!(
    curve_accuracy,
    is_curve_accuracy,
    micrometres(cut_craft_core::accuracy::CURVE)
);
advanced_default!(
    roughing_3d_accuracy,
    is_roughing_3d_accuracy,
    micrometres(cut_craft_core::accuracy::ROUGHING_3D)
);
advanced_default!(
    finishing_3d_accuracy,
    is_finishing_3d_accuracy,
    micrometres(cut_craft_core::accuracy::FINISHING_3D)
);
advanced_default!(
    rest_threshold,
    is_rest_threshold,
    micrometres(cut_craft_core::accuracy::REST)
);
advanced_default!(
    nest_most_sheets,
    is_nest_most_sheets,
    u32::try_from(cut_craft_core::accuracy::NEST_SHEETS).expect("a small count")
);
advanced_default!(
    nest_most_fillers,
    is_nest_most_fillers,
    u32::try_from(cut_craft_core::accuracy::NEST_FILLERS).expect("a small count")
);

/// `mm` in whole micrometres, as the accuracy settings show it.
#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "small positive lengths"
)]
fn micrometres(mm: f64) -> u32 {
    (mm * 1000.0).round() as u32
}

impl Default for Advanced {
    fn default() -> Self {
        Self {
            wayland_native: false,
            cores: 0,
            dxf_spline_detail: dxf_spline_detail(),
            library_rescan_seconds: library_rescan_seconds(),
            library_most_files: library_most_files(),
            preview_cache_days: preview_cache_days(),
            previews_on_card: previews_on_card(),
            curve_accuracy: curve_accuracy(),
            roughing_3d_accuracy: roughing_3d_accuracy(),
            finishing_3d_accuracy: finishing_3d_accuracy(),
            rest_threshold: rest_threshold(),
            nest_most_sheets: nest_most_sheets(),
            nest_most_fillers: nest_most_fillers(),
        }
    }
}

impl Advanced {
    /// Whether this desktop is a Wayland one on Linux or a BSD, where `wayland_native` applies.
    pub fn wayland() -> bool {
        cfg!(all(unix, not(target_os = "macos"))) && std::env::var_os("WAYLAND_DISPLAY").is_some()
    }

    /// Hands these settings to the code they steer.
    pub fn apply(&self) {
        cut_craft_core::simulation::set_core_limit(usize::try_from(self.cores).unwrap_or(0));
        cut_craft_core::import_settings::set_spline_samples(self.dxf_spline_detail);
        let count = |n: u32| usize::try_from(n).unwrap_or(usize::MAX);
        crate::clipart::limits::set(
            u64::from(self.library_rescan_seconds.clamp(1, 3600)),
            count(self.library_most_files.clamp(1000, 10_000_000)),
            u64::from(self.preview_cache_days.clamp(1, 3650)),
            count(self.previews_on_card.clamp(50, 5000)),
        );
        let mm = |um: u32| f64::from(um) / 1000.0;
        cut_craft_core::accuracy::set(cut_craft_core::accuracy::Accuracy {
            curve: mm(self.curve_accuracy),
            roughing_3d: mm(self.roughing_3d_accuracy),
            finishing_3d: mm(self.finishing_3d_accuracy),
            rest: mm(self.rest_threshold),
            nest_sheets: count(self.nest_most_sheets),
            nest_fillers: count(self.nest_most_fillers),
        });
    }

    /// The check saved beside these settings: a keyed hash of them, so an edit made outside
    /// Cut Craft shows. The key ships in the app, so it stops a hand edit, not a determined
    /// one; that is all it is for.
    fn seal(&self) -> String {
        seal_of(&serde_json::to_vec(self).expect("settings serialise"))
    }

    /// Whether `seal` is these settings' own. Options saved before the check have none: they
    /// pass when only `wayland_native`, the one setting there was then, is set.
    fn sealed_by(&self, seal: Option<&str>) -> bool {
        match seal {
            Some(seal) => seal == self.seal(),
            None => {
                *self
                    == Self {
                        wayland_native: self.wayland_native,
                        ..Self::default()
                    }
            }
        }
    }
}

/// The keyed hash of `bytes` that [`Advanced::seal`] saves, in hex.
fn seal_of(bytes: &[u8]) -> String {
    use hmac::Mac;
    let mut mac = hmac::Hmac::<sha2::Sha256>::new_from_slice(SEAL_KEY).expect("any key fits");
    mac.update(bytes);
    mac.finalize()
        .into_bytes()
        .iter()
        .fold(String::new(), |mut hex, b| {
            use std::fmt::Write as _;
            let _ = write!(hex, "{b:02x}");
            hex
        })
}

/// The key of [`Advanced::seal`], and the options file's name for it.
const SEAL_KEY: &[u8] = b"Cut Craft advanced settings, v1";
const SEAL: &str = "advanced_check";

/// The warning above the advanced settings, and what it goes on to say.
pub const WARNING: &str = "⚠ Changing these can freeze Cut Craft or spoil a cut.";
pub const WARNING_MORE: &str = "They set how hard Cut Craft works your computer and how exactly \
    it cuts. Leave them alone unless you know why you are changing one; Reset to defaults puts \
    them all back.";

/// What the status bar says when the advanced settings were changed outside Cut Craft.
pub const TAMPERED: &str = "Advanced settings were changed outside Cut Craft, so they were put \
    back to their safe defaults. Change them in Edit ▸ Options ▸ Advanced.";

/// Help ▸ Check for Updates, and the check at startup.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Updates {
    /// About once a day, when Cut Craft starts, ask GitHub whether there is a new version,
    /// and say so if there is. Nothing is downloaded or installed until you choose Update Now
    #[serde(default = "on")]
    #[schemars(title = "Check for updates when Cut Craft starts")]
    pub check_at_startup: bool,
    /// When GitHub was last asked, in seconds since 1970.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(skip)]
    pub last_check: Option<u64>,
    /// The version Skip This Version was pressed for: not offered again at startup.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(skip)]
    pub skipped_version: Option<String>,
}

impl Default for Updates {
    fn default() -> Self {
        Self {
            check_at_startup: on(),
            last_check: None,
            skipped_version: None,
        }
    }
}

/// Bug reports, and what happens after Cut Craft closes because of an error.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct BugReports {
    /// If Cut Craft closes because of an error, it keeps a crash log saying what it was
    /// doing. The next time it starts it can open a bug report with that log, for you to
    /// read and send. Your user name and home folder are left out of it, and nothing is sent
    /// until you press Submit.
    #[serde(default)]
    #[schemars(title = "If Cut Craft crashes")]
    pub crash_logs: CrashLogs,
    /// A random number for this install, made when a crash report is first sent, so the
    /// crash-report service can tell one install from another. Nothing about the person.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(skip)]
    pub install_id: Option<String>,
}

/// Whether a crash's log is offered as a bug report at the next start.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum CrashLogs {
    /// Send the crash log to Cut Craft's crash-report service by itself (home folders and
    /// your user name left out). Where that can't be done, open a bug report with it ticked
    #[schemars(title = "Send the crash log")]
    Send,
    /// Open a bug report, and ask each time whether the crash log goes with it
    #[default]
    #[schemars(title = "Ask me each time")]
    Ask,
    /// Don't open a report: the crash log is kept in Cut Craft's settings folder
    #[schemars(title = "Don't send crash logs")]
    DontSend,
}

/// How Cut Craft looks.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Appearance {
    /// Light, dark, or the desktop's own
    #[serde(default)]
    pub theme: Theme,
    /// The desktop's own title bar above the menus, rather than Cut Craft's in the menu bar
    #[serde(default)]
    #[schemars(title = "Use the desktop's title bar")]
    pub system_title_bar: bool,
    /// The snap grid's points in the 2D view
    #[serde(default = "grid_color")]
    #[schemars(title = "Grid color", extend("x-kind" = "colour"))]
    pub grid_color: [u8; 3],
    /// How big the snap grid's points are drawn, in screen points
    #[serde(default = "grid_point_size")]
    #[schemars(range(min = 0.5, max = 10))]
    pub grid_point_size: f32,
    /// How big text, buttons and lines are, as a percentage: larger is easier to read and to
    /// click, smaller fits more on a small screen (from the next start)
    #[serde(default = "text_size")]
    #[schemars(title = "Text size", extend("x-unit" = "%"), range(min = 80, max = 200))]
    pub text_size: u32,
}

/// The size Cut Craft has always drawn its text at: egui's, plus 15%.
fn text_size() -> u32 {
    115
}

impl Default for Appearance {
    fn default() -> Self {
        Self {
            theme: Theme::default(),
            system_title_bar: false,
            grid_color: grid_color(),
            grid_point_size: grid_point_size(),
            text_size: text_size(),
        }
    }
}

impl Appearance {
    /// What egui's text and control sizes are multiplied by, kept between 80% and 200%.
    pub fn text_scale(&self) -> f32 {
        #[expect(clippy::cast_precision_loss, reason = "80 to 200")]
        let percent = self.text_size.clamp(80, 200) as f32;
        percent / 100.0
    }
}

/// The light theme's faint text colour, as other faint marks are.
fn grid_color() -> [u8; 3] {
    let c = egui::Visuals::light().weak_text_color();
    [c.r(), c.g(), c.b()]
}

/// As thick as two of the theme's lines.
fn grid_point_size() -> f32 {
    2.0 * egui::Visuals::light()
        .widgets
        .noninteractive
        .fg_stroke
        .width
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Theme {
    /// Light, whatever the desktop's theme
    Light,
    Dark,
    /// The desktop's own light or dark theme
    System,
}

impl Default for Theme {
    fn default() -> Self {
        Self::for_system(egui::os::OperatingSystem::from_target_os())
    }
}

impl Theme {
    /// Light, as Aspire is; on a Mac the desktop's own, as Mac users expect apps to follow
    /// their light or dark appearance.
    pub fn for_system(os: egui::os::OperatingSystem) -> Self {
        if os == egui::os::OperatingSystem::Mac {
            Self::System
        } else {
            Self::Light
        }
    }

    pub fn preference(self) -> egui::ThemePreference {
        match self {
            Self::Light => egui::ThemePreference::Light,
            Self::Dark => egui::ThemePreference::Dark,
            Self::System => egui::ThemePreference::System,
        }
    }
}

/// The window's arrangement (`docs/superpowers/specs/2026-10-27-window-layout-setting-design.md`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum WindowLayout {
    /// Design page left, Toolpaths page right, side tabs
    #[default]
    Classic,
    /// A rail of tool groups on the left opening a list of tools; an inspector on the right
    Rail,
    /// Tabs of tool groups across the top; the browser left, the form right
    Ribbon,
}

// ponytail: Aspire's third choice, a folder remembered for each kind of operation, is left out
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum FileDialogDefault {
    /// The desktop's own choice, usually the last folder used
    #[default]
    Global,
    /// The open job's folder
    Job,
}

/// What a setting added after options were first saved starts as.
fn on() -> bool {
    true
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Options {
    #[schemars(title = "3D View Settings", extend("x-section" = true))]
    pub view_3d: View3d,
    #[schemars(title = "Toolpath Settings", extend("x-section" = true))]
    pub toolpaths: ToolpathSettings,
    #[schemars(title = "General Settings", extend("x-section" = true))]
    pub general: General,
    #[serde(default)]
    #[schemars(title = "Confirmation Choices", extend("x-section" = true))]
    pub confirmations: Confirmations,
    #[serde(default)]
    #[schemars(title = "Importing", extend("x-section" = true))]
    pub import: Import,
    #[serde(default)]
    #[schemars(extend("x-section" = true))]
    pub appearance: Appearance,
    #[serde(default)]
    #[schemars(title = "SpaceMouse", extend("x-section" = true))]
    pub space_mouse: SpaceMouse,
    #[serde(default)]
    #[schemars(title = "Updates", extend("x-section" = true))]
    pub updates: Updates,
    #[serde(default)]
    #[schemars(title = "Bug Reports", extend("x-section" = true))]
    pub bug_reports: BugReports,
    #[serde(default)]
    #[schemars(extend("x-section" = true))]
    pub advanced: Advanced,
    /// Snapping Options (F4): what the pointer snaps to, nudge distances and rotate snap
    /// angles, kept from one run to the next.
    #[serde(default)]
    #[schemars(skip)]
    pub snapping: cut_craft_core::snap::SnapOptions,
    /// Gadgets ▸ Job Setup Sheet Editor: how job sheets look.
    #[serde(default)]
    #[schemars(skip)]
    pub job_sheet: fmt_create_job_sheet::Style,
    /// Toolpaths ▸ Print Part Labels: the labels' size and what they show.
    #[serde(default)]
    #[schemars(skip)]
    pub part_labels: fmt_part_labels::Style,
    /// Export as STL: how the model is triangulated.
    #[serde(default)]
    #[schemars(skip)]
    pub stl: fmt_export_as_stl::Settings,
    /// What Job Setup starts a new job with, once its Set as Default is pressed; Cut Craft's
    /// own starting job until then.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(skip)]
    pub default_job: Option<cut_craft_core::document::Job>,
    /// First-time setup was answered or skipped, so it is not shown at start again (Help ▸
    /// Run First-Time Setup Again still opens it). Kept here, in the options file, wherever
    /// the platform keeps that.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    #[schemars(skip)]
    pub first_run_done: bool,
    /// The favourite tools of the Design and Toolpaths tabs, and whether each shows only
    /// them (right-click the tab).
    #[serde(default)]
    #[schemars(skip)]
    pub favourites: Favourites,
    /// The Design sub-tabs shown as sections of their own under the tab row rather than in
    /// it, by name (right-click a sub-tab). None to start with: every one in the row.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[schemars(skip)]
    pub design_sections: Vec<String>,
    /// The tool each toolbox flyout shows: the one picked from it last.
    #[serde(default, skip_serializing_if = "std::collections::BTreeMap::is_empty")]
    #[schemars(skip)]
    pub flyout_faces: crate::flyout::Faces,
}

/// Each tool page's favourite tools, in the order shown, and whether it shows only those.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Favourites {
    #[serde(default)]
    pub design: Vec<String>,
    #[serde(default)]
    pub toolpaths: Vec<String>,
    #[serde(default)]
    pub design_only: bool,
    #[serde(default)]
    pub toolpaths_only: bool,
}

impl Favourites {
    /// `page`'s favourites and whether it shows only them.
    pub fn of(&self, page: crate::pages::Page) -> (&[String], bool) {
        match page {
            crate::pages::Page::Design => (&self.design, self.design_only),
            crate::pages::Page::Toolpaths => (&self.toolpaths, self.toolpaths_only),
        }
    }

    pub fn of_mut(&mut self, page: crate::pages::Page) -> (&mut Vec<String>, &mut bool) {
        match page {
            crate::pages::Page::Design => (&mut self.design, &mut self.design_only),
            crate::pages::Page::Toolpaths => (&mut self.toolpaths, &mut self.toolpaths_only),
        }
    }
}

impl Default for Options {
    /// What the views draw without settings: the display's own starting state.
    fn default() -> Self {
        let d = Display::default();
        Self {
            view_3d: View3d {
                draw_origin: d.origin,
                draw_material_block: d.material_block,
                color_shaded_view: d.colour_shaded,
                shaded_background: d.shaded_background,
                light_follows_viewer: d.light_follows,
                background_style: BackgroundStyle::default(),
                background_color: background_top(),
                gradient_background_color: background_bottom(),
                image_file_path: String::new(),
                anti_aliasing: AntiAliasing::default(),
                simplify_while_moving: false,
                still_detail: Detail::default(),
                moving_detail: MovingDetail::default(),
                adjust_to_frame_rate: true,
                target_fps: TARGET_FPS,
                show_frame_rate: false,
                rotation: Rotation::default(),
                click_reach: click_reach(),
                perspective_angle: perspective_angle(),
                settle_seconds: settle_seconds(),
            },
            toolpaths: ToolpathSettings {
                auto_open_3d_view: d.auto_open_3d,
                create_2d_previews: d.previews_2d,
                solid_2d_previews: d.solid_previews,
                solid_2d_preview_color: d.solid_colour,
                tolerance_2d: tolerance(),
                tolerance_3d: tolerance(),
                tolerance_vcarve: tolerance(),
                preview_multiplier: preview_speed(),
                keep_out_labels: d.keep_out_labels,
                clip_at_keep_out_zones: false,
            },
            general: General {
                recent_file_list_size: u32::try_from(crate::files::RECENT_COUNT)
                    .expect("a small count"),
                preload_icons: on(),
                file_dialog_default: FileDialogDefault::default(),
                layout: WindowLayout::default(),
                autosave_seconds: autosave_seconds(),
                flyout_hold_seconds: flyout_hold_seconds(),
                fold_tool_groups: false,
            },
            confirmations: Confirmations::default(),
            snapping: cut_craft_core::snap::SnapOptions::default(),
            job_sheet: fmt_create_job_sheet::Style::default(),
            part_labels: fmt_part_labels::Style::default(),
            stl: fmt_export_as_stl::Settings::default(),
            import: Import::default(),
            appearance: Appearance::default(),
            advanced: Advanced::default(),
            space_mouse: SpaceMouse::default(),
            updates: Updates::default(),
            bug_reports: BugReports::default(),
            default_job: None,
            first_run_done: false,
            favourites: Favourites::default(),
            design_sections: vec![],
            flyout_faces: crate::flyout::Faces::new(),
        }
    }
}

impl Options {
    /// Hands the settings that code outside the app reads (cores, importing) to it.
    pub fn apply(&self) {
        self.advanced.apply();
        self.import.apply();
    }

    /// The options kept in the config folder; the defaults when there are none.
    pub fn load() -> Self {
        Self::load_checked().0
    }

    /// [`Self::load`], and whether the advanced settings had been changed outside Cut Craft
    /// (and so were put back to their defaults).
    pub fn load_checked() -> (Self, bool) {
        path().and_then(|p| std::fs::read(p).ok()).map_or_else(
            || (Self::default(), false),
            |bytes| Self::from_bytes(&bytes),
        )
    }

    /// Options read from a file's bytes, the advanced ones only if their check matches; the
    /// defaults for a file that is not options at all.
    pub fn from_bytes(bytes: &[u8]) -> (Self, bool) {
        let Ok(mut file) = serde_json::from_slice::<serde_json::Value>(bytes) else {
            return (Self::default(), false);
        };
        let seal = file
            .as_object_mut()
            .and_then(|o| o.remove(SEAL))
            .and_then(|s| s.as_str().map(String::from));
        let Ok(mut options) = serde_json::from_value::<Self>(file) else {
            return (Self::default(), false);
        };
        let tampered = !options.advanced.sealed_by(seal.as_deref());
        if tampered {
            options.advanced = Advanced::default();
        }
        (options, tampered)
    }

    /// These options with one page (`general`, `advanced`…) back to its defaults, as its
    /// Reset to defaults button does; anything else kept.
    #[must_use]
    pub fn reset(&self, page: &str) -> Self {
        let mut values = serde_json::to_value(self).expect("options serialise");
        let defaults = serde_json::to_value(Self::default()).expect("options serialise");
        if let Some(default) = defaults.get(page) {
            values[page] = default.clone();
        }
        serde_json::from_value(values).unwrap_or_else(|_| self.clone())
    }

    /// Where the options are kept, when there is a config folder.
    pub fn path() -> Option<PathBuf> {
        path()
    }

    /// Saves the options, with the check that keeps the advanced ones from hand edits.
    pub fn save_to(&self, path: &std::path::Path) -> Result<(), String> {
        let mut file = serde_json::to_value(self).expect("options serialise");
        file[SEAL] = self.advanced.seal().into();
        let json = serde_json::to_vec_pretty(&file).expect("options serialise");
        cut_craft_core::file::write_atomic(path, &json)
            .map_err(|e| format!("{}: {e}", path.display()))
    }

    /// Sets what the views draw to these options.
    pub fn apply_to(&self, d: &mut Display) {
        let (v, t) = (&self.view_3d, &self.toolpaths);
        d.origin = v.draw_origin;
        d.material_block = v.draw_material_block;
        d.colour_shaded = v.color_shaded_view;
        d.shaded_background = v.shaded_background;
        d.light_follows = v.light_follows_viewer;
        d.auto_open_3d = t.auto_open_3d_view;
        d.previews_2d = t.create_2d_previews;
        d.solid_previews = t.solid_2d_previews;
        d.solid_colour = t.solid_2d_preview_color;
        d.keep_out_labels = t.keep_out_labels;
    }

    /// How many recent files are kept.
    pub fn recent_count(&self) -> usize {
        usize::try_from(self.general.recent_file_list_size.max(1)).unwrap_or(1)
    }
}

fn path() -> Option<PathBuf> {
    crate::files::config_dir().map(|dir| dir.join("options.json"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_mac_starts_following_the_desktops_light_or_dark() {
        use egui::os::OperatingSystem;
        assert_eq!(Theme::for_system(OperatingSystem::Mac), Theme::System);
        assert_eq!(Theme::for_system(OperatingSystem::Windows), Theme::Light);
        assert_eq!(Theme::for_system(OperatingSystem::Nix), Theme::Light);
    }

    #[test]
    fn out_of_bounds_parts_are_five_percent_red_unless_the_options_say_otherwise() {
        let mut import = Import::default();
        assert_eq!(import.out_of_bounds_percent, 5);
        let c = import.out_of_bounds_colour();
        assert_eq!(c.to_srgba_unmultiplied(), [255, 0, 0, 13], "5% of 255");
        import.out_of_bounds_percent = 0;
        assert_eq!(import.out_of_bounds_colour().a(), 3, "never wholly hidden");
        import.out_of_bounds_percent = 200;
        assert_eq!(import.out_of_bounds_colour().a(), 255);
        // options saved before the setting existed
        let old: Import = serde_json::from_str(r#"{"preview_faces": 1000}"#).unwrap();
        assert_eq!(old.out_of_bounds_percent, 5);
    }

    #[test]
    fn the_window_layout_is_classic_unless_the_options_say_otherwise() {
        assert_eq!(Options::default().general.layout, WindowLayout::Classic);
        let mut o = Options::default();
        o.general.layout = WindowLayout::Rail;
        let text = serde_json::to_string(&o).unwrap();
        assert!(text.contains("\"layout\":\"rail\""));
        let back: Options = serde_json::from_str(&text).unwrap();
        assert_eq!(back.general.layout, WindowLayout::Rail);
        // options saved before the setting existed
        let mut old: serde_json::Value = serde_json::to_value(&o).unwrap();
        old["general"].as_object_mut().unwrap().remove("layout");
        let old: Options = serde_json::from_value(old).unwrap();
        assert_eq!(old.general.layout, WindowLayout::Classic);
    }

    #[test]
    fn the_defaults_leave_the_display_as_it_starts() {
        let mut d = Display::default();
        Options::default().apply_to(&mut d);
        assert_eq!(d, Display::default());
    }

    #[test]
    fn options_set_what_the_views_draw() {
        let mut o = Options::default();
        o.view_3d.draw_origin = false;
        o.toolpaths.solid_2d_previews = true;
        o.toolpaths.solid_2d_preview_color = [1, 2, 3];
        let mut d = Display::default();
        o.apply_to(&mut d);
        assert!(!d.origin && d.solid_previews);
        assert_eq!(d.solid_colour, [1, 2, 3]);
    }

    #[test]
    fn they_load_back_as_saved_and_fill_gaps_with_defaults() {
        let o: Options =
            serde_json::from_str(&serde_json::to_string(&Options::default()).unwrap()).unwrap();
        assert_eq!(o, Options::default());
        let older = r#"{ "view_3d": { "draw_origin": false, "draw_material_block": true,
            "color_shaded_view": true, "shaded_background": true, "light_follows_viewer": true },
            "toolpaths": { "auto_open_3d_view": true, "create_2d_previews": true,
            "solid_2d_previews": false }, "general": { "recent_file_list_size": 4 } }"#;
        let older: Options = serde_json::from_str(older).expect("options saved before preloading");
        assert!(!older.view_3d.draw_origin && older.general.preload_icons);
        assert!(
            older.updates.check_at_startup,
            "options saved before updates check for them"
        );
        assert_eq!(
            older.appearance.theme,
            Theme::default(),
            "the system's starting theme, as before themes"
        );
        assert_eq!(
            older.snapping,
            cut_craft_core::snap::SnapOptions::default(),
            "the standard snapping, as before it was kept"
        );
        // snapping is kept from one run to the next, rotate snap angles and all
        let mut snapped = Options::default();
        snapped.snapping.rotate_step = cut_craft_core::snap::RotateSnap::Ten;
        snapped.snapping.snap_to_grid = true;
        let back: Options =
            serde_json::from_str(&serde_json::to_string(&snapped).unwrap()).unwrap();
        assert_eq!(back.snapping, snapped.snapping);
        let fields =
            crate::form::fields(&serde_json::to_value(schemars::schema_for!(Options)).unwrap());
        let sections: Vec<&str> = fields
            .iter()
            .filter(|f| f.section)
            .map(|f| f.label.as_str())
            .collect();
        assert_eq!(
            sections,
            [
                "3D View Settings",
                "Toolpath Settings",
                "General Settings",
                "Confirmation Choices",
                "Importing",
                "Appearance",
                "SpaceMouse",
                "Updates",
                "Bug Reports",
                "Advanced"
            ]
        );
        // what the check keeps for itself is not a setting to change
        let updates = fields.iter().find(|f| f.label == "Updates").unwrap();
        let crate::form::Kind::Group(shown) = &updates.kind else {
            panic!("Updates is a group of settings")
        };
        let shown: Vec<&str> = shown.iter().map(|f| f.key.as_str()).collect();
        assert_eq!(shown, ["check_at_startup"]);
    }

    #[test]
    fn saving_thins_each_toolpath_to_its_familys_tolerance() {
        use cut_craft_core::glam::DVec3;
        use cut_craft_core::toolpath::{Move, MoveKind, Toolpath};
        let line = |strategy: &str| Toolpath {
            name: strategy.into(),
            strategy: strategy.into(),
            params: serde_json::Value::Null,
            tool: cut_craft_core::toolpath::Tool::default(),
            vectors: vec![],
            moves: (0..=10)
                .map(|i| Move {
                    kind: MoveKind::Cut,
                    to: DVec3::new(f64::from(i), 0.0, -1.0),
                })
                .collect(),
            visible: true,
            power: vec![],
            stale: false,
            group: None,
            drill_bank: None,
        };
        let mut doc = cut_craft_core::document::Document::default();
        doc.toolpaths.insert(
            cut_craft_core::document::EntityId(1),
            line("profile_toolpath"),
        );
        doc.toolpaths.insert(
            cut_craft_core::document::EntityId(2),
            line("3d_finishing_toolpath"),
        );
        let mut t = Options::default().toolpaths;
        t.tolerance_3d = 0.0;
        let thinned = t.thinned(&doc);
        let count = |id| {
            thinned.toolpaths[&cut_craft_core::document::EntityId(id)]
                .moves
                .len()
        };
        assert_eq!(
            (count(1), count(2)),
            (2, 11),
            "2D thinned, 3D kept whole at 0"
        );
    }

    /// `o` as saved to a file, read back.
    fn saved(o: &Options) -> Vec<u8> {
        static FILES: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
        let n = FILES.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!("cc-options-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join(format!("{n}.json"));
        o.save_to(&path).unwrap();
        let bytes = std::fs::read(&path).unwrap();
        let _ = std::fs::remove_file(path);
        bytes
    }

    #[test]
    fn advanced_settings_saved_by_cut_craft_load_back_as_saved() {
        let mut o = Options::default();
        o.advanced.cores = 2;
        o.general.recent_file_list_size = 7;
        let (back, tampered) = Options::from_bytes(&saved(&o));
        assert!(!tampered);
        assert_eq!(back, o);
    }

    #[test]
    fn advanced_settings_edited_outside_cut_craft_go_back_to_their_defaults() {
        let mut o = Options::default();
        o.advanced.cores = 2;
        let mut file: serde_json::Value = serde_json::from_slice(&saved(&o)).unwrap();
        file["advanced"]["cores"] = 64.into();
        // a normal setting edited by hand is kept
        file["general"]["recent_file_list_size"] = 9.into();
        let (back, tampered) = Options::from_bytes(&serde_json::to_vec(&file).unwrap());
        assert!(tampered);
        assert_eq!(back.advanced, Advanced::default());
        assert_eq!(back.general.recent_file_list_size, 9);
        // taking the check away does not get round it
        file.as_object_mut().unwrap().remove(SEAL);
        let (back, tampered) = Options::from_bytes(&serde_json::to_vec(&file).unwrap());
        assert!(tampered);
        assert_eq!(back.advanced, Advanced::default());
    }

    #[test]
    fn a_seal_made_before_dxf_curve_detail_still_passes_and_the_detail_is_sealed() {
        // the seal an older Cut Craft wrote over { wayland_native, cores } alone
        #[derive(Serialize)]
        struct Before {
            wayland_native: bool,
            cores: u32,
        }
        let before = Before {
            wayland_native: false,
            cores: 2,
        };
        let seal = seal_of(&serde_json::to_vec(&before).unwrap());
        let mut file = serde_json::to_value(Options::default()).unwrap();
        file["advanced"] = serde_json::to_value(&before).unwrap();
        file[SEAL] = seal.into();
        let (back, tampered) = Options::from_bytes(&serde_json::to_vec(&file).unwrap());
        assert!(!tampered, "an upgrade is not an edit");
        assert_eq!(back.advanced.cores, 2);
        assert_eq!(back.advanced.dxf_spline_detail, 32);
        // changed in Cut Craft it loads back; changed by hand it goes back to 32
        let mut o = Options::default();
        o.advanced.dxf_spline_detail = 64;
        let (back, tampered) = Options::from_bytes(&saved(&o));
        assert!(!tampered);
        assert_eq!(back.advanced.dxf_spline_detail, 64);
        let mut file: serde_json::Value = serde_json::from_slice(&saved(&o)).unwrap();
        file["advanced"]["dxf_spline_detail"] = 200.into();
        let (back, tampered) = Options::from_bytes(&serde_json::to_vec(&file).unwrap());
        assert!(tampered);
        assert_eq!(back.advanced.dxf_spline_detail, 32);
    }

    #[test]
    fn importing_settings_reach_the_importers() {
        let mut o = Options::default();
        o.import.picture_pixels_per_inch = 300.0;
        o.advanced.dxf_spline_detail = 64;
        o.apply();
        assert_eq!(cut_craft_core::import_settings::pixels_per_inch(), 300.0);
        assert_eq!(cut_craft_core::import_settings::spline_samples(), 64);
        Options::default().apply();
        assert_eq!(cut_craft_core::import_settings::pixels_per_inch(), 96.0);
    }

    #[test]
    fn library_limits_reach_the_library_and_are_sealed() {
        let mut o = Options::default();
        o.advanced.library_rescan_seconds = 60;
        o.advanced.library_most_files = 5000;
        o.advanced.preview_cache_days = 7;
        o.advanced.previews_on_card = 100;
        o.apply();
        assert_eq!(
            crate::clipart::limits::relist(),
            std::time::Duration::from_mins(1)
        );
        assert_eq!(crate::clipart::limits::most(), 5000);
        assert_eq!(
            crate::clipart::limits::unused(),
            std::time::Duration::from_hours(24 * 7)
        );
        assert_eq!(crate::clipart::limits::keep(), 100);
        // out of range in the file: kept to the range
        o.advanced.library_most_files = 1;
        o.apply();
        assert_eq!(crate::clipart::limits::most(), 1000);
        Options::default().apply();
        assert_eq!(crate::clipart::limits::most(), crate::clipart::MOST);
        // saved by Cut Craft they load back; edited by hand they go back to the defaults
        let mut o = Options::default();
        o.advanced.previews_on_card = 100;
        let (back, tampered) = Options::from_bytes(&saved(&o));
        assert!(!tampered);
        assert_eq!(back.advanced.previews_on_card, 100);
        let mut file: serde_json::Value = serde_json::from_slice(&saved(&o)).unwrap();
        file["advanced"]["library_rescan_seconds"] = 1.into();
        let (back, tampered) = Options::from_bytes(&serde_json::to_vec(&file).unwrap());
        assert!(tampered);
        assert_eq!(back.advanced, Advanced::default());
    }

    #[test]
    fn cutting_accuracy_reaches_the_toolpaths_in_mm_and_is_sealed() {
        use cut_craft_core::accuracy::{self, Accuracy};
        assert_eq!(
            (
                Advanced::default().curve_accuracy,
                Advanced::default().finishing_3d_accuracy
            ),
            (10, 2),
            "the fixed values they replace, in micrometres"
        );
        let mut o = Options::default();
        o.advanced.curve_accuracy = 50;
        o.advanced.roughing_3d_accuracy = 20;
        o.advanced.finishing_3d_accuracy = 5;
        o.advanced.rest_threshold = 100;
        o.advanced.nest_most_sheets = 200;
        o.advanced.nest_most_fillers = 10;
        o.apply();
        assert_eq!(
            accuracy::get(),
            Accuracy {
                curve: 0.05,
                roughing_3d: 0.02,
                finishing_3d: 0.005,
                rest: 0.1,
                nest_sheets: 200,
                nest_fillers: 10,
            }
        );
        Options::default().apply();
        assert_eq!(accuracy::get(), Accuracy::default());
        // a gouging finish set by hand goes back to the safe default
        let mut file: serde_json::Value =
            serde_json::from_slice(&saved(&Options::default())).unwrap();
        file["advanced"]["finishing_3d_accuracy"] = 20.into();
        let (back, tampered) = Options::from_bytes(&serde_json::to_vec(&file).unwrap());
        assert!(tampered);
        assert_eq!(back.advanced.finishing_3d_accuracy, 2);
    }

    #[test]
    fn options_saved_before_the_check_keep_their_wayland_choice() {
        let older = r#"{ "view_3d": { "draw_origin": true, "draw_material_block": true,
            "color_shaded_view": true, "shaded_background": true, "light_follows_viewer": true },
            "toolpaths": { "auto_open_3d_view": true, "create_2d_previews": true,
            "solid_2d_previews": false }, "general": { "recent_file_list_size": 4 },
            "advanced": { "wayland_native": true } }"#;
        let (back, tampered) = Options::from_bytes(older.as_bytes());
        assert!(!tampered, "nothing to warn about");
        assert!(back.advanced.wayland_native);
    }

    #[test]
    fn a_page_resets_to_its_defaults_alone() {
        let mut o = Options::default();
        o.general.recent_file_list_size = 9;
        o.advanced.cores = 3;
        o.favourites.design = vec!["draw_circle".into()];
        let general = o.reset("general");
        assert_eq!(general.general, Options::default().general);
        assert_eq!(general.advanced.cores, 3, "other pages kept");
        assert_eq!(general.favourites.design, ["draw_circle"]);
        assert_eq!(o.reset("advanced").advanced, Advanced::default());
        assert_eq!(o.reset("no_such_page"), o);
    }

    #[test]
    fn new_settings_start_as_the_values_they_replace() {
        let o = Options::default();
        assert_eq!(
            f64::from(o.general.autosave_seconds),
            crate::app::AUTOSAVE_SECONDS
        );
        assert!((o.general.flyout_hold_seconds - crate::flyout::HOLD_SECONDS).abs() < 1e-9);
        assert_eq!(o.appearance.text_size, 115);
        assert!((o.appearance.text_scale() - 1.15).abs() < 1e-6);
        // options saved before these settings get the same
        let older = r#"{ "view_3d": { "draw_origin": true, "draw_material_block": true,
            "color_shaded_view": true, "shaded_background": true, "light_follows_viewer": true },
            "toolpaths": { "auto_open_3d_view": true, "create_2d_previews": true,
            "solid_2d_previews": false }, "general": { "recent_file_list_size": 4 } }"#;
        let (older, _) = Options::from_bytes(older.as_bytes());
        assert_eq!(older.general.autosave_seconds, o.general.autosave_seconds);
        assert!((older.general.flyout_hold_seconds - o.general.flyout_hold_seconds).abs() < 1e-9);
        assert_eq!(older.appearance.text_size, 115);
        // every tool shows, in its group's box, unless folded on purpose
        assert!(!o.general.fold_tool_groups);
        assert!(!older.general.fold_tool_groups);
    }

    #[test]
    fn text_size_stays_readable_whatever_the_file_says() {
        let mut a = Appearance {
            text_size: 5,
            ..Appearance::default()
        };
        assert!((a.text_scale() - 0.8).abs() < 1e-6, "no smaller than 80%");
        a.text_size = 900;
        assert!((a.text_scale() - 2.0).abs() < 1e-6, "no larger than 200%");
    }

    #[test]
    fn the_3d_view_settings_start_as_the_values_they_replace() {
        let v = Options::default().view_3d;
        assert!((v.pick_reach() - crate::view3d::PICK_REACH).abs() < 1e-6);
        assert!((v.field_of_view() - crate::view3d::FIELD_OF_VIEW).abs() < 1e-9);
        assert!((v.settle() - crate::app::SETTLE_SECONDS).abs() < 1e-9);
        let wild = View3d {
            click_reach: 500.0,
            perspective_angle: 179.0,
            settle_seconds: 0.0,
            ..v
        };
        assert!((wild.pick_reach() - 20.0).abs() < 1e-6);
        assert!((wild.field_of_view() - 90_f64.to_radians()).abs() < 1e-9);
        assert!((wild.settle() - 0.05).abs() < 1e-9);
    }

    #[test]
    fn the_toolpath_settings_start_as_the_values_they_replace() {
        let o = Options::default();
        assert!((o.toolpaths.preview_speed() - crate::app::PLAYBACK_SPEED).abs() < 1e-9);
        assert!((o.confirmations.left() - cut_craft_core::checks::LEFT).abs() < 1e-9);
        let older: Confirmations = serde_json::from_str(r#"{ "cutting_checks": "ask" }"#).unwrap();
        assert_eq!(
            older,
            Confirmations::default(),
            "older options get the same"
        );
        let wild = ToolpathSettings {
            preview_multiplier: 1e9,
            ..o.toolpaths
        };
        assert!((wild.preview_speed() - 1000.0).abs() < 1e-9);
    }

    #[test]
    fn keep_out_labels_show_and_clipping_is_off_until_turned_on() {
        let o = Options::default();
        assert!(o.toolpaths.keep_out_labels);
        assert!(
            !o.toolpaths.clip_at_keep_out_zones,
            "clipping is off by default"
        );
        // options saved before these settings get the same
        let mut saved = serde_json::to_value(&o.toolpaths).unwrap();
        let fields = saved.as_object_mut().unwrap();
        fields.remove("keep_out_labels");
        fields.remove("clip_at_keep_out_zones");
        let older: ToolpathSettings = serde_json::from_value(saved).unwrap();
        assert_eq!(older, o.toolpaths);
    }
}
