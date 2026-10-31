//! Merge Visible Layers: every vector on a visible layer goes onto one layer, and the other
//! visible layers, now empty, are removed.

use cut_craft_core::document::{Document, LayerId};
use cut_craft_core::{Change, Command, CommandError, Meta, existing_layer};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub struct MergeVisibleLayers;

#[derive(Clone, Debug, Default, Serialize, Deserialize, JsonSchema)]
pub struct Params {
    /// The layer the visible layers merge into.
    pub layer: LayerId,
}

impl Command for MergeVisibleLayers {
    const ID: &'static str = "merge_visible_layers";
    type Params = Params;

    fn meta() -> Meta {
        Meta {
            label: "Merge Visible Layers",
            group: "Layers",
            icon: "",
            shortcut: "",
        }
    }

    fn initial_params(doc: &Document) -> Params {
        Params {
            layer: doc.current_layer,
        }
    }

    fn run(doc: &Document, p: &Params) -> Result<Change, CommandError> {
        let layer = existing_layer(doc, p.layer)?;
        let others: Vec<LayerId> = doc
            .layers
            .iter()
            .filter(|l| l.visible && l.id != p.layer)
            .map(|l| l.id)
            .collect();
        if others.is_empty() {
            return Err(CommandError::Failed(
                "no other visible layer to merge".into(),
            ));
        }
        let mut c = Change::builder(doc);
        for &other in &others {
            for id in doc.vectors_on(other) {
                c.move_to_layer(id, p.layer);
            }
            c.remove_layer(other);
        }
        // only editable vectors may stay selected
        if !layer.visible || layer.locked {
            c.select([]);
        }
        Ok(c.finish())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cut_craft_core::testing::{edit_layer, run_on, two_layers};

    cut_craft_core::contract_tests!(MergeVisibleLayers, two_layers);

    #[test]
    fn visible_layers_merge_into_one() {
        let mut doc = two_layers();
        let count = doc.vectors.len();
        run_on::<MergeVisibleLayers>(&mut doc, &Params { layer: LayerId(2) });
        assert_eq!(doc.layers.len(), 1);
        assert_eq!(doc.vectors.len(), count);
        assert!(doc.vectors.keys().all(|&id| doc.layer_of(id) == LayerId(2)));
    }

    #[test]
    fn hidden_layers_are_left_alone() {
        let mut doc = two_layers();
        edit_layer(&mut doc, LayerId(1), |l| l.visible = false);
        let err = MergeVisibleLayers::run(&doc, &Params { layer: LayerId(2) })
            .err()
            .expect("nothing else visible to merge");
        assert!(err.to_string().contains("no other visible layer"), "{err}");
    }
}
