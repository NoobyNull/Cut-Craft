//! The job being edited. Only [`Document::apply`](crate::Change) changes it.

use crate::bitmap::Bitmap;
use crate::component::{Component, Level};
use crate::geometry::{Path, Point, Rect, inside, polyline_distance, polyline_touches, winding};
use crate::selection::Selection;
use crate::toolpath::Toolpath;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
mod guide;
mod toolpath_groups;
pub use guide::*;
mod job;
pub use job::*;

#[derive(
    Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, JsonSchema,
)]
pub struct EntityId(pub u64);

#[derive(
    Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, JsonSchema,
)]
pub struct LayerId(pub u32);

/// The first layer of a new document.
impl Default for LayerId {
    fn default() -> Self {
        first_layer()
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Layer {
    pub id: LayerId,
    pub name: String,
    pub visible: bool,
    /// Locked layers are drawn but cannot be picked or edited.
    pub locked: bool,
    /// The material its vectors are cut from, from the Materials Library; empty for the
    /// job's (added with the Materials Library)
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub material: String,
    /// The tool and settings that made what is on this layer, and the outlines it was made
    /// for, so the tool can open on it again and remake it (added with editable
    /// embellishments): `{"command": .., "params": .., "paths": ..}`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub made: Option<serde_json::Value>,
    /// The colour its vectors are drawn in, red, green, blue; none draws them in the
    /// usual vector colour (added with keeping an imported drawing's layers)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub colour: Option<[u8; 3]>,
}

impl Layer {
    pub fn new(id: LayerId, name: &str) -> Self {
        Self {
            id,
            name: name.to_owned(),
            visible: true,
            locked: false,
            material: String::new(),
            made: None,
            colour: None,
        }
    }
}

/// One of the job's material sheets (the Sheets tab), with its own size and setup. The
/// active sheet's setup is [`Document::job`]; its copy here is brought up to date when
/// another sheet is activated, so read sheets through [`Document::sheet_job`].
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Sheet {
    pub name: String,
    pub job: Job,
    #[serde(default = "visible")]
    pub visible: bool,
    /// What is on the sheet while another is active; the active sheet's objects are the
    /// document's own.
    #[serde(default, skip_serializing_if = "SheetObjects::is_empty")]
    pub objects: SheetObjects,
    /// The material the sheet is, from the Materials Library (Job Setup's Material); empty
    /// when not named (added with the Materials Library)
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub material: String,
}

/// A sheet's vectors (each with its layer and group), components, pictures and toolpaths,
/// kept by their ids.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct SheetObjects {
    #[serde(default)]
    pub vectors: BTreeMap<EntityId, StoredVector>,
    #[serde(default)]
    pub components: BTreeMap<EntityId, Component>,
    #[serde(default)]
    pub bitmaps: BTreeMap<EntityId, Bitmap>,
    #[serde(default)]
    pub toolpaths: BTreeMap<EntityId, Toolpath>,
    /// Which of `vectors` are construction lines (added with construction lines).
    #[serde(default, skip_serializing_if = "std::collections::BTreeSet::is_empty")]
    pub construction: std::collections::BTreeSet<EntityId>,
    /// Double-sided jobs: the bottom side was being worked on.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub bottom: bool,
    /// Double-sided jobs: what is on the side not being worked on.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub other_side: Option<Box<SheetObjects>>,
}

/// A vector kept on a sheet that is not active.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct StoredVector {
    pub path: Path,
    pub layer: Option<LayerId>,
    #[serde(default)]
    pub group: Option<u64>,
}

impl SheetObjects {
    pub fn is_empty(&self) -> bool {
        self.vectors.is_empty()
            && self.components.is_empty()
            && self.bitmaps.is_empty()
            && self.toolpaths.is_empty()
            && !self.bottom
            && self.other_side.is_none()
    }

    /// The highest id kept here.
    fn last_id(&self) -> Option<EntityId> {
        [
            self.vectors.keys().next_back(),
            self.components.keys().next_back(),
            self.bitmaps.keys().next_back(),
            self.toolpaths.keys().next_back(),
        ]
        .into_iter()
        .flatten()
        .copied()
        .chain(self.other_side.as_ref().and_then(|o| o.last_id()))
        .max()
    }
}

impl Sheet {
    pub fn new(name: &str, job: Job) -> Self {
        Self {
            name: name.to_owned(),
            job,
            visible: true,
            objects: SheetObjects::default(),
            material: String::new(),
        }
    }
}

fn visible() -> bool {
    true
}

fn one_sheet() -> Vec<Sheet> {
    vec![Sheet::new("Sheet 1", Job::default())]
}

fn default_layers() -> Vec<Layer> {
    vec![Layer::new(LayerId(1), "Layer 1")]
}

