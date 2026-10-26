//! The job being edited. Only [`Document::apply`](crate::Change) changes it.

use crate::bitmap::Bitmap;
use crate::component::{Component, Level};
use crate::geometry::{Path, Point, Rect, polyline_distance, polyline_touches};
use crate::selection::Selection;
use crate::toolpath::Toolpath;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

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
}

impl Layer {
    pub fn new(id: LayerId, name: &str) -> Self {
        Self {
            id,
            name: name.to_owned(),
            visible: true,
            locked: false,
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

/// Display units. The document always stores millimetres.
#[derive(
    Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema,
)]
pub enum Units {
    #[default]
    #[schemars(title = "mm")]
    Mm,
    #[schemars(title = "inches")]
    Inch,
}

/// Where Z = 0 is.
#[derive(
    Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum ZZero {
    /// The tool is zeroed on the top of the material
    #[default]
    #[schemars(title = "Material Surface")]
    MaterialSurface,
    /// The tool is zeroed under the material: on the machine bed, or on a rotary job's
    /// cylinder axis
    #[schemars(title = "Machine Bed")]
    MachineBed,
}

/// A rotary job: a cylinder turned by a rotary axis, worked on as its surface unrolled flat.
/// A double-sided job's setup (Aspire's help: Job Setup - Double Sided).
#[derive(
    Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema,
)]
pub struct DoubleSided {
    /// How the material is turned over to reach the other side
    #[schemars(title = "Flip Direction Between Sides", extend("x-widget" = "row"))]
    pub flip: Flip,
    /// Z zero is on the same physical face whichever side is up
    #[schemars(title = "Zero off same side")]
    #[serde(default)]
    pub zero_same_side: bool,
}

#[derive(
    Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum Flip {
    /// Turned over left to right
    #[default]
    Horizontal,
    /// Turned over top to bottom
    Vertical,
}

impl Flip {
    /// Where a point of one side lies seen from the other, on material covering `area`.
    pub fn mirror(self, area: Rect) -> glam::DAffine2 {
        let c = area.center();
        let scale = match self {
            Self::Horizontal => Point::new(-1.0, 1.0),
            Self::Vertical => Point::new(1.0, -1.0),
        };
        glam::DAffine2::from_translation(c)
            * glam::DAffine2::from_scale(scale)
            * glam::DAffine2::from_translation(-c)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
pub struct Rotary {
    #[schemars(extend("x-unit" = "mm"), range(min = 0))]
    pub diameter: f64,
    /// The direction of the cylinder's axis; the other direction goes round it
    pub along: Along,
}

#[derive(
    Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum Along {
    #[default]
    #[schemars(title = "Along X Axis")]
    X,
    #[schemars(title = "Along Y Axis")]
    Y,
}

/// The point of the material the machine's X0, Y0 is.
#[derive(
    Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum Datum {
    #[default]
    BottomLeft,
    TopLeft,
    TopRight,
    BottomRight,
    Centre,
}

impl Datum {
    /// Where this datum is on a material of `size`, from its bottom-left corner.
    fn on(self, size: Point) -> Point {
        match self {
            Self::BottomLeft => Point::ZERO,
            Self::TopLeft => Point::new(0.0, size.y),
            Self::TopRight => size,
            Self::BottomRight => Point::new(size.x, 0.0),
            Self::Centre => size / 2.0,
        }
    }
}

/// How finely the 3D model is worked out over the job.
#[derive(
    Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum Resolution {
    /// 1 million points
    #[default]
    #[schemars(title = "Standard (fastest)")]
    Standard,
    /// 3 million points
    #[schemars(title = "High (3x slower)")]
    High,
    /// 7 million points
    #[schemars(title = "Highest (7x slower)")]
    Highest,
}

impl Resolution {
    /// Model points spread over the whole job.
    pub fn points(self) -> f64 {
        match self {
            Self::Standard => 1e6,
            Self::High => 3e6,
            Self::Highest => 7e6,
        }
    }
}

/// Which side of the material Material Setup's gap is measured from.
#[derive(
    Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum GapFrom {
    /// Between the model's top and the material surface (Gap Outside Model on rotary jobs)
    #[default]
    #[schemars(title = "Gap Above Model")]
    Above,
    /// Between the model's bottom and the material's bottom (Gap Inside Model, from the
    /// axis, on rotary jobs)
    #[schemars(title = "Gap Below Model")]
    Below,
}

/// Material Setup's Model Position in Material.
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
pub struct ModelPosition {
    pub from: GapFrom,
    #[schemars(extend("x-unit" = "mm"), range(min = 0))]
    pub gap: f64,
}

/// The material block, in millimetres. Job coordinates are the machine's: the datum is at
/// `offset` (0, 0 unless an offset is used), so the material lies where [`Job::area`] says.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Job {
    pub width: f64,
    pub height: f64,
    pub thickness: f64,
    pub units: Units,
    /// Safe height above the material for rapid moves (added after the first job format).
    #[serde(default = "default_clearance")]
    pub clearance: f64,
    // added with the full Job Setup form; older files get the defaults
    #[serde(default)]
    pub z_zero: ZZero,
    #[serde(default)]
    pub datum: Datum,
    #[serde(default)]
    pub offset: Point,
    #[serde(default)]
    pub resolution: Resolution,
    /// The material's colour in the 3D view.
    #[serde(default = "default_colour")]
    pub colour: [u8; 3],
    /// Set for a rotary job (added with rotary jobs; flat jobs leave it out).
    /// Double-sided jobs: how the material is turned over between sides.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub double_sided: Option<DoubleSided>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rotary: Option<Rotary>,
    /// Where the 3D model sits in the material (Material Setup; added with it).
    #[serde(default)]
    pub model_position: ModelPosition,
}

fn default_clearance() -> f64 {
    Job::default().clearance
}

fn default_colour() -> [u8; 3] {
    Job::default().colour
}

impl Job {
    /// A cylinder `length` long and `diameter` across, unrolled: `length` along its axis, the
    /// circumference the other way, and its radius deep, so Z zero under it is the axis.
    pub fn rotary(length: f64, diameter: f64, along: Along) -> Self {
        let round = std::f64::consts::PI * diameter;
        let (width, height) = match along {
            Along::X => (length, round),
            Along::Y => (round, length),
        };
        Self {
            width,
            height,
            thickness: diameter / 2.0,
            rotary: Some(Rotary { diameter, along }),
            ..Self::default()
        }
    }

