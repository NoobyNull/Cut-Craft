//! Chamfer Toolpath: a sloped edge along the selected shapes, inside or outside them and
//! sloping down or up away from them. A V-bit cuts the slope with its side; a ball nose
//! approximates it with a series of fine cuts.

use cut_craft_core::document::{Document, Units};
use cut_craft_core::entry::{Ways, contour_ways};
use cut_craft_core::geometry::{Path, winding};
use cut_craft_core::polygon::{self, Corners};
use cut_craft_core::toolpath::{Move, Tool, ToolDatabase, ToolShape, cut_points, made_by};
use cut_craft_core::{Change, Command, CommandError, Meta, ensure_positive};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub struct ChamferToolpath;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Side {
    /// The chamfer lies outside the shapes
    #[default]
    Outside,
    /// The chamfer lies inside the shapes
    Inside,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Slope {
    /// Going down away from the vector
    #[default]
    Down,
    /// Going up away from the vector
    Up,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum CutDirection {
    /// Clockwise round the shapes
    #[default]
    Climb,
    /// Anticlockwise round the shapes
    Conventional,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
pub struct Params {
    /// What the toolpath is called in the toolpath list
    pub name: String,
    /// Below the material surface where the top of the chamfer is
    #[schemars(extend("x-unit" = "mm"), range(min = 0))]
    #[schemars(extend("x-group" = "Cutting Depths"))]
    #[schemars(extend("x-dim" = "D", "x-diagram" = "diagrams/depth_chamfer.png"))]
    pub start_depth: f64,
    /// Across the chamfer. With a V-bit the chamfer must stay within its V: start depth plus
    /// chamfer depth (from the top surface) no deeper than the V is tall
    #[schemars(extend("x-unit" = "mm"), range(min = 0))]
    #[schemars(extend("x-group" = "Chamfer"))]
    #[schemars(extend("x-dim" = "W", "x-diagram" = "diagrams/chamfer_shape.png"))]
    pub width: f64,
    /// Slope from vertical for a ball nose (a V-bit's is half its angle)
    #[schemars(extend("x-unit" = "°"), range(min = 0, max = 90))]
    #[schemars(extend("x-group" = "Chamfer"))]
    #[schemars(extend("x-dim" = "A"))]
    pub angle: f64,
    /// Moves a V-bit's point off the chamfer so its side cuts (makes it cut deeper)
    #[schemars(extend("x-unit" = "mm"), range(min = 0))]
    #[schemars(extend("x-group" = "Chamfer"))]
    #[schemars(extend("x-dim" = "O"))]
    pub overcut: f64,
    /// Which side of the vectors the chamfer goes
    #[schemars(extend("x-group" = "Chamfer"))]
    pub side: Side,
    /// Which way the chamfer slopes from the vector
    #[schemars(extend("x-group" = "Chamfer"))]
    pub slope: Slope,
    #[schemars(extend(
        "x-group" = "Chamfer",
        "x-widget" = "named-icons",
        "x-icon" = "toolpaths/cut_{}_outside.png"
    ))]
    /// Which way round the shapes the tool travels
    pub cut_direction: CutDirection,
    /// Every pass the same way, or each back the way the last came
    #[serde(default)]
    #[schemars(
        title = "Passes",
        extend(
            "x-group" = "Chamfer",
            "x-widget" = "named-icons",
            "x-icon" = "toolpaths/passes_{}.png"
        )
    )]
    pub ways: Ways,
    /// A V-bit or a ball nose
    pub tool: Tool,
}

impl Default for Params {
    fn default() -> Self {
        Self {
            name: "Chamfer".into(),
            start_depth: 0.0,
            width: 3.0,
            angle: 45.0,
            overcut: 0.0,
            side: Side::Outside,
            slope: Slope::Down,
            cut_direction: CutDirection::Climb,
            ways: Ways::One,
            tool: ToolDatabase::starter()
                .tools
                .into_iter()
                .find(|t| t.shape == ToolShape::VBit)
                .expect("the starter tools include a V-bit"),
        }
    }
}

impl Params {
    /// The chamfer's slope from vertical, in radians.
    fn slope_angle(&self) -> f64 {
        match self.tool.shape {
            ToolShape::VBit => self.tool.angle.to_radians() / 2.0,
            _ => self.angle.to_radians(),
        }
    }

