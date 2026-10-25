//! Moulding Toolpath: sweeps an open cross-section profile along drive rails, cutting the
//! moulding in passes parallel to the rails, each as low as the tool can go without cutting
//! into the profile.

use cut_craft_core::document::Document;
use cut_craft_core::geometry::{Path, Point};
use cut_craft_core::glam::DVec3;
use cut_craft_core::polygon::{self, Corners};
use cut_craft_core::toolpath::{Move, Tool, ToolDatabase, ToolShape, cut_points, follow, made_by};
use cut_craft_core::{Change, Command, CommandError, Meta, ensure_positive};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub struct MouldingToolpath;

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
pub struct Params {
    pub name: String,
    /// Mitred corners where the rails have sharp corners, instead of rolling round them
    pub create_sharp_corners: bool,
    /// Carries the passes this far past both ends of the profile
    #[schemars(extend("x-unit" = "mm"), range(min = 0))]
    pub boundary_offset: f64,
    /// Usually a ball nose; its stepover spaces the passes
    pub tool: Tool,
}

impl Default for Params {
    fn default() -> Self {
        Self {
            name: "Moulding".into(),
            create_sharp_corners: false,
            boundary_offset: 0.0,
            tool: ToolDatabase::starter()
                .tools
                .into_iter()
                .find(|t| t.shape == ToolShape::BallNose)
                .expect("the starter tools include a ball nose"),
        }
    }
}

/// The cross section as (distance from where it hangs on the rail, height below its top)
/// samples, the top at 0 and lower parts negative, by distance.
fn cross_section(profile: &Path) -> Vec<(f64, f64)> {
    let points = cut_points(profile);
    let start = points[0];
    let end = points[points.len() - 1];
    let away = if end.x >= start.x { 1.0 } else { -1.0 };
    let top = points.iter().map(|p| p.y).fold(f64::NEG_INFINITY, f64::max);
    let mut samples: Vec<(f64, f64)> = points
        .iter()
        .map(|p| ((p.x - start.x) * away, p.y - top))
        .collect();
    samples.sort_by(|a, b| a.0.total_cmp(&b.0));
    samples
}

/// The profile's height at `u` across (flat beyond its ends).
fn height(section: &[(f64, f64)], u: f64) -> f64 {
    let (first, last) = (section[0], section[section.len() - 1]);
    if u <= first.0 {
        return first.1;
    }
    if u >= last.0 {
        return last.1;
    }
    let i = section.partition_point(|s| s.0 <= u);
    let (a, b) = (section[i - 1], section[i]);
    if b.0 - a.0 < 1e-12 {
        return a.1.max(b.1);
    }
    a.1 + (b.1 - a.1) * (u - a.0) / (b.0 - a.0)
}

/// How low the tool's tip goes at `h` across without cutting into the profile.
fn tip_height(section: &[(f64, f64)], tool: &Tool, h: f64) -> f64 {
    let radius = tool.diameter / 2.0;
    // ponytail: samples a fiftieth of the tool across; exact segment contact if finer is needed
    let steps = 50;
    (-steps..=steps)
        .map(|i| {
            let d = radius * f64::from(i) / f64::from(steps);
            height(section, h + d) - tool.profile(d.abs()).unwrap_or(0.0)
        })
        .fold(f64::NEG_INFINITY, f64::max)
}

/// An open polyline moved `by` to the right of its direction (mitred at the corners).
fn shift_right(points: &[Point], by: f64) -> Vec<Point> {
    let normal = |a: Point, b: Point| -(b - a).normalize_or_zero().perp();
    (0..points.len())
        .map(|i| {
            let before = (i > 0).then(|| normal(points[i - 1], points[i]));
            let after = (i + 1 < points.len()).then(|| normal(points[i], points[i + 1]));
            let n = match (before, after) {
                (Some(a), Some(b)) => {
                    let mid = (a + b).normalize_or(a);
                    mid / mid.dot(a).max(0.2)
                }
                (Some(a), None) | (None, Some(a)) => a,
                (None, None) => Point::ZERO,
            };
            points[i] + n * by
        })
        .collect()
}

