//! A tool: its shape, sizes and cutting data.

use super::{Deserialize, JsonSchema, Serialize, pass_depths};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ToolShape {
    /// Flat bottomed
    #[default]
    EndMill,
    /// Round bottomed, for 3D finishing
    BallNose,
    /// Pointed, for V-carving
    VBit,
    /// Flat bottomed with rounded corners (a bull nose, or toroidal cutter): stronger than an
    /// end mill, leaves less cusp than one when 3D roughing or finishing
    #[schemars(title = "Radiused End Mill")]
    RadiusedEndMill,
    /// A single-point thread mill: a triangular tooth standing out from a narrower shank,
    /// for the Thread Milling toolpath
    #[schemars(title = "Thread Mill")]
    ThreadMill,
    /// Cuts the profile it was given: half its section, drawn as a vector (a moulding or
    /// router bit's shape)
    #[schemars(title = "Form Tool")]
    FormTool,
    /// A wide, flat bit for flattening a spoilboard or a slab (a spoilboard bit or fly
    /// cutter): cuts like an end mill, but only sideways, never plunging into the material
    #[schemars(title = "Surfacing Bit")]
    SurfacingBit,
    /// A laser, for the laser toolpaths: its diameter is the kerf (the width it burns), its
    /// feed rate the speed it moves at. It has no pass depth or flutes: Laser Cut / Fill
    /// sets how many passes it makes
    Laser,
}

impl ToolShape {
    /// Flat bottomed: an end mill or a surfacing bit, which cut the same shape (and a
    /// laser, whose burn the preview shows as a flat cut its kerf wide).
    pub fn is_flat(self) -> bool {
        matches!(self, Self::EndMill | Self::SurfacingBit | Self::Laser)
    }
}

/// The units a bit's feed and plunge rates are shown in. Rates are kept
/// in mm/min whatever the tool shows them in.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RateUnits {
    MmPerSec,
    #[default]
    MmPerMin,
    MPerMin,
    InchPerSec,
    InchPerMin,
    FeetPerMin,
}

impl RateUnits {
    pub const ALL: [Self; 6] = [
        Self::MmPerSec,
        Self::MmPerMin,
        Self::MPerMin,
        Self::InchPerSec,
        Self::InchPerMin,
        Self::FeetPerMin,
    ];

    /// One of these units in mm/min.
    pub fn mm_per_min(self) -> f64 {
        const INCH: f64 = 25.4;
        match self {
            Self::MmPerSec => 60.0,
            Self::MmPerMin => 1.0,
            Self::MPerMin => 1000.0,
            Self::InchPerSec => 60.0 * INCH,
            Self::InchPerMin => INCH,
            Self::FeetPerMin => 12.0 * INCH,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::MmPerSec => "mm/sec",
            Self::MmPerMin => "mm/min",
            Self::MPerMin => "m/min",
            Self::InchPerSec => "inches/sec",
            Self::InchPerMin => "inches/min",
            Self::FeetPerMin => "feet/min",
        }
    }
}

