//! Toolpaths offered by what the machine has: a laser toolpath only with a laser fitted, a
//! rotary one only with a 4th axis, as the tool changer's settings show only with a tool
//! changer. Each command says what it needs ([`Needs`]); a toolpath already in the job stays,
//! with a red warning at the foot of its form saying how to fit the machine for it.

use super::CutCraftApp;
use cut_craft_core::Needs;
use cut_craft_machines::Machine;

/// Whether `machine` (none set up: none) has what a toolpath `needs`.
pub(super) fn fits(machine: Option<&Machine>, needs: Needs) -> bool {
    needs == Needs::Nothing || machine.is_some_and(|m| m.has(needs))
}

/// What a form of a toolpath that `needs` what `machine` lacks says, in red at its foot.
pub(super) fn lacks(machine: Option<&Machine>, needs: Needs) -> Option<&'static str> {
    if fits(machine, needs) {
        return None;
    }
    Some(match needs {
        Needs::Laser => {
            "This machine has no laser fitted, so it cannot cut this toolpath: tick Laser \
             fitted in Machine Configuration, or make the machine with the laser the active one"
        }
        Needs::Rotary | Needs::Nothing => {
            "This machine has no 4th axis, so it cannot cut this toolpath: tick 4th axis in \
             Machine Configuration (its firmware must drive an A axis), or make the machine \
             with one the active one"
        }
    })
}

impl CutCraftApp {
    /// Whether tool `id` is offered on the tool pages and in Find Tool: anything that needs
    /// nothing of the machine, else only when the active machine has what it needs.
    pub(super) fn offered(&self, id: &str) -> bool {
        fits(self.machines.active(), self.form_kind(id).needs())
    }

    /// The red line for a form whose toolpath `needs` what the active machine lacks.
    pub(super) fn machine_lacks(&self, needs: Needs) -> Option<&'static str> {
        lacks(self.machines.active(), needs)
    }
}

#[cfg(test)]
mod tests;
