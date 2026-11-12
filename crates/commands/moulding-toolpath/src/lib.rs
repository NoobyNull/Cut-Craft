//! Moulding Toolpath: sweeps an open cross-section profile along drive rails, cutting the
//! moulding in passes parallel to the rails, each as low as the tool can go without cutting
//! into the profile. Passes can be spaced along the profile's slope (vary stepover), and a
//! larger clearance tool can rough it first in levels, leaving an allowance, its own toolpath.

use cut_craft_core::document::Document;
use cut_craft_core::entry::Ways;
use cut_craft_core::geometry::{Path, Point, winding};
use cut_craft_core::glam::DVec3;
use cut_craft_core::polygon::{self, Corners};
use cut_craft_core::toolpath::{Move, Tool, ToolDatabase, ToolShape, cut_points, follow, made_by};
use cut_craft_core::{Change, Command, CommandError, Meta, ensure_positive};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub struct MouldingToolpath;

/// Which way the tool travels along the rails. The profile hangs outside a closed rail and on
/// the right of an open one, so on an open rail climb runs back against the rail's direction.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum CutDirection {
    /// Clockwise round a closed rail: usually the cleaner finish, the usual choice on a CNC
    #[default]
    #[schemars(title = "Climb (clockwise)")]
    Climb,
    /// Anticlockwise round a closed rail, as you would push a hand router: gentler on a
    /// light machine or a large form tool, sometimes rougher in the finish
    #[schemars(title = "Conventional (anticlockwise)")]
    Conventional,
}

/// Mouldings saved before the cut direction could be chosen went anticlockwise.
fn conventional() -> CutDirection {
    CutDirection::Conventional
}

