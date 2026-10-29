//! Thread Milling Toolpath: a single-point thread mill spirals round the centre of each
//! selected vector, cutting an internal thread (a nut) or an external one (a bolt). The
//! thread mill's tooth comes from the tool; jobs saved before thread mills were tools give
//! its offset on the form.

use cut_craft_core::document::Document;
use cut_craft_core::geometry::Point;
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

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
pub struct Params {
    pub name: String,
    /// Below the material surface where the thread starts
    #[schemars(extend("x-unit" = "mm"), range(min = 0))]
    pub start_depth: f64,
    /// Deepest the bottom of the tool goes below the start depth
    #[schemars(extend("x-unit" = "mm"), range(min = 0))]
    pub max_depth: f64,
    /// Between the ridges of the thread
    #[schemars(extend("x-unit" = "mm"), range(min = 0))]
    pub pitch: f64,
    /// The thread's major (largest) diameter
    #[schemars(extend("x-unit" = "mm"), range(min = 0))]
    pub diameter: f64,
    /// Cuts the thread this much deeper so it turns freely
    #[schemars(extend("x-unit" = "mm"), range(min = 0))]
    pub fit_tolerance: f64,
    pub thread_type: ThreadType,
    pub thread_direction: Hand,
    pub cut_direction: CutDirection,
    /// From the bottom of the thread mill up to the tip of its tooth, for a tool that is not
    /// a thread mill (a thread mill gives its own)
    #[schemars(extend("x-unit" = "mm"), range(min = 0))]
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
            pitch: 1.5,
            diameter: 20.0,
            fit_tolerance: 0.0,
            thread_type: ThreadType::Internal,
            thread_direction: Hand::RightHanded,
            cut_direction: CutDirection::Down,
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
                self.diameter / 2.0 - 0.613_435 * self.pitch - self.fit_tolerance + r
            }
        }
    }

    fn helix(&self, centre: Point, safe: f64) -> Vec<Move> {
        let radius = self.centre_radius();
        let top = -(self.start_depth + self.tooth_offset());
        let bottom = -(self.start_depth + self.max_depth);
        let turns = (top - bottom) / self.pitch;
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
        ensure_positive("pitch", p.pitch)?;
        ensure_positive("diameter", p.diameter)?;
        ensure_positive("tool diameter", p.tool.diameter)?;
        ensure_positive("tooth offset", p.tooth_offset())?;
        // Aspire's rule for a single-point tool's offset
        if p.tool.shape == ToolShape::ThreadMill && p.tool.tooth_offset < p.tool.tooth_size / 2.0 {
            return Err(CommandError::InvalidParams(
                "the thread mill's tooth offset must be at least half the tooth size".into(),
            ));
        }
        if p.thread_length() < p.pitch {
            return Err(CommandError::InvalidParams(
                "the max depth must reach at least a pitch below the tooth offset".into(),
            ));
        }
        if p.centre_radius() <= 0.0 {
            return Err(CommandError::InvalidParams(
                "the thread mill is too big for this thread".into(),
            ));
        }
        let (ids, paths) = doc.selected_paths(|_| true);
        if ids.is_empty() {
            return Err(CommandError::Failed(
                "select the vectors to thread round their centres".into(),
            ));
        }
        let safe = doc.job.clearance;
        let moves = paths
            .iter()
            .flat_map(|path| p.helix(path.bounds().center(), safe))
            .collect();
        let mut c = Change::builder(doc);
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
}
