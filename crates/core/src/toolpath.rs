//! Toolpaths: the tool, the strategy command that made them (so they can be recalculated),
//! and the computed moves.

use crate::component::placed_model;
use crate::document::{Document, EntityId};
use crate::geometry::{Path, Point};
use crate::simulation::Material;
use crate::{Command, CommandError, polygon};
use glam::DVec3;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

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
}

/// The units a tool's feed and plunge rates are shown in, as Aspire's list. Rates are kept
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

/// A tool's cutting data for one material: what Aspire keeps per material (and machine),
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
    /// The Tool Database's name format its name is made from (`{Tool Type} {Diameter|F}`,
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
    /// Cutting edges, for the chip load (added after the first tool format)
    #[serde(default = "default_flutes")]
    #[schemars(title = "Number of Flutes", range(min = 1, max = 12))]
    pub flutes: u32,
    /// Its tool number, for the post's `[T]` on a machine without a tool changer, and the
    /// holder Load by Tool Number puts it in; 0 for none (added with tool numbers)
    #[serde(default, skip_serializing_if = "is_zero")]
    #[schemars(skip)]
    pub number: u32,
    /// Pass depths set by hand in Edit Passes, each below where the cut starts, shallowest
    /// first; the cut depth is always the last. Empty for passes from the pass depth
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[schemars(skip)]
    pub passes: Vec<f64>,
    /// How thick the last pass is (Edit Passes' Last Pass Thickness), the passes above it
    /// even at the pass depth; 0 for all passes even
    #[serde(default, skip_serializing_if = "is_nothing")]
    #[schemars(skip)]
    pub last_pass: f64,
    /// The Tool Database group it sits in, `/` between levels (`Metric Tools/End Mills`);
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
}

/// A form tool's edge height at `r` out, between the profile's points (flat past its ends).
fn form_height(form: &[[f64; 2]], r: f64) -> f64 {
    let i = form.partition_point(|p| p[0] < r);
    match (i.checked_sub(1).map(|k| form[k]), form.get(i).copied()) {
        (Some(a), Some(b)) if b[0] > a[0] => a[1] + (b[1] - a[1]) * (r - a[0]) / (b[0] - a[0]),
        (_, Some(p)) | (Some(p), None) => p[1],
        (None, None) => 0.0,
    }
}

fn default_angle() -> f64 {
    90.0
}

fn default_tooth_size() -> f64 {
    1.0
}

fn default_tooth_offset() -> f64 {
    2.0
}

fn default_flutes() -> u32 {
    2
}

#[expect(
    clippy::trivially_copy_pass_by_ref,
    reason = "serde's skip_serializing_if"
)]
fn is_zero(n: &u32) -> bool {
    *n == 0
}

#[expect(
    clippy::trivially_copy_pass_by_ref,
    reason = "serde's skip_serializing_if"
)]
fn is_nothing(n: &f64) -> bool {
    *n == 0.0
}

fn default_tip_radius() -> f64 {
    1.0
}

impl Tool {
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
            return set.clone();
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

    /// Whether it keeps cutting data for `material` (always, for the default): Aspire greys
    /// the tools that do not.
    pub fn has_data_for(&self, material: &str) -> bool {
        material.is_empty() || self.material_data(material).is_some()
    }

    /// `material`'s own set, matched whatever the case; none for the default.
    fn material_data(&self, material: &str) -> Option<&CuttingData> {
        (!material.is_empty())
            .then(|| {
                self.materials
                    .iter()
                    .find(|m| m.material.eq_ignore_ascii_case(material))
            })
            .flatten()
    }

    /// A thread mill's tooth height (H): its size and angle make a triangle, so this
    /// follows from them.
    pub fn tooth_height(&self) -> f64 {
        2.0 * self.tooth_size * (self.angle.to_radians() / 2.0).tan()
    }

    /// Sets the tooth height by changing its size, keeping the angle, as Aspire's form does.
    pub fn set_tooth_height(&mut self, height: f64) {
        let slope = (self.angle.to_radians() / 2.0).tan();
        if slope > 0.0 {
            self.tooth_size = height / (2.0 * slope);
        }
    }

    /// Its passes (negative z) down to `depth`: those set by hand that are above it (none of
    /// them: one pass), or even
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
            (None, _) => radius,
        };
        r.min(self.reach())
    }

    /// Makes it a form tool cutting the shape of `path`: half its section, drawn upright
    /// with the axis on the left (the leftmost point) and the tip at the lowest point. Its
    /// diameter is twice the profile's width. False when the path has no width.
    pub fn set_form(&mut self, path: &crate::geometry::Path) -> bool {
        let pts = path.flatten(crate::polygon::TOLERANCE);
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
            ToolShape::EndMill => 0.0,
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
            flutes: default_flutes(),
            number: 0,
            passes: vec![],
            last_pass: 0.0,
            group: String::new(),
            notes: String::new(),
            rate_units: RateUnits::default(),
            units: crate::document::Units::Mm,
            materials: vec![],
            tooth_size: default_tooth_size(),
            tooth_offset: default_tooth_offset(),
            form: vec![],
        }
    }
}

/// The shared tools. A toolpath copies its tool, so editing the database never changes
/// toolpaths already made.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct ToolDatabase {
    pub tools: Vec<Tool>,
    /// The groups the tools sit in, with their notes, in tree order; a group any tool names
    /// is in the tree whether listed here or not (added with tool groups)
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub groups: Vec<ToolGroup>,
    /// The materials tools can keep cutting data for, Aspire's Material list (added with
    /// materials)
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub materials: Vec<String>,
    /// The active material: tools picked from here cut with its data. Empty for each
    /// tool's default data.
    // ponytail: active for the whole database, as Aspire's; not kept per job
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub material: String,
    /// Each tool type's default name format, by type (`end_mill`), for tools made new
    /// (added with name formats)
    #[serde(default, skip_serializing_if = "std::collections::BTreeMap::is_empty")]
    pub name_formats: std::collections::BTreeMap<String, String>,
}

