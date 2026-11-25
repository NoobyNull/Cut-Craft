//! Rotary Roughing: takes the bulk off a carved part on a rotary job, a level at a time from
//! the outside in, all the way round at each level before the next, down to the allowance
//! left on the model for the finish.
//!
//! Along the part (the usual), the bit runs the whole length with A held still, then A turns a
//! step and it runs back, like a typewriter; Round the part, A turns the bit all the way round,
//! then it steps along. It cuts only what is left: what the toolpaths above it in the list
//! leave of the blank, worked out by cutting their moves from it in order (unless Settings'
//! advanced rotary mode says to rough the whole blank). It never cuts air, and keeps the
//! collet nut and spindle clear of the material beside the cut and of the chuck and
//! tailstock: where they would not let the bit down it stops short, and says so in red.

use cut_craft_core::document::Document;
use cut_craft_core::rotary_axis::Surroundings;
use cut_craft_core::toolpath::{Tool, ToolDatabase, ToolShape, Toolpath, made_by};
use cut_craft_core::{Change, Command, CommandError, Form, Meta, Needs, ensure_positive};
use cut_craft_rotary::{Head, Stock, Voxels};
use cut_craft_rotary_carving::{
    CELLS, Carving, CutDirection, Levels, Reach, Way, cut_by, joined, too_near,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub struct RotaryRoughingToolpath;

/// Takes the bulk off a carved part a level at a time, all the way round at each level, down to
/// the allowance on the model: only what the toolpaths above it in the list leave.
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(default)]
pub struct Params {
    /// What the toolpath is called in the toolpath list
    pub name: String,
    /// The bit: an end mill clears the bulk quickest. Its stepover spaces the passes at the
    /// outside of the blank, and its pass depth sets how deep each level goes
    pub tool: Tool,
    /// Which way the bit goes over the part: along it, A turning a step between passes, or
    /// round it, stepping along between passes
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
    /// Material left on the model for the finish to take off, all the way round
    #[schemars(extend("x-unit" = "mm"), range(min = 0))]
    pub allowance: f64,
    /// Where along the axis the roughing starts, from the job's start (the chuck end)
    #[schemars(extend("x-unit" = "mm", "x-group" = "Along the part"), range(min = 0))]
    pub start: f64,
    /// Where along the axis the roughing ends, from the job's start; 0 runs to the job's end
    #[schemars(extend("x-unit" = "mm", "x-group" = "Along the part"), range(min = 0))]
    pub end: f64,
    /// The holder above the bit, and the chuck and tailstock: filled in from Machine
    /// Configuration as the toolpath is made (the usual ones without)
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(skip)]
    pub surroundings: Option<Surroundings>,
    /// Settings' advanced rotary mode, filled in as the toolpath is made: on, the whole blank
    /// is roughed, not only what the toolpaths above it leave
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    #[schemars(skip)]
    pub whole_blank: bool,
}

impl Default for Params {
    fn default() -> Self {
        Self {
            name: "Rotary Roughing".into(),
            tool: ToolDatabase::starter()
                .tools
                .into_iter()
                .find(|t| t.shape == ToolShape::EndMill)
                .expect("the starter tools include an end mill"),
            way: Way::AlongThePart,
            cut_direction: CutDirection::Both,
            allowance: 0.5,
            start: 0.0,
            end: 0.0,
            surroundings: None,
            whole_blank: false,
        }
    }
}

impl Command for RotaryRoughingToolpath {
    const ID: &'static str = "rotary_roughing_toolpath";
    type Params = Params;

    fn meta() -> Meta {
        Meta {
            label: "Rotary Roughing",
            group: "3D Toolpaths",
            icon: "toolpaths/rotary_roughing_toolpath.png",
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

/// Why nothing was roughed, when nothing was.
const NOTHING_LEFT: &str = "Nothing is left to rough between Start and End: the toolpaths \
    above this one already take the material down to the allowance on the model. Move this \
    toolpath further up the list, or leave it out.";

/// The toolpath, and what it has to say in red.
fn made(doc: &Document, p: &Params) -> Result<(Toolpath, Vec<String>), CommandError> {
    let tool = &p.tool;
    ensure_positive("bit diameter", tool.diameter)?;
    let stepover = tool.stepover;
    if !(stepover > 0.0 && stepover <= 1.0) {
        return Err(CommandError::InvalidParams(
            "the bit's stepover must be more than 0% and at most 100%: set it in the Bit Library"
                .into(),
        ));
    }
    let Some(stock) = doc.job.rotary.and_then(|_| Stock::of(&doc.job)) else {
        return Err(CommandError::Failed(
            "this toolpath turns the material: set the job up as rotary in Job Setup".into(),
        ));
    };
    let step = stepover * tool.diameter;
    let carving = Carving::of(
        doc,
        tool,
        Reach {
            start: p.start,
            end: p.end,
            step,
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
    let mut notes = too_near(
        &work,
        &head,
        &around,
        carving.length,
        carving.rotary.support.joined,
    );
    let depth = if tool.pass_depth > 0.0 {
        tool.pass_depth
    } else {
        tool.diameter / 2.0
    };
    let roughed = work.roughing(
        &mut material,
        &Levels {
            way: p.way,
            direction: p.cut_direction,
            depth,
            step,
            head,
        },
    )?;
    if !roughed.short.is_empty() {
        notes.push(short_note(&around, &roughed.short, carving.units));
    }
    if roughed.passes.is_empty() {
        notes.push(NOTHING_LEFT.to_owned());
    }
    let (mut moves, _) = joined(
        &roughed.passes,
        doc.job.turning_gap(),
        false,
        carving.radius,
        carving.along_x,
    );
    // worked out from the material's corner; written where the material is in the job, as
    // the job's datum puts it
    for m in &mut moves {
        m.to.x += corner.x;
        m.to.y += corner.y;
    }
    let toolpath =
        made_by::<RotaryRoughingToolpath>(p, p.name.clone(), tool.clone(), vec![], moves);
    Ok((toolpath, notes))
}

/// In red: where along the part the holder kept the bit up, and by how much at most.
fn short_note(
    around: &Surroundings,
    short: &[(f64, f64)],
    units: cut_craft_core::document::Units,
) -> String {
    let (from, to, most) = short.iter().fold(
        (f64::INFINITY, f64::NEG_INFINITY, 0.0f64),
        |(a, b, m), &(at, by)| (a.min(at), b.max(at), m.max(by)),
    );
    format!(
        "The collet nut ({} across) and spindle can't get down beside the material left, or \
         the chuck or tailstock beside the cut: between {} and {} along the part the bit stops \
         up to {} short. Rough the material beside it down first, keep Start and End further \
         from the chuck and tailstock, or use a bit that sticks out further.",
        units.length(around.holder.nut_diameter, 0),
        units.length(from, 1),
        units.length(to, 1),
        units.length(most, 1),
    )
}

#[cfg(test)]
mod tests;
