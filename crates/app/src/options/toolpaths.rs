//! Toolpath settings, the questions Cut Craft asks, job defaults, General and Import.

use super::{
    Deserialize, Display, FileDialogDefault, JsonSchema, RibbonIcons, Serialize, WindowLayout,
    egui, on,
};

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
    /// plays each move at its bit's feed or plunge rate, rapids at the rapid rate; the
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
    /// made. On: it is made with the part under the zone left uncut, the bit lifting over
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
    /// Off: a G-code file's name keeps its spaces, as typed. On: every space becomes an
    /// underscore, for controllers and USB readers that cannot open names with spaces
    #[serde(default)]
    #[schemars(title = "Underscores for spaces in G-code file names")]
    pub file_name_underscores: bool,
    /// When toolpaths are saved to multiple files, each file's name says the bit to fit as
    /// well as the toolpath
    #[serde(default = "on")]
    #[schemars(title = "Bit name in multiple file names")]
    pub bit_in_file_names: bool,
}

/// Two minutes to stop, change a bit by hand and zero it again.
pub(super) fn change_by_hand() -> f64 {
    2.0
}

/// A quarter of a minute for a tool changer.
pub(super) fn change_by_changer() -> f64 {
    0.25
}

pub(super) fn preview_speed() -> f64 {
    crate::app::PLAYBACK_SPEED
}

impl ToolpathSettings {
    /// How saved G-code files are named.
    pub fn file_names(&self) -> crate::files::FileNames {
        crate::files::FileNames {
            underscores: self.file_name_underscores,
            bit_names: self.bit_in_file_names,
        }
    }

    /// The preview's starting speed, times the machine's, kept to the slider's range.
    pub fn preview_speed(&self) -> f64 {
        self.preview_multiplier.clamp(0.25, 1000.0)
    }
}

pub(super) fn solid_colour() -> [u8; 3] {
    Display::default().solid_colour
}

/// A ten-thousandth of an inch.
// ponytail: our own defaults
pub(super) fn tolerance() -> f64 {
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

pub(super) fn ask_to_recalculate() -> bool {
    true
}

pub(super) fn material_left() -> f64 {
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
    /// How the window is arranged: a ribbon of tabs over the view, a rail of tool groups
    /// down the left, or the Classic Design and Toolpaths pages with side tabs. Every tool
    /// is in every layout
    #[serde(default = "WindowLayout::before_the_setting")]
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
    /// How many tools each group shows as icons in the Ribbon layout. Compact shows those
    /// that fit under the group's name, and the name lists the rest (pin any tool to show
    /// it); Every Tool shows them all, the groups going on to more rows as they need
    #[serde(default)]
    #[schemars(title = "Ribbon icons")]
    pub ribbon_icons: RibbonIcons,
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
pub(super) fn is_zero(n: &usize) -> bool {
    *n == 0
}

pub(super) fn autosave_seconds() -> u32 {
    // a whole number of seconds
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "30"
    )]
    let seconds = crate::app::AUTOSAVE_SECONDS as u32;
    seconds
}

pub(super) fn flyout_hold_seconds() -> f64 {
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

pub(super) fn export_line_points() -> f64 {
    fmt_export_pdf::LINE
}

pub(super) fn preview_faces() -> u32 {
    250_000
}

pub(super) fn picture_pixels_per_inch() -> f64 {
    cut_craft_core::import_settings::PIXELS_PER_INCH
}

pub(super) fn out_of_bounds_percent() -> u8 {
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
