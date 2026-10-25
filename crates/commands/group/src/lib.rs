//! Group: the selected vectors become one group, picked and moved together.

use cut_craft_core::document::Document;
use cut_craft_core::{Change, Command, CommandError, Meta, NoParams};

pub struct Group;

impl Command for Group {
    const ID: &'static str = "group";
    type Params = NoParams;

    fn meta() -> Meta {
        Meta {
            label: "Group",
            group: "Edit",
            icon: "drawing/create_vectors/group.svg",
            shortcut: "",
        }
    }

    fn run(doc: &Document, _: &NoParams) -> Result<Change, CommandError> {
        if doc.selection.len() < 2 {
            return Err(CommandError::Failed(
                "select at least two objects to group".into(),
            ));
        }
        // ponytail: groups do not nest; grouping groups merges them into one
        let group = doc.next_group();
        let mut c = Change::builder(doc);
        for &id in &doc.selection {
            c.set_group(id, Some(group));
        }
        Ok(c.finish())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cut_craft_core::document::EntityId;

    cut_craft_core::contract_tests!(Group);

    #[test]
    fn the_selection_becomes_one_group() {
        let doc = cut_craft_core::testing::after::<Group>(&NoParams {});
        let members = doc.group_members(EntityId(1));
        assert_eq!(members.len(), doc.vectors.len());
    }
}