fn first_layer() -> LayerId {
    LayerId(1)
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[expect(
    clippy::struct_excessive_bools,
    reason = "independent whole-job settings, each saved as it is"
)]
pub struct Document {
    pub job: Job,
    pub vectors: BTreeMap<EntityId, Path>,
    pub selection: Selection,
    /// Added after the first job format; older files have none.
    #[serde(default)]
    pub toolpaths: BTreeMap<EntityId, Toolpath>,
    /// Bottom to top. Older files have none and get the default layer.
    #[serde(default = "default_layers")]
    pub layers: Vec<Layer>,
    /// Where new vectors go.
    #[serde(default = "first_layer")]
    pub current_layer: LayerId,
    /// Which layer each vector is on; vectors not listed are on the first layer.
    #[serde(default)]
    pub vector_layers: BTreeMap<EntityId, LayerId>,
    /// 3D shapes, combined in id order into the model.
    #[serde(default)]
    pub components: BTreeMap<EntityId, Component>,
    /// Imported pictures (added with bitmap strategies; older files have none).
    #[serde(default)]
    pub bitmaps: BTreeMap<EntityId, Bitmap>,
    /// Components picked in the Components tab, in pick order: view state like `selection`,
    /// read by the commands that work on "the selected component".
    #[serde(default)]
    pub selected_components: Selection,
    /// Bitmaps picked in the 2D view, in pick order; view state like `selection`.
    #[serde(default)]
    pub selected_bitmaps: Selection,
    /// Which group each grouped vector is in (the innermost, when groups hold groups);
    /// picking one picks its outermost group (added with groups).
    #[serde(default)]
    pub vector_groups: BTreeMap<EntityId, u64>,
    /// Groups inside groups: the group each held group is in (added with nested groups;
    /// older files have none). Grouping text with something keeps the text's own group
    /// here, so it stays editable text.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub group_parents: BTreeMap<u64, u64>,
    /// The group stepped inside (double-click a group): view state like `selection`, never
    /// saved. A click there picks one item in it instead of the whole group.
    #[serde(skip)]
    pub inside_group: Option<u64>,
    /// Edit ▸ Notes: rich text kept with the job (see [`crate::notes`]), printed by posts as
    /// `[FILE_NOTES]`; notes starting with "." open by themselves when the job is opened.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub notes: String,
    /// Notes ▸ Put these notes at the top of the G-code: the notes, variables filled in, as
    /// comment lines under each file's header (added with rich notes; older jobs leave it off).
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub notes_in_gcode: bool,
    /// Groups that are text, with the Draw Text settings they were made from, so the text
    /// can be edited again; ungrouping turns it into plain curves.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub texts: BTreeMap<u64, serde_json::Value>,
    /// The component tree's levels, first (bottom) first (added with levels; older files get
    /// the one level every job has).
    #[serde(default = "crate::component::default_levels")]
    pub levels: BTreeMap<u64, Level>,
    /// The Sheets tab's sheets, first first (added with sheets; older files get one).
    #[serde(default = "one_sheet")]
    pub sheets: Vec<Sheet>,
    /// Which of `sheets` is being worked on; its setup is `job`.
    #[serde(default)]
    pub active_sheet: usize,
    /// Edit ▸ Document Variables: named values sums in number fields use as `{name}`.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub variables: BTreeMap<String, f64>,
    /// Areas a tool must not pass over (clamps), closed paths of their own, each named.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub keep_out: Vec<crate::keep_out::KeepOutZone>,
    /// The active machine's own keep-out zones (clamps fixed to its bed), in job
    /// millimetres: set by the app from Machine Configuration, never saved with the job.
    #[serde(skip)]
    pub machine_keep_out: Vec<crate::keep_out::KeepOutZone>,
    /// Toolpaths are clipped by the keep-out zones, what is under a zone left uncut, instead
    /// of not being made: set by the app from Options (off unless turned on), not saved.
    #[serde(skip)]
    pub clip_keep_out: bool,
    /// The toolpath being made again (Edit Toolpath, recalculating), while its strategy runs:
    /// a strategy that depends on what is cut before it (a drag knife needs a flat surface)
    /// looks only at the toolpaths above it in the list. `None` while a new one is made,
    /// which goes at the bottom. Never saved.
    #[serde(skip)]
    pub remaking: Option<EntityId>,
    /// Double-sided jobs: the bottom side is being worked on.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub bottom_side: bool,
    /// Double-sided jobs: what is on the side not being worked on.
    #[serde(default, skip_serializing_if = "SheetObjects::is_empty")]
    pub other_side: SheetObjects,
    /// Toolpath tiling: toolpaths saved in tiles for a machine smaller than the job.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tiling: Option<crate::tiling::Tiling>,
    /// The tiling settings kept while tiling is off, so turning it on again starts from
    /// them (added with keeping them).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tiling_kept: Option<crate::tiling::Tiling>,
    /// Guide lines dragged out of the 2D view's rulers.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub guides: Vec<Guide>,
    /// The toolpath list's groups in order (added with toolpath groups). A group made empty
    /// is only here; each toolpath names its own group.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub toolpath_groups: Vec<String>,
    /// The toolpath group each sub-group sits in, by name (added with groups inside
    /// toolpath groups). A group not here sits at the top of the list.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub toolpath_group_parents: BTreeMap<String, String>,
    /// The toolpath groups closed in the list (their ▶ arrow), so only their names show.
    /// Saved with the job, but view state: changing it is not a step to undo.
    #[serde(default, skip_serializing_if = "std::collections::BTreeSet::is_empty")]
    pub toolpath_groups_closed: std::collections::BTreeSet<String>,
    /// Part names (Name Part, Import Cut List), by vector group: what the cut list and part
    /// labels call the part the group belongs to (added with part labels).
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub part_names: BTreeMap<u64, String>,
    /// Points kept on what they were snapped to when drawn (added with constraints).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub constraints: Vec<crate::constraint::Constraint>,
    /// Material Setup has been shown for this job, so it is not opened by itself again on
    /// the first use of the Toolpaths page (added with that).
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub material_checked: bool,
    /// Construction lines: vectors used only to place things (Embellish along them, bend a
    /// drawing along them, snap to them), never cut. Toolpaths never take them, and
    /// exported files leave them out (added with construction lines).
    #[serde(default, skip_serializing_if = "std::collections::BTreeSet::is_empty")]
    pub construction: std::collections::BTreeSet<EntityId>,
    /// View ▸ Show Construction Lines turned off: construction lines are neither drawn nor
    /// picked. View state like `selection`, never saved.
    #[serde(skip)]
    pub hide_construction: bool,
}

impl Default for Document {
    fn default() -> Self {
        Self {
            job: Job::default(),
            vectors: BTreeMap::new(),
            selection: Selection::new(),
            toolpaths: BTreeMap::new(),
            layers: default_layers(),
            current_layer: first_layer(),
            vector_layers: BTreeMap::new(),
            components: BTreeMap::new(),
            bitmaps: BTreeMap::new(),
            selected_components: Selection::new(),
            selected_bitmaps: Selection::new(),
            vector_groups: BTreeMap::new(),
            group_parents: BTreeMap::new(),
            inside_group: None,
            notes: String::new(),
            notes_in_gcode: false,
            texts: BTreeMap::new(),
            levels: crate::component::default_levels(),
            sheets: one_sheet(),
            active_sheet: 0,
            variables: BTreeMap::new(),
            keep_out: vec![],
            machine_keep_out: vec![],
            clip_keep_out: false,
            remaking: None,
            guides: vec![],
            toolpath_groups: vec![],
            toolpath_group_parents: BTreeMap::new(),
            toolpath_groups_closed: std::collections::BTreeSet::new(),
            part_names: BTreeMap::new(),
            constraints: vec![],
            material_checked: false,
            construction: std::collections::BTreeSet::new(),
            hide_construction: false,
            tiling: None,
            tiling_kept: None,
            bottom_side: false,
            other_side: SheetObjects::default(),
        }
    }
}

impl Document {
    /// The toolpaths cut before the one being made, in list order: those above it when one
    /// is made again ([`Self::remaking`]), else every toolpath, a new one going last.
    pub fn toolpaths_before_made(&self) -> Vec<&crate::toolpath::Toolpath> {
        match self.remaking {
            Some(id) => self.toolpaths.range(..id).map(|(_, t)| t).collect(),
            None => self.toolpaths.values().collect(),
        }
    }

    /// Replaces the sheet list and active sheet, `before` being what they are now.
    pub(crate) fn set_sheets(
        &mut self,
        before: &(Vec<Sheet>, usize),
        after: &(Vec<Sheet>, usize),
    ) -> Result<(), crate::change::ApplyError> {
        if (&self.sheets, self.active_sheet) != (&before.0, before.1) {
            return Err(crate::change::ApplyError::JobConflict);
        }
        (self.sheets, self.active_sheet) = after.clone();
        Ok(())
    }

    /// The components on level `level`, a component on a level that is gone counting as on
    /// the first.
    pub fn level_components(&self, level: u64) -> Vec<EntityId> {
        let first = self.levels.keys().next().copied().unwrap_or(0);
        self.components
            .iter()
            .filter(|(_, k)| {
                let on = if self.levels.contains_key(&k.level) {
                    k.level
                } else {
                    first
                };
                on == level
            })
            .map(|(id, _)| *id)
            .collect()
    }

    /// Sheet `index`'s setup: the job itself for the active sheet.
    pub fn sheet_job(&self, index: usize) -> Option<Job> {
        if index == self.active_sheet {
            Some(self.job)
        } else {
            self.sheets.get(index).map(|s| s.job)
        }
    }

    /// First id not used by any entity.
    pub fn next_id(&self) -> EntityId {
        // ids are unique over every sheet, so objects keep theirs when sheets are switched
        let stored = self
            .sheets
            .iter()
            .map(|s| &s.objects)
            .chain([&self.other_side])
            .filter_map(SheetObjects::last_id)
            .max();
        let last = self
            .vectors
            .keys()
            .next_back()
            .copied()
            .max(self.toolpaths.keys().next_back().copied())
            .max(self.components.keys().next_back().copied())
            .max(self.bitmaps.keys().next_back().copied())
            .max(stored);
        EntityId(last.map_or(1, |id| id.0 + 1))
    }

    /// Every keep-out zone a toolpath must abide by: the job's and the machine's.
    pub fn keep_out_zones(&self) -> Vec<Path> {
        self.named_keep_out().map(|z| z.path.clone()).collect()
    }

    /// Every keep-out zone with its name: the job's, then the machine's.
    pub fn named_keep_out(&self) -> impl Iterator<Item = &crate::keep_out::KeepOutZone> {
        self.keep_out.iter().chain(&self.machine_keep_out)
    }

    /// The keep-out zone `p` is in, if any, and whether it is the machine's (edited in
    /// Machine Configuration) rather than the job's (Keep-Out Zones).
    pub fn keep_out_at(&self, p: Point) -> Option<(&crate::keep_out::KeepOutZone, bool)> {
        let job = self.keep_out.iter().find(|z| z.contains(p));
        job.map(|z| (z, false)).or_else(|| {
            self.machine_keep_out
                .iter()
                .find(|z| z.contains(p))
                .map(|z| (z, true))
        })
    }

    /// The shown components a model tool changes: the picked ones when any are picked (as in
    /// the Components panel), else every shown one.
    pub fn components_to_change(&self) -> impl Iterator<Item = (&EntityId, &Component)> {
        let picked: Vec<EntityId> = self
            .selected_components
            .iter()
            .filter(|id| self.components.contains_key(id))
            .copied()
            .collect();
        self.components
            .iter()
            .filter(move |(id, k)| k.visible && (picked.is_empty() || picked.contains(id)))
    }

    /// The component a "selected component" command works on: the last one picked, else the
    /// newest.
    pub fn chosen_component(&self) -> Option<EntityId> {
        self.selected_components
            .iter()
            .rev()
            .find(|id| self.components.contains_key(id))
            .or_else(|| self.components.keys().next_back())
            .copied()
    }

    /// The level a new 3D component goes on: the level of the component picked in the
    /// component tree, else the top shown level (else the first).
    pub fn new_component_level(&self) -> u64 {
        let first = self.levels.keys().next().copied().unwrap_or(0);
        let picked = self
            .selected_components
            .iter()
            .rev()
            .find_map(|id| self.components.get(id))
            .map(|k| {
                if self.levels.contains_key(&k.level) {
                    k.level
                } else {
                    first
                }
            });
        picked
            .or_else(|| {
                self.levels
                    .iter()
                    .rev()
                    .find(|(_, l)| l.visible)
                    .map(|(&id, _)| id)
            })
            .unwrap_or(first)
    }

    /// The groups vector `id` is in, innermost first, outermost last; empty when it is in
    /// none.
    pub fn group_chain(&self, id: EntityId) -> Vec<u64> {
        let mut chain = Vec::new();
        let mut at = self.vector_groups.get(&id).copied();
        while let Some(group) = at {
            if chain.contains(&group) {
                break; // a damaged file's loop: stop rather than spin
            }
            chain.push(group);
            at = self.group_parents.get(&group).copied();
        }
        chain
    }

    /// The outermost group vector `id` is in: the one a click picks whole.
    pub fn top_group(&self, id: EntityId) -> Option<u64> {
        self.group_chain(id).last().copied()
    }

    /// The text group vector `id` belongs to, when it is a letter of editable text (even
    /// inside a bigger group).
    pub fn text_group(&self, id: EntityId) -> Option<u64> {
        self.group_chain(id)
            .into_iter()
            .find(|g| self.texts.contains_key(g))
    }

    /// The part name vector `id` belongs to: the nearest named group around it.
    pub fn part_name(&self, id: EntityId) -> Option<&str> {
        self.group_chain(id)
            .iter()
            .find_map(|g| self.part_names.get(g))
            .map(String::as_str)
    }

    /// The vectors picked together with `id`: its outermost group's, or `id` alone.
    pub fn group_members(&self, id: EntityId) -> Vec<EntityId> {
        match self.top_group(id) {
            Some(group) => self.members_under(group),
            None => vec![id],
        }
    }

    /// The vectors in group `group` itself (not those in groups inside it).
    pub fn group_members_of(&self, group: u64) -> Vec<EntityId> {
        self.vector_groups
            .iter()
            .filter(|(v, g)| **g == group && self.vectors.contains_key(v))
            .map(|(&v, _)| v)
            .collect()
    }

    /// Every vector in group `group`, including those in groups inside it.
    pub fn members_under(&self, group: u64) -> Vec<EntityId> {
        self.vector_groups
            .keys()
            .filter(|v| self.vectors.contains_key(v))
            .filter(|&&v| self.group_chain(v).contains(&group))
            .copied()
            .collect()
    }

    /// The vectors a click on `id` picks while stepped inside group `inside` (`None`: not
    /// inside any): the item one level down in `inside` that holds `id` (a group inside it,
    /// whole, or `id` alone). `None` when `id` is not in `inside`.
    pub fn pick_inside(&self, id: EntityId, inside: Option<u64>) -> Option<Vec<EntityId>> {
        let Some(inside) = inside else {
            return Some(self.group_members(id));
        };
        let chain = self.group_chain(id);
        let at = chain.iter().position(|&g| g == inside)?;
        Some(match at.checked_sub(1) {
            Some(below) => self.members_under(chain[below]),
            None => vec![id],
        })
    }

    /// The outermost group around vector `id` whose every member is selected: what the
    /// selection holds of `id` as one item. `None` when `id` is picked on its own.
    pub fn picked_group(&self, id: EntityId) -> Option<u64> {
        self.group_chain(id).into_iter().rev().find(|&g| {
            self.members_under(g)
                .iter()
                .all(|m| self.selection.contains(m))
        })
    }

    /// The groups around vector `id` (innermost first) that lie wholly among `set`: the
    /// grouping a copy of `set` keeps.
    pub fn chain_within(
        &self,
        id: EntityId,
        set: &std::collections::BTreeSet<EntityId>,
    ) -> Vec<u64> {
        self.group_chain(id)
            .into_iter()
            .take_while(|&g| self.members_under(g).iter().all(|m| set.contains(m)))
            .collect()
    }

    /// The group a double-click on `id` steps into from `inside` (`None`: not inside any):
    /// the group one level down that holds `id`. `None` when `id` is a lone item there.
    pub fn group_to_enter(&self, id: EntityId, inside: Option<u64>) -> Option<u64> {
        let chain = self.group_chain(id);
        match inside {
            None => chain.last().copied(),
            Some(inside) => {
                let at = chain.iter().position(|&g| g == inside)?;
                chain.get(at.checked_sub(1)?).copied()
            }
        }
    }

    /// A group number no vector uses yet.
    pub fn next_group(&self) -> u64 {
        let stored = self
            .sheets
            .iter()
            .flat_map(|s| s.objects.vectors.values())
            .chain(self.other_side.vectors.values())
            .filter_map(|v| v.group);
        self.vector_groups
            .values()
            .copied()
            .chain(self.group_parents.iter().flat_map(|(&a, &b)| [a, b]))
            .chain(stored)
            .max()
            .map_or(1, |g| g + 1)
    }

    /// The picture a picture tool works on: the last one picked, else the newest.
    pub fn chosen_bitmap(&self) -> Option<&Bitmap> {
        self.selected_bitmaps
            .iter()
            .rev()
            .find_map(|id| self.bitmaps.get(id))
            .or_else(|| self.bitmaps.values().next_back())
    }

    /// The picture a picture toolpath carves: the one it was `made_from` when it has one
    /// (an error once that is deleted), else [`Self::chosen_bitmap`]; with its id, to keep.
    pub fn toolpath_bitmap(
        &self,
        made_from: Option<EntityId>,
    ) -> Result<(EntityId, &Bitmap), crate::CommandError> {
        if let Some(id) = made_from {
            return self.bitmaps.get(&id).map(|b| (id, b)).ok_or_else(|| {
                crate::CommandError::Failed(
                    "the picture this toolpath was made from has been deleted. Import it \
                     again and make a new toolpath from it"
                        .into(),
                )
            });
        }
        self.selected_bitmaps
            .iter()
            .rev()
            .find_map(|id| self.bitmaps.get_key_value(id))
            .or_else(|| self.bitmaps.iter().next_back())
            .map(|(&id, b)| (id, b))
            .ok_or_else(|| crate::CommandError::Failed("import the picture first".into()))
    }

    /// The newest bitmap under `at`, for picking in the 2D view.
    pub fn pick_bitmap(&self, at: Point) -> Option<EntityId> {
        self.bitmaps
            .iter()
            .rev()
            .find(|(_, b)| b.rect.contains(at))
            .map(|(&id, _)| id)
    }

    /// The newest visible component with shape under `at`, for picking in the 2D view.
    pub fn pick_component(&self, at: Point) -> Option<EntityId> {
        self.components
            .iter()
            .rev()
            .find(|(_, c)| c.visible && c.height_at(at).is_some())
            .map(|(&id, _)| id)
    }

    pub fn layer(&self, id: LayerId) -> Option<&Layer> {
        self.layers.iter().find(|l| l.id == id)
    }

    pub fn layer_of(&self, id: EntityId) -> LayerId {
        self.vector_layers
            .get(&id)
            .copied()
            .unwrap_or_else(|| self.layers[0].id)
    }

    /// The colour vector `id` is drawn in: its layer's own, or none for the usual one.
    pub fn colour_of(&self, id: EntityId) -> Option<[u8; 3]> {
        self.layer(self.layer_of(id)).and_then(|l| l.colour)
    }

    /// Every item of a kind, by id, with whether it is shown.
    pub fn items(&self, kind: crate::params::Items) -> Vec<(u64, bool)> {
        use crate::params::Items;
        match kind {
            Items::Layers => self
                .layers
                .iter()
                .map(|l| (u64::from(l.id.0), l.visible))
                .collect(),
            Items::Toolpaths => self
                .toolpaths
                .iter()
                .map(|(id, t)| (id.0, t.visible))
                .collect(),
            Items::Components => self
                .components
                .iter()
                .map(|(id, c)| (id.0, c.visible))
                .collect(),
            Items::Levels => self.levels.iter().map(|(&id, l)| (id, l.visible)).collect(),
        }
    }

    pub fn vectors_on(&self, layer: LayerId) -> impl Iterator<Item = EntityId> + '_ {
        self.vectors
            .keys()
            .copied()
            .filter(move |&id| self.layer_of(id) == layer)
    }

    /// Where else vectors sit on `layer`: the names of the other sheets (and "the other
    /// side" of a double-sided job) holding some. A layer is the whole job's, but each sheet
    /// not shown keeps its own vectors, so a layer empty here may not be empty.
    pub fn layer_used_elsewhere(&self, layer: LayerId) -> Vec<String> {
        let first = self.layers.first().map(|l| l.id);
        let uses = |objects: &SheetObjects| {
            objects
                .vectors
                .values()
                .any(|v| v.layer.or(first) == Some(layer))
        };
        let mut found: Vec<String> = self
            .sheets
            .iter()
            .enumerate()
            .filter(|&(i, sheet)| i != self.active_sheet && uses(&sheet.objects))
            .map(|(_, sheet)| sheet.name.clone())
            .collect();
        if uses(&self.other_side) {
            found.push("the other side".to_owned());
        }
        found
    }

    /// On a visible layer, and not a construction line while those are hidden: drawn in
    /// the views.
    pub fn shown(&self, id: EntityId) -> bool {
        self.layer(self.layer_of(id)).is_some_and(|l| l.visible) && !self.hidden_construction(id)
    }

    /// On a visible, unlocked layer, and not a hidden construction line: can be picked and
    /// edited.
    pub fn editable(&self, id: EntityId) -> bool {
        self.layer(self.layer_of(id))
            .is_some_and(|l| l.visible && !l.locked)
            && !self.hidden_construction(id)
    }

    /// Whether vector `id` is a construction line: used to place things, never cut.
    pub fn is_construction(&self, id: EntityId) -> bool {
        self.construction.contains(&id)
    }

    fn hidden_construction(&self, id: EntityId) -> bool {
        self.hide_construction && self.is_construction(id)
    }

    /// This job as a toolpath sees it: construction lines taken out of the selection, so
    /// nothing is ever cut along one.
    pub fn for_cutting(&self) -> std::borrow::Cow<'_, Self> {
        if self.selection.iter().any(|id| self.is_construction(*id)) {
            let mut doc = self.clone();
            doc.selection = self
                .selection
                .iter()
                .copied()
                .filter(|id| !self.is_construction(*id))
                .collect();
            std::borrow::Cow::Owned(doc)
        } else {
            std::borrow::Cow::Borrowed(self)
        }
    }

    /// This job as an exported file has it: without its construction lines.
    pub fn without_construction(&self) -> std::borrow::Cow<'_, Self> {
        if self.construction.is_empty() {
            return std::borrow::Cow::Borrowed(self);
        }
        let mut doc = self.clone();
        for id in &self.construction {
            doc.vectors.remove(id);
            doc.vector_layers.remove(id);
            doc.vector_groups.remove(id);
            doc.selection.remove(id);
        }
        doc.construction.clear();
        std::borrow::Cow::Owned(doc)
    }

    /// How many of the selected vectors are closed and how many open.
    pub fn selected_vector_count(&self) -> VectorCount {
        let closed = self
            .selection
            .iter()
            .filter(|id| self.vectors.get(id).is_some_and(|p| p.closed))
            .count();
        let open = self
            .selection
            .iter()
            .filter(|id| self.vectors.get(id).is_some_and(|p| !p.closed))
            .count();
        VectorCount { closed, open }
    }

    /// The selected vectors that pass `keep`, as ids and paths in matching order.
    pub fn selected_paths(&self, keep: impl Fn(&Path) -> bool) -> (Vec<EntityId>, Vec<Path>) {
        self.selection
            .iter()
            .filter(|id| keep(&self.vectors[id]))
            .map(|&id| (id, self.vectors[&id].clone()))
            .unzip()
    }

    /// The selected vectors as the items a user sees: a group's selected members are one
    /// item, and with `nest`, a closed vector also takes in the items lying inside it (a
    /// part and its holes). Items come in the order their first member was picked.
    pub fn selected_items(&self, nest: bool) -> Vec<Vec<EntityId>> {
        let mut items: Vec<Vec<EntityId>> = Vec::new();
        let mut by_group: BTreeMap<u64, usize> = BTreeMap::new();
        let mut whole: BTreeMap<u64, bool> = BTreeMap::new();
        for &id in &self.selection {
            // the outermost group picked whole is the item (inside a group, one item in it)
            let item = self.group_chain(id).into_iter().rev().find(|&g| {
                *whole.entry(g).or_insert_with(|| {
                    self.members_under(g)
                        .iter()
                        .all(|m| self.selection.contains(m))
                })
            });
            if let Some(g) = item {
                if let Some(&at) = by_group.get(&g) {
                    items[at].push(id);
                    continue;
                }
                by_group.insert(g, items.len());
            }
            items.push(vec![id]);
        }
        if !nest || items.len() < 2 {
            return items;
        }
        let bounds = |item: &[EntityId]| {
            item.iter()
                .map(|id| self.vectors[id].bounds())
                .reduce(Rect::union)
                .expect("an item has a member")
        };
        let boxes: Vec<Rect> = items.iter().map(|i| bounds(i)).collect();
        // a lone closed vector is an outline other items can sit inside
        let rings: Vec<Option<Vec<Point>>> = items
            .iter()
            .map(|i| match i.as_slice() {
                [id] if self.vectors[id].closed => Some(self.vectors[id].flatten(0.01)),
                _ => None,
            })
            .collect();
        let area = |r: Rect| r.size().x * r.size().y;
        // each item's tightest outline around it
        let parent: Vec<Option<usize>> = (0..items.len())
            .map(|a| {
                (0..items.len())
                    .filter(|&b| b != a && area(boxes[b]) > area(boxes[a]))
                    .filter(|&b| {
                        rings[b].as_ref().is_some_and(|ring| {
                            boxes[b].contains_rect(boxes[a])
                                && inside(
                                    std::slice::from_ref(ring),
                                    self.vectors[&items[a][0]].start,
                                )
                        })
                    })
                    .min_by(|&x, &y| area(boxes[x]).total_cmp(&area(boxes[y])))
            })
            .collect();
        let root = |mut a: usize| {
            while let Some(p) = parent[a] {
                a = p;
            }
            a
        };
        let mut merged: Vec<Vec<EntityId>> = vec![Vec::new(); items.len()];
        for (a, item) in items.iter().enumerate() {
            merged[root(a)].extend(item);
        }
        merged.retain(|i| !i.is_empty());
        merged
    }

    /// Bounds of everything selected, or `None` when nothing is.
    pub fn selection_bounds(&self) -> Option<Rect> {
        self.selection
            .iter()
            .map(|id| self.vectors[id].bounds())
            .chain(
                self.selected_components
                    .iter()
                    .filter_map(|id| self.components.get(id))
                    .map(Component::extent),
            )
            .reduce(Rect::union)
    }

    /// The path nearest `at`, if one passes within `tolerance` of it.
    pub fn pick(&self, at: Point, tolerance: f64) -> Option<EntityId> {
        self.vectors
            .iter()
            .filter(|(id, _)| self.editable(**id))
            .map(|(&id, path)| (id, polyline_distance(&path.flatten(tolerance / 4.0), at)))
            .filter(|&(_, d)| d <= tolerance)
            .min_by(|a, b| a.1.total_cmp(&b.1))
            .map(|(id, _)| id)
    }

    /// The part a click inside a closed shape picks: the smallest closed vector round `at`
    /// with every vector lying wholly inside it (its holes, and anything in them). Empty
    /// when `at` is inside no closed vector. Only editable vectors take part.
    pub fn pick_part(&self, at: Point, tolerance: f64) -> Vec<EntityId> {
        let rings = self.editable_rings(tolerance / 4.0);
        let around = rings
            .iter()
            .filter(|(_, ring, closed)| {
                *closed && ring.len() > 2 && inside(std::slice::from_ref(ring), at)
            })
            .min_by(|a, b| ring_area(&a.1).total_cmp(&ring_area(&b.1)));
        let Some((outer, outline, _)) = around else {
            return Vec::new();
        };
        let mut part = vec![*outer];
        part.extend(contents(&rings, *outer, outline));
        part
    }

    /// The editable vectors lying wholly inside any of the closed vectors `outlines`, not
    /// counting those vectors themselves, curves flattened to within `chord`.
    pub fn vectors_inside(&self, outlines: &[EntityId], chord: f64) -> Vec<EntityId> {
        let rings = self.editable_rings(chord);
        let mut found: Vec<EntityId> = Vec::new();
        for (outer, outline, closed) in &rings {
            if *closed && outlines.contains(outer) {
                for id in contents(&rings, *outer, outline) {
                    if !found.contains(&id) && !outlines.contains(&id) {
                        found.push(id);
                    }
                }
            }
        }
        found
    }

    /// Every editable vector, flattened, with whether it is closed.
    fn editable_rings(&self, chord: f64) -> Vec<(EntityId, Vec<Point>, bool)> {
        self.vectors
            .iter()
            .filter(|(id, _)| self.editable(**id))
            .map(|(&id, path)| (id, path.flatten(chord), path.closed))
            .collect()
    }

    /// Paths fully inside `area`; with `crossing`, also those whose outline touches it.
    pub fn window(&self, area: Rect, crossing: bool) -> Vec<EntityId> {
        // ponytail: chord tolerance relative to the box; exact curve/box tests if this proves coarse
        let tolerance = (area.size().length() * 1e-3).max(1e-6);
        self.vectors
            .iter()
            .filter(|(id, _)| self.editable(**id))
            .filter(|(_, path)| {
                area.contains_rect(path.bounds())
                    || (crossing && polyline_touches(&path.flatten(tolerance), area))
            })
            .map(|(&id, _)| id)
            .collect()
    }
}

