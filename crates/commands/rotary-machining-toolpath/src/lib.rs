//! 4-Axis Rotary Machining: finishes a model all the way round a turning cylinder, ring by
//! ring, the bit kept on the true round surface: X (or Y), Z and A move together.
//!
//! Each point is worked out round the axis itself, not on the unrolled sheet: a bit cutting
//! deep reaches further round the part than it does at the surface, and is kept clear of all
//! of it. Rings each start at A=0, and the machine's rotary settings say how A gets from one
//! to the next (carrying on round, cutting back the other way, or spinning back).
//!
//! The bit can also be held to one side of the axis (Off centre), so the side of the ball
//! cuts rather than its tip; or A can stop at a few angles in turn (Indexed), each side cut
//! like a flat 3D finish with A held still. Either way the moves are kept where the tip cuts
//! on the unrolled material, with how far across the bit is held beside them, so the preview
//! shows them true and the post works out A, the across axis and Z.

use cut_craft_core::component::placed_model;
use cut_craft_core::document::{Along, Document};
use cut_craft_core::geometry::Point;
use cut_craft_core::glam::DVec3;
use cut_craft_core::progress;
use cut_craft_core::rotary_axis;
use cut_craft_core::simulation::Material;
use cut_craft_core::toolpath::{Move, Tool, ToolDatabase, ToolShape, Toolpath, follow, made_by};
use cut_craft_core::{Change, Command, CommandError, Meta, ensure_positive};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub struct RotaryMachiningToolpath;

/// The way the bit goes round the part.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Strategy {
    /// The bit stays straight over the turning axis while A turns and it moves along: like
    /// unrolled 3D finishing, but kept true to the round part
    #[default]
    Radial,
    /// The bit is held a set distance to one side of the turning axis while A turns: the side
    /// of the ball cuts, not its slow-turning tip, for a cleaner finish, and walls that face
    /// sideways are reached
    OffCentre,
    /// A turns to a few angles in turn and stays still at each, while the bit cuts that side
    /// back and forth along the part like a flat 3D finish: for flats, faces and machines
    /// whose A is better at holding still than moving smoothly
    Indexed,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(default)]
pub struct Params {
    /// What the toolpath is called in the toolpath list
    pub name: String,
    /// The bit: a ball nose follows curves best. Its finishing stepover spaces the rings
    pub tool: Tool,
    /// How the bit goes round the part
    #[schemars(extend("x-group" = "Strategy"))]
    pub strategy: Strategy,
    /// How far to one side of the turning axis the bit is held. Positive is the way the part
    /// comes up towards the bit as A turns back
    #[schemars(extend("x-unit" = "mm", "x-group" = "Strategy", "x-show-when" = {"strategy": "off_centre"}))]
    pub offset: f64,
    /// How many sides the part is cut from, evenly round it, starting at A=0: A turns to
    /// each and holds still while that side is cut
    #[schemars(extend("x-group" = "Strategy", "x-show-when" = {"strategy": "indexed"}), range(min = 1, max = 36))]
    pub faces: u32,
    /// Where along the axis the rings start, from the job's start (the chuck end)
    #[schemars(extend("x-unit" = "mm", "x-group" = "Along the part"), range(min = 0))]
    pub start: f64,
    /// Where along the axis the rings end, from the job's start; 0 runs to the job's end
    #[schemars(extend("x-unit" = "mm", "x-group" = "Along the part"), range(min = 0))]
    pub end: f64,
    /// Material left on the part for a later pass, all the way round
    #[schemars(extend("x-unit" = "mm"), range(min = 0))]
    pub allowance: f64,
    /// A small notch at A=0, just past the last ring, so the part can be put back exactly as
    /// it was if a job has to be started again
    pub registration_mark: bool,
}

impl Default for Params {
    fn default() -> Self {
        Self {
            name: "4-Axis Rotary Machining".into(),
            tool: ToolDatabase::starter()
                .tools
                .into_iter()
                .find(|t| t.shape == ToolShape::BallNose)
                .expect("the starter tools include a ball nose"),
            strategy: Strategy::Radial,
            offset: 3.0,
            faces: 4,
            start: 0.0,
            end: 0.0,
            allowance: 0.0,
            registration_mark: false,
        }
    }
}

