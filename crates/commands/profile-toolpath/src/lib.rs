//! Profile Toolpath: cuts along the selected vectors, outside, inside or on the line.

use cut_craft_core::document::Document;
use cut_craft_core::entry::{
    Direction, Entry, Lead, Leads, Ramp, Ramping, TabSettings, TabSpacing, Tabs, path_moves,
};
use cut_craft_core::geometry::{Point, polyline_distance, winding};
use cut_craft_core::polygon::{self, Corners};
use cut_craft_core::toolpath::{Tool, cut_points, made_by, pass_depths};
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
    /// Which way round closed shapes are cut
    #[serde(default)]
    pub direction: Direction,
    /// Positive leaves the part bigger (or the hole smaller), negative cuts into it
    #[serde(default)]
    #[schemars(title = "Allowance Offset", extend("x-unit" = "mm"))]
    pub allowance: f64,
    /// How the tool goes down into each pass (ramps run at the plunge rate)
    #[serde(default)]
    pub ramp: Ramp,
    #[serde(default)]
    #[schemars(
        title = "Ramps",
        extend("x-section" = true, "x-show-when" = {"ramp": ["smooth", "zig_zag"]})
    )]
    pub ramping: Ramping,
    /// How the tool comes onto each path, from the waste side; a ramp then runs along the
    /// lead in, and there is no spiral
    #[serde(default)]
    pub lead: Lead,
    #[serde(default)]
    #[schemars(extend("x-section" = true, "x-hide-when" = {"lead": "none"}))]
    pub leads: Leads,
    /// Bridges left to hold the part in place
    #[serde(default)]
    pub tabs: Tabs,
    #[serde(default)]
    #[schemars(title = "Tab Size", extend("x-section" = true, "x-hide-when" = {"tabs": "none"}))]
    pub tab: TabSettings,
    /// Tabs placed by clicking in the 2D view, on the vectors
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[schemars(skip)]
    pub tab_points: Vec<Point>,
}

