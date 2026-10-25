//! Ungroup: the selected vectors leave their groups.

use cut_craft_core::document::Document;
use cut_craft_core::{Change, Command, CommandError, Meta, NoParams};

pub struct Ungroup;

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
        if !doc
            .selection
            .iter()
            .any(|id| doc.vector_groups.contains_key(id))
        {
            return Err(CommandError::Failed(
                "nothing selected is in a group".into(),
            ));
        }
        let mut c = Change::builder(doc);
        for &id in &doc.selection {
            c.set_group(id, None);
        }
        Ok(c.finish())
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
}
