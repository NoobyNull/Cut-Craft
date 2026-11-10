//! Ungroup: the selected vectors leave their groups, onto the layers they are on (grouping
//! never moves them), or all onto the current layer (Aspire's Ungroup onto Group's Layer).

use cut_craft_core::document::Document;
use cut_craft_core::{Change, Command, CommandError, Meta, NoParams};

/// Ungroup onto the original object layers (U).
pub struct Ungroup;

/// Ungroup onto the current layer.
pub struct UngroupOntoCurrentLayer;

/// The selected vectors out of their groups; `onto_current` moves them to the current layer.
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
    for &id in &doc.selection {
        // ungrouping text is Convert Text to Curves: it stops being editable text
        if let Some(&group) = doc.vector_groups.get(&id) {
            c.set_text(group, None);
        }
        c.set_group(id, None);
        if onto_current {
            c.move_to_layer(id, doc.current_layer);
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
}
