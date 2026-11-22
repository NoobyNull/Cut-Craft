//! Edit ▸ Options: settings kept between runs, in the sections. Only the settings Cut
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
    /// The vectors drawn over the model (the 3D view's Vectors button)
    #[serde(default = "yes")]
    #[schemars(title = "Draw vectors in 3D view")]
    pub draw_vectors: bool,
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
    /// a handle): lower keeps it smooth on a slower computer. Never finer than the detail
    /// when still. A playing preview keeps the detail when still
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
    /// How quickly the 3D view's Auto-rotate button turns the job round, degrees a second:
    /// slower to look closely, faster for a quick look all round
    #[serde(default = "spin_speed")]
    #[schemars(
        title = "Auto-rotate speed",
        extend("x-unit" = "°/s"),
        range(min = 2, max = 90)
    )]
    pub spin_speed: f64,
    /// How finely a toolpath preview is worked out to start with: toward Performance is
    /// quicker and uses less memory, toward Detail shows small corners, holes and thin
    /// walls truer (about 0.5 mm between points at 0%, 0.1 mm at 50%, 0.025 mm at 100%).
    /// The Preview tab has the same slider for now; Reset Preview goes back to this
    #[serde(default = "preview_detail")]
    #[schemars(
        title = "Preview detail",
        extend("x-unit" = "%", "x-widget" = "slider"),
        range(min = 0, max = 100)
    )]
    pub preview_detail: u8,
}

fn preview_detail() -> u8 {
    crate::preview_detail::DEFAULT_PERCENT
}

fn spin_speed() -> f64 {
    SPIN_SPEED
}

/// Degrees a second Auto-rotate turns the 3D view, until Options says otherwise.
pub const SPIN_SPEED: f64 = 20.0;

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

    /// Auto-rotate's degrees a second, kept to the setting's range.
    pub fn spin_speed(&self) -> f64 {
        self.spin_speed.clamp(2.0, 90.0)
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

/// The mouse in the 2D and 3D views, and the shortcuts set in place of Cut Craft's own.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[expect(
    clippy::struct_excessive_bools,
    reason = "the page's own check boxes, one for each control"
)]
pub struct Controls {
    /// What tilting the mouse wheel left or right does over the 2D and 3D views. Holding
    /// Shift while turning the wheel, or swiping sideways on a touchpad, does the same
    #[serde(default)]
    #[schemars(title = "Tilting the wheel left or right")]
    pub tilt: Tilt,
    /// Trackpad mode: two fingers dragging pan the 2D and 3D views, and pinching zooms (in
    /// place of the wheel). For laptops with no mouse
    #[serde(default)]
    #[schemars(title = "Trackpad mode")]
    pub trackpad: bool,
    /// Turns the 3D view the other way when dragging left or right
    #[serde(default)]
    #[schemars(title = "Invert drag left / right (3D view)")]
    pub invert_drag_x: bool,
    /// Tilts the 3D view the other way when dragging up or down: a drag down tilts the top
    /// of the job away from you instead of towards you
    #[serde(default)]
    #[schemars(title = "Invert drag up / down (3D view)")]
    pub invert_drag_y: bool,
    /// Turning the wheel towards you zooms in instead of out
    #[serde(default)]
    #[schemars(title = "Invert wheel zoom")]
    pub invert_wheel: bool,
    /// Pinching zooms in instead of out on a trackpad or touch screen
    #[serde(default)]
    #[schemars(title = "Invert pinch zoom")]
    pub invert_pinch: bool,
    /// In Trackpad mode, the view moves against your fingers instead of with them
    #[serde(default)]
    #[schemars(title = "Invert two-finger pan", extend("x-enable-when" = {"trackpad": true}))]
    pub invert_pan: bool,
    /// The shortcuts changed from Cut Craft's own, by action, written as the menus write
    /// them (`Ctrl+K`, `Mouse Back`); an empty list takes an action's shortcuts away. Set on
    /// this page's shortcut list, below its settings.
    #[serde(default, skip_serializing_if = "std::collections::BTreeMap::is_empty")]
    #[schemars(skip)]
    pub shortcuts: std::collections::BTreeMap<String, Vec<String>>,
}

/// What the wheel (or two fingers) and a pinch do to a 2D or 3D view in one frame.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ViewMove {
    /// How much bigger the view is drawn, about the pointer (1 for no change).
    pub zoom: f32,
    /// How far the drawing moves on the screen, in points.
    pub pan: egui::Vec2,
}

impl Controls {
    /// What the wheel's or two fingers' `scroll` and a `pinch` (a factor, 1 for none) do to
    /// a view `height` points tall: the wheel zooms (a quarter of the height doubles it) and
    /// tilting it pans sideways, or in Trackpad mode two fingers pan; a pinch always zooms.
    /// Each turned the other way when its Invert box is ticked.
    pub fn view_move(&self, scroll: egui::Vec2, pinch: f32, height: f32) -> ViewMove {
        let pinch = if self.invert_pinch && pinch > 0.0 {
            1.0 / pinch
        } else {
            pinch
        };
        if self.trackpad {
            let pan = if self.invert_pan { -scroll } else { scroll };
            return ViewMove { zoom: pinch, pan };
        }
        let wheel = if self.invert_wheel {
            -scroll.y
        } else {
            scroll.y
        };
        let zoom = if wheel == 0.0 {
            1.0
        } else {
            2f32.powf(wheel / (height / crate::app::VIEW_ZOOM_QUARTERS).max(1.0))
        };
        ViewMove {
            zoom: zoom * pinch,
            pan: egui::vec2(self.tilt.pan(scroll.x), 0.0),
        }
    }

