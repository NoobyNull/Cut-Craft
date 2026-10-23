use aspire_core::document::{Document, EntityId, LayerId};
use aspire_core::file::{load_job, save_job};
use aspire_core::geometry::{Point, Rect};
use aspire_core::testing::square;
use aspire_core::{Change, History};

fn add(doc: &mut Document, x: f64) -> EntityId {
    let change = Change::add_and_select(doc, square(x, 0.0, 10.0));
    doc.apply(&change).unwrap();
    doc.next_id().0.checked_sub(1).map(EntityId).unwrap()
}

#[test]
fn a_new_document_has_one_current_layer() {
    let doc = Document::default();
    assert_eq!(doc.layers.len(), 1);
    assert_eq!(doc.layers[0].name, "Layer 1");
    assert_eq!(doc.current_layer, doc.layers[0].id);
}

#[test]
fn new_vectors_go_on_the_current_layer_and_leave_it_when_removed() {
    let mut doc = Document::default();
    let mut c = Change::builder(&doc);
    let second = c.add_layer("Cut outs");
    c.set_current_layer(second);
    doc.apply(&c.finish()).unwrap();
    let id = add(&mut doc, 0.0);
    assert_eq!(doc.layer_of(id), second);
    let mut c = Change::builder(&doc);
    c.remove(id);
    let removal = c.finish();
    doc.apply(&removal).unwrap();
    assert!(!doc.vector_layers.contains_key(&id));
    doc.apply(&removal.inverse()).unwrap();
    assert_eq!(doc.layer_of(id), second, "undo puts it back on its layer");
}

#[test]
fn moving_to_a_layer_is_undoable() {
    let mut doc = Document::default();
    let id = add(&mut doc, 0.0);
    let mut history = History::default();
    let mut c = Change::builder(&doc);
    let other = c.add_layer("Other");
    c.move_to_layer(id, other);
    let change = c.finish();
    history.apply(&mut doc, change).unwrap();
    assert_eq!(doc.layer_of(id), other);
    history.undo(&mut doc);
    assert_eq!(doc.layer_of(id), LayerId(1));
    assert_eq!(doc.layers.len(), 1);
}

#[test]
fn hidden_and_locked_layers_cannot_be_picked_or_boxed() {
    let mut doc = Document::default();
    let id = add(&mut doc, 0.0);
    assert_eq!(doc.pick(Point::new(10.0, 5.0), 0.5), Some(id));
    for (visible, locked) in [(false, false), (true, true)] {
        let mut c = Change::builder(&doc);
        let mut layer = doc.layers[0].clone();
        (layer.visible, layer.locked) = (visible, locked);
        c.replace_layer(layer);
        let mut d = doc.clone();
        d.apply(&c.finish()).unwrap();
        assert_eq!(d.pick(Point::new(10.0, 5.0), 0.5), None);
        let boxed = d.window(
            Rect::new(Point::new(-1.0, -1.0), Point::new(20.0, 20.0)),
            false,
        );
        assert_eq!(boxed, vec![]);
    }
}

#[test]
fn layers_survive_saving_and_old_jobs_get_the_default_layer() {
    let mut doc = Document::default();
    let mut c = Change::builder(&doc);
    c.add_layer("Second");
    doc.apply(&c.finish()).unwrap();
    add(&mut doc, 0.0);
    assert_eq!(load_job(&save_job(&doc)).unwrap(), doc);
    let mut json: serde_json::Value =
        serde_json::from_slice(&save_job(&Document::default())).unwrap();
    for key in ["layers", "current_layer", "vector_layers"] {
        json["document"].as_object_mut().unwrap().remove(key);
    }
    assert_eq!(
        load_job(&serde_json::to_vec(&json).unwrap())
            .unwrap()
            .layers
            .len(),
        1
    );
}

#[test]
fn a_layer_cannot_be_removed_while_vectors_are_on_it() {
    let mut doc = Document::default();
    let mut c = Change::builder(&doc);
    let spare = c.add_layer("Spare");
    doc.apply(&c.finish()).unwrap();
    add(&mut doc, 0.0);
    let mut c = Change::builder(&doc);
    c.remove_layer(LayerId(1));
    assert!(
        doc.clone().apply(&c.finish()).is_err(),
        "layer 1 still holds a vector"
    );
    let mut c = Change::builder(&doc);
    c.remove_layer(spare);
    doc.apply(&c.finish()).unwrap();
    assert_eq!(doc.layers.len(), 1);
}
