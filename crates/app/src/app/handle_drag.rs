//! A drag on one of the 3D view's edit handles in Transform mode: the handle held and where
//! the press was, and for a height handle what the drag began from and last applied, so the
//! whole drag is one undo and the model is made again no more often than it can keep up.

use super::Handle3d;
use cmd_component_properties::Params;
use serde_json::Value;

#[derive(Default)]
pub(super) struct HandleDrag {
    /// The handle being dragged, and where the pointer pressed.
    held: Option<(Handle3d, egui::Pos2)>,
    /// A height handle's drag as it follows the pointer: the component's settings and its
    /// top when the drag began, and the settings last applied (undone and applied again
    /// each time the pointer moves, so the whole drag is one undo).
    height: Option<(Params, f64, Option<Value>)>,
    /// When a height handle's drag last remade the model (the app's clock, seconds).
    applied_at: Option<f64>,
}

impl HandleDrag {
    /// A drag starts on `handle`, pressed at `from`.
    pub(super) fn start(&mut self, handle: Handle3d, from: egui::Pos2) {
        self.held = Some((handle, from));
    }

    /// The handle being dragged and where the pointer pressed.
    pub(super) fn held(&self) -> Option<(Handle3d, egui::Pos2)> {
        self.held
    }

    /// Whether a handle is being dragged.
    pub(super) fn is_held(&self) -> bool {
        self.held.is_some()
    }

    /// Whether a height handle's drag has begun following the pointer.
    pub(super) fn is_setting_height(&self) -> bool {
        self.height.is_some()
    }

    /// The handle is let go of (a transform drag, done or off the plane).
    pub(super) fn let_go(&mut self) {
        self.held = None;
    }

    /// The drag is over, as far as it went: nothing of it is kept.
    pub(super) fn end(&mut self) {
        *self = Self::default();
    }

    /// Escape: the drag ends with nothing changed. `None` when no handle was held; else
    /// whether a height was already applied, which the caller undoes.
    pub(super) fn cancel(&mut self) -> Option<bool> {
        let held = self.held.is_some();
        let applied = self.height.as_ref().is_some_and(|(.., a)| a.is_some());
        if !held {
            return None;
        }
        self.end();
        Some(applied)
    }

    /// A height handle's drag: the settings and top it began from (taken by `begin` on its
    /// first frame), and the settings last applied.
    pub(super) fn height(
        &mut self,
        begin: impl FnOnce() -> (Params, f64),
    ) -> (&Params, f64, Option<&Value>) {
        let (base, top, applied) = self.height.get_or_insert_with(|| {
            let (base, top) = begin();
            (base, top, None)
        });
        (base, *top, applied.as_ref())
    }

    /// The height drag applied `params` (`None`: it failed, nothing applied) at `time`.
    pub(super) fn applied(&mut self, params: Option<Value>, time: f64) {
        if let Some((.., applied)) = &mut self.height {
            *applied = params;
        }
        self.applied_at = Some(time);
    }

    /// Seconds since the height drag last remade the model at `time` (forever if it hasn't).
    pub(super) fn since_applied(&self, time: f64) -> f64 {
        self.applied_at.map_or(f64::INFINITY, |t| time - t)
    }
}

#[cfg(test)]
mod tests;
