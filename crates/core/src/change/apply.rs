//! Applying a change to a document, and putting it back when an edit fails partway.

use super::{
    ApplyError, BTreeMap, Bitmap, Change, Component, DAffine2, Document, Edit, EntityId, Level,
    Path, Point, Sheet, SheetObjects, Toolpath,
};

/// What [`Document::apply`] puts back when an edit fails partway: the document apart from
/// its heavy maps, and of those maps (models, pictures, toolpaths, the other sheets' objects)
/// only the entries the change touches, so a stroke on one model never copies the others.
pub(super) struct Kept {
    light: Document,
    components: Vec<(EntityId, Option<Component>)>,
    bitmaps: Vec<(EntityId, Option<Bitmap>)>,
    toolpaths: Vec<(EntityId, Option<Toolpath>)>,
    /// The sheets and the active one, when the change replaces them.
    sheets: Option<(Vec<Sheet>, usize)>,
    /// The other side, when the change turns the job over.
    side: Option<(bool, SheetObjects)>,
}

impl Kept {
    pub(super) fn before(doc: &mut Document, change: &Change) -> Self {
        let (components, bitmaps, toolpaths, sheets, other_side) = (
            std::mem::take(&mut doc.components),
            std::mem::take(&mut doc.bitmaps),
            std::mem::take(&mut doc.toolpaths),
            std::mem::take(&mut doc.sheets),
            std::mem::take(&mut doc.other_side),
        );
        let light = doc.clone();
        doc.components = components;
        doc.bitmaps = bitmaps;
        doc.toolpaths = toolpaths;
        doc.sheets = sheets;
        doc.other_side = other_side;
        let mut kept = Self {
            light,
            components: vec![],
            bitmaps: vec![],
            toolpaths: vec![],
            sheets: None,
            side: None,
        };
        for edit in &change.edits {
            match edit {
                Edit::AddComponent { id, .. }
                | Edit::RemoveComponent { id, .. }
                | Edit::ReplaceComponent { id, .. } => {
                    kept.components.push((*id, doc.components.get(id).cloned()));
                }
                Edit::AddBitmap { id, .. }
                | Edit::RemoveBitmap { id, .. }
                | Edit::ReplaceBitmap { id, .. } => {
                    kept.bitmaps.push((*id, doc.bitmaps.get(id).cloned()));
                }
                Edit::AddToolpath { id, .. }
                | Edit::RemoveToolpath { id, .. }
                | Edit::ReplaceToolpath { id, .. } => {
                    kept.toolpaths.push((*id, doc.toolpaths.get(id).cloned()));
                }
                Edit::SetSheets { .. } if kept.sheets.is_none() => {
                    kept.sheets = Some((doc.sheets.clone(), doc.active_sheet));
                }
                Edit::SetSide { .. } if kept.side.is_none() => {
                    kept.side = Some((doc.bottom_side, doc.other_side.clone()));
                }
                _ => {}
            }
        }
        kept
    }

    /// The document as it was before the change began.
    pub(super) fn restore(self, doc: &mut Document) {
        let (components, bitmaps, toolpaths, sheets, other_side) = (
            std::mem::take(&mut doc.components),
            std::mem::take(&mut doc.bitmaps),
            std::mem::take(&mut doc.toolpaths),
            std::mem::take(&mut doc.sheets),
            std::mem::take(&mut doc.other_side),
        );
        *doc = self.light;
        doc.components = components;
        doc.bitmaps = bitmaps;
        doc.toolpaths = toolpaths;
        doc.sheets = sheets;
        doc.other_side = other_side;
        // the first entry kept for an id is what was there before any edit touched it
        for (id, before) in self.components.into_iter().rev() {
            put_back(&mut doc.components, id, before);
        }
        for (id, before) in self.bitmaps.into_iter().rev() {
            put_back(&mut doc.bitmaps, id, before);
        }
        for (id, before) in self.toolpaths.into_iter().rev() {
            put_back(&mut doc.toolpaths, id, before);
        }
        if let Some((sheets, active)) = self.sheets {
            (doc.sheets, doc.active_sheet) = (sheets, active);
        }
        if let Some(side) = self.side {
            (doc.bottom_side, doc.other_side) = side;
        }
    }
}

pub(super) fn put_back<T>(map: &mut BTreeMap<EntityId, T>, id: EntityId, before: Option<T>) {
    match before {
        Some(value) => {
            map.insert(id, value);
        }
        None => {
            map.remove(&id);
        }
    }
}

impl Document {
    /// Applies every edit or none of them.
    pub fn apply(&mut self, change: &Change) -> Result<(), ApplyError> {
        let kept = Kept::before(self, change);
        for edit in &change.edits {
            if let Err(e) = self.apply_edit(edit) {
                kept.restore(self);
                return Err(e);
            }
        }
        Ok(())
    }

    /// The component and bitmap edits.
    /// The edits of a whole-document setting: notes, variables, keep-out zones, guides.
    /// Like the selection they are not checked against `before`.
    pub(super) fn apply_setting(&mut self, edit: &Edit) {
        match edit {
            Edit::SetNotes { after, .. } => self.notes.clone_from(after),
            Edit::SetNotesInGcode { after, .. } => self.notes_in_gcode = *after,
            Edit::SetVariables { after, .. } => self.variables.clone_from(after),
            Edit::SetKeepOut { after, .. } => self.keep_out.clone_from(after),
            Edit::SetGuides { after, .. } => self.guides.clone_from(after),
            Edit::SetConstraints { after, .. } => self.constraints.clone_from(after),
            Edit::SetToolpathGroups { after, .. } => self.toolpath_groups.clone_from(after),
            Edit::SetToolpathGroupParents { after, .. } => {
                self.toolpath_group_parents.clone_from(after);
            }
            Edit::SetTiling {
                after, kept_after, ..
            } => {
                self.tiling = *after;
                self.tiling_kept = *kept_after;
            }
            _ => unreachable!("only settings come here"),
        }
    }

