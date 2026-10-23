//! Add Layer: a new layer on top, made current so new vectors go on it.

use aspire_core::document::Document;
use aspire_core::{Change, Command, CommandError, Meta, ensure_named};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub struct AddLayer;

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
pub struct Params {
    pub name: String,
}

impl Default for Params {
    fn default() -> Self {
        Self {
            name: "New Layer".into(),
        }
    }
}

impl Command for AddLayer {
    const ID: &'static str = "add_layer";
    type Params = Params;

    fn meta() -> Meta {
        Meta {
            label: "Add Layer",
            group: "Layers",
            icon: "",
            shortcut: "",
        }
    }

    fn initial_params(doc: &Document) -> Params {
        Params {
            name: format!("Layer {}", doc.layers.len() + 1),
        }
    }

    fn run(doc: &Document, p: &Params) -> Result<Change, CommandError> {
        let name = ensure_named(&p.name)?;
        let mut c = Change::builder(doc);
        let id = c.add_layer(&name);
        c.set_current_layer(id);
        Ok(c.finish())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use aspire_core::testing::{after, rejection, sample_document};

    aspire_core::contract_tests!(AddLayer);

    #[test]
    fn adds_a_layer_on_top_and_makes_it_current() {
        let doc = after::<AddLayer>(&Params {
            name: " Cut outs ".into(),
        });
        assert_eq!(doc.layers.len(), 2);
        assert_eq!(doc.layers[1].name, "Cut outs");
        assert_eq!(doc.current_layer, doc.layers[1].id);
    }

    #[test]
    fn rejects_a_blank_name() {
        let why = rejection::<AddLayer>(&Params { name: "  ".into() });
        assert!(why.contains("name is empty"), "{why}");
    }

    #[test]
    fn suggests_the_next_layer_number() {
        assert_eq!(AddLayer::initial_params(&sample_document()).name, "Layer 2");
    }
}
