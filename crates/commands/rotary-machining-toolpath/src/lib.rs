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
//!
//! Or it can work bulk to fine detail (Sides, then details): the bulk cleared from each side
//! with A held still, each side finished, then A turned only where detail is left. It keeps
//! what is left of the blank as it goes and never cuts air.

use cut_craft_core::component::{Combine, model, placed_model};
use cut_craft_core::document::{Along, Document};
use cut_craft_core::geometry::Point;
use cut_craft_core::glam::DVec3;
use cut_craft_core::progress;
use cut_craft_core::rotary_axis;
use cut_craft_core::simulation::Material;
use cut_craft_core::toolpath::{
    Move, MoveKind, Tool, ToolDatabase, ToolShape, Toolpath, follow, made_by,
};
use cut_craft_core::{Change, Command, CommandError, Meta, ensure_positive};
use cut_craft_rotary::{Cutter, Mesh, Pose, Stock, Voxels};
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
    /// Bulk to fine detail: A turns to each side in turn and holds still while the bulk is
    /// cleared from it, a level at a time, then each side is finished, like a two-sided job
    /// the machine turns over itself; last, A turns only where detail is left that no side
    /// could reach. The bit never cuts air: every move is checked against what is left
    SidesThenDetails,
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
    #[schemars(extend("x-group" = "Strategy", "x-show-when" = {"strategy": ["indexed", "sides_then_details"]}), range(min = 1, max = 36))]
    pub faces: u32,
    /// Material the bulk clearing leaves on the model for each side's finish to take off
    #[schemars(extend("x-unit" = "mm", "x-group" = "Strategy", "x-show-when" = {"strategy": "sides_then_details"}), range(min = 0))]
    pub roughing_allowance: f64,
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
            roughing_allowance: 0.5,
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