    /// The material in job coordinates.
    pub fn area(&self) -> Rect {
        let size = Point::new(self.width, self.height);
        let min = self.offset - self.datum.on(size);
        Rect::new(min, min + size)
    }

    /// A toolpath height (from the material surface) as the machine is told it.
    pub fn machine_z(&self, z: f64) -> f64 {
        match self.z_zero {
            ZZero::MaterialSurface => z,
            ZZero::MachineBed => z + self.thickness,
        }
    }

    /// What to add to a model height for its depth from the material surface, for a model
    /// whose heights run from `lowest` to `top`, placed as Material Setup says.
    pub fn model_shift(&self, lowest: f64, top: f64) -> f64 {
        match self.model_position.from {
            GapFrom::Above => -(top + self.model_position.gap),
            GapFrom::Below => self.model_position.gap - self.thickness - lowest,
        }
    }

    /// Edge of one 3D model cell: the resolution's points spread over the job.
    pub fn model_cell(&self) -> f64 {
        (self.width * self.height / self.resolution.points()).sqrt()
    }
}

impl Default for Job {
    fn default() -> Self {
        // ponytail: what Job Setup starts a new job with; preferences could remember the last one
        Self {
            width: 600.0,
            height: 400.0,
            thickness: 20.0,
            units: Units::Mm,
            clearance: 5.0,
            z_zero: ZZero::MaterialSurface,
            datum: Datum::BottomLeft,
            offset: Point::ZERO,
            resolution: Resolution::Standard,
            // ponytail: one wood colour until a Materials dialog offers textures
            colour: [214, 160, 102],
            rotary: None,
            double_sided: None,
            model_position: ModelPosition::default(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
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
    /// Which group each grouped vector is in; picking one picks the group (added with groups).
    #[serde(default)]
    pub vector_groups: BTreeMap<EntityId, u64>,
    /// Edit ▸ Notes: free text kept with the job, printed by posts as `[FILE_NOTES]`; notes
    /// starting with "." open by themselves when the job is opened.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub notes: String,
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
    /// Areas a tool must not pass over (clamps), closed paths of their own.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub keep_out: Vec<Path>,
    /// Double-sided jobs: the bottom side is being worked on.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub bottom_side: bool,
    /// Double-sided jobs: what is on the side not being worked on.
    #[serde(default, skip_serializing_if = "SheetObjects::is_empty")]
    pub other_side: SheetObjects,
    /// Toolpath tiling: toolpaths saved in tiles for a machine smaller than the job.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tiling: Option<crate::tiling::Tiling>,
    /// Guide lines dragged out of the 2D view's rulers.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub guides: Vec<Guide>,
}

/// A guide line in the 2D view: up and down at x = `at` when vertical, else across at
/// y = `at`. A locked guide cannot be dragged.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Guide {
    pub vertical: bool,
    pub at: f64,
    #[serde(default)]
    pub locked: bool,
}

impl Guide {
    pub fn vertical(x: f64) -> Self {
        Self {
            vertical: true,
            at: x,
            locked: false,
        }
    }

    pub fn horizontal(y: f64) -> Self {
        Self {
            vertical: false,
            at: y,
            locked: false,
        }
    }
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
            notes: String::new(),
            texts: BTreeMap::new(),
            levels: crate::component::default_levels(),
            sheets: one_sheet(),
            active_sheet: 0,
            variables: BTreeMap::new(),
            keep_out: vec![],
            guides: vec![],
            tiling: None,
            bottom_side: false,
            other_side: SheetObjects::default(),
        }
    }
}

impl Document {
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

    /// The vectors picked together with `id`: its group's, or `id` alone.
    pub fn group_members(&self, id: EntityId) -> Vec<EntityId> {
        match self.vector_groups.get(&id) {
            Some(group) => self
                .vector_groups
                .iter()
                .filter(|(v, g)| *g == group && self.vectors.contains_key(v))
                .map(|(&v, _)| v)
                .collect(),
            None => vec![id],
        }
    }

    /// The vectors in group `group`.
    pub fn group_members_of(&self, group: u64) -> Vec<EntityId> {
        self.vector_groups
            .iter()
            .filter(|(v, g)| **g == group && self.vectors.contains_key(v))
            .map(|(&v, _)| v)
            .collect()
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

    /// On a visible layer: drawn in the views.
    pub fn shown(&self, id: EntityId) -> bool {
        self.layer(self.layer_of(id)).is_some_and(|l| l.visible)
    }

    /// On a visible, unlocked layer: can be picked and edited.
    pub fn editable(&self, id: EntityId) -> bool {
        self.layer(self.layer_of(id))
            .is_some_and(|l| l.visible && !l.locked)
    }

    /// The selected vectors that pass `keep`, as ids and paths in matching order.
    pub fn selected_paths(&self, keep: impl Fn(&Path) -> bool) -> (Vec<EntityId>, Vec<Path>) {
        self.selection
            .iter()
            .filter(|id| keep(&self.vectors[id]))
            .map(|&id| (id, self.vectors[&id].clone()))
            .unzip()
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