    /// A drag across the 3D view as it turns the view: each way flipped when its Invert box
    /// is ticked.
    pub fn turn(&self, drag: egui::Vec2) -> egui::Vec2 {
        let flip = |v: f32, on: bool| if on { -v } else { v };
        egui::vec2(
            flip(drag.x, self.invert_drag_x),
            flip(drag.y, self.invert_drag_y),
        )
    }
}

/// What tilting the mouse wheel sideways does.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Tilt {
    /// Tilting right moves the view to show more to the right, as a web page scrolls
    #[default]
    #[schemars(title = "Pans the view sideways")]
    Pan,
    /// Tilting right moves the drawing to the right, for those whose wheel feels backwards
    #[schemars(title = "Pans the other way")]
    PanReversed,
    /// Tilting the wheel is ignored by the views
    #[schemars(title = "Does nothing")]
    Nothing,
}

impl Tilt {
    /// How far the views pan sideways for a sideways scroll of `x` points: nothing, `x`, or
    /// the other way.
    pub fn pan(self, x: f32) -> f32 {
        match self {
            Self::Pan => x,
            Self::PanReversed => -x,
            Self::Nothing => 0.0,
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
    /// shorter files; 0 keeps every point, 0.1 mm is the most. The keep-out zones are
    /// checked on what is saved. Profile, pocket and other 2D toolpaths
    #[serde(default = "tolerance")]
    #[schemars(title = "2D toolpath tolerance", extend("x-unit" = "mm"), range(min = 0, max = 0.1))]
    pub tolerance_2d: f64,
    /// The same for 3D roughing, finishing, texture, moulding and the like
    #[serde(default = "tolerance")]
    #[schemars(title = "3D toolpath tolerance", extend("x-unit" = "mm"), range(min = 0, max = 0.1))]
    pub tolerance_3d: f64,
    /// The same for V-carving, prism, inlay and chamfer toolpaths
    #[serde(default = "tolerance")]
    #[schemars(title = "V-Carve (V-bit carving) toolpath tolerance", extend("x-unit" = "mm"), range(min = 0, max = 0.1))]
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
    /// the zone, as if the zone were cut out of whatever is under it. Either way, moves
    /// across a zone go round it. Laser and drill bank toolpaths are never clipped: move the
    /// part or the zone for those
    #[serde(default)]
    #[schemars(title = "Clip toolpaths at keep-out zones")]
    pub clip_at_keep_out_zones: bool,
    /// How long changing a bit by hand takes, for the cutting times and Suggest's times on
    /// Pocket's clearing bits
    #[serde(default = "change_by_hand")]
    #[schemars(title = "Bit change by hand", extend("x-unit" = "min"), range(min = 0))]
    pub change_by_hand: f64,
    /// How long the tool changer takes to change a bit, the same way
    #[serde(default = "change_by_changer")]
    #[schemars(title = "Bit change by tool changer", extend("x-unit" = "min"), range(min = 0))]
    pub change_by_changer: f64,
}

/// Two minutes to stop, change a bit by hand and zero it again.
fn change_by_hand() -> f64 {
    2.0
}

/// A quarter of a minute for a tool changer.
fn change_by_changer() -> f64 {
    0.25
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
// ponytail: our own defaults
fn tolerance() -> f64 {
    0.002_54
}

/// The most a toolpath tolerance can be, mm (the form's limit, and a hand-edited file's):
/// far below a keep-out zone's clearance or a cut's accuracy.
pub const MAX_TOLERANCE: f64 = 0.1;

impl ToolpathSettings {
    /// The tolerance for a toolpath of this family.
    pub fn tolerance(&self, family: cut_craft_core::simplify::Family) -> f64 {
        use cut_craft_core::simplify::Family;
        match family {
            Family::TwoD => self.tolerance_2d,
            Family::ThreeD => self.tolerance_3d,
            Family::VCarve => self.tolerance_vcarve,
        }
        .clamp(0.0, MAX_TOLERANCE)
    }

    /// `doc` with each toolpath's cuts thinned to its tolerance, ready to save.
    pub fn thinned(
        &self,
        doc: &cut_craft_core::document::Document,
    ) -> cut_craft_core::document::Document {
        use cut_craft_core::simplify::{family, simplify};
        let mut doc = doc.clone();
        for toolpath in doc.toolpaths.values_mut() {
            // a laser's power goes move by move, and so does how far a rotary toolpath holds
            // the bit off the turning axis, so their moves stay as they are (thinning them
            // alone would pair each move with another move's power or place across the axis)
            if toolpath.power.is_empty() && toolpath.across.is_empty() {
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
    /// The checks before cutting warn when a tool's taper or shape will leave more material
    /// than this on the model. Raise it if the warnings are about bits too small to matter
    #[serde(default = "material_left")]
    #[schemars(
        title = "Warn when material left is over",
        extend("x-unit" = "mm"),
        range(min = 0, max = 10)
    )]
    pub material_left: f64,
    /// After an edit leaves toolpaths out of date, ask whether to recalculate them all. Off,
    /// they are still marked ⚠ in the Toolpaths list, and saving them still asks first
    #[serde(default = "ask_to_recalculate")]
    #[schemars(title = "Ask to recalculate out-of-date toolpaths")]
    pub ask_to_recalculate: bool,
    /// Saving toolpaths where an earlier save of the same name left G-code: ask, replace it
    /// (the older numbered files go too, so none is left to run by mistake), or archive it
    /// (moved into a dated folder under Archive beside it) and add the new files
    #[serde(default)]
    #[schemars(title = "G-code from an earlier save")]
    pub earlier_gcode: EarlierGcode,
}

impl Default for Confirmations {
    fn default() -> Self {
        Self {
            toolpath_intersections: IntersectionCheck::default(),
            radiused_offset_intersections: IntersectionCheck::default(),
            sharp_offset_intersections: IntersectionCheck::default(),
            material_left: material_left(),
            ask_to_recalculate: ask_to_recalculate(),
            earlier_gcode: EarlierGcode::default(),
        }
    }
}

fn ask_to_recalculate() -> bool {
    true
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
pub enum EarlierGcode {
    /// Ask each time: Replace or Archive
    #[default]
    Ask,
    /// Replace the earlier files
    Replace,
    /// Move the earlier files into a dated folder under Archive, then add the new ones
    Archive,
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

/// How Cut Craft starts. What new jobs start with is kept in `default_job`; the Job
/// Defaults page shows its settings under these.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct JobDefaults {
    /// Start with the start page (recent files, the sample project) and fill in every job
    /// setting before a new job opens, as Cut Craft used to. Off opens a blank job straight
    /// away, made with the settings below; Setup Material changes its size any time
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    #[schemars(title = "Classic Startup Flow")]
    pub classic_startup: bool,
}

/// The Options page of what new jobs start with.
pub const JOB_DEFAULTS: &str = "job_defaults";

/// General settings.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct General {
    /// How many files File ▸ Recent Files and the start page list. Up to 20 are always
    /// remembered, so a smaller number only hides the rest until it is raised again
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
    /// Open a short Did You Know? tip each time Cut Craft starts, a different one each
    /// time. Help ▸ Did You Know? shows them whenever you like
    #[serde(default = "on")]
    #[schemars(title = "Show a tip when Cut Craft starts")]
    pub show_tips_at_startup: bool,
}

#[expect(
    clippy::trivially_copy_pass_by_ref,
    reason = "serde's skip_serializing_if"
)]
fn is_zero(n: &usize) -> bool {
    *n == 0
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
/// Model offers; and the line width of exported PDF, EPS and AI files.
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
    /// How thick the lines are in exported PDF, EPS and AI files, in points (a point is
    /// 1/72 inch). Thicker shows better on paper; laser and vinyl cutter software often
    /// wants a hairline, so set it thin for those
    #[serde(default = "export_line_points")]
    #[schemars(
        title = "Exported PDF, EPS and AI line width",
        extend("x-unit" = "pt"),
        range(min = 0.05, max = 10)
    )]
    pub export_line_points: f64,
}