/// A Tool Database group: `/` between levels, as [`Tool::group`].
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct ToolGroup {
    pub path: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub notes: String,
}

impl ToolDatabase {
    /// What a first run starts with: common end mills, ball noses and V-bits.
    pub fn starter() -> Self {
        let end_mill =
            |name: &str, diameter: f64, pass_depth: f64, feed_rate: f64, plunge_rate: f64| Tool {
                name: name.into(),
                diameter,
                pass_depth,
                feed_rate,
                plunge_rate,
                ..Tool::default()
            };
        // a stepover to clear with and a smaller one to finish with, fractions of the
        // diameter: a ball nose roughs wide and finishes close for a smooth surface
        let shaped = |name: &str,
                      shape: ToolShape,
                      diameter: f64,
                      angle: f64,
                      (stepover, finish_stepover): (f64, f64)| Tool {
            shape,
            angle,
            stepover,
            finish_stepover,
            ..end_mill(name, diameter, 3.0, 1500.0, 500.0)
        };
        let mut tools = vec![
            end_mill("3 mm End Mill", 3.0, 1.5, 800.0, 250.0),
            Tool::default(),
            end_mill("12 mm End Mill", 12.0, 4.0, 2000.0, 500.0),
            end_mill("1/8 in End Mill", 3.175, 1.5, 800.0, 250.0),
            end_mill("1/4 in End Mill", 6.35, 3.0, 1000.0, 300.0),
            shaped("3 mm Ball Nose", ToolShape::BallNose, 3.0, 90.0, (0.4, 0.1)),
            shaped("6 mm Ball Nose", ToolShape::BallNose, 6.0, 90.0, (0.4, 0.1)),
            shaped(
                "6 mm Radiused End Mill",
                ToolShape::RadiusedEndMill,
                6.0,
                90.0,
                (0.4, 0.2),
            ),
            // a V-bit's stepover spaces V-carving passes, so it is small
            shaped("90° V-Bit", ToolShape::VBit, 12.7, 90.0, (0.02, 0.02)),
            shaped("60° V-Bit", ToolShape::VBit, 12.7, 60.0, (0.02, 0.02)),
            // 60° for ISO threads; a 1 mm tooth suits pitches up to about 1.5 mm
            Tool {
                tooth_size: 1.0,
                tooth_offset: 2.0,
                ..shaped(
                    "10 mm Thread Mill",
                    ToolShape::ThreadMill,
                    10.0,
                    60.0,
                    (0.1, 0.1),
                )
            },
        ];
        for tool in &mut tools {
            let set = if tool.name.contains(" in ") {
                "Imperial Tools"
            } else {
                "Metric Tools"
            };
            tool.group = format!("{set}/{}", shape_group(tool.shape));
        }
        let group = |path: &str, notes: &str| ToolGroup {
            path: path.into(),
            notes: notes.into(),
        };
        Self {
            tools,
            groups: vec![
                group("Metric Tools", "Starter tools sized in millimetres."),
                group("Imperial Tools", "Starter tools sized in inches."),
            ],
            ..Self::default()
        }
    }

    pub fn get(&self, name: &str) -> Option<&Tool> {
        self.tools.iter().find(|t| t.name == name)
    }

    /// The tool `name` as a toolpath takes it: cutting with the active material's data.
    /// Gives each tool loaded in a tool changer its slot (counted from 1) as its tool
    /// number, so the number shown is the holder it sits in. A tool not loaded keeps its
    /// number unless a loaded tool now has it (it was displaced, or taken out and the slot
    /// refilled): then it has none (0), so Load by Tool Number is never ambiguous.
    /// Whether any number changed.
    pub fn number_by_slots(&mut self, slots: &[Option<String>]) -> bool {
        let slot_of = |name: &str| {
            slots
                .iter()
                .position(|s| s.as_deref() == Some(name))
                .and_then(|i| u32::try_from(i + 1).ok())
        };
        let taken: Vec<u32> = self.tools.iter().filter_map(|t| slot_of(&t.name)).collect();
        let mut changed = false;
        for tool in &mut self.tools {
            let number = match slot_of(&tool.name) {
                Some(slot) => slot,
                None if taken.contains(&tool.number) => 0,
                None => tool.number,
            };
            changed |= tool.number != number;
            tool.number = number;
        }
        changed
    }

    pub fn pick(&self, name: &str) -> Option<Tool> {
        self.get(name).map(|t| t.for_material(&self.material))
    }

    /// Puts back a tool as [`Self::pick`] gave it: with a material active its cutting data
    /// become that material's, and the stored tool's default stays.
    pub fn put_picked(&mut self, mut tool: Tool) {
        if !self.material.is_empty() {
            let data = CuttingData {
                material: self.material.clone(),
                ..tool.cutting("")
            };
            if let Some(stored) = self.get(&tool.name) {
                tool.set_cutting(stored.cutting(""));
            }
            tool.set_cutting(data);
        }
        self.put(tool);
    }

    /// Adds `name` to the material list, unless it is there already (whatever the case).
    pub fn add_material(&mut self, name: &str) {
        let name = name.trim();
        if !name.is_empty() && !self.materials.iter().any(|m| m.eq_ignore_ascii_case(name)) {
            self.materials.push(name.to_owned());
        }
    }

    /// Aspire's Duplicate Material: `from` copied as `name` (its own name and " copy" when
    /// none is given; a number added while the name is taken), with every tool's cutting
    /// data for it when `with_data`. The copy becomes the active material; its name.
    pub fn duplicate_material(&mut self, from: &str, name: &str, with_data: bool) -> String {
        let base = if name.trim().is_empty() {
            format!("{from} copy")
        } else {
            name.trim().to_owned()
        };
        let mut copy = base.clone();
        let mut next = 2;
        while self.materials.iter().any(|m| m.eq_ignore_ascii_case(&copy)) {
            copy = format!("{base} {next}");
            next += 1;
        }
        self.materials.push(copy.clone());
        if with_data {
            for tool in &mut self.tools {
                if tool.material_data(from).is_some() {
                    tool.copy_cutting(from, &copy);
                }
            }
        }
        self.material.clone_from(&copy);
        copy
    }

