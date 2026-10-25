//! Create Component from Visible Model: a new component holding the model the visible
//! components make; they are hidden, so the model looks the same.

use cut_craft_core::component::{Combine, Component, model_at_resolution};
use cut_craft_core::document::Document;
use cut_craft_core::{Change, Command, CommandError, Meta, NoParams};

pub struct CreateComponentFromVisibleModel;

impl Command for CreateComponentFromVisibleModel {
    const ID: &'static str = "create_component_from_visible_model";
    type Params = NoParams;

    fn meta() -> Meta {
        Meta {
            label: "Create Component from Visible Model",
            group: "Modeling",
            icon: "",
            shortcut: "",
        }
    }

    fn run(doc: &Document, _: &NoParams) -> Result<Change, CommandError> {
        if !doc.components.values().any(|k| k.visible) {
            return Err(CommandError::Failed("there is no visible model".into()));
        }
        let made =
            Component::from_material("Visible Model", Combine::Add, &model_at_resolution(doc));
        let mut c = Change::builder(doc);
        for (&id, component) in doc.components.iter().filter(|(_, k)| k.visible) {
            c.replace_component(
                id,
                cut_craft_core::component::Component {
                    visible: false,
                    ..component.clone()
                },
            );
        }
        c.add_component(made);
        Ok(c.finish())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cut_craft_core::geometry::Point;
    use cut_craft_core::testing::two_components;

    cut_craft_core::contract_tests!(CreateComponentFromVisibleModel, two_components);

    #[test]
    fn adds_one_and_hides_the_rest_keeping_the_model() {
        let mut doc = two_components();
        let middle = Point::new(20.0, 20.0);
        let before = model_at_resolution(&doc).at(middle).unwrap();
        let change = CreateComponentFromVisibleModel::run(&doc, &NoParams {}).unwrap();
        doc.apply(&change).unwrap();
        assert_eq!(doc.components.len(), 3);
        assert_eq!(doc.components.values().filter(|k| k.visible).count(), 1);
        assert!((model_at_resolution(&doc).at(middle).unwrap() - before).abs() < 1e-9);
    }
}
