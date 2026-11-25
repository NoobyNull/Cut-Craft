//! The job: its units, material, zero, rotary setup, model position and home.

use super::{Deserialize, Point, Rect, Serialize};

/// Display units. The document always stores millimetres.
#[derive(
    Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema,
)]
pub enum Units {
    /// Lengths shown in millimetres
    #[default]
    #[schemars(title = "mm")]
    Mm,
    /// Lengths shown in inches
    #[schemars(title = "inches")]
    Inch,
}

impl Units {
    /// Millimetres in one of these units.
    pub fn per(self) -> f64 {
        match self {
            Self::Mm => 1.0,
            Self::Inch => 25.4,
        }
    }

    /// The short name lengths are shown with: `mm` or `in`.
    pub fn suffix(self) -> &'static str {
        match self {
            Self::Mm => "mm",
            Self::Inch => "in",
        }
    }

    /// `mm` as these units show it, with its unit: `decimals` places in millimetres, two more
    /// in inches (an inch being 25.4 times longer), so `12.7 mm` reads `0.500 in`, not
    /// `12.7 in`.
    pub fn length(self, mm: f64, decimals: usize) -> String {
        match self {
            Self::Mm => format!("{mm:.decimals$} mm"),
            Self::Inch => format!("{:.*} in", decimals + 2, mm / 25.4),
        }
    }

    /// An area of `mm2` square millimetres, as [`Units::length`] shows a length.
    pub fn area(self, mm2: f64, decimals: usize) -> String {
        match self {
            Self::Mm => format!("{mm2:.decimals$} mm²"),
            Self::Inch => format!("{:.*} in²", decimals + 2, mm2 / (25.4 * 25.4)),
        }
    }

    /// A speed of `mm_per_min` (a feed rate), as [`Units::length`] shows a length but one
    /// more place in inches.
    pub fn feed(self, mm_per_min: f64, decimals: usize) -> String {
        match self {
            Self::Mm => format!("{mm_per_min:.decimals$} mm/min"),
            Self::Inch => format!("{:.*} in/min", decimals + 1, mm_per_min / 25.4),
        }
    }
}

/// Where Z = 0 is.
#[derive(
    Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum ZZero {
    /// The tool is zeroed on the top of the material
    #[default]
    #[schemars(title = "Material Surface")]
    MaterialSurface,
    /// The tool is zeroed under the material: on the machine bed, or on a rotary job's
    /// cylinder axis
    #[schemars(title = "Machine Bed")]
    MachineBed,
}

/// A rotary job: a cylinder turned by a rotary axis, worked on as its surface unrolled flat.
/// A double-sided job's setup.
#[derive(
    Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema,
)]
pub struct DoubleSided {
    /// How the material is turned over to reach the other side
    #[schemars(title = "Flip Direction Between Sides", extend("x-widget" = "row"))]
    pub flip: Flip,
    /// Z zero is on the same physical face whichever side is up
    #[schemars(title = "Zero off same side")]
    #[serde(default)]
    pub zero_same_side: bool,
}

#[derive(
    Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum Flip {
    /// Turned over left to right
    #[default]
    Horizontal,
    /// Turned over top to bottom
    Vertical,
}

