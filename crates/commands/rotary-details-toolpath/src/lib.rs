//! Rotary Details: on a rotary job, a smaller bit after the finish, cutting only what the
//! bigger bits before it could not reach: the grooves, corners and fine detail they left.
//!
//! Each bit that finished before is run over the part as a finish all the way round, in
//! thought, so what it reached is taken off; the smaller bit then goes round the part ring by
//! ring wherever some is still left, and last A locks at the angles that reach the most of
//! what the rings could not, as many as it is allowed, stopping when nothing worth another is
//! left. Like the other carving toolpaths it also cuts only what the toolpaths above it in the
//! list leave (unless Settings' advanced rotary mode is on), and never cuts air.

use cut_craft_core::document::Document;
use cut_craft_core::rotary_axis::Surroundings;
use cut_craft_core::toolpath::{MADE_NOTES, Tool, ToolDatabase, ToolShape, Toolpath, made_by};
use cut_craft_core::{Change, Command, CommandError, Form, Meta, Needs, ensure_positive};
use cut_craft_rotary::{Head, Stock, Voxels};
use cut_craft_rotary_carving::{
    CELLS, Carving, CutDirection, Finish, Locked, Reach, Way, cut_by, joined, left_note, too_near,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub struct RotaryDetailsToolpath;

/// A smaller bit after the finish, only where the bigger bits before it left material.
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(default)]
pub struct Params {
    /// What the toolpath is called in the toolpath list
    pub name: String,
    /// The detail bit: a ball nose smaller than the bits that finished before, so it fits
    /// where they could not. Its finishing stepover spaces the passes
    pub tool: Tool,
    /// The bigger bits that finished the part before this one: it cuts only what they could
    /// not reach. With none, it cuts what the toolpaths above it leave
    #[schemars(title = "Bits that finished before")]
    pub bits_before: Vec<Tool>,
    /// After the rings, the most angles A locks at for what they could not reach, each
    /// picked where it reaches the most. 0 for rings alone
    #[schemars(title = "Most angles", range(min = 0, max = 24))]
    pub most_angles: u32,
    /// Material left on the part for a later pass, all the way round
    #[schemars(extend("x-unit" = "mm"), range(min = 0))]
    pub allowance: f64,
    /// Where along the axis it starts, from the job's start (the chuck end)
    #[schemars(extend("x-unit" = "mm", "x-group" = "Along the part"), range(min = 0))]
    pub start: f64,
    /// Where along the axis it ends, from the job's start; 0 runs to the job's end
    #[schemars(extend("x-unit" = "mm", "x-group" = "Along the part"), range(min = 0))]
    pub end: f64,
    /// The holder above the bit, and the chuck and tailstock: filled in from Machine
    /// Configuration as the toolpath is made (the usual ones without)
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(skip)]
    pub surroundings: Option<Surroundings>,
    /// Settings' advanced rotary mode, filled in as the toolpath is made: on, it starts from
    /// the whole blank, not what the toolpaths above it leave (the bits before still count)
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    #[schemars(skip)]
    pub whole_blank: bool,
}

impl Default for Params {
    fn default() -> Self {
        Self {
            name: "Rotary Details".into(),
            tool: ToolDatabase::starter()
                .tools
                .into_iter()
                .filter(|t| t.shape == ToolShape::BallNose)
                .min_by(|a, b| a.diameter.total_cmp(&b.diameter))
                .expect("the starter tools include a ball nose"),
            bits_before: vec![],
            most_angles: 4,
            allowance: 0.0,
            start: 0.0,
            end: 0.0,
            surroundings: None,
            whole_blank: false,
        }
    }
}

impl Command for RotaryDetailsToolpath {
    const ID: &'static str = "rotary_details_toolpath";
    type Params = Params;