impl Command for RotaryMachiningToolpath {
    const ID: &'static str = "rotary_machining_toolpath";
    type Params = Params;

    fn meta() -> Meta {
        Meta {
            label: "4-Axis Rotary Machining",
            group: "3D Toolpaths",
            icon: "toolpaths/rotary_machining_toolpath.png",
            shortcut: "",
        }
    }

    fn run(doc: &Document, p: &Params) -> Result<Change, CommandError> {
        let toolpath = machine(doc, p)?;
        let mut c = Change::builder(doc);
        c.add_toolpath(toolpath);
        Ok(c.finish())
    }
}

/// How deep the registration mark goes, mm.
const MARK_DEPTH: f64 = 0.5;

/// The model as radii round the axis, on a grid of (along, round) cells: what the drop
/// works from.
struct Turned {
    /// Cells along the axis, and round it.
    along: usize,
    round: usize,
    /// Cell sizes along and round (at the surface), mm.
    along_cell: f64,
    round_cell: f64,
    /// The material's radius.
    radius: f64,
    /// Radius of each cell, row by row round the axis.
    radii: Vec<f64>,
    /// The smallest radius anywhere, and the largest.
    least: f64,
    most: f64,
}

impl Turned {
    /// `surface` (heights below the material's top, unrolled) as radii, on cells no larger
    /// than `cell`, each the highest of the surface under it so nothing is missed.
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        clippy::cast_precision_loss,
        reason = "cell counts are small and positive"
    )]
    fn of(surface: &Material, radius: f64, along_x: bool, size: (f64, f64), cell: f64) -> Self {
        let (length, circumference) = size;
        let along = ((length / cell).ceil() as usize).max(1);
        let round = ((circumference / cell).ceil() as usize).max(3);
        let (along_cell, round_cell) = (length / along as f64, circumference / round as f64);
        // a few samples across each cell, as fine as the surface's own
        let samples = ((cell / surface.cell).ceil() as usize).clamp(1, 4);
        let mut radii = Vec::with_capacity(along * round);
        for j in 0..round {
            for i in 0..along {
                let mut highest = f64::NEG_INFINITY;
                for si in 0..samples {
                    for sj in 0..samples {
                        let a = (i as f64 + (si as f64 + 0.5) / samples as f64) * along_cell;
                        let w = (j as f64 + (sj as f64 + 0.5) / samples as f64) * round_cell;
                        let at = if along_x {
                            Point::new(a, w)
                        } else {
                            Point::new(w, a)
                        };
                        if let Some(h) = surface.at(at).filter(|h| h.is_finite()) {
                            highest = highest.max(h);
                        }
                    }
                }
                // nothing of the model there: down to the axis
                radii.push((radius + highest).max(0.0));
            }
        }
        let least = radii.iter().copied().fold(f64::INFINITY, f64::min);
        let most = radii.iter().copied().fold(0.0, f64::max);
        Self {
            along,
            round,
            along_cell,
            round_cell,
            radius,
            radii,
            least,
            most,
        }
    }

    /// The highest the tip of `tool` can sit, held `across` mm to the side of the axis, with A
    /// turned to `round` (mm round the surface) and `along` the part, without touching the
    /// model grown by `allowance`: its height above the axis.
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_possible_wrap,
        clippy::cast_sign_loss,
        clippy::cast_precision_loss,
        reason = "cell indices are small"
    )]
    fn drop(&self, tool: &Tool, along: f64, round: f64, across: f64, allowance: f64) -> f64 {
        use std::f64::consts::FRAC_PI_2;
        let rb = tool.diameter / 2.0;
        let ball = tool.shape == ToolShape::BallNose;
        let theta = round / self.radius;
        // how far round the part a point can be and still be under the bit: furthest round
        // where the model comes nearest the axis, or furthest out for the far edge
        let (nearest, furthest) = (
            (self.least + allowance).max(1e-6),
            (self.most + allowance).max(1e-6),
        );
        let angle = |s: f64| s.clamp(-1.0, 1.0).asin();
        let low = if across - rb > 0.0 { furthest } else { nearest };
        let high = if across + rb > 0.0 { nearest } else { furthest };
        let (from, to) = (angle((across - rb) / low), angle((across + rb) / high));
        let rows = |phi: f64| (theta + phi) * self.radius / self.round_cell;
        let first = rows(from).floor() as i64 - 1;
        let last = (rows(to).ceil() as i64 + 1).min(first + self.round as i64 - 1);
        let cols = (rb / self.along_cell).ceil() as i64 + 1;
        let first_col = (along / self.along_cell).floor() as i64;
        let mut tip: f64 = 0.0;
        for col in first_col - cols..=first_col + cols {
            if col < 0 || col >= self.along as i64 {
                continue;
            }
            let dx = (col as f64 + 0.5) * self.along_cell - along;
            if dx.abs() > rb {
                continue;
            }
            for row in first..=last {
                let phi = (row as f64 + 0.5) * self.round_cell / self.radius - theta;
                if phi.abs() >= FRAC_PI_2 {
                    continue;
                }
                let at = row.rem_euclid(self.round as i64) as usize;
                let r = self.radii[at * self.along + col as usize] + allowance;
                let side = r * phi.sin() - across;
                let gap = rb * rb - dx * dx - side * side;
                if gap < 0.0 {
                    continue;
                }
                let here = if ball {
                    // the ball's centre, rb out from the tip, stays rb from the point
                    r * phi.cos() + gap.sqrt() - rb
                } else {
                    // a flat bottom rests on the point
                    r * phi.cos()
                };
                tip = tip.max(here);
            }
        }
        tip
    }
}

