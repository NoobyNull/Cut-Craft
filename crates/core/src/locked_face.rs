//! Flat cuts on a locked face, for any flat toolpath on a rotary job: A turns to one angle
//! and stays there, and the toolpath is cut flat on the face it turned up, the way it would
//! be on a flat job. Without it a flat toolpath on a rotary job is wrapped round the part.
//!
//! The toolpath is drawn on the unrolled sheet as usual. The face is where the cut is drawn
//! (the middle of its cuts round the part), turned to the nearest flat face of a square or a
//! board, or at an angle typed in. Round the part on the sheet becomes across the face: the
//! bit is held that far off the turning axis ([`Toolpath::across`]), and its depth is
//! measured down from the face, so a cut 3 mm deep is 3 mm deep all across it.

use crate::document::{Along, Blank, Job};
use crate::rotary_axis::tip_of;
use crate::toolpath::{MoveKind, Toolpath};
use crate::{Change, CommandError, Edit};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::f64::consts::FRAC_PI_2;

/// Where a toolpath's settings keep it (`locked_face` in them).
pub const KEY: &str = "locked_face";

/// A flat toolpath cut flat on a locked face of a rotary job.
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(default)]
pub struct LockedFace {
    /// The angle A locks at, degrees; none for the face the cut is drawn on
    pub angle: Option<f64>,
}

impl LockedFace {
    /// The locked face a toolpath's settings ask for, if any.
    #[must_use]
    pub fn of(params: &serde_json::Value) -> Option<Self> {
        serde_json::from_value(params.get(KEY)?.clone()).ok()
    }
}

/// Why a drill bank's toolpath can't be cut on a locked face: the post puts its holes where
/// the drills are, not where the moves go.
const DRILL_BANK: &str = "a drill bank's holes go where its drills are: Flat on a locked face \
    is only for toolpaths the bit cuts itself";

/// Why a toolpath that already holds the bit off the axis can't be cut on a locked face.
const ROTARY_OWN: &str = "this toolpath already turns A as it cuts: Flat on a locked face is \
    only for flat toolpaths";

/// `toolpath` cut flat on the locked face its settings ask for, on a rotary `job`: A held at
/// the face's angle, the bit held across the face and its depth measured from the face.
/// Nothing changes on a flat job, or when its settings don't ask for it.
///
/// # Errors
/// When the toolpath already holds the bit off the axis itself (a rotary toolpath).
pub fn apply(
    job: &Job,
    toolpath: &mut Toolpath,
    params: &serde_json::Value,
) -> Result<(), CommandError> {
    let (Some(face), Some(rotary)) = (LockedFace::of(params), job.rotary) else {
        return Ok(());
    };
    if !toolpath.across.is_empty() {
        return Err(CommandError::InvalidParams(ROTARY_OWN.into()));
    }
    if toolpath.drill_bank.is_some() {
        return Err(CommandError::InvalidParams(DRILL_BANK.into()));
    }
    let radius = rotary.diameter / 2.0;
    if radius <= 0.0 || toolpath.moves.is_empty() {
        return Ok(());
    }
    let along_x = rotary.along == Along::X;
    let corner = job.area().min;
    let start = if along_x { corner.y } else { corner.x };
    let round_of = |p: glam::DVec3| if along_x { p.y } else { p.x } - start;
    let angle = face.angle.map_or_else(
        || facing(rotary.blank, middle(toolpath, round_of) / radius),
        f64::to_radians,
    );
    // the face is as far from the axis as the material reaches straight up at that angle
    let height = face_height(rotary.blank, rotary.diameter, angle);
    let face_round = radius * angle;
    let mut across = Vec::with_capacity(toolpath.moves.len());
    if let Some(deepest) = toolpath.moves.iter().map(|m| m.to.z).reduce(f64::min)
        && height + deepest < 0.0
    {
        return Err(CommandError::InvalidParams(format!(
            "Flat on a locked face: this toolpath goes {:.1} mm deep, past the turning axis \
             ({height:.1} mm under the face). Cut less deep",
            -deepest
        )));
    }
    for m in &mut toolpath.moves {
        let off = round_of(m.to) - face_round;
        // how high the bit is, from the material's top straight over the axis
        let held = height + m.to.z - radius;
        let (round, z) = tip_of(radius, face_round, held, off);
        if along_x {
            m.to.y = round + start;
        } else {
            m.to.x = round + start;
        }
        m.to.z = z;
        across.push(off);
    }
    toolpath.across = across;
    toolpath.params[KEY] =
        serde_json::to_value(LockedFace { angle: face.angle }).expect("a locked face serialises");
    Ok(())
}

/// `change` with each toolpath it makes cut on a locked face, as [`apply`].
///
/// # Errors
/// As [`apply`].
pub fn with_locked_face(
    job: &Job,
    params: &serde_json::Value,
    mut change: Change,
) -> Result<Change, CommandError> {
    for edit in &mut change.edits {
        if let Edit::AddToolpath { toolpath, .. }
        | Edit::ReplaceToolpath {
            after: toolpath, ..
        } = edit
        {
            apply(job, toolpath, params)?;
        }
    }
    Ok(change)
}

/// The middle of a toolpath's cuts round the part, mm round the surface from A=0.
fn middle(toolpath: &Toolpath, round_of: impl Fn(glam::DVec3) -> f64) -> f64 {
    let (low, high) = toolpath
        .moves
        .iter()
        .filter(|m| m.kind != MoveKind::Rapid)
        .map(|m| round_of(m.to))
        .fold((f64::INFINITY, f64::NEG_INFINITY), |(a, b), r| {
            (a.min(r), b.max(r))
        });
    if low.is_finite() {
        f64::midpoint(low, high)
    } else {
        0.0
    }
}

/// The angle A locks at for a cut whose middle is at `angle` (radians): a square or a board
/// turns to its nearest flat face; a round blank is cut flat right there.
fn facing(blank: Blank, angle: f64) -> f64 {
    match blank {
        Blank::Round => angle,
        Blank::Square | Blank::Board { .. } => (angle / FRAC_PI_2).round() * FRAC_PI_2,
    }
}

/// How far from the turning axis the material reaches straight up at `angle`, for a blank
/// `diameter` across at A=0: a round blank's radius, a square's half side, a board's half
/// thickness (A=0 and 180) or half width (90 and 270); turned part way, a square or a board
/// reaches as high as its furthest corner.
fn face_height(blank: Blank, diameter: f64, angle: f64) -> f64 {
    match blank {
        Blank::Round => diameter / 2.0,
        // a square turned part way is as high as its furthest corner, as a board is
        Blank::Square => face_height(Blank::Board { width: diameter }, diameter, angle),
        Blank::Board { width } => {
            let (sin, cos) = angle.sin_cos();
            // a board turned part way is as high as its furthest corner
            (diameter / 2.0 * cos.abs()) + (width / 2.0 * sin.abs())
        }
    }
}

#[cfg(test)]
mod tests;