/// Mouldings saved before then went back and forth along an open rail.
fn both_ways() -> Ways {
    Ways::Both
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[expect(clippy::struct_excessive_bools, reason = "one tick box per option")]
pub struct Params {
    pub name: String,
    /// Mitred corners where the rails have sharp corners, instead of rolling round them
    #[schemars(extend("x-group" = "Moulding"))]
    pub create_sharp_corners: bool,
    /// Carries the passes this far past both ends of the profile
    #[schemars(extend("x-unit" = "mm"), range(min = 0))]
    #[schemars(extend("x-group" = "Moulding"))]
    pub boundary_offset: f64,
    /// Usually a ball nose; its stepover spaces the passes
    pub tool: Tool,
    /// Which way the passes go round the rails: climb (clockwise) or conventional
    /// (anticlockwise), for any bit, a custom form tool too
    #[serde(default = "conventional")]
    #[schemars(extend(
        "x-group" = "Cut Direction",
        "x-widget" = "named-icons",
        "x-icon" = "toolpaths/cut_{}_outside.png"
    ))]
    pub cut_direction: CutDirection,
    /// Every pass the cut direction's way, or each pass back the way the last came (quicker
    /// along an open rail, but half the passes cut the other way)
    #[serde(default = "both_ways")]
    #[schemars(
        title = "Passes",
        extend(
            "x-group" = "Cut Direction",
            "x-widget" = "named-icons",
            "x-icon" = "toolpaths/passes_{}.png"
        )
    )]
    pub ways: Ways,
    /// Space the passes by the stepover along the profile's surface, so they come closer
    /// together where it is steep
    #[serde(default)]
    #[schemars(title = "Vary Stepover")]
    pub vary_stepover: bool,
    /// Use Larger Area Clearance Tool: the first of these roughs the moulding first, in
    /// levels of its pass depth, as a toolpath of its own named [Clear]
    #[serde(default)]
    #[schemars(title = "Larger Area Clearance Tool")]
    pub clearance_tools: Vec<Tool>,
    /// Material the clearance tool leaves on the profile for the finishing tool
    #[serde(default)]
    #[schemars(extend("x-unit" = "mm"), range(min = 0))]
    #[schemars(extend("x-group" = "Larger Area Clearance Tool"))]
    pub machining_allowance: f64,
    /// The clearance tool cuts the profile's flat parts it fits in to their finished height
    #[serde(default)]
    #[schemars(extend("x-group" = "Larger Area Clearance Tool"))]
    pub machine_flat_regions: bool,
    /// The finishing tool leaves out the flat parts the clearance tool finished
    #[serde(default)]
    #[schemars(extend("x-show-when" = {"machine_flat_regions": true}))]
    #[schemars(extend("x-group" = "Larger Area Clearance Tool"))]
    pub skip_flat_regions: bool,
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
            cut_direction: CutDirection::Climb,
            ways: Ways::One,
            vary_stepover: false,
            clearance_tools: vec![],
            machining_allowance: 0.0,
            machine_flat_regions: false,
            skip_flat_regions: false,
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
            icon: "toolpaths/moulding_toolpath.png",
            shortcut: "",
        }
    }

    fn run(doc: &Document, p: &Params) -> Result<Change, CommandError> {
        ensure_positive("tool diameter", p.tool.diameter)?;
        let stepover = p.tool.finishing_stepover();
        if !(stepover > 0.0 && stepover <= 1.0) {
            return Err(CommandError::InvalidParams(
                "the tool's finishing stepover must be more than 0% and at most 100%: set it in the Tool Database".into(),
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
        let rails: Vec<&Path> = ids.iter().map(|id| &doc.vectors[id]).collect();
        // ponytail: a profile taller than the material is cut off at its bottom; Aspire asks
        // for the material or profile to be changed
        let floor = -doc.job.thickness;
        let reach = p.boundary_offset.max(0.0);
        let span = (section[0].0 - reach, section[section.len() - 1].0 + reach);
        let sweep = Sweep {
            rails: &rails,
            corners: if p.create_sharp_corners {
                Corners::Sharp
            } else {
                Corners::Round
            },
            safe: doc.job.clearance,
            direction: p.cut_direction,
            ways: p.ways,
        };
        let vectors: Vec<_> = std::iter::once(profile_id)
            .chain(ids.iter().copied())
            .collect();
        let mut c = Change::builder(doc);
        let mut flats = vec![];
        if let Some(clearer) = p.clearance_tools.first() {
            ensure_positive("clearance tool diameter", clearer.diameter)?;
            ensure_positive("clearance tool pass depth", clearer.pass_depth)?;
            let (levels, flat) = clearance(&section, clearer, p, span, floor);
            flats = flat;
            c.add_toolpath(made_by::<Self>(
                p,
                format!("{} [Clear]", p.name),
                clearer.clone(),
                vectors.clone(),
                sweep.moves(&levels),
            ));
        }
        let finished_flat = |h: f64| {
            p.skip_flat_regions
                && p.clearance_tools.first().is_some_and(|clearer| {
                    let r = p.tool.diameter / 2.0;
                    flat(&section, h - r, h + r)
                        && flats
                            .iter()
                            .any(|&f: &f64| (f - h).abs() <= clearer.diameter / 2.0)
                })
        };
        let levels: Vec<(f64, f64)> = finish_positions(&section, p, span, floor)
            .into_iter()
            .filter(|&(h, _)| !finished_flat(h))
            .collect();
        c.add_toolpath(made_by::<Self>(
            p,
            p.name.clone(),
            p.tool.clone(),
            vectors,
            sweep.moves(&levels),
        ));
        Ok(c.finish())
    }
}

/// Where the finishing passes go: (distance across, tip height), a stepover apart across,
/// or (vary stepover) a stepover apart along the surface the tip follows.
fn finish_positions(
    section: &[(f64, f64)],
    p: &Params,
    (from, to): (f64, f64),
    floor: f64,
) -> Vec<(f64, f64)> {
    let step = p.tool.finishing_stepover() * p.tool.diameter;
    let at = |h: f64| (h, tip_height(section, &p.tool, h).max(floor));
    let mut out = vec![at(from)];
    if p.vary_stepover {
        // ponytail: walked in twentieths of a stepover
        let fine = step / 20.0;
        let (mut h, mut walked) = (from, 0.0);
        let mut last = out[0];
        while h + fine <= to + 1e-9 {
            h += fine;
            let here = at(h);
            let piece = (here.0 - last.0).hypot(here.1 - last.1);
            // a pass where going on would take it further than a stepover
            if walked > 0.0 && walked + piece > step + 1e-9 {
                out.push(last);
                walked = 0.0;
            }
            walked += piece;
            last = here;
        }
        if walked > 0.0 {
            out.push(last);
        }
    } else {
        let mut h = from + step;
        while h <= to + 1e-9 {
            out.push(at(h));
            h += step;
        }
    }
    out
}

/// Whether the profile is level from `a` to `b` across.
fn flat(section: &[(f64, f64)], a: f64, b: f64) -> bool {
    let level = height(section, a);
    (height(section, b) - level).abs() < 1e-6
        && section
            .iter()
            .filter(|s| s.0 > a && s.0 < b)
            .all(|s| (s.1 - level).abs() < 1e-6)
}

/// The clearance tool's passes, level by level: at each, wherever the material it may cut
/// reaches below the level above, it cuts down to this level or as far as it may (the
/// profile plus the allowance, or the profile itself on flats it fits, when asked). Also
/// where it finished flats.
fn clearance(
    section: &[(f64, f64)],
    clearer: &Tool,
    p: &Params,
    (from, to): (f64, f64),
    floor: f64,
) -> (Vec<(f64, f64)>, Vec<f64>) {
    let radius = clearer.diameter / 2.0;
    let step = clearer.stepover * clearer.diameter;
    let mut across = vec![];
    let mut h = from;
    while h <= to + 1e-9 {
        across.push(h);
        h += step;
    }
    let on_flat = |h: f64| p.machine_flat_regions && flat(section, h - radius, h + radius);
    let target = |h: f64| {
        let tip = tip_height(section, clearer, h);
        let allowed = if on_flat(h) {
            tip
        } else {
            tip + p.machining_allowance
        };
        allowed.max(floor)
    };
    let deepest = across.iter().map(|&h| target(h)).fold(0.0, f64::min);
    let mut levels = vec![];
    let mut above = 0.0;
    for z in clearer.pass_depths(-deepest) {
        for &h in &across {
            let lowest = target(h);
            if lowest < above - 1e-9 {
                levels.push((h, z.max(lowest)));
            }
        }
        above = z;
    }
    let flats = across.into_iter().filter(|&h| on_flat(h)).collect();
    (levels, flats)
}

/// The rails a moulding follows, and how passes along them are cut.
struct Sweep<'a> {
    rails: &'a [&'a Path],
    corners: Corners,
    safe: f64,
    direction: CutDirection,
    ways: Ways,
}