    /// Takes `name` off the material list with every tool's data for it, as Aspire's Delete
    /// Material does; the default is active again if it was.
    pub fn delete_material(&mut self, name: &str) {
        self.materials.retain(|m| !m.eq_ignore_ascii_case(name));
        for tool in &mut self.tools {
            tool.materials
                .retain(|m| !m.material.eq_ignore_ascii_case(name));
        }
        if self.material.eq_ignore_ascii_case(name) {
            self.material.clear();
        }
    }

    /// Replaces the tool with the same name, or adds it at the end.
    pub fn put(&mut self, tool: Tool) {
        match self.tools.iter_mut().find(|t| t.name == tool.name) {
            Some(existing) => *existing = tool,
            None => self.tools.push(tool),
        }
    }

    pub fn to_json(&self) -> Vec<u8> {
        serde_json::to_vec_pretty(self).expect("tools always serialize")
    }

    pub fn from_json(bytes: &[u8]) -> Result<Self, serde_json::Error> {
        serde_json::from_slice(bytes)
    }
}

/// The group a starter tool of `shape` goes in.
pub fn shape_group(shape: ToolShape) -> &'static str {
    match shape {
        ToolShape::EndMill => "End Mills",
        ToolShape::BallNose => "Ball Nose",
        ToolShape::VBit => "V-Bits",
        ToolShape::RadiusedEndMill => "Radiused End Mills",
        ToolShape::ThreadMill => "Thread Mills",
        ToolShape::FormTool => "Form Tools",
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MoveKind {
    /// Full speed, clear of the material
    Rapid,
    /// Straight down into the material at the plunge rate
    Plunge,
    /// Cutting at the feed rate
    Cut,
}

/// A straight move to `to` (x, y in job millimetres; z relative to the material surface).
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Move {
    pub kind: MoveKind,
    pub to: DVec3,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Toolpath {
    pub name: String,
    /// Id of the command that calculates it, re-run with `params` to recalculate.
    pub strategy: String,
    pub params: serde_json::Value,
    pub tool: Tool,
    /// The vectors it was made from: recalculating re-runs the strategy on these.
    #[serde(default)]
    pub vectors: Vec<EntityId>,
    #[serde(with = "crate::packed::moves")]
    pub moves: Vec<Move>,
    /// Hidden toolpaths are not drawn or previewed (added after the first job format).
    #[serde(default = "shown")]
    pub visible: bool,
    /// A laser toolpath's power (percent of the laser's maximum) for each move; empty for
    /// a router or mill (added with lasers).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub power: Vec<f64>,
    /// What it was made from has changed since (its vectors, or the model it cuts):
    /// recalculating makes it fresh again.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub stale: bool,
    /// The toolpath group it is listed under, if any (added with toolpath groups).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub group: Option<String>,
    /// A drill bank toolpath's bank and the plunges of its head; its moves drill each hole
    /// one after another (for the preview), and the post writes one plunge for each hit
    /// (added with drill banks).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub drill_bank: Option<crate::drill_bank::BankToolpath>,
}

/// The strategies that always cut the 3D model; others do when projected onto it.
const MODEL_STRATEGIES: [&str; 3] = [
    "3d_roughing_toolpath",
    "3d_finishing_toolpath",
    "sketch_carving_toolpath",
];

/// The key a toolpath's own preview colour is kept under in its params: `[r, g, b]`, or
/// null for no fill (the material's colour).
pub const PREVIEW_COLOUR: &str = "preview_colour";

/// The key a toolpath's notes for the Toolpath Summary are kept under in its params.
pub const SUMMARY_NOTES: &str = "summary_notes";

/// What a toolpath keeps in its params that is its own, not one of its settings: kept when
/// it is recalculated.
pub const OWN_PARAMS: [&str; 2] = [PREVIEW_COLOUR, SUMMARY_NOTES];

/// The colours toolpaths take in a Toolpath Color preview until given their own, in list
/// order.
const PREVIEW_PALETTE: [[u8; 3]; 8] = [
    [220, 60, 60],
    [60, 140, 220],
    [70, 170, 80],
    [230, 170, 40],
    [150, 80, 200],
    [40, 180, 180],
    [230, 110, 170],
    [130, 130, 130],
];

fn shown() -> bool {
    true
}

impl Toolpath {
    /// Its notes for the Toolpath Summary.
    pub fn summary_notes(&self) -> &str {
        self.params
            .get(SUMMARY_NOTES)
            .and_then(serde_json::Value::as_str)
            .unwrap_or_default()
    }

    /// Its colour in a Toolpath Color preview, `place` being where it is in the list: the
    /// one it was given, none if it was given no fill, else the palette's for its place.
    pub fn preview_colour(&self, place: usize) -> Option<[u8; 3]> {
        match self.params.get(PREVIEW_COLOUR) {
            Some(given) => serde_json::from_value(given.clone()).ok(),
            None => Some(PREVIEW_PALETTE[place % PREVIEW_PALETTE.len()]),
        }
    }

    /// Whether the 3D model is what it cuts, so changing the model changes it.
    pub fn reads_model(&self) -> bool {
        MODEL_STRATEGIES.contains(&self.strategy.as_str())
            || self.params["project_onto_model"].as_bool() == Some(true)
    }

    /// Estimated machining time in minutes: each move's length at its rate (rapids at
    /// `rapid_rate`, mm/min), from the first move on.
    // ponytail: no acceleration; Toolpath Summary's scale factor stands in for it, as Aspire's does
    pub fn machining_minutes(&self, rapid_rate: f64) -> f64 {
        self.moves
            .windows(2)
            .map(|pair| {
                let rate = match pair[1].kind {
                    MoveKind::Rapid => rapid_rate,
                    MoveKind::Plunge => self.tool.plunge_rate,
                    MoveKind::Cut => self.tool.feed_rate,
                };
                let length = pair[0].to.distance(pair[1].to);
                if rate > 0.0 { length / rate } else { 0.0 }
            })
            .sum()
    }

    /// [`Self::machining_minutes`] allowing for the machine's `acceleration` (X, Y, Z,
    /// mm/s²; 0 for an axis places no limit, all 0 is no allowance): the machine stops at
    /// every corner where the direction changes, and speeds up and slows down along each
    /// move as fast as the slowest axis it uses allows.
    // ponytail: corners within 5° run through at the slower of the two feeds [inf]
    pub fn machining_minutes_accelerating(&self, rapid_rate: f64, acceleration: [f64; 3]) -> f64 {
        const STRAIGHT: f64 = 0.996; // cos 5°
        if acceleration.iter().all(|a| *a <= 0.0) {
            return self.machining_minutes(rapid_rate);
        }
        // each move's direction, length and speed (mm/s)
        let legs: Vec<Option<(DVec3, f64, f64)>> = (1..self.moves.len())
            .map(|i| {
                let d = self.moves[i].to - self.moves[i - 1].to;
                let rate = match self.moves[i].kind {
                    MoveKind::Rapid => rapid_rate,
                    MoveKind::Plunge => self.tool.plunge_rate,
                    MoveKind::Cut => self.tool.feed_rate,
                } / 60.0;
                let length = d.length();
                (length > 0.0 && rate > 0.0).then(|| (d / length, length, rate))
            })
            .collect();
        // the speed through the joint between two legs: on through a near-straight one
        let joint = |a: Option<(DVec3, f64, f64)>, b: Option<(DVec3, f64, f64)>| match (a, b) {
            (Some((da, _, va)), Some((db, _, vb))) if da.dot(db) > STRAIGHT => va.min(vb),
            _ => 0.0,
        };
        let mut seconds = 0.0;
        for (i, leg) in legs.iter().enumerate() {
            let Some((dir, length, v)) = *leg else {
                continue;
            };
            let limit = (0..3)
                .filter(|&k| dir[k].abs() > 1e-9 && acceleration[k] > 0.0)
                .map(|k| acceleration[k] / dir[k].abs())
                .fold(f64::INFINITY, f64::min);
            if !limit.is_finite() {
                seconds += length / v;
                continue;
            }
            let (vs, ve) = (
                joint(
                    i.checked_sub(1).and_then(|j| legs[j]),
                    Some((dir, length, v)),
                ),
                joint(Some((dir, length, v)), legs.get(i + 1).copied().flatten()),
            );
            let (up, down) = (
                (v * v - vs * vs) / (2.0 * limit),
                (v * v - ve * ve) / (2.0 * limit),
            );
            seconds += if up + down <= length {
                (v - vs) / limit + (v - ve) / limit + (length - up - down) / v
            } else {
                // too short to reach the feed: up to a peak and straight down again
                let peak = ((2.0 * limit * length + vs * vs + ve * ve) / 2.0).sqrt();
                (peak - vs) / limit + (peak - ve) / limit
            };
        }
        seconds / 60.0
    }
}

#[cfg(test)]
#[test]
fn machining_time_allowing_for_acceleration_stops_at_corners_and_runs_through_straights() {
    let at = |kind, x, y| Move {
        kind,
        to: DVec3::new(x, y, -1.0),
    };
    let tool = Tool {
        feed_rate: 1000.0,
        plunge_rate: 100.0,
        ..Tool::default()
    };
    let make = |moves| Toolpath {
        name: "t".into(),
        strategy: String::new(),
        params: serde_json::Value::Null,
        tool: tool.clone(),
        vectors: vec![],
        moves,
        visible: true,
        power: vec![],
        stale: false,
        group: None,
        drill_bank: None,
    };
    // one 500 mm cut at 1000 mm/min (16.67 mm/s): 30 s flat out; with 10 mm/s² the machine
    // spends 1.67 s speeding up and the same slowing down, 13.9 mm each
    let one = make(vec![
        at(MoveKind::Cut, 0.0, 0.0),
        at(MoveKind::Cut, 500.0, 0.0),
    ]);
    let v = 1000.0 / 60.0;
    let expected = ((500.0 - 2.0 * v * v / 20.0) / v + 2.0 * v / 10.0) / 60.0;
    assert!(
        (one.machining_minutes_accelerating(5000.0, [10.0, 10.0, 10.0]) - expected).abs() < 1e-9
    );
    assert!(
        (one.machining_minutes_accelerating(5000.0, [0.0; 3]) - 0.5).abs() < 1e-12,
        "no allowance"
    );
    // a corner stops the machine; a straight joint does not
    let bent = make(vec![
        at(MoveKind::Cut, 0.0, 0.0),
        at(MoveKind::Cut, 250.0, 0.0),
        at(MoveKind::Cut, 250.0, 250.0),
    ]);
    let straight = make(vec![
        at(MoveKind::Cut, 0.0, 0.0),
        at(MoveKind::Cut, 250.0, 0.0),
        at(MoveKind::Cut, 500.0, 0.0),
    ]);
    let a = [10.0, 10.0, 10.0];
    assert!(
        bent.machining_minutes_accelerating(5000.0, a)
            > straight.machining_minutes_accelerating(5000.0, a)
    );
    assert!(
        (straight.machining_minutes_accelerating(5000.0, a) - expected).abs() < 1e-9,
        "as one leg"
    );
    // too short to reach the feed: a 1 mm move takes a peak-and-down 0.63 s, not 0.06 s
    let short = make(vec![
        at(MoveKind::Cut, 0.0, 0.0),
        at(MoveKind::Cut, 1.0, 0.0),
    ]);
    let minutes = short.machining_minutes_accelerating(5000.0, a);
    assert!(
        (minutes * 60.0 - 2.0 * (1.0_f64 / 10.0).sqrt()).abs() < 1e-9,
        "{minutes}"
    );
}

#[cfg(test)]
#[test]
fn machining_time_is_each_move_at_its_rate() {
    let at = |kind, x, z| Move {
        kind,
        to: DVec3::new(x, 0.0, z),
    };
    let tp = Toolpath {
        name: "t".into(),
        strategy: String::new(),
        params: serde_json::Value::Null,
        tool: Tool {
            feed_rate: 1000.0,
            plunge_rate: 100.0,
            ..Tool::default()
        },
        vectors: vec![],
        moves: vec![
            at(MoveKind::Rapid, 0.0, 5.0),
            at(MoveKind::Plunge, 0.0, -5.0),
            at(MoveKind::Cut, 500.0, -5.0),
            at(MoveKind::Rapid, 500.0, 5.0),
        ],
        visible: true,
        power: vec![],
        stale: false,
        group: None,
        drill_bank: None,
    };
    // 10 mm plunge at 100, 500 mm cut at 1000, 10 mm rapid at 5000
    assert!((tp.machining_minutes(5000.0) - (0.1 + 0.5 + 0.002)).abs() < 1e-12);
}

/// Depth of each pass (negative z) down to `depth`, in equal steps no deeper than `pass_depth`.
pub fn pass_depths(depth: f64, pass_depth: f64) -> Vec<f64> {
    // ponytail: equal steps; a lighter final "finish" pass is a later option
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "a small positive count"
    )]
    let passes = (depth / pass_depth).ceil().max(1.0) as u32;
    let step = depth / f64::from(passes);
    (1..=passes)
        .map(|i| {
            if i == passes {
                -depth
            } else {
                -step * f64::from(i)
            }
        })
        .collect()
}