/// One pass of the bit: points (along, A as mm round the surface, the bit's height from the
/// material's top) each with how far across the axis the bit is held there.
type Pass = Vec<(DVec3, f64)>;

/// The part as the passes are worked out on it.
struct Work<'a> {
    turned: &'a Turned,
    tool: &'a Tool,
    allowance: f64,
    along_x: bool,
    circumference: f64,
    /// Where along the part the passes start and end, and the step between them.
    start: f64,
    end: f64,
    step: f64,
}

impl Work<'_> {
    /// The point `along` the part and `round` it at height `z`, in the job's X and Y.
    fn point(&self, along: f64, round: f64, z: f64) -> DVec3 {
        if self.along_x {
            DVec3::new(along, round, z)
        } else {
            DVec3::new(round, along, z)
        }
    }

    /// The bit as low as it goes `along` the part with A at `round`, held `across`.
    fn down(&self, along: f64, round: f64, across: f64) -> (DVec3, f64) {
        let radius = self.turned.radius;
        let tip = self.turned.drop(
            self.tool,
            along,
            round.rem_euclid(self.circumference),
            across,
            self.allowance,
        );
        (self.point(along, round, (tip - radius).min(0.0)), across)
    }

    /// A ring round the part at each step along it, from A=0 all the way back round, the bit
    /// held `across`.
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        clippy::cast_precision_loss,
        reason = "ring and point counts are small"
    )]
    fn rings(&self, across: f64) -> Result<Vec<Pass>, CommandError> {
        progress::stage("Following the model round");
        let rings = ((self.end - self.start) / self.step).ceil() as usize + 1;
        let mut passes = Vec::with_capacity(rings);
        for k in 0..rings {
            progress::stop_if_cancelled()?;
            let along = (self.start + k as f64 * self.step).min(self.end);
            // the last point closes the ring
            passes.push(
                (0..=self.turned.round)
                    .map(|j| self.down(along, j as f64 * self.turned.round_cell, across))
                    .collect(),
            );
            progress::report_steps(k + 1, rings);
        }
        Ok(passes)
    }

    /// Each of `faces` sides in turn, from A=0, cut back and forth along the part with A held
    /// still: as far across as its share of the way round, and a bit's width more, so the
    /// sides overlap.
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        clippy::cast_precision_loss,
        reason = "line and point counts are small"
    )]
    fn sides(&self, faces: u32) -> Result<Vec<Pass>, CommandError> {
        progress::stage("Cutting each side");
        let radius = self.turned.radius;
        let half = if faces == 1 {
            radius
        } else {
            (radius * (std::f64::consts::PI / f64::from(faces)).sin() + self.tool.diameter / 2.0)
                .min(radius)
        };
        let lines = ((2.0 * half / self.step).ceil() as usize).max(1) + 1;
        let spacing = 2.0 * half / (lines - 1) as f64;
        let cell = self.turned.along_cell;
        let points = ((self.end - self.start) / cell).ceil() as usize + 1;
        let mut passes = Vec::with_capacity(faces as usize);
        for face in 0..faces {
            progress::stop_if_cancelled()?;
            let round = self.circumference * f64::from(face) / f64::from(faces);
            let mut pass: Pass = vec![];
            for line in 0..lines {
                let across = -half + line as f64 * spacing;
                if let Some(&(last, held)) = pass.last() {
                    // over to the next line, staying down on the part
                    let along = if self.along_x { last.x } else { last.y };
                    let over = ((across - held) / cell).abs().ceil() as usize;
                    for k in 1..over {
                        let between = held + (across - held) * k as f64 / over as f64;
                        pass.push(self.down(along, round, between));
                    }
                }
                let along = |k: usize| (self.start + k as f64 * cell).min(self.end);
                if line % 2 == 0 {
                    pass.extend((0..points).map(|k| self.down(along(k), round, across)));
                } else {
                    pass.extend(
                        (0..points)
                            .rev()
                            .map(|k| self.down(along(k), round, across)),
                    );
                }
            }
            passes.push(pass);
            progress::report_steps(face as usize + 1, faces as usize);
        }
        Ok(passes)
    }
}

