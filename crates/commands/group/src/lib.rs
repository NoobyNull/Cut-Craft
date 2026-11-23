//! Group: the selected objects become one group, picked and moved together (dragging any
//! one of them, picked or not, moves the whole group). Groups hold groups: a group or a
//! text grouped with something else keeps its own group inside the new one, so text stays
//! editable (double-click the group to step inside) and part names stay. Grouping inside a
//! group (stepped into it) makes the new group there.

use cut_craft_core::document::Document;
use cut_craft_core::{Change, Command, CommandError, Form, Meta, NoParams};
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
            form: Form::Apply,
        }
    }

    fn run(doc: &Document, _: &NoParams) -> Result<Change, CommandError> {
        // each picked item: a group picked whole, or a vector on its own
        let mut groups = BTreeSet::new();
        let mut lone = Vec::new();
        for &id in &doc.selection {
            match doc.picked_group(id) {
                Some(g) => {
                    groups.insert(g);
                }
                None => lone.push(id),
            }
        }
        if groups.len() + lone.len() < 2 {
            return Err(CommandError::Failed(
                "select at least two objects to group".into(),
            ));
        }
        // the group they are all in (stepped inside it), which the new group goes into
        let around: BTreeSet<Option<u64>> = groups
            .iter()
            .map(|g| doc.group_parents.get(g).copied())
            .chain(lone.iter().map(|id| doc.vector_groups.get(id).copied()))
            .collect();
        let around = if around.len() == 1 {
            around.into_iter().next().flatten()
        } else {
            None
        };
        let mut c = Change::builder(doc);
        let group = c.next_group();
        c.set_group_parent(group, around);
        for &g in &groups {
            c.set_group_parent(g, Some(group));
        }
        for &id in &lone {
            c.set_group(id, Some(group));
        }
        let names: BTreeSet<&str> = doc
            .selection
            .iter()
            .filter_map(|&id| doc.part_name(id))
            .collect();
        if let (true, Some(&first)) = (names.len() > 1, names.first()) {
            c.note(format!(
                "The cut list counts a group as one part, so it lists these as one part named \
                 {}. Ungroup to list {} apart again",
                first,
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
    use cut_craft_core::testing::{document_with, square};

    cut_craft_core::contract_tests!(Group);

    #[test]
    fn the_selection_becomes_one_group() {
        let doc = cut_craft_core::testing::after::<Group>(&NoParams {});
        let members = doc.group_members(EntityId(1));
        assert_eq!(members.len(), doc.vectors.len());
    }

    /// Text "A" (group 4, part "Sign") and a border, both selected.
    fn text_and_border() -> Document {
        let mut doc = document_with(vec![square(0.0, 0.0, 5.0), square(-5.0, -5.0, 20.0)]);
        doc.vector_groups.insert(EntityId(1), 4);
        doc.texts.insert(4, serde_json::json!({"text": "A"}));
        doc.part_names.insert(4, "Sign".into());
        doc
    }

    #[test]
    fn grouped_text_stays_editable_text_with_its_part_name() {
        let mut doc = text_and_border();
        let change = Group::run(&doc, &NoParams {}).unwrap();
        assert_eq!(change.notes, Vec::<String>::new(), "nothing to warn about");
        doc.apply(&change).unwrap();
        assert_eq!(doc.group_members(EntityId(1)).len(), 2, "one group");
        assert_eq!(doc.text_group(EntityId(1)), Some(4), "still text");
        assert!(doc.texts.contains_key(&4));
        assert_eq!(doc.part_name(EntityId(1)), Some("Sign"));
        assert_eq!(doc.group_chain(EntityId(2)).len(), 1);
    }

    #[test]
    fn grouping_inside_a_group_makes_the_new_group_there() {
        let mut doc = document_with(vec![
            square(0.0, 0.0, 5.0),
            square(10.0, 0.0, 5.0),
            square(20.0, 0.0, 5.0),
        ]);
        for id in [1, 2, 3] {
            doc.vector_groups.insert(EntityId(id), 8);
        }
        // stepped inside group 8, two of its three picked
        let mut c = Change::builder(&doc);
        c.select([EntityId(1), EntityId(2)]);
        doc.apply(&c.finish()).unwrap();
        doc.apply(&Group::run(&doc, &NoParams {}).unwrap()).unwrap();
        let inner = doc.vector_groups[&EntityId(1)];
        assert_eq!(doc.vector_groups[&EntityId(2)], inner);
        assert_eq!(doc.group_chain(EntityId(1)), [inner, 8]);
        assert_eq!(doc.group_chain(EntityId(3)), [8]);
        assert_eq!(
            doc.group_members(EntityId(3)).len(),
            3,
            "still one outer group"
        );
    }

    #[test]
    fn joining_two_named_parts_says_the_cut_list_lists_one() {
        let mut doc = text_and_border();
        doc.vector_groups.insert(EntityId(2), 5);
        doc.part_names.insert(5, "Frame".into());
        let notes = Group::run(&doc, &NoParams {}).unwrap().notes;
        assert!(notes.iter().any(|n| n.contains("one part")), "{notes:?}");
    }

    #[test]
    fn grouping_plain_vectors_says_nothing() {
        let doc = cut_craft_core::testing::sample_document();
        assert_eq!(
            Group::run(&doc, &NoParams {}).unwrap().notes,
            Vec::<String>::new()
        );
    }

    #[test]
    fn one_group_alone_is_not_enough() {
        let mut doc = text_and_border();
        let mut c = Change::builder(&doc);
        c.select([EntityId(1)]);
        doc.apply(&c.finish()).unwrap();
        assert!(Group::run(&doc, &NoParams {}).is_err());
    }
}
