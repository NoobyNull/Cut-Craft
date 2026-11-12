//! The Vector Selector's filters: which vectors pass (open, closed, circles, circles of a
//! diameter) on which layers. Edit ▸ Vector Selector selects them; a toolpath associated with
//! one (`selector` in its settings) takes its vectors from it each time it is calculated, so
//! a template's toolpaths find new vectors on the same layers.

use crate::document::{Document, EntityId};
use crate::geometry::Path;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ClosedKind {
    /// Any shape whose ends meet
    #[default]
    AllClosedVectors,
    /// Circles only, no other shapes
    OnlyCircles,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Circles {
    /// Every circle, whatever its size
    #[default]
    SelectAllCircles,
    /// Circles within the tolerance of the diameter
    SelectCirclesMatching,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Layers {
    /// Every layer that is shown
    #[default]
    AllVisibleLayers,
    /// Only the layer you are working on
    CurrentLayerOnly,
    /// The layers named below, shown or not
    SelectedLayers,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Selector {
    /// Pick vectors whose ends do not meet
    pub open_vectors: bool,
    /// Pick shapes whose ends meet
    pub closed_vectors: bool,
    /// Every closed shape, or circles only
    pub closed_kind: ClosedKind,
    /// Every circle, or only circles of one size
    pub circles: Circles,
    /// The size of circle to pick, for holes of one size
    #[schemars(title = "Circle diameter", extend("x-unit" = "mm"), range(min = 0))]
    pub diameter: f64,
    /// How far a circle may be from that size and still be picked
    #[schemars(title = "Circle tolerance", extend("x-unit" = "mm"), range(min = 0))]
    pub tolerance: f64,
    /// Which layers to pick vectors from
    #[schemars(title = "Select vectors on")]
    pub layers: Layers,
    /// Selected layers: their names
    #[serde(default)]
    #[schemars(extend("x-show-when" = {"layers": "selected_layers"}))]
    pub layer_names: Vec<String>,
}

impl Default for Selector {
    fn default() -> Self {
        Self {
            open_vectors: true,
            closed_vectors: true,
            closed_kind: ClosedKind::AllClosedVectors,
            circles: Circles::SelectAllCircles,
            diameter: 6.0,
            tolerance: 0.1,
            layers: Layers::AllVisibleLayers,
            layer_names: vec![],
        }
    }
}

impl Selector {
    /// The selector a toolpath's settings associate with it (`selector` in them), if any.
    pub fn of(params: &serde_json::Value) -> Option<Self> {
        serde_json::from_value(params.get("selector")?.clone()).ok()
    }

    /// The vectors of `doc` that pass, in the order they were drawn.
    pub fn picked(&self, doc: &Document) -> Vec<EntityId> {
        let keep = |path: &Path| {
            if !path.closed {
                return self.open_vectors;
            }
            if !self.closed_vectors {
                return false;
            }
            match (self.closed_kind, diameter(path)) {
                (ClosedKind::AllClosedVectors, _) => true,
                (ClosedKind::OnlyCircles, None) => false,
                (ClosedKind::OnlyCircles, Some(d)) => match self.circles {
                    Circles::SelectAllCircles => true,
                    Circles::SelectCirclesMatching => (d - self.diameter).abs() <= self.tolerance,
                },
            }
        };
        let on_layer = |id: EntityId| match self.layers {
            Layers::AllVisibleLayers => doc.shown(id),
            Layers::CurrentLayerOnly => doc.shown(id) && doc.layer_of(id) == doc.current_layer,
            Layers::SelectedLayers => doc
                .layer(doc.layer_of(id))
                .is_some_and(|l| self.layer_names.contains(&l.name)),
        };
        doc.vectors
            .iter()
            .filter(|(id, path)| on_layer(**id) && keep(path))
            .map(|(&id, _)| id)
            .collect()
    }
}

/// `change` with the selector in `params` (if any) kept on each toolpath it makes, so they
/// take their vectors from it again when recalculated.
pub fn kept(params: &serde_json::Value, mut change: crate::Change) -> crate::Change {
    let Some(selector) = params.get("selector") else {
        return change;
    };
    for edit in &mut change.edits {
        if let crate::Edit::AddToolpath { toolpath, .. }
        | crate::Edit::ReplaceToolpath {
            after: toolpath, ..
        } = edit
        {
            toolpath.params["selector"] = selector.clone();
        }
    }
    change
}

/// A closed path's diameter when it is a circle: every point the same distance from the
/// middle of its square box.
fn diameter(path: &Path) -> Option<f64> {
    let b = path.bounds();
    let size = b.size();
    if (size.x - size.y).abs() > 1e-6 * size.x.max(1.0) {
        return None;
    }
    let r = size.x / 2.0;
    path.flatten(r * 1e-4)
        .iter()
        .all(|q| (q.distance(b.center()) - r).abs() <= r * 1e-3)
        .then_some(size.x)
}
