//! Pocket Toolpath: clears the inside of the selected closed vectors with rings that step
//! in from the outline, working from the middle outwards at each depth.

use cut_craft_core::document::Document;
use cut_craft_core::polygon::{self, Corners};
use cut_craft_core::toolpath::{Tool, contour_moves, cut_points, made_by, pass_depths};
use cut_craft_core::{Change, Command, CommandError, Meta, ensure_positive};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub struct PocketToolpath;

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
pub struct Params {
    pub name: String,
    /// Below the material surface
    #[schemars(extend("x-unit" = "mm"), range(min = 0))]
    pub depth: f64,
    pub tool: Tool,
}

impl Default for Params {
    fn default() -> Self {
        Self {
            name: "Pocket".into(),
            depth: 3.0,
            tool: Tool::default(),
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
        let mut rings = vec![];
        while !ring.is_empty() {
            let next = polygon::offset(&ring, -step, Corners::Round);
            rings.extend(ring);
            ring = next;
        }
        rings.reverse(); // from the middle outwards
        let safe = doc.job.clearance;
        // ponytail: each ring retracts between passes; link rings with a stepover move to save time
        let moves = pass_depths(p.depth, p.tool.pass_depth)
            .into_iter()
            .flat_map(|z| {
                rings
                    .iter()
                    .flat_map(move |r| contour_moves(&cut_points(r), true, &[z], safe))
            })
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
}
