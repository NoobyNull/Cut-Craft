//! Scale Model Z Height: stretches or squashes the model to a new height, scaling every
//! visible component's heights, base heights and tilts, and the shown levels' base heights,
//! by the same factor. Components on hidden levels are left as they are.

use cut_craft_core::component::{Combine, model_at_resolution, shown_components};
use cut_craft_core::document::Document;
use cut_craft_core::{Change, Command, CommandError, Form, Meta, ensure_positive};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub struct ScaleModelZHeight;

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
pub struct Params {
    /// The model's new height, from its lowest to its highest point
    #[schemars(extend("x-unit" = "mm"), range(min = 0))]
    #[schemars(extend("x-dim" = "H", "x-diagram" = "diagrams/height_only.png"))]
    pub height: f64,
}

impl Default for Params {
    fn default() -> Self {
        Self { height: 10.0 }
    }
}

/// From the model's lowest point to its highest.
pub fn model_height(doc: &Document) -> f64 {
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
            form: Form::Apply,
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
                "the model is flat, so there is no height to scale. Give it height with a modelling tool first".into(),
            ));
        }
        let factor = p.height / now;
        let mut c = Change::builder(doc);
        // the components the model is made of; a multiplying one is a factor, not a height
        for id in shown_components(doc) {
            let component = &doc.components[&id];
            if component.combine == Combine::Multiply {
                continue;
            }
            let mut scaled = component.clone();
            for h in scaled.heights.iter_mut().filter(|h| !h.is_nan()) {
                *h *= factor;
            }
            if let Some(before) = &mut scaled.unsculpted {
                for h in before.iter_mut().filter(|h| !h.is_nan()) {
                    *h *= factor;
                }
            }
            // its own base height and tilt scale with it, so the whole model does
            scaled.shaping.base_height *= factor;
            if let Some(tilt) = &mut scaled.shaping.tilt {
                tilt.angle = (tilt.angle.to_radians().tan() * factor).atan().to_degrees();
            }
            c.replace_component(id, scaled);
        }
        for (&id, level) in doc.levels.iter().filter(|(_, l)| l.visible) {
            let mut scaled = level.clone();
            scaled.base_height *= factor;
            c.set_level(id, Some(scaled));
        }
        let mut change = c.finish();
        let mut after = doc.clone();
        if after.apply(&change).is_ok() {
            let reached = model_height(&after);
            if (reached - p.height).abs() > 0.01 * p.height.max(1.0) {
                change.notes.push(format!(
                    "The model reached {:.0} % of the height asked for: multiplying \
                     components or clipping do not scale. Run the tool again to get closer.",
                    reached / p.height * 100.0
                ));
            }
        }
        Ok(change)
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

    #[test]
    fn base_heights_scale_too_so_the_height_is_reached() {
        let mut doc = domed();
        let id = *doc.components.keys().next().unwrap();
        doc.components.get_mut(&id).unwrap().shaping.base_height = 3.0;
        doc.levels.get_mut(&0).unwrap().base_height = 1.0;
        let before = model_height(&doc);
        let p = Params {
            height: before * 2.0,
        };
        let change = ScaleModelZHeight::run(&doc, &p).unwrap();
        assert!(change.notes.is_empty(), "{:?}", change.notes);
        doc.apply(&change).unwrap();
        let now = model_height(&doc);
        assert!(
            (now - before * 2.0).abs() < 1e-6,
            "{now} vs {}",
            before * 2.0
        );
    }

    #[test]
    fn components_on_hidden_levels_are_left_alone() {
        use cut_craft_core::component::Level;
        let mut doc = domed();
        let mut on_hidden = doc.components.values().next().unwrap().clone();
        on_hidden.level = 1;
        let mut c = Change::builder(&doc);
        c.set_level(
            1,
            Some(Level {
                visible: false,
                ..Level::default()
            }),
        );
        c.add_component(on_hidden.clone());
        doc.apply(&c.finish()).unwrap();
        let p = Params { height: 20.0 };
        let change = ScaleModelZHeight::run(&doc, &p).unwrap();
        doc.apply(&change).unwrap();
        let kept = doc.components.values().last().unwrap();
        assert!(cut_craft_core::component::same_heights(
            &kept.heights,
            &on_hidden.heights
        ));
    }
}