    fn meta() -> Meta {
        Meta {
            label: "Rotary Details",
            group: "3D Toolpaths",
            icon: "toolpaths/rotary_details_toolpath.png",
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

/// Why nothing was cut, when nothing was.
const NOTHING_LEFT: &str = "Nothing is left for the detail bit between Start and End: the \
    bits that finished before, and the toolpaths above this one, already reach all of the \
    model. Pick a smaller detail bit, or leave this toolpath out.";

/// The reach of a finish between Start and End with `tool`.
fn reach(p: &Params, tool: &Tool) -> Result<Reach, CommandError> {
    ensure_positive("tool diameter", tool.diameter)?;
    let stepover = tool.finishing_stepover();
    if !(stepover > 0.0 && stepover <= 1.0) {
        return Err(CommandError::InvalidParams(format!(
            "the finishing stepover of {} must be more than 0% and at most 100%: set it in \
             the Tool Database",
            tool.name
        )));
    }
    Ok(Reach {
        start: p.start,
        end: p.end,
        step: stepover * tool.diameter,
        allowance: p.allowance.max(0.0),
        cut_support: false,
    })
}

/// A finish all the way round over the axis, as a pass that cuts what it meets.
fn round_the_part(head: Head) -> Finish {
    Finish {
        way: Way::RoundThePart,
        direction: CutDirection::Both,
        held: vec![0.0],
        head,
    }
}

/// The toolpath, and what it has to say in red.
fn made(doc: &Document, p: &Params) -> Result<(Toolpath, Vec<String>), CommandError> {
    let tool = &p.tool;
    let own = reach(p, tool)?;
    let Some(stock) = doc.job.rotary.and_then(|_| Stock::of(&doc.job)) else {
        return Err(CommandError::Failed(
            "this toolpath turns the material: set the job up as rotary in Job Setup".into(),
        ));
    };
    let around = p.surroundings.unwrap_or_default();
    let carving = Carving::of(doc, tool, own)?;
    let work = carving.work();
    let corner = doc.job.area().min;
    let corner = cut_craft_core::glam::DVec3::new(corner.x, corner.y, 0.0);
    // no finer than a twelfth of the detail bit: what is left between is not worth a pass
    let mut material = Voxels::of(
        &stock,
        Voxels::cell_for(&stock, CELLS).max(tool.diameter / 12.0),
    );
    if !p.whole_blank {
        cut_craft_core::progress::stage("Working out what the toolpaths above leave");
        cut_by(
            &mut material,
            &doc.toolpaths_before_made(),
            corner,
            carving.radius,
            carving.along_x,
        );
    }
    // what each bit before reached, taken off: its finish all the way round, in thought
    for (k, before) in p.bits_before.iter().enumerate() {
        cut_craft_core::progress::stage("Working out what the bits before reached");
        cut_craft_core::progress::stop_if_cancelled()?;
        let theirs = Carving::of(doc, before, reach(p, before)?)?;
        let head = Head::new(before, &around, theirs.length, stock.swing());
        theirs
            .work()
            .finishing(&mut material, &round_the_part(head))?;
        cut_craft_core::progress::report_steps(k + 1, p.bits_before.len());
    }
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
    let rings = work.finishing(&mut material, &round_the_part(head.clone()))?;
    let mut passes = rings.passes;
    if p.most_angles > 0 {
        let Locked {
            passes: locked,
            outcome,
            more,
        } = work.locked_angles(&mut material, p.most_angles.min(24), &head)?;
        passes.extend(locked);
        kept.extend(left_note(&outcome, tool, &around, doc.job.units));
        if more {
            kept.push(
                "Some of the model is still left rough where neither the rings nor the angles \
                 A locked at reach it: raise Most angles, or add a smaller detail bit after."
                    .into(),
            );
        }
    }
    if passes.is_empty() {
        notes.push(NOTHING_LEFT.to_owned());
    }
    let (mut moves, across) = joined(
        &passes,
        doc.job.turning_gap(),
        true,
        carving.radius,
        carving.along_x,
    );
    // worked out from the material's corner; written where the material is in the job, as
    // the job's datum puts it
    for m in &mut moves {
        m.to.x += corner.x;
        m.to.y += corner.y;
    }
    let mut toolpath =
        made_by::<RotaryDetailsToolpath>(p, p.name.clone(), tool.clone(), vec![], moves);
    toolpath.across = across;
    if !kept.is_empty() {
        toolpath.params[MADE_NOTES] = kept.clone().into();
    }
    notes.extend(kept);
    Ok((toolpath, notes))
}

#[cfg(test)]
mod tests;