/// How far above where a stretch starts the bit comes down to at the rapid rate, when
/// nothing is in the way, before it plunges, mm.
const APPROACH: f64 = 1.0;

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
    fn of(
        surface: Option<&Material>,
        radius: f64,
        along_x: bool,
        size: (f64, f64),
        cell: f64,
    ) -> Self {
        let (length, circumference) = size;
        let along = ((length / cell).ceil() as usize).max(1);
        let round = ((circumference / cell).ceil() as usize).max(3);
        let (along_cell, round_cell) = (length / along as f64, circumference / round as f64);
        // a few samples across each cell, as fine as the surface's own
        let samples = surface.map_or(1, |m| ((cell / m.cell).ceil() as usize).clamp(1, 4));
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
                        if let Some(h) = surface.and_then(|m| m.at(at)).filter(|h| h.is_finite()) {
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

/// Full 3D models as they really are round the axis: undercuts, holes through and all.
struct Solid {
    mesh: Mesh,
    /// The bit grown by the allowance all round.
    cutter: Cutter,
    allowance: f64,
    /// How far out the model was moved to sit in the material where Material Setup puts it.
    out: f64,
}

impl Solid {
    /// Every shown full 3D model of `doc` kept round the axis, raised or merged onto the
    /// rest (one sunk into the rest, or multiplying it, is left to its heights), moved `out`
    /// from the axis.
    fn of(doc: &Document, tool: &Tool, allowance: f64, out: f64) -> Option<Self> {
        let triangles: Vec<_> = doc
            .components
            .values()
            .filter(|k| k.visible && takes_solid(k))
            .flat_map(cut_craft_core::component::Component::solid_triangles)
            .collect();
        if triangles.is_empty() {
            return None;
        }
        let grown = Tool {
            diameter: tool.diameter + 2.0 * allowance.max(0.0),
            ..tool.clone()
        };
        Some(Self {
            mesh: Mesh::new(triangles),
            cutter: Cutter::new(&grown),
            allowance: allowance.max(0.0),
            out,
        })
    }

    /// [`Turned::drop`] on the models themselves, A turned `turn` radians.
    fn drop(&self, along: f64, turn: f64, across: f64) -> f64 {
        self.mesh
            .drop(&self.cutter, along, across, turn)
            .map_or(0.0, |tip| tip + self.allowance + self.out)
    }
}

/// Whether a component is cut as its real 3D shape: a full model on a rotary job, raised
/// onto or merged with what is under it.
fn takes_solid(k: &cut_craft_core::component::Component) -> bool {
    !k.solid.is_empty() && matches!(k.combine, Combine::Add | Combine::Merge)
}

/// One pass of the bit: points (along, A as mm round the surface, the bit's height from the
/// material's top) each with how far across the axis the bit is held there.
type Pass = Vec<(DVec3, f64)>;

/// The part as the passes are worked out on it.
struct Work<'a> {
    turned: &'a Turned,
    /// The full 3D models kept as they really are round the axis, the bit to drop on them
    /// (grown by the allowance), and how far out from the axis their heights were moved to
    /// sit in the material as the rest of the model was.
    solid: Option<&'a Solid>,
    tool: &'a Tool,
    allowance: f64,
    along_x: bool,
    circumference: f64,
    /// Where along the part the passes start and end, and the step between them.
    start: f64,
    end: f64,
    step: f64,
    /// The highest the tip ever needs to be above the axis: how far out the blank swings
    /// (a board or a square reaches past its own radius as it turns).
    top: f64,
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
        let tip = self.tip(along, round, across);
        (
            self.point(
                along,
                round,
                (tip - self.turned.radius).min(self.top - self.turned.radius),
            ),
            across,
        )
    }

    /// The lowest the tip of the bit can go without touching the model: its height above the
    /// axis.
    fn tip(&self, along: f64, round: f64, across: f64) -> f64 {
        let radius = self.turned.radius;
        let turned = round.rem_euclid(self.circumference);
        let mut tip = self
            .turned
            .drop(self.tool, along, turned, across, self.allowance);
        if let Some(solid) = self.solid {
            tip = tip.max(solid.drop(along, turned / radius, across));
        }
        tip
    }

    /// Where the bit is at a point of a pass, in the machine's frame.
    fn pose(&self, &(point, across): &(DVec3, f64)) -> Pose {
        let (along, round) = if self.along_x {
            (point.x, point.y)
        } else {
            (point.y, point.x)
        };
        Pose::new(
            along,
            across,
            self.turned.radius + point.z,
            round / self.turned.radius,
        )
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
        // a batch at a time over every core, so it can be stopped and show how far it is
        let batch = 4 * cores();
        for first in (0..rings).step_by(batch) {
            progress::stop_if_cancelled()?;
            let count = batch.min(rings - first);
            passes.extend(parallel(count, |i| {
                let along = (self.start + (first + i) as f64 * self.step).min(self.end);
                // the last point closes the ring
                (0..=self.turned.round)
                    .map(|j| self.down(along, j as f64 * self.turned.round_cell, across))
                    .collect::<Pass>()
            }));
            progress::report_steps(first + count, rings);
        }
        Ok(passes)
    }

    /// Each of `faces` sides in turn, from A=0, cut back and forth along the part with A held
    /// still: as far across as its share of the way round, and a bit's width more, so the
    /// sides overlap.
    fn sides(&self, faces: u32) -> Result<Vec<Pass>, CommandError> {
        progress::stage("Cutting each side");
        let radius = self.turned.radius;
        let half = if faces == 1 {
            radius
        } else {
            (radius * (std::f64::consts::PI / f64::from(faces)).sin() + self.tool.diameter / 2.0)
                .min(radius)
        };
        let mut passes = Vec::with_capacity(faces as usize);
        for face in 0..faces {
            progress::stop_if_cancelled()?;
            let round = self.circumference * f64::from(face) / f64::from(faces);
            passes.push(
                self.side(half, self.step, self.turned.along_cell, |along, across| {
                    self.down(along, round, across)
                }),
            );
            progress::report_steps(face as usize + 1, faces as usize);
        }
        Ok(passes)
    }

    /// One side: lines `step` apart across it from `-half` to `half`, cut
    /// back and forth along the part at points `cell` apart, staying down from one line
    /// over to the next; `at` says where the bit is at each point.
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        clippy::cast_precision_loss,
        reason = "line and point counts are small"
    )]
    fn side(
        &self,
        half: f64,
        step: f64,
        cell: f64,
        at: impl Fn(f64, f64) -> (DVec3, f64) + Sync,
    ) -> Pass {
        let lines = ((2.0 * half / step).ceil() as usize).max(1) + 1;
        let spacing = 2.0 * half / (lines - 1) as f64;
        let points = ((self.end - self.start) / cell).ceil() as usize + 1;
        let along = |k: usize| (self.start + k as f64 * cell).min(self.end);
        let across = |line: usize| -half + line as f64 * spacing;
        // each line worked out on its own, over every core: back and forth, and over to it
        // from where the line before ended, staying down on the part
        parallel(lines, |line| {
            let mut pass: Pass = vec![];
            if line > 0 {
                let (held, to) = (across(line - 1), across(line));
                let ended = along(if line % 2 == 1 { points - 1 } else { 0 });
                let over = ((to - held) / cell).abs().ceil() as usize;
                for k in 1..over {
                    pass.push(at(ended, held + (to - held) * k as f64 / over as f64));
                }
            }
            if line % 2 == 0 {
                pass.extend((0..points).map(|k| at(along(k), across(line))));
            } else {
                pass.extend((0..points).rev().map(|k| at(along(k), across(line))));
            }
            pass
        })
        .concat()
    }

    /// Bulk to fine detail ([`Strategy::SidesThenDetails`]): each of `faces` sides cleared
    /// a level at a time down to `roughing_allowance` above the model, A held still; then
    /// each side finished; then rings round the part where material is still left. What is
    /// left of the `stock` is kept as the passes are worked out, and any stretch where the
    /// bit would cut nothing is left out.
    fn sides_then_details(
        &self,
        faces: u32,
        stock: &Stock,
        roughing_allowance: f64,
    ) -> Result<Vec<(Pass, bool)>, CommandError> {
        let tool = self.tool;
        let d = tool.diameter;
        let radius = self.turned.radius;
        let depth = if tool.pass_depth > 0.0 {
            tool.pass_depth
        } else {
            d / 2.0
        };
        // no finer than a twelfth of the bit: what is left between is not worth a pass
        let mut material = Voxels::of(stock, Voxels::cell_for(stock, CELLS).max(d / 12.0));
        // only the bottom of the bit matters for what a pass cuts: the material above it
        // went with the levels before. The bulk is cleared wherever the bit finds material;
        // a side is finished wherever there is any within a cell of it, however thin; a
        // ring goes round only where the sides left a few cells' worth
        let cell = material.cell();
        let rough = Check::new(tool, depth + cell, 0.0, 1, cell);
        let fine = Check::new(tool, d, cell, 1, cell);
        let detail = Check::new(tool, d, 0.0, DETAIL, cell);
        let swing = stock.swing();
        let mut passes = vec![];

        progress::stage("Clearing the bulk from each side");
        let rough_step = tool.stepover.clamp(0.05, 1.0) * d;
        let rough_cell = (d / 4.0).max(self.turned.along_cell);
        for face in 0..faces {
            let round = self.circumference * f64::from(face) / f64::from(faces);
            // the lowest the bit may go anywhere on the side, and the highest a fine drop
            // comes anywhere near each coarse point, so the straight moves between them
            // never dip into the model
            let floor = |along: f64, across: f64| {
                let fine_cell = self.turned.along_cell;
                let n = (rough_cell / fine_cell).ceil();
                let mut top = f64::NEG_INFINITY;
                let mut k = -n;
                while k <= n {
                    let a = (along + k * fine_cell).clamp(self.start, self.end);
                    top = top.max(self.tip(a, round, across));
                    k += 1.0;
                }
                top + roughing_allowance
            };
            let floors = self.side(swing, rough_step, rough_cell, |along, across| {
                (
                    self.point(along, round, floor(along, across) - radius),
                    across,
                )
            });
            let lowest = floors
                .iter()
                .map(|(p, _)| p.z + radius)
                .fold(f64::INFINITY, f64::min);
            let mut level = swing;
            let mut k = 0;
            while level > lowest {
                progress::stop_if_cancelled()?;
                k += 1;
                level = (swing - f64::from(k) * depth).max(lowest);
                let pass: Pass = floors
                    .iter()
                    .map(|&(p, across)| {
                        let mut p = p;
                        p.z = p.z.max(level - radius);
                        (p, across)
                    })
                    .collect();
                passes.extend(self.cutting(&pass, &mut material, &rough));
            }
            progress::report_steps(face as usize + 1, 3 * faces as usize);
        }

        progress::stage("Finishing each side");
        // right across the blank: a board is wider than it is thick, and what is air is
        // left out anyway
        let half = swing;
        for face in 0..faces {
            progress::stop_if_cancelled()?;
            let round = self.circumference * f64::from(face) / f64::from(faces);
            let pass = self.side(half, self.step, self.turned.along_cell, |along, across| {
                (
                    self.point(along, round, self.tip(along, round, across) - radius),
                    across,
                )
            });
            passes.extend(self.cutting(&pass, &mut material, &fine));
            progress::report_steps((faces + face) as usize + 1, 3 * faces as usize);
        }

        progress::stage("Turning for the details");
        // a few cells, not one: the sides leave slivers finer than a cell that are not
        // worth a ring
        for ring in self.rings(0.0)? {
            progress::stop_if_cancelled()?;
            passes.extend(self.cutting(&ring, &mut material, &detail));
        }
        progress::report_steps(3 * faces as usize, 3 * faces as usize);
        Ok(passes)
    }

    /// `pass` broken where the bit would cut too little of what is left of the `material`
    /// to be worth it (as `check` says), which loses what the rest cuts.
    fn cutting(
        &self,
        pass: &[(DVec3, f64)],
        material: &mut Voxels,
        check: &Check,
    ) -> Vec<(Pass, bool)> {
        let Check {
            cutter,
            looks,
            above,
            margin,
            enough,
        } = check;
        let mut cuts = Vec::with_capacity(pass.len());
        // where a stretch may start, whether the bit can come straight down to it quickly:
        // nothing of the material anywhere above it, just before it goes there
        let mut clear = vec![false; pass.len()];
        let mut last: Option<Pose> = None;
        for (i, point) in pass.iter().enumerate() {
            let pose = self.pose(point);
            let looking = Pose {
                height: pose.height - margin,
                ..pose
            };
            let cutting = material.held(looks, looking, *enough) >= *enough;
            if cutting {
                if last.is_none() {
                    let start = if i > 0 { i - 1 } else { i };
                    clear[start] = material.held(above, self.pose(&pass[start]), 1) == 0;
                }
                match last {
                    Some(from) => material.cut(cutter, from, pose),
                    None => material.cut_at(cutter, pose),
                };
                last = Some(pose);
            } else {
                last = None;
            }
            cuts.push(cutting);
        }
        let found = runs(pass, &cuts, 2.0 * self.tool.diameter);
        found
            .into_iter()
            .map(|run| (clear[run.start], pass[run].to_vec()))
            .map(|(clear, run)| (run, clear))
            .collect()
    }
}

