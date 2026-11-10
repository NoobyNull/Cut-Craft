//! Merge Visible Layers (a layer's pop-up menu): every vector on a visible layer moves onto
//! this layer, and the other visible layers, then empty, are removed. Hidden layers stay.

use cut_craft_core::document::{Document, LayerId};
use cut_craft_core::{Change, Command, CommandError, Meta, existing_layer};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub struct MergeVisibleLayers;

#[derive(Clone, Debug, Default, Serialize, Deserialize, JsonSchema)]
pub struct Params {
    /// The layer the visible layers merge into
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
        existing_layer(doc, p.layer)?;
        let merged: Vec<LayerId> = doc
            .layers
            .iter()
            .filter(|l| l.visible && l.id != p.layer)
            .map(|l| l.id)
            .collect();
        if merged.is_empty() {
            return Err(CommandError::Failed(
                "there is no other visible layer to merge. Show the layers to merge".into(),
            ));
        }
        let mut c = Change::builder(doc);
        if merged.contains(&doc.current_layer) {
            c.set_current_layer(p.layer);
        }
        for &layer in &merged {
            for id in doc.vectors_on(layer) {
                c.move_to_layer(id, p.layer);
            }
        }
        for layer in merged {
            c.remove_layer(layer);
        }
        Ok(c.finish())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cut_craft_core::testing::{run_on, two_layers};

    cut_craft_core::contract_tests!(MergeVisibleLayers, two_layers);

    #[test]
    fn the_visible_layers_vectors_come_onto_this_one_and_hidden_layers_stay() {
        let mut doc = two_layers();
        // a third, hidden layer
        let mut c = Change::builder(&doc);
        let hidden = c.add_layer("Hidden");
        doc.apply(&c.finish()).unwrap();
        doc.layers
            .iter_mut()
            .find(|l| l.id == hidden)
            .unwrap()
            .visible = false;
        let vectors = doc.vectors.len();
        let target = doc.layers[0].id;
        run_on::<MergeVisibleLayers>(&mut doc, &Params { layer: target });
        let ids: Vec<LayerId> = doc.layers.iter().map(|l| l.id).collect();
        assert_eq!(ids, [target, hidden], "the other visible layer is gone");
        assert_eq!(doc.vectors.len(), vectors, "no vector lost");
        assert!(doc.vectors.keys().all(|&v| doc.layer_of(v) == target));
        assert!(MergeVisibleLayers::run(&doc, &Params { layer: target }).is_err());
    }
}
