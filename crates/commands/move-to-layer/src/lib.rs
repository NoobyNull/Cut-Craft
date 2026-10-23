//! Move to Layer: puts the selected vectors on a layer.

use aspire_core::document::{Document, LayerId};
use aspire_core::{Change, Command, CommandError, Meta, existing_layer};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub struct MoveToLayer;

#[derive(Clone, Debug, Default, Serialize, Deserialize, JsonSchema)]
pub struct Params {
    /// Where the selected vectors go.
    pub layer: LayerId,
}

impl Command for MoveToLayer {
    const ID: &'static str = "move_to_layer";
    type Params = Params;

    fn meta() -> Meta {
        Meta {
            label: "Move to Layer",
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
        if doc.selection.is_empty() {
            return Err(CommandError::NothingSelected);
        }
        let mut c = Change::builder(doc);
        for &id in &doc.selection {
            c.move_to_layer(id, p.layer);
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
    use aspire_core::testing::{edit_layer, rejection, run_on, two_layers};

    aspire_core::contract_tests!(MoveToLayer);

    #[test]
    fn moves_the_selection() {
        let mut doc = two_layers();
        run_on::<MoveToLayer>(&mut doc, &Params { layer: LayerId(2) });
        assert!(doc.vectors.keys().all(|&id| doc.layer_of(id) == LayerId(2)));
        assert_eq!(doc.selection.len(), doc.vectors.len());
    }

    #[test]
    fn moving_onto_a_hidden_layer_deselects() {
        let mut doc = two_layers();
        edit_layer(&mut doc, LayerId(2), |l| l.visible = false);
        run_on::<MoveToLayer>(&mut doc, &Params { layer: LayerId(2) });
        assert!(doc.selection.is_empty());
    }

    #[test]
    fn needs_a_selection() {
        assert_eq!(
            rejection::<MoveToLayer>(&Params::default()),
            CommandError::NothingSelected.to_string()
        );
    }
}