/// How many threads to work out drops on.
fn cores() -> usize {
    std::thread::available_parallelism().map_or(1, std::num::NonZero::get)
}

/// `f` of each of `0..count`, in order, worked out over every core.
fn parallel<T: Send>(count: usize, f: impl Fn(usize) -> T + Sync) -> Vec<T> {
    let threads = cores().min(count).max(1);
    if threads == 1 {
        return (0..count).map(f).collect();
    }
    let f = &f;
    let mut parts: Vec<Vec<(usize, T)>> = std::thread::scope(|scope| {
        let handles: Vec<_> = (0..threads)
            .map(|t| {
                scope.spawn(move || {
                    (t..count)
                        .step_by(threads)
                        .map(|i| (i, f(i)))
                        .collect::<Vec<_>>()
                })
            })
            .collect();
        handles
            .into_iter()
            .map(|h| h.join().expect("a drop never panics"))
            .collect()
    });
    let mut all: Vec<(usize, T)> = parts.drain(..).flatten().collect();
    all.sort_by_key(|(i, _)| *i);
    all.into_iter().map(|(_, t)| t).collect()
}

/// How a pass of bulk to fine detail decides whether a point cuts anything.
struct Check {
    /// The bit, as far up as matters.
    cutter: Cutter,
    /// The bit grown by `margin` all round (and its tip lowered by it), to look for
    /// material with: the material is kept in cells, and a layer thinner than a cell must
    /// still be found.
    looks: Cutter,
    /// The whole bit and holder, all the way up, a cell wider all round: what must find
    /// nothing above a point for the bit to come straight down to it at the rapid rate.
    above: Cutter,
    margin: f64,
    /// How many cells it must find.
    enough: usize,
}

