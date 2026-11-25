//! The previewed material: what the previewed toolpaths have cut, the preview before the
//! last one (for Undo Last Preview), the side of the job it is cut on, what the tool hit,
//! and the mark each toolpath's cells take. The 3D view draws it; the player
//! ([`super::player`]) plays into it.

use super::{BeforePreview, Collided, EntityId, Material};
use std::collections::BTreeMap;

#[derive(Default)]
pub(crate) struct Preview {
    /// The material as cut by the previewed toolpaths.
    pub(super) material: Option<Material>,
    /// `material` at the 3D view's moving detail, and that detail (mesh cells).
    pub(super) coarse: Option<(usize, Material)>,
    /// The previewed material before the last preview, for Undo Last Preview.
    pub(super) before: Option<BeforePreview>,
    /// The side of a double-sided job the preview has up (`Document::bottom_side`): when
    /// the job turns over (Swap Sides, or its undo or redo), the preview turns with it.
    pub(super) side: bool,
    /// What the collet nut, spindle or shank hit in the last preview, by toolpath: the
    /// toolpath list marks them ⚠ in red and the 3D view shows the stock in the way red.
    pub(super) collisions: BTreeMap<EntityId, Collided>,
    /// The mark each toolpath's cells take in the preview, where it is not its own id: the
    /// one it was previewed under before a reorder moved it to another id.
    pub(super) marks: BTreeMap<EntityId, u32>,
}

impl Preview {
    /// The material shown now, kept for Undo Last Preview with the side it is on, before
    /// a preview changes it. `take` leaves none shown (Reset Preview).
    pub(super) fn keep_for_undo(&mut self, take: bool) {
        let material = if take {
            self.material.take()
        } else {
            self.material.clone()
        };
        self.before = Some(BeforePreview(material, self.side));
    }

    /// Undo Last Preview: the material before the last preview shown again. The side it was
    /// cut on, if there was one to undo.
    pub(super) fn undo(&mut self) -> Option<bool> {
        let BeforePreview(material, side) = self.before.take()?;
        self.material = material;
        Some(side)
    }

    /// The preview and its undo, when `old` (the material of a block Job Setup has
    /// changed since), go. Whether they did.
    pub(super) fn drop_old(&mut self, old: impl Fn(&Material) -> bool) -> bool {
        let before_old = (self.before.as_ref()).is_some_and(|b| b.0.as_ref().is_some_and(&old));
        if !self.material.as_ref().is_some_and(&old) && !before_old {
            return false;
        }
        (self.material, self.before) = (None, None);
        true
    }
}

#[cfg(test)]
mod tests;
