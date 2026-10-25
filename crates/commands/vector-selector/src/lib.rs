//! Vector Selector: selects the shown vectors that pass its filters: open, closed, circles,
//! circles of a diameter, on every visible layer or only the current one.

use cut_craft_core::document::{Document, EntityId};
use cut_craft_core::geometry::Path;
use cut_craft_core::{Change, Command, CommandError, Meta};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub struct VectorSelector;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ClosedKind {
    #[default]
    AllClosedVectors,
    OnlyCircles,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Circles {
    #[default]
    SelectAllCircles,
    /// Circles within the tolerance of the diameter
    SelectCirclesMatching,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Layers {
    #[default]
    AllVisibleLayers,
    CurrentLayerOnly,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
pub struct Params {
    pub open_vectors: bool,
    pub closed_vectors: bool,
    pub closed_kind: ClosedKind,
    pub circles: Circles,
    #[schemars(title = "Circle diameter", extend("x-unit" = "mm"), range(min = 0))]
    pub diameter: f64,
    #[schemars(title = "Circle tolerance", extend("x-unit" = "mm"), range(min = 0))]
    pub tolerance: f64,
    #[schemars(title = "Select vectors on")]
    pub layers: Layers,
}

impl Default for Params {
    fn default() -> Self {
        Self {
            open_vectors: true,
            closed_vectors: true,
            closed_kind: ClosedKind::AllClosedVectors,
            circles: Circles::SelectAllCircles,
            diameter: 6.0,
            tolerance: 0.1,
            layers: Layers::AllVisibleLayers,
        }
    }
}

impl Command for VectorSelector {
    const ID: &'static str = "vector_selector";
    type Params = Params;

    fn meta() -> Meta {
        Meta {
            label: "Vector Selector",
            group: "Edit",
            icon: "",
            shortcut: "",
        }
    }

    fn run(doc: &Document, p: &Params) -> Result<Change, CommandError> {
        let keep = |path: &Path| {
            if !path.closed {
                return p.open_vectors;
            }
            if !p.closed_vectors {
                return false;
            }
            match (p.closed_kind, diameter(path)) {
                (ClosedKind::AllClosedVectors, _) => true,
                (ClosedKind::OnlyCircles, None) => false,
                (ClosedKind::OnlyCircles, Some(d)) => match p.circles {
                    Circles::SelectAllCircles => true,
                    Circles::SelectCirclesMatching => (d - p.diameter).abs() <= p.tolerance,
                },
            }
        };
        let picked: Vec<EntityId> = doc
            .vectors
            .iter()
            .filter(|(id, path)| {
                doc.shown(**id)
                    && (p.layers == Layers::AllVisibleLayers
                        || doc.layer_of(**id) == doc.current_layer)
                    && keep(path)
            })
            .map(|(&id, _)| id)
            .collect();
        if picked.is_empty() {
            return Err(CommandError::Failed("no vectors pass those filters".into()));
        }
        let mut c = Change::builder(doc);
        c.select(picked);
        Ok(c.finish())
    }
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

#[cfg(test)]
mod tests {
    use super::*;
    use cut_craft_core::geometry::Point;
    use cut_craft_core::testing::{run_on, sample_document};

    cut_craft_core::contract_tests!(VectorSelector);

    fn picked(p: &Params) -> Vec<u64> {
        let mut doc = sample_document();
        let mut c = Change::builder(&doc);
        c.add(Path::circle(Point::new(0.0, 0.0), 3.0));
        let change = c.finish();
        doc.apply(&change).unwrap();
        run_on::<VectorSelector>(&mut doc, p);
        doc.selection.iter().map(|id| id.0).collect()
    }

    #[test]
    fn filters_pick_open_closed_and_circles() {
        // the sample: two squares (1, 2), an open polyline (3), a 50 mm circle (4); a 6 mm one (5)
        let open = Params {
            closed_vectors: false,
            ..Params::default()
        };
        assert_eq!(picked(&open), vec![3]);
        let circles = Params {
            open_vectors: false,
            closed_kind: ClosedKind::OnlyCircles,
            ..Params::default()
        };
        assert_eq!(picked(&circles), vec![4, 5]);
        let six = Params {
            circles: Circles::SelectCirclesMatching,
            ..circles
        };
        assert_eq!(picked(&six), vec![5]);
    }
}
