//! Invertible edits. Every document mutation is a [`Change`]; undo applies its inverse.

use crate::CommandError;
use crate::bitmap::Bitmap;
use crate::component::{Component, Level};
use crate::document::{
    Document, EntityId, Guide, Job, Layer, LayerId, Sheet, SheetObjects, StoredVector,
};
use crate::geometry::{Path, Point, Rect};
use crate::selection::Selection;
use crate::toolpath::Toolpath;
use glam::DAffine2;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fmt;
mod apply;
mod builder;
use apply::{shadowed, shift_of, shifted_text};

/// The order spaced objects are laid out in, from the low end (left, bottom) of the span.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Sequence {
    /// Where they already are
    ByPosition,
    /// The first picked at the low end
    PickedFromLow,
    /// The first picked at the high end
    PickedFromHigh,
}

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
    /// Puts a group inside another group or takes it out (`None`).
    SetGroupParent {
        group: u64,
        before: Option<u64>,
        after: Option<u64>,
    },
    /// Makes a vector a construction line (used only to place things, never cut) or a
    /// real one again.
    SetConstruction {
        id: EntityId,
        before: bool,
        after: bool,
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
    /// The layer at `from` in the list moved to `to` (the order they are listed and drawn).
    MoveLayer {
        from: usize,
        to: usize,
    },
    SetCurrentLayer {
        before: LayerId,
        after: LayerId,
    },
    SetNotes {
        before: String,
        after: String,
    },
    /// Whether the notes go at the top of the G-code.
    SetNotesInGcode {
        before: bool,
        after: bool,
    },
    /// A group's text settings (what Draw Text made it from); `None` when it is not text.
    SetText {
        group: u64,
        before: Option<serde_json::Value>,
        after: Option<serde_json::Value>,
    },
    /// A part's name, by its vector group; `None` when it has none.
    SetPartName {
        group: u64,
        before: Option<String>,
        after: Option<String>,
    },
    /// Adds, changes or removes (`None`) a level of the component tree.
    SetLevel {
        id: u64,
        before: Option<Level>,
        after: Option<Level>,
    },
    /// Turns toolpath tiling on (with its settings) or off, and the settings kept while it
    /// is off.
    SetTiling {
        before: Option<crate::tiling::Tiling>,
        after: Option<crate::tiling::Tiling>,
        #[serde(default)]
        kept_before: Option<crate::tiling::Tiling>,
        #[serde(default)]
        kept_after: Option<crate::tiling::Tiling>,
    },
    /// Replaces the guide lines.
    SetGuides {
        before: Vec<Guide>,
        after: Vec<Guide>,
    },
    /// Replaces the toolpath group list.
    SetToolpathGroups {
        before: Vec<String>,
        after: Vec<String>,
    },
    /// Replaces which toolpath group each sub-group sits in.
    SetToolpathGroupParents {
        before: BTreeMap<String, String>,
        after: BTreeMap<String, String>,
    },
    /// Replaces the keep-out zones.
    SetKeepOut {
        before: Vec<crate::keep_out::KeepOutZone>,
        after: Vec<crate::keep_out::KeepOutZone>,
    },
    /// Replaces the document variables.
    SetVariables {
        before: BTreeMap<String, f64>,
        after: BTreeMap<String, f64>,
    },
    /// Double-sided jobs: which side is worked on (bottom or not) and what is on the other.
    SetSide {
        before: (bool, SheetObjects),
        after: (bool, SheetObjects),
    },
    /// Replaces the points kept on their snaps.
    SetConstraints {
        before: Vec<crate::constraint::Constraint>,
        after: Vec<crate::constraint::Constraint>,
    },
    /// Replaces the sheet list and which sheet is active.
    SetSheets {
        before: (Vec<Sheet>, usize),
        after: (Vec<Sheet>, usize),
    },
}

