//! Laser Cut / Fill: a laser cuts round the selected shapes, allowing for its kerf, or marks
//! them with a hatched fill.

use cut_craft_core::document::Document;
use cut_craft_core::geometry::Path;
use cut_craft_core::polygon::{self, Corners};
use cut_craft_core::toolpath::{
    Move, MoveKind, Tool, contour_moves, cut_points, hatch_lines, made_by,
};
use cut_craft_core::{Change, Command, CommandError, Meta, ensure_positive};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub struct LaserCutFillToolpath;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Strategy {
    /// Outside the shapes by half the kerf, so the part is the drawn size
    #[default]
    CutOutside,
    /// Inside the shapes by half the kerf, so the hole is the drawn size
    CutInside,
    /// The beam's centre on the line
    CutOn,
    /// Stripes across the shapes
    HatchFill,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
pub struct Params {
    pub name: String,
    /// Of the laser's maximum
    #[schemars(extend("x-unit" = "%"), range(min = 0, max = 100))]
    pub power: f64,
    /// Times the toolpath is run, for thicker material
    #[schemars(range(min = 1))]
    pub passes: u32,
    pub strategy: Strategy,
    /// Extra offset for cutting outside or inside, to ease or tighten a fit
    #[schemars(extend("x-unit" = "mm"))]
    pub allowance: f64,
    /// Between hatch lines
    #[schemars(extend("x-unit" = "mm"), range(min = 0))]
    pub stepover: f64,
    #[schemars(extend("x-unit" = "°"))]
    pub hatch_angle: f64,
    /// A second set of hatch lines at right angles
    pub cross_hatch: bool,
    /// Round the shapes as well, so a hatch has crisp edges
    pub outline: bool,
    /// The laser: its diameter is the kerf, its feed rate the move speed
    pub tool: Tool,
}

impl Default for Params {
    fn default() -> Self {
        Self {
            name: "Laser Cut".into(),
            power: 80.0,
            passes: 1,
            strategy: Strategy::CutOutside,
            allowance: 0.0,
            stepover: 0.2,
            hatch_angle: 0.0,
            cross_hatch: false,
            outline: true,
            tool: laser(),
        }
    }
}

/// A laser as a tool: its diameter is the kerf.
pub fn laser() -> Tool {
    Tool {
        name: "Laser (0.15 mm kerf)".into(),
        diameter: 0.15,
        feed_rate: 1500.0,
        plunge_rate: 1500.0,
        spindle_speed: 0.0,
        ..Tool::default()
    }
}

/// `moves` with the laser's `power` on every move but rapids: what a laser toolpath holds.
pub fn powered(moves: &[Move], power: f64) -> Vec<f64> {
    moves
        .iter()
        .map(|m| {
            if m.kind == MoveKind::Rapid {
                0.0
            } else {
                power
            }
        })
        .collect()
}

impl Command for LaserCutFillToolpath {
    const ID: &'static str = "laser_cut_fill_toolpath";
    type Params = Params;

    fn meta() -> Meta {
        Meta {
            label: "Laser Cut / Fill",
            group: "Toolpath Operations",
            icon: "toolpaths/laser_cut_fill.png",
            shortcut: "",
        }
    }

    fn run(doc: &Document, p: &Params) -> Result<Change, CommandError> {
        if !(p.power > 0.0 && p.power <= 100.0) {
            return Err(CommandError::InvalidParams(
                "power must be more than 0 and at most 100 %".into(),
            ));
        }
        if p.passes == 0 {
            return Err(CommandError::InvalidParams(
                "it needs at least 1 pass".into(),
            ));
        }
        ensure_positive("kerf", p.tool.diameter)?;
        let hatching = p.strategy == Strategy::HatchFill;
        if hatching {
            ensure_positive("stepover", p.stepover)?;
        }
        let (ids, paths) =
            doc.selected_paths(|path| path.closed || matches!(p.strategy, Strategy::CutOn));
        if ids.is_empty() {
            return Err(CommandError::Failed(
                if hatching || p.strategy != Strategy::CutOn {
                    "select the closed vectors to cut or fill".into()
                } else {
                    "select the vectors to cut".into()
                },
            ));
        }
        let kerf = p.tool.diameter / 2.0;
        let safe = doc.job.clearance;
        let around = |rings: &[Path]| -> Vec<Move> {
            rings
                .iter()
                .flat_map(|r| contour_moves(&cut_points(r), r.closed, &[0.0], safe))
                .collect()
        };
        let once = match p.strategy {
            Strategy::CutOutside => {
                around(&polygon::offset(&paths, kerf + p.allowance, Corners::Round))
            }
            Strategy::CutInside => around(&polygon::offset(
                &paths,
                -(kerf + p.allowance),
                Corners::Round,
            )),
            Strategy::CutOn => around(&paths),
            Strategy::HatchFill => {
                let regions = polygon::regions(&paths);
                let mut angles = vec![p.hatch_angle];
                if p.cross_hatch {
                    angles.push(p.hatch_angle + 90.0);
                }
                // ponytail: every region is filled together; filling each on its own is
                // Aspire's "Fill Regionally"
                let mut moves: Vec<Move> = angles
                    .into_iter()
                    .flat_map(|a| hatch_lines(&regions, p.stepover, a))
                    .flatten()
                    .flat_map(|(a, b)| contour_moves(&[a, b], false, &[0.0], safe))
                    .collect();
                if p.outline {
                    moves.extend(around(&paths));
                }
                moves
            }
        };
        if once.is_empty() {
            return Err(CommandError::Failed(
                "the kerf leaves nothing to cut inside the shapes".into(),
            ));
        }
        // ponytail: every pass at the surface; lowering the focus each pass is a later option
        let moves: Vec<Move> = (0..p.passes).flat_map(|_| once.iter().copied()).collect();
        let mut toolpath = made_by::<Self>(p, p.name.clone(), p.tool.clone(), ids, moves);
        toolpath.power = powered(&toolpath.moves, p.power);
        let mut c = Change::builder(doc);
        c.add_toolpath(toolpath);
        Ok(c.finish())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cut_craft_core::testing::{cut_bounds, made_toolpath, square};
    use cut_craft_core::toolpath::Toolpath;

    cut_craft_core::contract_tests!(LaserCutFillToolpath);

    fn cut(p: &Params) -> Toolpath {
        made_toolpath::<LaserCutFillToolpath>(vec![square(0.0, 0.0, 10.0)], p).unwrap()
    }

    #[test]
    fn cutting_outside_allows_for_half_the_kerf() {
        let b = cut_bounds(&cut(&Params::default()));
        assert!(
            (b.min.x + 0.075).abs() < 0.01 && (b.max.x - 10.075).abs() < 0.01,
            "{b:?}"
        );
        let inside = cut_bounds(&cut(&Params {
            strategy: Strategy::CutInside,
            allowance: 0.1,
            ..Params::default()
        }));
        assert!((inside.min.x - 0.175).abs() < 0.01, "{inside:?}");
    }

    #[test]
    fn the_laser_is_powered_on_cuts_only_and_stays_at_the_surface() {
        let t = cut(&Params::default());
        assert_eq!(t.power.len(), t.moves.len());
        for (m, &p) in t.moves.iter().zip(&t.power) {
            assert_eq!(p, if m.kind == MoveKind::Rapid { 0.0 } else { 80.0 });
            if m.kind != MoveKind::Rapid {
                assert!(m.to.z.abs() < 1e-12);
            }
        }
    }

    #[test]
    fn passes_repeat_it() {
        let one = cut(&Params::default()).moves.len();
        assert_eq!(
            cut(&Params {
                passes: 3,
                ..Params::default()
            })
            .moves
            .len(),
            3 * one
        );
    }

    #[test]
    fn hatch_fill_stripes_the_shape_and_outlines_it() {
        let p = Params {
            strategy: Strategy::HatchFill,
            stepover: 1.0,
            ..Params::default()
        };
        let plunges = |t: &Toolpath| {
            t.moves
                .iter()
                .filter(|m| m.kind == MoveKind::Plunge)
                .count()
        };
        assert_eq!(plunges(&cut(&p)), 10 + 1);
        assert_eq!(
            plunges(&cut(&Params {
                cross_hatch: true,
                outline: false,
                ..p
            })),
            20
        );
    }
}
