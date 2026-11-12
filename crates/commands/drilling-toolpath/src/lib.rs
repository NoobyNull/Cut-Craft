//! Drilling Toolpath: a hole at the centre of each selected closed vector, straight down or
//! pecked a pass depth at a time, retracting above the start or above the last peck, and
//! dwelling at the bottom of each peck if asked (the post needs a `DWELL_MOVE` block).

use cut_craft_core::document::Document;
use cut_craft_core::glam::DVec3;
use cut_craft_core::toolpath::{Move, MoveKind, Tool, made_by};
use cut_craft_core::{Change, Command, CommandError, Meta, ensure_positive};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub struct DrillingToolpath;

/// Where a peck drill rises to between pecks.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Retract {
    /// Right out of the hole, the retract gap above the start depth: clears the chips
    #[default]
    AboveStartDepth,
    /// Only the retract gap above the peck just made: quicker, but the chips may stay
    AbovePreviousPass,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
pub struct Params {
    /// What the toolpath is called in the toolpath list
    pub name: String,
    /// Below the material surface
    #[schemars(extend("x-unit" = "mm"), range(min = 0))]
    #[schemars(title = "Cut Depth", extend("x-group" = "Cutting Depths"))]
    pub depth: f64,
    /// Peck drilling goes one pass depth deeper each peck
    pub tool: Tool,
    /// Drill in pecks of the tool's pass depth, rising between them; off drills straight down
    #[serde(default = "yes")]
    #[schemars(title = "Use Peck Drilling", extend("x-group" = "Peck Drilling"))]
    pub peck: bool,
    #[serde(default)]
    #[schemars(extend("x-group" = "Peck Drilling", "x-show-when" = {"peck": "true"}))]
    pub retract: Retract,
    /// How far above the start depth, or the last peck, the drill rises between pecks
    #[serde(default = "retract_gap")]
    #[schemars(extend("x-unit" = "mm", "x-group" = "Peck Drilling"), range(min = 0))]
    pub retract_gap: f64,
    /// Seconds to wait at the bottom of each peck (0: none); the post must be able to dwell
    #[serde(default)]
    #[schemars(title = "Dwell", extend("x-unit" = "s"), range(min = 0))]
    pub dwell: f64,
    /// Drill the holes in the order the vectors were picked; else in the order they were drawn
    #[serde(default)]
    pub use_vector_selection_order: bool,
}

fn yes() -> bool {
    true
}

fn retract_gap() -> f64 {
    1.0
}

impl Default for Params {
    fn default() -> Self {
        Self {
            name: "Drilling".into(),
            depth: 10.0,
            tool: Tool::default(),
            peck: true,
            retract: Retract::AboveStartDepth,
            retract_gap: retract_gap(),
            dwell: 0.0,
            use_vector_selection_order: false,
        }
    }
}

impl Command for DrillingToolpath {
    const ID: &'static str = "drilling_toolpath";
    type Params = Params;

    fn meta() -> Meta {
        Meta {
            label: "Drilling Toolpath",
            group: "Toolpath Operations",
            icon: "toolpaths/drilling_toolpath.png",
            shortcut: "",
        }
    }

