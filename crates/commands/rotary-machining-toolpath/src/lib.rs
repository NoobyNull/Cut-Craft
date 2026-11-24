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
//! with A held still, each side finished, then A locked at a few more angles, picked where
//! they reach the most of what is left, for the details. It keeps what is left of the blank as
//! it goes and never cuts air, and keeps the collet nut and spindle above the bit clear of the
//! material and of the chuck and tailstock: where they would not fit it leaves the material,
//! and says so in red.

use cut_craft_core::component::{Combine, placed_model};
use cut_craft_core::document::{Along, Document, Joined, Support};
use cut_craft_core::geometry::Point;
use cut_craft_core::glam::DVec3;
use cut_craft_core::progress;
use cut_craft_core::rotary_axis;
use cut_craft_core::rotary_axis::Surroundings;
use cut_craft_core::simulation::Material;
use cut_craft_core::toolpath::{
    Move, MoveKind, Tool, ToolDatabase, ToolShape, Toolpath, follow, made_by,
};
use cut_craft_core::{Change, Command, CommandError, Form, Meta, ensure_positive};
use cut_craft_rotary::{Clearance, Cutter, Head, Mesh, Pose, Stock, Voxels};
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
    /// the machine turns over itself; last, A locks at a few more angles, picked where they
    /// reach the most of what is left, for the details. The bit never cuts air, and the holder
    /// above it never touches the material, the chuck or the tailstock
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
    /// The most angles A locks at after the sides, for the details: each picked where it
    /// reaches the most of what the sides and the angles before it left. 0 for none
    #[schemars(extend("x-group" = "Strategy", "x-show-when" = {"strategy": "sides_then_details"}), range(min = 0, max = 12))]
    pub tilts: u32,
    /// The angles A locks at for the details, in degrees, in order (such as 15, 165, 45).
    /// Left empty, they are picked by themselves
    #[schemars(extend("x-group" = "Strategy", "x-show-when" = {"strategy": "sides_then_details"}))]
    pub tilt_angles: String,
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
    /// Cut away the centre stock Job Setup leaves along the axis (Holding the Part), down to
    /// the model: for the last toolpath, once everything else is cut. The ends the part is
    /// kept joined at stay as they are
    pub cut_support: bool,
    /// The holder above the bit, and the chuck and tailstock: filled in from Machine
    /// Configuration as the toolpath is made (the usual ones without)
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(skip)]
    pub surroundings: Option<Surroundings>,
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
            tilts: 6,
            tilt_angles: String::new(),
            start: 0.0,
            end: 0.0,
            allowance: 0.0,
            registration_mark: false,
            cut_support: false,
            surroundings: None,
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
            form: Form::Toolpath,
        }
    }

    fn run(doc: &Document, p: &Params) -> Result<Change, CommandError> {
        let (toolpath, notes) = made(doc, p)?;
        let mut c = Change::builder(doc);
        c.add_toolpath(toolpath);
        for note in notes {
            c.note(note);
        }
        Ok(c.finish())
    }
}

/// How far above where a stretch starts the bit comes down to at the rapid rate, when
/// nothing is in the way, before it plunges, mm.
const APPROACH: f64 = 1.0;

/// How deep the registration mark goes, mm.
const MARK_DEPTH: f64 = 0.5;

/// Why a registration mark asked for is not cut.
const NO_ROOM_FOR_MARK: &str = "No room for the registration mark past End without cutting \
    into the part, so it is left out: pencil a line along the material at A=0 before you \
    start, or bring End in from the end of the material.";

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
    /// than `cell`, each the highest of the surface under it so nothing is missed. Along and
    /// round count from the material's corner, `origin` in the job's X and Y.
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
        origin: Point,
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
                            Point::new(origin.x + a, origin.y + w)
                        } else {
                            Point::new(origin.x + w, origin.y + a)
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
    /// How far out the models are moved from where they really are: none.
    out: f64,
    /// Where the material starts along the axis in the job's X or Y: the models are kept
    /// along the job as it is, the toolpath works from the material's start.
    along_from: f64,
}