/// Cuts along `points` once per depth: rapid above the start, plunge, cut, and finally
/// retract. A closed contour plunges straight to its next pass; an open one retracts and
/// returns to its start first.
pub fn contour_moves(points: &[Point], closed: bool, depths: &[f64], safe_z: f64) -> Vec<Move> {
    let Some((&start, rest)) = points.split_first() else {
        return vec![];
    };
    let at = |p: Point, z: f64| DVec3::new(p.x, p.y, z);
    let end = if closed {
        start
    } else {
        *points.last().unwrap_or(&start)
    };
    let mut moves = vec![Move {
        kind: MoveKind::Rapid,
        to: at(start, safe_z),
    }];
    for (pass, &z) in depths.iter().enumerate() {
        if pass > 0 && !closed {
            moves.push(Move {
                kind: MoveKind::Rapid,
                to: at(end, safe_z),
            });
            moves.push(Move {
                kind: MoveKind::Rapid,
                to: at(start, safe_z),
            });
        }
        moves.push(Move {
            kind: MoveKind::Plunge,
            to: at(start, z),
        });
        moves.extend(rest.iter().map(|&p| Move {
            kind: MoveKind::Cut,
            to: at(p, z),
        }));
        if closed {
            moves.push(Move {
                kind: MoveKind::Cut,
                to: at(start, z),
            });
        }
    }
    moves.push(Move {
        kind: MoveKind::Rapid,
        to: at(end, safe_z),
    });
    moves
}

