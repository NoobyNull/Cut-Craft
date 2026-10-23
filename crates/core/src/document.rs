//! The job being edited. Only [`Document::apply`](crate::Change) changes it.

use crate::geometry::{Path, Rect};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct EntityId(pub u64);

/// Display units. The document always stores millimetres.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
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
}

impl Default for Job {
    fn default() -> Self {
        // ponytail: fixed default until the Job Setup form / preferences exist
        Self {
            width: 600.0,
            height: 400.0,
            thickness: 20.0,
            units: Units::Mm,
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Document {
    pub job: Job,
    pub vectors: BTreeMap<EntityId, Path>,
    pub selection: BTreeSet<EntityId>,
}

impl Document {
    /// First id not used by any entity.
    pub fn next_id(&self) -> EntityId {
        EntityId(self.vectors.keys().next_back().map_or(1, |id| id.0 + 1))
    }

    /// Bounds of everything selected, or `None` when nothing is.
    pub fn selection_bounds(&self) -> Option<Rect> {
        self.selection
            .iter()
            .map(|id| self.vectors[id].bounds())
            .reduce(Rect::union)
    }
}