/// The vectors among `rings` lying wholly inside `outline` (the ring of `outer`); a closed
/// one must also be smaller, so two copies of a shape are not inside each other.
fn contents<'a>(
    rings: &'a [(EntityId, Vec<Point>, bool)],
    outer: EntityId,
    outline: &'a Vec<Point>,
) -> impl Iterator<Item = EntityId> + 'a {
    let size = ring_area(outline);
    rings
        .iter()
        .filter(move |(id, ring, closed)| {
            *id != outer
                && (!closed || ring_area(ring) < size)
                && ring
                    .iter()
                    .all(|&p| inside(std::slice::from_ref(outline), p))
                && !crosses(ring, outline)
        })
        .map(|(id, _, _)| *id)
}

/// Whether any span of the polyline `points` crosses a span of the closed `outline`: a
/// straight span can leave a concave outline and come back with both its ends inside.
fn crosses(points: &[Point], outline: &[Point]) -> bool {
    let n = outline.len();
    let edges = (0..n).map(|i| (outline[i], outline[(i + 1) % n]));
    let spans: Vec<(Point, Point)> = points.windows(2).map(|w| (w[0], w[1])).collect();
    edges
        .into_iter()
        .any(|(c, d)| spans.iter().any(|&(a, b)| segments_cross(a, b, c, d)))
}

