//! Toolpaths: the tool, the strategy command that made them (so they can be recalculated),
//! and the computed moves.

use crate::document::EntityId;
use crate::geometry::{Path, Point};
use crate::{Command, polygon};
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
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Tool {
    pub name: String,
    #[serde(default)]
    pub shape: ToolShape,
    /// Included angle of a V-bit's point
    #[serde(default = "default_angle")]
    #[schemars(extend("x-unit" = "°"), range(min = 1, max = 179))]
    pub angle: f64,
    #[schemars(extend("x-unit" = "mm"), range(min = 0))]
    pub diameter: f64,
    /// Depth cut on each pass
    #[schemars(extend("x-unit" = "mm"), range(min = 0))]
    pub pass_depth: f64,
    /// Sideways step between pocketing passes, as a fraction of the diameter
    #[schemars(range(min = 0, max = 1))]
    pub stepover: f64,
    #[schemars(extend("x-unit" = "mm/min"), range(min = 0))]
    pub feed_rate: f64,
    #[schemars(extend("x-unit" = "mm/min"), range(min = 0))]
    pub plunge_rate: f64,
    #[schemars(extend("x-unit" = "rpm"), range(min = 0))]
    pub spindle_speed: f64,
}

fn default_angle() -> f64 {
    90.0
}

impl Tool {
    /// How far above its tip the cutting edge is at `r` from the axis; `None` beyond the tool.
    pub fn profile(&self, r: f64) -> Option<f64> {
        let radius = self.diameter / 2.0;
        (r <= radius).then(|| match self.shape {
            ToolShape::EndMill => 0.0,
            ToolShape::BallNose => radius - (radius * radius - r * r).max(0.0).sqrt(),
            ToolShape::VBit => r / (self.angle.to_radians() / 2.0).tan(),
        })
    }
}

impl Default for Tool {
    fn default() -> Self {
        Self {
            name: "6 mm End Mill".into(),
            shape: ToolShape::EndMill,
            angle: default_angle(),
            diameter: 6.0,
            pass_depth: 3.0,
            stepover: 0.4,
            feed_rate: 1000.0,
            plunge_rate: 300.0,
            spindle_speed: 18000.0,
        }
    }
}

/// The shared tools. A toolpath copies its tool, so editing the database never changes
/// toolpaths already made.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ToolDatabase {
    pub tools: Vec<Tool>,
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
        let shaped =
            |name: &str, shape: ToolShape, diameter: f64, angle: f64, stepover: f64| Tool {
                shape,
                angle,
                stepover,
                ..end_mill(name, diameter, 3.0, 1500.0, 500.0)
            };
        Self {
            tools: vec![
                end_mill("3 mm End Mill", 3.0, 1.5, 800.0, 250.0),
                Tool::default(),
                end_mill("12 mm End Mill", 12.0, 4.0, 2000.0, 500.0),
                end_mill("1/8 in End Mill", 3.175, 1.5, 800.0, 250.0),
                end_mill("1/4 in End Mill", 6.35, 3.0, 1000.0, 300.0),
                shaped("3 mm Ball Nose", ToolShape::BallNose, 3.0, 90.0, 0.1),
                shaped("6 mm Ball Nose", ToolShape::BallNose, 6.0, 90.0, 0.1),
                shaped("90° V-Bit", ToolShape::VBit, 12.7, 90.0, 0.2),
                shaped("60° V-Bit", ToolShape::VBit, 12.7, 60.0, 0.2),
            ],
        }
    }

    pub fn get(&self, name: &str) -> Option<&Tool> {
        self.tools.iter().find(|t| t.name == name)
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
    pub moves: Vec<Move>,
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
