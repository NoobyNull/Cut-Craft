//! Invertible edits. Every document mutation is a [`Change`]; undo applies its inverse.

use crate::CommandError;
use crate::document::{Document, EntityId, Job};
use crate::geometry::{Path, Point, Rect};
use glam::DAffine2;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

/// One primitive edit. Each stores what it replaces, so its inverse is exact.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum Edit {
    Add {
        id: EntityId,
        path: Path,
    },
    Remove {
        id: EntityId,
        path: Path,
    },
    Replace {
        id: EntityId,
        before: Path,
        after: Path,
    },
    Select {
        before: BTreeSet<EntityId>,
        after: BTreeSet<EntityId>,
    },
    SetJob {
        before: Job,
        after: Job,
    },
}

impl Edit {
    #[must_use]
    pub fn inverse(&self) -> Self {
        match self.clone() {
            Self::Add { id, path } => Self::Remove { id, path },
            Self::Remove { id, path } => Self::Add { id, path },
            Self::Replace { id, before, after } => Self::Replace {
                id,
                before: after,
                after: before,
            },
            Self::Select { before, after } => Self::Select {
                before: after,
                after: before,
            },
            Self::SetJob { before, after } => Self::SetJob {
                before: after,
                after: before,
            },
        }
    }
}

/// Why a change could not be applied. The document is left untouched.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ApplyError {
    Missing(EntityId),
    Exists(EntityId),
    /// The entity no longer matches what the change was built against.
    Conflict(EntityId),
    JobConflict,
}

impl fmt::Display for ApplyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Missing(id) => write!(f, "entity {} does not exist", id.0),
            Self::Exists(id) => write!(f, "entity {} already exists", id.0),
            Self::Conflict(id) => write!(f, "entity {} was changed by something else", id.0),
            Self::JobConflict => f.write_str("the job setup was changed by something else"),
        }
    }
}

impl std::error::Error for ApplyError {}

/// One step of a [`Change::chain`]: usually `&|d| SomeCommand::run(d, &params)`.
pub type Step<'a> = &'a dyn Fn(&Document) -> Result<Change, CommandError>;

/// An ordered list of edits: the output of every command.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Change {
    pub edits: Vec<Edit>,
}

impl Change {
    pub fn builder(doc: &Document) -> ChangeBuilder<'_> {
        ChangeBuilder {
            doc,
            next_id: doc.next_id().0,
            selection: doc.selection.clone(),
            pending: BTreeMap::new(),
            edits: vec![],
        }
    }

    /// Adds `path` and makes it the whole selection: what every drawing command does.
    pub fn add_and_select(doc: &Document, path: Path) -> Self {
        let mut c = Self::builder(doc);
        let id = c.add(path);
        c.select([id]);
        c.finish()
    }

    /// Transforms each selected path by `f(path)`; the shared core of move, scale, rotate,
    /// mirror and align. Nothing selected is an error; identity transforms make no edit.
    pub fn transform_selected(
        doc: &Document,
        f: impl Fn(&Path) -> DAffine2,
    ) -> Result<Self, CommandError> {
        if doc.selection.is_empty() {
            return Err(CommandError::NothingSelected);
        }
        let mut c = Self::builder(doc);
        for &id in &doc.selection {
            let path = &doc.vectors[&id];
            let t = f(path);
            if t != DAffine2::IDENTITY {
                c.replace(id, path.transformed(&t));
            }
        }
        Ok(c.finish())
    }

    /// Moves each selected path by `offset(its bounds, the selection's bounds)`: every align
    /// command is one call to this.
    pub fn align_selected(
        doc: &Document,
        offset: impl Fn(Rect, Rect) -> Point,
    ) -> Result<Self, CommandError> {
        let sel = doc
            .selection_bounds()
            .ok_or(CommandError::NothingSelected)?;
        Self::transform_selected(doc, |path| {
            DAffine2::from_translation(offset(path.bounds(), sel))
        })
    }

    /// Paths this change creates, e.g. to preview a command before applying it.
    pub fn added(&self) -> impl Iterator<Item = &Path> {
        self.edits.iter().filter_map(|e| match e {
            Edit::Add { path, .. } => Some(path),
            _ => None,
        })
    }

    /// Replaces `remove` with `add`, selecting what was added: the shape of every boolean,
    /// offset or join result.
    pub fn swap(
        doc: &Document,
        remove: impl IntoIterator<Item = EntityId>,
        add: Vec<Path>,
    ) -> Self {
        let mut c = Self::builder(doc);
        for id in remove {
            c.remove(id);
        }
        let ids: Vec<EntityId> = add.into_iter().map(|p| c.add(p)).collect();
        c.select(ids);
        c.finish()
    }

    pub fn is_empty(&self) -> bool {
        self.edits.is_empty()
    }

    #[must_use]
    pub fn inverse(&self) -> Self {
        Self {
            edits: self.edits.iter().rev().map(Edit::inverse).collect(),
        }
    }

    #[must_use]
    pub fn then(mut self, next: Self) -> Self {
        self.edits.extend(next.edits);
        self
    }

    /// Runs `steps` in order, each seeing the document as left by the previous ones, and
    /// returns their combined change. This is how a command cascades from smaller commands.
    pub fn chain(doc: &Document, steps: &[Step<'_>]) -> Result<Self, CommandError> {
        let mut scratch = doc.clone();
        let mut total = Self::default();
        for step in steps {
            let change = step(&scratch)?;
            scratch
                .apply(&change)
                .map_err(|e| CommandError::Failed(e.to_string()))?;
            total = total.then(change);
        }
        Ok(total)
    }
}

