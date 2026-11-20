//! Reset Outside Selected Vectors: clears the shape outside the selected closed vectors, so the model
//! there is back to the flat base: only on the components picked, or on every shown one when
//! none is picked.

use cut_craft_core::document::Document;
use cut_craft_core::geometry::inside;
use cut_craft_core::{Change, Command, CommandError, Meta, NoParams};

pub struct ResetOutsideSelectedVectors;

impl Command for ResetOutsideSelectedVectors {
    const ID: &'static str = "reset_outside_selected_vectors";
    type Params = NoParams;

    fn meta() -> Meta {
        Meta {
            label: "Reset Outside Selected Vectors",
            group: "Modeling",
            icon: "",
            shortcut: "",
        }
    }

    fn run(doc: &Document, _: &NoParams) -> Result<Change, CommandError> {
        let (ids, paths) = doc.selected_paths(|path| path.closed);
        if ids.is_empty() {
            return Err(CommandError::Failed(
                "select the closed vectors to reset outside".into(),
            ));
        }
        let rings: Vec<_> = paths
            .iter()
            .map(|p| p.flatten(cut_craft_core::polygon::TOLERANCE))
            .collect();
        let mut c = Change::builder(doc);
        for (&id, component) in doc.components_to_change() {
            let mut reset = component.clone();
            for (i, h) in reset.heights.iter_mut().enumerate() {
                if !inside(&rings, component.centre_of(i)) {
                    *h = f64::NAN;
                }
            }
            if reset != *component {
                c.replace_component(id, reset.hand_edited());
            }
        }
        Ok(c.finish())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cut_craft_core::geometry::Point;
    use cut_craft_core::testing::{domed, square};

    /// The dome with a 10 mm square over its middle selected.
    fn with_square() -> Document {
        let mut doc = domed();
        let mut c = Change::builder(&doc);
        let id = c.add(square(15.0, 15.0, 10.0));
        c.select([id]);
        let change = c.finish();
        doc.apply(&change).unwrap();
        doc
    }

    cut_craft_core::contract_tests!(ResetOutsideSelectedVectors, with_square);

    #[test]
    fn clears_the_shape_only_outside() {
        let mut doc = with_square();
        let change = ResetOutsideSelectedVectors::run(&doc, &NoParams {}).unwrap();
        doc.apply(&change).unwrap();
        let dome = doc.components.values().next().unwrap();
        let (middle, rim) = (Point::new(20.0, 20.0), Point::new(13.5, 20.0));
        assert!(dome.height_at(middle).is_some(), "kept inside");
        assert!(dome.height_at(rim).is_none(), "cleared outside");
    }

    /// #591: with one component picked, only it changes; the others are left as they are.
    #[test]
    fn changes_only_the_picked_component() {
        let mut doc = with_square();
        let dome = *doc.components.keys().next().unwrap();
        let mut c = Change::builder(&doc);
        let other = c.add_component(doc.components[&dome].clone());
        let change = c.finish();
        doc.apply(&change).unwrap();
        doc.selected_components = [other].into();
        let before = doc.components[&dome].clone();
        let change = ResetOutsideSelectedVectors::run(&doc, &NoParams {}).unwrap();
        doc.apply(&change).unwrap();
        assert_eq!(
            doc.components[&dome], before,
            "the one not picked is untouched"
        );
        assert_ne!(doc.components[&other], before, "the picked one is reset");
    }

    #[test]
    fn with_none_picked_every_shown_component_changes() {
        let mut doc = with_square();
        let dome = *doc.components.keys().next().unwrap();
        let mut c = Change::builder(&doc);
        let other = c.add_component(doc.components[&dome].clone());
        let change = c.finish();
        doc.apply(&change).unwrap();
        doc.selected_components.clear();
        let before = doc.components[&dome].clone();
        let change = ResetOutsideSelectedVectors::run(&doc, &NoParams {}).unwrap();
        doc.apply(&change).unwrap();
        assert_ne!(doc.components[&dome], before);
        assert_ne!(doc.components[&other], before);
    }
}