/// A tool's cutting data for one material: what is kept per material (and machine),
/// as against the tool's geometry. Rates in mm/min; stepovers a fraction of the diameter.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CuttingData {
    /// The material it is for; empty for the tool's default data
    pub material: String,
    pub pass_depth: f64,
    /// The clearing (roughing) stepover, as [`Tool::stepover`]
    pub stepover: f64,
    /// The finishing stepover, as [`Tool::finish_stepover`]; 0 for the clearing one
    /// (cutting data saved before there were two)
    #[serde(default, skip_serializing_if = "is_nothing")]
    pub finish_stepover: f64,
    pub feed_rate: f64,
    pub plunge_rate: f64,
    pub spindle_speed: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Tool {
    pub name: String,
    /// The Bit Library's name format its name is made from (`{Bit Type} {Diameter|F}`,
    /// see [`crate::tool_names`]); empty for a name typed as it is (added with name formats)
    #[serde(default, skip_serializing_if = "String::is_empty")]
    #[schemars(skip)]
    pub name_format: String,
    /// Who makes it (`Amana`), for the name format's `{Manufacturer}`; empty if not given
    /// (added with manufacturers)
    #[serde(default, skip_serializing_if = "String::is_empty")]
    #[schemars(skip)]
    pub manufacturer: String,
    #[serde(default)]
    pub shape: ToolShape,
    /// Radius of a radiused end mill's rounded corners
    #[serde(default = "default_tip_radius")]
    #[schemars(title = "Tip Radius", extend("x-unit" = "mm"), range(min = 0))]
    pub tip_radius: f64,
    /// Included angle of a V-bit's point
    #[serde(default = "default_angle")]
    #[schemars(extend("x-unit" = "°"), range(min = 1, max = 179))]
    pub angle: f64,
    #[schemars(extend("x-unit" = "mm"), range(min = 0))]
    pub diameter: f64,
    /// Depth cut on each pass
    #[schemars(extend("x-unit" = "mm"), range(min = 0))]
    pub pass_depth: f64,
    /// Sideways step between clearing (roughing) passes, as a fraction of the diameter:
    /// pockets, flat bottoms, inlay clearing and 3D roughing step over by this
    #[schemars(range(min = 0, max = 1))]
    pub stepover: f64,
    /// Sideways step between finishing passes, as a fraction of the diameter: 3D
    /// finishing, V-carved edges, chamfers, roundings and moulding profiles step over by
    /// this, smaller for a smoother surface. 0 for the clearing stepover (tools saved before
    /// there were two); read it with [`Tool::finishing_stepover`]
    #[serde(default, skip_serializing_if = "is_nothing")]
    #[schemars(skip)]
    pub finish_stepover: f64,
    #[schemars(extend("x-unit" = "mm/min"), range(min = 0))]
    pub feed_rate: f64,
    #[schemars(extend("x-unit" = "mm/min"), range(min = 0))]
    pub plunge_rate: f64,
    #[schemars(extend("x-unit" = "rpm"), range(min = 0))]
    pub spindle_speed: f64,
    /// Side angle of a tapered ball nose or tapered flat bottom, each side of the axis; 0
    /// for a straight tool. The taper widens to the shank, and toolpaths keep all of it off
    /// the model (added with tapers).
    #[serde(default)]
    #[schemars(title = "Taper Angle", extend("x-unit" = "°"), range(min = 0, max = 45))]
    pub taper: f64,
    /// Where a tapered tool's taper ends (added with tapers; 0 if not given)
    #[serde(default)]
    #[schemars(extend("x-unit" = "mm"), range(min = 0))]
    pub shank_diameter: f64,
    /// How deep the tool can cut; toolpaths going deeper are flagged (added with tapers;
    /// 0 if not given)
    #[serde(default)]
    #[schemars(extend("x-unit" = "mm"), range(min = 0))]
    pub flute_length: f64,
    /// The plain part between the flutes and the shank, which does not cut (a neck or
    /// shoulder); 0 for none or not given (added with drawn tool profiles)
    #[serde(default, skip_serializing_if = "is_nothing")]
    #[schemars(skip)]
    pub neck_length: f64,
    /// How long the shank is, the part held in the collet, from the neck (or flutes) to the
    /// top; 0 if not given (added with drawn tool profiles)
    #[serde(default, skip_serializing_if = "is_nothing")]
    #[schemars(skip)]
    pub shank_length: f64,
    /// How far the bit sticks out below the collet nut, from its tip up to the nut's face:
    /// the collet and spindle are this high above the tip, and the checks look for them
    /// hitting the material on deep cuts. 0 if not given, when it is taken to be a little
    /// more than the flutes ([`Tool::stickout`]; added with collision checks)
    #[serde(default, skip_serializing_if = "is_nothing")]
    #[schemars(skip)]
    pub stickout: f64,
    /// The holder (collet nut and spindle) this bit sits in, when it has its own: used only
    /// on a machine with Holder per bit ticked in Machine Configuration (a tool changer);
    /// `None` for the machine's (added with collision checks)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(skip)]
    pub holder: Option<crate::collision::Holder>,
    /// Cutting edges, for the chip load (added after the first tool format)
    #[serde(default = "default_flutes")]
    #[schemars(title = "Number of Flutes", range(min = 1, max = 12))]
    pub flutes: u32,
    /// A drill (a twist or brad-point bit, as a vendor's tool library lists them): it cuts
    /// only straight down, so a toolpath form other than Drilling says so in red when it is
    /// picked, and the checks before cutting warn of it. Ticked as Drill only beside the
    /// Bit Type in the Bit Library (added with drills from tool libraries)
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    #[schemars(skip)]
    pub drill_only: bool,
    /// Its bit number, for the post's `[T]` on a machine without a tool changer, and the
    /// holder Load by Bit Number puts it in; 0 for none (added with bit numbers)
    #[serde(default, skip_serializing_if = "is_zero")]
    #[schemars(skip)]
    pub number: u32,
    /// Pass depths set by hand in Edit Passes, each below where the cut starts, shallowest
    /// first; the cut depth is always the last. Empty for passes from the pass depth
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[schemars(skip)]
    pub passes: Vec<f64>,
    /// The cut depth the passes set by hand were set for (0 when not known, as in jobs made
    /// before it was kept): a deeper cut later goes on below it at the pass depth
    #[serde(default, skip_serializing_if = "is_nothing")]
    #[schemars(skip)]
    pub passes_for: f64,
    /// How thick the last pass is (Edit Passes' Last Pass Thickness), the passes above it
    /// even at the pass depth; 0 for all passes even
    #[serde(default, skip_serializing_if = "is_nothing")]
    #[schemars(skip)]
    pub last_pass: f64,
    /// The Bit Library group it sits in, `/` between levels (`Metric Bits/End Mills`);
    /// empty at the top (added with tool groups)
    #[serde(default, skip_serializing_if = "String::is_empty")]
    #[schemars(skip)]
    pub group: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    #[schemars(skip)]
    pub notes: String,
    /// What the feed and plunge rates are shown in (they are kept in mm/min; added with
    /// rate units)
    #[serde(default)]
    #[schemars(skip)]
    pub rate_units: RateUnits,
    /// What the tool's lengths are shown in, whatever the job's units (they are kept in mm;
    /// added with the unit switch)
    #[serde(default)]
    #[schemars(skip)]
    pub units: crate::document::Units,
    /// Cutting data for particular materials; the fields above are the default, used for
    /// any other (added with materials)
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[schemars(skip)]
    pub materials: Vec<CuttingData>,
    /// A thread mill's tooth: how far it stands out from the shank (S). Its angle is
    /// `angle` (A) and its tip to tip diameter `diameter` (D) (added with thread mills).
    #[serde(default = "default_tooth_size")]
    #[schemars(skip)]
    pub tooth_size: f64,
    /// From the bottom of a thread mill up to the tip of its tooth (O)
    #[serde(default = "default_tooth_offset")]
    #[schemars(skip)]
    pub tooth_offset: f64,
    /// A form tool's cutting edge: (distance from the axis, height above the tip) pairs,
    /// outward (added with form tools)
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[schemars(skip)]
    pub form: Vec<[f64; 2]>,
    /// Feeds and speeds for whole material classes, from the class's softest material to
    /// its hardest: a material in between cuts with data placed by its hardness
    /// ([`crate::material_library`]); a material's own data comes first (added with the
    /// Materials Library)
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[schemars(skip)]
    pub class_feeds: Vec<crate::material_library::ClassFeeds>,
}

