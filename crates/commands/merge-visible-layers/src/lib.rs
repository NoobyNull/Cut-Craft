//! Merge Visible Layers (a layer's pop-up menu): every vector on a visible layer moves onto
//! this layer, and the other visible layers, then empty, are removed. Hidden layers stay, and
//! so do locked ones, their vectors with them. A hidden layer merged into is shown, so the
//! work stays in sight. What was left out, and layers cut from another material, are said in
//! red.

use cut_craft_core::document::{Document, LayerId};
use cut_craft_core::{Change, Command, CommandError, Meta, existing_layer};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub struct MergeVisibleLayers;

#[derive(Clone, Debug, Default, Serialize, Deserialize, JsonSchema)]
pub struct Params {
    /// The layer the visible layers merge into
    pub layer: LayerId,
}

impl Command for MergeVisibleLayers {
    const ID: &'static str = "merge_visible_layers";
    type Params = Params;

    fn meta() -> Meta {
        Meta {
            label: "Merge Visible Layers",
            group: "Layers",
            icon: "",
            shortcut: "",
        }
    }

    fn initial_params(doc: &Document) -> Params {
        Params {
            layer: doc.current_layer,
        }
    }

    fn run(doc: &Document, p: &Params) -> Result<Change, CommandError> {
        let target = existing_layer(doc, p.layer)?.clone();
        let others = || doc.layers.iter().filter(|l| l.visible && l.id != p.layer);
        if others().next().is_none() {
            return Err(CommandError::Failed(
                "there is no other visible layer to merge. Show the layers to merge".into(),
            ));
        }
        let merged: Vec<LayerId> = others().filter(|l| !l.locked).map(|l| l.id).collect();
        let mut c = Change::builder(doc);
        let locked: Vec<String> = others()
            .filter(|l| l.locked)
            .map(|l| format!("'{}'", l.name))
            .collect();
        if !locked.is_empty() {
            c.note(format!(
                "Left out, as locked: {}. Unlock a layer to merge it",
                locked.join(", ")
            ));
        }
        if !target.visible {
            c.note(format!(
                "'{}' was hidden, so it is shown now with the merged vectors on it",
                target.name
            ));
            c.replace_layer(cut_craft_core::document::Layer {
                visible: true,
                ..target.clone()
            });
        }
        let job = "the job's material";
        let material = |m: &str| {
            if m.is_empty() {
                job.to_owned()
            } else {
                m.to_owned()
            }
        };
        let other: Vec<String> = others()
            .filter(|l| !l.locked && l.material != target.material)
            .map(|l| format!("'{}' ({})", l.name, material(&l.material)))
            .collect();
        if !other.is_empty() {
            c.note(format!(
                "Cut from another material: {}. Merged, they are cut from {}. If that is \
                 wrong, set the material with the layer's right-click Rename",
                other.join(", "),
                material(&target.material)
            ));
        }
        if merged.contains(&doc.current_layer) {
            c.set_current_layer(p.layer);
        }
        for &layer in &merged {
            for id in doc.vectors_on(layer) {
                c.move_to_layer(id, p.layer);
            }
        }
        // the other sheets' vectors on them come over too, or those sheets would hold
        // vectors on layers that are gone
        let mut sheets = c.sheets().to_vec();
        let first = doc.layers.first().map(|l| l.id);
        let onto = |objects: &mut cut_craft_core::document::SheetObjects| {
            for v in objects.vectors.values_mut() {
                if v.layer.or(first).is_some_and(|l| merged.contains(&l)) {
                    v.layer = Some(p.layer);
                }
            }
        };
        for (i, sheet) in sheets.iter_mut().enumerate() {
            if i != doc.active_sheet {
                onto(&mut sheet.objects);
            }
        }
        c.set_sheets(sheets, doc.active_sheet);
        for layer in merged {
            c.remove_layer(layer);
        }
        Ok(c.finish())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cut_craft_core::testing::{run_on, two_layers};

    cut_craft_core::contract_tests!(MergeVisibleLayers, two_layers);

    #[test]
    fn the_visible_layers_vectors_come_onto_this_one_and_hidden_layers_stay() {
        let mut doc = two_layers();
        // a third, hidden layer
        let mut c = Change::builder(&doc);
        let hidden = c.add_layer("Hidden");
        doc.apply(&c.finish()).unwrap();
        doc.layers
            .iter_mut()
            .find(|l| l.id == hidden)
            .unwrap()
            .visible = false;
        let vectors = doc.vectors.len();
        let target = doc.layers[0].id;
        run_on::<MergeVisibleLayers>(&mut doc, &Params { layer: target });
        let ids: Vec<LayerId> = doc.layers.iter().map(|l| l.id).collect();
        assert_eq!(ids, [target, hidden], "the other visible layer is gone");
        assert_eq!(doc.vectors.len(), vectors, "no vector lost");
        assert!(doc.vectors.keys().all(|&v| doc.layer_of(v) == target));
        assert!(MergeVisibleLayers::run(&doc, &Params { layer: target }).is_err());
    }

    fn layer(doc: &mut Document, id: LayerId) -> &mut cut_craft_core::document::Layer {
        doc.layers.iter_mut().find(|l| l.id == id).unwrap()
    }

    #[test]
    fn a_hidden_target_is_shown_and_locked_layers_are_left_out_with_a_warning() {
        let mut doc = two_layers();
        let mut c = Change::builder(&doc);
        let locked = c.add_layer("Clamps");
        let inlay = c.add_layer("Inlay");
        let first = doc.layers[0].id;
        let some = *doc.vectors.keys().next().unwrap();
        c.move_to_layer(some, locked);
        doc.apply(&c.finish()).unwrap();
        layer(&mut doc, locked).locked = true;
        layer(&mut doc, inlay).material = "Walnut".into();
        let target = doc.layers[1].id;
        layer(&mut doc, target).visible = false;
        let change = MergeVisibleLayers::run(&doc, &Params { layer: target }).unwrap();
        doc.apply(&change).unwrap();
        let target_layer = doc.layer(target).unwrap();
        assert!(target_layer.visible, "the merged work stays in sight");
        let ids: Vec<LayerId> = doc.layers.iter().map(|l| l.id).collect();
        assert_eq!(
            ids,
            [target, locked],
            "{first:?} and Inlay merged, Clamps kept"
        );
        assert!(doc.layer(locked).unwrap().locked);
        assert_eq!(
            doc.layer_of(some),
            locked,
            "a locked layer's vectors stay put"
        );
        let notes = change.notes.join("\n");
        assert!(notes.contains("Clamps"), "{notes}");
        assert!(notes.contains("locked"), "{notes}");
        assert!(notes.contains("was hidden"), "{notes}");
        assert!(notes.contains("Walnut"), "{notes}");
    }
}