/// A toolpath stamped with the command that made it and its params, so it can be recalculated.
pub fn made_by<C: Command>(
    params: &C::Params,
    name: String,
    tool: Tool,
    vectors: Vec<EntityId>,
    moves: Vec<Move>,
) -> Toolpath {
    let params = serde_json::to_value(params).expect("command params must serialize");
    Toolpath {
        name,
        strategy: C::ID.into(),
        params,
        tool,
        vectors,
        moves,
        visible: true,
        power: vec![],
        stale: false,
        group: None,
        drill_bank: None,
    }
}

/// The points to cut along: the path flattened to the polygon tolerance, without repeating
/// the start at the end of a closed path.
pub fn cut_points(path: &Path) -> Vec<Point> {
    let mut points = path.flatten(polygon::TOLERANCE);
    if path.closed && points.len() > 1 && points.first() == points.last() {
        points.pop();
    }
    points
}

/// Most times a move between two zigzag samples is halved to stay within the tolerance.
const HALVINGS: u32 = 6;

/// A zigzag of `tool` over `grid`: lines along x `step` apart, sampled at each cell centre,
/// each point as low as the tool goes there without touching `grid`, and never below `floor`.
/// Every move is checked: one that would cut more than `tolerance` into `grid` is halved, and
/// one too short to halve is lifted over. Points in the middle of level runs are dropped.
pub fn zigzag(grid: &Material, tool: &Tool, step: f64, tolerance: f64, floor: f64) -> Vec<DVec3> {
    zigzag_within(grid, tool, step, tolerance, floor, &|_| true).concat()
}

/// [`zigzag`] over only the points `inside` says: each line split into runs where it leaves
/// them, in cutting order (every other line reversed).
#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_precision_loss,
    reason = "line counts are small and non-negative"
)]
pub fn zigzag_within(
    grid: &Material,
    tool: &Tool,
    step: f64,
    tolerance: f64,
    floor: f64,
    inside: &(dyn Fn(Point) -> bool + Sync),
) -> Vec<Vec<DVec3>> {
    let (first, last) = (
        grid.centre(0, 0),
        grid.centre(grid.columns - 1, grid.rows - 1),
    );
    let lines = ((last.y - first.y) / step).floor() as usize + 1;
    let drop = Drop {
        grid,
        tool,
        tolerance,
        floor,
        ceiling: f64::INFINITY,
        over_ceiling: false,
    };
    let lines: Vec<Vec<Point>> = (0..lines)
        .map(|line| {
            let y = first.y + line as f64 * step;
            let mut ends = vec![Point::new(first.x, y), Point::new(last.x, y)];
            if line % 2 == 1 {
                ends.reverse();
            }
            ends
        })
        .collect();
    drop.along_all(&lines, inside)
}