fn export_line_points() -> f64 {
    fmt_export_pdf::LINE
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
        fmt_export_pdf::set_line_width(self.export_line_points);
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
            export_line_points: export_line_points(),
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
    /// How much work a gadget may do, in millions of Lua steps, before it is stopped as
    /// stuck. Higher lets big gadgets finish; a gadget caught in a loop then freezes Cut Craft
    /// for longer before it is stopped
    #[serde(
        default = "gadget_million_instructions",
        skip_serializing_if = "is_gadget_million_instructions"
    )]
    #[schemars(
        title = "Gadgets: most work (million steps)",
        range(min = 10, max = 5000)
    )]
    pub gadget_million_instructions: u32,
    /// How much memory a gadget may use before it is stopped. Higher lets big gadgets finish,
    /// but a runaway one can take that much from the computer
    #[serde(
        default = "gadget_memory_mb",
        skip_serializing_if = "is_gadget_memory_mb"
    )]
    #[schemars(
        title = "Gadgets: most memory",
        extend("x-unit" = "MB"),
        range(min = 8, max = 1024)
    )]
    pub gadget_memory_mb: u32,
    /// How many commands one gadget run may do. Higher lets a gadget make more shapes in one
    /// go, but each one adds to the job's memory and to the time Undo takes
    #[serde(
        default = "gadget_most_commands",
        skip_serializing_if = "is_gadget_most_commands"
    )]
    #[schemars(
        title = "Gadgets: most commands in one run",
        range(min = 100, max = 1_000_000)
    )]
    pub gadget_most_commands: u32,
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

advanced_default!(
    gadget_million_instructions,
    is_gadget_million_instructions,
    u32::try_from(cut_craft_scripting::Limits::default().instructions / 1_000_000)
        .expect("a count of millions")
);
advanced_default!(
    gadget_memory_mb,
    is_gadget_memory_mb,
    u32::try_from(cut_craft_scripting::Limits::default().memory >> 20).expect("a count of MB")
);
advanced_default!(
    gadget_most_commands,
    is_gadget_most_commands,
    u32::try_from(cut_craft_scripting::Limits::default().commands).expect("a small count")
);

