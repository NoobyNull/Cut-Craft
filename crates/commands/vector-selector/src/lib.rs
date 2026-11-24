//! Vector Selector: selects the vectors that pass its filters (open, closed, circles,
//! circles of a diameter) on every visible layer, the current one, or the layers named;
//! the filters live in `cut_craft_core::selector`, where toolpaths use them too. Vectors on
//! hidden or locked layers are never selected: the form says in red how many were left out.

use cut_craft_core::document::Document;
pub use cut_craft_core::selector::{Circles, ClosedKind, Layers, Selector as Params};
use cut_craft_core::{Change, Command, CommandError, Form, Meta};

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
            form: Form::Apply,
        }
    }

    fn run(doc: &Document, p: &Params) -> Result<Change, CommandError> {
        let picked = p.picked(doc);
        if picked.is_empty() {
            return Err(CommandError::Failed(
                "no vectors pass those filters. Turn some filters off, or loosen them".into(),
            ));
        }
        // only what could be edited is selected, so Delete cannot reach a hidden or locked
        // layer's vectors
        let (editable, left_out): (Vec<_>, Vec<_>) =
            picked.into_iter().partition(|&id| doc.editable(id));
        let mut c = Change::builder(doc);
        if !left_out.is_empty() {
            let n = left_out.len();
            let (vectors, were) = if n == 1 {
                ("vector", "was")
            } else {
                ("vectors", "were")
            };
            c.note(format!(
                "{n} {vectors} on hidden or locked layers {were} left out. Show and unlock \
                 the layer to select them"
            ));
        }
        c.select(editable);
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

    #[test]
    fn the_command_leaves_out_hidden_and_locked_vectors_and_says_so() {
        fn layer(
            doc: &mut Document,
            id: cut_craft_core::document::LayerId,
        ) -> &mut cut_craft_core::document::Layer {
            doc.layers.iter_mut().find(|l| l.id == id).unwrap()
        }
        use cut_craft_core::document::EntityId;
        let mut doc = sample_document();
        let mut c = Change::builder(&doc);
        let holes = c.add_layer("Holes");
        c.move_to_layer(EntityId(2), holes);
        doc.apply(&c.finish()).unwrap();
        layer(&mut doc, holes).visible = false;
        let named = Params {
            layers: Layers::SelectedLayers,
            layer_names: vec!["Holes".into(), "Layer 1".into()],
            ..Params::default()
        };
        let change = VectorSelector::run(&doc, &named).unwrap();
        let mut after = doc.clone();
        after.apply(&change).unwrap();
        assert!(!after.selection.contains(&EntityId(2)), "hidden: left out");
        assert!(after.selection.contains(&EntityId(1)));
        assert!(
            change
                .notes
                .iter()
                .any(|n| n.contains("1 vector") && n.contains("hidden or locked")),
            "{:?}",
            change.notes
        );
        // a locked layer that is shown is left out too
        layer(&mut doc, holes).visible = true;
        layer(&mut doc, holes).locked = true;
        let change = VectorSelector::run(&doc, &Params::default()).unwrap();
        let mut after = doc.clone();
        after.apply(&change).unwrap();
        assert!(!after.selection.contains(&EntityId(2)), "locked: left out");
    }
}