impl Check {
    fn new(tool: &Tool, length: f64, margin: f64, enough: usize, cell: f64) -> Self {
        let bit = |grow: f64, length: f64| {
            Cutter::new(&Tool {
                diameter: tool.diameter + 2.0 * grow,
                flute_length: length + grow,
                ..tool.clone()
            })
        };
        Self {
            cutter: bit(0.0, length),
            looks: bit(margin, length),
            // past the material's swing however long the bit is: as high as matters
            above: bit(cell, f64::MAX / 4.0),
            margin,
            enough,
        }
    }
}

/// How many cells the material is kept in as bulk to fine detail works it out: about 16 MB.
const CELLS: usize = 120_000_000;

/// How many cells of material a ring must find to be worth cutting for detail.
const DETAIL: usize = 3;

/// The model in two parts: the rest as heights (`None` when there is none), and full 3D
/// models cut round as they really are, each moved as the whole model was (`surface`) to
/// sit in the material.
fn parts(
    doc: &Document,
    tool: &Tool,
    allowance: f64,
    radius: f64,
    surface: Material,
) -> (Option<Material>, Option<Solid>) {
    // full 3D models are cut round as they really are; the rest from their heights, moved
    // as the whole model was to sit in the material
    let shift = {
        let raw = model(doc, surface.columns.max(surface.rows));
        let top = |m: &Material| m.heights.iter().copied().fold(f64::NEG_INFINITY, f64::max);
        top(&surface) - top(&raw)
    };
    let solid = Solid::of(doc, tool, allowance, radius + shift);
    let rest = if solid.is_some() {
        let rest = Document {
            components: doc
                .components
                .iter()
                .filter(|(_, k)| !takes_solid(k))
                .map(|(id, k)| (*id, k.clone()))
                .collect(),
            ..doc.clone()
        };
        (!rest.components.is_empty()).then(|| {
            let mut m = model(&rest, surface.columns.max(surface.rows));
            for h in &mut m.heights {
                *h += shift;
            }
            m
        })
    } else {
        Some(surface)
    };
    (rest, solid)
}