/// A count from the options file as a `usize`.
fn count(n: u32) -> usize {
    usize::try_from(n).unwrap_or(usize::MAX)
}

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
            gadget_million_instructions: gadget_million_instructions(),
            gadget_memory_mb: gadget_memory_mb(),
            gadget_most_commands: gadget_most_commands(),
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
        let (relist, most, unused_days, keep) = self.library_limits();
        crate::clipart::limits::set(relist, most, unused_days, keep);
        cut_craft_core::accuracy::set(self.accuracy());
        cut_craft_scripting::set_gadget_limits(self.gadget_limits());
    }

    /// What a gadget may do before it is stopped, each kept to its range.
    pub fn gadget_limits(&self) -> cut_craft_scripting::Limits {
        cut_craft_scripting::kept_to_ranges(cut_craft_scripting::Limits {
            instructions: u64::from(self.gadget_million_instructions) * 1_000_000,
            memory: count(self.gadget_memory_mb) << 20,
            commands: count(self.gadget_most_commands),
        })
    }

    /// The Library's rescan seconds, most files, days an unused preview is kept and previews
    /// on the graphics card, each kept to its range.
    pub fn library_limits(&self) -> (u64, usize, u64, usize) {
        (
            u64::from(self.library_rescan_seconds.clamp(1, 3600)),
            count(self.library_most_files.clamp(1000, 10_000_000)),
            u64::from(self.preview_cache_days.clamp(1, 3650)),
            count(self.previews_on_card.clamp(50, 5000)),
        )
    }

    /// How exactly toolpaths are worked out, in mm, each kept to its range.
    pub fn accuracy(&self) -> cut_craft_core::accuracy::Accuracy {
        let mm = |um: u32| f64::from(um) / 1000.0;
        cut_craft_core::accuracy::kept_to_ranges(cut_craft_core::accuracy::Accuracy {
            curve: mm(self.curve_accuracy),
            roughing_3d: mm(self.roughing_3d_accuracy),
            finishing_3d: mm(self.finishing_3d_accuracy),
            rest: mm(self.rest_threshold),
            nest_sheets: count(self.nest_most_sheets),
            nest_fillers: count(self.nest_most_fillers),
        })
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

/// What reading the options file found: the options, whether the advanced ones were
/// changed outside Cut Craft, and the settings that could not be read.
#[derive(Debug, Default)]
pub struct Read {
    pub options: Options,
    pub tampered: bool,
    /// Each setting left at its default, as `section ▸ setting` (or the section, or
    /// `everything`); empty when the whole file was read.
    pub unread: Vec<String>,
}

/// What the status bar says when some settings could not be read.
pub fn unread_message(unread: &[String]) -> String {
    format!(
        "Some settings could not be read, so they are back to their defaults: {}. Your old \
         settings file is kept beside the new one as options.json.bak. Set them again in \
         Edit ▸ Options.",
        unread.join(", ")
    )
}

/// What the status bar says when the advanced settings were changed outside Cut Craft.
pub const TAMPERED: &str = "Advanced settings were changed outside Cut Craft, so they were put \
    back to their safe defaults. Change them in Edit ▸ Options ▸ Advanced.";

/// What the status bar says after the options are read: what could not be read, and that
/// the advanced settings were put back, both when both happened; nothing when all was well.
pub fn load_message(unread: &[String], tampered: bool) -> Option<String> {
    match (unread.is_empty(), tampered) {
        (true, false) => None,
        (true, true) => Some(TAMPERED.to_owned()),
        (false, false) => Some(unread_message(unread)),
        (false, true) => Some(format!("{} {TAMPERED}", unread_message(unread))),
    }
}

/// Help ▸ Check for Updates, and the check at startup.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Updates {
    /// About once a day, when Cut Craft starts, ask GitHub whether there is a new version,
    /// and say so if there is. Nothing is downloaded or installed until you choose Install Now
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

/// Options ▸ Web API: lets a script, a test or Claude drive Cut Craft over HTTP, the way you
/// do with the mouse. Off until switched on.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct WebApi {
    /// Let programs on this computer drive Cut Craft: select vectors, run tools, press any
    /// button. Calls from this computer need nothing more; other computers need the access
    /// token. The status bar shows WEB API Running in green while it is listening, or in red if
    /// it could not start and why
    #[serde(default)]
    #[schemars(title = "Turn on the Web API")]
    pub enabled: bool,
    /// Also answer other computers on your network, not only this one (listens on 0.0.0.0).
    /// Every call then needs the access token shown below. Calls travel unencrypted, so only
    /// use this on a home or workshop network you trust
    #[serde(default)]
    #[schemars(title = "Allow other computers on my network")]
    pub allow_network: bool,
    /// The number programs call it on, the 7340 in localhost:7340. Pick another if a different
    /// program already uses this one
    #[serde(default = "web_api_port")]
    #[schemars(title = "Port", range(min = 1024, max = 65535))]
    pub port: u16,
    /// The access token calls must send while other computers are allowed; made the first
    /// time the Web API is switched on.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    #[schemars(skip)]
    pub token: String,
}