/// Whether segments `ab` and `cd` cross, each passing strictly between the other's ends.
fn segments_cross(a: Point, b: Point, c: Point, d: Point) -> bool {
    let side = |p: Point, q: Point, r: Point| (q - p).perp_dot(r - p);
    let (d1, d2) = (side(a, b, c), side(a, b, d));
    let (d3, d4) = (side(c, d, a), side(c, d, b));
    d1 * d2 < 0.0 && d3 * d4 < 0.0
}

/// The area a closed ring encloses.
fn ring_area(ring: &[Point]) -> f64 {
    winding(ring).abs() / 2.0
}

impl Document {
    /// A name no toolpath group has yet: Group 1, Group 2…
    #[must_use]
    pub fn next_toolpath_group_name(&self) -> String {
        let groups = self.toolpath_groups_in_use();
        // among one more name than there are groups, one is free
        (1..=groups.len() + 1)
            .map(|n| format!("Group {n}"))
            .find(|name| !groups.contains(name))
            .expect("one more name than groups")
    }
}

/// How many selected vectors are closed (they enclose an area to pocket or carve) and how
/// many open (a line with two loose ends).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct VectorCount {
    pub closed: usize,
    pub open: usize,
}

impl VectorCount {
    pub fn total(self) -> usize {
        self.closed + self.open
    }

