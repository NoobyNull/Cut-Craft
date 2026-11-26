//! The toolpaths picked in the Toolpath List: the one picked, any Ctrl-clicked with it to
//! group, move or drag together, and whether a list row was the last thing clicked (so Del
//! deletes the picked toolpath rather than the drawing's selection).

use cut_craft_core::document::EntityId;
use std::collections::BTreeSet;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(super) struct ToolpathPick {
    /// The toolpath picked in the list.
    picked: Option<EntityId>,
    /// The toolpaths Ctrl-clicked besides `picked`.
    more: BTreeSet<EntityId>,
    /// A toolpath row was the last thing clicked.
    list_clicked: bool,
}

impl ToolpathPick {
    /// The toolpath picked in the list.
    pub(super) fn picked(&self) -> Option<EntityId> {
        self.picked
    }

    /// Whether `id` is picked, on its own or Ctrl-clicked with the picked one.
    pub(super) fn is_picked(&self, id: EntityId) -> bool {
        self.picked == Some(id) || self.more.contains(&id)
    }

    /// Picks just `id`, or nothing.
    pub(super) fn only(&mut self, id: Option<EntityId>) {
        self.picked = id;
        self.more.clear();
    }

    /// Makes `id` the picked toolpath, keeping any Ctrl-clicked with it.
    pub(super) fn set(&mut self, id: Option<EntityId>) {
        self.picked = id;
    }

    /// Adds `id` to what is picked, as a Ctrl-click on an unpicked row does.
    pub(super) fn also(&mut self, id: EntityId) {
        if self.picked.is_none() {
            self.picked = Some(id);
        } else if self.picked != Some(id) {
            self.more.insert(id);
        }
    }

    /// A Ctrl-click on `id`'s row: one more picked, or one less.
    pub(super) fn ctrl_click(&mut self, id: EntityId) {
        if self.picked.is_none() {
            self.picked = Some(id);
        } else if self.picked == Some(id) {
            self.picked = self.more.pop_first();
        } else if !self.more.remove(&id) {
            self.more.insert(id);
        }
    }

    /// A plain click on `id`'s row: it alone is picked, unless it was the only one picked,
    /// when the click unpicks it.
    pub(super) fn click(&mut self, id: EntityId, only_one_picked: bool) {
        let unpick = only_one_picked && self.is_picked(id);
        self.only((!unpick).then_some(id));
    }

    /// Takes the picked toolpath away, leaving any Ctrl-clicked with it.
    pub(super) fn take(&mut self) -> Option<EntityId> {
        self.picked.take()
    }

    /// The same toolpaths picked after the list is put in a new order, `place` saying where
    /// each one went (a toolpath with no place is no longer picked).
    pub(super) fn moved(&self, place: impl Fn(EntityId) -> Option<EntityId>) -> Self {
        Self {
            picked: self.picked.and_then(&place),
            more: self.more.iter().filter_map(|&id| place(id)).collect(),
            list_clicked: self.list_clicked,
        }
    }

    /// A toolpath row was clicked (`true`), or something else was pressed (`false`).
    pub(super) fn list_clicked(&mut self, row: bool) {
        self.list_clicked = row;
    }

    /// Whether Del deletes the picked toolpath: a row was the last thing clicked and a
    /// toolpath is picked.
    pub(super) fn del_deletes_it(&self) -> bool {
        self.list_clicked && self.picked.is_some()
    }
}

#[cfg(test)]
mod tests;
