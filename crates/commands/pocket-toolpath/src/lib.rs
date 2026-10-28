//! Pocket Toolpath: clears the inside of the selected closed vectors at each depth, with
//! rings that step in from the outline (worked from the middle outwards) or with raster
//! lines and a profile pass round the edge.

use cut_craft_core::document::Document;
use cut_craft_core::entry::{Entry, Ramp, Ramping, path_moves};
use cut_craft_core::geometry::Path;
use cut_craft_core::polygon::{self, Corners};
use cut_craft_core::toolpath::{Move, Tool, cut_points, hatch_lines, made_by, pass_depths};
use cut_craft_core::{Change, Command, CommandError, Meta, ensure_positive};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub struct PocketToolpath;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Strategy {
    /// Rings stepping in from the outline
    #[default]
    Offset,
    /// Straight lines across, at the raster angle
    Raster,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ProfilePass {
    /// Round the edge before the raster lines
    First,
    /// Round the edge after them
    #[default]
    Last,
    /// No pass round the edge: it is left rough
    None,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
pub struct Params {
    pub name: String,
    /// Below the material surface
    #[schemars(extend("x-unit" = "mm"), range(min = 0))]
    pub depth: f64,
    pub tool: Tool,
    #[serde(default)]
    pub strategy: Strategy,
    /// 0° along X, 90° along Y
    #[serde(default)]
    #[schemars(
        extend("x-unit" = "°", "x-show-when" = {"strategy": "raster"}),
        range(min = 0, max = 90)
    )]
    pub raster_angle: f64,
    #[serde(default)]
    #[schemars(extend("x-show-when" = {"strategy": "raster"}))]
    pub profile_pass: ProfilePass,
    /// Zig zag down over the ramp distance instead of plunging straight down
    #[serde(default)]
    pub ramp_plunge_moves: bool,
    #[serde(default = "ramp_distance")]
    #[schemars(extend("x-unit" = "mm"), range(min = 0))]
    pub ramp_distance: f64,
}

fn ramp_distance() -> f64 {
    Ramping::default().distance
}

impl Default for Params {
    fn default() -> Self {
        Self {
            name: "Pocket".into(),
            depth: 3.0,
            tool: Tool::default(),
            strategy: Strategy::Offset,
            raster_angle: 0.0,
            profile_pass: ProfilePass::Last,
            ramp_plunge_moves: false,
            ramp_distance: ramp_distance(),
        }
    }
}

impl Command for PocketToolpath {
    const ID: &'static str = "pocket_toolpath";
    type Params = Params;

    fn meta() -> Meta {
        Meta {
            label: "Pocket Toolpath",
            group: "Toolpath Operations",
            icon: "toolpaths/pocket_toolpath.png",
            shortcut: "",
        }
    }