impl Flip {
    /// Where a point of one side lies seen from the other, on material covering `area`.
    pub fn mirror(self, area: Rect) -> glam::DAffine2 {
        let c = area.center();
        let scale = match self {
            Self::Horizontal => Point::new(-1.0, 1.0),
            Self::Vertical => Point::new(1.0, -1.0),
        };
        glam::DAffine2::from_translation(c)
            * glam::DAffine2::from_scale(scale)
            * glam::DAffine2::from_translation(-c)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
pub struct Rotary {
    /// How far across the material is, straight up through the axis at A=0: a round blank's
    /// diameter, a square's side, a board's thickness. The unrolled sheet goes round a
    /// cylinder this size.
    #[schemars(extend("x-unit" = "mm"), range(min = 0))]
    pub diameter: f64,
    /// The direction of the cylinder's axis; the other direction goes round it
    pub along: Along,
    /// The material's shape across the axis: round, square, or a flat board
    #[serde(default)]
    pub blank: Blank,
    /// What of the material is left to hold the part while it is cut (added with support
    /// beams)
    #[serde(default)]
    pub support: Support,
}

/// What of a rotary job's material the rotary toolpaths leave to hold the part while it is
/// cut, so it doesn't flex or break free before the end.
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(default)]
pub struct Support {
    /// Where the part stays joined to the material: past the model's end there the material
    /// is left whole, so that end of the model is a flat face to saw it free on. Neither
    /// cuts the material away past both ends
    #[schemars(title = "Keep Joined At", extend("x-widget" = "drop-down"))]
    pub joined: Joined,
    /// Leave a round beam of material this far across along the turning axis, from the
    /// chuck to the tailstock, through the model and past it: the rotary toolpaths cut round
    /// it, so a long or thin part is held at both ends. 0 leaves none. Needs the material
    /// turning true on the axis (centred in the chuck). A last toolpath with Cut Away the
    /// Support ticked takes it off
    #[schemars(title = "Centre Stock Diameter", extend("x-unit" = "mm"), range(min = 0))]
    pub beam: f64,
}

/// The ends of a rotary job's material its part stays joined to.
#[derive(
    Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum Joined {
    /// Neither end: the material past the model is cut away at both ends
    #[default]
    Neither,
    /// The chuck end (the job's start)
    #[schemars(title = "Chuck end")]
    Chuck,
    /// The tailstock end (the job's end)
    #[schemars(title = "Tailstock end")]
    Tailstock,
    /// Both ends: the material is left whole past the model at the chuck and the
    /// tailstock, and the part is sawn free at both
    Both,
}

impl Joined {
    /// Whether the part stays joined at the chuck end, and at the tailstock end.
    #[must_use]
    pub fn ends(self) -> (bool, bool) {
        match self {
            Self::Neither => (false, false),
            Self::Chuck => (true, false),
            Self::Tailstock => (false, true),
            Self::Both => (true, true),
        }
    }
}

/// The shape of the material held on the rotary, across its axis.
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case", tag = "shape")]
pub enum Blank {
    /// A round blank: a dowel, a turned spindle or a log
    #[default]
    Round,
    /// A square blank, its faces flat to the top at A=0
    Square,
    /// A flat board lying flat at A=0, `width` across: its thickness is the job's diameter
    Board {
        #[schemars(extend("x-unit" = "mm"), range(min = 0))]
        width: f64,
    },
}

impl Rotary {
    /// A round blank `diameter` across, along `along`.
    #[must_use]
    pub fn round(diameter: f64, along: Along) -> Self {
        Self {
            diameter,
            along,
            blank: Blank::Round,
            support: Support::default(),
        }
    }

    /// How far out from the axis the material reaches as it turns, mm: what must clear the
    /// bed below and the bit above. A square swings by half its diagonal, a board by half of
    /// its across the corners, not half its thickness.
    #[must_use]
    pub fn swing(&self) -> f64 {
        let half = self.diameter / 2.0;
        match self.blank {
            Blank::Round => half,
            Blank::Square => half * std::f64::consts::SQRT_2,
            Blank::Board { width } => half.hypot(width / 2.0),
        }
    }
}

#[derive(
    Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum Along {
    /// The cylinder lies left to right
    #[default]
    #[schemars(title = "Along X Axis")]
    X,
    /// The cylinder lies front to back
    #[schemars(title = "Along Y Axis")]
    Y,
}

/// The point of the material the machine's X0, Y0 is.
#[derive(
    Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum Datum {
    /// The material's bottom left corner
    #[default]
    BottomLeft,
    /// The material's top left corner
    TopLeft,
    /// The material's top right corner
    TopRight,
    /// The material's bottom right corner
    BottomRight,
    /// The middle of the material
    Centre,
}

impl Datum {
    /// Where this datum is on a material of `size`, from its bottom-left corner.
    pub(super) fn on(self, size: Point) -> Point {
        match self {
            Self::BottomLeft => Point::ZERO,
            Self::TopLeft => Point::new(0.0, size.y),
            Self::TopRight => size,
            Self::BottomRight => Point::new(size.x, 0.0),
            Self::Centre => size / 2.0,
        }
    }
}

/// How finely the 3D model is worked out over the job.
#[derive(
    Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum Resolution {
    /// 1 million points
    #[default]
    #[schemars(title = "Standard (fastest)")]
    Standard,
    /// 3 million points
    #[schemars(title = "High (3x slower)")]
    High,
    /// 7 million points
    #[schemars(title = "Highest (7x slower)")]
    Highest,
}

impl Resolution {
    /// Model points spread over the whole job.
    pub fn points(self) -> f64 {
        match self {
            Self::Standard => 1e6,
            Self::High => 3e6,
            Self::Highest => 7e6,
        }
    }
}

/// Which side of the material Material Setup's gap is measured from.
#[derive(
    Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum GapFrom {
    /// Between the model's top and the material surface (Gap Outside Model on rotary jobs)
    #[default]
    #[schemars(title = "Gap Above Model")]
    Above,
    /// Between the model's bottom and the material's bottom (Gap Inside Model, from the
    /// axis, on rotary jobs)
    #[schemars(title = "Gap Below Model")]
    Below,
}

/// Material Setup's Model Position in Material.
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
pub struct ModelPosition {
    /// Which face of the material the gap is measured from: down from its top to the
    /// model's top, or up from its bottom to the model's bottom
    pub from: GapFrom,
    /// How far the model sits from the material's face
    #[schemars(extend("x-unit" = "mm"), range(min = 0))]
    pub gap: f64,
}

/// Which face of the material a model that does not fit pokes out of, and by how much (mm).
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Poke {
    /// Out of the material's top: the cutter would cut air over it
    Top(f64),
    /// Out of the material's bottom: below it, the cut would go into the spoilboard
    Bottom(f64),
}

impl ModelPosition {
    /// The gap above and the gap below a model `model` tall in material `thickness`
    /// thick: the gap typed on its side, what is left on the other. They always add up to
    /// the material less the model; one below 0 is the model sticking out.
    pub fn gaps(&self, thickness: f64, model: f64) -> (f64, f64) {
        let left = thickness - model - self.gap;
        match self.from {
            GapFrom::Above => (self.gap, left),
            GapFrom::Below => (left, self.gap),
        }
    }