/// A form tool's edge height at `r` out, between the profile's points (flat past its ends).
pub(super) fn form_height(form: &[[f64; 2]], r: f64) -> f64 {
    let i = form.partition_point(|p| p[0] < r);
    match (i.checked_sub(1).map(|k| form[k]), form.get(i).copied()) {
        (Some(a), Some(b)) if b[0] > a[0] => a[1] + (b[1] - a[1]) * (r - a[0]) / (b[0] - a[0]),
        (_, Some(p)) | (Some(p), None) => p[1],
        (None, None) => 0.0,
    }
}

pub(super) fn default_angle() -> f64 {
    90.0
}

pub(super) fn default_tooth_size() -> f64 {
    1.0
}

pub(super) fn default_tooth_offset() -> f64 {
    2.0
}

pub(super) fn default_flutes() -> u32 {
    2
}

#[expect(
    clippy::trivially_copy_pass_by_ref,
    reason = "serde's skip_serializing_if"
)]
pub(super) fn is_zero(n: &u32) -> bool {
    *n == 0
}

#[expect(
    clippy::trivially_copy_pass_by_ref,
    reason = "serde's skip_serializing_if"
)]
pub(super) fn is_nothing(n: &f64) -> bool {
    *n == 0.0
}

pub(super) fn default_tip_radius() -> f64 {
    1.0
}

