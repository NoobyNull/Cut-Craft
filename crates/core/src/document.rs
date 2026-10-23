//! The job being edited. Only [`Document::apply`](crate::Change) changes it.

use crate::component::Component;
use crate::geometry::{Path, Point, Rect, polyline_distance, polyline_touches};
use crate::toolpath::Toolpath;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
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
    Mm,
    Inch,
}

/// The material block, in millimetres.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Job {
    pub width: f64,
    pub height: f64,
    pub thickness: f64,
    pub units: Units,
    /// Safe height above the material for rapid moves (added after the first job format).
    #[serde(default = "default_clearance")]
    pub clearance: f64,
}

fn default_clearance() -> f64 {
    Job::default().clearance
}

impl Default for Job {
    fn default() -> Self {
        // ponytail: fixed default until the Job Setup form / preferences exist
        Self {
            width: 600.0,
            height: 400.0,
            thickness: 20.0,
            units: Units::Mm,
            clearance: 5.0,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Document {
    pub job: Job,
    pub vectors: BTreeMap<EntityId, Path>,
    pub selection: BTreeSet<EntityId>,
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
}

impl Default for Document {
    fn default() -> Self {
        Self {
            job: Job::default(),
            vectors: BTreeMap::new(),
            selection: BTreeSet::new(),
            toolpaths: BTreeMap::new(),
            layers: default_layers(),
            current_layer: first_layer(),
            vector_layers: BTreeMap::new(),
            components: BTreeMap::new(),
        }
    }
}

impl Document {
    /// First id not used by any entity.
    pub fn next_id(&self) -> EntityId {
        let last = self
            .vectors
            .keys()
            .next_back()
            .max(self.toolpaths.keys().next_back())
            .max(self.components.keys().next_back());
        EntityId(last.map_or(1, |id| id.0 + 1))
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