    /// "42 vectors selected: 40 closed, 2 open", or `None` with none selected.
    pub fn text(self) -> Option<String> {
        let n = self.total();
        let what = match (self.closed, self.open) {
            (0, 0) => return None,
            (_, 0) if n == 1 => "closed".to_owned(),
            (0, _) if n == 1 => "open".to_owned(),
            (_, 0) => "all closed".to_owned(),
            (0, _) => "all open".to_owned(),
            (closed, open) => format!("{closed} closed, {open} open"),
        };
        Some(format!("{n} vector{} selected: {what}", plural(n)))
    }

    /// For a tool that only works on closed vectors: what it will leave out, if anything.
    pub fn left_out(self, tool: &str) -> Option<String> {
        (self.open > 0).then(|| {
            let (n, s) = (self.open, plural(self.open));
            let they = if n == 1 { "it is" } else { "they are" };
            format!(
                "{n} open vector{s} will be left out: {tool} only cuts closed shapes, and \
                 {they} not closed"
            )
        })
    }
}

fn plural(n: usize) -> &'static str {
    if n == 1 { "" } else { "s" }
}

#[cfg(test)]
#[test]
fn the_selection_count_says_how_many_are_closed_and_open() {
    let count = |closed, open| VectorCount { closed, open };
    assert_eq!(count(0, 0).text(), None);
    assert_eq!(count(1, 0).text().unwrap(), "1 vector selected: closed");
    assert_eq!(count(0, 1).text().unwrap(), "1 vector selected: open");
    assert_eq!(
        count(5, 0).text().unwrap(),
        "5 vectors selected: all closed"
    );
    assert_eq!(count(0, 3).text().unwrap(), "3 vectors selected: all open");
    assert_eq!(
        count(40, 2).text().unwrap(),
        "42 vectors selected: 40 closed, 2 open"
    );
    assert_eq!(count(4, 0).left_out("Pocket"), None);
    assert_eq!(
        count(4, 1).left_out("Pocket").unwrap(),
        "1 open vector will be left out: Pocket only cuts closed shapes, and it is not closed"
    );
    assert!(
        count(4, 2)
            .left_out("V-Carve")
            .unwrap()
            .starts_with("2 open vectors")
    );
}