impl Command for MouldingToolpath {
    const ID: &'static str = "moulding_toolpath";
    type Params = Params;

    fn meta() -> Meta {
        Meta {
            label: "Moulding Toolpath",
            group: "Toolpath Operations",
            icon: "toolpaths/moulding_toolpath.svg",
            shortcut: "",
        }
    }

    fn run(doc: &Document, p: &Params) -> Result<Change, CommandError> {
        ensure_positive("tool diameter", p.tool.diameter)?;
        if !(p.tool.stepover > 0.0 && p.tool.stepover <= 1.0) {
            return Err(CommandError::InvalidParams(
                "stepover must be more than 0 and at most 1".into(),
            ));
        }
        // the profile is the open vector picked last, as in Aspire
        let mut ids: Vec<_> = doc.selection.iter().copied().collect();
        let Some(at) = ids.iter().rposition(|id| !doc.vectors[id].closed) else {
            return Err(CommandError::Failed(
                "select the drive rails and an open cross-section profile".into(),
            ));
        };
        let profile_id = ids.remove(at);
        if ids.is_empty() {
            return Err(CommandError::Failed(
                "select a drive rail as well as the profile".into(),
            ));
        }
        let section = cross_section(&doc.vectors[&profile_id]);
        let width = section[section.len() - 1].0 - section[0].0;
        // ponytail: a profile taller than the material is cut off at its bottom; Aspire asks
        // for the material or profile to be changed
        let floor = -doc.job.thickness;
        let step = p.tool.stepover * p.tool.diameter;
        let reach = p.boundary_offset.max(0.0);
        let corners = if p.create_sharp_corners {
            Corners::Sharp
        } else {
            Corners::Round
        };
        let safe = doc.job.clearance;
        let mut moves: Vec<Move> = vec![];
        for rail in ids.iter().map(|id| &doc.vectors[id]) {
            let mut h = section[0].0 - reach;
            let mut pass = 0;
            while h <= section[0].0 + width + reach + 1e-9 {
                let z = tip_height(&section, &p.tool, h).max(floor);
                let lines: Vec<Vec<Point>> = if rail.closed {
                    // the profile hangs outside a closed rail
                    let rings = if h.abs() < 1e-9 {
                        vec![rail.clone()]
                    } else {
                        polygon::offset(std::slice::from_ref(rail), h, corners)
                    };
                    rings
                        .iter()
                        .map(|r| {
                            let mut points = cut_points(r);
                            points.push(points[0]);
                            points
                        })
                        .collect()
                } else {
                    vec![shift_right(&cut_points(rail), h)]
                };
                for mut line in lines {
                    // back and forth along an open rail
                    if !rail.closed && pass % 2 == 1 {
                        line.reverse();
                    }
                    let cut: Vec<DVec3> = line.iter().map(|q| q.extend(z)).collect();
                    // ponytail: lifts to the safe height between passes; Aspire lifts a stepover
                    moves.extend(follow(&cut, safe));
                }
                h += step;
                pass += 1;
            }
        }
        let vectors = std::iter::once(profile_id).chain(ids).collect();
        let mut c = Change::builder(doc);
        c.add_toolpath(made_by::<Self>(
            p,
            p.name.clone(),
            p.tool.clone(),
            vectors,
            moves,
        ));
        Ok(c.finish())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cut_craft_core::testing::{made_toolpath, square};
    use cut_craft_core::toolpath::{MoveKind, Toolpath};

    cut_craft_core::contract_tests!(MouldingToolpath);

    /// A 10 mm wide profile: flat at the top for 5 mm, then a 45° slope 5 mm down.
    fn chamfered_profile() -> Path {
        Path::polyline(
            &[
                Point::new(200.0, 0.0),
                Point::new(205.0, 0.0),
                Point::new(210.0, -5.0),
            ],
            false,
        )
    }

    fn moulding(rail: Path, p: &Params) -> Toolpath {
        made_toolpath::<MouldingToolpath>(vec![rail, chamfered_profile()], p).unwrap()
    }

    fn end_mill() -> Params {
        Params {
            tool: Tool {
                stepover: 0.5,
                ..Tool::default()
            },
            ..Params::default()
        }
    }

    #[test]
    fn an_open_rail_is_followed_at_the_profiles_heights_to_its_right() {
        let rail = Path::polyline(&[Point::new(0.0, 50.0), Point::new(100.0, 50.0)], false);
        let t = moulding(rail, &end_mill());
        let passes: Vec<(f64, f64)> = t
            .moves
            .iter()
            .filter(|m| m.kind == MoveKind::Plunge)
            .map(|m| (m.to.y, m.to.z))
            .collect();
        // 6 mm end mill, 3 mm apart: the right of an eastward rail is south (lower y)
        assert_eq!(passes.len(), 4, "{passes:?}");
        assert!((passes[0].0 - 50.0).abs() < 1e-9 && passes[0].1.abs() < 1e-9);
        assert!(
            (passes[1].0 - 47.0).abs() < 1e-9 && passes[1].1.abs() < 1e-9,
            "a flat end mill stays up while its edge is on the flat"
        );
        assert!(
            (passes[2].0 - 44.0).abs() < 1e-9 && passes[2].1.abs() < 1e-9,
            "{passes:?}"
        );
        assert!(
            (passes[3].1 + 1.0).abs() < 1e-6,
            "its edge first reaches the slope: {passes:?}"
        );
    }

    #[test]
    fn passes_go_back_and_forth_along_an_open_rail() {
        let rail = Path::polyline(&[Point::new(0.0, 50.0), Point::new(100.0, 50.0)], false);
        let t = moulding(rail, &end_mill());
        let starts: Vec<f64> = t
            .moves
            .iter()
            .filter(|m| m.kind == MoveKind::Plunge)
            .map(|m| m.to.x)
            .collect();
        assert_eq!(starts[..2], [0.0, 100.0]);
    }

    #[test]
    fn a_closed_rail_has_the_profile_outside_it() {
        let t = moulding(square(20.0, 20.0, 40.0), &end_mill());
        let deepest = t
            .moves
            .iter()
            .filter(|m| m.to.z < -0.5)
            .map(|m| m.to)
            .next()
            .unwrap();
        assert!(
            deepest.x < 20.0 || deepest.x > 60.0 || deepest.y < 20.0 || deepest.y > 60.0,
            "{deepest:?}"
        );
    }

    #[test]
    fn a_ball_nose_follows_down_the_slope() {
        let rail = Path::polyline(&[Point::new(0.0, 50.0), Point::new(100.0, 50.0)], false);
        let t = moulding(rail, &Params::default());
        let deepest = t.moves.iter().map(|m| m.to.z).fold(0.0, f64::min);
        assert!(deepest < -4.0, "{deepest}");
    }

    #[test]
    fn the_profile_is_the_open_vector_picked_last() {
        use cut_craft_core::document::EntityId;
        // the profile drawn first, the rail second, then picked rail first
        let rail = Path::polyline(&[Point::new(0.0, 50.0), Point::new(100.0, 50.0)], false);
        let mut doc = cut_craft_core::testing::document_with(vec![chamfered_profile(), rail]);
        doc.selection = [EntityId(2), EntityId(1)].into();
        let change = MouldingToolpath::run(&doc, &end_mill()).unwrap();
        doc.apply(&change).unwrap();
        let t = doc.toolpaths.values().next().unwrap();
        assert_eq!(t.vectors[0], EntityId(1), "the profile comes first");
    }

    #[test]
    fn the_newest_open_vector_is_the_profile_and_a_rail_is_needed() {
        let why =
            made_toolpath::<MouldingToolpath>(vec![chamfered_profile()], &end_mill()).unwrap_err();
        assert!(why.contains("drive rail"), "{why}");
    }
}