fn web_api_port() -> u16 {
    cut_craft_web_api::DEFAULT_PORT
}

impl Default for WebApi {
    fn default() -> Self {
        Self {
            enabled: false,
            allow_network: false,
            port: web_api_port(),
            token: String::new(),
        }
    }
}

impl WebApi {
    /// What the server runs with, or `None` while it is switched off.
    pub fn server_settings(&self) -> Option<cut_craft_web_api::Settings> {
        self.enabled.then(|| cut_craft_web_api::Settings {
            allow_network: self.allow_network,
            port: self.port,
            // this computer's own programs need no token; open to the network, every call does
            token: self.allow_network.then(|| self.token.clone()),
        })
    }
}

/// Bug reports, and what happens after Cut Craft closes because of an error.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct BugReports {
    /// If Cut Craft closes because of an error, it keeps a crash log saying what it was
    /// doing. The next time it starts it can open a bug report with that log, for you to
    /// read and send. Your user name and home folder are left out of it, and nothing is sent
    /// until you press Submit.
    #[serde(default)]
    #[schemars(title = "If Cut Craft crashes")]
    pub crash_logs: CrashLogs,
    /// Send which system and version this computer runs (Windows 11 23H2, macOS 14 Sonoma,
    /// Ubuntu 24.04...), its processor (Intel/AMD or ARM) and graphics card with crash and
    /// bug reports: some bugs happen only on some computers. Never the computer's name or
    /// your user name. Off: only the report and its log are sent
    #[serde(default = "yes")]
    #[schemars(title = "Send details of my computer")]
    pub computer_details: bool,
    /// A random number for this install, made when a crash report is first sent, so the
    /// crash-report service can tell one install from another. Nothing about the person.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(skip)]
    pub install_id: Option<String>,
}

impl Default for BugReports {
    fn default() -> Self {
        Self {
            crash_logs: CrashLogs::default(),
            computer_details: true,
            install_id: None,
        }
    }
}

/// Whether a crash's log is offered as a bug report at the next start.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum CrashLogs {
    /// Send the crash log to Cut Craft's crash-report service by itself (your user name,
    /// folders and file names left out). Where that can't be done, open a bug report with it ticked
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
    /// The colors vectors, toolpaths and rapid moves are drawn in. Standard: as Cut Craft
    /// has always drawn them. Color-blind friendly: colors told apart by how light or dark
    /// they are, not only by their hue. My own colors: pick each one below, for each theme
    #[serde(default)]
    #[schemars(title = "Vector and toolpath colors")]
    pub line_colors: LinePalette,
    /// The 2D view's colors while the light theme is on
    #[serde(default = "CanvasColours::light")]
    #[schemars(title = "2D view in the light theme")]
    pub light_2d: CanvasColours,
    /// The 2D view's colors while the dark theme is on
    #[serde(default = "CanvasColours::dark")]
    #[schemars(title = "2D view in the dark theme")]
    pub dark_2d: CanvasColours,
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
            line_colors: LinePalette::default(),
            light_2d: CanvasColours::light(),
            dark_2d: CanvasColours::dark(),
            grid_point_size: grid_point_size(),
            text_size: text_size(),
        }
    }
}

impl Appearance {
    /// The 2D view's colours for the dark theme, or for the light one.
    pub fn canvas(&self, dark: bool) -> &CanvasColours {
        if dark { &self.dark_2d } else { &self.light_2d }
    }

    /// The colours the 2D view draws vectors, toolpaths and rapid moves in (vectors in the
    /// 3D view too), for the dark theme or the light one, as [`Self::line_colors`] says.
    pub fn lines(&self, dark: bool) -> Lines {
        let (visuals, own) = if dark {
            (egui::Visuals::dark(), &self.dark_2d)
        } else {
            (egui::Visuals::light(), &self.light_2d)
        };
        match self.line_colors {
            LinePalette::Standard => Lines::standard(&visuals),
            LinePalette::ColorBlind => Lines::colour_blind(dark),
            LinePalette::MyOwn => Lines {
                vector: own.vector_color,
                toolpath: own.toolpath_color,
                rapid: own.rapid_color,
            },
        }
    }

    /// What egui's text and control sizes are multiplied by, kept between 80% and 200%.
    pub fn text_scale(&self) -> f32 {
        #[expect(clippy::cast_precision_loss, reason = "80 to 200")]
        let percent = self.text_size.clamp(80, 200) as f32;
        percent / 100.0
    }
}

