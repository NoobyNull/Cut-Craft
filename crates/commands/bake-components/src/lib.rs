//! Bake Components: the visible components replaced by one holding the model they make.

use cut_craft_core::component::{Combine, Component, model_at_resolution};
use cut_craft_core::document::Document;
use cut_craft_core::{Change, Command, CommandError, Meta, NoParams};

pub struct BakeComponents;

impl Command for BakeComponents {
    const ID: &'static str = "bake_components";
    type Params = NoParams;

    fn meta() -> Meta {
        Meta {
            label: "Bake Components",
            group: "Modeling",
            icon: "",
            shortcut: "",
        }
    }

    fn run(doc: &Document, _: &NoParams) -> Result<Change, CommandError> {
        let visible: Vec<_> = doc
            .components
            .iter()
            .filter(|(_, k)| k.visible)
            .map(|(&id, _)| id)
            .collect();
        if visible.len() < 2 {
            return Err(CommandError::Failed(
                "show at least two components to bake together".into(),
            ));
        }
        let baked = Component::from_material("Baked", Combine::Add, &model_at_resolution(doc));
        let mut c = Change::builder(doc);
        for id in visible {
            c.remove_component(id);
        }
        c.add_component(baked);
        Ok(c.finish())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cut_craft_core::geometry::Point;
    use cut_craft_core::testing::two_components;

    cut_craft_core::contract_tests!(BakeComponents, two_components);

    #[test]
    fn one_component_makes_the_same_model() {
        let mut doc = two_components();
        let middle = Point::new(20.0, 20.0);
        let before = model_at_resolution(&doc).at(middle);
        let change = BakeComponents::run(&doc, &NoParams {}).unwrap();
        doc.apply(&change).unwrap();
        assert_eq!(doc.components.len(), 1);
        let after = model_at_resolution(&doc).at(middle);
        assert!(
            (before.unwrap() - after.unwrap()).abs() < 1e-9,
            "{before:?} {after:?}"
        );
    }
}
