//! Set Current Layer: where new vectors go.

use aspire_core::document::{Document, LayerId};
use aspire_core::{Change, Command, CommandError, Meta, existing_layer};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub struct SetCurrentLayer;

#[derive(Clone, Debug, Default, Serialize, Deserialize, JsonSchema)]
pub struct Params {
    /// New vectors go on this layer.
    pub layer: LayerId,
}

impl Command for SetCurrentLayer {
    const ID: &'static str = "set_current_layer";
    type Params = Params;

    fn meta() -> Meta {
        Meta {
            label: "Set Current Layer",
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
        let mut c = Change::builder(doc);
        c.set_current_layer(p.layer);
        Ok(c.finish())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use aspire_core::testing::{after_err, run_on, two_layers};

    aspire_core::contract_tests!(SetCurrentLayer);

    #[test]
    fn makes_the_layer_current() {
        let mut doc = two_layers();
        run_on::<SetCurrentLayer>(&mut doc, &Params { layer: LayerId(2) });
        assert_eq!(doc.current_layer, LayerId(2));
    }

    #[test]
    fn a_missing_layer_is_reported() {
        let why = after_err::<SetCurrentLayer>(&Params { layer: LayerId(9) });
        assert!(why.contains("layer 9 does not exist"), "{why}");
    }
}