    fn run(doc: &Document, p: &Params) -> Result<Change, CommandError> {
        ensure_positive("depth", p.depth)?;
        ensure_positive("tool diameter", p.tool.diameter)?;
        ensure_positive("pass depth", p.tool.pass_depth)?;
        if !(p.tool.stepover > 0.0 && p.tool.stepover <= 1.0) {
            return Err(CommandError::InvalidParams(
                "stepover must be more than 0 and at most 1".into(),
            ));
        }
        let (ids, paths) = doc.selected_paths(|path| path.closed);
        if ids.is_empty() {
            return Err(CommandError::Failed(
                "select the closed vectors to pocket".into(),
            ));
        }
        // ponytail: each selected shape is a filled region, so islands (shapes inside shapes)
        // are pocketed over; nest-aware regions when islands are needed
        let mut ring = polygon::offset(
            &polygon::union(&paths),
            -p.tool.diameter / 2.0,
            Corners::Round,
        );
        if ring.is_empty() {
            return Err(CommandError::Failed(
                "the tool is too big to fit inside the selected shapes".into(),
            ));
        }
        let step = p.tool.stepover * p.tool.diameter;
        let edge = ring.clone();
        let mut rings = vec![];
        while !ring.is_empty() {
            let next = polygon::offset(&ring, -step, Corners::Round);
            rings.extend(ring);
            ring = next;
        }
        rings.reverse(); // from the middle outwards
        let safe = doc.job.clearance;
        let entry = Entry {
            ramp: if p.ramp_plunge_moves {
                Ramp::ZigZag
            } else {
                Ramp::None
            },
            ramping: Ramping {
                distance: p.ramp_distance,
                ..Ramping::default()
            },
            ..Entry::default()
        };
        // each path at depth `z`, ramping down from the level above
        let cut = |paths: &[Path], z: f64, above: f64| -> Vec<Move> {
            paths
                .iter()
                .flat_map(|r| path_moves(&cut_points(r), r.closed, &[z], above, safe, &entry))
                .collect()
        };
        // ponytail: each ring or line retracts between cuts; link them with a stepover move
        // to save time
        let mut moves = vec![];
        let mut above = 0.0;
        for z in pass_depths(p.depth, p.tool.pass_depth) {
            match p.strategy {
                Strategy::Offset => moves.extend(cut(&rings, z, above)),
                Strategy::Raster => {
                    if p.profile_pass == ProfilePass::First {
                        moves.extend(cut(&edge, z, above));
                    }
                    let lines: Vec<Path> = hatch_lines(&edge, step, p.raster_angle)
                        .into_iter()
                        .enumerate()
                        .flat_map(|(row, spans)| {
                            let mut spans: Vec<Path> = spans
                                .into_iter()
                                .map(|(a, b)| Path::polyline(&[a, b], false))
                                .collect();
                            // every other row back the other way
                            if row % 2 == 1 {
                                spans.reverse();
                                spans = spans.iter().map(Path::reversed).collect();
                            }
                            spans
                        })
                        .collect();
                    moves.extend(cut(&lines, z, above));
                    if p.profile_pass == ProfilePass::Last {
                        moves.extend(cut(&edge, z, above));
                    }
                }
            }
            above = z;
        }
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
    use cut_craft_core::geometry::{Point, polyline_distance};
    use cut_craft_core::testing::{cut_bounds, made_toolpath, square};
    use cut_craft_core::toolpath::{MoveKind, Toolpath};

    cut_craft_core::contract_tests!(PocketToolpath);

    fn pocket(size: f64, p: &Params) -> Result<Toolpath, String> {
        made_toolpath::<PocketToolpath>(vec![square(0.0, 0.0, size)], p)
    }

    #[test]
    fn stays_a_tool_radius_inside_the_shape() {
        let b = cut_bounds(&pocket(20.0, &Params::default()).unwrap());
        assert!(
            b.min.x >= 3.0 - 0.02
                && b.max.x <= 17.0 + 0.02
                && b.min.y >= 3.0 - 0.02
                && b.max.y <= 17.0 + 0.02,
            "{b:?}"
        );
    }

    #[test]
    fn the_tool_reaches_the_middle() {
        let tp = pocket(20.0, &Params::default()).unwrap();
        let middle = Point::new(10.0, 10.0);
        // distance to the cut segments themselves, not just their end points
        let closest = tp
            .moves
            .windows(2)
            .filter(|w| w[1].kind == MoveKind::Cut)
            .map(|w| polyline_distance(&[w[0].to.truncate(), w[1].to.truncate()], middle))
            .fold(f64::MAX, f64::min);
        assert!(
            closest <= Tool::default().diameter / 2.0,
            "the middle is {closest} mm from the nearest cut"
        );
    }

    #[test]
    fn clears_each_depth_before_going_deeper() {
        let tp = pocket(
            20.0,
            &Params {
                depth: 6.0,
                ..Params::default()
            },
        )
        .unwrap();
        let depths: Vec<f64> = tp
            .moves
            .iter()
            .filter(|m| m.kind == MoveKind::Plunge)
            .map(|m| m.to.z)
            .collect();
        assert!(
            depths.windows(2).all(|w| w[1] <= w[0]),
            "never climbs back to a shallower pass: {depths:?}"
        );
        assert_eq!(depths.first(), Some(&-3.0));
        assert_eq!(depths.last(), Some(&-6.0));
    }

    #[test]
    fn a_shape_smaller_than_the_tool_cannot_be_pocketed() {
        assert_eq!(
            pocket(4.0, &Params::default()).unwrap_err(),
            "the tool is too big to fit inside the selected shapes"
        );
    }

    #[test]
    fn stepover_must_be_a_fraction_of_the_tool() {
        let p = Params {
            tool: Tool {
                stepover: 0.0,
                ..Tool::default()
            },
            ..Params::default()
        };
        assert_eq!(
            pocket(20.0, &p).unwrap_err(),
            "invalid parameters: stepover must be more than 0 and at most 1"
        );
    }

    #[test]
    fn raster_lines_run_at_the_angle_with_a_profile_pass_round_the_edge() {
        let p = Params {
            strategy: Strategy::Raster,
            raster_angle: 90.0,
            ..Params::default()
        };
        let tp = pocket(20.0, &p).unwrap();
        let b = cut_bounds(&tp);
        assert!(b.min.x >= 3.0 - 0.02 && b.max.x <= 17.0 + 0.02, "{b:?}");
        // the lines run along Y: most cuts keep x and change y
        let along_y = tp
            .moves
            .windows(2)
            .filter(|w| w[1].kind == MoveKind::Cut)
            .filter(|w| (w[0].to.x - w[1].to.x).abs() < 1e-9 && (w[0].to.y - w[1].to.y).abs() > 5.0)
            .count();
        assert!(along_y >= 4, "{along_y}");
        // the last cuts go round the edge, a tool radius in
        let last = tp
            .moves
            .iter()
            .rev()
            .find(|m| m.kind == MoveKind::Cut)
            .unwrap();
        let p = last.to.truncate();
        let on_edge = [p.x - 3.0, 17.0 - p.x, p.y - 3.0, 17.0 - p.y]
            .iter()
            .any(|d| d.abs() < 0.05);
        assert!(on_edge, "{p}");
    }

    #[test]
    fn ramp_plunge_moves_zig_zag_down_from_the_level_above() {
        let p = Params {
            depth: 6.0,
            ramp_plunge_moves: true,
            ramp_distance: 4.0,
            ..Params::default()
        };
        let tp = pocket(20.0, &p).unwrap();
        // straight down only to the surface or the level already cleared, never into
        // material
        let straight: Vec<f64> = tp
            .moves
            .windows(2)
            .filter(|w| {
                let (a, b) = (w[0].to, w[1].to);
                b.z < a.z - 1e-9 && a.truncate().distance(b.truncate()) < 1e-9
            })
            .map(|w| w[1].to.z)
            .collect();
        assert!(
            straight
                .iter()
                .all(|&z| z.abs() < 1e-9 || (z + 3.0).abs() < 1e-9),
            "{straight:?}"
        );
    }
}