impl Tool {
    /// Whether the bit must never go straight down into the material: a surfacing,
    /// spoilboard or fly cutter bit (the Surfacing Bit shape, or known by its name), or a
    /// form tool. Its entries ramp instead.
    pub fn cannot_plunge(&self) -> bool {
        let name = self.name.to_lowercase();
        matches!(self.shape, ToolShape::FormTool | ToolShape::SurfacingBit)
            || [
                "surfac",
                "spoilboard",
                "fly cut",
                "flycut",
                "planer",
                "planing",
            ]
            .iter()
            .any(|w| name.contains(w))
    }

    /// The stepover finishing passes use: its own, else (0, a tool saved with one
    /// stepover) the clearing stepover.
    pub fn finishing_stepover(&self) -> f64 {
        if self.finish_stepover > 0.0 {
            self.finish_stepover
        } else {
            self.stepover
        }
    }

    /// The flute length when one was given: `None` for 0 (not set, as in tools saved or
    /// imported without one), so nothing trusts a made-up number.
    pub fn known_flute_length(&self) -> Option<f64> {
        (self.flute_length.is_finite() && self.flute_length > 0.0).then_some(self.flute_length)
    }

    /// How deep the tool can cut, when that is known: a V-bit's V (half its width over the
    /// tangent of half its angle; V-bits have no flute length, and past the V the shank
    /// rubs), otherwise its [`Self::known_flute_length`].
    pub fn cutting_length(&self) -> Option<f64> {
        if self.shape == ToolShape::VBit {
            let half = (self.angle / 2.0).to_radians();
            return (self.diameter > 0.0 && half > 0.0 && half < std::f64::consts::FRAC_PI_2)
                .then(|| self.diameter / 2.0 / half.tan());
        }
        self.known_flute_length()
    }

    /// The My Bits group a tool copied in from a catalogue (or added) goes in: its maker,
    /// then its type, as `Amana/V-Bits` (`Generic/...` with no maker).
    pub fn own_group(&self) -> String {
        let maker = self.manufacturer.trim().replace('/', "-");
        let maker = if maker.is_empty() { "Generic" } else { &maker };
        let kind = match self.shape {
            ToolShape::EndMill => "End Mills",
            ToolShape::BallNose => "Ball Noses",
            ToolShape::VBit => "V-Bits",
            ToolShape::RadiusedEndMill => "Radiused End Mills",
            ToolShape::ThreadMill => "Thread Mills",
            ToolShape::FormTool => "Form Bits",
            ToolShape::SurfacingBit => "Surfacing Bits",
            ToolShape::Laser => "Lasers",
        };
        format!("{maker}/{kind}")
    }

    /// How much each flute cuts per turn (mm): the feed rate over the spindle speed and
    /// flutes, to compare with the cutter maker's figure. `None` with no spindle or flutes.
    pub fn chip_load(&self) -> Option<f64> {
        let teeth_per_minute = self.spindle_speed * f64::from(self.flutes);
        (teeth_per_minute > 0.0).then(|| self.feed_rate / teeth_per_minute)
    }

    /// The cutting data for `material` (`""` for the default): its own set if it has one,
    /// else the default's values under its name, ready to become its set.
    pub fn cutting(&self, material: &str) -> CuttingData {
        if let Some(set) = self.material_data(material) {
            // its class's data, as this material's
            return CuttingData {
                material: material.to_owned(),
                ..set.clone()
            };
        }
        CuttingData {
            material: material.to_owned(),
            pass_depth: self.pass_depth,
            stepover: self.stepover,
            finish_stepover: self.finish_stepover,
            feed_rate: self.feed_rate,
            plunge_rate: self.plunge_rate,
            spindle_speed: self.spindle_speed,
        }
    }