/// How far into the material the bit must go for a point to count as cutting, mm.
const GRAZE: f64 = 0.05;

/// `passes` with every stretch that cuts only air taken out, each broken where the bit
/// would be clear of the `stock` (the blank as Job Setup has it, turned as A says): the bit
/// lifts and moves over instead. One point of air is kept at each end of what is left, so
/// the bit comes onto the material and leaves it at its cutting height, and short gaps (less
/// than two bit widths) are cut through rather than lifted over.
fn in_material(
    passes: Vec<Pass>,
    stock: &Stock,
    cutter: &Cutter,
    along_x: bool,
    radius: f64,
) -> Vec<Pass> {
    let short = 2.0 * cutter.tool().diameter;
    let cuts = |&(point, across): &(DVec3, f64)| {
        let (along, round) = if along_x {
            (point.x, point.y)
        } else {
            (point.y, point.x)
        };
        stock
            .drop(cutter, along, across, round / radius)
            .is_some_and(|top| radius + point.z < top - GRAZE)
    };
    passes
        .into_iter()
        .flat_map(|pass| {
            let cutting: Vec<bool> = pass.iter().map(cuts).collect();
            runs(&pass, &cutting, short)
                .into_iter()
                .map(|run| pass[run].to_vec())
                .collect::<Vec<_>>()
        })
        .collect()
}