/// The toolpath: rings round the part at each step along it, each from A=0 all the way round
/// (over the axis, or held to one side of it), or each side in turn cut back and forth with A
/// held still; the bit as low as it goes without touching the model.
fn machine(doc: &Document, p: &Params) -> Result<Toolpath, CommandError> {
    let Some(rotary) = doc.job.rotary else {
        return Err(CommandError::Failed(
            "this toolpath turns the material: set the job up as rotary in Job Setup".into(),
        ));
    };
    let tool = &p.tool;
    ensure_positive("tool diameter", tool.diameter)?;
    let stepover = tool.finishing_stepover();
    if !(stepover > 0.0 && stepover <= 1.0) {
        return Err(CommandError::InvalidParams(
            "the tool's finishing stepover must be more than 0% and at most 100%: set it in the Tool Database".into(),
        ));
    }
    let along_x = rotary.along == Along::X;
    let (length, circumference) = if along_x {
        (doc.job.width, doc.job.height)
    } else {
        (doc.job.height, doc.job.width)
    };
    let radius = rotary.diameter / 2.0;
    progress::stage("Placing the model");
    let surface = placed_model(doc, tool)?;
    let cell = (tool.diameter / 16.0).max(surface.cell);
    let turned = Turned::of(&surface, radius, along_x, (length, circumference), cell);
    let start = p.start.clamp(0.0, length);
    let end = if p.end > 0.0 {
        p.end.min(length)
    } else {
        length
    };
    if end <= start {
        return Err(CommandError::InvalidParams(
            "the rings must end further along than they start".into(),
        ));
    }
    let work = Work {
        turned: &turned,
        tool,
        allowance: p.allowance,
        along_x,
        circumference,
        start,
        end,
        step: stepover * tool.diameter,
    };
    let mut passes = match p.strategy {
        Strategy::Radial => work.rings(0.0)?,
        Strategy::OffCentre => work.rings(p.offset)?,
        Strategy::Indexed => work.sides(p.faces.clamp(1, 36))?,
    };
    if p.registration_mark {
        // just past the last ring, clear of the part when there is room
        let along = (end + tool.diameter).min(length);
        passes.push(vec![(work.point(along, 0.0, -MARK_DEPTH), 0.0)]);
    }
    let held_off = p.strategy != Strategy::Radial;
    let round = usize::from(along_x);
    let mut moves: Vec<Move> = vec![];
    let mut across = vec![];
    for pass in &passes {
        let points: Vec<DVec3> = pass.iter().map(|&(point, _)| point).collect();
        let mut followed = follow(&points, doc.job.turning_gap());
        if held_off {
            // follow adds a rise to above the first point and from the last
            let held: Vec<f64> = std::iter::once(pass[0].1)
                .chain(pass.iter().map(|&(_, d)| d))
                .chain(std::iter::once(pass[pass.len() - 1].1))
                .collect();
            for (m, &d) in followed.iter_mut().zip(&held) {
                // kept where the tip of the bit held d across cuts, on the unrolled material
                let (w, z) = rotary_axis::tip_of(radius, m.to[round], m.to.z, d);
                m.to[round] = w;
                m.to.z = z;
            }
            across.extend(held);
        }
        moves.extend(followed);
    }
    let mut toolpath =
        made_by::<RotaryMachiningToolpath>(p, p.name.clone(), tool.clone(), vec![], moves);
    toolpath.across = across;
    Ok(toolpath)
}

