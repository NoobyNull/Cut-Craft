//! Checks before cutting for a job's bit changes (Bit changes you can make): Z zero set
//! again with each bit changed by hand, a pause the controller can be zeroed at, and the
//! cutting order Group by bit makes. Worked out from the job, the machine and the plan,
//! beside the checks [`cut_craft_core::preflight`] makes of the cutting itself.

use super::{EntityId, files};
use cut_craft_core::document::{Document, ZZero};
use cut_craft_core::preflight::{Check, Finding};
use cut_craft_machines::Machine;

/// What the bit changes of `plan` (file by file, in cutting order) need looking at, saved
/// with `changes` for `machine`.
pub(super) fn findings(
    doc: &Document,
    machine: Option<&Machine>,
    plan: &[Vec<EntityId>],
    changes: files::ToolChanges,
) -> Vec<Finding> {
    let cut: Vec<EntityId> = plan
        .iter()
        .flatten()
        .copied()
        .filter(|id| doc.toolpaths.contains_key(id))
        .collect();
    let mut found = vec![];
    if let Some(order) = reordered(doc, &cut) {
        found.push(Finding::warning(
            Check::BitChanges,
            format!(
                "Group by bit is on (Machine Configuration), so the toolpaths are cut bit by \
                 bit, not in the order they are listed: {order}. Check nothing is cut free \
                 before the work inside it is done; turn Group by bit off to cut in list order"
            ),
        ));
    }
    let bits = runs(doc, &cut);
    if bits.len() < 2 || changes == files::ToolChanges::ToolChanger {
        return found;
    }
    let zero = match doc.job.z_zero {
        ZZero::MaterialSurface => "on top of the material",
        ZZero::MachineBed if doc.job.rotary.is_some() => "on the turning axis",
        ZZero::MachineBed => "on the bed",
    };
    let how = if changes == files::ToolChanges::Pause {
        "at each pause"
    } else {
        "before running each file"
    };
    found.push(Finding::warning(
        Check::BitChanges,
        format!(
            "This job changes bits by hand {} times ({}). Set Z zero again {how} with the \
             new bit, the same way as the first ({zero}): a new bit sits at another height \
             in the collet, and every cut after the change is measured from it",
            bits.len() - 1,
            bits.join(", then ")
        ),
    ));
    if changes == files::ToolChanges::Pause && machine.is_some_and(Machine::cannot_jog_when_paused)
    {
        found.push(Finding::warning(
            Check::BitChanges,
            "One file that pauses is chosen, but standard GRBL cannot jog while paused at \
             M0, so you cannot move the new bit down to set Z zero. Choose Separate file \
             for each tool, or pick the controller's firmware in Machine Configuration if \
             it is not standard GRBL"
                .to_owned(),
        ));
    }
    found
}

/// The bits `cut` uses in the order they are fitted: a bit again each time it comes back
/// after another.
fn runs(doc: &Document, cut: &[EntityId]) -> Vec<String> {
    let mut bits: Vec<String> = vec![];
    for id in cut {
        let name = &doc.toolpaths[id].tool.name;
        if bits.last() != Some(name) {
            bits.push(name.clone());
        }
    }
    bits
}

/// The cutting order of `cut`, by name, when it is not the order the toolpaths are listed
/// in (Group by bit moved some); `None` when it is.
fn reordered(doc: &Document, cut: &[EntityId]) -> Option<String> {
    let listed: Vec<EntityId> = doc
        .toolpaths
        .keys()
        .copied()
        .filter(|id| cut.contains(id))
        .collect();
    (listed != cut).then(|| {
        cut.iter()
            .map(|id| format!("\"{}\"", doc.toolpaths[id].name))
            .collect::<Vec<_>>()
            .join(", ")
    })
}

#[cfg(test)]
#[path = "bit_changes/tests.rs"]
mod tests;
