//! What the 3D view drew last frame, kept so the next frame (and a right-click) can pick
//! from it: where the picture was fitted, where its edit handles are, and the components
//! under the last right-click for the view's menu.

use super::Handle3d;
use cut_craft_core::document::EntityId;
use egui::{Pos2, Rect};

#[derive(Default)]
pub(super) struct Drawn3d {
    /// Where the 3D view's picture was fitted last frame.
    fit: Option<Rect>,
    /// The 3D view's edit handles, where they were drawn last frame.
    handles: Vec<(Handle3d, Pos2)>,
    /// The components under the 3D view's last right-click, topmost first.
    under: Vec<EntityId>,
}

impl Drawn3d {
    /// The picture is fitted in `fit` this frame.
    pub(super) fn fitted(&mut self, fit: Rect) {
        self.fit = Some(fit);
    }

    /// Where the picture was fitted last, if the 3D view has been drawn.
    #[cfg(test)]
    pub(super) fn fit(&self) -> Option<Rect> {
        self.fit
    }

    /// A new frame's handles are about to be drawn: last frame's are gone.
    pub(super) fn clear_handles(&mut self) {
        self.handles.clear();
    }

    /// `handle` is drawn at `at`.
    pub(super) fn add_handle(&mut self, handle: Handle3d, at: Pos2) {
        self.handles.push((handle, at));
    }

    /// The handles drawn last frame and where.
    #[cfg(test)]
    pub(super) fn handles(&self) -> &[(Handle3d, Pos2)] {
        &self.handles
    }

    /// Where `which` was drawn last frame, if it was.
    #[cfg(test)]
    pub(super) fn handle_at(&self, which: Handle3d) -> Option<Pos2> {
        self.handles
            .iter()
            .find(|(k, _)| *k == which)
            .map(|(_, at)| *at)
    }

    /// A right-click found `under` (topmost first): kept for the menu, and the topmost
    /// returned to be picked.
    pub(super) fn right_clicked(&mut self, under: Vec<EntityId>) -> Option<EntityId> {
        self.under = under;
        self.under.first().copied()
    }

    /// The components under the last right-click, topmost first.
    pub(super) fn under(&self) -> &[EntityId] {
        &self.under
    }
}

#[cfg(test)]
mod tests;