    /// Height of the chamfer.
    pub fn cut_depth(&self) -> f64 {
        self.width / self.slope_angle().tan()
    }

    /// How far below the material's top surface the chamfer goes: the start depth, the
    /// chamfer's height, and for a V-bit the overcut, which moves its point lower.
    pub fn deepest(&self) -> f64 {
        let over = if self.tool.shape == ToolShape::VBit {
            self.overcut / self.slope_angle().tan()
        } else {
            0.0
        };
        self.start_depth + self.cut_depth() + over
    }

    /// Why a V-bit can't cut this chamfer, with how to fix it: deeper than its V (measured
    /// from the top surface), the straight shank above the V would cut a wall, not a slope.
    /// `None` when it fits, or the tool is not a V-bit.
    fn beyond_the_v(&self, units: Units) -> Option<String> {
        if self.tool.shape != ToolShape::VBit {
            return None;
        }
        let v = self.tool.cutting_length()?;
        let deepest = self.deepest();
        if deepest <= v + 1e-6 {
            return None;
        }
        let tool = &self.tool.name;
        let why = format!(
            "This chamfer cuts {} deep from the top surface, but the V of {tool} is only {} \
             deep: past that its straight shank cuts a wall, not a slope.",
            units.length(deepest, 1),
            units.length(v, 2)
        );
        // the widest chamfer that fits what the start depth and overcut leave of the V
        let room = v - (deepest - self.cut_depth());
        let fix = if room * self.slope_angle().tan() > 1e-3 {
            let mut fix = format!(
                "Set the Width to {} or less, or pick a wider V-bit",
                floored(units, room * self.slope_angle().tan())
            );
            if self.start_depth > 0.0 || self.overcut > 0.0 {
                fix.push_str(", or lower the Start Depth or Overcut");
            }
            fix
        } else {
            format!(
                "The Start Depth and Overcut alone use up the whole V: lower them so together \
                 they stay under {}, or pick a wider V-bit",
                floored(units, v)
            )
        };
        Some(format!("{why} {fix}."))
    }

    /// The tool tip's runs, each (how far out from the vector towards the chamfer side, z),
    /// shallowest first.
    fn tips(&self) -> Vec<(f64, f64)> {
        let (width, height, start) = (self.width, self.cut_depth(), self.start_depth);
        let down = self.slope == Slope::Down;
        if self.tool.shape == ToolShape::VBit {
            // the cone's side lies on the slope: its point at the bottom end, moved away
            // from the slope (and down along it) by the overcut
            let over = self.overcut;
            let out = if down { width + over } else { -over };
            return vec![(out, -(start + height + over / self.slope_angle().tan()))];
        }
        // a ball touching the slope at evenly spaced points down it
        let radius = self.tool.diameter / 2.0;
        let normal = {
            let (nh, nz) = if down {
                (height, width)
            } else {
                (-height, width)
            };
            let len = nh.hypot(nz);
            (nh / len, nz / len)
        };
        let step = (self.tool.finishing_stepover() * self.tool.diameter).max(1e-3);
        #[expect(
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            reason = "a count"
        )]
        let count = (width.hypot(height) / step).ceil().max(1.0) as u32;
        let mut tips: Vec<(f64, f64)> = (0..=count)
            .map(|i| {
                let along = f64::from(i) / f64::from(count);
                let out = width * along;
                let fall = if down { along } else { 1.0 - along };
                let z = -(start + height * fall);
                (out + radius * normal.0, z + radius * normal.1 - radius)
            })
            .collect();
        tips.sort_by(|a, b| b.1.total_cmp(&a.1));
        tips
    }
}

/// `mm` as `units` shows a length, rounded down so a value typed from it still fits.
fn floored(units: Units, mm: f64) -> String {
    let step = match units {
        Units::Mm => 0.01,
        Units::Inch => 0.002_54,
    };
    units.length((mm / step + 1e-6).floor() * step, 2)
}

impl Command for ChamferToolpath {
    const ID: &'static str = "chamfer_toolpath";
    type Params = Params;

    fn meta() -> Meta {
        Meta {
            label: "Chamfer Toolpath",
            group: "Toolpath Operations",
            icon: "toolpaths/chamfer_toolpath.png",
            shortcut: "",
        }
    }