/// The stretches of `pass` where `cutting` says the bit cuts something, each with one
/// point of air kept at either end, so the bit comes onto the material and leaves it at its
/// cutting height; gaps shorter than `short` are cut through rather than lifted over.
fn runs(pass: &[(DVec3, f64)], cutting: &[bool], short: f64) -> Vec<std::ops::Range<usize>> {
    let mut keep = cutting.to_vec();
    for (i, &c) in cutting.iter().enumerate() {
        if c {
            if i > 0 {
                keep[i - 1] = true;
            }
            if i + 1 < keep.len() {
                keep[i + 1] = true;
            }
        }
    }
    let mut i = 0;
    while i < keep.len() {
        if keep[i] {
            i += 1;
            continue;
        }
        let from = i;
        while i < keep.len() && !keep[i] {
            i += 1;
        }
        if from > 0 && i < keep.len() {
            let length: f64 = pass[from - 1..=i]
                .windows(2)
                .map(|w| w[0].0.distance(w[1].0))
                .sum();
            if length < short {
                keep[from..i].fill(true);
            }
        }
    }
    let mut out = vec![];
    let mut from: Option<usize> = None;
    for (i, &kept) in keep.iter().enumerate() {
        match (kept, from) {
            (true, None) => from = Some(i),
            (false, Some(f)) => {
                out.push(f..i);
                from = None;
            }
            _ => {}
        }
    }
    if let Some(f) = from {
        out.push(f..keep.len());
    }
    out
}

