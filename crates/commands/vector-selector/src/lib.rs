//! Vector Selector: selects the vectors that pass its filters (open, closed, circles,
//! circles of a diameter) on every visible layer, the current one, or the layers named;
//! the filters live in `cut_craft_core::selector`, where toolpaths use them too.

use cut_craft_core::document::Document;
pub use cut_craft_core::selector::{Circles, ClosedKind, Layers, Selector as Params};
use cut_craft_core::{Change, Command, CommandError, Meta};

pub struct VectorSelector;

impl Command for VectorSelector {
    const ID: &'static str = "vector_selector";
    type Params = Params;

    fn meta() -> Meta {
        Meta {
            label: "Vector Selector",
            group: "Edit",
            icon: "",
            shortcut: "",
        }
    }

    fn run(doc: &Document, p: &Params) -> Result<Change, CommandError> {
        let picked = p.picked(doc);
        if picked.is_empty() {
            return Err(CommandError::Failed("no vectors pass those filters".into()));
        }
        let mut c = Change::builder(doc);
        c.select(picked);
        Ok(c.finish())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cut_craft_core::geometry::{Path, Point};
    use cut_craft_core::testing::{run_on, sample_document};

    cut_craft_core::contract_tests!(VectorSelector);

    fn picked(p: &Params) -> Vec<u64> {
        let mut doc = sample_document();
        let mut c = Change::builder(&doc);
        c.add(Path::circle(Point::new(0.0, 0.0), 3.0));
        let change = c.finish();
        doc.apply(&change).unwrap();
        run_on::<VectorSelector>(&mut doc, p);
        doc.selection.iter().map(|id| id.0).collect()
    }

    #[test]
    fn filters_pick_open_closed_and_circles() {
        // the sample: two squares (1, 2), an open polyline (3), a 50 mm circle (4); a 6 mm one (5)
        let open = Params {
            closed_vectors: false,
            ..Params::default()
        };
        assert_eq!(picked(&open), vec![3]);
        let circles = Params {
            open_vectors: false,
            closed_kind: ClosedKind::OnlyCircles,
            ..Params::default()
        };
        assert_eq!(picked(&circles), vec![4, 5]);
        let six = Params {
            circles: Circles::SelectCirclesMatching,
            ..circles
        };
        assert_eq!(picked(&six), vec![5]);
    }

    #[test]
    fn selected_layers_pick_vectors_on_the_layers_named_even_hidden() {
        use cut_craft_core::document::EntityId;
        let mut doc = sample_document();
        let mut c = Change::builder(&doc);
        let holes = c.add_layer("Holes");
        c.move_to_layer(EntityId(2), holes);
        doc.apply(&c.finish()).unwrap();
        doc.layers
            .iter_mut()
            .find(|l| l.id == holes)
            .unwrap()
            .visible = false;
        let p = Params {
            layers: Layers::SelectedLayers,
            layer_names: vec!["Holes".into()],
            ..Params::default()
        };
        assert_eq!(p.picked(&doc), [EntityId(2)]);
    }
}
