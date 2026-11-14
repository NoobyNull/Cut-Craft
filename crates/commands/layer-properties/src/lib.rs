//! Layer Properties: changes a layer's name, whether it is shown or locked against changes,
//! and the material its vectors are cut from.

use cut_craft_core::document::{Document, Layer, LayerId};
use cut_craft_core::{Change, Command, CommandError, Meta, ensure_named, existing_layer};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub struct LayerProperties;

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
pub struct Params {
    /// The layer to change
    pub layer: LayerId,
    /// What the layer is called in the layer list
    pub name: String,
    /// Show the layer's vectors; hidden ones are not drawn or picked
    pub visible: bool,
    /// Locked vectors are drawn but cannot be selected or edited.
    pub locked: bool,
    /// What this layer's vectors are cut from, from the Materials Library; a toolpath
    /// over several layers cuts with the hardest. Left empty, the job's material
    #[serde(default)]
    #[schemars(extend("x-kind" = "material"))]
    pub material: String,
}

impl Default for Params {
    fn default() -> Self {
        Layer::new(LayerId::default(), "Layer 1").into()
    }
}

impl From<Layer> for Params {
    fn from(l: Layer) -> Self {
        Self {
            layer: l.id,
            name: l.name,
            visible: l.visible,
            locked: l.locked,
            material: l.material,
        }
    }
}

impl Command for LayerProperties {
    const ID: &'static str = "layer_properties";
    type Params = Params;

    fn meta() -> Meta {
        Meta {
            label: "Layer Properties",
            group: "Layers",
            icon: "",
            shortcut: "",
        }
    }

    fn initial_params(doc: &Document) -> Params {
        doc.layer(doc.current_layer)
            .cloned()
            .map_or_default(Into::into)
    }

    fn run(doc: &Document, p: &Params) -> Result<Change, CommandError> {
        existing_layer(doc, p.layer)?;
        let after = Layer {
            id: p.layer,
            name: ensure_named(&p.name)?,
            visible: p.visible,
            locked: p.locked,
            material: p.material.trim().to_owned(),
        };
        let mut c = Change::builder(doc);
        // only editable vectors may stay selected
        if !after.visible || after.locked {
            c.select(
                doc.selection
                    .iter()
                    .copied()
                    .filter(|&id| doc.layer_of(id) != p.layer),
            );
        }
        c.replace_layer(after);
        Ok(c.finish())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cut_craft_core::testing::{after, rejection, two_layers};

    cut_craft_core::contract_tests!(LayerProperties);

    fn hidden_and_locked() -> Params {
        Params {
            layer: LayerId(1),
            name: "Top".into(),
            visible: false,
            locked: true,
            material: "Oak".into(),
        }
    }

    #[test]
    fn renames_hides_and_locks() {
        let doc = after::<LayerProperties>(&hidden_and_locked());
        assert_eq!(
            doc.layers[0],
            Layer {
                id: LayerId(1),
                name: "Top".into(),
                visible: false,
                locked: true,
                material: "Oak".into(),
            }
        );
    }

    #[test]
    fn hiding_or_locking_a_layer_deselects_its_vectors() {
        assert!(
            after::<LayerProperties>(&hidden_and_locked())
                .selection
                .is_empty()
        );
    }

    #[test]
    fn starts_from_the_current_layer() {
        let mut doc = two_layers();
        let mut c = Change::builder(&doc);
        c.set_current_layer(LayerId(2));
        let change = c.finish();
        doc.apply(&change).unwrap();
        let p = LayerProperties::initial_params(&doc);
        assert_eq!((p.layer, p.name.as_str()), (LayerId(2), "Layer 2"));
    }

    #[test]
    fn rejects_a_blank_name() {
        let why = rejection::<LayerProperties>(&Params {
            name: String::new(),
            ..Params::default()
        });
        assert!(why.contains("Type a name"), "{why}");
    }
}