impl Default for Params {
    fn default() -> Self {
        Self {
            name: "Profile".into(),
            depth: 6.0,
            side: Side::Outside,
            tool: Tool::default(),
            direction: Direction::Climb,
            allowance: 0.0,
            ramp: Ramp::None,
            ramping: Ramping::default(),
            lead: Lead::None,
            leads: Leads::default(),
            tabs: Tabs::None,
            tab: TabSettings::default(),
            tab_points: vec![],
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
        let radius = p.tool.diameter / 2.0 + p.allowance;
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
        let entry = Entry {
            ramp: p.ramp,
            ramping: p.ramping,
            lead: p.lead,
            leads: p.leads,
            side: 1.0,
            tabs: p.tabs,
            tab: p.tab,
            tab_top: -p.depth + p.tab.thickness,
            tool_diameter: p.tool.diameter,
            placed: vec![],
        };
        let lines: Vec<(Vec<Point>, bool)> = centre_line
            .iter()
            .map(|path| {
                let points = cut_points(path);
                // a hole's centre line runs round inside it, a part's round outside
                let points = if path.closed && p.side != Side::On {
                    p.direction.orient(points, p.side == Side::Inside)
                } else {
                    points
                };
                (points, path.closed)
            })
            .collect();
        // each clicked tab goes on the path nearest it
        let nearest = |at: Point| {
            (0..lines.len()).min_by(|&a, &b| {
                let gap = |i: usize| polyline_distance(&lines[i].0, at);
                gap(a).total_cmp(&gap(b))
            })
        };
        let owners: Vec<Option<usize>> = p.tab_points.iter().map(|&t| nearest(t)).collect();
        if p.tabs != Tabs::None && p.tab.spacing == TabSpacing::Placed && p.tab_points.is_empty() {
            return Err(CommandError::Failed(
                "click on the vectors in the 2D view to place the tabs".into(),
            ));
        }
        let moves = lines
            .iter()
            .enumerate()
            .flat_map(|(i, (points, closed))| {
                // leads come from the waste: outside a part's outline, inside a hole; an
                // anticlockwise path has its outside on the right
                let anticlockwise = *closed && winding(points) > 0.0;
                let outside = if anticlockwise { -1.0 } else { 1.0 };
                let side = match p.side {
                    Side::Inside => -outside,
                    Side::Outside | Side::On if *closed => outside,
                    _ => 1.0,
                };
                let placed = p
                    .tab_points
                    .iter()
                    .zip(&owners)
                    .filter(|&(_, &owner)| owner == Some(i))
                    .map(|(&t, _)| t)
                    .collect();
                path_moves(
                    points,
                    *closed,
                    &depths,
                    0.0,
                    doc.job.clearance,
                    &Entry {
                        side,
                        placed,
                        ..entry.clone()
                    },
                )
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

    #[test]
    fn leads_come_from_outside_for_an_outside_profile_and_inside_for_an_inside_one() {
        for (side, from_outside) in [(Side::Outside, true), (Side::Inside, false)] {
            let tp = made_toolpath::<ProfileToolpath>(
                vec![square(0.0, 0.0, 20.0)],
                &Params {
                    side,
                    lead: Lead::Straight,
                    leads: Leads {
                        angle: 90.0,
                        length: 2.0,
                        ..Leads::default()
                    },
                    ..Params::default()
                },
            )
            .unwrap();
            let plunge = tp
                .moves
                .iter()
                .find(|m| m.kind == MoveKind::Plunge)
                .unwrap();
            let p = plunge.to.truncate();
            let inside_square = p.x > 0.0 && p.x < 20.0 && p.y > 0.0 && p.y < 20.0;
            let off_line = !cut_bounds(&tp).contains(p) || inside_square;
            assert!(off_line, "{side:?}: {p}");
            assert_eq!(!inside_square, from_outside, "{side:?}: {p}");
        }
    }

    #[test]
    fn tabs_are_left_at_the_bottom_of_the_cut() {
        let tp = made_toolpath::<ProfileToolpath>(
            vec![square(0.0, 0.0, 40.0)],
            &Params {
                tabs: Tabs::Flat,
                ramp: Ramp::Smooth,
                ..Params::default()
            },
        )
        .unwrap();
        let tab_top = -6.0 + TabSettings::default().thickness;
        assert!(
            tp.moves
                .iter()
                .any(|m| m.kind == MoveKind::Cut && (m.to.z - tab_top).abs() < 1e-9)
        );
        assert!(tp.moves.iter().any(|m| (m.to.z + 6.0).abs() < 1e-9));
    }

    /// The way round the first closed pass at full depth runs: true anticlockwise.
    fn anticlockwise(tp: &Toolpath) -> bool {
        let cuts: Vec<Point> = tp
            .moves
            .iter()
            .filter(|m| m.kind == MoveKind::Cut)
            .map(|m| m.to.truncate())
            .collect();
        winding(&cuts) > 0.0
    }

    #[test]
    fn climb_runs_clockwise_round_a_part_and_anticlockwise_round_a_hole() {
        for (side, direction, expect) in [
            (Side::Outside, Direction::Climb, false),
            (Side::Outside, Direction::Conventional, true),
            (Side::Inside, Direction::Climb, true),
            (Side::Inside, Direction::Conventional, false),
        ] {
            let tp = made_toolpath::<ProfileToolpath>(
                vec![square(0.0, 0.0, 10.0)],
                &Params {
                    side,
                    direction,
                    ..Params::default()
                },
            )
            .unwrap();
            assert_eq!(anticlockwise(&tp), expect, "{side:?} {direction:?}");
        }
    }

    #[test]
    fn the_allowance_leaves_the_part_bigger() {
        let tp = made_toolpath::<ProfileToolpath>(
            vec![square(0.0, 0.0, 10.0)],
            &Params {
                allowance: 1.0,
                ..Params::default()
            },
        )
        .unwrap();
        let b = cut_bounds(&tp);
        assert!(near(b, (-4.0, -4.0), (14.0, 14.0)), "{b:?}");
    }

    #[test]
    fn clicked_tabs_sit_where_clicked_on_the_path() {
        let tp = made_toolpath::<ProfileToolpath>(
            vec![square(0.0, 0.0, 40.0)],
            &Params {
                tabs: Tabs::Flat,
                tab: TabSettings {
                    spacing: TabSpacing::Placed,
                    thickness: 2.0,
                    ..TabSettings::default()
                },
                // on the vector's bottom edge, 10 mm in
                tab_points: vec![Point::new(10.0, 0.0)],
                ..Params::default()
            },
        )
        .unwrap();
        let top = -6.0 + 2.0;
        let lifted: Vec<Point> = tp
            .moves
            .iter()
            .filter(|m| (m.to.z - top).abs() < 1e-9)
            .map(|m| m.to.truncate())
            .collect();
        // one tab, lifted over its length and the tool, round x = 10 on the line 3 mm out
        assert!(lifted.len() > 1, "lifted over the tab");
        assert!(
            lifted
                .iter()
                .all(|p| (p.y + 3.0).abs() < 1e-6 && (p.x - 10.0).abs() <= 8.0 + 1e-6),
            "{lifted:?}"
        );
        let none = made_toolpath::<ProfileToolpath>(
            vec![square(0.0, 0.0, 40.0)],
            &Params {
                tabs: Tabs::Flat,
                tab: TabSettings {
                    spacing: TabSpacing::Placed,
                    ..TabSettings::default()
                },
                ..Params::default()
            },
        );
        assert!(none.unwrap_err().contains("click"));
    }
}