    /// Where, and how far, a model `model` tall pokes out of material `thickness` thick
    /// placed this way; `None` when it fits.
    pub fn sticks_out(&self, thickness: f64, model: f64) -> Option<Poke> {
        // ponytail: 1 µm slack for rounding, as the toolpaths allow
        const SLACK: f64 = 1e-6;
        let (above, below) = self.gaps(thickness, model);
        if above < -SLACK {
            Some(Poke::Top(-above))
        } else if below < -SLACK {
            Some(Poke::Bottom(-below))
        } else {
            None
        }
    }
}

/// Material Setup's Home / Start Position: where the tool starts from and goes back to at the
/// end, X and Y in job coordinates and Z as a gap above the material.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
pub struct Home {
    /// Where the tool starts and ends, across the job
    #[schemars(title = "X", extend("x-unit" = "mm"))]
    pub x: f64,
    /// Where the tool starts and ends, up the job
    #[schemars(title = "Y", extend("x-unit" = "mm"))]
    pub y: f64,
    /// How high above the material it waits. Left at the clearance, it follows Clearance
    /// (Z1) when that is changed, so the start and end moves never travel lower
    #[schemars(title = "Z Gap above Material", extend("x-unit" = "mm"), range(min = 0))]
    pub z: f64,
}

/// The material block, in millimetres. Job coordinates are the machine's: the datum is at
/// `offset` (0, 0 unless an offset is used), so the material lies where [`Job::area`] says.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Job {
    pub width: f64,
    pub height: f64,
    pub thickness: f64,
    pub units: Units,
    /// Safe height above the material for rapid moves (added after the first job format).
    #[serde(default = "default_clearance")]
    pub clearance: f64,
    // added with the full Job Setup form; older files get the defaults
    #[serde(default)]
    pub z_zero: ZZero,
    #[serde(default)]
    pub datum: Datum,
    #[serde(default)]
    pub offset: Point,
    #[serde(default)]
    pub resolution: Resolution,
    /// The material's colour in the 3D view.
    #[serde(default = "default_colour")]
    pub colour: [u8; 3],
    /// How the preview colours what the toolpaths cut (added with preview colours).
    #[serde(default)]
    pub machined_area: MachinedArea,
    /// Global Fill Color's colour.
    #[serde(default = "default_fill")]
    pub fill_colour: [u8; 3],
    /// The lithophane preview's light, 0 (high contrast) to 1 (light, as with no light
    /// behind it).
    #[serde(default = "half_light")]
    pub lithophane_light: f64,
    /// Set for a rotary job (added with rotary jobs; flat jobs leave it out).
    /// Double-sided jobs: how the material is turned over between sides.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub double_sided: Option<DoubleSided>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rotary: Option<Rotary>,
    /// Where the 3D model sits in the material (Material Setup; added with it).
    #[serde(default)]
    pub model_position: ModelPosition,
    /// Plunge (Z2): the gap above the material that plunges go down to at the rapid rate
    /// before slowing to the plunge rate; unset, it is the clearance (no rapid plunges).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub plunge: Option<f64>,
    /// Home / Start Position; unset, X0 Y0 at the clearance.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub home: Option<Home>,
    /// How adaptive and trochoidal clearing cut, and raster rows coming back (Clearing
    /// Settings; added with them, older jobs get the defaults)
    #[serde(default)]
    pub clearing: crate::entry::ClearingSettings,
}

