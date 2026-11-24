//! Bake Components: the visible components replaced by one holding the model they make, with
//! shape only where they had it (its outline, for a Model Boundary or Create Vector Boundary,
//! is theirs, not the whole job). Each shown level's components are baked into one on that
//! level, so the model looks the same; components on hidden levels are left as they are.

use cut_craft_core::component::{Combine, Component, level_shape_at_resolution, shown_components};
use cut_craft_core::document::Document;
use cut_craft_core::{Change, Command, CommandError, Form, Meta, NoParams};

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
            form: Form::Apply,
        }
    }

    fn run(doc: &Document, _: &NoParams) -> Result<Change, CommandError> {
        let shown = shown_components(doc);
        if shown.len() < 2 {
            return Err(CommandError::Failed(
                "show at least two components to bake together".into(),
            ));
        }
        let mut c = Change::builder(doc);
        // each shown level's components baked into one on that level, so the level's base
        // height, combine mode and clipping still apply once
        for (&level, _) in doc.levels.iter().filter(|(_, l)| l.visible) {
            let on: Vec<_> = doc
                .level_components(level)
                .into_iter()
                .filter(|id| shown.contains(id))
                .collect();
            if on.is_empty() {
                continue;
            }
            let mut baked = Component::from_material(
                "Baked",
                Combine::Add,
                &level_shape_at_resolution(doc, level),
            );
            baked.level = level;
            for id in on {
                c.remove_component(id);
            }
            c.add_component(baked);
        }
        let hidden_level = doc
            .components
            .iter()
            .any(|(id, k)| k.visible && !shown.contains(id));
        if hidden_level {
            c.note(
                "Components on a hidden level were left as they are: show the level and bake \
                 again to bake them too.",
            );
        }
        Ok(c.finish())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cut_craft_core::component::model_at_resolution;
    use cut_craft_core::geometry::Point;
    use cut_craft_core::testing::two_components;

    cut_craft_core::contract_tests!(BakeComponents, two_components);

    /// Two components on a first level 2 mm up, and one on a second level that is hidden.
    fn on_levels() -> Document {
        use cut_craft_core::component::Level;
        let mut doc = two_components();
        let mut c = Change::builder(&doc);
        let mut first = doc.levels[&0].clone();
        first.base_height = 2.0;
        c.set_level(0, Some(first));
        c.set_level(
            1,
            Some(Level {
                name: "Level 2".into(),
                visible: false,
                ..Level::default()
            }),
        );
        let mut hidden = doc.components.values().next().unwrap().clone();
        hidden.level = 1;
        c.add_component(hidden);
        doc.apply(&c.finish()).unwrap();
        doc
    }

    #[test]
    fn levels_keep_their_own_components_and_base_height() {
        let mut doc = on_levels();
        let middle = Point::new(20.0, 20.0);
        let before = model_at_resolution(&doc).at(middle).unwrap();
        let change = BakeComponents::run(&doc, &NoParams {}).unwrap();
        assert!(
            change.notes.iter().any(|n| n.contains("hidden level")),
            "{:?}",
            change.notes
        );
        doc.apply(&change).unwrap();
        assert_eq!(
            doc.components.len(),
            2,
            "the hidden level's component stays"
        );
        let after = model_at_resolution(&doc).at(middle).unwrap();
        assert!((before - after).abs() < 1e-9, "{before} {after}");
        let baked = doc.components.values().last().unwrap();
        assert_eq!(baked.level, 0);
        // showing the hidden level again gives the model it would have given
        doc.levels.get_mut(&1).unwrap().visible = true;
        let shown = model_at_resolution(&doc).at(middle).unwrap();
        let mut unbaked = on_levels();
        unbaked.levels.get_mut(&1).unwrap().visible = true;
        let want = model_at_resolution(&unbaked).at(middle).unwrap();
        assert!((shown - want).abs() < 1e-9, "{shown} {want}");
    }

    #[test]
    fn the_baked_component_has_shape_only_where_the_model_has() {
        let mut doc = two_components();
        let change = BakeComponents::run(&doc, &NoParams {}).unwrap();
        doc.apply(&change).unwrap();
        let baked = doc.components.values().next().unwrap();
        assert!(
            baked.height_at(Point::new(2.0, 2.0)).is_none(),
            "the empty corner"
        );
        assert!(baked.height_at(Point::new(20.0, 20.0)).is_some());
    }

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