#[cfg(test)]
#[test]
fn the_selection_count_counts_the_selected_vectors_only() {
    use crate::testing::{document_with, square};
    let line = Path::polyline(&[Point::new(0.0, 0.0), Point::new(9.0, 0.0)], false);
    let mut doc = document_with(vec![square(0.0, 0.0, 5.0), square(9.0, 0.0, 5.0), line]);
    assert_eq!(
        doc.selected_vector_count().total(),
        3,
        "new vectors come selected"
    );
    doc.selection.clear();
    assert_eq!(doc.selected_vector_count(), VectorCount::default());
    let ids: Vec<EntityId> = doc.vectors.keys().copied().collect();
    doc.selection = ids[1..].iter().copied().collect();
    assert_eq!(
        doc.selected_vector_count(),
        VectorCount { closed: 1, open: 1 }
    );
}

#[cfg(test)]
#[test]
fn measurements_read_in_the_units_chosen_never_relabelled() {
    assert_eq!(Units::Mm.length(12.7, 1), "12.7 mm");
    assert_eq!(Units::Inch.length(12.7, 1), "0.500 in");
    assert_eq!(Units::Inch.length(15.0, 1), "0.591 in");
    assert_eq!(Units::Inch.area(645.16, 1), "1.000 in²");
    assert_eq!(Units::Mm.area(2.5, 1), "2.5 mm²");
    assert_eq!(Units::Inch.feed(2540.0, 0), "100.0 in/min");
    assert_eq!(Units::Mm.feed(2540.0, 0), "2540 mm/min");
    assert_eq!((Units::Inch.per(), Units::Inch.suffix()), (25.4, "in"));
    assert_eq!((Units::Mm.per(), Units::Mm.suffix()), (1.0, "mm"));
}

