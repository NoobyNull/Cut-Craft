//! Ungroup: the selected groups come apart one level, onto the layers their vectors are on
//! (grouping never moves them), or all onto the current layer. A group or text that was
//! grouped comes back out as it was (text stays editable); ungrouping text itself turns it
//! into curves.

use cut_craft_core::document::Document;
use cut_craft_core::{Change, Command, CommandError, Form, Meta, NoParams};

/// Ungroup onto the original object layers (U).
pub struct Ungroup;

/// Ungroup onto the current layer.
pub struct UngroupOntoCurrentLayer;

/// The selected groups taken apart one level: what they held (vectors, and groups inside
/// them, whole) goes into the group around them, if any. A vector picked on its own inside a
/// group leaves that group. `onto_current` moves them to the current layer.
fn ungroup(doc: &Document, onto_current: bool) -> Result<Change, CommandError> {
    if !doc
        .selection
        .iter()
        .any(|id| doc.vector_groups.contains_key(id))
    {
        return Err(CommandError::Failed(
            "nothing selected is in a group. Select a group to ungroup".into(),
        ));
    }
    let mut c = Change::builder(doc);
    let mut taken_apart = std::collections::BTreeSet::new();
    for &id in &doc.selection {
        match doc.picked_group(id) {
            Some(group) => {
                taken_apart.insert(group);
            }
            // picked on its own inside a group: only it leaves, for the group around that
            None => {
                if let Some(&group) = doc.vector_groups.get(&id) {
                    // a letter pulled out of text is plain curves; so is the rest of it
                    c.set_text(group, None);
                    c.set_group(id, doc.group_parents.get(&group).copied());
                }
            }
        }
        if onto_current {
            c.move_to_layer(id, doc.current_layer);
        }
    }
    for &group in &taken_apart {
        let around = doc.group_parents.get(&group).copied();
        // ungrouping text is Convert Text to Curves: it stops being editable text
        c.set_text(group, None);
        c.set_group_parent(group, None);
        for &id in &doc.selection {
            if doc.vector_groups.get(&id) == Some(&group) {
                c.set_group(id, around);
            }
        }
        for (&inner, _) in doc.group_parents.iter().filter(|(_, p)| **p == group) {
            c.set_group_parent(inner, around);
        }
    }
    Ok(c.finish())
}

impl Command for UngroupOntoCurrentLayer {
    const ID: &'static str = "ungroup_onto_current_layer";
    type Params = NoParams;

    fn meta() -> Meta {
        Meta {
            label: "Ungroup onto Current Layer",
            group: "Edit",
            icon: "",
            shortcut: "",
            form: Form::Apply,
        }
    }

    fn run(doc: &Document, _: &NoParams) -> Result<Change, CommandError> {
        ungroup(doc, true)
    }
}

impl Command for Ungroup {
    const ID: &'static str = "ungroup";
    type Params = NoParams;

    fn meta() -> Meta {
        Meta {
            label: "Ungroup",
            group: "Edit",
            icon: "",
            shortcut: "",
            form: Form::Apply,
        }
    }

