//! Group: the selected vectors become one group, picked and moved together. Groups do not
//! hold groups yet: text grouped with something else becomes curves (Group says so in red
//! under the tools), and a part's name moves to the new group.

use cut_craft_core::document::Document;
use cut_craft_core::{Change, Command, CommandError, Meta, NoParams};
use std::collections::BTreeSet;

pub struct Group;

impl Command for Group {
    const ID: &'static str = "group";
    type Params = NoParams;

    fn meta() -> Meta {
        Meta {
            label: "Group",
            group: "Edit",
            icon: "drawing/create_vectors/group.png",
            shortcut: "",
        }
    }

    fn run(doc: &Document, _: &NoParams) -> Result<Change, CommandError> {
        if doc.selection.len() < 2 {
            return Err(CommandError::Failed(
                "select at least two objects to group".into(),
            ));
        }
        // ponytail: groups do not nest; grouping groups merges them into one, so text in it
        // loses its settings (becomes curves) until groups can hold groups
        let group = doc.next_group();
        let mut c = Change::builder(doc);
        let old: BTreeSet<u64> = doc
            .selection
            .iter()
            .filter_map(|id| doc.vector_groups.get(id).copied())
            .collect();
        for &id in &doc.selection {
            c.set_group(id, Some(group));
        }
        let mut names = BTreeSet::new();
        let mut text = false;
        for &g in &old {
            if doc.texts.contains_key(&g) {
                text = true;
                c.set_text(g, None);
            }
            if let Some(name) = doc.part_names.get(&g) {
                names.insert(name.clone());
                c.set_part_name(g, None);
            }
        }
        // one part name carries over; more than one cannot all fit
        if names.len() == 1 {
            c.set_part_name(group, names.pop_first());
        }
        if text {
            c.note(
                "Grouping text turns it into curves: it can no longer be changed with Edit \
                 Text. Undo to keep it as text",
            );
        }
        if names.len() > 1 {
            c.note(format!(
                "The group has no part name: it joined parts named {}. Name it again with \
                 Name Part",
                names.into_iter().collect::<Vec<_>>().join(", ")
            ));
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

    #[test]
    fn grouping_text_warns_it_becomes_curves_and_keeps_a_part_name() {
        use cut_craft_core::testing::{document_with, square};
        let mut doc = document_with(vec![square(0.0, 0.0, 5.0), square(20.0, 0.0, 5.0)]);
        let ids: Vec<EntityId> = doc.vectors.keys().copied().collect();
        doc.vector_groups.insert(ids[0], 4);
        doc.texts.insert(4, serde_json::json!({"text": "A"}));
        doc.part_names.insert(4, "Sign".into());
        let change = Group::run(&doc, &NoParams {}).unwrap();
        assert!(
            change.notes.iter().any(|n| n.contains("curves")),
            "{:?}",
            change.notes
        );
        doc.apply(&change).unwrap();
        let group = doc.vector_groups[&ids[1]];
        assert_eq!(doc.part_names.get(&group).map(String::as_str), Some("Sign"));
    }

    #[test]
    fn grouping_plain_vectors_says_nothing() {
        let doc = cut_craft_core::testing::sample_document();
        assert_eq!(
            Group::run(&doc, &NoParams {}).unwrap().notes,
            Vec::<String>::new()
        );
    }
}