#[cfg(test)]
#[test]
fn an_angled_guide_is_a_line_through_where_it_crosses_its_axis() {
    let g = Guide::horizontal(10.0).with_angle(45.0);
    assert!((g.angle() - 45.0).abs() < 1e-9 && g.tilted());
    assert!(!Guide::vertical(3.0).tilted());
    assert_eq!(Guide::vertical(3.0).angle(), 90.0);
    // y = x + 10: the point (10, 10) is 10 / sqrt 2 off it
    assert!((g.distance(Point::new(10.0, 10.0)) - 10.0 / 2f64.sqrt()).abs() < 1e-9);
    assert!(g.distance(Point::new(5.0, 15.0)) < 1e-9);
    // moved through (40, 30): y = x - 10
    let moved = g.through(Point::new(40.0, 30.0));
    assert!((moved.at + 10.0).abs() < 1e-9 && moved.distance(Point::new(40.0, 30.0)) < 1e-9);
    // a parallel 10 to its left is 10 away everywhere
    let beside = g.parallel(10.0);
    assert!((beside.distance(Point::new(5.0, 15.0)) - 10.0).abs() < 1e-9);
    assert!((beside.angle() - 45.0).abs() < 1e-9);
}

#[cfg(test)]
mod part_tests {
    use crate::geometry::{Path, Point};
    use crate::testing::{document_with, square};

    fn ids(doc: &super::Document) -> Vec<super::EntityId> {
        doc.vectors.keys().copied().collect()
    }

