//! Building a change: [`ChangeBuilder`]'s edits, each tracked against the document as the change has left it.

use super::*;

impl ChangeBuilder<'_> {
    /// Adds a copy of each of `originals` under each of `transforms`, on its original's
    /// layer: the originals then the copies, in order. Each copy of a group is a group of its
    /// own with the original's part name, and stays text when it is only moved.
    pub fn copies(
        &mut self,
        doc: &Document,
        originals: &[EntityId],
        transforms: &[DAffine2],
    ) -> Vec<EntityId> {
        self.copies_grouped(doc, originals, transforms, true)
    }

    pub(super) fn copies_grouped(
        &mut self,
        doc: &Document,
        originals: &[EntityId],
        transforms: &[DAffine2],
        regroup: bool,
    ) -> Vec<EntityId> {
        let mut all = originals.to_vec();
        let set: std::collections::BTreeSet<EntityId> = originals.iter().copied().collect();
        let chains: Vec<Vec<u64>> = originals
            .iter()
            .map(|&id| {
                if regroup {
                    doc.chain_within(id, &set)
                } else {
                    Vec::new()
                }
            })
            .collect();
        for t in transforms {
            let paths = originals
                .iter()
                .zip(&chains)
                .map(|(id, chain)| (doc.vectors[id].transformed(t), chain.clone()))
                .collect();
            let copies = self.add_regrouped(paths, &doc.texts, &doc.part_names, shift_of(t));
            for (&id, &copy) in originals.iter().zip(&copies) {
                self.move_to_layer(copy, doc.layer_of(id));
                // a copy of a construction line is one too
                self.set_construction(copy, doc.is_construction(id));
            }
            all.extend(copies);
        }
        all
    }

    /// Adds `paths`, each with the groups it was in where it came from, innermost first
    /// (empty: none). Each of those groups becomes one new group here, held in the new
    /// group of the next one out, with its part name from `names` and, when the copy is
    /// only moved (by `shift`), its text settings from `texts`, moved with it. A dimension's
    /// copy is plain curves unless it lies on the original (Paste).
    pub fn add_regrouped(
        &mut self,
        paths: Vec<(Path, Vec<u64>)>,
        texts: &BTreeMap<u64, serde_json::Value>,
        names: &BTreeMap<u64, String>,
        shift: Option<Point>,
    ) -> Vec<EntityId> {
        let mut new_groups: BTreeMap<u64, u64> = BTreeMap::new();
        let mut ids = Vec::with_capacity(paths.len());
        for (path, chain) in paths {
            let id = self.add(path);
            let mut inner: Option<u64> = None;
            for old in chain {
                let g = if let Some(&g) = new_groups.get(&old) {
                    g
                } else {
                    let g = self.next_group();
                    new_groups.insert(old, g);
                    self.set_part_name(g, names.get(&old).cloned());
                    let text = shift.and_then(|by| shifted_text(texts.get(&old)?, by));
                    self.set_text(g, text);
                    g
                };
                // putting the vector or group in it claims the number for this change
                match inner {
                    None => self.set_group(id, Some(g)),
                    Some(inner) => self.set_group_parent(inner, Some(g)),
                }
                inner = Some(g);
            }
            ids.push(id);
        }
        ids
    }

    /// A group number neither the document nor this change uses yet.
    pub fn next_group(&self) -> u64 {
        let given = self
            .edits
            .iter()
            .filter_map(|e| match e {
                Edit::SetGroup { after, .. } => *after,
                Edit::SetGroupParent { group, after, .. } => Some(after.unwrap_or(0).max(*group)),
                Edit::SetText { group, .. } | Edit::SetPartName { group, .. } => Some(*group),
                _ => None,
            })
            .max()
            .map_or(0, |g| g + 1);
        self.doc.next_group().max(given)
    }

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
        self.set_construction(id, false);
        self.edits.push(Edit::Remove { id, path });
        self.set_vector_layer(id, None);
        // a removed vector leaves its group; the last one out takes the group's text with it
        if let Some(&group) = self.doc.vector_groups.get(&id) {
            self.set_group(id, None);
            let others_left = self.doc.vector_groups.iter().any(|(v, g)| {
                *g == group
                    && *v != id
                    && self.doc.vectors.contains_key(v)
                    && !matches!(self.pending.get(v), Some(None))
            });
            if !others_left {
                self.set_text(group, None);
            }
        }
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

    /// `toolpath` with its rapids routed round the keep-out zones (a laser's with the laser
    /// off), and when clipping is on, what would cut into one left out (laser toolpaths,
    /// whose power runs move for move, and drill banks, whose hits are their moves, not
    /// clipped).
    pub(super) fn kept_out(&self, mut toolpath: Toolpath) -> Toolpath {
        let zones = self.doc.keep_out_zones();
        if !zones.is_empty() {
            // a laser's lines are not clipped (its power goes with each move), only its
            // rapids routed round
            let laser = !toolpath.power.is_empty();
            if self.doc.clip_keep_out && toolpath.drill_bank.is_none() && !laser {
                let radius = toolpath.tool.diameter / 2.0;
                let safe = self.doc.job.clearance;
                toolpath.moves = crate::keep_out::clip(&zones, &toolpath.moves, radius, safe);
            }
            let (moves, origin, _) = crate::keep_out::route_indexed(&zones, &toolpath.moves);
            // the ways round are rapids, with the laser off; every other move keeps its power
            if laser {
                toolpath.power = moves
                    .iter()
                    .zip(&origin)
                    .map(|(m, &o)| {
                        if m.kind == crate::toolpath::MoveKind::Rapid {
                            0.0
                        } else {
                            toolpath.power.get(o).copied().unwrap_or(0.0)
                        }
                    })
                    .collect();
            }
            // a drill bank's plunges keep their moves, and the ways round that lead to them
            if let Some(bank) = &mut toolpath.drill_bank {
                let at = |old: usize| origin.partition_point(|&o| o < old);
                for hit in &mut bank.hits {
                    (hit.start, hit.end) = (at(hit.start), at(hit.end));
                }
            }
            toolpath.moves = moves;
        }
        toolpath
    }

    pub fn add_toolpath(&mut self, toolpath: Toolpath) -> EntityId {
        let toolpath = self.kept_out(toolpath);
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
        let after = self.kept_out(after);
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

    /// The toolpaths `ids` hidden, those shown: ones a Merged or Array Copy toolpath now
    /// cuts, so saving the shown toolpaths does not cut them twice.
    pub fn hide_toolpaths(&mut self, ids: &[EntityId]) {
        for &id in ids {
            if let Some(shown) = self.toolpath(id).filter(|t| t.visible).cloned() {
                self.replace_toolpath(
                    id,
                    Toolpath {
                        visible: false,
                        ..shown
                    },
                );
            }
        }
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

    /// Adds a component made from scratch on the level new components go on
    /// ([`Document::new_component_level`]).
    pub fn add_new_component(&mut self, mut component: Component) -> EntityId {
        component.level = self.doc.new_component_level();
        self.add_component(component)
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

    pub(super) fn vector_layer(&self, id: EntityId) -> Option<LayerId> {
        match self.vector_layers.get(&id) {
            Some(layer) => *layer,
            None => self.doc.vector_layers.get(&id).copied(),
        }
    }

    /// Whether vector `id` is a construction line, as this change has left it.
    pub fn is_construction(&self, id: EntityId) -> bool {
        self.construction
            .get(&id)
            .copied()
            .unwrap_or_else(|| self.doc.is_construction(id))
    }

    /// Makes vector `id` a construction line (`true`) or a real one.
    pub fn set_construction(&mut self, id: EntityId, after: bool) {
        let before = self.is_construction(id);
        if before != after {
            self.construction.insert(id, after);
            self.edits.push(Edit::SetConstruction { id, before, after });
        }
    }

    pub(super) fn set_vector_layer(&mut self, id: EntityId, after: Option<LayerId>) {
        let before = self.vector_layer(id);
        if before != after {
            self.vector_layers.insert(id, after);
            self.edits.push(Edit::SetLayer { id, before, after });
        }
    }

    /// Puts vector `id` in `group`, or takes it out of any (`None`).
    pub fn set_group(&mut self, id: EntityId, group: Option<u64>) {
        // the group as this change has already left it, else as the document has it
        let before = self
            .edits
            .iter()
            .rev()
            .find_map(|e| match e {
                Edit::SetGroup { id: i, after, .. } if *i == id => Some(*after),
                _ => None,
            })
            .unwrap_or_else(|| self.doc.vector_groups.get(&id).copied());
        if before != group {
            self.edits.push(Edit::SetGroup {
                id,
                before,
                after: group,
            });
        }
    }

    /// The outermost group vector `id` is in, as this change has left it.
    pub fn top_group_now(&self, id: EntityId) -> Option<u64> {
        let group_of = |id: EntityId| {
            self.edits
                .iter()
                .rev()
                .find_map(|e| match e {
                    Edit::SetGroup { id: i, after, .. } if *i == id => Some(*after),
                    _ => None,
                })
                .unwrap_or_else(|| self.doc.vector_groups.get(&id).copied())
        };
        let parent_of = |g: u64| {
            self.edits
                .iter()
                .rev()
                .find_map(|e| match e {
                    Edit::SetGroupParent { group, after, .. } if *group == g => Some(*after),
                    _ => None,
                })
                .unwrap_or_else(|| self.doc.group_parents.get(&g).copied())
        };
        let mut at = group_of(id)?;
        let mut seen = vec![at];
        while let Some(parent) = parent_of(at) {
            if seen.contains(&parent) {
                break;
            }
            seen.push(parent);
            at = parent;
        }
        Some(at)
    }

    /// Puts group `group` inside group `parent`, or takes it out of any (`None`).
    pub fn set_group_parent(&mut self, group: u64, parent: Option<u64>) {
        let before = self
            .edits
            .iter()
            .rev()
            .find_map(|e| match e {
                Edit::SetGroupParent {
                    group: g, after, ..
                } if *g == group => Some(*after),
                _ => None,
            })
            .unwrap_or_else(|| self.doc.group_parents.get(&group).copied());
        if before != parent {
            self.edits.push(Edit::SetGroupParent {
                group,
                before,
                after: parent,
            });
        }
    }

    pub fn move_to_layer(&mut self, id: EntityId, layer: LayerId) {
        self.set_vector_layer(id, Some(layer));
    }

    /// A new layer on top of the others.
    /// Moves layer `id` to place `to` in the list.
    pub fn move_layer(&mut self, id: LayerId, to: usize) {
        let Some(from) = self.layers.iter().position(|l| l.id == id) else {
            return;
        };
        let to = to.min(self.layers.len() - 1);
        if from != to {
            let layer = self.layers.remove(from);
            self.layers.insert(to, layer);
            self.edits.push(Edit::MoveLayer { from, to });
        }
    }

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

    /// The layers as this change has left them so far.
    pub fn layers(&self) -> &[Layer] {
        &self.layers
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
        let before = self
            .edits
            .iter()
            .rev()
            .find_map(|e| match e {
                Edit::SetText {
                    group: g, after, ..
                } if *g == group => Some(after.clone()),
                _ => None,
            })
            .unwrap_or_else(|| self.doc.texts.get(&group).cloned());
        if before != after {
            self.edits.push(Edit::SetText {
                group,
                before,
                after,
            });
        }
    }

    /// Names the part vector group `group` belongs to, or (`None`) takes its name away.
    pub fn set_part_name(&mut self, group: u64, after: Option<String>) {
        let before = self
            .edits
            .iter()
            .rev()
            .find_map(|e| match e {
                Edit::SetPartName {
                    group: g, after, ..
                } if *g == group => Some(after.clone()),
                _ => None,
            })
            .unwrap_or_else(|| self.doc.part_names.get(&group).cloned());
        if before != after {
            self.edits.push(Edit::SetPartName {
                group,
                before,
                after,
            });
        }
    }

    /// Level `id` as this change has left it.
    pub fn level(&self, id: u64) -> Option<&Level> {
        match self.levels.get(&id) {
            Some(pending) => pending.as_ref(),
            None => self.doc.levels.get(&id),
        }
    }

    /// Adds, changes or (with `None`) removes level `id`.
    pub fn set_level(&mut self, id: u64, after: Option<Level>) {
        let before = self.level(id).cloned();
        if before != after {
            self.levels.insert(id, after.clone());
            self.edits.push(Edit::SetLevel { id, before, after });
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

    /// Whether the notes go at the top of the G-code.
    pub fn set_notes_in_gcode(&mut self, after: bool) {
        if self.doc.notes_in_gcode != after {
            self.edits.push(Edit::SetNotesInGcode {
                before: self.doc.notes_in_gcode,
                after,
            });
        }
    }

    /// Replaces the toolpath group list.
    pub fn set_toolpath_groups(&mut self, after: Vec<String>) {
        if self.doc.toolpath_groups != after {
            self.edits.push(Edit::SetToolpathGroups {
                before: self.doc.toolpath_groups.clone(),
                after,
            });
        }
    }

    /// Replaces the sheet list and which sheet is active (as it stands, not its setup: see
    /// [`Self::activate_sheet`]).
    pub fn set_sheets(&mut self, after: Vec<Sheet>, active: usize) {
        let after = (after, active);
        if self.sheets != after {
            let before = std::mem::replace(&mut self.sheets, after.clone());
            self.edits.push(Edit::SetSheets { before, after });
        }
    }

    /// Turns tiling on with `settings`, or off keeping them for when it is turned on again
    /// (not kept when it was never on and they are the defaults: nothing to keep).
    pub fn set_tiling_with(&mut self, on: bool, settings: crate::tiling::Tiling) {
        if on {
            self.set_tiling_kept(Some(settings), None);
        } else {
            let worth = self.doc.tiling.is_some()
                || self.doc.tiling_kept.is_some()
                || settings != crate::tiling::Tiling::default();
            self.set_tiling_kept(None, worth.then_some(settings));
        }
    }

    pub(super) fn set_tiling_kept(
        &mut self,
        after: Option<crate::tiling::Tiling>,
        kept_after: Option<crate::tiling::Tiling>,
    ) {
        if self.doc.tiling != after || self.doc.tiling_kept != kept_after {
            self.edits.push(Edit::SetTiling {
                before: self.doc.tiling,
                after,
                kept_before: self.doc.tiling_kept,
                kept_after,
            });
        }
    }

    pub fn set_guides(&mut self, after: Vec<Guide>) {
        if self.doc.guides != after {
            let before = self.doc.guides.clone();
            self.edits.push(Edit::SetGuides { before, after });
        }
    }

    /// The points kept on their snaps, all of them.
    pub fn set_constraints(&mut self, after: Vec<crate::constraint::Constraint>) {
        if self.doc.constraints != after {
            let before = self.doc.constraints.clone();
            self.edits.push(Edit::SetConstraints { before, after });
        }
    }

    pub fn set_keep_out(&mut self, after: Vec<crate::keep_out::KeepOutZone>) {
        if self.doc.keep_out != after {
            let before = self.doc.keep_out.clone();
            self.edits.push(Edit::SetKeepOut { before, after });
        }
    }

    pub fn set_variables(&mut self, after: BTreeMap<String, f64>) {
        if self.doc.variables != after {
            let before = self.doc.variables.clone();
            self.edits.push(Edit::SetVariables { before, after });
        }
    }

    /// The sheet list as this change has left it.
    pub fn sheets(&self) -> &[Sheet] {
        &self.sheets.0
    }

    /// Makes sheet `index` the one worked on: the job takes its setup and the document its
    /// objects (both sides of a double-sided job), and the sheet left keeps the job's setup
    /// and the document's objects. Call it before anything else in the change.
    pub fn activate_sheet(&mut self, index: usize) {
        let (mut sheets, active) = self.sheets.clone();
        if index == active || index >= sheets.len() {
            return;
        }
        let mut arriving = std::mem::take(&mut sheets[index].objects);
        let mut left = self.swap_objects(&arriving);
        let (bottom, other) = self.side.clone();
        left.bottom = bottom;
        left.other_side = (!other.is_empty()).then(|| Box::new(other));
        let other = arriving.other_side.take().map_or_default(|o| *o);
        self.set_side(arriving.bottom, other);
        sheets[active].objects = left;
        sheets[active].job = self.job;
        let job = sheets[index].job;
        self.set_sheets(sheets, index);
        self.set_job(job);
    }

    /// Double-sided jobs: work on the other side. The document's objects are kept as the
    /// side left, and the other side's come in.
    pub fn swap_sides(&mut self) {
        let (bottom, arriving) = self.side.clone();
        let left = self.swap_objects(&arriving);
        self.set_side(!bottom, left);
    }

    /// Which side is worked on and what is on the other one.
    pub fn set_side(&mut self, bottom: bool, other: SheetObjects) {
        let after = (bottom, other);
        if self.side != after {
            let before = std::mem::replace(&mut self.side, after.clone());
            self.edits.push(Edit::SetSide { before, after });
        }
    }

    /// The side worked on (bottom or not) and what is on the other one, as this change has
    /// left them.
    pub fn side(&self) -> &(bool, SheetObjects) {
        &self.side
    }

    /// A free id for something stored off the document (on the other side).
    pub fn fresh_id(&mut self) -> EntityId {
        let id = EntityId(self.next_id);
        self.next_id += 1;
        id
    }

    /// Takes the document's objects out (returned, to be stored) and puts `arriving`'s in,
    /// with ordinary edits so undo is exact.
    pub(super) fn swap_objects(&mut self, arriving: &SheetObjects) -> SheetObjects {
        let doc = self.doc;
        self.select([]);
        let mut left = SheetObjects::default();
        for (&id, path) in &doc.vectors {
            let layer = doc.vector_layers.get(&id).copied();
            let group = doc.vector_groups.get(&id).copied();
            if group.is_some() {
                self.edits.push(Edit::SetGroup {
                    id,
                    before: group,
                    after: None,
                });
            }
            if layer.is_some() {
                self.edits.push(Edit::SetLayer {
                    id,
                    before: layer,
                    after: None,
                });
            }
            if doc.is_construction(id) {
                self.edits.push(Edit::SetConstruction {
                    id,
                    before: true,
                    after: false,
                });
                left.construction.insert(id);
            }
            self.edits.push(Edit::Remove {
                id,
                path: path.clone(),
            });
            let path = path.clone();
            left.vectors.insert(id, StoredVector { path, layer, group });
        }
        for (&id, k) in &doc.components {
            let component = Box::new(k.clone());
            self.edits.push(Edit::RemoveComponent { id, component });
            left.components.insert(id, k.clone());
        }
        for (&id, b) in &doc.bitmaps {
            let bitmap = Box::new(b.clone());
            self.edits.push(Edit::RemoveBitmap { id, bitmap });
            left.bitmaps.insert(id, b.clone());
        }
        for (&id, t) in &doc.toolpaths {
            let toolpath = Box::new(t.clone());
            self.edits.push(Edit::RemoveToolpath { id, toolpath });
            left.toolpaths.insert(id, t.clone());
        }
        for (&id, v) in &arriving.vectors {
            self.edits.push(Edit::Add {
                id,
                path: v.path.clone(),
            });
            // a layer gone while the sheet was put away (from an older version): its vectors
            // come onto the first layer, rather than the sheet never opening again
            let layer = v.layer.filter(|&l| doc.layer(l).is_some());
            if layer.is_some() {
                self.edits.push(Edit::SetLayer {
                    id,
                    before: None,
                    after: layer,
                });
            }
            if v.group.is_some() {
                self.edits.push(Edit::SetGroup {
                    id,
                    before: None,
                    after: v.group,
                });
            }
            if arriving.construction.contains(&id) {
                self.edits.push(Edit::SetConstruction {
                    id,
                    before: false,
                    after: true,
                });
            }
        }
        for (&id, k) in &arriving.components {
            let component = Box::new(k.clone());
            self.edits.push(Edit::AddComponent { id, component });
        }
        for (&id, b) in &arriving.bitmaps {
            let bitmap = Box::new(b.clone());
            self.edits.push(Edit::AddBitmap { id, bitmap });
        }
        for (&id, t) in &arriving.toolpaths {
            let toolpath = Box::new(t.clone());
            self.edits.push(Edit::AddToolpath { id, toolpath });
        }
        left
    }

    pub fn set_job(&mut self, after: Job) {
        if self.job != after {
            let before = std::mem::replace(&mut self.job, after);
            self.edits.push(Edit::SetJob { before, after });
        }
    }

    /// Something for the person to know about what this change makes.
    pub fn note(&mut self, note: impl Into<String>) {
        self.notes.push(note.into());
    }

    pub fn finish(self) -> Change {
        Change {
            edits: self.edits,
            notes: self.notes,
            unit_guess: None,
        }
    }
}
