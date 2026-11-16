//! Create Component from Visible Model: a new component holding the model the visible
//! components make, with shape only where they have it; they are hidden, so the model looks
//! the same. It goes on the lowest shown level, the level's Base Height not added twice;
//! components on hidden levels are left as they are.

use cut_craft_core::component::{model_component, model_shape_at_resolution, shown_components};
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
        let shown = shown_components(doc);
        if shown.is_empty() {
            return Err(CommandError::Failed(
                "there is no visible model. Show a component, or create one with a modelling tool"
                    .into(),
            ));
        }
        let made = model_component(doc, "Visible Model", &model_shape_at_resolution(doc));
        let mut c = Change::builder(doc);
        for id in shown {
            let mut k = doc.components[&id].clone();
            k.visible = false;
            c.replace_component(id, k);
        }
        c.add_component(made);
        Ok(c.finish())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cut_craft_core::component::model_at_resolution;
    use cut_craft_core::geometry::Point;
    use cut_craft_core::testing::two_components;

    cut_craft_core::contract_tests!(CreateComponentFromVisibleModel, two_components);

    #[test]
    fn a_level_base_height_is_not_added_twice() {
        let mut doc = two_components();
        doc.levels.get_mut(&0).unwrap().base_height = 2.0;
        let middle = Point::new(20.0, 20.0);
        let before = model_at_resolution(&doc).at(middle).unwrap();
        let change = CreateComponentFromVisibleModel::run(&doc, &NoParams {}).unwrap();
        doc.apply(&change).unwrap();
        let after = model_at_resolution(&doc).at(middle).unwrap();
        assert!((after - before).abs() < 1e-9, "{before} {after}");
    }

    #[test]
    fn the_new_component_has_shape_only_where_the_model_has() {
        let mut doc = two_components();
        let change = CreateComponentFromVisibleModel::run(&doc, &NoParams {}).unwrap();
        doc.apply(&change).unwrap();
        let made = doc.components.values().last().unwrap();
        assert!(
            made.height_at(Point::new(2.0, 2.0)).is_none(),
            "the empty corner"
        );
        assert!(made.height_at(Point::new(20.0, 20.0)).is_some());
    }

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