#[cfg(test)]
mod tests {
    use super::*;
    use cut_craft_core::component::{Combine, Component};
    use cut_craft_core::document::Job;
    use cut_craft_core::toolpath::MoveKind;

    /// A rotary job 40 mm across, 60 mm long along X, holding a part `radius(x)` from the
    /// axis all the way round (its top at the material's surface).
    fn turned(radius: impl Fn(f64) -> f64) -> Document {
        let job = Job::rotary(60.0, 40.0, Along::X);
        let mut doc = Document {
            job,
            ..Document::default()
        };
        let model = Component::from_fn("Turned", Combine::Add, doc.job.area(), 0.25, |p| {
            Some(radius(p.x))
        });
        let mut c = Change::builder(&doc);
        c.add_new_component(model);
        doc.apply(&c.finish()).unwrap();
        doc
    }

    /// A bead: 20 mm round at the ends, 15 mm in the middle.
    fn beaded() -> Document {
        turned(|x| 20.0 - 5.0 * (std::f64::consts::PI * x / 60.0).sin())
    }

    /// A plain cylinder 12 mm from the axis, with a lip at the material's full 20 mm for
    /// its first 2 mm (the model's top sits at the material's surface).
    fn lipped() -> Document {
        turned(|x| if x < 2.0 { 20.0 } else { 12.0 })
    }

    cut_craft_core::contract_tests!(RotaryMachiningToolpath, beaded);

    fn ball(diameter: f64) -> Tool {
        Tool {
            diameter,
            stepover: 0.25,
            finish_stepover: 0.25,
            ..Params::default().tool
        }
    }

    fn cuts(doc: &Document, p: &Params) -> Vec<DVec3> {
        machine(doc, p)
            .unwrap()
            .moves
            .iter()
            .filter(|m| m.kind == MoveKind::Cut)
            .map(|m| m.to)
            .collect()
    }

    #[test]
    fn a_plain_cylinder_is_followed_at_its_own_radius_all_the_way_round() {
        let doc = lipped();
        let p = Params {
            tool: ball(6.0),
            ..Params::default()
        };
        // the part is 12 mm from the axis in a 20 mm one: 8 mm below the top all round
        for c in cuts(&doc, &p).into_iter().filter(|c| c.x > 6.0) {
            assert!((c.z + 8.0).abs() < 0.05, "{c:?}");
        }
    }

    #[test]
    fn each_ring_starts_at_a_zero_and_goes_all_the_way_round() {
        let doc = beaded();
        let p = Params {
            tool: ball(6.0),
            ..Params::default()
        };
        let toolpath = machine(&doc, &p).unwrap();
        let circumference = doc.job.height;
        let plunges: Vec<DVec3> = toolpath
            .moves
            .iter()
            .filter(|m| m.kind == MoveKind::Plunge)
            .map(|m| m.to)
            .collect();
        // 60 mm in 1.5 mm steps: 41 rings, each plunging at A=0
        assert_eq!(plunges.len(), 41);
        assert!(plunges.iter().all(|p| p.y.abs() < 1e-9));
        let last_cut = |i: usize| {
            toolpath.moves[..i]
                .iter()
                .rev()
                .find(|m| m.kind == MoveKind::Cut)
                .map(|m| m.to.y)
        };
        let lifts: Vec<usize> = (1..toolpath.moves.len())
            .filter(|&i| toolpath.moves[i].kind == MoveKind::Rapid)
            .collect();
        for i in lifts.into_iter().skip(1) {
            assert!(
                (last_cut(i).unwrap() - circumference).abs() < 1e-9,
                "each ring ends back at A=0, a whole turn round"
            );
        }
    }