/// How to drop a tool along lines over a grid: never below `floor`, every move checked to
/// cut no more than `tolerance` into the grid, and runs split where the tool would sit above
/// `ceiling` (a Z level's flat cut stops where the model stands above it).
#[derive(Clone, Copy)]
pub struct Drop<'a> {
    pub grid: &'a Material,
    pub tool: &'a Tool,
    pub tolerance: f64,
    pub floor: f64,
    pub ceiling: f64,
    /// Runs split at the ceiling keep the point just over it at each end, so the move up to
    /// it is still cut.
    pub over_ceiling: bool,
}

impl Drop<'_> {
    /// The tool along the polyline `line`, sampled about a cell apart, as low as it goes at
    /// each point; split into runs where `inside` says no or it would be above the ceiling.
    /// Points in the middle of level runs are left out.
    pub fn along(
        &self,
        line: &[Point],
        inside: &(dyn Fn(Point) -> bool + Sync),
    ) -> Vec<Vec<DVec3>> {
        self.along_all(&[line.to_vec()], inside)
    }

    /// [`Self::along`] each of `lines`, in order, the heavy work in batches over every line
    /// at once (on the GPU when there is one): the drops at the samples, then each round of
    /// refinement's gouge checks and midpoints.
    pub fn along_all(
        &self,
        lines: &[Vec<Point>],
        inside: &(dyn Fn(Point) -> bool + Sync),
    ) -> Vec<Vec<DVec3>> {
        let samples: Vec<Vec<(Point, bool)>> = lines
            .iter()
            .map(|line| {
                self.samples(line)
                    .into_iter()
                    .map(|p| (p, inside(p)))
                    .collect()
            })
            .collect();
        let wanted: Vec<Point> = samples
            .iter()
            .flatten()
            .filter_map(|&(p, inside)| inside.then_some(p))
            .collect();
        let dropped = crate::compute::compute().drop_at(self.grid, self.tool, &wanted);
        let mut dropped = dropped.into_iter();
        let mut gaps: Vec<(DVec3, DVec3)> = vec![];
        let mut gap = |a, b, run: &mut Vec<Piece>| {
            run.push(Piece::Gap(gaps.len()));
            gaps.push((a, b));
        };
        let mut runs: Vec<Vec<Piece>> = vec![];
        for line in &samples {
            let mut run: Vec<Piece> = vec![];
            // the last point passed over for being above the ceiling, while still inside
            let mut over: Option<DVec3> = None;
            for &(p, inside) in line {
                let point = inside.then(|| {
                    let z = dropped.next().expect("a drop for each sample inside");
                    p.extend(z.max(self.floor))
                });
                let next = point.filter(|q| q.z <= self.ceiling + 1e-9);
                let Some(next) = next else {
                    if let (true, Some(up), Some(&Piece::Point(previous))) =
                        (self.over_ceiling, point, run.last())
                    {
                        gap(previous, up, &mut run);
                        run.push(Piece::Point(up));
                    }
                    if !run.is_empty() {
                        runs.push(std::mem::take(&mut run));
                    }
                    over = point;
                    continue;
                };
                if let (true, Some(up), true) = (self.over_ceiling, over, run.is_empty()) {
                    run.push(Piece::Point(up));
                }
                over = None;
                if let Some(&Piece::Point(previous)) = run.last() {
                    gap(previous, next, &mut run);
                }
                run.push(Piece::Point(next));
            }
            if !run.is_empty() {
                runs.push(run);
            }
        }
        let fills = self.refine_all(&gaps, HALVINGS);
        runs.into_iter()
            .map(|run| {
                tidy(
                    run.into_iter()
                        .flat_map(|piece| match piece {
                            Piece::Point(p) => vec![p],
                            Piece::Gap(i) => fills[i].clone(),
                        })
                        .collect(),
                )
            })
            .collect()
    }

    /// The points along the polyline `line`, about a cell apart.
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "a small positive count"
    )]
    fn samples(&self, line: &[Point]) -> Vec<Point> {
        let mut samples = line.first().copied().into_iter().collect::<Vec<_>>();
        for w in line.windows(2) {
            let steps = (w[0].distance(w[1]) / self.grid.cell - 1e-9)
                .ceil()
                .max(1.0) as u32;
            samples.extend((1..=steps).map(|i| w[0].lerp(w[1], f64::from(i) / f64::from(steps))));
        }
        samples
    }

    /// The points to put in each of `gaps` (a move from one dropped point to the next) so
    /// the moves through them stay out of the grid: none where the move already does, else
    /// the tool dropped at the middle and the two halves refined again, `halvings` deep,
    /// then up, across level and down.
    fn refine_all(&self, gaps: &[(DVec3, DVec3)], halvings: u32) -> Vec<Vec<DVec3>> {
        let mut fills = vec![vec![]; gaps.len()];
        if gaps.is_empty() {
            return fills;
        }
        let compute = crate::compute::compute();
        let over = compute.overcut_along(self.grid, self.tool, gaps);
        let gouging: Vec<usize> = (0..gaps.len())
            .filter(|&i| over[i] > self.tolerance)
            .collect();
        if gouging.is_empty() {
            return fills;
        }
        let across = |i: usize, z: f64| {
            (
                gaps[i].0.truncate().extend(z),
                gaps[i].1.truncate().extend(z),
            )
        };
        if halvings == 0 {
            // too short to halve again: up, across level, and down
            let level = |i: usize| gaps[i].0.z.max(gaps[i].1.z);
            let flats: Vec<(DVec3, DVec3)> = gouging.iter().map(|&i| across(i, level(i))).collect();
            let lifted = compute.overcut_along(self.grid, self.tool, &flats);
            for (&i, lift) in gouging.iter().zip(lifted) {
                let (from, to) = across(i, level(i) + lift.max(0.0));
                fills[i] = vec![from, to];
            }
            return fills;
        }
        let middles: Vec<Point> = gouging
            .iter()
            .map(|&i| (gaps[i].0.truncate() + gaps[i].1.truncate()) / 2.0)
            .collect();
        let middles: Vec<DVec3> = middles
            .iter()
            .zip(compute.drop_at(self.grid, self.tool, &middles))
            .map(|(p, z)| p.extend(z.max(self.floor)))
            .collect();
        let halves: Vec<(DVec3, DVec3)> = gouging
            .iter()
            .zip(&middles)
            .flat_map(|(&i, &middle)| [(gaps[i].0, middle), (middle, gaps[i].1)])
            .collect();
        let mut parts = self.refine_all(&halves, halvings - 1).into_iter();
        for (&i, &middle) in gouging.iter().zip(&middles) {
            let (first, second) = (
                parts.next().expect("a fill per half"),
                parts.next().expect("a fill per half"),
            );
            fills[i].extend(first);
            fills[i].push(middle);
            fills[i].extend(second);
        }
        fills
    }
}