impl Solid {
    /// Every shown full 3D model of `doc` kept round the axis, raised or merged onto the
    /// rest (one sunk into the rest, or multiplying it, is left to its heights), moved `out`
    /// from the axis.
    fn of(doc: &Document, tool: &Tool, allowance: f64, out: f64, along_from: f64) -> Option<Self> {
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
            along_from,
        })
    }

    /// [`Turned::drop`] on the models themselves, A turned `turn` radians.
    fn drop(&self, along: f64, turn: f64, across: f64) -> f64 {
        self.mesh
            .drop(&self.cutter, along + self.along_from, across, turn)
            .map_or(0.0, |tip| tip + self.allowance + self.out)
    }
}

/// Whether a component is cut as its real 3D shape: a full model on a rotary job, raised
/// onto or merged with what is under it.
fn takes_solid(k: &cut_craft_core::component::Component) -> bool {
    !k.solid.is_empty() && matches!(k.combine, Combine::Add | Combine::Merge)
}

/// Segments round each round part of the support: fine enough that the bit stays within
/// a few thousandths of a millimetre of it, and never inside it.
const SUPPORT_SEGMENTS: usize = 96;

/// A round bar on the turning axis, `radius` out, from `from` to `to` along it (the job's X
/// or Y), ends closed: its sides just outside the circle, so nothing inside it is cut.
fn bar(radius: f64, from: f64, to: f64) -> Vec<[DVec3; 3]> {
    use std::f64::consts::TAU;
    #[expect(clippy::cast_precision_loss, reason = "a few segments")]
    let step = TAU / SUPPORT_SEGMENTS as f64;
    let out = radius / (step / 2.0).cos();
    let at = |along: f64, k: usize| {
        #[expect(clippy::cast_precision_loss, reason = "a few segments")]
        let (sin, cos) = (k as f64 * step).sin_cos();
        DVec3::new(along, out * sin, out * cos)
    };
    let (a, b) = (DVec3::new(from, 0.0, 0.0), DVec3::new(to, 0.0, 0.0));
    (0..SUPPORT_SEGMENTS)
        .flat_map(|k| {
            let n = k + 1;
            [
                [at(from, k), at(to, k), at(to, n)],
                [at(from, k), at(to, n), at(from, n)],
                [a, at(from, n), at(from, k)],
                [b, at(to, k), at(to, n)],
            ]
        })
        .collect()
}

