//! Rotary Locked Angles: on a rotary job, A turns to an angle and locks there while the bit
//! finishes the side facing it back and forth along the part, like a flat 3D finish; then the
//! next angle, and the next. Each is picked where it reaches the most of what is left, up to
//! the most angles asked for, and it stops sooner when the angles already cut leave nothing
//! worth another: a square part takes four, however many are allowed.
//!
//! For flats, faces and machines whose A is better at holding still than moving smoothly. It
//! cuts only what the toolpaths above it in the list leave (unless Settings' advanced rotary
//! mode says to finish the whole blank), so it never cuts air. Where the collet nut and
//! spindle can't get down to some of the model at any angle, or more is left than the angles
//! allowed reach, it says so in red.

use cut_craft_core::document::Document;
use cut_craft_core::rotary_axis::Surroundings;
use cut_craft_core::toolpath::{MADE_NOTES, Tool, ToolDatabase, ToolShape, Toolpath, made_by};
use cut_craft_core::{Change, Command, CommandError, Form, Meta, Needs, ensure_positive};
use cut_craft_rotary::{Head, Stock, Voxels};
use cut_craft_rotary_carving::{CELLS, Carving, Reach, cut_by, joined, left_note, too_near};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub struct RotaryLockedAnglesToolpath;

/// A locked at one angle after another, each where it reaches the most of what is left, and
/// the side facing the bit finished with A held still.
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(default)]
pub struct Params {
    /// What the toolpath is called in the toolpath list
    pub name: String,
    /// The bit: a ball nose follows curves best. Its finishing stepover spaces the passes
    pub tool: Tool,
    /// The most angles A locks at. Each is picked where it reaches the most of what is left,
    /// and it stops sooner when nothing worth another is left
    #[schemars(title = "Most angles", range(min = 1, max = 24))]
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
    /// Settings' advanced rotary mode, filled in as the toolpath is made: on, the whole blank
    /// is cut, not only what the toolpaths above it leave
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    #[schemars(skip)]
    pub whole_blank: bool,
}

impl Default for Params {
    fn default() -> Self {
        Self {
            name: "Rotary Locked Angles".into(),
            tool: ToolDatabase::starter()
                .tools
                .into_iter()
                .find(|t| t.shape == ToolShape::BallNose)
                .expect("the starter tools include a ball nose"),
            most_angles: 8,
            allowance: 0.0,
            start: 0.0,
            end: 0.0,
            surroundings: None,
            whole_blank: false,
        }
    }
}

impl Command for RotaryLockedAnglesToolpath {
    const ID: &'static str = "rotary_locked_angles_toolpath";
    type Params = Params;

    fn meta() -> Meta {
        Meta {
            label: "Rotary Locked Angles",
            group: "3D Toolpaths",
            icon: "toolpaths/rotary_locked_angles_toolpath.png",
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
const NOTHING_LEFT: &str = "Nothing is left for a locked angle to reach between Start and \
    End: the toolpaths above this one already take the material down to the model. Move this \
    toolpath further up the list, or leave it out.";

/// The toolpath, and what it has to say in red.
fn made(doc: &Document, p: &Params) -> Result<(Toolpath, Vec<String>), CommandError> {
    let tool = &p.tool;
    ensure_positive("tool diameter", tool.diameter)?;
    let stepover = tool.finishing_stepover();
    if !(stepover > 0.0 && stepover <= 1.0) {
        return Err(CommandError::InvalidParams(
            "the tool's finishing stepover must be more than 0% and at most 100%: set it in the Tool Database".into(),
        ));
    }
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
            step: stepover * tool.diameter,
            allowance: p.allowance.max(0.0),
            cut_support: false,
        },
    )?;
    let work = carving.work();
    let corner = doc.job.area().min;
    let corner = cut_craft_core::glam::DVec3::new(corner.x, corner.y, 0.0);
    // no finer than a twelfth of the bit: what is left between is not worth a pass
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
    let cut_craft_rotary_carving::Locked {
        passes,
        outcome,
        more,
    } = work.locked_angles(&mut material, p.most_angles.clamp(1, 24), &head)?;
    kept.extend(left_note(&outcome, tool, &around, doc.job.units));
    if more {
        kept.push(more_note(&outcome.tilts));
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
        made_by::<RotaryLockedAnglesToolpath>(p, p.name.clone(), tool.clone(), vec![], moves);
    toolpath.across = across;
    if !kept.is_empty() {
        toolpath.params[MADE_NOTES] = kept.clone().into();
    }
    notes.extend(kept);
    Ok((toolpath, notes))
}

/// In red: the angles allowed were all used, and another would still reach some of the model.
fn more_note(tilts: &[f64]) -> String {
    let list: Vec<String> = tilts.iter().map(|a| format!("{a:.0}°")).collect();
    format!(
        "A locked at {} and another angle would still reach some of the model: it is left \
         rough there. Raise Most angles, or finish it after with Rotary Finishing.",
        list.join(", ")
    )
}

#[cfg(test)]
mod tests;