/// A run of a line being dropped: a point, or a gap between two still to be refined.
enum Piece {
    Point(DVec3),
    Gap(usize),
}

/// Leaves out the middle points of straight level runs.
fn tidy(run: Vec<DVec3>) -> Vec<DVec3> {
    let mut points: Vec<DVec3> = vec![];
    for point in run {
        let n = points.len();
        if n >= 2 && level_run(points[n - 2], points[n - 1], point) {
            points.pop();
        }
        points.push(point);
    }
    points
}

/// Three points along one straight line at one height.
fn level_run(a: DVec3, b: DVec3, c: DVec3) -> bool {
    let same = |u: f64, v: f64| (u - v).abs() < 1e-9;
    let (first, second) = (b.truncate() - a.truncate(), c.truncate() - b.truncate());
    same(first.perp_dot(second), 0.0) && first.dot(second) > 0.0 && same(a.z, b.z) && same(b.z, c.z)
}

/// Raster lines `step` apart at `angle` (degrees from X) across `regions`, every other one
/// back the other way, as lines to drop a tool along.
pub fn raster_lines(regions: &[Path], step: f64, angle: f64) -> Vec<Vec<Point>> {
    hatch_lines(regions, step, angle)
        .into_iter()
        .enumerate()
        .flat_map(|(row, spans)| {
            let mut spans: Vec<Vec<Point>> = spans.into_iter().map(|(a, b)| vec![a, b]).collect();
            if row % 2 == 1 {
                spans.reverse();
                for span in &mut spans {
                    span.reverse();
                }
            }
            spans
        })
        .collect()
}

/// [`follow`] for each run in turn, but staying down from the end of one run to the start
/// of the next when they are no more than `reach` apart: lifted by `lift` over the gap (a
/// stepover retract), when `clear` says each move of that keeps out of the work.
pub fn follow_linked(
    runs: &[Vec<DVec3>],
    safe_z: f64,
    reach: f64,
    lift: f64,
    clear: &dyn Fn(DVec3, DVec3) -> bool,
) -> Vec<Move> {
    let mut moves = vec![];
    let mut last: Option<DVec3> = None;
    for run in runs.iter().filter(|r| !r.is_empty()) {
        let first = run[0];
        let link = last
            .filter(|end| end.truncate().distance(first.truncate()) <= reach)
            .and_then(|end| {
                let (up, over) = (end + DVec3::Z * lift, first + DVec3::Z * lift);
                (clear(end, up) && clear(up, over) && clear(over, first)).then_some([up, over])
            });
        if let Some(link) = link {
            // the run before ended with its rapid up: stay down instead
            moves.pop();
            if lift > 0.0 {
                moves.extend(link.map(|to| Move {
                    kind: MoveKind::Cut,
                    to,
                }));
            }
            moves.push(Move {
                kind: MoveKind::Cut,
                to: first,
            });
            moves.extend(run[1..].iter().map(|&to| Move {
                kind: MoveKind::Cut,
                to,
            }));
            moves.push(Move {
                kind: MoveKind::Rapid,
                to: run[run.len() - 1].truncate().extend(safe_z),
            });
        } else {
            moves.extend(follow(run, safe_z));
        }
        last = run.last().copied();
    }
    moves
}

/// Rapid to above the first point, plunge to it, cut through the rest, rapid back up.
pub fn follow(points: &[DVec3], safe_z: f64) -> Vec<Move> {
    let (Some(&first), Some(&last)) = (points.first(), points.last()) else {
        return vec![];
    };
    let above = |p: DVec3| p.truncate().extend(safe_z);
    let mut moves = vec![
        Move {
            kind: MoveKind::Rapid,
            to: above(first),
        },
        Move {
            kind: MoveKind::Plunge,
            to: first,
        },
    ];
    moves.extend(points[1..].iter().map(|&to| Move {
        kind: MoveKind::Cut,
        to,
    }));
    moves.push(Move {
        kind: MoveKind::Rapid,
        to: above(last),
    });
    moves
}

/// Project toolpath onto 3D Model: `moves` dropped in Z onto the job's 3D model, so a depth
/// below (or a lift above) the material surface becomes the same below (or above) the model's
/// surface, never below the material's bottom. Cuts are split into steps no longer than one of
/// the model's cells (sized for `tool`) so they follow it; rapids keep their height.
pub fn project_onto_model(
    doc: &Document,
    tool: &Tool,
    moves: &[Move],
) -> Result<Vec<Move>, CommandError> {
    let surface = placed_model(doc, tool)?;
    // the grid's far edges belong to its last cells, as does anything past them
    #[expect(clippy::cast_precision_loss, reason = "cell counts are small")]
    let far = surface.origin
        + Point::new(surface.columns as f64, surface.rows as f64) * surface.cell
        - Point::splat(1e-9);
    // ponytail: the tip follows the model's height under it, as the option is described; the
    // cutter is not dropped as a whole, so a wide tool's sides can touch a steep slope
    let drop = |p: DVec3| {
        let model = surface
            .at(p.truncate().clamp(surface.origin, far))
            .unwrap_or(0.0);
        p.truncate().extend((model + p.z).max(surface.bottom))
    };
    let mut projected = Vec::with_capacity(moves.len());
    let mut from: Option<DVec3> = None;
    for m in moves {
        match (m.kind, from) {
            (MoveKind::Rapid, _) => projected.push(*m),
            (kind, Some(a)) => {
                #[expect(
                    clippy::cast_possible_truncation,
                    clippy::cast_sign_loss,
                    reason = "a small positive count"
                )]
                let steps = (a.truncate().distance(m.to.truncate()) / surface.cell)
                    .ceil()
                    .max(1.0) as u32;
                projected.extend((1..=steps).map(|i| Move {
                    kind,
                    to: drop(a.lerp(m.to, f64::from(i) / f64::from(steps))),
                }));
            }
            (kind, None) => projected.push(Move {
                kind,
                to: drop(m.to),
            }),
        }
        from = Some(m.to);
    }
    Ok(projected)
}

