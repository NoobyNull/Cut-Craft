//! Scale Model Z Height: stretches or squashes the model to a new height, scaling every
//! visible component's heights by the same factor.

use cut_craft_core::component::{Combine, model_at_resolution};
use cut_craft_core::document::Document;
use cut_craft_core::{Change, Command, CommandError, Meta, ensure_positive};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub struct ScaleModelZHeight;

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
pub struct Params {
    /// The model's new height, from its lowest to its highest point
    #[schemars(extend("x-unit" = "mm"), range(min = 0))]
    pub height: f64,
}

impl Default for Params {
    fn default() -> Self {
        Self { height: 10.0 }
    }
}

/// From the model's lowest point to its highest.
fn model_height(doc: &Document) -> f64 {
    let m = model_at_resolution(doc);
    let (lo, hi) = m
        .heights
        .iter()
        .fold((f64::INFINITY, f64::NEG_INFINITY), |(lo, hi), &h| {
            (lo.min(h), hi.max(h))
        });
    hi - lo
}

impl Command for ScaleModelZHeight {
    const ID: &'static str = "scale_model_z_height";
    type Params = Params;

    fn meta() -> Meta {
        Meta {
            label: "Scale Model Z Height",
            group: "Modeling",
            icon: "",
            shortcut: "",
        }
    }

    fn initial_params(doc: &Document) -> Params {
        Params {
            height: model_height(doc),
        }
    }

    fn run(doc: &Document, p: &Params) -> Result<Change, CommandError> {
        ensure_positive("height", p.height)?;
        let now = model_height(doc);
        if now <= 0.0 {
            return Err(CommandError::Failed(
                "the model is flat: there is no height to scale".into(),
            ));
        }
        let factor = p.height / now;
        let mut c = Change::builder(doc);
        // a multiplying component is a factor, not a height
        for (&id, component) in doc
            .components
            .iter()
            .filter(|(_, k)| k.visible && k.combine != Combine::Multiply)
        {
            let mut scaled = component.clone();
            for h in scaled.heights.iter_mut().flatten() {
                *h *= factor;
            }
            c.replace_component(id, scaled);
        }
        Ok(c.finish())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cut_craft_core::testing::domed;

    cut_craft_core::contract_tests!(ScaleModelZHeight, domed);

    #[test]
    fn the_model_ends_the_height_asked_for() {
        let mut doc = domed();
        let before = model_height(&doc);
        let change = ScaleModelZHeight::run(
            &doc,
            &Params {
                height: before * 2.0,
            },
        )
        .unwrap();
        doc.apply(&change).unwrap();
        assert!((model_height(&doc) - before * 2.0).abs() < 1e-6);
        assert!((ScaleModelZHeight::initial_params(&doc).height - before * 2.0).abs() < 1e-6);
    }
}
