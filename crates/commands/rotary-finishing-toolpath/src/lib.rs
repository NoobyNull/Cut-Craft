//! Rotary Finishing: the last pass over a carved part on a rotary job, the bit kept on the
//! model all the way round, as low as it goes without touching it.
//!
//! Round the part (the usual), A turns the bit all the way round the part, then it steps
//! along; Along the part, the bit runs the whole length with A held still, then A turns a
//! step and it runs back. The bit can be held to one side of the turning axis (Off centre),
//! so the side of the ball cuts rather than its slow-turning tip, from one side or from both
//! in turn. It cuts only what is left: what the toolpaths above it in the list leave of the
//! blank (unless Settings' advanced rotary mode says to finish the whole blank), so it never
//! cuts air. Where the collet nut and spindle can't get down beside what is left, or the
//! chuck or tailstock is in the way, it stays up there and says so in red.

use cut_craft_core::component::MachiningBoundary;
use cut_craft_core::document::Document;
use cut_craft_core::rotary_axis::Surroundings;
use cut_craft_core::toolpath::{MADE_NOTES, Tool, ToolDatabase, ToolShape, Toolpath, made_by};
use cut_craft_core::{Change, Command, CommandError, Form, Meta, Needs};
use cut_craft_rotary::{Head, Stock};
use cut_craft_rotary_carving::{
    Carving, CutDirection, Finish, Limit, NO_ROOM_FOR_MARK, Pass, Reach, Way, Work, finishing_step,
    joined, limit_vectors, too_near,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub struct RotaryFinishingToolpath;

/// Where the bit is held over the part.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Held {
    /// Straight over the turning axis: the tip of the bit cuts
    #[default]
    OverTheAxis,
    /// A set distance to one side of the turning axis: the side of the ball cuts, not its
    /// slow-turning tip, for a cleaner finish, and walls that face sideways are reached
    OffCentre,
    /// A and the bit's place across the axis change together, so the bit meets the model
    /// square on: flat faces are cut with A still, steep sides by the side of the ball
    FollowsTheSurface,
}

/// Which sides of the axis an off-centre finish is held to.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Sides {
    /// Held to one side for the whole finish
    #[default]
    OneSide,
    /// Held to one side for the whole part, then to the other: walls facing either way are
    /// reached, in twice the time
    BothSides,
}

/// The last pass over a carved part, the bit on the model all the way round: only what the
/// toolpaths above it in the list leave.
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(default)]
pub struct Params {
    /// What the toolpath is called in the toolpath list
    pub name: String,
    /// The bit: a ball nose follows curves best. Its finishing stepover spaces the passes
    pub tool: Tool,
    /// Which way the bit goes over the part: round it, stepping along between passes, or
    /// along it, A turning a step between passes
    #[schemars(
        title = "Direction",
        extend(
            "x-group" = "Strategy",
            "x-widget" = "named-icons",
            "x-icon" = "toolpaths/rotary_way_{}.png"
        )
    )]
    pub way: Way,
    /// Climb or Conventional cut every pass the same way, lifting back to start each one;
    /// Both cuts back and forth, staying down
    #[schemars(title = "Cut direction", extend("x-group" = "Strategy"))]
    pub cut_direction: CutDirection,
    /// Where the bit is held: over the turning axis, to one side of it, or following the
    /// surface
    #[schemars(title = "Bit held", extend("x-group" = "Strategy"))]
    pub held: Held,
    /// How far to one side of the turning axis the bit is held. Positive is the way the part
    /// comes up towards the bit as A turns back
    #[schemars(extend("x-unit" = "mm", "x-group" = "Strategy", "x-show-when" = {"held": "off_centre"}))]
    pub offset: f64,
    /// Held to one side for the whole finish, or to one side then the other
    #[schemars(extend("x-group" = "Strategy", "x-show-when" = {"held": "off_centre"}))]
    pub sides: Sides,
    /// Material left on the part for a later pass, all the way round
    #[schemars(extend("x-unit" = "mm"), range(min = 0))]
    pub allowance: f64,
    /// Where along the axis the finish starts, from the job's start (the chuck end)
    #[schemars(extend("x-unit" = "mm", "x-group" = "Along the part"), range(min = 0))]
    pub start: f64,
    /// Where along the axis the finish ends, from the job's start; 0 runs to the job's end
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
    /// A small notch at A=0, just past the end of the finish, so the part can be put back
    /// exactly as it was if a job has to be started again
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
    /// Settings' advanced rotary mode, filled in as the toolpath is made: on, the whole blank
    /// is finished, not only what the toolpaths above it leave
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    #[schemars(skip)]
    pub whole_blank: bool,
}

impl Default for Params {
    fn default() -> Self {
        Self {
            name: "Rotary Finishing".into(),
            tool: ToolDatabase::starter()
                .tools
                .into_iter()
                .find(|t| t.shape == ToolShape::BallNose)
                .expect("the starter tools include a ball nose"),
            way: Way::RoundThePart,
            cut_direction: CutDirection::Both,
            held: Held::OverTheAxis,
            offset: 3.0,
            sides: Sides::OneSide,
            allowance: 0.0,
            start: 0.0,
            end: 0.0,
            machining_boundary: MachiningBoundary::Material,
            boundary_level: String::new(),
            machining_boundary_offset: 0.0,
            registration_mark: false,
            cut_support: false,
            surroundings: None,
            whole_blank: false,
        }
    }
}