/// What Job Setup's Holding the Part leaves of the material, as a model the bit is kept
/// off: the centre stock along the whole axis (unless it is being cut away), and the
/// material whole past the model (from `model`, along the material) at an end it stays
/// joined at. `None` when it leaves nothing.
fn supporting(
    support: Support,
    tool: &Tool,
    swing: f64,
    length: f64,
    along_from: f64,
    model: Option<(f64, f64)>,
    cut_beam: bool,
) -> Option<Solid> {
    let mut triangles = vec![];
    if support.beam > 0.0 && !cut_beam {
        triangles.extend(bar(support.beam / 2.0, along_from, along_from + length));
    }
    if let Some((first, last)) = model {
        // a little past the blank's corners, so all of it is left
        let whole = swing + 1.0;
        let (chuck, tailstock) = support.joined.ends();
        if chuck && first > 0.0 {
            triangles.extend(bar(whole, along_from, along_from + first));
        }
        if tailstock && last < length {
            triangles.extend(bar(whole, along_from + last, along_from + length));
        }
    }
    (!triangles.is_empty()).then(|| Solid {
        mesh: Mesh::new(triangles),
        cutter: Cutter::new(tool),
        allowance: 0.0,
        out: 0.0,
        along_from,
    })
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
    /// What is left of the material to hold the part ([`supporting`]): kept like a model.
    support: Option<&'a Solid>,
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
    /// What lengths are said in.
    units: cut_craft_core::document::Units,
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

    /// The registration mark `along` the part at A=0, 0.5 mm deep: none where the part is
    /// less than that below the top there, as the notch would cut into it.
    fn mark(&self, along: f64) -> Option<Pass> {
        let floor = self.tip(along, 0.0, 0.0) - self.turned.radius;
        (floor <= -MARK_DEPTH).then(|| vec![(self.point(along, 0.0, -MARK_DEPTH), 0.0)])
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
        for solid in self.solid.iter().chain(&self.support) {
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

    /// Bulk to fine detail ([`Strategy::SidesThenDetails`]): each side cleared a level at a
    /// time down to the roughing allowance above the model, A held still; then each side
    /// finished; then A locked at more angles for the details, as `details` says. What is
    /// left of the `stock` is kept as the passes are worked out, and any stretch where the bit
    /// would cut nothing is left out, or where the holder above it would touch the material,
    /// the chuck or the tailstock. Also says where the holder kept the bit from material it
    /// should have cut, and the angles locked at for the details.
    #[expect(
        clippy::too_many_lines,
        reason = "bulk, finish and details worked out in turn"
    )]
    fn sides_then_details(
        &self,
        stock: &Stock,
        details: &Details,
    ) -> Result<(Vec<(Pass, bool)>, Outcome), CommandError> {
        let Details {
            faces,
            roughing_allowance,
            head,
            ..
        } = details;
        let faces = *faces;
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
        // a side is finished wherever there is any within a cell of it, however thin; an
        // angle for the details is picked where it finds a few cells' worth
        let cell = material.cell();
        let rough = Check::new(tool, depth + cell, 0.0, 1, cell);
        let fine = Check::new(tool, d, cell, 1, cell);
        let detail = Check::new(tool, d, 0.0, DETAIL, cell);
        // the holder's room, worked out a cell or a millimetre at a time, whichever is coarser
        let room = cell.max(1.0);
        let swing = stock.swing();
        let mut passes = vec![];
        let mut refused = vec![];
        let turn = |round: f64| round / radius;
        let steps = 2 * faces as usize + details.tilts as usize;

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
                // the holder's room as the levels before left it
                let clear = Clearance::of(&material, turn(round), head, room);
                // what the holder keeps the bulk from is left for the finishes after
                passes.extend(self.cutting(
                    &pass,
                    &mut material,
                    &rough,
                    Some(&clear),
                    &mut vec![],
                ));
            }
            progress::report_steps(face as usize + 1, steps);
        }

        progress::stage("Finishing each side");
        for face in 0..faces {
            progress::stop_if_cancelled()?;
            let round = self.circumference * f64::from(face) / f64::from(faces);
            passes.extend(self.locked(round, &mut material, &fine, head, room, &mut refused));
            progress::report_steps((faces + face) as usize + 1, steps);
        }

        progress::stage("Locking A for the details");
        let sides: Vec<f64> = (0..faces)
            .map(|face| 360.0 * f64::from(face) / f64::from(faces))
            .collect();
        let mut tilts: Vec<f64> = vec![];
        for k in 0..details.tilts as usize {
            progress::stop_if_cancelled()?;
            let next = if let Some(angles) = &details.angles {
                angles.get(k).copied()
            } else {
                let tried: Vec<f64> = sides.iter().chain(&tilts).copied().collect();
                self.best_tilt(&material, &detail, head, &tried)
            };
            let Some(angle) = next else { break };
            let round = angle.to_radians() * radius;
            passes.extend(self.locked(round, &mut material, &fine, head, room, &mut refused));
            tilts.push(angle);
            progress::report_steps(2 * faces as usize + k + 1, steps);
        }
        progress::report_steps(steps, steps);
        // where the holder kept the bit off, and something is still left for it there now
        // (not in the chuck or tailstock: that is meant to be left)
        let left: Vec<Pose> = refused
            .into_iter()
            .filter(|p: &Pose| !head.held(p.along))
            .filter(|&p| {
                let looking = Pose {
                    height: p.height - fine.margin,
                    ..p
                };
                material.held(&fine.looks, looking, DETAIL) >= DETAIL
            })
            .collect();
        Ok((passes, Outcome { left, tilts }))
    }

    /// A finish of the part with A locked at `round` (mm round the surface): lines back and
    /// forth along it right across the blank (a board is wider than it is thick, and what is
    /// air is left out anyway), cut where `check` finds material and the holder has room.
    fn locked(
        &self,
        round: f64,
        material: &mut Voxels,
        check: &Check,
        head: &Head,
        room: f64,
        refused: &mut Vec<Pose>,
    ) -> Vec<(Pass, bool)> {
        let radius = self.turned.radius;
        let half = material.swing();
        let pass = self.side(half, self.step, self.turned.along_cell, |along, across| {
            (
                self.point(along, round, self.tip(along, round, across) - radius),
                across,
            )
        });
        let clear = Clearance::of(material, round / radius, head, room);
        self.cutting(&pass, material, check, Some(&clear), refused)
    }

    /// The angle (degrees) A locked at reaches the most of what is left of the `material`
    /// for a finish, the holder kept clear, of every 15° but those `tried`: none when no
    /// angle reaches enough to be worth it. Looked at a bit's width apart.
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        clippy::cast_precision_loss,
        reason = "line and point counts are small"
    )]
    fn best_tilt(
        &self,
        material: &Voxels,
        check: &Check,
        head: &Head,
        tried: &[f64],
    ) -> Option<f64> {
        let radius = self.turned.radius;
        let step = self.tool.diameter;
        let half = material.swing();
        let lines = ((2.0 * half / step).ceil() as usize).max(1) + 1;
        let points = ((self.end - self.start) / step).ceil() as usize + 1;
        let near =
            |a: f64, b: f64| ((a - b).rem_euclid(360.0)).min((b - a).rem_euclid(360.0)) < 1.0;
        // every angle at once, each worked through on its own thread
        let scores = parallel(24, |k| {
            let angle = 15.0 * f64::from(u32::try_from(k).unwrap_or(0));
            if tried.iter().any(|&t| near(t, angle)) {
                return (angle, 0);
            }
            let round = angle.to_radians() * radius;
            // coarser than the finish's own: only to choose by
            let clear = Clearance::of(
                material,
                round / radius,
                head,
                (2.0 * material.cell()).max(2.0),
            );
            let found: usize = (0..lines)
                .map(|line| {
                    let across = -half + line as f64 * 2.0 * half / (lines - 1) as f64;
                    (0..points)
                        .map(|i| {
                            let along = (self.start + i as f64 * step).min(self.end);
                            if head.held(along) {
                                return 0;
                            }
                            let tip = self.tip(along, round, across);
                            let pose = Pose::new(along, across, tip, round / radius);
                            if !clear.allows(pose) {
                                return 0;
                            }
                            let n = material.held(&check.looks, pose, 64);
                            if n >= check.enough { n } else { 0 }
                        })
                        .sum::<usize>()
                })
                .sum();
            (angle, found)
        });
        let mut best: Option<(f64, usize)> = None;
        for (angle, found) in scores {
            if found > best.map_or(0, |b| b.1) {
                best = Some((angle, found));
            }
        }
        // a few points' worth at least: less is slivers, not detail
        best.filter(|b| b.1 >= 3 * DETAIL * 8).map(|b| b.0)
    }

    /// `pass` broken where the bit would cut too little of what is left of the `material`
    /// to be worth it (as `check` says), which loses what the rest cuts; and, with a
    /// `clear`ance, where the holder would touch the material, the chuck or the tailstock
    /// (those points, where the bit would have cut, go in `refused`).
    fn cutting(
        &self,
        pass: &[(DVec3, f64)],
        material: &mut Voxels,
        check: &Check,
        clear: Option<&Clearance>,
        refused: &mut Vec<Pose>,
    ) -> Vec<(Pass, bool)> {
        let Check {
            cutter,
            looks,
            above,
            margin,
            enough,
        } = check;
        let mut cuts = Vec::with_capacity(pass.len());
        let mut blocked = Vec::with_capacity(pass.len());
        // where a stretch may start, whether the bit can come straight down to it quickly:
        // nothing of the material anywhere above it, just before it goes there
        let mut quick = vec![false; pass.len()];
        let mut last: Option<Pose> = None;
        for (i, point) in pass.iter().enumerate() {
            let pose = self.pose(point);
            let fits = clear.is_none_or(|c| c.allows(pose));
            blocked.push(!fits);
            let looking = Pose {
                height: pose.height - margin,
                ..pose
            };
            let mut cutting = material.held(looks, looking, *enough) >= *enough;
            if cutting && !fits {
                refused.push(pose);
                cutting = false;
            }
            if cutting {
                if last.is_none() {
                    let start = if i > 0 && !blocked[i - 1] { i - 1 } else { i };
                    quick[start] = material.held(above, self.pose(&pass[start]), 1) == 0;
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
        let found = runs(pass, &cuts, &blocked, 2.0 * self.tool.diameter);
        found
            .into_iter()
            .map(|run| (pass[run.clone()].to_vec(), quick[run.start]))
            .collect()
    }
}

/// How bulk to fine detail is to go: how many sides, what the bulk leaves, how many angles
/// A locks at for the details (and which, when they are given), and the holder, chuck and
/// tailstock to keep clear of.
struct Details {
    faces: u32,
    roughing_allowance: f64,
    tilts: u32,
    angles: Option<Vec<f64>>,
    head: Head,
}

/// What bulk to fine detail found as it went: where the holder kept the bit from material
/// still left for it, and the angles A locked at for the details (degrees).
#[derive(Debug, Default)]
struct Outcome {
    left: Vec<Pose>,
    tilts: Vec<f64>,
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

/// The model in two parts: the rest as heights placed in the material where Material
/// Setup puts them (`None` when there is none), and the full 3D models cut round exactly
/// where they really are in space: Material Setup's model position is a flat job's, and
/// never moves them.
fn parts(
    doc: &Document,
    tool: &Tool,
    allowance: f64,
) -> Result<(Option<Material>, Option<Solid>), CommandError> {
    let corner = doc.job.area().min;
    let along_from = match doc.job.rotary.map(|r| r.along) {
        Some(Along::Y) => corner.y,
        _ => corner.x,
    };
    let Some(solid) = Solid::of(doc, tool, allowance, 0.0, along_from) else {
        return Ok((Some(placed_model(doc, tool)?), None));
    };
    let rest = Document {
        components: doc
            .components
            .iter()
            .filter(|(_, k)| !takes_solid(k))
            .map(|(id, k)| (*id, k.clone()))
            .collect(),
        ..doc.clone()
    };
    let rest = if rest.components.is_empty() {
        None
    } else {
        Some(placed_model(&rest, tool)?)
    };
    Ok((rest, Some(solid)))
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
            runs(&pass, &cutting, &[], short)
                .into_iter()
                .map(|run| pass[run].to_vec())
                .collect::<Vec<_>>()
        })
        .collect()
}

/// The stretches of `pass` where `cutting` says the bit cuts something, each with one
/// point of air kept at either end, so the bit comes onto the material and leaves it at its
/// cutting height; gaps shorter than `short` are cut through rather than lifted over. A point
/// `blocked` (the holder would touch something there; none when it is empty) is never kept.
fn runs(
    pass: &[(DVec3, f64)],
    cutting: &[bool],
    blocked: &[bool],
    short: f64,
) -> Vec<std::ops::Range<usize>> {
    let free = |i: usize| !blocked.get(i).copied().unwrap_or(false);
    let mut keep = cutting.to_vec();
    for (i, &c) in cutting.iter().enumerate() {
        if c {
            if i > 0 && free(i - 1) {
                keep[i - 1] = true;
            }
            if i + 1 < keep.len() && free(i + 1) {
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
        if from > 0 && i < keep.len() && (from..i).all(free) {
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

/// The angles typed in Tilt angles, in degrees: none when nothing is typed (they are picked),
/// and, in red, any word that is not an angle (left out; none left, they are picked).
fn angles(typed: &str) -> (Option<Vec<f64>>, Option<String>) {
    let mut good = vec![];
    let mut bad = vec![];
    for w in typed
        .split(|c: char| c == ',' || c == ';' || c.is_whitespace())
        .map(|w| w.trim_end_matches(['°', 'º']))
        .filter(|w| !w.is_empty())
    {
        match w.parse::<f64>().ok().filter(|a| a.is_finite()) {
            Some(a) => good.push(a.rem_euclid(360.0)),
            None => bad.push(format!("\"{w}\"")),
        }
    }
    let note = (!bad.is_empty()).then(|| {
        format!(
            "Tilt angles: {} {} not an angle in degrees, so {} left out{}. Type angles such as \
             15, 165, 45, or leave it empty to have them picked.",
            bad.join(", "),
            if bad.len() == 1 { "is" } else { "are" },
            if bad.len() == 1 { "it is" } else { "they are" },
            if good.is_empty() {
                " and the angles are picked"
            } else {
                ""
            },
        )
    });
    ((!good.is_empty()).then_some(good), note)
}

/// Where along the part the model is, from the material's start: its first point to its
/// last.
fn model_along(work: &Work) -> Option<(f64, f64)> {
    // the full models are kept in the job's own X and Y: counted from the material's start
    if let Some(solid) = work.solid
        && let Some((lo, hi)) = solid.mesh.bounds()
    {
        return Some((lo.x - solid.along_from, hi.x - solid.along_from));
    }
    let t = work.turned;
    let mut found: Option<(f64, f64)> = None;
    for i in 0..t.along {
        if (0..t.round)
            .any(|j| t.radii[j * t.along + i] > 1e-9 && t.radii[j * t.along + i] < t.radius - 1e-9)
        {
            #[expect(clippy::cast_precision_loss, reason = "cell counts are small")]
            let (a, b) = (i as f64 * t.along_cell, (i + 1) as f64 * t.along_cell);
            found = Some(found.map_or((a, b), |(lo, hi)| (lo.min(a), hi.max(b))));
        }
    }
    found
}

/// In red, when the model is in the chuck or tailstock, or so near them that the collet nut
/// and spindle can't get down beside it: how much room to leave.
fn too_near(
    work: &Work,
    head: &Head,
    around: &Surroundings,
    length: f64,
    joined: Joined,
) -> Vec<String> {
    let Some((first, last)) = model_along(work) else {
        return vec![];
    };
    let units = work.units;
    let room = head.widest() + 2.0;
    let (free_from, free_to) = head.free();
    let mut notes = vec![];
    // an end the part stays joined at is not cut beside the model: no room needed there
    let (at_chuck, at_tailstock) = joined.ends();
    if around.chuck > 0.0 && !at_chuck && first < free_from + room {
        notes.push(format!(
            "The model starts {} from the job's start, but the chuck holds the first {} \
             (Machine Configuration's rotary settings), and the collet nut and spindle, {} \
             across, can't get down beside it. Leave at least {} between the chuck and the \
             model: move the model along, or make the material longer.",
            units.length(first, 1),
            units.length(around.chuck, 1),
            units.length(2.0 * head.widest(), 0),
            units.length(room, 0),
        ));
    }
    if around.tailstock > 0.0 && !at_tailstock && last > free_to - room {
        notes.push(format!(
            "The model ends {} from the job's end, but the tailstock takes the last {}, and \
             the collet nut and spindle can't get down beside it. Leave at least {} between \
             the model and the tailstock.",
            units.length((length - last).max(0.0), 1),
            units.length(around.tailstock, 1),
            units.length(room, 0),
        ));
    }
    notes
}

/// In red, when the holder kept the bit from some of the model at every angle tried: where
/// along the part, and what would reach it.
fn left_note(
    outcome: &Outcome,
    tool: &Tool,
    around: &Surroundings,
    units: cut_craft_core::document::Units,
) -> Option<String> {
    if outcome.left.is_empty() {
        return None;
    }
    let (from, to) = outcome
        .left
        .iter()
        .fold((f64::INFINITY, f64::NEG_INFINITY), |(lo, hi), p| {
            (lo.min(p.along), hi.max(p.along))
        });
    let tilts = if outcome.tilts.is_empty() {
        String::new()
    } else {
        let list: Vec<String> = outcome.tilts.iter().map(|a| format!("{a:.0}°")).collect();
        format!(" (A locked at {} for the details)", list.join(", "))
    };
    // the reach was taken, not given: the likeliest thing to put right
    let (stickout, guessed) = cut_craft_core::collision::stickout(tool);
    let guess = if guessed {
        format!(
            " The Tool Database doesn't say how far this bit sticks out of the collet, so {} \
             was taken: set its stick-out there if it sticks out further.",
            units.length(stickout, 0)
        )
    } else {
        String::new()
    };
    Some(format!(
        "The collet nut ({} across) and spindle can't get down to some of the model at any \
         angle tried{tilts}: it is left rough between {} and {} along the part. A bit that \
         sticks out further, a slimmer holder, or the model sliced into parts would reach it.\
         {guess}",
        units.length(around.holder.nut_diameter, 0),
        units.length(from, 1),
        units.length(to, 1),
    ))
}

/// The toolpath: rings round the part at each step along it, each from A=0 all the way round
/// (over the axis, or held to one side of it), or each side in turn cut back and forth with A
/// held still; the bit as low as it goes without touching the model.
#[cfg(test)]
fn machine(doc: &Document, p: &Params) -> Result<Toolpath, CommandError> {
    made(doc, p).map(|(toolpath, _)| toolpath)
}

/// [`machine`], and what it has to say in red: where the holder kept the bit from the model,
/// and a model too near the chuck or tailstock.
#[expect(
    clippy::too_many_lines,
    reason = "the strategies set up and joined in one place"
)]
fn made(doc: &Document, p: &Params) -> Result<(Toolpath, Vec<String>), CommandError> {
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
    let (rest, solid) = parts(doc, tool, p.allowance)?;
    // as fine as the model's own cells (as placing the model makes them), or the bit's
    let cell = (tool.diameter / 16.0).max(
        rest.as_ref()
            .map_or((tool.diameter / 20.0).max(doc.job.model_cell()), |m| m.cell),
    );
    let turned = Turned::of(
        rest.as_ref(),
        radius,
        along_x,
        (length, circumference),
        cell,
        doc.job.area().min,
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
        units: doc.job.units,
        support: None,
    };
    // what Job Setup leaves to hold the part, kept like a model
    let holding = supporting(
        rotary.support,
        tool,
        work.top,
        length,
        if along_x {
            doc.job.area().min.x
        } else {
            doc.job.area().min.y
        },
        model_along(&work),
        p.cut_support,
    );
    let work = Work {
        support: holding.as_ref(),
        ..work
    };
    let plain = |passes: Vec<Pass>| passes.into_iter().map(|p| (p, false)).collect();
    let mut notes = vec![];
    // what of the model it can't cut: kept with the toolpath, for the checks before cutting
    let mut kept: Vec<String> = vec![];
    // the holder, chuck and tailstock, where the toolpath keeps clear of them
    let mut kept_clear: Option<Head> = None;
    // each pass, and whether the bit can come down to it at the rapid rate
    let mut passes: Vec<(Pass, bool)> = match p.strategy {
        Strategy::Radial => plain(work.rings(0.0)?),
        Strategy::OffCentre => plain(work.rings(p.offset)?),
        Strategy::Indexed => plain(work.sides(p.faces.clamp(1, 36))?),
        Strategy::SidesThenDetails => match Stock::of(&doc.job) {
            Some(stock) => {
                let around = p.surroundings.unwrap_or_default();
                let (typed, wrong) = angles(&p.tilt_angles);
                notes.extend(wrong);
                let details = Details {
                    faces: p.faces.clamp(1, 36),
                    roughing_allowance: p.roughing_allowance,
                    tilts: p.tilts.min(12),
                    angles: typed,
                    head: Head::new(tool, &around, length, stock.swing()),
                };
                kept.extend(too_near(
                    &work,
                    &details.head,
                    &around,
                    length,
                    rotary.support.joined,
                ));
                let (passes, outcome) = work.sides_then_details(&stock, &details)?;
                kept.extend(left_note(&outcome, tool, &around, doc.job.units));
                kept_clear = Some(details.head);
                passes
            }
            None => plain(work.sides(p.faces.clamp(1, 36))?),
        },
    };
    if p.registration_mark {
        // just past the last ring, clear of the chuck and tailstock, and only where the part
        // is at least the mark's depth below the top
        let along = (end + tool.diameter).min(length);
        let reach = tool.diameter / 2.0;
        match &kept_clear {
            Some(head) if head.held(along - reach) || head.held(along + reach) => {
                notes.push(
                    "No room for the registration mark clear of the chuck and tailstock, so it \
                     is left out: pencil a line along the material at A=0 before you start, or \
                     bring End in from the tailstock."
                        .into(),
                );
            }
            _ => match work.mark(along) {
                Some(mark) => passes.push((mark, false)),
                None => notes.push(NO_ROOM_FOR_MARK.to_owned()),
            },
        }
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
    let (mut moves, across) = joined(&passes, doc.job.turning_gap(), held_off, radius, along_x);
    // worked out from the material's corner; written where the material is in the job, as
    // the job's datum puts it
    let corner = doc.job.area().min;
    for m in &mut moves {
        m.to.x += corner.x;
        m.to.y += corner.y;
    }
    let mut toolpath =
        made_by::<RotaryMachiningToolpath>(p, p.name.clone(), tool.clone(), vec![], moves);
    toolpath.across = across;
    if !kept.is_empty() {
        toolpath.params[cut_craft_core::toolpath::MADE_NOTES] = kept.clone().into();
    }
    notes.extend(kept);
    Ok((toolpath, notes))
}

#[cfg(test)]
mod tests;