impl Document {
    /// Applies every edit or none of them.
    pub fn apply(&mut self, change: &Change) -> Result<(), ApplyError> {
        // ponytail: whole-document copy per change keeps this atomic; validate-then-apply
        // in place if documents get big enough for the copy to show up in profiles
        let mut next = self.clone();
        for edit in &change.edits {
            next.apply_edit(edit)?;
        }
        *self = next;
        Ok(())
    }

    fn apply_edit(&mut self, edit: &Edit) -> Result<(), ApplyError> {
        match edit {
            Edit::Add { id, path } => {
                if self.vectors.contains_key(id) {
                    return Err(ApplyError::Exists(*id));
                }
                self.vectors.insert(*id, path.clone());
            }
            Edit::Remove { id, path } => {
                match self.vectors.get(id) {
                    None => return Err(ApplyError::Missing(*id)),
                    Some(current) if current != path => return Err(ApplyError::Conflict(*id)),
                    Some(_) => {}
                }
                self.vectors.remove(id);
            }
            Edit::Replace { id, before, after } => match self.vectors.get_mut(id) {
                None => return Err(ApplyError::Missing(*id)),
                Some(current) if current != before => return Err(ApplyError::Conflict(*id)),
                Some(current) => current.clone_from(after),
            },
            // Selection is view state: it may have been changed outside the history (clicks),
            // so `before` is not checked; only that everything selected exists.
            Edit::Select { after, .. } => {
                if let Some(id) = after.iter().find(|id| !self.vectors.contains_key(id)) {
                    return Err(ApplyError::Missing(*id));
                }
                self.selection.clone_from(after);
            }
            Edit::SetJob { before, after } => {
                if &self.job != before {
                    return Err(ApplyError::JobConflict);
                }
                self.job = *after;
            }
        }
        Ok(())
    }
}

/// Builds a [`Change`] against a document, tracking ids and state across its own edits.
pub struct ChangeBuilder<'a> {
    doc: &'a Document,
    next_id: u64,
    selection: BTreeSet<EntityId>,
    /// Paths as this change has left them (`None` = removed), shadowing `doc`.
    pending: BTreeMap<EntityId, Option<Path>>,
    edits: Vec<Edit>,
}

impl ChangeBuilder<'_> {
    /// Current path of `id` as seen by this change so far.
    pub fn path(&self, id: EntityId) -> Option<&Path> {
        match self.pending.get(&id) {
            Some(p) => p.as_ref(),
            None => self.doc.vectors.get(&id),
        }
    }

    pub fn add(&mut self, path: Path) -> EntityId {
        let id = EntityId(self.next_id);
        self.next_id += 1;
        self.pending.insert(id, Some(path.clone()));
        self.edits.push(Edit::Add { id, path });
        id
    }

    /// Removes `id`, deselecting it first so the selection never names a missing entity.
    pub fn remove(&mut self, id: EntityId) {
        let path = self
            .path(id)
            .unwrap_or_else(|| panic!("remove: entity {} does not exist", id.0))
            .clone();
        if self.selection.contains(&id) {
            let mut after = self.selection.clone();
            after.remove(&id);
            self.select(after);
        }
        self.pending.insert(id, None);
        self.edits.push(Edit::Remove { id, path });
    }

    pub fn replace(&mut self, id: EntityId, after: Path) {
        let before = self
            .path(id)
            .unwrap_or_else(|| panic!("replace: entity {} does not exist", id.0))
            .clone();
        self.pending.insert(id, Some(after.clone()));
        self.edits.push(Edit::Replace { id, before, after });
    }

    /// Makes exactly `ids` selected.
    pub fn select(&mut self, ids: impl IntoIterator<Item = EntityId>) {
        let after: BTreeSet<EntityId> = ids.into_iter().collect();
        if after != self.selection {
            let before = std::mem::replace(&mut self.selection, after.clone());
            self.edits.push(Edit::Select { before, after });
        }
    }

    pub fn set_job(&mut self, after: Job) {
        let before = self.doc.job;
        if before != after {
            self.edits.push(Edit::SetJob { before, after });
        }
    }

    pub fn finish(self) -> Change {
        Change { edits: self.edits }
    }
}

/// Undo/redo stacks of applied changes.
#[derive(Debug, Default)]
pub struct History {
    undo: Vec<Change>,
    redo: Vec<Change>,
    revision: u64,
}

impl History {
    /// Applies `change` and records it; empty changes are not recorded.
    pub fn apply(&mut self, doc: &mut Document, change: Change) -> Result<(), ApplyError> {
        if change.is_empty() {
            return Ok(());
        }
        doc.apply(&change)?;
        self.undo.push(change);
        self.redo.clear();
        self.revision += 1;
        Ok(())
    }

    pub fn undo(&mut self, doc: &mut Document) -> bool {
        let stepped = Self::step(&mut self.undo, &mut self.redo, doc, true);
        self.revision += u64::from(stepped);
        stepped
    }

    pub fn redo(&mut self, doc: &mut Document) -> bool {
        let stepped = Self::step(&mut self.redo, &mut self.undo, doc, false);
        self.revision += u64::from(stepped);
        stepped
    }

    /// Changes on every apply, undo and redo and never repeats: compare it with the revision
    /// at the last save to know whether there is unsaved work.
    pub fn revision(&self) -> u64 {
        self.revision
    }

    pub fn can_undo(&self) -> bool {
        !self.undo.is_empty()
    }

    pub fn can_redo(&self) -> bool {
        !self.redo.is_empty()
    }

    fn step(
        from: &mut Vec<Change>,
        to: &mut Vec<Change>,
        doc: &mut Document,
        invert: bool,
    ) -> bool {
        let Some(change) = from.pop() else {
            return false;
        };
        let edit = if invert {
            change.inverse()
        } else {
            change.clone()
        };
        doc.apply(&edit)
            .expect("history only holds changes that were applied to this document");
        to.push(change);
        true
    }
}
