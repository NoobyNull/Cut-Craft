//! Profile Toolpath: cuts along the selected vectors, outside, inside or on the line.

use cut_craft_core::document::Document;
use cut_craft_core::geometry::Point;
use cut_craft_core::polygon::{self, Corners};
use cut_craft_core::toolpath::{Tool, contour_moves, cut_points, made_by, pass_depths};
use cut_craft_core::{Change, Command, CommandError, Meta, ensure_positive};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub struct ProfileToolpath;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Side {
    /// Around the outside of closed shapes
    #[default]
    Outside,
    /// Around the inside of closed shapes
    Inside,
    /// Along the line itself, open vectors included
    On,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
pub struct Params {
    pub name: String,
    /// Below the material surface
    #[schemars(extend("x-unit" = "mm"), range(min = 0))]
    pub depth: f64,
    pub side: Side,
    pub tool: Tool,
}

impl Default for Params {
    fn default() -> Self {
        Self {
            name: "Profile".into(),
            depth: 6.0,
            side: Side::Outside,
            tool: Tool::default(),
        }
    }
}

impl Command for ProfileToolpath {
    const ID: &'static str = "profile_toolpath";
    type Params = Params;

    fn meta() -> Meta {
        Meta {
            label: "Profile Toolpath",
            group: "Toolpath Operations",
            icon: "toolpaths/profile_toolpath.png",
            shortcut: "",
        }
    }

    fn run(doc: &Document, p: &Params) -> Result<Change, CommandError> {
        ensure_positive("depth", p.depth)?;
        ensure_positive("tool diameter", p.tool.diameter)?;
        ensure_positive("pass depth", p.tool.pass_depth)?;
        let (ids, paths) = doc.selected_paths(|path| path.closed || p.side == Side::On);
        if ids.is_empty() {
            return Err(CommandError::Failed("select the vectors to profile".into()));
        }
        let radius = p.tool.diameter / 2.0;
        let centre_line = match p.side {
            Side::Outside => polygon::offset(&paths, radius, Corners::Round),
            Side::Inside => polygon::offset(&paths, -radius, Corners::Round),
            Side::On => paths,
        };
        if centre_line.is_empty() {
            return Err(CommandError::Failed(
                "the tool is too big for the inside of the selected shapes".into(),
            ));
        }
        let depths = pass_depths(p.depth, p.tool.pass_depth);
        let moves = centre_line
            .iter()
            .flat_map(|path| {
                let points: Vec<Point> = cut_points(path);
                contour_moves(&points, path.closed, &depths, doc.job.clearance)
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
    use cut_craft_core::geometry::Rect;
    use cut_craft_core::testing::{cut_bounds, made_toolpath, square};
    use cut_craft_core::toolpath::{MoveKind, Toolpath};

    cut_craft_core::contract_tests!(ProfileToolpath);

    fn profile(side: Side, square_side: f64) -> Result<Toolpath, String> {
        made_toolpath::<ProfileToolpath>(
            vec![square(0.0, 0.0, square_side)],
            &Params {
                side,
                ..Params::default()
            },
        )
    }

    fn near(a: Rect, min: (f64, f64), max: (f64, f64)) -> bool {
        a.min.distance(Point::new(min.0, min.1)) < 0.02
            && a.max.distance(Point::new(max.0, max.1)) < 0.02
    }

    #[test]
    fn outside_runs_a_tool_radius_outside_the_shape() {
        let b = cut_bounds(&profile(Side::Outside, 10.0).unwrap());
        assert!(near(b, (-3.0, -3.0), (13.0, 13.0)), "{b:?}");
    }

    #[test]
    fn inside_runs_a_tool_radius_inside_the_shape() {
        let b = cut_bounds(&profile(Side::Inside, 10.0).unwrap());
        assert!(near(b, (3.0, 3.0), (7.0, 7.0)), "{b:?}");
    }

    #[test]
    fn on_follows_the_line_itself() {
        let b = cut_bounds(&profile(Side::On, 10.0).unwrap());
        assert!(near(b, (0.0, 0.0), (10.0, 10.0)), "{b:?}");
    }

    #[test]
    fn cuts_in_passes_down_to_the_depth() {
        let tp = profile(Side::Outside, 10.0).unwrap();
        let plunges: Vec<f64> = tp
            .moves
            .iter()
            .filter(|m| m.kind == MoveKind::Plunge)
            .map(|m| m.to.z)
            .collect();
        assert_eq!(plunges, [-3.0, -6.0], "6 mm deep in 3 mm passes");
    }

    #[test]
    fn remembers_how_it_was_made_so_it_can_be_recalculated() {
        let tp = profile(Side::Inside, 10.0).unwrap();
        assert_eq!(tp.strategy, "profile_toolpath");
        assert_eq!(tp.params["side"], "inside");
        assert_eq!(tp.vectors.len(), 1);
    }

    #[test]
    fn a_shape_smaller_than_the_tool_cannot_be_cut_inside() {
        assert_eq!(
            profile(Side::Inside, 4.0).unwrap_err(),
            "the tool is too big for the inside of the selected shapes"
        );
    }
}
