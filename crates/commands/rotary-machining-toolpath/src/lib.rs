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

use cut_craft_core::component::MachiningBoundary;
use cut_craft_core::document::Document;
use cut_craft_core::rotary_axis::Surroundings;
use cut_craft_core::toolpath::{Tool, ToolDatabase, ToolShape, Toolpath, made_by};
use cut_craft_core::{Change, Command, CommandError, Form, Meta, Needs, ensure_positive};
use cut_craft_rotary::{Cutter, Head, Stock};
use cut_craft_rotary_carving::{
    Carving, Details, Limit, NO_ROOM_FOR_MARK, Pass, Reach, angles, in_material, joined, left_note,
    limit_vectors, too_near,
};
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
    /// Where on the material the bit may cut: all of it, or within the model's outline, a
    /// level's, or the selected closed vectors. Drawn on the job, they wrap round the part
    #[serde(default)]
    #[schemars(title = "Machining limit", extend("x-group" = "Machining Limit Boundary"))]
    pub machining_boundary: MachiningBoundary,
    /// The level whose components make the limit
    #[serde(default)]
    #[schemars(title = "Level", extend("x-show-when" = {"machining_boundary": "selected_level"}, "x-group" = "Machining Limit Boundary"))]
    pub boundary_level: String,
    /// Grows the limit outward by this much (negative shrinks it)
    #[serde(default)]
    #[schemars(title = "Boundary Offset", extend("x-unit" = "mm", "x-group" = "Machining Limit Boundary", "x-hide-when" = {"machining_boundary": "material"}))]
    pub machining_boundary_offset: f64,
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
            machining_boundary: MachiningBoundary::Material,
            boundary_level: String::new(),
            machining_boundary_offset: 0.0,
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
            form: Form::Toolpath {
                needs: Needs::Rotary,
            },
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
    let carving = Carving::of(
        doc,
        tool,
        Reach {
            start: p.start,
            end: p.end,
            step: stepover * tool.diameter,
            allowance: p.allowance,
            cut_support: p.cut_support,
        },
    )?;
    let carving = carving.limited(Limit::of(
        doc,
        p.machining_boundary,
        p.machining_boundary_offset,
        &p.boundary_level,
    )?)?;
    let (along_x, length, radius, end) =
        (carving.along_x, carving.length, carving.radius, carving.end);
    let work = carving.work();
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
    if let Some(limit) = &carving.limit {
        passes = limit.keep(along_x, passes);
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
    let mut toolpath = made_by::<RotaryMachiningToolpath>(
        p,
        p.name.clone(),
        tool.clone(),
        limit_vectors(doc, p.machining_boundary),
        moves,
    );
    toolpath.across = across;
    if !kept.is_empty() {
        toolpath.params[cut_craft_core::toolpath::MADE_NOTES] = kept.clone().into();
    }
    notes.extend(kept);
    Ok((toolpath, notes))
}

#[cfg(test)]
mod tests;