pub(super) fn default_clearance() -> f64 {
    Job::default().clearance
}

/// How the toolpath preview colours the areas it cuts.
#[derive(
    Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum MachinedArea {
    /// The material's own colour, as the uncut surface
    #[default]
    MaterialColor,
    /// Every machined area in the one fill colour
    GlobalFillColor,
    /// Each toolpath's areas in its own colour (none: the material's)
    ToolpathColor,
    /// Lit from behind: white where nothing is left, the material's colour at full
    /// thickness, between by what is left (added with the lithophane preview)
    Lithophane,
}

pub(super) fn half_light() -> f64 {
    0.5
}

pub(super) fn default_fill() -> [u8; 3] {
    Job::default().fill_colour
}

pub(super) fn default_colour() -> [u8; 3] {
    Job::default().colour
}

impl Job {
    /// A cylinder `length` long and `diameter` across, unrolled: `length` along its axis, the
    /// circumference the other way, and its radius deep, so Z zero under it is the axis.
    pub fn rotary(length: f64, diameter: f64, along: Along) -> Self {
        let round = std::f64::consts::PI * diameter;
        let (width, height) = match along {
            Along::X => (length, round),
            Along::Y => (round, length),
        };
        Self {
            width,
            height,
            thickness: diameter / 2.0,
            rotary: Some(Rotary::round(diameter, along)),
            ..Self::default()
        }
    }

    /// How far above the material's top the bit must be for A to turn under it: the
    /// clearance gap, and on a rotary job whose material swings out further than its top (a
    /// square or a board's corners) that much more.
    #[must_use]
    pub fn turning_gap(&self) -> f64 {
        let beyond = self
            .rotary
            .map_or(0.0, |r| (r.swing() - r.diameter / 2.0).max(0.0));
        self.clearance + beyond
    }

    /// Plunge (Z2) as a gap above the material.
    pub fn plunge_gap(&self) -> f64 {
        self.plunge.unwrap_or(self.clearance)
    }

    pub fn home(&self) -> Home {
        self.home.unwrap_or(Home {
            x: 0.0,
            y: 0.0,
            z: self.clearance,
        })
    }

    /// The material in job coordinates.
    pub fn area(&self) -> Rect {
        let size = Point::new(self.width, self.height);
        let min = self.offset - self.datum.on(size);
        Rect::new(min, min + size)
    }

    /// A toolpath height (from the material surface) as the machine is told it.
    pub fn machine_z(&self, z: f64) -> f64 {
        match self.z_zero {
            ZZero::MaterialSurface => z,
            ZZero::MachineBed => z + self.thickness,
        }
    }

    /// Where the machine's Z zero is, as a height from the material surface: 0 on top, or a
    /// thickness under it on the bed (a rotary job's axis, a radius under its top).
    pub fn zero_height(&self) -> f64 {
        -self.machine_z(0.0)
    }

    /// What to add to a model height for its depth from the material surface, for a model
    /// whose heights run from `lowest` to `top`, placed as Material Setup says.
    pub fn model_shift(&self, lowest: f64, top: f64) -> f64 {
        match self.model_position.from {
            GapFrom::Above => -(top + self.model_position.gap),
            GapFrom::Below => self.model_position.gap - self.thickness - lowest,
        }
    }

    /// Edge of one 3D model cell: the resolution's points spread over the job.
    pub fn model_cell(&self) -> f64 {
        (self.width * self.height / self.resolution.points()).sqrt()
    }
}

impl Default for Job {
    fn default() -> Self {
        // what Job Setup starts a new job with, until its Set as Default keeps another
        Self {
            width: 600.0,
            height: 400.0,
            thickness: 20.0,
            units: Units::Mm,
            clearance: 5.0,
            z_zero: ZZero::MaterialSurface,
            datum: Datum::BottomLeft,
            offset: Point::ZERO,
            resolution: Resolution::Standard,
            // pine, as Toolpaths ▸ Preview Colors has it
            colour: [214, 160, 102],
            machined_area: MachinedArea::MaterialColor,
            lithophane_light: 0.5,
            fill_colour: [70, 130, 180],
            rotary: None,
            double_sided: None,
            model_position: ModelPosition::default(),
            plunge: None,
            home: None,
            clearing: crate::entry::ClearingSettings::default(),
        }
    }
}