impl Edit {
    #[must_use]
    #[expect(clippy::too_many_lines, reason = "one arm for each kind of edit")]
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
            Self::SetConstruction { id, before, after } => Self::SetConstruction {
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
            Self::MoveLayer { from, to } => Self::MoveLayer { from: to, to: from },
            Self::SetCurrentLayer { before, after } => Self::SetCurrentLayer {
                before: after,
                after: before,
            },
            Self::SetTiling {
                before,
                after,
                kept_before,
                kept_after,
            } => Self::SetTiling {
                before: after,
                after: before,
                kept_before: kept_after,
                kept_after: kept_before,
            },
            Self::SetGuides { before, after } => Self::SetGuides {
                before: after,
                after: before,
            },
            Self::SetConstraints { before, after } => Self::SetConstraints {
                before: after,
                after: before,
            },
            Self::SetToolpathGroups { before, after } => Self::SetToolpathGroups {
                before: after,
                after: before,
            },
            Self::SetToolpathGroupParents { before, after } => Self::SetToolpathGroupParents {
                before: after,
                after: before,
            },
            Self::SetKeepOut { before, after } => Self::SetKeepOut {
                before: after,
                after: before,
            },
            Self::SetVariables { before, after } => Self::SetVariables {
                before: after,
                after: before,
            },
            Self::SetSide { before, after } => Self::SetSide {
                before: after,
                after: before,
            },
            Self::SetSheets { before, after } => Self::SetSheets {
                before: after,
                after: before,
            },
            Self::SetNotes { before, after } => Self::SetNotes {
                before: after.clone(),
                after: before.clone(),
            },
            Self::SetNotesInGcode { before, after } => Self::SetNotesInGcode {
                before: after,
                after: before,
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
            Self::SetPartName {
                group,
                before,
                after,
            } => Self::SetPartName {
                group,
                before: after,
                after: before,
            },
            Self::SetGroupParent {
                group,
                before,
                after,
            } => Self::SetGroupParent {
                group,
                before: after,
                after: before,
            },
            Self::SetLevel { id, before, after } => Self::SetLevel {
                id,
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
    MissingLayer(LayerId),
    /// A layer move from or to a place past the end of the list.
    LayerOrder,
    LayerExists(LayerId),
    LayerConflict(LayerId),
    /// Vectors are still on it, or it is the last layer.
    LayerInUse(LayerId),
    /// The level was changed by something else.
    LevelConflict(u64),
    /// The group was put in or taken out of another group by something else.
    GroupConflict(u64),
}

impl fmt::Display for ApplyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Missing(id) => write!(f, "entity {} does not exist", id.0),
            Self::Exists(id) => write!(f, "entity {} already exists", id.0),
            Self::GroupConflict(g) => write!(f, "group {g} was changed by something else"),
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
            Self::LevelConflict(id) => write!(f, "level {id} was changed by something else"),
            Self::LayerOrder => f.write_str("a layer was moved past the end of the list"),
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
    /// What the command would have the person know about what it made (an imported model's
    /// undercuts...), shown with its result; not part of the document.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub notes: Vec<String>,
    /// An import that had to guess its drawing's units (a DXF that doesn't say them), so the
    /// person can be asked; not part of the document.
    #[serde(skip)]
    pub unit_guess: Option<UnitGuess>,
}

/// The units an import took its drawing to be in, when the file doesn't say, and the
/// drawing's size in its own units: read in other units it is that size times their length.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct UnitGuess {
    pub taken: crate::document::Units,
    pub size: glam::DVec2,
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
            construction: BTreeMap::new(),
            levels: BTreeMap::new(),
            sheets: (doc.sheets.clone(), doc.active_sheet),
            side: (doc.bottom_side, doc.other_side.clone()),
            job: doc.job,
            edits: vec![],
            notes: vec![],
        }
    }

    /// Adds `path` and makes it the whole selection: what every drawing command does.
    pub fn add_and_select(doc: &Document, path: Path) -> Self {
        let mut c = Self::builder(doc);
        let id = c.add(path);
        c.select([id]);
        c.finish()
    }

    /// Transforms each selected item (a lone path, or a group's selected paths together) and
    /// component by `f(its bounds)`; the shared core of move, scale, rotate, mirror and align. Nothing selected is an error; identity
    /// transforms make no edit.
    pub fn transform_selected(
        doc: &Document,
        f: impl Fn(Rect) -> DAffine2,
    ) -> Result<Self, CommandError> {
        Self::transform_items(doc, &doc.selected_items(false), f)
    }

    /// [`Self::transform_selected`] with the selected vectors taken as `items` (see
    /// [`Document::selected_items`]): every member of an item gets the one transform
    /// worked out from the whole item's bounds.
    pub fn transform_items(
        doc: &Document,
        items: &[Vec<EntityId>],
        f: impl Fn(Rect) -> DAffine2,
    ) -> Result<Self, CommandError> {
        let components: Vec<_> = doc
            .selected_components
            .iter()
            .filter(|id| doc.components.contains_key(id))
            .copied()
            .collect();
        if doc.selection.is_empty() && components.is_empty() {
            return Err(CommandError::NothingSelected);
        }
        let mut c = Self::builder(doc);
        for item in items {
            let Some(bounds) = item
                .iter()
                .map(|id| doc.vectors[id].bounds())
                .reduce(Rect::union)
            else {
                continue;
            };
            let t = f(bounds);
            if t != DAffine2::IDENTITY {
                for id in item {
                    c.replace(*id, doc.vectors[id].transformed(&t));
                }
            }
        }
        for id in components {
            let component = &doc.components[&id];
            let t = f(component.extent());
            if t != DAffine2::IDENTITY {
                let moved = component.transformed(&t);
                // a texture area's tiles keep their size: its new box is tiled again
                let seed_of = |seed| doc.components.get(&seed).cloned();
                let again = crate::texture_area::retiled(&moved, seed_of);
                c.replace_component(id, again.unwrap_or(moved));
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
        Self::transform_selected(doc, |bounds| {
            DAffine2::from_translation(offset(bounds, sel))
        })
    }

    /// Spaces the selected objects (a group counts as one) evenly along x (or y when `vertical`) in `sequence`: the
    /// outermost stay put; or with `inside_last`, the last one picked stays and the others
    /// are spread inside it, the same gap at its edges as between them.
    pub fn space_selected(
        doc: &Document,
        vertical: bool,
        sequence: Sequence,
        inside_last: bool,
    ) -> Result<Self, CommandError> {
        let along = |r: Rect| {
            if vertical {
                (r.min.y, r.max.y)
            } else {
                (r.min.x, r.max.x)
            }
        };
        // a group spaces as one object
        let mut items: Vec<(Vec<EntityId>, f64, f64)> = doc
            .selected_items(false)
            .into_iter()
            .map(|item| {
                let bounds = item
                    .iter()
                    .map(|id| doc.vectors[id].bounds())
                    .reduce(Rect::union)
                    .expect("an item has a member");
                let (lo, hi) = along(bounds);
                (item, lo, hi)
            })
            .collect();
        let frame = if inside_last { items.pop() } else { None };
        if frame.is_some() && items.is_empty() {
            return Err(CommandError::Failed(
                "select the objects to space, then the vector to space them inside".into(),
            ));
        }
        if frame.is_none() && items.len() < 3 {
            return Err(CommandError::Failed(
                "select at least three objects to space".into(),
            ));
        }
        let start = items.iter().map(|i| i.1).fold(f64::INFINITY, f64::min);
        let end = items.iter().map(|i| i.2).fold(f64::NEG_INFINITY, f64::max);
        match sequence {
            Sequence::ByPosition => items.sort_by(|a, b| (a.1 + a.2).total_cmp(&(b.1 + b.2))),
            Sequence::PickedFromLow => {}
            Sequence::PickedFromHigh => items.reverse(),
        }
        let sizes: f64 = items.iter().map(|(_, lo, hi)| hi - lo).sum();
        #[expect(clippy::cast_precision_loss, reason = "a few objects")]
        let (gap, mut at) = match frame {
            // a gap at each edge of the frame as well as between them
            Some((_, lo, hi)) => {
                let gap = (hi - lo - sizes) / (items.len() + 1) as f64;
                (gap, lo + gap)
            }
            None => ((end - start - sizes) / (items.len() - 1) as f64, start),
        };
        let mut c = Self::builder(doc);
        for (item, lo, hi) in &items {
            let by = at - lo;
            if by.abs() > 1e-12 {
                let shift = if vertical {
                    Point::new(0.0, by)
                } else {
                    Point::new(by, 0.0)
                };
                for &id in item {
                    c.replace(
                        id,
                        doc.vectors[&id].transformed(&DAffine2::from_translation(shift)),
                    );
                }
            }
            at += hi - lo + gap;
        }
        Ok(c.finish())
    }

    /// Adds a copy of every selected vector under each of `transforms`, on the same layer as
    /// its original, and selects the originals and all the copies.
    pub fn copy_selected(doc: &Document, transforms: &[DAffine2]) -> Self {
        let originals: Vec<EntityId> = doc.selection.iter().copied().collect();
        Self::copy_vectors(doc, &originals, transforms)
    }

    /// [`Self::copy_selected`], with the originals and all the copies made one new group.
    pub fn copy_selected_grouped(doc: &Document, transforms: &[DAffine2]) -> Self {
        let originals: Vec<EntityId> = doc.selection.iter().copied().collect();
        let mut c = Self::builder(doc);
        let all = c.copies_grouped(doc, &originals, transforms, true);
        let group = c.next_group();
        // each item (a group, kept whole with any inside it, or a lone vector) goes in it
        for &id in &all {
            match c.top_group_now(id) {
                Some(top) if top != group => c.set_group_parent(top, Some(group)),
                Some(_) => {}
                None => c.set_group(id, Some(group)),
            }
        }
        c.select(all);
        c.finish()
    }

    /// Adds a copy of each of `originals` under each of `transforms`, on the same layer as
    /// its original, and selects the originals and all the copies.
    pub fn copy_vectors(doc: &Document, originals: &[EntityId], transforms: &[DAffine2]) -> Self {
        let mut c = Self::builder(doc);
        let selected = c.copies(doc, originals, transforms);
        c.select(selected);
        c.finish()
    }

    /// Paths this change creates, e.g. to preview a command before applying it.
    pub fn added(&self) -> impl Iterator<Item = &Path> {
        self.edits.iter().filter_map(|e| match e {
            Edit::Add { path, .. } => Some(path),
            _ => None,
        })
    }

    /// Replaces `remove` with `add`, selecting what was added: the shape of every boolean,
    /// offset or join result. The results go on the layer of the first original removed (the
    /// current layer when nothing is), and a result identical to an original just keeps it.
    pub fn swap(
        doc: &Document,
        remove: impl IntoIterator<Item = EntityId>,
        add: Vec<Path>,
    ) -> Self {
        let mut c = Self::builder(doc);
        let mut removing: Vec<EntityId> = remove.into_iter().collect();
        let layer = removing.first().map(|&id| doc.layer_of(id));
        // a result the same as an original keeps the original, its id, layer and group
        let ids: Vec<Option<EntityId>> = add
            .iter()
            .map(|p| {
                let at = removing.iter().position(|id| doc.vectors[id] == *p)?;
                Some(removing.remove(at))
            })
            .collect();
        for id in removing {
            c.remove(id);
        }
        let ids: Vec<EntityId> = add
            .into_iter()
            .zip(ids)
            .map(|(p, kept)| {
                kept.unwrap_or_else(|| {
                    let id = c.add(p);
                    if let Some(layer) = layer {
                        c.move_to_layer(id, layer);
                    }
                    id
                })
            })
            .collect();
        c.select(ids);
        c.finish()
    }

    pub fn is_empty(&self) -> bool {
        self.edits.is_empty()
    }

    /// This change with the toolpaths it adds put in `group` (the group picked in the list).
    #[must_use]
    pub fn into_toolpath_group(mut self, group: &str) -> Self {
        for edit in &mut self.edits {
            if let Edit::AddToolpath { toolpath, .. } = edit {
                toolpath.group = Some(group.to_owned());
            }
        }
        self
    }

    /// This change's new toolpaths put in place of `old` (what a toolpath form made last
    /// time), in turn: each takes the id, so its place in the cut order, and the group of
    /// the one it replaces. Old ones left over are taken away and new ones left over added,
    /// so pressing Calculate again never leaves a second copy. Any of `old` no longer in
    /// `doc` is left out.
    #[must_use]
    pub fn onto_toolpaths(mut self, old: &[EntityId], doc: &Document) -> Self {
        let mut old = old
            .iter()
            .filter_map(|id| Some((*id, doc.toolpaths.get(id)?)));
        for edit in &mut self.edits {
            let Edit::AddToolpath { toolpath, .. } = edit else {
                continue;
            };
            let Some((id, before)) = old.next() else {
                break;
            };
            let mut after = toolpath.as_ref().clone();
            after.group.clone_from(&before.group);
            *edit = Edit::ReplaceToolpath {
                id,
                before: Box::new(before.clone()),
                after: Box::new(after),
            };
        }
        for (id, toolpath) in old {
            self.edits.push(Edit::RemoveToolpath {
                id,
                toolpath: Box::new(toolpath.clone()),
            });
        }
        self
    }

    /// The toolpaths this change adds, and those it replaces others with, as they will be.
    pub fn toolpaths_making(&self) -> impl Iterator<Item = &Toolpath> {
        self.edits.iter().filter_map(|e| match e {
            Edit::AddToolpath { toolpath, .. } => Some(toolpath.as_ref()),
            Edit::ReplaceToolpath { after, .. } => Some(after.as_ref()),
            _ => None,
        })
    }

    /// The toolpaths this change adds or replaces.
    pub fn toolpaths_made(&self) -> Vec<EntityId> {
        self.edits
            .iter()
            .filter_map(|e| match e {
                Edit::AddToolpath { id, .. } | Edit::ReplaceToolpath { id, .. } => Some(*id),
                _ => None,
            })
            .collect()
    }

    /// The toolpaths following a Vector Selector whose vectors this change alters: ones it
    /// adds or takes away from what passes (a vector drawn on the layer, a line closed). A
    /// template's toolpaths then show they want making again once its text is edited.
    fn picks_again(&self, doc: &Document) -> Vec<EntityId> {
        let following: Vec<(EntityId, crate::selector::Selector)> = doc
            .toolpaths
            .iter()
            .filter_map(|(&id, t)| crate::selector::Selector::of(&t.params).map(|s| (id, s)))
            .collect();
        let only_toolpaths_or_selection = self.edits.iter().all(|e| {
            matches!(
                e,
                Edit::Select { .. }
                    | Edit::AddToolpath { .. }
                    | Edit::RemoveToolpath { .. }
                    | Edit::ReplaceToolpath { .. }
            )
        });
        if following.is_empty() || only_toolpaths_or_selection {
            return vec![];
        }
        let mut after = doc.clone();
        if after.apply(self).is_err() {
            return vec![];
        }
        following
            .into_iter()
            .filter(|(_, s)| s.picked(doc) != s.picked(&after))
            .map(|(id, _)| id)
            .collect()
    }

    /// `self` with the toolpaths it leaves out of date marked stale: those made from a vector
    /// it changes or removes, those copying (Merged, Array Copy) a toolpath it removes, cuts
    /// differently or leaves out of date, those carving a picture it moves or alters, when it changes the model those that cut the model, and when
    /// it changes the material's thickness, where the model sits, or the rotary or
    /// double-sided set-up, every toolpath (they bake those in when calculated). The marks
    /// are edits of the change, so undoing it undoes them.
    #[must_use]
    pub fn marking_stale(mut self, doc: &Document) -> Self {
        let mut model = false;
        let mut clearing = false;
        let mut material = false;
        let mut vectors = std::collections::BTreeSet::new();
        let mut made = std::collections::BTreeSet::new();
        // toolpaths this change takes away or makes cut differently
        let mut recut = std::collections::BTreeSet::new();
        // pictures this change moves, alters or takes away
        let mut pictures = std::collections::BTreeSet::new();
        for edit in &self.edits {
            match edit {
                Edit::Replace { id, .. }
                | Edit::Remove { id, .. }
                | Edit::SetConstruction { id, .. } => {
                    vectors.insert(*id);
                }
                Edit::ReplaceBitmap { id, .. } | Edit::RemoveBitmap { id, .. } => {
                    pictures.insert(*id);
                }
                Edit::AddComponent { .. }
                | Edit::RemoveComponent { .. }
                | Edit::ReplaceComponent { .. }
                | Edit::SetLevel { .. } => model = true,
                Edit::SetJob { before, after } => {
                    clearing |= before.clearing != after.clearing;
                    model |= before.resolution != after.resolution;
                    material |= before.thickness != after.thickness
                        || before.model_position != after.model_position
                        || before.rotary != after.rotary
                        || before.double_sided != after.double_sided;
                }
                Edit::AddToolpath { id, .. } => {
                    made.insert(*id);
                }
                Edit::RemoveToolpath { id, .. } => {
                    made.insert(*id);
                    recut.insert(*id);
                }
                Edit::ReplaceToolpath { id, before, after } => {
                    made.insert(*id);
                    // shown, hidden or renamed, it cuts the same
                    if before.moves != after.moves
                        || before.tool != after.tool
                        || before.power != after.power
                    {
                        recut.insert(*id);
                    }
                }
                _ => {}
            }
        }
        let picks_again = self.picks_again(doc);
        let mut outdated: std::collections::BTreeSet<EntityId> = doc
            .toolpaths
            .iter()
            .filter(|&(id, toolpath)| {
                material
                    || (model && toolpath.reads_model())
                    || (clearing && toolpath.reads_clearing())
                    || toolpath.vectors.iter().any(|v| vectors.contains(v))
                    || picture_of(toolpath).is_some_and(|b| pictures.contains(&b))
                    || picks_again.contains(id)
            })
            .map(|(&id, _)| id)
            .collect();
        // a Merged or Array Copy toolpath copies the moves of its sources: out of date once
        // one of them cuts differently or goes, or is itself out of date
        loop {
            let more: Vec<EntityId> = doc
                .toolpaths
                .iter()
                .filter(|(id, _)| !outdated.contains(id))
                .filter(|(_, t)| {
                    sources_of(t)
                        .iter()
                        .any(|s| recut.contains(s) || outdated.contains(s))
                })
                .map(|(&id, _)| id)
                .collect();
            if more.is_empty() {
                break;
            }
            outdated.extend(more);
        }
        self.keep_copies_behind_stale(doc, &outdated, &recut);
        for id in outdated {
            let toolpath = &doc.toolpaths[&id];
            if !toolpath.stale && !made.contains(&id) {
                let after = Toolpath {
                    stale: true,
                    ..toolpath.clone()
                };
                self.edits.push(Edit::ReplaceToolpath {
                    id,
                    before: Box::new(toolpath.clone()),
                    after: Box::new(after),
                });
            }
        }
        self
    }

    /// Marks out of date each Merged or Array Copy toolpath this change makes again while a
    /// source of it is still out of date, or is cut again only after it: it holds old moves.
    fn keep_copies_behind_stale(
        &mut self,
        doc: &Document,
        outdated: &std::collections::BTreeSet<EntityId>,
        recut: &std::collections::BTreeSet<EntityId>,
    ) {
        let mut last = std::collections::BTreeMap::new();
        for (i, edit) in self.edits.iter().enumerate() {
            if let Edit::AddToolpath { id, .. }
            | Edit::RemoveToolpath { id, .. }
            | Edit::ReplaceToolpath { id, .. } = edit
            {
                last.insert(*id, i);
            }
        }
        let still_stale = |s: EntityId, edits: &[Edit]| match last.get(&s) {
            Some(&i) => match &edits[i] {
                Edit::AddToolpath { toolpath, .. } => toolpath.stale,
                Edit::ReplaceToolpath { after, .. } => after.stale,
                _ => false,
            },
            None => doc
                .toolpaths
                .get(&s)
                .is_some_and(|t| t.stale || outdated.contains(&s)),
        };
        let behind: Vec<usize> = last
            .values()
            .copied()
            .filter(|&i| match &self.edits[i] {
                Edit::AddToolpath { toolpath: t, .. } | Edit::ReplaceToolpath { after: t, .. } => {
                    !t.stale
                        && sources_of(t).iter().any(|&s| {
                            still_stale(s, &self.edits)
                                || last.get(&s).is_some_and(|&j| j > i && recut.contains(&s))
                        })
                }
                _ => false,
            })
            .collect();
        for i in behind {
            if let Edit::AddToolpath { toolpath: t, .. } | Edit::ReplaceToolpath { after: t, .. } =
                &mut self.edits[i]
            {
                t.stale = true;
            }
        }
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
            ..Self::default()
        }
    }

    #[must_use]
    pub fn then(mut self, next: Self) -> Self {
        self.edits.extend(next.edits);
        self.notes.extend(next.notes);
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
    /// Construction lines as this change has left them, shadowing `doc`.
    construction: BTreeMap<EntityId, bool>,
    /// Levels as this change has left them (`None` = removed), shadowing `doc`.
    levels: BTreeMap<u64, Option<Level>>,
    /// The sheet list and active sheet as this change has left them.
    sheets: (Vec<Sheet>, usize),
    /// The side worked on and the other side's objects as this change has left them.
    side: (bool, SheetObjects),
    /// The job as this change has left it.
    job: Job,
    edits: Vec<Edit>,
    notes: Vec<String>,
}

/// The toolpaths `toolpath` copies its moves from: a Merged or Array Copy toolpath's
/// `sources`.
pub(crate) fn sources_of(toolpath: &Toolpath) -> Vec<EntityId> {
    toolpath
        .params
        .get("sources")
        .and_then(|s| serde_json::from_value(s.clone()).ok())
        .unwrap_or_default()
}

/// The picture a Photo V-Carve or Sketch Carving toolpath was made from: its `bitmap`.
fn picture_of(toolpath: &Toolpath) -> Option<EntityId> {
    serde_json::from_value(toolpath.params.get("bitmap")?.clone()).ok()?
}

/// Undo/redo stacks of applied changes.
#[derive(Debug, Default)]
pub struct History {
    undo: Vec<Change>,
    redo: Vec<Change>,
    revision: u64,
    /// What the last change applied would have the person know, with what keeping the
    /// snapped points added (see [`crate::constraint::keeping`]).
    last_notes: Vec<String>,
}

impl History {
    /// Applies `change` and records it; empty changes are not recorded.
    pub fn apply(&mut self, doc: &mut Document, change: Change) -> Result<(), ApplyError> {
        if change.is_empty() {
            return Ok(());
        }
        // points snapped to what moved follow it
        let change = crate::constraint::keeping(doc, change).marking_stale(doc);
        doc.apply(&change)?;
        self.last_notes.clone_from(&change.notes);
        self.undo.push(change);
        self.redo.clear();
        self.revision += 1;
        Ok(())
    }

    /// The notes of the change last applied (empty after undo and redo).
    pub fn last_notes(&self) -> &[String] {
        &self.last_notes
    }

    pub fn undo(&mut self, doc: &mut Document) -> bool {
        self.last_notes.clear();
        let stepped = Self::step(&mut self.undo, &mut self.redo, doc, true);
        self.revision += u64::from(stepped);
        stepped
    }

    pub fn redo(&mut self, doc: &mut Document) -> bool {
        self.last_notes.clear();
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

    /// How many changes can be undone: where to undo back to, to take back what was done
    /// since.
    pub fn undo_depth(&self) -> usize {
        self.undo.len()
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
