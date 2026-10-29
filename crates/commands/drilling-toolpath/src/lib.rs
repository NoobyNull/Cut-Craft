//! Drilling Toolpath: pecks a hole at the centre of each selected closed vector.

use cut_craft_core::document::Document;
use cut_craft_core::glam::DVec3;
use cut_craft_core::toolpath::{Move, MoveKind, Tool, made_by, pass_depths};
use cut_craft_core::{Change, Command, CommandError, Meta, ensure_positive};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub struct DrillingToolpath;

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
pub struct Params {
    pub name: String,
    /// Below the material surface
    #[schemars(extend("x-unit" = "mm"), range(min = 0))]
    #[schemars(title = "Cut Depth", extend("x-group" = "Cutting Depths"))]
    pub depth: f64,
    /// Each peck goes one pass depth deeper, retracting in between to clear chips
    pub tool: Tool,
}

impl Default for Params {
    fn default() -> Self {
        Self {
            name: "Drilling".into(),
            depth: 10.0,
            tool: Tool::default(),
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
        let safe = doc.job.clearance;
        let depths = pass_depths(p.depth, p.tool.pass_depth);
        let mut moves = vec![];
        for path in &paths {
            let c = path.bounds().center();
            moves.push(Move {
                kind: MoveKind::Rapid,
                to: DVec3::new(c.x, c.y, safe),
            });
            for &z in &depths {
                moves.push(Move {
                    kind: MoveKind::Plunge,
                    to: DVec3::new(c.x, c.y, z),
                });
                moves.push(Move {
                    kind: MoveKind::Rapid,
                    to: DVec3::new(c.x, c.y, safe),
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
        assert_eq!(err, "select closed vectors to drill at their centres");
    }
}
