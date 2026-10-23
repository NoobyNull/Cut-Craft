//! Select Vectors on Layer.

use aspire_core::document::{Document, LayerId};
use aspire_core::{Change, Command, CommandError, Meta, existing_layer};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub struct SelectLayerVectors;

#[derive(Clone, Debug, Default, Serialize, Deserialize, JsonSchema)]
pub struct Params {
    /// Selects every vector on this layer, and nothing else.
    pub layer: LayerId,
}

impl Command for SelectLayerVectors {
    const ID: &'static str = "select_layer_vectors";
    type Params = Params;

    fn meta() -> Meta {
        Meta {
            label: "Select Vectors on Layer",
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
        if !layer.visible || layer.locked {
            return Err(CommandError::Failed(format!(
                "layer \"{}\" is hidden or locked",
                layer.name
            )));
        }
        let mut c = Change::builder(doc);
        c.select(doc.vectors_on(p.layer));
        Ok(c.finish())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use aspire_core::document::EntityId;
    use aspire_core::testing::{after_err, edit_layer, run_on, two_layers};

    aspire_core::contract_tests!(SelectLayerVectors);

    #[test]
    fn selects_only_the_vectors_on_the_layer() {
        let mut doc = two_layers();
        let moved = EntityId(1);
        let mut c = Change::builder(&doc);
        c.move_to_layer(moved, LayerId(2));
        let change = c.finish();
        doc.apply(&change).unwrap();
        run_on::<SelectLayerVectors>(&mut doc, &Params { layer: LayerId(2) });
        assert_eq!(doc.selection, [moved].into());
    }

    #[test]
    fn hidden_or_locked_layers_cannot_be_selected_from() {
        for (visible, locked) in [(false, false), (true, true)] {
            let mut doc = two_layers();
            edit_layer(&mut doc, LayerId(2), |l| {
                l.visible = visible;
                l.locked = locked;
            });
            let result = SelectLayerVectors::run(&doc, &Params { layer: LayerId(2) });
            let why = result.err().map(|e| e.to_string()).unwrap_or_default();
            assert!(why.contains("hidden or locked"), "{why}");
        }
        let why = after_err::<SelectLayerVectors>(&Params { layer: LayerId(9) });
        assert!(why.contains("does not exist"), "{why}");
    }
}
