//! Toolpath ids are places in the cut order: a reorder keeps the ids where they are and
//! moves the toolpaths between them, and so does its undo and redo. What the app holds by
//! a toolpath's id follows the toolpath to its new id, so it is never left on another.

use std::collections::BTreeMap;

use cut_craft_core::document::EntityId;
use cut_craft_core::toolpath::Toolpath;

use super::CutCraftApp;

/// What tells one toolpath from another: its name, strategy, how many moves it has and
/// where the last one ends.
type Key = (String, String, usize, Option<[u64; 3]>);

fn key(t: &Toolpath) -> Key {
    (
        t.name.clone(),
        t.strategy.clone(),
        t.moves.len(),
        t.moves.last().map(|m| m.to.to_array().map(f64::to_bits)),
    )
}

/// Each toolpath's key, by id, as at a change of a job (none yet: the job and its history
/// revision).
#[derive(Default)]
pub(super) struct Keys {
    at: Option<(u64, u64)>,
    keys: BTreeMap<EntityId, Key>,
}

/// Where the toolpaths in `before` went in `after`: old id to new id, for those that moved.
/// A toolpath is taken to have moved when its id now holds another and exactly one of the
/// changed ids holds one like it; and only when the moves swap toolpaths between the same
/// ids (an edit is not a move).
fn moved(
    before: &BTreeMap<EntityId, Key>,
    after: &BTreeMap<EntityId, Key>,
) -> BTreeMap<EntityId, EntityId> {
    let changed: Vec<EntityId> = before
        .iter()
        .filter(|(id, k)| after.get(id) != Some(k))
        .map(|(&id, _)| id)
        .collect();
    let mut moved = BTreeMap::new();
    for &from in &changed {
        let mut like = changed
            .iter()
            .filter(|&&to| to != from && after.get(&to) == before.get(&from));
        if let (Some(&to), None) = (like.next(), like.next()) {
            moved.insert(from, to);
        }
    }
    let mut to: Vec<EntityId> = moved.values().copied().collect();
    to.sort_unstable();
    let from: Vec<EntityId> = moved.keys().copied().collect();
    if from == to { moved } else { BTreeMap::new() }
}

impl CutCraftApp {
    /// Run each frame: once the job has changed, what follows a toolpath goes to the id the
    /// toolpath is now at.
    pub(super) fn follow_toolpaths(&mut self) {
        let at = Some((self.job_epoch, self.history.revision()));
        if self.toolpath_keys.at == at {
            return;
        }
        let keys: BTreeMap<EntityId, Key> = self
            .doc
            .toolpaths
            .iter()
            .map(|(&id, t)| (id, key(t)))
            .collect();
        let before = std::mem::replace(&mut self.toolpath_keys, Keys { at, keys });
        if before.at.map(|(job, _)| job) != at.map(|(job, _)| job) {
            return;
        }
        let moved = moved(&before.keys, &self.toolpath_keys.keys);
        if !moved.is_empty() {
            self.toolpaths_moved(&moved);
        }
    }

    /// Toolpaths moved to other ids, old id to new: the toolpath an open form edits goes
    /// with them.
    pub(super) fn toolpaths_moved(&mut self, moved: &BTreeMap<EntityId, EntityId>) {
        let follow = |id: &mut Option<EntityId>| {
            if let Some(to) = id.and_then(|id| moved.get(&id)) {
                *id = Some(*to);
            }
        };
        if let Some(form) = self.form.as_mut() {
            follow(&mut form.editing);
        }
    }
}