    #[test]
    #[expect(clippy::cast_precision_loss, reason = "small counts")]
    fn the_ball_rests_on_the_part_round_the_axis_and_never_cuts_into_it() {
        // a ridge 1 mm wide all along the part at A=0, the rest 10 mm down: deep in, the
        // ball reaches further round the part than it would on the unrolled sheet
        let mut doc = beaded();
        let circumference = doc.job.height;
        doc.components.clear();
        let model = Component::from_fn("Ridge", Combine::Add, doc.job.area(), 0.1, |p| {
            Some(if p.y.min(circumference - p.y) < 0.5 {
                20.0
            } else {
                10.0
            })
        });
        let mut c = Change::builder(&doc);
        c.add_new_component(model);
        doc.apply(&c.finish()).unwrap();
        let tool = ball(6.0);
        let surface = placed_model(&doc, &tool).unwrap();
        let round = Turned::of(&surface, 20.0, true, (60.0, circumference), 0.1);
        let rb = 3.0;
        for w in [
            0.0,
            1.0,
            2.0,
            3.0,
            4.0,
            6.0,
            10.0,
            30.0,
            circumference - 2.0,
        ] {
            let tip = round.drop(&tool, 30.0, w, 0.0, 0.0);
            let theta = w / 20.0;
            let centre = DVec3::new(30.0, (tip + rb) * theta.cos(), (tip + rb) * theta.sin());
            // the nearest the ball's centre comes to any cell of the part
            let mut nearest = f64::INFINITY;
            for j in 0..round.round {
                for i in 0..round.along {
                    let r = round.radii[j * round.along + i];
                    let phi = (j as f64 + 0.5) * round.round_cell / 20.0;
                    let at = DVec3::new(
                        (i as f64 + 0.5) * round.along_cell,
                        r * phi.cos(),
                        r * phi.sin(),
                    );
                    nearest = nearest.min(centre.distance(at));
                }
            }
            assert!(nearest >= rb - 1e-6, "cuts in at {w}: {nearest}");
            assert!(
                nearest <= rb + 0.05,
                "hangs above the part at {w}: {nearest}"
            );
        }
    }

    #[test]
    fn an_allowance_leaves_material_all_the_way_round() {
        let doc = lipped();
        let p = Params {
            tool: ball(6.0),
            allowance: 0.5,
            ..Params::default()
        };
        for c in cuts(&doc, &p).into_iter().filter(|c| c.x > 6.0) {
            assert!((c.z + 7.5).abs() < 0.05, "{c:?}");
        }
    }

    #[test]
    fn rings_run_from_start_to_end_and_a_registration_mark_comes_after() {
        let doc = lipped();
        let p = Params {
            tool: ball(6.0),
            start: 10.0,
            end: 40.0,
            registration_mark: true,
            ..Params::default()
        };
        let toolpath = machine(&doc, &p).unwrap();
        let plunges: Vec<DVec3> = toolpath
            .moves
            .iter()
            .filter(|m| m.kind == MoveKind::Plunge)
            .map(|m| m.to)
            .collect();
        assert!((plunges[0].x - 10.0).abs() < 1e-9);
        let mark = plunges[plunges.len() - 1];
        assert!((plunges[plunges.len() - 2].x - 40.0).abs() < 1e-9);
        assert_eq!((mark.x, mark.y, mark.z), (46.0, 0.0, -MARK_DEPTH));
    }

    /// Each cut of `toolpath` as the machine makes it: (along, A as mm round, the bit's
    /// height from the top, how far across it is held).
    fn held(toolpath: &Toolpath) -> Vec<(f64, f64, f64, f64)> {
        toolpath
            .moves
            .iter()
            .zip(&toolpath.across)
            .filter(|(m, _)| m.kind == MoveKind::Cut)
            .map(|(m, &d)| {
                let (w, z) = rotary_axis::held_at(20.0, m.to.y, m.to.z, d);
                (m.to.x, w, z, d)
            })
            .collect()
    }

    #[test]
    fn held_off_centre_the_ball_still_rests_on_the_part_all_the_way_round() {
        let doc = lipped();
        let p = Params {
            tool: ball(6.0),
            strategy: Strategy::OffCentre,
            offset: 5.0,
            ..Params::default()
        };
        let toolpath = machine(&doc, &p).unwrap();
        assert_eq!(toolpath.across.len(), toolpath.moves.len());
        let cuts = held(&toolpath);
        assert!(cuts.iter().all(|c| c.3 == 5.0));
        // the part is 12 from the axis: the ball's centre 15 from it, wherever A is
        for (a, _, z, d) in cuts.into_iter().filter(|c| c.0 > 6.0) {
            let centre = d.hypot(20.0 + z + 3.0);
            assert!((centre - 15.0).abs() < 0.05, "{a}: {centre}");
        }
        // kept where the tip cuts: 5 across is a little round from A, and not as deep
        let tip = toolpath
            .moves
            .iter()
            .find(|m| m.kind == MoveKind::Cut)
            .unwrap();
        assert!(tip.to.y > 0.0 && tip.to.z > -8.0, "{tip:?}");
    }

