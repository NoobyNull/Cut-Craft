//! Invertible edits. Every document mutation is a [`Change`]; undo applies its inverse.

use crate::CommandError;
use crate::bitmap::Bitmap;
use crate::component::Component;
use crate::document::{Document, EntityId, Job, Layer, LayerId};
use crate::geometry::{Path, Point, Rect};
use crate::selection::Selection;
use crate::toolpath::Toolpath;
use glam::DAffine2;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
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
        before: Selection,
        after: Selection,
    },
    SetJob {
        before: Job,
        after: Job,
    },
    AddToolpath {
        id: EntityId,
        toolpath: Box<Toolpath>,
    },
    RemoveToolpath {
        id: EntityId,
        toolpath: Box<Toolpath>,
    },
    ReplaceToolpath {
        id: EntityId,
        before: Box<Toolpath>,
        after: Box<Toolpath>,
    },
    AddComponent {
        id: EntityId,
        component: Box<Component>,
    },
    RemoveComponent {
        id: EntityId,
        component: Box<Component>,
    },
    ReplaceComponent {
        id: EntityId,
        before: Box<Component>,
        after: Box<Component>,
    },
    AddBitmap {
        id: EntityId,
        bitmap: Box<Bitmap>,
    },
    RemoveBitmap {
        id: EntityId,
        bitmap: Box<Bitmap>,
    },
    ReplaceBitmap {
        id: EntityId,
        before: Box<Bitmap>,
        after: Box<Bitmap>,
    },
    /// Puts a vector into a group or takes it out (`None`).
    SetGroup {
        id: EntityId,
        before: Option<u64>,
        after: Option<u64>,
    },
    /// Moves a vector between layers; `None` is "not on any layer" (added or removed).
    SetLayer {
        id: EntityId,
        before: Option<LayerId>,
        after: Option<LayerId>,
    },
    AddLayer {
        index: usize,
        layer: Layer,
    },
    RemoveLayer {
        index: usize,
        layer: Layer,
    },
    ReplaceLayer {
        before: Layer,
        after: Layer,
    },
    SetCurrentLayer {
        before: LayerId,
        after: LayerId,
    },
    SetNotes {
        before: String,
        after: String,
    },
    /// A group's text settings (what Draw Text made it from); `None` when it is not text.
    SetText {
        group: u64,
        before: Option<serde_json::Value>,
        after: Option<serde_json::Value>,
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
            Self::AddToolpath { id, toolpath } => Self::RemoveToolpath { id, toolpath },
            Self::RemoveToolpath { id, toolpath } => Self::AddToolpath { id, toolpath },
            Self::ReplaceToolpath { id, before, after } => Self::ReplaceToolpath {
                id,
                before: after,
                after: before,
            },
            Self::AddComponent { id, component } => Self::RemoveComponent { id, component },
            Self::RemoveComponent { id, component } => Self::AddComponent { id, component },
            Self::AddBitmap { id, bitmap } => Self::RemoveBitmap { id, bitmap },
            Self::RemoveBitmap { id, bitmap } => Self::AddBitmap { id, bitmap },
            Self::ReplaceBitmap { id, before, after } => Self::ReplaceBitmap {
                id,
                before: after,
                after: before,
            },
            Self::ReplaceComponent { id, before, after } => Self::ReplaceComponent {
                id,
                before: after,
                after: before,
            },
            Self::SetGroup { id, before, after } => Self::SetGroup {
                id,
                before: after,
                after: before,
            },
            Self::SetLayer { id, before, after } => Self::SetLayer {
                id,
                before: after,
                after: before,
            },
            Self::AddLayer { index, layer } => Self::RemoveLayer { index, layer },
            Self::RemoveLayer { index, layer } => Self::AddLayer { index, layer },
            Self::ReplaceLayer { before, after } => Self::ReplaceLayer {
                before: after,
                after: before,
            },
            Self::SetCurrentLayer { before, after } => Self::SetCurrentLayer {
                before: after,
                after: before,
            },
            Self::SetNotes { before, after } => Self::SetNotes {
                before: after.clone(),
                after: before.clone(),
            },
            Self::SetText {
                group,
                before,
                after,
            } => Self::SetText {
                group,
                before: after.clone(),
                after: before.clone(),
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
    MissingLayer(LayerId),
    LayerExists(LayerId),
    LayerConflict(LayerId),
    /// Vectors are still on it, or it is the last layer.
    LayerInUse(LayerId),
}

impl fmt::Display for ApplyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Missing(id) => write!(f, "entity {} does not exist", id.0),
            Self::Exists(id) => write!(f, "entity {} already exists", id.0),
            Self::Conflict(id) => write!(f, "entity {} was changed by something else", id.0),
            Self::JobConflict => f.write_str("the job setup was changed by something else"),
            Self::MissingLayer(id) => write!(f, "layer {} does not exist", id.0),
            Self::LayerExists(id) => write!(f, "layer {} already exists", id.0),
            Self::LayerConflict(id) => write!(f, "layer {} was changed by something else", id.0),
            Self::LayerInUse(id) => write!(
                f,
                "layer {} still has vectors on it, or is the last layer",
                id.0
            ),
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
            pending_toolpaths: BTreeMap::new(),
            pending_components: BTreeMap::new(),
            layers: doc.layers.clone(),
            current_layer: doc.current_layer,
            vector_layers: BTreeMap::new(),
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

    /// Adds a copy of the selection under each of `places`, selecting the copies; `group`
    /// makes them one group.
    pub fn copy_selected(
        doc: &Document,
        places: &[DAffine2],
        group: bool,
    ) -> Result<Self, CommandError> {
        if doc.selection.is_empty() {
            return Err(CommandError::NothingSelected);
        }
        let group = group.then(|| doc.next_group());
        let mut c = Self::builder(doc);
        let mut copies = Vec::new();
        for t in places {
            for id in &doc.selection {
                copies.push(c.add(doc.vectors[id].transformed(t)));
            }
        }
        for &id in &copies {
            c.set_group(id, group);
        }
        c.select(copies);
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

    /// Spaces the selected paths evenly along x (`vertical` false) or y: the first and last
    /// stay put and the others move so the gaps between their extents are equal, keeping
    /// their order and their other coordinate.
    pub fn space_selected(doc: &Document, vertical: bool) -> Result<Self, CommandError> {
        let along = |r: Rect| {
            if vertical {
                (r.min.y, r.max.y)
            } else {
                (r.min.x, r.max.x)
            }
        };
        let mut items: Vec<(EntityId, f64, f64)> = doc
            .selection
            .iter()
            .map(|&id| {
                let (lo, hi) = along(doc.vectors[&id].bounds());
                (id, lo, hi)
            })
            .collect();
        if items.len() < 3 {
            return Err(CommandError::Failed(
                "select at least three objects to space".into(),
            ));
        }
        items.sort_by(|a, b| (a.1 + a.2).total_cmp(&(b.1 + b.2)));
        let span = items[items.len() - 1].2 - items[0].1;
        let sizes: f64 = items.iter().map(|(_, lo, hi)| hi - lo).sum();
        #[expect(clippy::cast_precision_loss, reason = "a few objects")]
        let gap = (span - sizes) / (items.len() - 1) as f64;
        let mut c = Self::builder(doc);
        let mut at = items[0].1;
        for &(id, lo, hi) in &items {
            let by = at - lo;
            if by.abs() > 1e-12 {
                let shift = if vertical {
                    Point::new(0.0, by)
                } else {
                    Point::new(by, 0.0)
                };
                c.replace(
                    id,
                    doc.vectors[&id].transformed(&DAffine2::from_translation(shift)),
                );
            }
            at += hi - lo + gap;
        }
        Ok(c.finish())
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

    /// The job this change sets, if it sets one: lets a view show a job before it is applied.
    pub fn job(&self) -> Option<&Job> {
        self.edits.iter().rev().find_map(|e| match e {
            Edit::SetJob { after, .. } => Some(after),
            _ => None,
        })
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
        Self::chain_over(doc, steps, |scratch, step| step(scratch))
    }

    /// Runs `step` for each of `items` in turn, each seeing the document as the previous one
    /// left it: a cascade over many items (Show Only This, Delete Visible...).
    pub fn chain_over<T>(
        doc: &Document,
        items: &[T],
        step: impl Fn(&Document, &T) -> Result<Self, CommandError>,
    ) -> Result<Self, CommandError> {
        let mut scratch = doc.clone();
        let mut total = Self::default();
        for item in items {
            let change = step(&scratch, item)?;
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

    /// The component and bitmap edits.
    fn apply_model_edit(&mut self, edit: &Edit) -> Result<(), ApplyError> {
        match edit {
            Edit::AddComponent { id, component } => {
                insert_new(&mut self.components, *id, (**component).clone())?;
            }
            Edit::RemoveComponent { id, component } => {
                remove_matching(&mut self.components, *id, component)?;
            }
            Edit::AddBitmap { id, bitmap } => {
                insert_new(&mut self.bitmaps, *id, (**bitmap).clone())?;
            }
            Edit::RemoveBitmap { id, bitmap } => {
                remove_matching(&mut self.bitmaps, *id, bitmap)?;
                self.selected_bitmaps.remove(id);
            }
            Edit::ReplaceBitmap { id, before, after } => {
                replace_matching(&mut self.bitmaps, *id, before, after)?;
            }
            Edit::ReplaceComponent { id, before, after } => {
                replace_matching(&mut self.components, *id, before, after)?;
            }
            _ => unreachable!("only component and bitmap edits come here"),
        }
        Ok(())
    }

    fn set_text(&mut self, group: u64, text: Option<serde_json::Value>) {
        match text {
            Some(text) => self.texts.insert(group, text),
            None => self.texts.remove(&group),
        };
    }

    fn apply_edit(&mut self, edit: &Edit) -> Result<(), ApplyError> {
        match edit {
            Edit::Add { id, path } => insert_new(&mut self.vectors, *id, path.clone())?,
            Edit::Remove { id, path } => remove_matching(&mut self.vectors, *id, path)?,
            Edit::Replace { id, before, after } => {
                replace_matching(&mut self.vectors, *id, before, after)?;
            }
            Edit::AddToolpath { id, toolpath } => {
                insert_new(&mut self.toolpaths, *id, (**toolpath).clone())?;
            }
            Edit::RemoveToolpath { id, toolpath } => {
                remove_matching(&mut self.toolpaths, *id, toolpath)?;
            }
            Edit::ReplaceToolpath { id, before, after } => {
                replace_matching(&mut self.toolpaths, *id, before, after)?;
            }
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
            Edit::AddComponent { .. }
            | Edit::RemoveComponent { .. }
            | Edit::ReplaceComponent { .. }
            | Edit::AddBitmap { .. }
            | Edit::RemoveBitmap { .. }
            | Edit::ReplaceBitmap { .. } => self.apply_model_edit(edit)?,
            Edit::SetGroup { id, before, after } => {
                if self.vector_groups.get(id) != before.as_ref() {
                    return Err(ApplyError::Conflict(*id));
                }
                match after {
                    Some(group) => self.vector_groups.insert(*id, *group),
                    None => self.vector_groups.remove(id),
                };
            }
            Edit::SetLayer { id, before, after } => {
                if self.vector_layers.get(id) != before.as_ref() {
                    return Err(ApplyError::Conflict(*id));
                }
                match after {
                    Some(layer) if self.layer(*layer).is_none() => {
                        return Err(ApplyError::MissingLayer(*layer));
                    }
                    Some(layer) => {
                        self.vector_layers.insert(*id, *layer);
                    }
                    None => {
                        self.vector_layers.remove(id);
                    }
                }
            }
            Edit::AddLayer { index, layer } => {
                if self.layer(layer.id).is_some() {
                    return Err(ApplyError::LayerExists(layer.id));
                }
                self.layers
                    .insert((*index).min(self.layers.len()), layer.clone());
            }
            Edit::RemoveLayer { index, layer } => {
                if self.layers.get(*index) != Some(layer) {
                    return Err(ApplyError::LayerConflict(layer.id));
                }
                let holds_vectors = self.vectors_on(layer.id).next().is_some();
                if holds_vectors || self.layers.len() == 1 || self.current_layer == layer.id {
                    return Err(ApplyError::LayerInUse(layer.id));
                }
                self.layers.remove(*index);
            }
            Edit::ReplaceLayer { before, after } => {
                match self.layers.iter_mut().find(|l| l.id == before.id) {
                    None => return Err(ApplyError::MissingLayer(before.id)),
                    Some(current) if current != before => {
                        return Err(ApplyError::LayerConflict(before.id));
                    }
                    Some(current) => current.clone_from(after),
                }
            }
            // Like the selection, the current layer is view state: `before` is not checked.
            Edit::SetCurrentLayer { after, .. } => {
                if self.layer(*after).is_none() {
                    return Err(ApplyError::MissingLayer(*after));
                }
                self.current_layer = *after;
            }
            Edit::SetNotes { after, .. } => self.notes.clone_from(after),
            Edit::SetText { group, after, .. } => self.set_text(*group, after.clone()),
        }
        Ok(())
    }
}

fn insert_new<T>(
    map: &mut BTreeMap<EntityId, T>,
    id: EntityId,
    value: T,
) -> Result<(), ApplyError> {
    if map.contains_key(&id) {
        return Err(ApplyError::Exists(id));
    }
    map.insert(id, value);
    Ok(())
}

/// `id` as a change has left it (`pending`, where `None` is removed), else as in `doc`.
fn shadowed<'a, T>(
    pending: &'a BTreeMap<EntityId, Option<T>>,
    doc: &'a BTreeMap<EntityId, T>,
    id: EntityId,
) -> Option<&'a T> {
    match pending.get(&id) {
        Some(value) => value.as_ref(),
        None => doc.get(&id),
    }
}

/// Removes `id` only if it still holds `expected`.
fn remove_matching<T: PartialEq>(
    map: &mut BTreeMap<EntityId, T>,
    id: EntityId,
    expected: &T,
) -> Result<(), ApplyError> {
    match map.get(&id) {
        None => Err(ApplyError::Missing(id)),
        Some(current) if current != expected => Err(ApplyError::Conflict(id)),
        Some(_) => {
            map.remove(&id);
            Ok(())
        }
    }
}

/// Replaces `id` only if it still holds `before`.
fn replace_matching<T: PartialEq + Clone>(
    map: &mut BTreeMap<EntityId, T>,
    id: EntityId,
    before: &T,
    after: &T,
) -> Result<(), ApplyError> {
    match map.get_mut(&id) {
        None => Err(ApplyError::Missing(id)),
        Some(current) if current != before => Err(ApplyError::Conflict(id)),
        Some(current) => {
            current.clone_from(after);
            Ok(())
        }
    }
}

/// Builds a [`Change`] against a document, tracking ids and state across its own edits.
pub struct ChangeBuilder<'a> {
    doc: &'a Document,
    next_id: u64,
    selection: Selection,
    /// Paths as this change has left them (`None` = removed), shadowing `doc`.
    pending: BTreeMap<EntityId, Option<Path>>,
    pending_toolpaths: BTreeMap<EntityId, Option<Toolpath>>,
    pending_components: BTreeMap<EntityId, Option<Component>>,
    /// Layer list, current layer and vector layers as this change has left them.
    layers: Vec<Layer>,
    current_layer: LayerId,
    vector_layers: BTreeMap<EntityId, Option<LayerId>>,
    edits: Vec<Edit>,
}

impl ChangeBuilder<'_> {
    /// Current path of `id` as seen by this change so far.
    pub fn path(&self, id: EntityId) -> Option<&Path> {
        shadowed(&self.pending, &self.doc.vectors, id)
    }

    pub fn add(&mut self, path: Path) -> EntityId {
        let id = EntityId(self.next_id);
        self.next_id += 1;
        self.pending.insert(id, Some(path.clone()));
        self.edits.push(Edit::Add { id, path });
        self.set_vector_layer(id, Some(self.current_layer));
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
        self.set_vector_layer(id, None);
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
        let after: Selection = ids.into_iter().collect();
        if after != self.selection {
            let before = std::mem::replace(&mut self.selection, after.clone());
            self.edits.push(Edit::Select { before, after });
        }
    }

    pub fn add_toolpath(&mut self, toolpath: Toolpath) -> EntityId {
        let id = EntityId(self.next_id);
        self.next_id += 1;
        self.pending_toolpaths.insert(id, Some(toolpath.clone()));
        self.edits.push(Edit::AddToolpath {
            id,
            toolpath: Box::new(toolpath),
        });
        id
    }

    /// Current toolpath `id` as seen by this change so far.
    pub fn toolpath(&self, id: EntityId) -> Option<&Toolpath> {
        shadowed(&self.pending_toolpaths, &self.doc.toolpaths, id)
    }

    pub fn remove_toolpath(&mut self, id: EntityId) {
        let toolpath = self
            .toolpath(id)
            .unwrap_or_else(|| panic!("remove_toolpath: {} does not exist", id.0))
            .clone();
        self.pending_toolpaths.insert(id, None);
        self.edits.push(Edit::RemoveToolpath {
            id,
            toolpath: Box::new(toolpath),
        });
    }

    pub fn replace_toolpath(&mut self, id: EntityId, after: Toolpath) {
        let before = self
            .toolpath(id)
            .unwrap_or_else(|| panic!("replace_toolpath: {} does not exist", id.0))
            .clone();
        self.pending_toolpaths.insert(id, Some(after.clone()));
        self.edits.push(Edit::ReplaceToolpath {
            id,
            before: Box::new(before),
            after: Box::new(after),
        });
    }

    pub fn add_component(&mut self, component: Component) -> EntityId {
        let id = EntityId(self.next_id);
        self.next_id += 1;
        self.pending_components.insert(id, Some(component.clone()));
        self.edits.push(Edit::AddComponent {
            id,
            component: Box::new(component),
        });
        id
    }

    pub fn add_bitmap(&mut self, bitmap: Bitmap) -> EntityId {
        let id = EntityId(self.next_id);
        self.next_id += 1;
        self.edits.push(Edit::AddBitmap {
            id,
            bitmap: Box::new(bitmap),
        });
        id
    }

    /// Removes bitmap `id` (as it is in the document).
    pub fn remove_bitmap(&mut self, id: EntityId) {
        let bitmap = self.doc.bitmaps[&id].clone();
        self.edits.push(Edit::RemoveBitmap {
            id,
            bitmap: Box::new(bitmap),
        });
    }

    /// Replaces bitmap `id` (as it is in the document) with `after`.
    pub fn replace_bitmap(&mut self, id: EntityId, after: Bitmap) {
        let before = self.doc.bitmaps[&id].clone();
        self.edits.push(Edit::ReplaceBitmap {
            id,
            before: Box::new(before),
            after: Box::new(after),
        });
    }

    /// Current component `id` as seen by this change so far.
    pub fn component(&self, id: EntityId) -> Option<&Component> {
        shadowed(&self.pending_components, &self.doc.components, id)
    }

    pub fn remove_component(&mut self, id: EntityId) {
        let component = self
            .component(id)
            .unwrap_or_else(|| panic!("remove_component: {} does not exist", id.0))
            .clone();
        self.pending_components.insert(id, None);
        self.edits.push(Edit::RemoveComponent {
            id,
            component: Box::new(component),
        });
    }

    pub fn replace_component(&mut self, id: EntityId, after: Component) {
        let before = self
            .component(id)
            .unwrap_or_else(|| panic!("replace_component: {} does not exist", id.0))
            .clone();
        self.pending_components.insert(id, Some(after.clone()));
        self.edits.push(Edit::ReplaceComponent {
            id,
            before: Box::new(before),
            after: Box::new(after),
        });
    }

    fn vector_layer(&self, id: EntityId) -> Option<LayerId> {
        match self.vector_layers.get(&id) {
            Some(layer) => *layer,
            None => self.doc.vector_layers.get(&id).copied(),
        }
    }

    fn set_vector_layer(&mut self, id: EntityId, after: Option<LayerId>) {
        let before = self.vector_layer(id);
        if before != after {
            self.vector_layers.insert(id, after);
            self.edits.push(Edit::SetLayer { id, before, after });
        }
    }

    /// Puts vector `id` in `group`, or takes it out of any (`None`).
    pub fn set_group(&mut self, id: EntityId, group: Option<u64>) {
        let before = self.doc.vector_groups.get(&id).copied();
        if before != group {
            self.edits.push(Edit::SetGroup {
                id,
                before,
                after: group,
            });
        }
    }

    pub fn move_to_layer(&mut self, id: EntityId, layer: LayerId) {
        self.set_vector_layer(id, Some(layer));
    }

    /// A new layer on top of the others.
    pub fn add_layer(&mut self, name: &str) -> LayerId {
        let id = LayerId(self.layers.iter().map(|l| l.id.0).max().unwrap_or(0) + 1);
        let layer = Layer::new(id, name);
        self.edits.push(Edit::AddLayer {
            index: self.layers.len(),
            layer: layer.clone(),
        });
        self.layers.push(layer);
        id
    }

    /// Removes an empty layer, first making another layer current if it was.
    pub fn remove_layer(&mut self, id: LayerId) {
        let index = self
            .layers
            .iter()
            .position(|l| l.id == id)
            .unwrap_or_else(|| panic!("remove_layer: layer {} does not exist", id.0));
        if self.current_layer == id
            && let Some(other) = self.layers.iter().find(|l| l.id != id).map(|l| l.id)
        {
            self.set_current_layer(other);
        }
        let layer = self.layers.remove(index);
        self.edits.push(Edit::RemoveLayer { index, layer });
    }

    /// Renames, hides or locks a layer: `after` replaces the layer with the same id.
    pub fn replace_layer(&mut self, after: Layer) {
        let current = self
            .layers
            .iter_mut()
            .find(|l| l.id == after.id)
            .unwrap_or_else(|| panic!("replace_layer: layer {} does not exist", after.id.0));
        let before = std::mem::replace(current, after.clone());
        self.edits.push(Edit::ReplaceLayer { before, after });
    }

    pub fn set_current_layer(&mut self, after: LayerId) {
        let before = std::mem::replace(&mut self.current_layer, after);
        if before != after {
            self.edits.push(Edit::SetCurrentLayer { before, after });
        }
    }

    /// Marks `group` as text made from `after` (Draw Text's settings), or not text.
    pub fn set_text(&mut self, group: u64, after: Option<serde_json::Value>) {
        let before = self.doc.texts.get(&group).cloned();
        if before != after {
            self.edits.push(Edit::SetText {
                group,
                before,
                after,
            });
        }
    }

    pub fn set_notes(&mut self, after: &str) {
        if self.doc.notes != after {
            self.edits.push(Edit::SetNotes {
                before: self.doc.notes.clone(),
                after: after.into(),
            });
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