    /// Keeps `data` as its material's set, or as the default for no material.
    pub fn set_cutting(&mut self, data: CuttingData) {
        if data.material.is_empty() {
            self.pass_depth = data.pass_depth;
            self.stepover = data.stepover;
            self.finish_stepover = data.finish_stepover;
            self.feed_rate = data.feed_rate;
            self.plunge_rate = data.plunge_rate;
            self.spindle_speed = data.spindle_speed;
            return;
        }
        let at = self
            .materials
            .iter()
            .position(|m| m.material.eq_ignore_ascii_case(&data.material));
        match at {
            Some(i) => self.materials[i] = data,
            None => self.materials.push(data),
        }
    }

    /// Copy Settings From: the data this tool cuts `from` with (its default for none) kept
    /// as its data for `to` (its default for none).
    pub fn copy_cutting(&mut self, from: &str, to: &str) {
        let data = self.cutting(from);
        self.set_cutting(CuttingData {
            material: to.to_owned(),
            ..data
        });
    }

    /// A copy that cuts with `material`'s data where it has some: what a toolpath takes.
    /// A name made from a format is made again, for the material.
    #[must_use]
    pub fn for_material(&self, material: &str) -> Tool {
        let mut tool = self.clone();
        if let Some(set) = self.material_data(material) {
            tool.set_cutting(CuttingData {
                material: String::new(),
                ..set.clone()
            });
        }
        tool.renamed(material);
        tool
    }

    /// Its name made again from its name format, if it has one, for `material`.
    pub fn renamed(&mut self, material: &str) {
        if !self.name_format.is_empty() {
            self.name = crate::tool_names::expand(&self.name_format, self, material, "");
        }
    }

    /// Whether it keeps cutting data for `material` (always, for the default): the
    /// Bit Library greys the tools that do not.
    pub fn has_data_for(&self, material: &str) -> bool {
        // its own: data a material only borrows from its class is not set for it
        material.is_empty()
            || self
                .materials
                .iter()
                .any(|m| m.material.eq_ignore_ascii_case(material))
    }

    /// Its cutting data for `material`: the material's own, else its class's (data kept for
    /// the class by name, or for another material in it: a bit's Hardwood data cuts Maple).
    pub(super) fn material_data(&self, material: &str) -> Option<&CuttingData> {
        use crate::material_class::MaterialClass;
        if material.is_empty() {
            return None;
        }
        let own = self
            .materials
            .iter()
            .find(|m| m.material.eq_ignore_ascii_case(material));
        own.or_else(|| {
            let class = MaterialClass::of(material)?;
            let named = self
                .materials
                .iter()
                .find(|m| m.material.eq_ignore_ascii_case(class.name()));
            named.or_else(|| {
                self.materials
                    .iter()
                    .find(|m| MaterialClass::of(&m.material) == Some(class))
            })
        })
    }

    /// A thread mill's tooth height (H): its size and angle make a triangle, so this
    /// follows from them.
    pub fn tooth_height(&self) -> f64 {
        2.0 * self.tooth_size * (self.angle.to_radians() / 2.0).tan()
    }

    /// Sets the tooth height by changing its size, keeping the angle.
    pub fn set_tooth_height(&mut self, height: f64) {
        let slope = (self.angle.to_radians() / 2.0).tan();
        if slope > 0.0 {
            self.tooth_size = height / (2.0 * slope);
        }
    }

    /// Its passes (negative z) down to `depth`: those set by hand that are above it (none of
    /// them: one pass), going on at the pass depth below the depth they were set for, or even
    /// ones at the pass depth over all but a set last pass, then `depth` itself.
    pub fn pass_depths(&self, depth: f64) -> Vec<f64> {
        let mut set: Vec<f64> = self
            .passes
            .iter()
            .copied()
            .filter(|&d| d > 0.0 && d < depth - 1e-6)
            .collect();
        if !self.passes.is_empty() {
            set.sort_by(f64::total_cmp);
            set.dedup_by(|a, b| (*a - *b).abs() < 1e-6);
            // set for a shallower cut, they stop where that cut did: the rest goes on at the
            // pass depth, so making the cut deeper later never ends in one heavy pass
            let set_for = self.passes_for;
            if set_for > 0.0 && depth > set_for + 1e-6 && self.pass_depth > 0.0 {
                if set.last().is_none_or(|&d| d < set_for - 1e-6) {
                    set.push(set_for);
                }
                let below = set_for;
                let rest = pass_depths(depth - below, self.pass_depth);
                return set
                    .iter()
                    .map(|d| -d)
                    .chain(rest.into_iter().map(|z| z - below))
                    .collect();
            }
            return set.iter().map(|d| -d).chain([-depth]).collect();
        }
        if self.last_pass > 0.0 && self.last_pass < depth {
            let mut depths = pass_depths(depth - self.last_pass, self.pass_depth);
            depths.push(-depth);
            return depths;
        }
        pass_depths(depth, self.pass_depth)
    }