/// The colors of the 2D view's material sheet and of the snap grid's points over it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[expect(
    clippy::struct_field_names,
    reason = "the names options.json has kept them under"
)]
pub struct CanvasColours {
    /// The material sheet the drawing sits on
    #[schemars(title = "Sheet color", extend("x-kind" = "colour"))]
    pub sheet_color: [u8; 3],
    /// The snap grid's points: pick one that stands out from the sheet color
    #[schemars(title = "Grid color", extend("x-kind" = "colour"))]
    pub grid_color: [u8; 3],
    /// Your drawing's lines, the shapes toolpaths are made from
    #[schemars(
        title = "Vector color",
        extend("x-kind" = "colour", "x-show-when" = {"appearance.line_colors": "my_own"})
    )]
    pub vector_color: [u8; 3],
    /// Where the bit cuts: pick one that looks different from the vector color, lighter or
    /// darker as well as another hue
    #[schemars(
        title = "Toolpath color",
        extend("x-kind" = "colour", "x-show-when" = {"appearance.line_colors": "my_own"})
    )]
    pub toolpath_color: [u8; 3],
    /// The bit's quick moves above the material, and its plunges and lifts, when the
    /// Toolpath Drawing menu shows them
    #[schemars(
        title = "Rapid move color",
        extend("x-kind" = "colour", "x-show-when" = {"appearance.line_colors": "my_own"})
    )]
    pub rapid_color: [u8; 3],
}

impl CanvasColours {
    /// The light theme's: its own field colour for the sheet, a grey that stands out on it.
    pub fn light() -> Self {
        Self::of(&egui::Visuals::light())
    }

    /// The dark theme's, as [`Self::light`].
    pub fn dark() -> Self {
        Self::of(&egui::Visuals::dark())
    }

    /// The sheet in the theme's field colour, the grid as near it as stays easy to see.
    fn of(visuals: &egui::Visuals) -> Self {
        let sheet = rgb(visuals.widgets.inactive.bg_fill);
        let lines = Lines::standard(visuals);
        Self {
            sheet_color: sheet,
            grid_color: contrasting(sheet, rgb(visuals.text_color())),
            vector_color: lines.vector,
            toolpath_color: lines.toolpath,
            rapid_color: lines.rapid,
        }
    }
}

/// Which colours vectors, toolpaths and rapid moves are drawn in.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum LinePalette {
    /// Vectors in the text color, toolpaths orange, rapid moves red: as Cut Craft has
    /// always drawn them
    #[default]
    Standard,
    /// Vectors black (white in the dark theme), toolpaths deep orange, rapid moves pink:
    /// told apart by how light or dark they are as well as by their hue
    #[schemars(title = "Color-blind friendly")]
    ColorBlind,
    /// The vector, toolpath and rapid move colors you pick for each theme below
    #[schemars(title = "My own colors")]
    MyOwn,
}

/// The colours the views draw vectors, toolpaths and rapid moves in, `[r, g, b]`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Lines {
    pub vector: [u8; 3],
    pub toolpath: [u8; 3],
    pub rapid: [u8; 3],
}

impl Lines {
    /// The theme's own: text, warning and error colours, as the views always drew them.
    fn standard(visuals: &egui::Visuals) -> Self {
        Self {
            vector: rgb(visuals.text_color()),
            toolpath: rgb(visuals.warn_fg_color),
            rapid: rgb(visuals.error_fg_color),
        }
    }

    /// From Okabe and Ito's colour-blind safe colours: vermillion toolpaths and reddish
    /// purple rapids (one warm, one blue-ish to a red-green colour-blind eye) against black
    /// or white vectors, far apart in lightness so hue is not needed to tell them apart.
    /// Blue is left to the selection.
    fn colour_blind(dark: bool) -> Self {
        Self {
            vector: if dark { [255, 255, 255] } else { [0, 0, 0] },
            toolpath: [213, 94, 0],
            rapid: [204, 121, 167],
        }
    }
}

/// The colour the grid was before each theme had its own: the light theme's faint text,
/// all but hidden on the dark theme's sheet. Options saved with it drop it.
fn old_grid_color() -> [u8; 3] {
    rgb(egui::Visuals::light().weak_text_color())
}

/// The least blend of `under` toward `toward` that stands out from `under` as well as
/// WCAG asks of shapes (3 to 1); black or white, whichever stands out more, if none does.
fn contrasting(under: [u8; 3], toward: [u8; 3]) -> [u8; 3] {
    const ENOUGH: f64 = 3.0;
    let mixed = |t: f64| {
        let mix = |a: u8, b: u8| {
            #[expect(
                clippy::cast_possible_truncation,
                clippy::cast_sign_loss,
                reason = "between two bytes"
            )]
            let v = (f64::from(a) + (f64::from(b) - f64::from(a)) * t).round() as u8;
            v
        };
        [
            mix(under[0], toward[0]),
            mix(under[1], toward[1]),
            mix(under[2], toward[2]),
        ]
    };
    (1..=20)
        .map(|step| mixed(f64::from(step) / 20.0))
        .find(|c| contrast(*c, under) >= ENOUGH)
        .unwrap_or_else(|| {
            let (black, white) = ([0, 0, 0], [255, 255, 255]);
            if contrast(black, under) > contrast(white, under) {
                black
            } else {
                white
            }
        })
}