    fn run(doc: &Document, p: &Params) -> Result<Change, CommandError> {
        ensure_positive("depth", p.depth)?;
        ensure_positive("pass depth", p.tool.pass_depth)?;
        let (ids, paths) = doc.selected_paths(|path| path.closed);
        if ids.is_empty() {
            return Err(CommandError::Failed(
                "select closed vectors to drill at their centres".into(),
            ));
        }
        if p.dwell < 0.0 || p.retract_gap < 0.0 {
            return Err(CommandError::InvalidParams(
                "set the dwell and retract gap to 0 or more".into(),
            ));
        }
        let safe = doc.job.clearance;
        let depths = if p.peck {
            p.tool.pass_depths(p.depth)
        } else {
            vec![-p.depth]
        };
        // the picked order, or the order the vectors were drawn in (their ids)
        let mut order: Vec<usize> = (0..paths.len()).collect();
        if !p.use_vector_selection_order {
            order.sort_by_key(|&i| ids[i]);
        }
        let mut moves = vec![];
        for &i in &order {
            let c = paths[i].bounds().center();
            let at = |z: f64| DVec3::new(c.x, c.y, z);
            moves.push(Move {
                kind: MoveKind::Rapid,
                to: at(safe),
            });
            for (n, &z) in depths.iter().enumerate() {
                moves.push(Move {
                    kind: MoveKind::Plunge,
                    to: at(z),
                });
                let last = n + 1 == depths.len();
                let rise = match p.retract {
                    _ if last => safe,
                    Retract::AboveStartDepth => p.retract_gap,
                    Retract::AbovePreviousPass => z + p.retract_gap,
                };
                moves.push(Move {
                    kind: MoveKind::Rapid,
                    to: at(rise),
                });
            }
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
    use cut_craft_core::geometry::{Path, Point};
    use cut_craft_core::testing::made_toolpath;

    cut_craft_core::contract_tests!(DrillingToolpath);

    fn holes() -> Vec<Path> {
        vec![
            Path::circle(Point::new(10.0, 10.0), 2.0),
            Path::circle(Point::new(30.0, 10.0), 2.0),
        ]
    }

    #[test]
    fn pecks_down_at_each_centre() {
        let tp = made_toolpath::<DrillingToolpath>(holes(), &Params::default()).unwrap();
        let plunges: Vec<DVec3> = tp
            .moves
            .iter()
            .filter(|m| m.kind == MoveKind::Plunge)
            .map(|m| m.to)
            .collect();
        // 10 mm in 3 mm pass depths is 4 pecks per hole
        assert_eq!(plunges.len(), 8);
        assert!(
            plunges[..4]
                .iter()
                .all(|p| p.truncate().distance(Point::new(10.0, 10.0)) < 1e-9)
        );
        assert_eq!(plunges[3].z, -10.0);
        assert!(
            plunges[4..]
                .iter()
                .all(|p| p.truncate().distance(Point::new(30.0, 10.0)) < 1e-9)
        );
    }

    #[test]
    fn pecks_rise_above_the_start_or_the_last_peck_and_a_plain_drill_goes_straight_down() {
        let rises = |p: &Params| -> Vec<f64> {
            let tp = made_toolpath::<DrillingToolpath>(holes(), p).unwrap();
            tp.moves
                .windows(2)
                .filter(|w| w[0].kind == MoveKind::Plunge && w[1].kind == MoveKind::Rapid)
                .map(|w| w[1].to.z)
                .take(4)
                .collect()
        };
        let safe = cut_craft_core::document::Job::default().clearance;
        assert_eq!(rises(&Params::default()), [1.0, 1.0, 1.0, safe]);
        let previous = Params {
            retract: Retract::AbovePreviousPass,
            ..Params::default()
        };
        // 10 mm in four even pecks of 2.5 mm, each rising 1 mm
        assert_eq!(rises(&previous), [-1.5, -4.0, -6.5, safe]);
        let straight = Params {
            peck: false,
            ..Params::default()
        };
        let tp = made_toolpath::<DrillingToolpath>(holes(), &straight).unwrap();
        let plunges: Vec<f64> = tp
            .moves
            .iter()
            .filter(|m| m.kind == MoveKind::Plunge)
            .map(|m| m.to.z)
            .collect();
        assert_eq!(plunges, [-10.0, -10.0], "one plunge a hole");
    }

    #[test]
    fn holes_are_drilled_in_the_order_picked_when_asked() {
        use cut_craft_core::testing::document_with;
        let mut doc = document_with(holes());
        // picked right hole first
        let ids: Vec<_> = doc.selection.iter().copied().collect();
        doc.selection = cut_craft_core::selection::Selection::default();
        doc.selection.insert(ids[1]);
        doc.selection.insert(ids[0]);
        let first_x = |p: &Params| {
            let change = DrillingToolpath::run(&doc, p).unwrap();
            let mut d = doc.clone();
            d.apply(&change).unwrap();
            d.toolpaths.values().next().unwrap().moves[0].to.x
        };
        assert_eq!(first_x(&Params::default()), 10.0, "drawn order");
        let picked = Params {
            use_vector_selection_order: true,
            ..Params::default()
        };
        assert_eq!(first_x(&picked), 30.0, "picked order");
    }

    #[test]
    fn finishes_clear_of_the_material() {
        let tp = made_toolpath::<DrillingToolpath>(holes(), &Params::default()).unwrap();
        assert!(tp.moves.last().unwrap().to.z > 0.0);
    }

    #[test]
    fn needs_something_to_drill() {
        let err = made_toolpath::<DrillingToolpath>(
            vec![Path::polyline(&[Point::ZERO, Point::X], false)],
            &Params::default(),
        )
        .unwrap_err();
        assert_eq!(err, "Select closed vectors to drill at their centres.");
    }
}