impl Command for RotaryFinishingToolpath {
    const ID: &'static str = "rotary_finishing_toolpath";
    type Params = Params;

    fn meta() -> Meta {
        Meta {
            label: "Rotary Finishing",
            group: "3D Toolpaths",
            icon: "toolpaths/rotary_finishing_toolpath.png",
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

/// Why nothing was finished, when nothing was.
const NOTHING_LEFT: &str = "Nothing is left to finish between Start and End: the toolpaths \
    above this one already take the material down to the model. Move this toolpath further up \
    the list, or leave it out.";

/// How far across the bit is held for each run over the whole part.
fn held(p: &Params) -> Vec<f64> {
    match (p.held, p.sides) {
        (Held::OverTheAxis | Held::FollowsTheSurface, _) => vec![0.0],
        (Held::OffCentre, Sides::OneSide) => vec![p.offset],
        (Held::OffCentre, Sides::BothSides) => vec![p.offset, -p.offset],
    }
}

/// The toolpath, and what it has to say in red.
fn made(doc: &Document, p: &Params) -> Result<(Toolpath, Vec<String>), CommandError> {
    let tool = &p.tool;
    let Some(stock) = doc.job.rotary.and_then(|_| Stock::of(&doc.job)) else {
        return Err(CommandError::Failed(
            "this toolpath turns the material: set the job up as rotary in Job Setup".into(),
        ));
    };
    let carving = Carving::of(
        doc,
        tool,
        Reach {
            start: p.start,
            end: p.end,
            step: finishing_step(tool)?,
            allowance: p.allowance.max(0.0),
            cut_support: p.cut_support,
        },
    )?;
    let carving = carving.limited(Limit::of(
        doc,
        p.machining_boundary,
        p.machining_boundary_offset,
        &p.boundary_level,
    )?)?;
    let work = carving.work();
    let corner = carving.corner(doc);
    let mut material = carving.left_of(doc, &stock, p.whole_blank);
    let around = p.surroundings.unwrap_or_default();
    let head = Head::new(tool, &around, carving.length, stock.swing());
    // what of the model it can't cut: kept with the toolpath, for the checks before cutting
    let mut kept = too_near(
        &work,
        &head,
        &around,
        carving.length,
        carving.rotary.support.joined,
    );
    let mut notes = vec![];
    let mark_head = head.clone();
    let finished = work.finishing(
        &mut material,
        &Finish {
            way: p.way,
            direction: p.cut_direction,
            held: held(p),
            head,
            follow: p.held == Held::FollowsTheSurface,
        },
    )?;
    if !finished.kept_off.is_empty() {
        kept.push(kept_off_note(&around, &finished.kept_off, carving.units));
    }
    let mut passes = finished.passes;
    if passes.is_empty() {
        notes.push(NOTHING_LEFT.to_owned());
    }
    if p.registration_mark {
        // just past the end of the finish
        let along = (carving.end + tool.diameter).min(carving.length);
        match mark(&work, &mark_head, along) {
            Ok(mark) => passes.push((mark, false)),
            Err(why) => notes.push(why),
        }
    }
    let held_off = p.held != Held::OverTheAxis;
    let (mut moves, across) = joined(
        &passes,
        doc.job.turning_gap(),
        held_off,
        carving.radius,
        carving.along_x,
    );
    // worked out from the material's corner; written where the material is in the job, as
    // the job's datum puts it
    for m in &mut moves {
        m.to.x += corner.x;
        m.to.y += corner.y;
    }
    let mut toolpath = made_by::<RotaryFinishingToolpath>(
        p,
        p.name.clone(),
        tool.clone(),
        limit_vectors(doc, p.machining_boundary),
        moves,
    );
    toolpath.across = across;
    if !kept.is_empty() {
        toolpath.params[MADE_NOTES] = kept.clone().into();
    }
    notes.extend(kept);
    Ok((toolpath, notes))
}

/// The registration mark `along` the part, clear of the chuck and tailstock, and only where
/// the part is at least the mark's depth below the top; else why it is left out.
fn mark(work: &Work<'_>, head: &Head, along: f64) -> Result<Pass, String> {
    let reach = work.tool.diameter / 2.0;
    if head.held(along - reach) || head.held(along + reach) {
        return Err(
            "No room for the registration mark clear of the chuck and tailstock, so it \
             is left out: pencil a line along the material at A=0 before you start, or bring \
             End in from the tailstock."
                .into(),
        );
    }
    work.mark(along).ok_or_else(|| NO_ROOM_FOR_MARK.to_owned())
}

/// In red: where along the part the holder kept the bit off material it should have cut.
fn kept_off_note(
    around: &Surroundings,
    kept_off: &[f64],
    units: cut_craft_core::document::Units,
) -> String {
    let (from, to) = kept_off
        .iter()
        .fold((f64::INFINITY, f64::NEG_INFINITY), |(a, b), &at| {
            (a.min(at), b.max(at))
        });
    format!(
        "The collet nut ({} across) and spindle can't get down beside the material left, or \
         the chuck or tailstock beside the cut: between {} and {} along the part the bit stays \
         up and leaves some of the model uncut. Rough the material beside it down first, keep \
         Start and End further from the chuck and tailstock, or use a bit that sticks out \
         further.",
        units.length(around.holder.nut_diameter, 0),
        units.length(from, 1),
        units.length(to, 1),
    )
}

#[cfg(test)]
mod tests;