    pub(super) fn apply_model_edit(&mut self, edit: &Edit) -> Result<(), ApplyError> {
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

    pub(super) fn set_text(&mut self, group: u64, text: Option<serde_json::Value>) {
        match text {
            Some(text) => self.texts.insert(group, text),
            None => self.texts.remove(&group),
        };
    }

    pub(super) fn set_level(
        &mut self,
        id: u64,
        before: Option<&Level>,
        after: Option<&Level>,
    ) -> Result<(), ApplyError> {
        if self.levels.get(&id) != before {
            return Err(ApplyError::LevelConflict(id));
        }
        match after {
            Some(level) => self.levels.insert(id, level.clone()),
            None => self.levels.remove(&id),
        };
        Ok(())
    }

    #[expect(clippy::too_many_lines, reason = "one arm per kind of edit")]
    pub(super) fn apply_edit(&mut self, edit: &Edit) -> Result<(), ApplyError> {
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
            Edit::SetGroupParent {
                group,
                before,
                after,
            } => {
                if self.group_parents.get(group) != before.as_ref() {
                    return Err(ApplyError::GroupConflict(*group));
                }
                match after {
                    Some(parent) => self.group_parents.insert(*group, *parent),
                    None => self.group_parents.remove(group),
                };
            }
            Edit::SetConstruction { id, before, after } => {
                if self.construction.contains(id) != *before {
                    return Err(ApplyError::Conflict(*id));
                }
                if *after {
                    self.construction.insert(*id);
                } else {
                    self.construction.remove(id);
                }
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
                let holds_vectors = self.vectors_on(layer.id).next().is_some()
                    || !self.layer_used_elsewhere(layer.id).is_empty();
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
            Edit::MoveLayer { from, to } => {
                if *from >= self.layers.len() || *to >= self.layers.len() {
                    return Err(ApplyError::LayerOrder);
                }
                let layer = self.layers.remove(*from);
                self.layers.insert(*to, layer);
            }
            // Like the selection, the current layer is view state: `before` is not checked.
            Edit::SetCurrentLayer { after, .. } => {
                if self.layer(*after).is_none() {
                    return Err(ApplyError::MissingLayer(*after));
                }
                self.current_layer = *after;
            }
            Edit::SetNotes { .. }
            | Edit::SetNotesInGcode { .. }
            | Edit::SetVariables { .. }
            | Edit::SetKeepOut { .. }
            | Edit::SetGuides { .. }
            | Edit::SetConstraints { .. }
            | Edit::SetToolpathGroups { .. }
            | Edit::SetToolpathGroupParents { .. }
            | Edit::SetTiling { .. } => self.apply_setting(edit),
            Edit::SetText { group, after, .. } => self.set_text(*group, after.clone()),
            Edit::SetPartName { group, after, .. } => match after {
                Some(name) => {
                    self.part_names.insert(*group, name.clone());
                }
                None => {
                    self.part_names.remove(group);
                }
            },
            Edit::SetLevel { id, before, after } => {
                self.set_level(*id, before.as_ref(), after.as_ref())?;
            }
            Edit::SetSheets { before, after } => self.set_sheets(before, after)?,
            Edit::SetSide { after, .. } => {
                (self.bottom_side, self.other_side) = after.clone();
            }
        }
        Ok(())
    }
}

pub(super) fn insert_new<T>(
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

/// How far `t` moves things, when all it does is move them.
pub(super) fn shift_of(t: &DAffine2) -> Option<Point> {
    (t.matrix2 == glam::DMat2::IDENTITY).then_some(t.translation)
}

/// Text settings `text` for its letters moved by `by`: its anchor, and the curve it was laid
/// on, go with them. `None` for another tool's settings moved anywhere (a dimension's name
/// their tool under `command`): they point at what was measured, which the copy is not at.
pub(super) fn shifted_text(text: &serde_json::Value, by: Point) -> Option<serde_json::Value> {
    if text.get("command").is_some() && by != Point::ZERO {
        return None;
    }
    let mut text = text.clone();
    if let (Some(x), Some(y)) = (text["anchor"]["x"].as_f64(), text["anchor"]["y"].as_f64()) {
        text["anchor"] = serde_json::json!({ "x": x + by.x, "y": y + by.y });
    }
    let curve = text.get("on_curve").and_then(|o| o.get("curve"));
    if let Some(curve) = curve.and_then(|c| serde_json::from_value::<Path>(c.clone()).ok()) {
        let moved = curve.transformed(&DAffine2::from_translation(by));
        text["on_curve"]["curve"] = serde_json::to_value(moved).expect("paths serialise");
    }
    Some(text)
}

/// `id` as a change has left it (`pending`, where `None` is removed), else as in `doc`.
pub(super) fn shadowed<'a, T>(
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
pub(super) fn remove_matching<T: PartialEq>(
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
pub(super) fn replace_matching<T: PartialEq + Clone>(
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