/// WCAG's contrast ratio between two colours, from 1 (the same) to 21 (black on white).
fn contrast(a: [u8; 3], b: [u8; 3]) -> f64 {
    let luminance = |c: [u8; 3]| {
        let channel = |v: u8| {
            let v = f64::from(v) / 255.0;
            if v <= 0.040_45 {
                v / 12.92
            } else {
                ((v + 0.055) / 1.055).powf(2.4)
            }
        };
        0.2126 * channel(c[0]) + 0.7152 * channel(c[1]) + 0.0722 * channel(c[2])
    };
    let (la, lb) = (luminance(a), luminance(b));
    (la.max(lb) + 0.05) / (la.min(lb) + 0.05)
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
    /// Light; on a Mac the desktop's own, as Mac users expect apps to follow
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

// ponytail: the third choice, a folder remembered for each kind of operation, is left out
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

/// Which language Cut Craft's words are in. Its page lists the choices itself (the packs
/// installed), so it has no settings of the ordinary kind.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct Language {
    /// The language pack in use, by its add-on id (`language-ru`), or
    /// [`crate::languages::TEST_LANGUAGE`]; empty for English.
    #[serde(default)]
    #[schemars(skip)]
    pub pack: String,
}

/// The Language page's key.
pub const LANGUAGE: &str = "language";

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Options {
    #[schemars(title = "3D View Settings", extend("x-section" = true))]
    pub view_3d: View3d,
    #[schemars(title = "Toolpath Settings", extend("x-section" = true))]
    pub toolpaths: ToolpathSettings,
    #[schemars(title = "General Settings", extend("x-section" = true))]
    pub general: General,
    /// The language Cut Craft's words are in: English, built in, or a language pack
    /// installed from Gadgets ▸ Add-ons.
    #[serde(default)]
    #[schemars(title = "Language", extend("x-section" = true))]
    pub language: Language,
    /// How Cut Craft starts, and what every new job starts with: the material, where X0 Y0
    /// and Z zero are, the rapid heights and home (the job settings under it).
    #[serde(default)]
    #[schemars(title = "Job Defaults", extend("x-section" = true))]
    pub job_defaults: JobDefaults,
    #[serde(default)]
    #[schemars(title = "Confirmation Choices", extend("x-section" = true))]
    pub confirmations: Confirmations,
    #[serde(default)]
    #[schemars(title = "Importing and Exporting", extend("x-section" = true))]
    pub import: Import,
    #[serde(default)]
    #[schemars(extend("x-section" = true))]
    pub appearance: Appearance,
    #[serde(default)]
    #[schemars(title = "Mouse and Shortcuts", extend("x-section" = true))]
    pub controls: Controls,
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
    #[schemars(title = "Web API", extend("x-section" = true))]
    pub web_api: WebApi,
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
    /// own starting job until then. Set as Default changes only what is on its form, so the
    /// clearing kept by Clearing Settings' Use as My Default stays.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(skip)]
    pub default_job: Option<cut_craft_core::document::Job>,
    /// First-time setup was answered or skipped, so it is not shown at start again (Help ▸
    /// Run First-Time Setup Again still opens it). Kept here, in the options file, wherever
    /// the platform keeps that.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    #[schemars(skip)]
    pub first_run_done: bool,
    /// The Did You Know? tip shown at the next start, so each start shows another.
    #[serde(default, skip_serializing_if = "is_zero")]
    #[schemars(skip)]
    pub next_tip: usize,
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
    /// The Toolpaths sub-tabs shown as sections of their own, as `design_sections`.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[schemars(skip)]
    pub toolpath_sections: Vec<String>,
    /// The tool each toolbox flyout shows: the one picked from it last.
    #[serde(default, skip_serializing_if = "std::collections::BTreeMap::is_empty")]
    #[schemars(skip)]
    pub flyout_faces: crate::flyout::Faces,
    /// How many times each tool has been picked from a tool button, by id: the Ribbon folds
    /// the groups used least first when the window is too narrow for them all.
    #[serde(default, skip_serializing_if = "std::collections::BTreeMap::is_empty")]
    #[schemars(skip)]
    pub tool_uses: std::collections::BTreeMap<String, u32>,
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
                draw_vectors: d.vectors_3d,
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
                spin_speed: SPIN_SPEED,
                preview_detail: preview_detail(),
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
                change_by_hand: change_by_hand(),
                change_by_changer: change_by_changer(),
            },
            language: Language::default(),
            general: General {
                recent_file_list_size: u32::try_from(crate::files::RECENT_COUNT)
                    .expect("a small count"),
                preload_icons: on(),
                file_dialog_default: FileDialogDefault::default(),
                layout: WindowLayout::default(),
                autosave_seconds: autosave_seconds(),
                flyout_hold_seconds: flyout_hold_seconds(),
                fold_tool_groups: false,
                show_tips_at_startup: true,
            },
            job_defaults: JobDefaults::default(),
            confirmations: Confirmations::default(),
            snapping: cut_craft_core::snap::SnapOptions::default(),
            job_sheet: fmt_create_job_sheet::Style::default(),
            part_labels: fmt_part_labels::Style::default(),
            stl: fmt_export_as_stl::Settings::default(),
            import: Import::default(),
            appearance: Appearance::default(),
            advanced: Advanced::default(),
            controls: Controls::default(),
            space_mouse: SpaceMouse::default(),
            updates: Updates::default(),
            bug_reports: BugReports::default(),
            web_api: WebApi::default(),
            default_job: None,
            first_run_done: false,
            next_tip: 0,
            favourites: Favourites::default(),
            design_sections: vec![],
            toolpath_sections: vec![],
            flyout_faces: crate::flyout::Faces::new(),
            tool_uses: std::collections::BTreeMap::new(),
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
        Self::load_checked().options
    }

    /// [`Self::load`], and what could not be read of it: whether the advanced settings had
    /// been changed outside Cut Craft (and so were put back to their defaults), and the
    /// settings left at their defaults. A file with settings left out is first copied to
    /// `options.json.bak`, so saving the defaults over it loses nothing.
    pub fn load_checked() -> Read {
        let Some(path) = path() else {
            return Read::default();
        };
        let Ok(bytes) = std::fs::read(&path) else {
            return Read::default();
        };
        let read = Self::read_bytes(&bytes);
        if !read.unread.is_empty() {
            let _ = std::fs::write(path.with_extension("json.bak"), &bytes);
        }
        read
    }

    /// Options read from a file's bytes, the advanced ones only if their check matches; the
    /// defaults for a file that is not options at all.
    #[cfg(test)]
    pub fn from_bytes(bytes: &[u8]) -> (Self, bool) {
        let read = Self::read_bytes(bytes);
        (read.options, read.tampered)
    }

    /// [`Self::from_bytes`], naming what could not be read. A setting this version cannot
    /// read (one from a newer version, or a hand edit gone wrong) goes back to its default
    /// on its own: the page it is on, and the rest of the file, are kept.
    pub fn read_bytes(bytes: &[u8]) -> Read {
        let Ok(mut file) = serde_json::from_slice::<serde_json::Value>(bytes) else {
            return Read {
                unread: vec!["everything".to_owned()],
                ..Read::default()
            };
        };
        let seal = file
            .as_object_mut()
            .and_then(|o| o.remove(SEAL))
            .and_then(|s| s.as_str().map(String::from));
        // the one grid colour of older versions: kept for the light theme if it was chosen
        let old_grid = file
            .get_mut("appearance")
            .and_then(serde_json::Value::as_object_mut)
            .and_then(|a| a.remove("grid_color"))
            .and_then(|g| serde_json::from_value::<[u8; 3]>(g).ok());
        // each theme's colours from before line colours: the theme's own filled in
        if let Some(appearance) = file
            .get_mut("appearance")
            .and_then(serde_json::Value::as_object_mut)
        {
            for (key, theme) in [
                ("light_2d", CanvasColours::light()),
                ("dark_2d", CanvasColours::dark()),
            ] {
                let Some(saved) = appearance
                    .get_mut(key)
                    .and_then(serde_json::Value::as_object_mut)
                else {
                    continue;
                };
                if let serde_json::Value::Object(defaults) =
                    serde_json::to_value(theme).expect("colours serialise")
                {
                    for (name, value) in defaults {
                        saved.entry(name).or_insert(value);
                    }
                }
            }
        }
        let (mut options, unread) = match serde_json::from_value::<Self>(file.clone()) {
            Ok(options) => (options, vec![]),
            Err(_) => Self::read_each(file),
        };
        if let Some(grid) = old_grid.filter(|g| *g != old_grid_color()) {
            options.appearance.light_2d.grid_color = grid;
        }
        let tampered = !options.advanced.sealed_by(seal.as_deref());
        if tampered {
            options.advanced = Advanced::default();
        }
        Read {
            options,
            tampered,
            unread,
        }
    }

    /// A file that does not read whole, read setting by setting onto the defaults: each
    /// section kept if it reads, else each of its settings that does; and what was left out.
    fn read_each(file: serde_json::Value) -> (Self, Vec<String>) {
        let reads = |v: &serde_json::Value| serde_json::from_value::<Self>(v.clone()).is_ok();
        let mut kept = serde_json::to_value(Self::default()).expect("options serialise");
        let mut unread = vec![];
        let serde_json::Value::Object(sections) = file else {
            return (Self::default(), vec!["everything".to_owned()]);
        };
        for (section, value) in sections {
            let mut trial = kept.clone();
            trial[&section] = value.clone();
            if reads(&trial) {
                kept = trial;
                continue;
            }
            let (Some(settings), true) = (value.as_object(), kept[&section].is_object()) else {
                unread.push(section);
                continue;
            };
            for (name, setting) in settings {
                let mut trial = kept.clone();
                trial[&section][name] = setting.clone();
                if reads(&trial) {
                    kept = trial;
                } else {
                    unread.push(format!("{section} ▸ {name}"));
                }
            }
        }
        let options = serde_json::from_value(kept).unwrap_or_default();
        (options, unread)
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
        // the Job Defaults page shows the job new jobs start with: back to Cut Craft's own
        if page == JOB_DEFAULTS
            && let Some(values) = values.as_object_mut()
        {
            values.remove("default_job");
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
        d.vectors_3d = v.draw_vectors;
        d.colour_shaded = v.color_shaded_view;
        d.shaded_background = v.shaded_background;
        d.light_follows = v.light_follows_viewer;
        d.auto_open_3d = t.auto_open_3d_view;
        d.previews_2d = t.create_2d_previews;
        d.solid_previews = t.solid_2d_previews;
        d.solid_colour = t.solid_2d_preview_color;
        d.keep_out_labels = t.keep_out_labels;
        d.detail = v.preview_detail.min(100);
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
mod tests;
