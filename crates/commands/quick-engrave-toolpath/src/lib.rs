//! Quick Engrave: marks the selected vectors at a shallow depth, along their lines or filling
//! them with an offset, hatch or cross hatch pattern.

use cut_craft_core::document::Document;
use cut_craft_core::geometry::Path;
use cut_craft_core::polygon::{self, Corners};
use cut_craft_core::toolpath::{Move, Tool, contour_moves, cut_points, hatch_lines, made_by};
use cut_craft_core::{Change, Command, CommandError, Meta, ensure_positive};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub struct QuickEngraveToolpath;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Strategy {
    /// The tip runs along the lines
    #[default]
    Outline,
    /// A pattern marks the inside of closed shapes
    Fill,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum FillPattern {
    /// Rings stepping in from the outline
    #[default]
    Offset,
    /// Lines along X
    Hatch,
    /// Lines along X and along Y
    CrossHatch,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
pub struct Params {
    pub name: String,
    /// Below the material surface (or below the nose cone)
    #[schemars(extend("x-unit" = "mm"), range(min = 0))]
    pub depth: f64,
    pub strategy: Strategy,
    pub fill_pattern: FillPattern,
    /// Between fill lines (set here, not from the tool)
    #[schemars(extend("x-unit" = "mm"), range(min = 0))]
    pub stepover: f64,
    /// Times the whole pattern is run
    #[schemars(range(min = 1))]
    pub passes: u32,
    pub tool: Tool,
}

impl Default for Params {
    fn default() -> Self {
        Self {
            name: "Quick Engrave".into(),
            depth: 0.3,
            strategy: Strategy::Outline,
            fill_pattern: FillPattern::Offset,
            stepover: 0.5,
            passes: 1,
            tool: Tool {
                name: "Engraver (0.5 mm)".into(),
                diameter: 0.5,
                ..Tool::default()
            },
        }
    }
}

/// Lines across `regions` `step` apart, at each of `angles`.
fn hatch(regions: &[Path], step: f64, angles: &[f64], z: f64, safe: f64) -> Vec<Move> {
    angles
        .iter()
        .flat_map(|&angle| hatch_lines(regions, step, angle))
        .flatten()
        .flat_map(|(a, b)| contour_moves(&[a, b], false, &[z], safe))
        .collect()
}

impl Command for QuickEngraveToolpath {
    const ID: &'static str = "quick_engrave_toolpath";
    type Params = Params;

    fn meta() -> Meta {
        Meta {
            label: "Quick Engrave",
            group: "Toolpath Operations",
            icon: "toolpaths/quick_engrave.png",
            shortcut: "Ctrl+Q",
        }
    }

    fn run(doc: &Document, p: &Params) -> Result<Change, CommandError> {
        ensure_positive("depth", p.depth)?;
        ensure_positive("tool diameter", p.tool.diameter)?;
        if p.passes == 0 {
            return Err(CommandError::InvalidParams(
                "it needs at least 1 pass".into(),
            ));
        }
        let fill = p.strategy == Strategy::Fill;
        if fill {
            ensure_positive("stepover", p.stepover)?;
        }
        let (ids, paths) = doc.selected_paths(|path| path.closed || !fill);
        if ids.is_empty() {
            return Err(CommandError::Failed(if fill {
                "select the closed vectors to fill".into()
            } else {
                "select the vectors to engrave".into()
            }));
        }
        let (z, safe) = (-p.depth, doc.job.clearance);
        let outline = |paths: &[Path]| -> Vec<Move> {
            paths
                .iter()
                .flat_map(|path| contour_moves(&cut_points(path), path.closed, &[z], safe))
                .collect()
        };
        // ponytail: a fill also runs the outline, so the edges are crisp
        let mut once = outline(&paths);
        if fill {
            let regions = polygon::regions(&paths);
            once.extend(match p.fill_pattern {
                FillPattern::Offset => {
                    let mut rings = vec![];
                    let mut by = p.stepover;
                    loop {
                        let ring = polygon::offset_regions(&regions, -by, Corners::Round);
                        if ring.is_empty() {
                            break;
                        }
                        rings.extend(ring);
                        by += p.stepover;
                    }
                    outline(&rings)
                }
                FillPattern::Hatch => hatch(&regions, p.stepover, &[0.0], z, safe),
                FillPattern::CrossHatch => hatch(&regions, p.stepover, &[0.0, 90.0], z, safe),
            });
        }
        let moves = (0..p.passes).flat_map(|_| once.iter().copied()).collect();
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
    use cut_craft_core::geometry::Point;
    use cut_craft_core::testing::{made_toolpath, square};
    use cut_craft_core::toolpath::{MoveKind, Toolpath};

    cut_craft_core::contract_tests!(QuickEngraveToolpath);

    fn engrave(paths: Vec<Path>, p: &Params) -> Toolpath {
        made_toolpath::<QuickEngraveToolpath>(paths, p).unwrap()
    }

    fn fill(pattern: FillPattern) -> Params {
        Params {
            strategy: Strategy::Fill,
            fill_pattern: pattern,
            stepover: 1.0,
            ..Params::default()
        }
    }

    fn plunges(t: &Toolpath) -> usize {
        t.moves
            .iter()
            .filter(|m| m.kind == MoveKind::Plunge)
            .count()
    }

    #[test]
    fn outline_runs_on_open_and_closed_lines_at_the_depth() {
        let line = Path::polyline(&[Point::new(0.0, 0.0), Point::new(9.0, 0.0)], false);
        let t = engrave(vec![square(0.0, 0.0, 10.0), line], &Params::default());
        assert_eq!(plunges(&t), 2);
        assert!(
            t.moves
                .iter()
                .filter(|m| m.kind != MoveKind::Rapid)
                .all(|m| (m.to.z + 0.3).abs() < 1e-12)
        );
    }

    #[test]
    fn passes_repeat_the_whole_pattern() {
        let p = Params {
            passes: 3,
            ..Params::default()
        };
        assert_eq!(plunges(&engrave(vec![square(0.0, 0.0, 10.0)], &p)), 3);
    }

    #[test]
    fn hatch_and_cross_hatch_fill_with_lines() {
        let one = |pattern| plunges(&engrave(vec![square(0.0, 0.0, 10.0)], &fill(pattern)));
        assert_eq!(one(FillPattern::Hatch), 1 + 10, "the outline and 10 lines");
        assert_eq!(one(FillPattern::CrossHatch), 1 + 20);
        assert_eq!(one(FillPattern::Offset), 1 + 4, "rings 1 to 4 mm in");
    }

    #[test]
    fn fill_needs_closed_vectors() {
        let line = Path::polyline(&[Point::new(0.0, 0.0), Point::new(9.0, 0.0)], false);
        let why = made_toolpath::<QuickEngraveToolpath>(vec![line], &fill(FillPattern::Hatch))
            .unwrap_err();
        assert_eq!(why, "select the closed vectors to fill");
    }
}
