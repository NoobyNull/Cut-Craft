//! Delete Layer: removes a layer and every vector on it.

use aspire_core::document::{Document, LayerId};
use aspire_core::{Change, Command, CommandError, Meta, existing_layer};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub struct DeleteLayer;

#[derive(Clone, Debug, Default, Serialize, Deserialize, JsonSchema)]
pub struct Params {
    /// The layer to delete, with every vector on it.
    pub layer: LayerId,
}

impl Command for DeleteLayer {
    const ID: &'static str = "delete_layer";
    type Params = Params;

    fn meta() -> Meta {
        Meta {
            label: "Delete Layer",
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
        if doc.layers.len() == 1 {
            return Err(CommandError::Failed(
                "the last layer cannot be deleted".into(),
            ));
        }
        let mut c = Change::builder(doc);
        for id in doc.vectors_on(p.layer) {
            c.remove(id);
        }
        c.remove_layer(p.layer);
        Ok(c.finish())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use aspire_core::testing::{after_err, run_on, sample_document, two_layers};

    aspire_core::contract_tests!(DeleteLayer, two_layers);

    #[test]
    fn deletes_the_layer_and_its_vectors() {
        let mut doc = two_layers();
        run_on::<DeleteLayer>(&mut doc, &Params { layer: LayerId(1) });
        assert!(doc.vectors.is_empty() && doc.selection.is_empty());
        assert_eq!(doc.layers.len(), 1);
        assert_eq!(
            doc.current_layer,
            LayerId(2),
            "another layer becomes current"
        );
    }

    #[test]
    fn keeps_vectors_on_other_layers() {
        let mut doc = two_layers();
        run_on::<DeleteLayer>(&mut doc, &Params { layer: LayerId(2) });
        assert_eq!(doc.vectors.len(), sample_document().vectors.len());
        assert_eq!(doc.layers.len(), 1);
    }

    #[test]
    fn the_last_layer_cannot_be_deleted() {
        let why = after_err::<DeleteLayer>(&Params::default());
        assert!(why.contains("last layer"), "{why}");
    }

    #[test]
    fn a_missing_layer_is_reported() {
        let why = after_err::<DeleteLayer>(&Params { layer: LayerId(9) });
        assert!(why.contains("layer 9 does not exist"), "{why}");
    }
}