impl Sweep<'_> {
    /// A pass along every rail at each (distance across, height), in order, each going the
    /// cut direction's way (or, both ways, every other one back).
    fn moves(&self, levels: &[(f64, f64)]) -> Vec<Move> {
        let mut moves = vec![];
        for rail in self.rails {
            for (pass, &(h, z)) in levels.iter().enumerate() {
                let lines: Vec<Vec<Point>> = if rail.closed {
                    // the profile hangs outside a closed rail
                    let rings = if h.abs() < 1e-9 {
                        vec![(*rail).clone()]
                    } else {
                        polygon::offset(std::slice::from_ref(*rail), h, self.corners)
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
                // the passes taking turns go back the other way every other pass
                let back = self.ways == Ways::Both && pass % 2 == 1;
                let climb = self.direction == CutDirection::Climb;
                for mut line in lines {
                    // as drawn, the profile hangs on the right of an open rail; climb runs
                    // clockwise round a closed one, with the profile outside on its left
                    let reverse = if rail.closed {
                        (winding(&line) < 0.0) != climb
                    } else {
                        climb
                    };
                    if reverse != back {
                        line.reverse();
                    }
                    let cut: Vec<DVec3> = line.iter().map(|q| q.extend(z)).collect();
                    // ponytail: lifts to the safe height between passes; Aspire lifts a stepover
                    moves.extend(follow(&cut, self.safe));
                }
            }
        }
        moves
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

    /// Flat 2 mm at the top, a 45° slope 3 mm down, then flat 10 mm at -3.
    fn stepped_profile() -> Path {
        Path::polyline(
            &[
                Point::new(200.0, 0.0),
                Point::new(202.0, 0.0),
                Point::new(205.0, -3.0),
                Point::new(215.0, -3.0),
            ],
            false,
        )
    }

    fn rail() -> Path {
        Path::polyline(&[Point::new(0.0, 50.0), Point::new(100.0, 50.0)], false)
    }

    /// The toolpaths made, clearance first: (name, (distance across, height) of each pass).
    fn made(p: &Params) -> Vec<(String, Vec<(f64, f64)>)> {
        let mut doc = cut_craft_core::testing::document_with(vec![rail(), stepped_profile()]);
        doc.apply(&MouldingToolpath::run(&doc, p).unwrap()).unwrap();
        doc.toolpaths
            .values()
            .map(|t| {
                let passes = t
                    .moves
                    .iter()
                    .filter(|m| m.kind == MoveKind::Plunge)
                    .map(|m| (50.0 - m.to.y, m.to.z))
                    .collect();
                (t.name.clone(), passes)
            })
            .collect()
    }

    fn mill(diameter: f64) -> Tool {
        Tool {
            diameter,
            stepover: 0.5,
            pass_depth: 1.0,
            ..Tool::default()
        }
    }

    #[test]
    fn varying_the_stepover_brings_passes_closer_on_the_slope() {
        let even = made(&Params::default());
        let varied = made(&Params {
            vary_stepover: true,
            ..Params::default()
        });
        assert!(
            varied[0].1.len() > even[0].1.len(),
            "{varied:?} vs {even:?}"
        );
        // a stepover apart along the surface the tip follows, never further
        let step = Params::default().tool.finishing_stepover() * Params::default().tool.diameter;
        for pair in varied[0].1.windows(2) {
            let along = (pair[1].0 - pair[0].0).hypot(pair[1].1 - pair[0].1);
            assert!(along <= step + 1e-6, "{pair:?}");
        }
    }

    #[test]
    fn a_clearance_tool_roughs_in_levels_leaving_the_allowance() {
        let p = Params {
            tool: mill(2.0),
            clearance_tools: vec![mill(4.0)],
            machining_allowance: 0.5,
            ..Params::default()
        };
        let made = made(&p);
        assert_eq!(made.len(), 2);
        let (name, clear) = &made[0];
        assert_eq!(name, "Moulding [Clear]");
        assert_eq!(made[1].0, "Moulding");
        #[expect(clippy::cast_possible_truncation, reason = "µm")]
        let mut levels: Vec<i64> = clear
            .iter()
            .map(|&(_, z)| (z * 1000.0).round() as i64)
            .collect();
        levels.dedup();
        // three equal levels down to the flat bottom plus the allowance (2.5 mm)
        for level in [-833, -1667, -2500] {
            assert!(levels.contains(&level), "{levels:?}");
        }
        // never below the profile plus the allowance, for the 4 mm tool
        let section = cross_section(&stepped_profile());
        for &(h, z) in clear {
            assert!(
                z >= tip_height(&section, &mill(4.0), h) + 0.5 - 1e-9,
                "{h} {z}"
            );
        }
    }

    #[test]
    fn flats_the_clearance_tool_fits_are_cut_to_height_and_can_be_skipped() {
        let p = Params {
            tool: mill(2.0),
            clearance_tools: vec![mill(4.0)],
            machining_allowance: 0.5,
            machine_flat_regions: true,
            ..Params::default()
        };
        let made_flat = made(&p);
        assert!(
            made_flat[0]
                .1
                .iter()
                .any(|&(h, z)| h > 7.0 && h < 13.0 && (z + 3.0).abs() < 1e-9),
            "the flat bottom cut to -3: {:?}",
            made_flat[0].1
        );
        let skipped = made(&Params {
            skip_flat_regions: true,
            ..p
        });
        assert!(skipped[1].1.len() < made_flat[1].1.len());
        assert!(
            skipped[1].1.iter().all(|&(h, _)| !(h > 8.0 && h < 12.0)),
            "{:?}",
            skipped[1].1
        );
    }

    /// Where each pass along an open rail from x 0 to 100 starts, in order.
    fn open_rail_starts(p: &Params) -> Vec<f64> {
        let rail = Path::polyline(&[Point::new(0.0, 50.0), Point::new(100.0, 50.0)], false);
        moulding(rail, p)
            .moves
            .iter()
            .filter(|m| m.kind == MoveKind::Plunge)
            .map(|m| m.to.x)
            .collect()
    }

    /// The way round each pass of a toolpath goes: positive anticlockwise.
    fn windings(t: &Toolpath) -> Vec<f64> {
        let mut windings = vec![];
        let mut pass: Vec<Point> = vec![];
        let mut done = |pass: &mut Vec<Point>| {
            if pass.len() > 2 {
                windings.push(cut_craft_core::geometry::winding(pass));
            }
            pass.clear();
        };
        for m in &t.moves {
            match m.kind {
                MoveKind::Plunge => pass = vec![m.to.truncate()],
                MoveKind::Cut => pass.push(m.to.truncate()),
                MoveKind::Rapid => done(&mut pass),
            }
        }
        done(&mut pass);
        windings
    }

    #[test]
    fn climb_goes_clockwise_round_a_closed_rail_and_conventional_anticlockwise() {
        let anticlockwise = square(20.0, 20.0, 40.0);
        let clockwise = anticlockwise.reversed();
        for rail in [anticlockwise, clockwise] {
            for (direction, sign) in [
                (CutDirection::Climb, -1.0),
                (CutDirection::Conventional, 1.0),
            ] {
                let p = Params {
                    cut_direction: direction,
                    ..end_mill()
                };
                let windings = windings(&moulding(rail.clone(), &p));
                assert!(windings.len() > 2, "{windings:?}");
                assert!(
                    windings.iter().all(|w| w * sign > 0.0),
                    "{direction:?}: {windings:?}"
                );
            }
        }
    }

    #[test]
    fn the_clearance_tool_cuts_the_same_way_round() {
        let p = Params {
            tool: mill(2.0),
            clearance_tools: vec![mill(4.0)],
            cut_direction: CutDirection::Climb,
            ..Params::default()
        };
        let mut doc = cut_craft_core::testing::document_with(vec![
            square(20.0, 20.0, 40.0),
            stepped_profile(),
        ]);
        doc.apply(&MouldingToolpath::run(&doc, &p).unwrap())
            .unwrap();
        assert_eq!(doc.toolpaths.len(), 2);
        for t in doc.toolpaths.values() {
            let windings = windings(t);
            assert!(!windings.is_empty(), "{}", t.name);
            assert!(
                windings.iter().all(|&w| w < 0.0),
                "{}: {windings:?}",
                t.name
            );
        }
    }

    #[test]
    fn along_an_open_rail_conventional_follows_it_and_climb_comes_back() {
        // the profile hangs on the rail's right, as outside an anticlockwise closed rail
        let single = |cut_direction| Params {
            cut_direction,
            ways: Ways::One,
            ..end_mill()
        };
        let along = open_rail_starts(&single(CutDirection::Conventional));
        assert!(
            along.len() > 2 && along.iter().all(|&x| x == 0.0),
            "{along:?}"
        );
        let back = open_rail_starts(&single(CutDirection::Climb));
        assert!(
            back.len() > 2 && back.iter().all(|&x| x == 100.0),
            "{back:?}"
        );
    }

    #[test]
    fn both_directions_start_the_chosen_way_then_turn_back() {
        let p = |cut_direction| Params {
            cut_direction,
            ways: Ways::Both,
            ..end_mill()
        };
        let starts = open_rail_starts(&p(CutDirection::Conventional));
        assert_eq!(starts[..3], [0.0, 100.0, 0.0]);
        let starts = open_rail_starts(&p(CutDirection::Climb));
        assert_eq!(starts[..3], [100.0, 0.0, 100.0]);
    }

    #[test]
    fn a_new_moulding_climbs_in_one_direction_and_an_old_one_keeps_its_way() {
        let new = Params::default();
        assert_eq!(
            (new.cut_direction, new.ways),
            (CutDirection::Climb, Ways::One)
        );
        // saved before the choice: anticlockwise round a closed rail, back and forth along an open one
        let mut saved = serde_json::to_value(&new).unwrap();
        let fields = saved.as_object_mut().unwrap();
        fields.remove("cut_direction");
        fields.remove("ways");
        let old: Params = serde_json::from_value(saved).unwrap();
        assert_eq!(
            (old.cut_direction, old.ways),
            (CutDirection::Conventional, Ways::Both)
        );
    }

    #[test]
    fn passes_go_back_and_forth_along_an_open_rail() {
        let rail = Path::polyline(&[Point::new(0.0, 50.0), Point::new(100.0, 50.0)], false);
        let t = moulding(
            rail,
            &Params {
                cut_direction: CutDirection::Conventional,
                ways: Ways::Both,
                ..end_mill()
            },
        );
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