/// Parallel lines `step` apart at `angle` degrees from the x axis, clipped to `regions`
/// (holes left out): each row's pieces as (start, end), rows in order across the regions.
pub fn hatch_lines(regions: &[Path], step: f64, angle: f64) -> Vec<Vec<(Point, Point)>> {
    let turn = Point::from_angle(angle.to_radians());
    // work with the rows along x: turn the shapes the other way, and back at the end
    let level = |p: Point| Point::new(turn.x * p.x + turn.y * p.y, turn.x * p.y - turn.y * p.x);
    let rings: Vec<Vec<Point>> = regions
        .iter()
        .map(|r| cut_points(r).into_iter().map(level).collect())
        .collect();
    let (low, high) = rings
        .iter()
        .flatten()
        .fold((f64::INFINITY, f64::NEG_INFINITY), |(l, h), p| {
            (l.min(p.y), h.max(p.y))
        });
    let mut rows = vec![];
    let mut y = low + step / 2.0;
    while y < high {
        let mut xs: Vec<f64> = rings
            .iter()
            .flat_map(|ring| {
                let n = ring.len();
                (0..n).filter_map(move |i| {
                    let (a, b) = (ring[i], ring[(i + 1) % n]);
                    ((a.y <= y) != (b.y <= y)).then(|| a.x + (y - a.y) / (b.y - a.y) * (b.x - a.x))
                })
            })
            .collect();
        xs.sort_by(f64::total_cmp);
        rows.push(
            xs.as_chunks::<2>()
                .0
                .iter()
                .map(|&[a, b]| (turn.rotate(Point::new(a, y)), turn.rotate(Point::new(b, y))))
                .collect(),
        );
        y += step;
    }
    rows
}

#[cfg(test)]
mod projection {
    use super::*;
    use crate::testing::domed;

    /// A cut `depth` deep along y = 20 over the middle of the dome, from flat to flat.
    fn across(depth: f64) -> Vec<Move> {
        let at = |kind, x: f64, z: f64| Move {
            kind,
            to: DVec3::new(x, 20.0, z),
        };
        vec![
            at(MoveKind::Rapid, 2.0, 5.0),
            at(MoveKind::Plunge, 2.0, -depth),
            at(MoveKind::Cut, 38.0, -depth),
            at(MoveKind::Rapid, 38.0, 5.0),
        ]
    }

    /// The dome's surface along that line: 8 mm tall with its top at the material surface.
    fn dome(x: f64) -> f64 {
        let r = (x - 20.0).abs();
        if r < 8.0 {
            (64.0 - r * r).sqrt() - 8.0
        } else {
            -8.0
        }
    }

    fn cuts(moves: &[Move]) -> Vec<DVec3> {
        moves
            .iter()
            .filter(|m| m.kind == MoveKind::Cut)
            .map(|m| m.to)
            .collect()
    }

    #[test]
    fn a_cut_follows_the_model_at_its_depth_below_it() {
        let moved = project_onto_model(&domed(), &Tool::default(), &across(1.0)).unwrap();
        let cuts = cuts(&moved);
        // split into steps no longer than a model cell, a twentieth of the 6 mm tool
        assert!(cuts.len() > 100, "{} pieces", cuts.len());
        assert!(
            cuts.windows(2)
                .all(|w| w[0].truncate().distance(w[1].truncate()) <= 0.3 + 1e-9)
        );
        for p in &cuts {
            // off the true dome by no more than its slope across a cell
            assert!(
                (p.z - (dome(p.x) - 1.0)).abs() < 0.5,
                "at x = {} cut at {}, the model at {}",
                p.x,
                p.z,
                dome(p.x)
            );
        }
        assert!(
            cuts.iter().any(|p| (p.z + 1.0).abs() < 0.05),
            "over the top"
        );
        // right on the material's far edge too
        let edge = [Move {
            kind: MoveKind::Plunge,
            to: DVec3::new(40.0, 40.0, -1.0),
        }];
        let dropped = project_onto_model(&domed(), &Tool::default(), &edge).unwrap();
        assert!((dropped[0].to.z + 9.0).abs() < 0.01, "{}", dropped[0].to.z);
        // rapids keep their height above the material
        assert!((moved.first().unwrap().to.z - 5.0).abs() < 1e-12);
        assert!((moved.last().unwrap().to.z - 5.0).abs() < 1e-12);
    }

    #[test]
    fn it_never_goes_below_the_material_bottom() {
        // 3 mm below the flat 8 mm down would be 11 mm into 10 mm thick material
        let moved = project_onto_model(&domed(), &Tool::default(), &across(3.0)).unwrap();
        let lowest = moved.iter().map(|m| m.to.z).fold(f64::INFINITY, f64::min);
        assert!((lowest + 10.0).abs() < 1e-9, "lowest {lowest}");
        let top = cuts(&moved)
            .into_iter()
            .find(|p| (p.x - 20.0).abs() < 0.2)
            .unwrap();
        assert!((top.z + 3.0).abs() < 0.05, "over the top at {}", top.z);
    }

    #[test]
    fn it_needs_a_model() {
        let why = project_onto_model(&Document::default(), &Tool::default(), &across(1.0))
            .unwrap_err()
            .to_string();
        assert!(why.contains("3D model"), "{why}");
    }
}
