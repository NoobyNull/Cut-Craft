//! 4-Axis Rotary Machining: finishes a model all the way round a turning cylinder, ring by
//! ring, the bit kept on the true round surface: X (or Y), Z and A move together.
//!
//! Each point is worked out round the axis itself, not on the unrolled sheet: a bit cutting
//! deep reaches further round the part than it does at the surface, and is kept clear of all
//! of it. Rings each start at A=0, and the machine's rotary settings say how A gets from one
//! to the next (carrying on round, cutting back the other way, or spinning back).

use cut_craft_core::component::placed_model;
use cut_craft_core::document::{Along, Document};
use cut_craft_core::geometry::Point;
use cut_craft_core::glam::DVec3;
use cut_craft_core::progress;
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
struct Round {
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
    /// The smallest radius anywhere.
    least: f64,
}

impl Round {
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
        Self {
            along,
            round,
            along_cell,
            round_cell,
            radius,
            radii,
            least,
        }
    }

    /// The highest the tip of `tool` can sit straight over the axis at `a` along and `w`
    /// round (at the surface) without touching the model grown by `allowance`: its distance
    /// from the axis.
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_possible_wrap,
        clippy::cast_sign_loss,
        clippy::cast_precision_loss,
        reason = "cell indices are small"
    )]
    fn drop(&self, tool: &Tool, a: f64, w: f64, allowance: f64) -> f64 {
        use std::f64::consts::FRAC_PI_2;
        let rb = tool.diameter / 2.0;
        let ball = tool.shape == ToolShape::BallNose;
        let theta = w / self.radius;
        // how far round the bit can reach: furthest where the model comes nearest the axis
        let least = self.least + allowance;
        let reach = if least > rb {
            (rb / least).asin()
        } else {
            FRAC_PI_2
        };
        let across = (rb / self.along_cell).ceil() as i64 + 1;
        let round =
            ((reach * self.radius / self.round_cell).ceil() as i64 + 1).min(self.round as i64 / 2);
        let i0 = (a / self.along_cell).floor() as i64;
        let j0 = (w / self.round_cell).floor() as i64;
        let mut tip: f64 = 0.0;
        for di in -across..=across {
            let i = i0 + di;
            if i < 0 || i >= self.along as i64 {
                continue;
            }
            let dx = (i as f64 + 0.5) * self.along_cell - a;
            if dx.abs() > rb {
                continue;
            }
            for dj in -round..=round {
                let j = (j0 + dj).rem_euclid(self.round as i64) as usize;
                let phi = (j0 + dj) as f64 * self.round_cell + 0.5 * self.round_cell;
                let phi = phi / self.radius - theta;
                if phi.abs() >= FRAC_PI_2 {
                    continue;
                }
                let r = self.radii[j * self.along + i as usize] + allowance;
                let side = r * phi.sin();
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

/// The toolpath: a ring round the part at each step along it, each from A=0 all the way
/// round, the bit as low as it goes without touching the model.
#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_precision_loss,
    reason = "ring and point counts are small"
)]
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
    let round = Round::of(&surface, radius, along_x, (length, circumference), cell);
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
    let step = stepover * tool.diameter;
    let rings = ((end - start) / step).ceil() as usize + 1;
    let points = round.round;
    let at = |a: f64, w: f64, z: f64| {
        if along_x {
            DVec3::new(a, w, z)
        } else {
            DVec3::new(w, a, z)
        }
    };
    progress::stage("Following the model round");
    let mut moves: Vec<Move> = vec![];
    for k in 0..rings {
        progress::stop_if_cancelled()?;
        let a = (start + k as f64 * step).min(end);
        // round from A=0 back to A=0, the last point closing the ring
        let ring: Vec<DVec3> = (0..=points)
            .map(|j| {
                let w = j as f64 * round.round_cell;
                let tip = round.drop(tool, a, w % circumference, p.allowance);
                at(a, w, (tip - radius).min(0.0))
            })
            .collect();
        moves.extend(follow(&ring, doc.job.clearance));
        progress::report_steps(k + 1, rings);
    }
    if p.registration_mark {
        // just past the last ring, clear of the part when there is room
        let a = (end + tool.diameter).min(length);
        let notch = at(a, 0.0, -MARK_DEPTH);
        moves.extend(follow(&[notch], doc.job.clearance));
    }
    Ok(made_by::<RotaryMachiningToolpath>(
        p,
        p.name.clone(),
        tool.clone(),
        vec![],
        moves,
    ))
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
        let round = Round::of(&surface, 20.0, true, (60.0, circumference), 0.1);
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
            let tip = round.drop(&tool, 30.0, w, 0.0);
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

    #[test]
    fn a_flat_job_is_asked_to_be_rotary() {
        let doc = cut_craft_core::testing::sample_document();
        let why = machine(&doc, &Params::default()).unwrap_err().to_string();
        assert!(why.contains("rotary"), "{why}");
    }
}
