//! Thread Milling Toolpath: a single-point thread mill spirals round the centre of each
//! selected vector, cutting an internal thread (a nut) or an external one (a bolt). The
//! thread mill's tooth comes from the tool; jobs saved before thread mills were tools give
//! its offset on the form.

use cut_craft_core::document::Document;
use cut_craft_core::geometry::{Path, Point};
use cut_craft_core::glam::DVec3;
use cut_craft_core::toolpath::{Move, MoveKind, Tool, ToolDatabase, ToolShape, made_by};
use cut_craft_core::{Change, Command, CommandError, Meta, ensure_positive};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub struct ThreadMillingToolpath;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ThreadType {
    /// Inside a hole, for a bolt to screw into
    #[default]
    Internal,
    /// Round a post, as on a bolt
    External,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Hand {
    #[default]
    RightHanded,
    LeftHanded,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum CutDirection {
    /// Spiralling down from the top of the thread
    #[default]
    Down,
    /// Spiralling up from the bottom
    Up,
}

/// Aspire's Pitch Preset: a standard thread's pitch, ISO metric coarse or Unified (UNC),
/// else the pitch typed.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum PitchPreset {
    #[default]
    Custom,
    #[schemars(title = "ISO M2 × 0.4")]
    IsoM2,
    #[schemars(title = "ISO M3 × 0.5")]
    IsoM3,
    #[schemars(title = "ISO M4 × 0.7")]
    IsoM4,
    #[schemars(title = "ISO M5 × 0.8")]
    IsoM5,
    #[schemars(title = "ISO M6 × 1.0")]
    IsoM6,
    #[schemars(title = "ISO M8 × 1.25")]
    IsoM8,
    #[schemars(title = "ISO M10 × 1.5")]
    IsoM10,
    #[schemars(title = "ISO M12 × 1.75")]
    IsoM12,
    #[schemars(title = "ISO M14 × 2.0")]
    IsoM14,
    #[schemars(title = "ISO M16 × 2.0")]
    IsoM16,
    #[schemars(title = "ISO M20 × 2.5")]
    IsoM20,
    #[schemars(title = "ISO M24 × 3.0")]
    IsoM24,
    #[schemars(title = "#4-40 UNC")]
    Uts4x40,
    #[schemars(title = "#6-32 UNC")]
    Uts6x32,
    #[schemars(title = "#8-32 UNC")]
    Uts8x32,
    #[schemars(title = "#10-24 UNC")]
    Uts10x24,
    #[schemars(title = "#10-32 UNF")]
    Uts10x32,
    #[schemars(title = "1/4-20 UNC")]
    UtsQuarterX20,
    #[schemars(title = "1/4-28 UNF")]
    UtsQuarterX28,
    #[schemars(title = "5/16-18 UNC")]
    UtsFiveSixteenthsX18,
    #[schemars(title = "3/8-16 UNC")]
    UtsThreeEighthsX16,
    #[schemars(title = "7/16-14 UNC")]
    UtsSevenSixteenthsX14,
    #[schemars(title = "1/2-13 UNC")]
    UtsHalfX13,
    #[schemars(title = "9/16-12 UNC")]
    UtsNineSixteenthsX12,
    #[schemars(title = "5/8-11 UNC")]
    UtsFiveEighthsX11,
    #[schemars(title = "3/4-10 UNC")]
    UtsThreeQuartersX10,
    #[schemars(title = "1-8 UNC")]
    UtsOneX8,
}

impl PitchPreset {
    /// The standard's pitch in millimetres; none for Custom.
    #[must_use]
    pub fn pitch(self) -> Option<f64> {
        let per_inch = |threads: f64| 25.4 / threads;
        Some(match self {
            Self::Custom => return None,
            Self::IsoM2 => 0.4,
            Self::IsoM3 => 0.5,
            Self::IsoM4 => 0.7,
            Self::IsoM5 => 0.8,
            Self::IsoM6 => 1.0,
            Self::IsoM8 => 1.25,
            Self::IsoM10 => 1.5,
            Self::IsoM12 => 1.75,
            Self::IsoM14 | Self::IsoM16 => 2.0,
            Self::IsoM20 => 2.5,
            Self::IsoM24 => 3.0,
            Self::Uts4x40 => per_inch(40.0),
            Self::Uts6x32 | Self::Uts8x32 | Self::Uts10x32 => per_inch(32.0),
            Self::Uts10x24 => per_inch(24.0),
            Self::UtsQuarterX20 => per_inch(20.0),
            Self::UtsQuarterX28 => per_inch(28.0),
            Self::UtsFiveSixteenthsX18 => per_inch(18.0),
            Self::UtsThreeEighthsX16 => per_inch(16.0),
            Self::UtsSevenSixteenthsX14 => per_inch(14.0),
            Self::UtsHalfX13 => per_inch(13.0),
            Self::UtsNineSixteenthsX12 => per_inch(12.0),
            Self::UtsFiveEighthsX11 => per_inch(11.0),
            Self::UtsThreeQuartersX10 => per_inch(10.0),
            Self::UtsOneX8 => per_inch(8.0),
        })
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
pub struct Params {
    pub name: String,
    /// Below the material surface where the thread starts
    #[schemars(extend("x-unit" = "mm"), range(min = 0))]
    #[schemars(extend("x-group" = "Cutting Depths"))]
    pub start_depth: f64,
    /// Deepest the bottom of the tool goes below the start depth
    #[schemars(extend("x-unit" = "mm"), range(min = 0))]
    #[schemars(extend("x-group" = "Cutting Depths"))]
    pub max_depth: f64,
    /// A standard thread's pitch, else the pitch typed below
    #[serde(default)]
    #[schemars(title = "Pitch preset", extend("x-widget" = "drop-down"))]
    #[schemars(extend("x-group" = "Thread"))]
    pub pitch_preset: PitchPreset,
    /// Between the ridges of the thread
    #[schemars(extend("x-unit" = "mm"), range(min = 0))]
    #[schemars(extend("x-group" = "Thread"), extend("x-show-when" = {"pitch_preset": "custom"}))]
    pub pitch: f64,
    /// The thread's major (largest) diameter
    #[schemars(extend("x-unit" = "mm"), range(min = 0))]
    #[schemars(extend("x-group" = "Thread"))]
    pub diameter: f64,
    /// Cuts the thread this much deeper so it turns freely
    #[schemars(extend("x-unit" = "mm"), range(min = 0))]
    #[schemars(extend("x-group" = "Thread"))]
    pub fit_tolerance: f64,
    #[schemars(extend("x-group" = "Thread"))]
    pub thread_type: ThreadType,
    #[schemars(extend("x-group" = "Thread"))]
    pub thread_direction: Hand,
    #[schemars(extend("x-group" = "Thread"))]
    pub cut_direction: CutDirection,
    /// Internal threads: also draws a circle of the thread's minor diameter at each centre,
    /// on the current layer, for a Pocket toolpath to clear the hole first
    #[serde(default)]
    #[schemars(title = "Create Circles for Internal Threads")]
    #[schemars(extend("x-group" = "Thread"))]
    pub create_circles: bool,
    /// Thread the vectors in the order they were selected, else in the order that cuts
    /// quickest
    #[serde(default)]
    #[schemars(title = "Use Vector Selection Order")]
    pub use_selection_order: bool,
    /// From the bottom of the thread mill up to the tip of its tooth, for a tool that is not
    /// a thread mill (a thread mill gives its own)
    #[schemars(extend("x-unit" = "mm"), range(min = 0))]
    #[schemars(extend("x-group" = "Thread"))]
    pub tooth_offset: f64,
    /// A single-point thread mill; its diameter is tip to tip
    pub tool: Tool,
}

impl Default for Params {
    fn default() -> Self {
        Self {
            name: "Thread Milling".into(),
            start_depth: 0.0,
            max_depth: 12.0,
            pitch_preset: PitchPreset::Custom,
            pitch: 1.5,
            diameter: 20.0,
            fit_tolerance: 0.0,
            thread_type: ThreadType::Internal,
            thread_direction: Hand::RightHanded,
            cut_direction: CutDirection::Down,
            create_circles: false,
            use_selection_order: false,
            tooth_offset: 2.0,
            tool: ToolDatabase::starter()
                .tools
                .into_iter()
                .find(|t| t.shape == ToolShape::ThreadMill)
                .expect("the starter tools include a thread mill"),
        }
    }
}

/// Straight moves per turn of the helix.
const TURN_STEPS: u32 = 72;

impl Params {
    /// From the bottom of the tool to the tip of its tooth: the thread mill's own, else the
    /// form's.
    fn tooth_offset(&self) -> f64 {
        match self.tool.shape {
            ToolShape::ThreadMill => self.tool.tooth_offset,
            _ => self.tooth_offset,
        }
    }

    /// The pitch: the preset's, else the one typed.
    #[must_use]
    pub fn pitch(&self) -> f64 {
        self.pitch_preset.pitch().unwrap_or(self.pitch)
    }

    /// How far the threads go down: from the start to the tooth at the deepest.
    pub fn thread_length(&self) -> f64 {
        self.max_depth - self.tooth_offset()
    }

    /// Radius of the circle the tool's centre runs round.
    fn centre_radius(&self) -> f64 {
        let r = self.tool.diameter / 2.0;
        match self.thread_type {
            // the tooth reaches out to the major diameter
            ThreadType::Internal => self.diameter / 2.0 + self.fit_tolerance - r,
            // the tooth reaches in to an ISO external thread's minor diameter (d - 1.22687 P)
            ThreadType::External => {
                self.diameter / 2.0 - 0.613_435 * self.pitch() - self.fit_tolerance + r
            }
        }
    }

    fn helix(&self, centre: Point, safe: f64) -> Vec<Move> {
        let radius = self.centre_radius();
        let top = -(self.start_depth + self.tooth_offset());
        let bottom = -(self.start_depth + self.max_depth);
        let turns = (top - bottom) / self.pitch();
        // a right-handed thread climbs anticlockwise
        let climb = if self.thread_direction == Hand::RightHanded {
            1.0
        } else {
            -1.0
        };
        #[expect(
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            reason = "a count"
        )]
        let steps = (turns * f64::from(TURN_STEPS)).ceil().max(1.0) as u32;
        let mut spiral: Vec<DVec3> = (0..=steps)
            .map(|i| {
                let t = f64::from(i) / f64::from(steps);
                let angle = climb * turns * std::f64::consts::TAU * t;
                (centre + radius * Point::from_angle(angle)).extend(bottom + (top - bottom) * t)
            })
            .collect();
        if self.cut_direction == CutDirection::Down {
            spiral.reverse();
        }
        let first = spiral[0];
        let last = spiral[spiral.len() - 1];
        // inside a hole the tool goes down the middle; round a post it comes down outside
        let clear = |p: DVec3| match self.thread_type {
            ThreadType::Internal => centre.extend(p.z),
            ThreadType::External => {
                let out = (p.truncate() - centre).normalize_or(Point::X);
                (p.truncate() + out * self.tool.diameter).extend(p.z)
            }
        };
        let mut moves = vec![
            Move {
                kind: MoveKind::Rapid,
                to: clear(first).truncate().extend(safe),
            },
            Move {
                kind: MoveKind::Plunge,
                to: clear(first),
            },
            Move {
                kind: MoveKind::Cut,
                to: first,
            },
        ];
        moves.extend(spiral[1..].iter().map(|&to| Move {
            kind: MoveKind::Cut,
            to,
        }));
        moves.push(Move {
            kind: MoveKind::Cut,
            to: clear(last),
        });
        moves.push(Move {
            kind: MoveKind::Rapid,
            to: clear(last).truncate().extend(safe),
        });
        moves
    }
}

/// `points` from the first, each the nearest to the one before: the order that cuts quickest.
fn nearest_first(mut points: Vec<Point>) -> Vec<Point> {
    let mut ordered = Vec::with_capacity(points.len());
    while !points.is_empty() {
        let from = ordered.last().copied();
        let next = from.map_or(0, |from: Point| {
            points
                .iter()
                .enumerate()
                .min_by(|(_, a), (_, b)| a.distance(from).total_cmp(&b.distance(from)))
                .map_or(0, |(i, _)| i)
        });
        ordered.push(points.remove(next));
    }
    ordered
}

impl Command for ThreadMillingToolpath {
    const ID: &'static str = "thread_milling_toolpath";
    type Params = Params;

    fn meta() -> Meta {
        Meta {
            label: "Thread Milling Toolpath",
            group: "Toolpath Operations",
            icon: "toolpaths/circular_pocket.png",
            shortcut: "",
        }
    }

    fn run(doc: &Document, p: &Params) -> Result<Change, CommandError> {
        ensure_positive("pitch", p.pitch())?;
        ensure_positive("diameter", p.diameter)?;
        ensure_positive("tool diameter", p.tool.diameter)?;
        ensure_positive("tooth offset", p.tooth_offset())?;
        // Aspire's rule for a single-point tool's offset
        if p.tool.shape == ToolShape::ThreadMill && p.tool.tooth_offset < p.tool.tooth_size / 2.0 {
            return Err(CommandError::InvalidParams(
                "set the thread mill's tooth offset to at least half the tooth size".into(),
            ));
        }
        if p.thread_length() < p.pitch() {
            return Err(CommandError::InvalidParams(
                "set the max depth at least a pitch below the tooth offset".into(),
            ));
        }
        if p.centre_radius() <= 0.0 {
            return Err(CommandError::InvalidParams(
                "the thread mill is too big for this thread. Pick a smaller thread mill".into(),
            ));
        }
        let (ids, paths) = doc.selected_paths(|_| true);
        if ids.is_empty() {
            return Err(CommandError::Failed(
                "select the vectors to thread round their centres".into(),
            ));
        }
        let safe = doc.job.clearance;
        let mut centres: Vec<Point> = paths.iter().map(|path| path.bounds().center()).collect();
        if !p.use_selection_order {
            centres = nearest_first(centres);
        }
        let moves = centres.iter().flat_map(|&at| p.helix(at, safe)).collect();
        let mut c = Change::builder(doc);
        // the hole to pocket out before an internal thread: ISO's minor diameter
        let minor = p.diameter - 1.082_532 * p.pitch();
        if p.create_circles && p.thread_type == ThreadType::Internal && minor > 0.0 {
            for &at in &centres {
                c.add(Path::circle(at, minor / 2.0));
            }
        }
        c.add_toolpath(made_by::<Self>(
            p,
            p.name.clone(),
            p.tool.clone(),
            ids,
            moves,
        ));
        Ok(c.finish())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cut_craft_core::geometry::Path;
    use cut_craft_core::testing::made_toolpath;
    use cut_craft_core::toolpath::Toolpath;

    cut_craft_core::contract_tests!(ThreadMillingToolpath);

    fn thread(p: &Params) -> Toolpath {
        made_toolpath::<ThreadMillingToolpath>(vec![Path::circle(Point::new(50.0, 50.0), 10.0)], p)
            .unwrap()
    }

    fn helix(t: &Toolpath) -> Vec<DVec3> {
        let cuts: Vec<DVec3> = t
            .moves
            .iter()
            .filter(|m| m.kind == MoveKind::Cut)
            .map(|m| m.to)
            .collect();
        cuts[..cuts.len() - 1].to_vec() // leave out the move off the thread
    }

    #[test]
    fn an_internal_thread_spirals_down_one_pitch_a_turn_at_the_major_diameter() {
        let p = Params::default();
        let h = helix(&thread(&p));
        let centre = Point::new(50.0, 50.0);
        assert!(
            h.iter()
                .all(|q| (q.truncate().distance(centre) - 5.0).abs() < 1e-9),
            "20 mm thread, 10 mm tool"
        );
        assert!(
            (h[0].z + 2.0).abs() < 1e-9,
            "starts with the tooth at the surface"
        );
        assert!(
            (h[h.len() - 1].z + 12.0).abs() < 1e-9,
            "ends at the max depth"
        );
        let per_turn = h[0].z - h[TURN_STEPS as usize].z;
        assert!((per_turn - 1.5).abs() < 1e-6, "{per_turn}");
    }

    #[test]
    fn going_down_a_right_handed_thread_turns_clockwise_and_a_left_handed_one_anticlockwise() {
        let spin = |hand| {
            let h = helix(&thread(&Params {
                thread_direction: hand,
                ..Params::default()
            }));
            let c = Point::new(50.0, 50.0);
            (h[0].truncate() - c).perp_dot(h[1].truncate() - c)
        };
        assert!(spin(Hand::RightHanded) < 0.0);
        assert!(spin(Hand::LeftHanded) > 0.0);
    }

    #[test]
    fn cutting_up_starts_at_the_bottom() {
        let h = helix(&thread(&Params {
            cut_direction: CutDirection::Up,
            ..Params::default()
        }));
        assert!((h[0].z + 12.0).abs() < 1e-9);
    }

    #[test]
    fn an_external_thread_runs_outside_reaching_the_minor_diameter() {
        let p = Params {
            thread_type: ThreadType::External,
            ..Params::default()
        };
        let t = thread(&p);
        let c = Point::new(50.0, 50.0);
        let tip = helix(&t)[0].truncate().distance(c) - 5.0;
        assert!((tip - (10.0 - 0.613_435 * 1.5)).abs() < 1e-6, "{tip}");
        let plunge = t.moves.iter().find(|m| m.kind == MoveKind::Plunge).unwrap();
        assert!(
            plunge.to.truncate().distance(c) > 15.0,
            "comes down clear of the post"
        );
    }

    #[test]
    fn a_thread_mill_brings_its_own_tooth_offset_and_older_tools_use_the_forms() {
        let mill = Params {
            tooth_offset: 5.0,
            tool: Tool {
                tooth_offset: 3.0,
                ..Params::default().tool
            },
            ..Params::default()
        };
        assert_eq!(mill.tool.shape, ToolShape::ThreadMill, "the starter's");
        let h = helix(&thread(&mill));
        assert!(
            (h[0].z + 3.0).abs() < 1e-9,
            "the tool's offset, not the form's"
        );
        assert!((mill.thread_length() - 9.0).abs() < 1e-9);
        // a job saved before thread mills were tools: an end mill and the form's offset
        let old = Params {
            tool: Tool {
                diameter: 10.0,
                ..Tool::default()
            },
            ..mill
        };
        let h = helix(&thread(&old));
        assert!((h[0].z + 5.0).abs() < 1e-9);
    }

    #[test]
    fn a_tooth_offset_under_half_the_tooth_size_is_refused() {
        let p = Params {
            tool: Tool {
                tooth_size: 2.0,
                tooth_offset: 0.9,
                ..Params::default().tool
            },
            ..Params::default()
        };
        let why = made_toolpath::<ThreadMillingToolpath>(vec![Path::circle(Point::ZERO, 10.0)], &p)
            .unwrap_err();
        assert!(why.contains("half the tooth size"), "{why}");
    }

    #[test]
    fn a_tool_too_big_for_the_hole_is_refused() {
        let p = Params {
            diameter: 8.0,
            ..Params::default()
        };
        let why = made_toolpath::<ThreadMillingToolpath>(vec![Path::circle(Point::ZERO, 4.0)], &p)
            .unwrap_err();
        assert!(why.contains("too big"), "{why}");
    }

    #[test]
    fn a_pitch_preset_sets_the_pitch_and_internal_threads_can_draw_their_circles() {
        assert!((PitchPreset::UtsQuarterX20.pitch().unwrap() - 1.27).abs() < 1e-12);
        assert_eq!(PitchPreset::Custom.pitch(), None);
        let p = Params {
            pitch_preset: PitchPreset::IsoM20,
            pitch: 9.0,
            create_circles: true,
            ..Params::default()
        };
        assert!(
            (p.pitch() - 2.5).abs() < 1e-12,
            "the preset, not the 9 typed"
        );
        let h = helix(&thread(&p));
        let per_turn = h[0].z - h[TURN_STEPS as usize].z;
        assert!((per_turn - 2.5).abs() < 1e-6, "{per_turn}");
        // the circle to pocket first: the minor diameter, 1.0825 pitches under the major
        let mut doc = cut_craft_core::testing::document_with(vec![Path::circle(
            Point::new(50.0, 50.0),
            10.0,
        )]);
        let change = ThreadMillingToolpath::run(&doc, &p).unwrap();
        doc.apply(&change).unwrap();
        assert_eq!(
            doc.vectors.len(),
            2,
            "the circle beside the vector threaded"
        );
        let drawn = doc
            .vectors
            .values()
            .map(|v| v.bounds().size().x)
            .fold(f64::INFINITY, f64::min);
        assert!((drawn - (20.0 - 1.082_532 * 2.5)).abs() < 1e-6, "{drawn}");
        let external = Params {
            thread_type: ThreadType::External,
            ..p
        };
        let change = ThreadMillingToolpath::run(&doc, &external).unwrap();
        assert!(
            change.added().next().is_none(),
            "no circle for an external thread"
        );
    }

    #[test]
    fn threads_are_cut_nearest_first_unless_the_selection_order_is_kept() {
        let circles = |xs: &[f64]| {
            xs.iter()
                .map(|&x| Path::circle(Point::new(x, 50.0), 10.0))
                .collect::<Vec<_>>()
        };
        let starts = |p: &Params| -> Vec<f64> {
            let t =
                made_toolpath::<ThreadMillingToolpath>(circles(&[0.0, 100.0, 10.0]), p).unwrap();
            let mut xs: Vec<f64> = t
                .moves
                .iter()
                .filter(|m| m.kind == MoveKind::Rapid)
                .map(|m| m.to.x)
                .collect();
            xs.dedup();
            xs
        };
        assert_eq!(starts(&Params::default()), [0.0, 10.0, 100.0]);
        let kept = Params {
            use_selection_order: true,
            ..Params::default()
        };
        assert_eq!(starts(&kept), [0.0, 100.0, 10.0]);
    }
}