    fn run(doc: &Document, p: &Params) -> Result<Change, CommandError> {
        if !matches!(p.tool.shape, ToolShape::VBit | ToolShape::BallNose) {
            return Err(CommandError::InvalidParams(
                "pick a V-bit or a ball nose: a chamfer needs a sloped tip".into(),
            ));
        }
        ensure_positive("width", p.width)?;
        ensure_positive("tool diameter", p.tool.diameter)?;
        ensure_positive("pass depth", p.tool.pass_depth)?;
        if !(p.slope_angle() > 0.0 && p.slope_angle() < std::f64::consts::FRAC_PI_2) {
            return Err(CommandError::InvalidParams(
                "set the angle between 0° and 90°".into(),
            ));
        }
        if p.start_depth < 0.0 || p.overcut < 0.0 {
            return Err(CommandError::InvalidParams(
                "set start depth and overcut to 0 or more".into(),
            ));
        }
        if let Some(why) = p.beyond_the_v(doc.job.units) {
            return Err(CommandError::InvalidParams(why));
        }
        let (ids, paths) = doc.selected_paths(|path| path.closed);
        if ids.is_empty() {
            return Err(CommandError::Failed(
                "select the closed vectors to chamfer".into(),
            ));
        }
        let outward = if p.side == Side::Outside { 1.0 } else { -1.0 };
        let safe = doc.job.clearance;
        let v_bit = p.tool.shape == ToolShape::VBit;
        let mut moves: Vec<Move> = vec![];
        for (out, z) in p.tips() {
            let rings: Vec<Path> = if out.abs() < 1e-12 {
                paths.clone()
            } else {
                polygon::offset(&paths, outward * out, Corners::Round)
            };
            // a V-bit steps down in passes; a ball nose's fine cuts are each shallow
            let depths = if v_bit {
                p.tool.pass_depths(-z)
            } else {
                vec![z]
            };
            for ring in &rings {
                let mut points = cut_points(ring);
                if (winding(&points) < 0.0) != (p.cut_direction == CutDirection::Climb) {
                    points.reverse();
                }
                moves.extend(contour_ways(p.ways, &points, true, &depths, safe));
            }
        }
        if moves.is_empty() {
            return Err(CommandError::Failed(
                "The chamfer does not fit inside the selected shapes. Make the width smaller, or \
                 chamfer Outside instead."
                    .into(),
            ));
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
    use cut_craft_core::document::Job;
    use cut_craft_core::geometry::Point;
    use cut_craft_core::simulation::Material;
    use cut_craft_core::testing::{made_toolpath, square};
    use cut_craft_core::toolpath::Toolpath;

    cut_craft_core::contract_tests!(ChamferToolpath);

    fn chamfer(p: &Params) -> Toolpath {
        made_toolpath::<ChamferToolpath>(vec![square(20.0, 20.0, 40.0)], p).unwrap()
    }

    /// Height of the cut surface at x along the middle of the square (y = 40).
    fn surface(t: &Toolpath) -> impl Fn(f64) -> f64 + use<> {
        let job = Job {
            width: 80.0,
            height: 80.0,
            ..Job::default()
        };
        let mut m = Material::new(&job, 640);
        m.cut(t);
        move |x| m.at(Point::new(x, 40.0)).unwrap()
    }

    #[test]
    fn outside_down_slopes_from_the_edge_away_from_the_shape() {
        let z = surface(&chamfer(&Params::default())); // 90° V-bit: 3 wide, 3 deep
        assert!(
            z(20.5).abs() < 0.1,
            "the shape's edge stays at the surface: {}",
            z(20.5)
        );
        assert!(
            (z(18.5) + 1.5).abs() < 0.2,
            "half way down the slope: {}",
            z(18.5)
        );
        assert!(
            (z(17.0) + 3.0).abs() < 0.2,
            "the bottom of the slope: {}",
            z(17.0)
        );
    }

    #[test]
    fn inside_up_slopes_up_into_the_shape_from_the_edge() {
        let z = surface(&chamfer(&Params {
            side: Side::Inside,
            slope: Slope::Up,
            ..Params::default()
        }));
        assert!(
            (z(20.0) + 3.0).abs() < 0.2,
            "deepest at the edge: {}",
            z(20.0)
        );
        assert!((z(21.5) + 1.5).abs() < 0.2, "{}", z(21.5));
        assert!(
            z(23.5).abs() < 0.1,
            "back at the surface past the width: {}",
            z(23.5)
        );
    }

    #[test]
    fn overcut_keeps_the_slope_and_cuts_deeper() {
        let p = Params {
            overcut: 1.0,
            ..Params::default()
        };
        let t = chamfer(&p);
        let deepest = t.moves.iter().map(|m| m.to.z).fold(f64::INFINITY, f64::min);
        assert!((deepest + 4.0).abs() < 1e-9, "{deepest}");
        let z = surface(&t);
        assert!((z(18.5) + 1.5).abs() < 0.2, "{}", z(18.5));
    }

    #[test]
    fn a_ball_nose_follows_the_slope_in_fine_cuts() {
        let ball = ToolDatabase::starter()
            .tools
            .into_iter()
            .find(|t| t.shape == ToolShape::BallNose)
            .unwrap();
        let p = Params {
            tool: ball,
            ..Params::default()
        };
        let z = surface(&chamfer(&p));
        assert!((z(18.5) + 1.5).abs() < 0.4, "{}", z(18.5));
        assert!(z(20.5).abs() < 0.1, "{}", z(20.5));
    }

    #[test]
    fn the_chamfer_depth_is_measured_from_the_top_surface() {
        // 90° V-bit: 3 wide is 3 deep, from 2 below the surface, 1 overcut is 1 more
        let p = Params {
            start_depth: 2.0,
            overcut: 1.0,
            ..Params::default()
        };
        assert!((p.deepest() - 6.0).abs() < 1e-9, "{}", p.deepest());
    }

    #[test]
    fn a_chamfer_deeper_than_the_v_is_refused_with_how_to_fix_it() {
        // the starter 90° V-bit is 12.7 across: its V is 6.35 deep
        let p = Params {
            width: 7.0,
            ..Params::default()
        };
        let why = made_toolpath::<ChamferToolpath>(vec![square(20.0, 20.0, 40.0)], &p).unwrap_err();
        assert!(why.contains("7.0 mm deep"), "{why}");
        assert!(why.contains("only 6.35 mm"), "the V's depth: {why}");
        assert!(
            why.contains("Width to 6.35 mm"),
            "the widest that fits: {why}"
        );
        assert!(why.contains("wider V-bit"), "{why}");
    }

    #[test]
    fn the_start_depth_and_overcut_count_towards_the_v() {
        let p = Params {
            start_depth: 4.0,
            ..Params::default()
        };
        let why = made_toolpath::<ChamferToolpath>(vec![square(20.0, 20.0, 40.0)], &p).unwrap_err();
        assert!(why.contains("Width to 2.35 mm"), "{why}");
        // so deep nothing fits: the start depth itself must come up
        let p = Params {
            start_depth: 6.0,
            overcut: 1.0,
            ..Params::default()
        };
        let why = made_toolpath::<ChamferToolpath>(vec![square(20.0, 20.0, 40.0)], &p).unwrap_err();
        assert!(why.contains("Start Depth"), "{why}");
        assert!(!why.contains("Width to"), "no width fits: {why}");
    }

    #[test]
    fn a_chamfer_using_the_whole_v_is_allowed() {
        let p = Params {
            width: 6.35,
            ..Params::default()
        };
        assert!(made_toolpath::<ChamferToolpath>(vec![square(20.0, 20.0, 40.0)], &p).is_ok());
    }

    #[test]
    fn a_ball_nose_is_not_held_to_a_v() {
        let ball = ToolDatabase::starter()
            .tools
            .into_iter()
            .find(|t| t.shape == ToolShape::BallNose)
            .unwrap();
        let p = Params {
            tool: ball,
            width: 8.0,
            ..Params::default()
        };
        assert!(made_toolpath::<ChamferToolpath>(vec![square(20.0, 20.0, 60.0)], &p).is_ok());
    }

    #[test]
    fn an_end_mill_cannot_chamfer() {
        let p = Params {
            tool: Tool::default(),
            ..Params::default()
        };
        let why = made_toolpath::<ChamferToolpath>(vec![square(0.0, 0.0, 10.0)], &p).unwrap_err();
        assert!(why.contains("V-bit or a ball nose"), "{why}");
    }
}