/// The passes as moves, each from above its start and back up after: with how far across
/// the bit is held at each when it is `held_off` the axis (otherwise none).
fn joined(
    passes: &[(Pass, bool)],
    safe: f64,
    held_off: bool,
    radius: f64,
    along_x: bool,
) -> (Vec<Move>, Vec<f64>) {
    let round = usize::from(along_x);
    let mut moves: Vec<Move> = vec![];
    let mut across = vec![];
    for (pass, quick) in passes {
        let points: Vec<DVec3> = pass.iter().map(|&(point, _)| point).collect();
        let mut followed = follow(&points, safe);
        let mut held: Vec<f64> = std::iter::once(pass[0].1)
            .chain(pass.iter().map(|&(_, d)| d))
            .chain(std::iter::once(pass[pass.len() - 1].1))
            .collect();
        if *quick && followed.len() > 1 {
            // nothing above where it starts: down at the rapid rate to just above it, and
            // only the last of the way at the plunge rate
            let mut near = followed[1];
            near.kind = MoveKind::Rapid;
            near.to.z += APPROACH;
            followed.insert(1, near);
            held.insert(1, pass[0].1);
        }
        if held_off {
            // follow adds a rise to above the first point and from the last
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
    (moves, across)
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
    let (rest, solid) = parts(doc, tool, p.allowance, radius, surface);
    let turned = Turned::of(
        rest.as_ref(),
        radius,
        along_x,
        (length, circumference),
        cell,
    );
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
        solid: solid.as_ref(),
        tool,
        allowance: p.allowance,
        along_x,
        circumference,
        start,
        end,
        step: stepover * tool.diameter,
        top: Stock::of(&doc.job).map_or(radius, |stock| stock.swing().max(radius)),
    };
    let plain = |passes: Vec<Pass>| passes.into_iter().map(|p| (p, false)).collect();
    // each pass, and whether the bit can come down to it at the rapid rate
    let mut passes: Vec<(Pass, bool)> = match p.strategy {
        Strategy::Radial => plain(work.rings(0.0)?),
        Strategy::OffCentre => plain(work.rings(p.offset)?),
        Strategy::Indexed => plain(work.sides(p.faces.clamp(1, 36))?),
        Strategy::SidesThenDetails => match Stock::of(&doc.job) {
            Some(stock) => {
                work.sides_then_details(p.faces.clamp(1, 36), &stock, p.roughing_allowance)?
            }
            None => plain(work.sides(p.faces.clamp(1, 36))?),
        },
    };
    if p.registration_mark {
        // just past the last ring, clear of the part when there is room
        let along = (end + tool.diameter).min(length);
        passes.push((vec![(work.point(along, 0.0, -MARK_DEPTH), 0.0)], false));
    }
    if let Some(stock) = Stock::of(&doc.job).filter(|_| p.strategy != Strategy::SidesThenDetails) {
        let only: Vec<Pass> = passes.into_iter().map(|(p, _)| p).collect();
        passes = plain(in_material(
            only,
            &stock,
            &Cutter::new(tool),
            along_x,
            radius,
        ));
    }
    let held_off = p.strategy != Strategy::Radial;
    let (moves, across) = joined(&passes, doc.job.turning_gap(), held_off, radius, along_x);
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

    fn cuts_of(doc: &Document, p: &Params) -> Vec<DVec3> {
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
        for c in cuts_of(&doc, &p).into_iter().filter(|c| c.x > 6.0) {
            assert!((c.z + 8.0).abs() < 0.05, "{c:?}");
        }
    }

    #[test]
    fn where_the_part_is_the_material_surface_the_bit_cuts_no_air() {
        // the lip is the material's own surface: nothing to cut there, so no ring goes round
        // it; the rest of the part is still followed all the way round
        let doc = lipped();
        let p = Params {
            tool: ball(6.0),
            ..Params::default()
        };
        let cuts = cuts_of(&doc, &p);
        let first = cuts.iter().map(|c| c.x).fold(f64::MAX, f64::min);
        let at: Vec<_> = cuts.iter().filter(|c| c.x < 1.0).take(5).collect();
        assert!(first > 1.0, "no ring on the lip: {first} {at:?}");
        assert!(cuts.iter().any(|c| c.x > 58.0));
        // held to one side, the same
        let off = Params {
            strategy: Strategy::OffCentre,
            ..p.clone()
        };
        assert!(cuts_of(&doc, &off).iter().all(|c| c.x > 1.0));
        // cut side by side, the bit only crosses the lip at the ends of its lines, from one
        // line over to the next: never further than two bit widths through the air
        let sides = Params {
            strategy: Strategy::Indexed,
            ..p
        };
        let toolpath = machine(&doc, &sides).unwrap();
        let (mut through, mut most) = (0.0, 0.0f64);
        for w in toolpath.moves.windows(2) {
            if w[1].kind == MoveKind::Cut && w[1].to.x < 1.0 {
                through += w[0].to.distance(w[1].to);
                most = most.max(through);
            } else {
                through = 0.0;
            }
        }
        assert!(most < 12.0, "{most}");
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
        // 60 mm in 1.5 mm steps: 41 rings, each plunging at A=0, but for the two at the
        // ends, where the bead is the material's own surface and there is nothing to cut
        assert_eq!(plunges.len(), 39);
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
        let round = Turned::of(Some(&surface), 20.0, true, (60.0, circumference), 0.1);
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
        for c in cuts_of(&doc, &p).into_iter().filter(|c| c.x > 6.0) {
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
        let turned = Turned::of(Some(&surface), 20.0, true, (60.0, circumference), 0.1);
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

    /// A rotary job 40 mm across, 60 long along X, holding a box off to one side of the
    /// axis as a full 3D model kept as it really is, heights and all.
    fn boxed() -> Document {
        let mut doc = Document {
            job: Job::rotary(60.0, 40.0, Along::X),
            ..Document::default()
        };
        let (lo, hi) = (DVec3::new(10.0, -6.0, -8.0), DVec3::new(50.0, 12.0, 10.0));
        let at = |i: usize| {
            DVec3::new(
                if i & 1 == 1 { hi.x } else { lo.x },
                if i & 2 == 2 { hi.y } else { lo.y },
                if i & 4 == 4 { hi.z } else { lo.z },
            )
        };
        let triangles: Vec<[DVec3; 3]> = [
            [0, 2, 3, 1],
            [4, 5, 7, 6],
            [0, 1, 5, 4],
            [2, 6, 7, 3],
            [0, 4, 6, 2],
            [1, 3, 7, 5],
        ]
        .iter()
        .flat_map(|&[a, b, c, d]| [[at(a), at(b), at(c)], [at(a), at(c), at(d)]])
        .collect();
        // its heights as an import wraps them: reaching the material's surface
        let mut model =
            Component::from_fn("Box", Combine::Add, doc.job.area(), 0.25, |_| Some(20.0));
        model.set_solid(&triangles);
        let mut c = Change::builder(&doc);
        c.add_new_component(model);
        doc.apply(&c.finish()).unwrap();
        doc
    }

    /// Each cut as the bit is held: along, A turned (radians), height above the axis, across.
    fn poses(toolpath: &Toolpath) -> Vec<(f64, f64, f64, f64)> {
        toolpath
            .moves
            .iter()
            .enumerate()
            .filter(|(_, m)| m.kind == MoveKind::Cut)
            .map(|(i, m)| {
                let d = toolpath.across.get(i).copied().unwrap_or(0.0);
                let (w, z) = rotary_axis::held_at(20.0, m.to.y, m.to.z, d);
                (m.to.x, w / 20.0, 20.0 + z, d)
            })
            .collect()
    }

    #[test]
    fn a_full_model_on_a_rotary_job_is_cut_round_as_it_really_is() {
        let doc = boxed();
        let mesh = Mesh::new(doc.components.values().next().unwrap().solid_triangles());
        for (strategy, offset) in [(Strategy::Radial, 0.0), (Strategy::OffCentre, 5.0)] {
            let p = Params {
                tool: ball(6.0),
                strategy,
                offset,
                ..Params::default()
            };
            let cutter = Cutter::new(&p.tool);
            let toolpath = machine(&doc, &p).unwrap();
            let mut on = 0;
            for (along, turn, height, across) in poses(&toolpath) {
                // exactly where the bit rests on the box, looked for on every triangle
                let rests = mesh
                    .drop_every(&cutter, along, across, turn)
                    .unwrap_or(0.0)
                    .clamp(0.0, 20.0);
                assert!(
                    (height - rests).abs() < 1e-6,
                    "{strategy:?} at {along}, {turn}: {height} against {rests}"
                );
                on += usize::from(rests > 1.0);
            }
            assert!(on > 100, "{strategy:?}: the box is cut round ({on})");
        }
    }

    #[test]
    fn an_allowance_leaves_at_least_that_much_on_a_full_model() {
        let doc = boxed();
        let mesh = Mesh::new(doc.components.values().next().unwrap().solid_triangles());
        let p = Params {
            tool: ball(6.0),
            allowance: 0.5,
            ..Params::default()
        };
        let cutter = Cutter::new(&p.tool);
        for (along, turn, height, across) in poses(&machine(&doc, &p).unwrap()) {
            if let Some(rests) = mesh.drop_every(&cutter, along, across, turn)
                && rests + 0.5 < 20.0
            {
                assert!(
                    height >= rests + 0.5 - 1e-6,
                    "{along}, {turn}: {height} vs {rests}"
                );
            }
        }
    }
}