    /// The side angle (radians) of a tool that tapers out to its shank, if it does.
    pub fn tapered(&self) -> Option<f64> {
        let tapers = matches!(self.shape, ToolShape::BallNose | ToolShape::EndMill)
            && self.taper > 0.0
            && self.taper < 90.0
            && self.shank_diameter > self.diameter;
        tapers.then(|| self.taper.to_radians())
    }

    /// How far from its axis the tool reaches: its shank for a tapered tool.
    pub fn reach(&self) -> f64 {
        match self.tapered() {
            Some(_) => self.shank_diameter / 2.0,
            None => self.diameter / 2.0,
        }
    }

    /// How far out the tool can touch something up to `height` above its tip: anything
    /// further away is above the tool's profile.
    pub fn reach_below(&self, height: f64) -> f64 {
        let radius = self.diameter / 2.0;
        let r = match (self.tapered(), self.shape) {
            (Some(a), ToolShape::BallNose) => {
                radius * a.cos() + (height - radius * (1.0 - a.sin())).max(0.0) * a.tan()
            }
            (Some(a), _) => radius + height.max(0.0) * a.tan(),
            // a V-bit's cone widens to the full width only at the top of the V
            (None, ToolShape::VBit) if self.angle > 0.0 && self.angle < 180.0 => {
                (height.max(0.0) * (self.angle / 2.0).to_radians().tan()).min(radius)
            }
            (None, ToolShape::BallNose) if height < radius => {
                (height.max(0.0) * (2.0 * radius - height.max(0.0))).sqrt()
            }
            (None, ToolShape::RadiusedEndMill) => {
                let tip = self.tip_radius.clamp(0.0, radius);
                let h = height.clamp(0.0, tip);
                radius - tip + (h * (2.0 * tip - h)).sqrt()
            }
            (None, _) => radius,
        };
        r.min(self.reach())
    }

    /// Makes it a form tool cutting the shape of `path`: half its section, drawn upright
    /// with the axis on the left (the leftmost point) and the tip at the lowest point. Its
    /// diameter is twice the profile's width. False when the path has no width.
    pub fn set_form(&mut self, path: &crate::geometry::Path) -> bool {
        let pts = path.flatten(crate::accuracy::curve());
        let left = pts.iter().map(|p| p.x).fold(f64::INFINITY, f64::min);
        let low = pts.iter().map(|p| p.y).fold(f64::INFINITY, f64::min);
        let mut form: Vec<[f64; 2]> = pts.iter().map(|p| [p.x - left, p.y - low]).collect();
        form.sort_by(|a, b| a[0].total_cmp(&b[0]).then(a[1].total_cmp(&b[1])));
        // the edge is what the tool cuts: the lowest point at each distance out
        form.dedup_by(|b, a| (b[0] - a[0]).abs() < 1e-9);
        let width = form.last().map_or(0.0, |p| p[0]);
        if width <= 0.0 {
            return false;
        }
        self.shape = ToolShape::FormTool;
        self.diameter = 2.0 * width;
        self.form = form;
        true
    }