    fn run(doc: &Document, _: &NoParams) -> Result<Change, CommandError> {
        ungroup(doc, false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cut_craft_core::document::EntityId;

    fn grouped() -> Document {
        let mut doc = cut_craft_core::testing::sample_document();
        let group = doc.next_group();
        let mut c = Change::builder(&doc);
        for &id in &doc.selection {
            c.set_group(id, Some(group));
        }
        let change = c.finish();
        doc.apply(&change).unwrap();
        doc
    }

    cut_craft_core::contract_tests!(Ungroup, grouped);

    #[test]
    fn the_vectors_stand_alone_again() {
        let mut doc = grouped();
        assert!(doc.group_members(EntityId(1)).len() > 1);
        let change = Ungroup::run(&doc, &NoParams {}).unwrap();
        doc.apply(&change).unwrap();
        assert_eq!(doc.group_members(EntityId(1)), [EntityId(1)]);
        doc.apply(&change.inverse()).unwrap();
        assert!(
            doc.group_members(EntityId(1)).len() > 1,
            "undo regroups them"
        );
    }

    #[test]
    fn ungrouping_onto_the_current_layer_moves_every_member_there() {
        let mut doc = grouped();
        let layers_before: Vec<_> = doc.vectors.keys().map(|&id| doc.layer_of(id)).collect();
        // onto their original layers: nothing moves
        let kept = Ungroup::run(&doc, &NoParams {}).unwrap();
        let mut same = doc.clone();
        same.apply(&kept).unwrap();
        let layers: Vec<_> = same.vectors.keys().map(|&id| same.layer_of(id)).collect();
        assert_eq!(layers, layers_before);
        // a new current layer: onto it, every member goes there
        let mut c = Change::builder(&doc);
        let layer = c.add_layer("Ungrouped");
        c.set_current_layer(layer);
        doc.apply(&c.finish()).unwrap();
        doc.apply(&UngroupOntoCurrentLayer::run(&doc, &NoParams {}).unwrap())
            .unwrap();
        assert!(doc.vectors.keys().all(|&id| doc.layer_of(id) == layer));
    }

    #[test]
    fn ungrouping_gives_back_the_text_inside_as_text() {
        use cut_craft_core::testing::{document_with, square};
        let mut doc = document_with(vec![square(0.0, 0.0, 5.0), square(-5.0, -5.0, 20.0)]);
        doc.vector_groups.insert(EntityId(1), 4);
        doc.vector_groups.insert(EntityId(2), 9);
        doc.group_parents.insert(4, 9);
        doc.texts.insert(4, serde_json::json!({"text": "A"}));
        doc.apply(&Ungroup::run(&doc, &NoParams {}).unwrap())
            .unwrap();
        assert_eq!(doc.group_chain(EntityId(1)), [4], "the text's own group");
        assert_eq!(doc.text_group(EntityId(1)), Some(4), "still text");
        assert_eq!(doc.group_chain(EntityId(2)), Vec::<u64>::new());
        assert!(doc.group_parents.is_empty());
    }

    #[test]
    fn ungrouping_a_group_inside_a_group_keeps_it_in_the_outer_one() {
        use cut_craft_core::testing::{document_with, square};
        let mut doc = document_with(vec![
            square(0.0, 0.0, 5.0),
            square(10.0, 0.0, 5.0),
            square(20.0, 0.0, 5.0),
        ]);
        doc.vector_groups.insert(EntityId(1), 4);
        doc.vector_groups.insert(EntityId(2), 4);
        doc.vector_groups.insert(EntityId(3), 9);
        doc.group_parents.insert(4, 9);
        // stepped inside group 9, the inner group picked
        let mut c = Change::builder(&doc);
        c.select([EntityId(1), EntityId(2)]);
        doc.apply(&c.finish()).unwrap();
        doc.apply(&Ungroup::run(&doc, &NoParams {}).unwrap())
            .unwrap();
        assert_eq!(doc.group_chain(EntityId(1)), [9]);
        assert_eq!(doc.group_members(EntityId(1)).len(), 3);
    }

    #[test]
    fn one_item_picked_inside_a_group_leaves_it_alone() {
        use cut_craft_core::testing::{document_with, square};
        let mut doc = document_with(vec![
            square(0.0, 0.0, 5.0),
            square(10.0, 0.0, 5.0),
            square(20.0, 0.0, 5.0),
        ]);
        // group 20 holds 1, 2 and group 30 (holding 3), and sits in group 10
        doc.vector_groups.insert(EntityId(1), 20);
        doc.vector_groups.insert(EntityId(2), 20);
        doc.vector_groups.insert(EntityId(3), 30);
        doc.group_parents.insert(30, 20);
        doc.group_parents.insert(20, 10);
        let mut c = Change::builder(&doc);
        c.select([EntityId(1)]);
        doc.apply(&c.finish()).unwrap();
        doc.apply(&Ungroup::run(&doc, &NoParams {}).unwrap())
            .unwrap();
        assert_eq!(
            doc.group_chain(EntityId(1)),
            [10],
            "out to the group around"
        );
        assert_eq!(doc.group_chain(EntityId(2)), [20, 10], "the rest stays");
        assert_eq!(doc.group_chain(EntityId(3)), [30, 20, 10]);
    }
}