    #[test]
    #[expect(clippy::cast_precision_loss, reason = "small counts")]
    fn held_off_centre_the_ball_never_cuts_into_a_narrow_ridge() {
        let mut doc = beaded();
        let circumference = doc.job.height;
        doc.components.clear();
        let model = Component::from_fn("Ridge", Combine::Add, doc.job.area(), 0.1, |p| {
            Some(if p.y.min(circumference - p.y) < 0.5 {
                20.0
            } else {
                10.0
            })
        });
        let mut c = Change::builder(&doc);
        c.add_new_component(model);
        doc.apply(&c.finish()).unwrap();
        let tool = ball(6.0);
        let surface = placed_model(&doc, &tool).unwrap();
        let turned = Turned::of(&surface, 20.0, true, (60.0, circumference), 0.1);
        let rb = 3.0;
        for across in [-6.0, -2.0, 4.0, 9.0] {
            for w in [0.0, 1.0, 3.0, 6.0, 10.0, circumference - 2.0] {
                let tip = turned.drop(&tool, 30.0, w, across, 0.0);
                let theta = w / 20.0;
                // the ball's centre, across and up, with A turned to theta
                let centre = DVec3::new(30.0, across, tip + rb);
                let mut nearest = f64::INFINITY;
                for j in 0..turned.round {
                    for i in 0..turned.along {
                        let r = turned.radii[j * turned.along + i];
                        let phi = (j as f64 + 0.5) * turned.round_cell / 20.0 - theta;
                        let at = DVec3::new(
                            (i as f64 + 0.5) * turned.along_cell,
                            r * phi.sin(),
                            r * phi.cos(),
                        );
                        nearest = nearest.min(centre.distance(at));
                    }
                }
                assert!(nearest >= rb - 1e-6, "cuts in at {across}, {w}: {nearest}");
                assert!(
                    nearest <= rb + 0.05 || tip == 0.0,
                    "hangs above the part at {across}, {w}: {nearest}"
                );
            }
        }
    }

    #[test]
    fn indexed_cuts_each_side_with_a_held_still_and_the_bit_moving_across() {
        let doc = lipped();
        let p = Params {
            tool: ball(6.0),
            strategy: Strategy::Indexed,
            faces: 4,
            ..Params::default()
        };
        let toolpath = machine(&doc, &p).unwrap();
        let circumference = doc.job.height;
        // four sides, each plunged into once, at A = 0, 90, 180 and 270
        let mut sides: Vec<f64> = vec![];
        for (a, w, z, d) in held(&toolpath) {
            if !sides.iter().any(|s| (s - w).abs() < 1e-6) {
                sides.push(w);
            }
            // never into the part, 12 from the axis
            if a > 6.0 {
                let centre = d.hypot(20.0 + z + 3.0);
                assert!(centre >= 15.0 - 0.05, "{a}, {d}: {centre}");
            }
        }
        let quarter = circumference / 4.0;
        assert_eq!(sides.len(), 4, "{sides:?}");
        for (s, k) in sides.iter().zip([0.0, 1.0, 2.0, 3.0]) {
            assert!((s - quarter * k).abs() < 1e-6, "{sides:?}");
        }
        let plunges = toolpath
            .moves
            .iter()
            .filter(|m| m.kind == MoveKind::Plunge)
            .count();
        assert_eq!(plunges, 4);
        // the bit goes back and forth across each side, a little past its share
        let most = toolpath.across.iter().copied().fold(0.0, f64::max);
        assert!(most > 20.0 * std::f64::consts::FRAC_PI_4.sin(), "{most}");
    }

    #[test]
    fn a_flat_job_is_asked_to_be_rotary() {
        let doc = cut_craft_core::testing::sample_document();
        let why = machine(&doc, &Params::default()).unwrap_err().to_string();
        assert!(why.contains("rotary"), "{why}");
    }
}
