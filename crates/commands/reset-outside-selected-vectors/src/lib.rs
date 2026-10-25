//! Reset Outside Selected Vectors: clears the visible components' shape outside the selected closed vectors, so the
//! model there is back to the flat base.

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
        for (&id, component) in doc.components.iter().filter(|(_, k)| k.visible) {
            let mut reset = component.clone();
            for (i, h) in reset.heights.iter_mut().enumerate() {
                if !inside(&rings, component.centre_of(i)) {
                    *h = None;
                }
            }
            if reset != *component {
                c.replace_component(id, reset);
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
}