    /// How far above its tip the cutting edge is at `r` from the axis; `None` beyond the tool.
    /// A tapered tool's taper counts as edge, out to its shank.
    pub fn profile(&self, r: f64) -> Option<f64> {
        let radius = self.diameter / 2.0;
        if r > self.reach() {
            return None;
        }
        if let Some(a) = self.tapered() {
            // a ball meets its taper where the ball is as steep as the taper
            let (turn, rise) = match self.shape {
                ToolShape::BallNose => (radius * a.cos(), radius * (1.0 - a.sin())),
                _ => (radius, 0.0),
            };
            if r > turn {
                return Some(rise + (r - turn) / a.tan());
            }
        }
        Some(match self.shape {
            ToolShape::EndMill | ToolShape::SurfacingBit | ToolShape::Laser => 0.0,
            ToolShape::BallNose => radius - (radius * radius - r * r).max(0.0).sqrt(),
            ToolShape::VBit => r / (self.angle.to_radians() / 2.0).tan(),
            ToolShape::RadiusedEndMill => {
                let tip = self.tip_radius.clamp(0.0, radius);
                let past_flat = (r - (radius - tip)).max(0.0);
                tip - (tip * tip - past_flat * past_flat).max(0.0).sqrt()
            }
            // the core under the tooth is flat to the bottom; the tooth's lower flank
            // rises to its tip
            // ponytail: the core is taken to reach the bottom, as most single-point mills do
            ToolShape::FormTool => form_height(&self.form, r),
            ToolShape::ThreadMill => {
                let core = radius - self.tooth_size.clamp(0.0, radius);
                if r <= core {
                    0.0
                } else {
                    let slope = (self.angle.to_radians() / 2.0).tan();
                    self.tooth_offset - (radius - r) * slope
                }
            }
        })
    }
}

impl Tool {
    /// The bit a new toolpath's form starts with before one is picked: the default 6 mm
    /// end mill with the sizes a common two-flute one has (6 mm shank, 22 mm of flute), so
    /// the form can say how deep it reaches and the preview draws its own shank, rather
    /// than warning a beginner about sizes they never set. [`Tool::default`] keeps them
    /// unset, as a bit read from a file that does not give them.
    #[must_use]
    pub fn starter() -> Self {
        Self {
            shank_diameter: 6.0,
            flute_length: 22.0,
            ..Self::default()
        }
    }
}

impl Default for Tool {
    fn default() -> Self {
        Self {
            name: "6 mm End Mill".into(),
            name_format: String::new(),
            manufacturer: String::new(),
            shape: ToolShape::EndMill,
            tip_radius: default_tip_radius(),
            angle: default_angle(),
            diameter: 6.0,
            pass_depth: 3.0,
            stepover: 0.4,
            finish_stepover: 0.0,
            feed_rate: 1000.0,
            plunge_rate: 300.0,
            spindle_speed: 18000.0,
            taper: 0.0,
            shank_diameter: 0.0,
            flute_length: 0.0,
            neck_length: 0.0,
            shank_length: 0.0,
            stickout: 0.0,
            holder: None,
            flutes: default_flutes(),
            drill_only: false,
            number: 0,
            passes: vec![],
            passes_for: 0.0,
            last_pass: 0.0,
            group: String::new(),
            notes: String::new(),
            rate_units: RateUnits::default(),
            units: crate::document::Units::Mm,
            materials: vec![],
            tooth_size: default_tooth_size(),
            tooth_offset: default_tooth_offset(),
            form: vec![],
            class_feeds: vec![],
        }
    }
}

impl Tool {
    /// A 1 inch surfacing bit (spoilboard bit): what the Surfacing Toolpath starts with, and
    /// a starter tool.
    pub fn surfacing_bit() -> Self {
        Self {
            name: "1 in Surfacing Bit".into(),
            shape: ToolShape::SurfacingBit,
            diameter: 25.4,
            pass_depth: 1.0,
            stepover: 0.75,
            feed_rate: 3000.0,
            plunge_rate: 500.0,
            spindle_speed: 16000.0,
            flutes: 2,
            flute_length: 12.7,
            ..Self::default()
        }
    }

    /// A laser with a 0.15 mm kerf moving at 1500 mm/min: what the laser toolpaths start
    /// with, and a starter tool.
    pub fn laser() -> Self {
        Self {
            name: "Laser (0.15 mm kerf)".into(),
            shape: ToolShape::Laser,
            diameter: 0.15,
            pass_depth: 0.0,
            stepover: 0.0,
            feed_rate: 1500.0,
            plunge_rate: 1500.0,
            spindle_speed: 0.0,
            flutes: 1,
            ..Self::default()
        }
    }

    /// Whether it is a laser: the Laser bit type, or a laser toolpath's bit from before
    /// lasers were a bit type (an end mill named as a laser, with no spindle).
    pub fn is_laser(&self) -> bool {
        self.shape == ToolShape::Laser
            || (self.spindle_speed == 0.0 && self.name.to_lowercase().starts_with("laser"))
    }
}