    #[test]
    fn clicking_inside_a_shape_picks_it_with_its_holes() {
        // a panel with two holes, and a shape beside it
        let doc = document_with(vec![
            square(0.0, 0.0, 100.0),
            square(10.0, 10.0, 20.0),
            Path::circle(Point::new(70.0, 70.0), 10.0),
            square(200.0, 0.0, 50.0),
        ]);
        let [panel, hole, round, beside] = ids(&doc)[..] else {
            panic!("four vectors")
        };
        let mut part = doc.pick_part(Point::new(50.0, 50.0), 0.5);
        part.sort();
        assert_eq!(part, vec![panel, hole, round], "the panel and its holes");
        assert_eq!(doc.pick_part(Point::new(220.0, 20.0), 0.5), vec![beside]);
        assert!(
            doc.pick_part(Point::new(150.0, 50.0), 0.5).is_empty(),
            "outside every shape"
        );
    }

    #[test]
    fn clicking_inside_a_hole_picks_the_hole_and_what_is_in_it() {
        // a frame whose opening holds an island
        let doc = document_with(vec![
            square(0.0, 0.0, 100.0),
            square(20.0, 20.0, 60.0),
            square(40.0, 40.0, 20.0),
        ]);
        let [frame, opening, island] = ids(&doc)[..] else {
            panic!("three vectors")
        };
        let mut part = doc.pick_part(Point::new(30.0, 30.0), 0.5);
        part.sort();
        assert_eq!(
            part,
            vec![opening, island],
            "the innermost shape round the click"
        );
        assert_eq!(doc.pick_part(Point::new(50.0, 50.0), 0.5), vec![island]);
        let mut whole = doc.pick_part(Point::new(10.0, 10.0), 0.5);
        whole.sort();
        assert_eq!(whole, vec![frame, opening, island]);
    }

    #[test]
    fn an_open_vector_inside_comes_along_but_one_crossing_out_does_not() {
        let doc = document_with(vec![
            square(0.0, 0.0, 100.0),
            Path::polyline(&[Point::new(20.0, 50.0), Point::new(80.0, 50.0)], false),
            Path::polyline(&[Point::new(50.0, 20.0), Point::new(50.0, 150.0)], false),
        ]);
        let [panel, groove, crossing] = ids(&doc)[..] else {
            panic!("three vectors")
        };
        let part = doc.pick_part(Point::new(30.0, 30.0), 0.5);
        assert!(part.contains(&panel) && part.contains(&groove));
        assert!(!part.contains(&crossing), "it runs outside the panel");
    }

    #[test]
    fn a_line_across_a_notch_is_not_inside() {
        // a U: both ends of the bar sit in its arms, but the bar crosses the gap between
        let p = Point::new;
        let u = Path::polyline(
            &[
                p(0.0, 0.0),
                p(100.0, 0.0),
                p(100.0, 100.0),
                p(70.0, 100.0),
                p(70.0, 30.0),
                p(30.0, 30.0),
                p(30.0, 100.0),
                p(0.0, 100.0),
            ],
            true,
        );
        let doc = document_with(vec![
            u,
            Path::polyline(&[p(10.0, 80.0), p(90.0, 80.0)], false),
            square(10.0, 10.0, 10.0),
        ]);
        let [outline, bar, hole] = ids(&doc)[..] else {
            panic!("three vectors")
        };
        assert_eq!(
            doc.pick_part(p(50.0, 10.0), 0.5),
            vec![outline, hole],
            "the bar runs outside the U"
        );
        assert!(!doc.vectors_inside(&[outline], 0.1).contains(&bar));
    }

    #[test]
    fn overlapping_shapes_are_not_holes_of_each_other() {
        let doc = document_with(vec![square(0.0, 0.0, 100.0), square(50.0, 50.0, 100.0)]);
        let [first, second] = ids(&doc)[..] else {
            panic!("two vectors")
        };
        assert_eq!(doc.pick_part(Point::new(10.0, 10.0), 0.5), vec![first]);
        // in the overlap: the two are the same size, so either one, but not both
        assert_eq!(doc.pick_part(Point::new(75.0, 75.0), 0.5).len(), 1);
        assert_eq!(doc.pick_part(Point::new(140.0, 140.0), 0.5), vec![second]);
    }

    #[test]
    fn a_locked_layer_is_never_picked() {
        let mut doc = document_with(vec![square(0.0, 0.0, 100.0)]);
        doc.layers[0].locked = true;
        assert!(
            doc.pick_part(Point::new(50.0, 50.0), 0.5).is_empty(),
            "its layer is locked"
        );
    }
}

#[cfg(test)]
mod model_position_tests {
    use super::*;

    fn at(from: GapFrom, gap: f64) -> ModelPosition {
        ModelPosition { from, gap }
    }

    #[test]
    fn the_gaps_above_and_below_share_what_the_model_leaves_of_the_material() {
        // a 10 mm model in 20 mm material: 10 mm to share, whichever gap is typed
        assert_eq!(at(GapFrom::Above, 0.0).gaps(20.0, 10.0), (0.0, 10.0));
        assert_eq!(at(GapFrom::Above, 3.0).gaps(20.0, 10.0), (3.0, 7.0));
        assert_eq!(at(GapFrom::Below, 0.0).gaps(20.0, 10.0), (10.0, 0.0));
        assert_eq!(at(GapFrom::Below, 4.0).gaps(20.0, 10.0), (6.0, 4.0));
    }

    #[test]
    fn a_model_sticks_out_when_a_gap_leaves_no_room_and_is_said_by_how_much() {
        assert_eq!(at(GapFrom::Above, 10.0).sticks_out(20.0, 10.0), None);
        // 6 below leaves -1 above: the top pokes 1 mm out of the material's top
        assert_eq!(
            at(GapFrom::Below, 11.0).sticks_out(20.0, 10.0),
            Some(Poke::Top(1.0))
        );
        assert_eq!(
            at(GapFrom::Above, 12.5).sticks_out(20.0, 10.0),
            Some(Poke::Bottom(2.5))
        );
        // taller than the material: out whatever the gap
        assert_eq!(
            at(GapFrom::Above, 0.0).sticks_out(20.0, 25.0),
            Some(Poke::Bottom(5.0))
        );
    }
}
