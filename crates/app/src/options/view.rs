//! The 3D view and the ways to move it: mouse, keys and the SpaceMouse.

use super::*;

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

pub(super) fn preview_detail() -> u8 {
    crate::preview_detail::DEFAULT_PERCENT
}

pub(super) fn spin_speed() -> f64 {
    SPIN_SPEED
}

/// Degrees a second Auto-rotate turns the 3D view, until Options says otherwise.
pub const SPIN_SPEED: f64 = 20.0;

pub(super) fn click_reach() -> f32 {
    crate::view3d::PICK_REACH
}

pub(super) fn perspective_angle() -> f64 {
    crate::view3d::FIELD_OF_VIEW.to_degrees()
}

pub(super) fn yes() -> bool {
    true
}

pub(super) fn target_fps() -> f64 {
    TARGET_FPS
}

/// Frames a second the 3D view aims for while it moves, until Options says otherwise.
pub const TARGET_FPS: f64 = 30.0;

pub(super) fn settle_seconds() -> f64 {
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
    pub(super) const ALL: [Self; 4] = [Self::Low, Self::Medium, Self::High, Self::VeryHigh];

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

    pub(super) fn at(self) -> usize {
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

pub(super) fn one() -> f64 {
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
pub(super) fn background_top() -> [u8; 3] {
    rgb(egui::Visuals::light().faint_bg_color)
}

pub(super) fn background_bottom() -> [u8; 3] {
    rgb(egui::Visuals::light().extreme_bg_color)
}

pub(super) fn rgb(c: egui::Color32) -> [u8; 3] {
    [c.r(), c.g(), c.b()]
}
